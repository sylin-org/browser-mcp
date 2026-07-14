// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- policy-free, memory-only JavaScript dialog state (ADR-0078 D7).
//
// The service worker owns CDP routing. This module only keeps the latest observed dialog per tab
// and translates one explicit resolution action into Page.handleJavaScriptDialog parameters.
(function () {

const MESSAGE_LIMIT = 400;

function clip(value) {
  const text = String(value || "").replace(/\s+/g, " ").trim();
  if (text.length <= MESSAGE_LIMIT) return text;
  return text.slice(0, MESSAGE_LIMIT - 3) + "...";
}

function createDialogStore() {
  const records = new Map();

  function opened(tabId, params) {
    const record = {
      type: String((params && params.type) || "unknown"),
      message: clip(params && params.message),
    };
    records.set(tabId, record);
    return { ...record };
  }

  function current(tabId) {
    const record = records.get(tabId);
    return record ? { ...record } : null;
  }

  function remove(tabId) {
    return records.delete(tabId);
  }

  function clear() {
    records.clear();
  }

  return { opened, current, remove, clear };
}

function resolutionCommand(action, text) {
  if (action === "accept") return { accept: true };
  if (action === "dismiss") return { accept: false };
  if (action === "respond") {
    if (typeof text !== "string") throw new Error("respond requires text");
    return { accept: true, promptText: text };
  }
  throw new Error(`unsupported dialog action: ${action}`);
}

const GhostlightDialog = { MESSAGE_LIMIT, createDialogStore, resolutionCommand };
if (typeof module !== "undefined" && module.exports) {
  module.exports = GhostlightDialog;
} else {
  self.GhostlightDialog = GhostlightDialog;
}
})();
