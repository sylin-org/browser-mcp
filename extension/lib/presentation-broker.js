// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- document-aware, policy-free presentation delivery (ADR-0081).
(function initPresentationBroker(root) {
  "use strict";

  const SNAPSHOT_VERSION = 1;
  const DEFAULT_EVENT_TTL_MS = 2500;
  const DEFAULT_DELIVERY_WAIT_MS = 1500;
  const MAX_TABS = 128;
  const MAX_STATES_PER_TAB = 16;
  const MAX_EVENTS_PER_TAB = 32;
  const MAX_RETAINED_BYTES = 256 * 1024;

  function estimateBytes(value) {
    try { return new TextEncoder().encode(JSON.stringify(value)).byteLength; }
    catch { return 0; }
  }

  function createPresentationBroker(options) {
    const deliver = options.deliver;
    const activate = options.activate || (async () => ({ ready: false, reason: "visual layer unavailable" }));
    const now = options.now || Date.now;
    const setTimer = options.setTimer || setTimeout;
    const clearTimer = options.clearTimer || clearTimeout;
    const onStateChange = options.onStateChange || (() => {});
    const maxTabs = options.maxTabs || MAX_TABS;
    const maxStatesPerTab = options.maxStatesPerTab || MAX_STATES_PER_TAB;
    const maxEventsPerTab = options.maxEventsPerTab || MAX_EVENTS_PER_TAB;
    const maxRetainedBytes = options.maxRetainedBytes || MAX_RETAINED_BYTES;
    const deliveryWaitMs = options.deliveryWaitMs || DEFAULT_DELIVERY_WAIT_MS;
    const tabs = new Map();
    let nextRevision = 1;
    let retainedBytes = 0;

    function schedule(callback, delayMs, keepAlive) {
      const timer = setTimer(callback, delayMs);
      if (!keepAlive && timer && typeof timer.unref === "function") timer.unref();
      return timer;
    }

    function statePriority(channel) {
      if (channel.startsWith("attention:")) return 0;
      if (channel === "notification") return 1;
      if (channel === "narration") return 2;
      return 3;
    }

    function tabFor(tabId, create) {
      let tab = tabs.get(tabId);
      if (tab || !create) return tab || null;
      if (!Number.isSafeInteger(tabId)) return null;
      if (tabs.size >= maxTabs) {
        for (const [candidateId, candidate] of tabs) {
          if (!candidate.ready && candidate.states.size === 0 && candidate.events.length === 0) {
            tabs.delete(candidateId);
            break;
          }
        }
      }
      if (tabs.size >= maxTabs) return null;
      tab = {
        documentId: null,
        ready: false,
        states: new Map(),
        events: [],
        flushing: false,
        waking: false,
        loading: false,
        readyWaiters: [],
        idleWaiters: [],
        captureBarrier: false,
      };
      tabs.set(tabId, tab);
      return tab;
    }

    function settleWaiters(record, result) {
      for (const waiter of record.waiters.splice(0)) {
        if (waiter.timer) clearTimer(waiter.timer);
        waiter.resolve(result);
      }
    }

    function addWaiter(record, waitMs) {
      return new Promise((resolve) => {
        const waiter = { resolve, timer: null };
        waiter.timer = schedule(() => {
          const index = record.waiters.indexOf(waiter);
          if (index >= 0) record.waiters.splice(index, 1);
          resolve({ shown: false, reason: "visual delivery was not acknowledged" });
        }, waitMs || deliveryWaitMs, true);
        record.waiters.push(waiter);
      });
    }

    function releaseRecord(record, result) {
      if (record.timer) clearTimer(record.timer);
      record.timer = null;
      retainedBytes = Math.max(0, retainedBytes - record.bytes);
      settleWaiters(record, result || { shown: false, reason: "presentation retired" });
      record.message = null;
      record.clearMessage = null;
    }

    function snapshot() {
      const savedTabs = [];
      const at = now();
      for (const [tabId, tab] of tabs) {
        const states = [];
        for (const record of tab.states.values()) {
          if (record.deadline !== null && record.deadline <= at) continue;
          states.push({
            channel: record.channel,
            revision: record.revision,
            message: record.message,
            clearMessage: record.clearMessage,
            deadline: record.deadline,
          });
        }
        if (states.length > 0) savedTabs.push({ tabId, states });
      }
      return { version: SNAPSHOT_VERSION, nextRevision, tabs: savedTabs };
    }

    function stateChanged() {
      try { onStateChange(snapshot()); } catch { /* persistence is best-effort */ }
    }

    function envelope(record, documentId) {
      return Object.assign({}, record.message, {
        presentation: {
          channel: record.channel,
          revision: record.revision,
          documentId,
        },
      });
    }

    function validAck(record, documentId, response) {
      const ack = response && response.presentationAck;
      return !!ack && ack.channel === record.channel &&
        ack.revision === record.revision && ack.documentId === documentId;
    }

    function failPending(tab, reason) {
      const result = { shown: false, reason: reason || "visual layer unavailable" };
      for (const record of tab.states.values()) settleWaiters(record, result);
      for (const record of tab.events) settleWaiters(record, result);
    }

    function settleReadyWaiters(tab, ready) {
      for (const waiter of tab.readyWaiters.splice(0)) {
        if (waiter.timer) clearTimer(waiter.timer);
        waiter.resolve(ready);
      }
    }

    function waitForReady(tabId, tab) {
      if (tab.ready && tab.documentId) return Promise.resolve(true);
      const waiting = new Promise((resolve) => {
        const waiter = { resolve, timer: null };
        waiter.timer = schedule(() => {
          const index = tab.readyWaiters.indexOf(waiter);
          if (index >= 0) tab.readyWaiters.splice(index, 1);
          resolve(false);
        }, deliveryWaitMs, true);
        tab.readyWaiters.push(waiter);
      });
      requestActivation(tabId, tab);
      return waiting;
    }

    function waitForIdle(tab) {
      if (!tab.flushing) return Promise.resolve();
      return new Promise((resolve) => tab.idleWaiters.push(resolve));
    }

    function settleIdleWaiters(tab) {
      if (tab.flushing) return;
      for (const resolve of tab.idleWaiters.splice(0)) resolve();
    }

    function requestActivation(tabId, tab) {
      if (tab.waking) return;
      tab.waking = true;
      Promise.resolve()
        .then(() => activate(tabId))
        .then((result) => {
          tab.waking = false;
          if (result && result.ready === false) {
            failPending(tab, result.reason);
            settleReadyWaiters(tab, false);
          }
        })
        .catch((error) => {
          tab.waking = false;
          failPending(tab, (error && error.message) || "visual layer unavailable");
          settleReadyWaiters(tab, false);
        });
    }

    function dueStates(tab) {
      const at = now();
      return Array.from(tab.states.values())
        .filter((record) => {
          if (record.deadline !== null && record.deadline <= at) return false;
          return record.deliveredDocumentId !== tab.documentId;
        })
        .sort((a, b) => statePriority(a.channel) - statePriority(b.channel) || a.revision - b.revision);
    }

    async function flushTab(tabId) {
      const tab = tabFor(tabId, false);
      if (!tab || tab.flushing || tab.captureBarrier || !tab.ready || !tab.documentId) return;
      tab.flushing = true;
      try {
        while (tab.ready && tab.documentId && !tab.captureBarrier) {
          const documentId = tab.documentId;
          const state = dueStates(tab)[0];
          const event = state ? null : tab.events[0];
          const record = state || event;
          if (!record) break;
          if (record.deadline !== null && record.deadline <= now()) {
            if (state) tab.states.delete(record.channel);
            else tab.events.shift();
            releaseRecord(record, { shown: false, reason: "presentation expired before delivery" });
            if (state) stateChanged();
            continue;
          }
          let response;
          try {
            response = await deliver(tabId, documentId, envelope(record, documentId));
          } catch (error) {
            if (tab.documentId === documentId) tab.ready = false;
            requestActivation(tabId, tab);
            break;
          }
          if (tab.documentId !== documentId) continue;
          if (!validAck(record, documentId, response)) {
            tab.ready = false;
            requestActivation(tabId, tab);
            break;
          }
          const result = Object.assign({}, response);
          delete result.presentationAck;
          settleWaiters(record, result);
          if (state) {
            record.deliveredDocumentId = documentId;
          } else {
            tab.events.shift();
            releaseRecord(record, result);
          }
        }
      } finally {
        tab.flushing = false;
        settleIdleWaiters(tab);
      }
    }

    function expireState(tabId, channel, revision) {
      const tab = tabFor(tabId, false);
      const record = tab && tab.states.get(channel);
      if (!record || record.revision !== revision) return;
      const clearMessage = record.clearMessage ? Object.assign({}, record.clearMessage) : null;
      tab.states.delete(channel);
      releaseRecord(record, { shown: false, reason: "presentation expired" });
      stateChanged();
      if (clearMessage && tab.ready && tab.documentId) {
        publishEvent(tabId, clearMessage, { channel: `clear:${channel}`, ttlMs: 1000 });
      }
    }

    function makeRecord(channel, message, clearMessage, deadline) {
      const revision = nextRevision++;
      const bytes = estimateBytes({ channel, message, clearMessage });
      return {
        channel,
        revision,
        message: Object.assign({}, message),
        clearMessage: clearMessage ? Object.assign({}, clearMessage) : null,
        deadline,
        bytes,
        deliveredDocumentId: null,
        timer: null,
        waiters: [],
      };
    }

    function publishState(tabId, channel, message, optionsForState) {
      const config = optionsForState || {};
      const tab = tabFor(tabId, true);
      if (!tab || typeof channel !== "string" || !channel || !message) {
        return {
          accepted: false,
          replaced: false,
          revision: null,
          delivery: Promise.resolve({ shown: false, reason: "presentation capacity exceeded" }),
        };
      }
      const prior = tab.states.get(channel) || null;
      if (!prior && tab.states.size >= maxStatesPerTab) {
        return {
          accepted: false,
          replaced: false,
          revision: null,
          delivery: Promise.resolve({ shown: false, reason: "presentation state capacity exceeded" }),
        };
      }
      const deadline = Number.isFinite(config.deadline)
        ? config.deadline
        : (Number.isFinite(config.ttlMs) ? now() + Math.max(1, config.ttlMs) : null);
      const record = makeRecord(channel, message, config.clearMessage, deadline);
      const prospective = retainedBytes - (prior ? prior.bytes : 0) + record.bytes;
      if (prospective > maxRetainedBytes) {
        return {
          accepted: false,
          replaced: !!prior,
          revision: null,
          delivery: Promise.resolve({ shown: false, reason: "presentation byte capacity exceeded" }),
        };
      }
      if (prior) releaseRecord(prior, { shown: false, reason: "presentation replaced before acknowledgement" });
      tab.states.set(channel, record);
      retainedBytes = prospective;
      if (deadline !== null) {
        record.timer = schedule(
          () => expireState(tabId, channel, record.revision),
          Math.max(1, deadline - now())
        );
      }
      const delivery = config.waitForDelivery === false
        ? Promise.resolve(null)
        : addWaiter(record, config.deliveryWaitMs);
      stateChanged();
      if (tab.ready) flushTab(tabId);
      else requestActivation(tabId, tab);
      return { accepted: true, replaced: !!prior, revision: record.revision, delivery };
    }

    function clearState(tabId, channel, clearMessage) {
      const tab = tabFor(tabId, false);
      if (!tab) return false;
      const record = tab.states.get(channel);
      if (!record) return false;
      tab.states.delete(channel);
      releaseRecord(record, { shown: false, reason: "presentation cleared" });
      stateChanged();
      if (clearMessage && tab.ready && tab.documentId) {
        publishEvent(tabId, clearMessage, { channel: `clear:${channel}`, ttlMs: 1000 });
      }
      return true;
    }

    function clearPrefix(prefix, clearMessageFor) {
      const cleared = [];
      for (const [tabId, tab] of tabs) {
        for (const channel of Array.from(tab.states.keys())) {
          if (!channel.startsWith(prefix)) continue;
          const message = typeof clearMessageFor === "function"
            ? clearMessageFor(tab.states.get(channel).message)
            : clearMessageFor;
          if (clearState(tabId, channel, message)) cleared.push({ tabId, channel });
        }
      }
      return cleared;
    }

    function publishEvent(tabId, message, optionsForEvent) {
      const config = optionsForEvent || {};
      const tab = tabFor(tabId, true);
      if (!tab || !message) return Promise.resolve({ shown: false, reason: "presentation capacity exceeded" });
      while (tab.events.length >= maxEventsPerTab) {
        const oldest = tab.events.shift();
        releaseRecord(oldest, { shown: false, reason: "presentation event queue overflow" });
      }
      const channel = config.channel || "effect";
      const deadline = now() + Math.max(1, Number(config.ttlMs) || DEFAULT_EVENT_TTL_MS);
      const record = makeRecord(channel, message, null, deadline);
      while (retainedBytes + record.bytes > maxRetainedBytes && tab.events.length > 0) {
        const oldest = tab.events.shift();
        releaseRecord(oldest, { shown: false, reason: "presentation byte capacity exceeded" });
      }
      if (retainedBytes + record.bytes > maxRetainedBytes) {
        return Promise.resolve({ shown: false, reason: "presentation byte capacity exceeded" });
      }
      retainedBytes += record.bytes;
      tab.events.push(record);
      record.timer = schedule(() => {
        const index = tab.events.indexOf(record);
        if (index < 0) return;
        tab.events.splice(index, 1);
        releaseRecord(record, { shown: false, reason: "presentation expired before delivery" });
      }, Math.max(1, deadline - now()));
      const delivery = config.waitForDelivery === false
        ? Promise.resolve(null)
        : addWaiter(record, config.deliveryWaitMs);
      if (tab.ready) flushTab(tabId);
      else requestActivation(tabId, tab);
      return delivery;
    }

    async function publishCapture(tabId, message) {
      const tab = tabFor(tabId, true);
      if (!tab || !(await waitForReady(tabId, tab))) {
        return { success: true, unavailable: true };
      }
      const isHide = message && message.type === "HIDE_FOR_TOOL_USE";
      if (isHide) {
        tab.captureBarrier = true;
        await waitForIdle(tab);
      }
      const record = makeRecord("capture", message, null, now() + 1000);
      try {
        const response = await deliver(tabId, tab.documentId, envelope(record, tab.documentId));
        if (!validAck(record, tab.documentId, response)) {
          tab.ready = false;
          return { success: true, unavailable: true };
        }
        const result = Object.assign({}, response);
        delete result.presentationAck;
        return result;
      } catch {
        tab.ready = false;
        return { success: true, unavailable: true };
      } finally {
        if (isHide) {
          tab.captureBarrier = false;
          flushTab(tabId);
        }
      }
    }

    function retireEvents(tab, reason) {
      for (const record of tab.events.splice(0)) {
        releaseRecord(record, { shown: false, reason });
      }
    }

    function documentLoading(tabId) {
      const tab = tabFor(tabId, true);
      if (!tab) return false;
      tab.ready = false;
      tab.documentId = null;
      tab.loading = true;
      retireEvents(tab, "document changed before delivery");
      for (const record of tab.states.values()) record.deliveredDocumentId = null;
      return true;
    }

    function documentReady(tabId, documentId) {
      if (!Number.isSafeInteger(tabId) || typeof documentId !== "string" || !documentId) return false;
      const tab = tabFor(tabId, true);
      if (!tab) return false;
      if (tab.documentId !== documentId && !tab.loading) {
        retireEvents(tab, "document changed before delivery");
        for (const record of tab.states.values()) record.deliveredDocumentId = null;
      }
      tab.documentId = documentId;
      tab.ready = true;
      tab.loading = false;
      settleReadyWaiters(tab, true);
      flushTab(tabId);
      return true;
    }

    function activateTab(tabId) {
      const tab = tabFor(tabId, true);
      if (!tab) return false;
      if (!tab.ready) requestActivation(tabId, tab);
      return true;
    }

    function states(prefix) {
      const values = [];
      const at = now();
      for (const [tabId, tab] of tabs) {
        for (const record of tab.states.values()) {
          if (prefix && !record.channel.startsWith(prefix)) continue;
          if (record.deadline !== null && record.deadline <= at) continue;
          values.push({
            tabId,
            channel: record.channel,
            revision: record.revision,
            deadline: record.deadline,
            message: Object.assign({}, record.message),
          });
        }
      }
      return values;
    }

    function destroyTab(tabId) {
      const tab = tabs.get(tabId);
      if (!tab) return false;
      for (const record of tab.states.values()) releaseRecord(record, { shown: false, reason: "tab closed" });
      retireEvents(tab, "tab closed");
      settleReadyWaiters(tab, false);
      for (const resolve of tab.idleWaiters.splice(0)) resolve();
      tabs.delete(tabId);
      stateChanged();
      return true;
    }

    function clear() {
      for (const tabId of Array.from(tabs.keys())) destroyTab(tabId);
    }

    function restore(saved) {
      if (!saved || saved.version !== SNAPSHOT_VERSION || !Array.isArray(saved.tabs)) return false;
      clear();
      let highestRevision = 0;
      const at = now();
      for (const savedTab of saved.tabs.slice(0, maxTabs)) {
        if (!Number.isSafeInteger(savedTab.tabId) || !Array.isArray(savedTab.states)) continue;
        const tab = tabFor(savedTab.tabId, true);
        if (!tab) break;
        for (const raw of savedTab.states.slice(0, maxStatesPerTab)) {
          if (!raw || typeof raw.channel !== "string" || !raw.message) continue;
          const deadline = Number.isFinite(raw.deadline) ? raw.deadline : null;
          if (deadline !== null && deadline <= at) continue;
          const record = makeRecord(raw.channel, raw.message, raw.clearMessage, deadline);
          if (Number.isSafeInteger(raw.revision) && raw.revision > 0) record.revision = raw.revision;
          if (retainedBytes + record.bytes > maxRetainedBytes) break;
          retainedBytes += record.bytes;
          tab.states.set(record.channel, record);
          highestRevision = Math.max(highestRevision, record.revision);
          if (deadline !== null) {
            record.timer = schedule(
              () => expireState(savedTab.tabId, record.channel, record.revision),
              Math.max(1, deadline - at)
            );
          }
        }
      }
      nextRevision = Math.max(nextRevision, highestRevision + 1, Number(saved.nextRevision) || 1);
      return true;
    }

    return {
      publishState,
      clearState,
      clearPrefix,
      publishEvent,
      publishCapture,
      documentLoading,
      documentReady,
      activateTab,
      states,
      snapshot,
      restore,
      destroyTab,
      clear,
      stats: () => ({
        tabs: tabs.size,
        states: Array.from(tabs.values()).reduce((sum, tab) => sum + tab.states.size, 0),
        events: Array.from(tabs.values()).reduce((sum, tab) => sum + tab.events.length, 0),
        bytes: retainedBytes,
      }),
    };
  }

  const GhostlightPresentationBroker = {
    SNAPSHOT_VERSION,
    DEFAULT_EVENT_TTL_MS,
    DEFAULT_DELIVERY_WAIT_MS,
    MAX_TABS,
    MAX_STATES_PER_TAB,
    MAX_EVENTS_PER_TAB,
    MAX_RETAINED_BYTES,
    createPresentationBroker,
  };
  if (typeof module !== "undefined" && module.exports) {
    module.exports = GhostlightPresentationBroker;
  } else {
    root.GhostlightPresentationBroker = GhostlightPresentationBroker;
  }
})(typeof self !== "undefined" ? self : globalThis);
