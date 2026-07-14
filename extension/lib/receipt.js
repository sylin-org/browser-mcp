// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- bounded interaction receipts and recovery capsules (ADR-0078 D1/D4).
//
// Pure result vocabulary: callers collect browser facts and this module bounds and renders them.
// It deliberately says "observed after" and never assigns causality or transaction success.
(function () {
// Leave room for the service-authored nonce/origin boundary while keeping the final text under
// ADR-0078's 800/1200-character budgets. StructuredContent retains the bounded detail.
const SUCCESS_LIMIT = 420;
const FAILURE_LIMIT = 820;

function clip(value, max) {
  const text = String(value || "").replace(/\s+/g, " ").trim();
  if (text.length <= max) return text;
  return text.slice(0, Math.max(0, max - 3)) + "...";
}

function originOf(url) {
  try { return new URL(url).origin; } catch { return ""; }
}

function makeReceipt(facts) {
  const f = facts || {};
  const before = f.before || {};
  const after = f.after || {};
  const observed = {};
  if (after.url && after.url !== before.url) observed.urlChanged = clip(after.url, 240);
  if (after.title !== undefined && after.title !== before.title) observed.titleChanged = clip(after.title, 120);
  if (Number.isFinite(after.mutations)) observed.mutations = Math.max(0, after.mutations);
  if (Number.isFinite(after.renderSerial) && after.renderSerial > (before.renderSerial || 0)) {
    observed.renderAdvanced = true;
  }
  if (Array.isArray(after.changedElements) && after.changedElements.length) {
    observed.changedElements = after.changedElements.slice(0, 3);
  }
  const alertOrStatus = after.alert || after.status;
  if (alertOrStatus) observed.alertOrStatus = clip(alertOrStatus, 200);

  const blockers = [];
  if (after.dialogOpened) {
    blockers.push({
      kind: "dialog_open",
      summary: "A JavaScript dialog is blocking the tab.",
      nextStep: "Inspect and resolve the dialog explicitly before continuing.",
    });
  }
  for (const blocker of (f.blockers || []).slice(0, 3 - blockers.length)) {
    blockers.push({
      kind: String(blocker.kind || "target_missing"),
      summary: clip(blocker.summary, 200),
      nextStep: clip(blocker.nextStep, 200),
    });
  }

  const receipt = {
    targetAssurance: f.targetAssurance || "none",
    action: f.action || "unknown",
    observedAfter: observed,
    blockers,
    page: {
      tabId: f.tabId,
      url: clip(after.url || before.url, 240),
      origin: originOf(after.url || before.url),
      title: clip(after.title === undefined ? before.title : after.title, 120),
      renderSerial: Number.isFinite(after.renderSerial) ? after.renderSerial : (before.renderSerial || 0),
    },
    more: !!f.more,
  };
  if (f.target) receipt.target = f.target;
  return receipt;
}

function renderReceipt(receipt) {
  const r = receipt || {};
  const o = r.observedAfter || {};
  const segments = [];
  if (o.urlChanged) segments.push(`URL changed to ${o.urlChanged}`);
  if (o.titleChanged) segments.push(`title changed to "${o.titleChanged}"`);
  if (o.mutations > 0) segments.push(`${o.mutations} DOM mutations`);
  if (o.renderAdvanced) segments.push("render advanced");
  if (o.changedElements && o.changedElements.length) {
    segments.push(`focus moved to ${o.changedElements.map((item) => `[${item.ref}] ${item.role} "${clip(item.name, 120)}"`).join(", ")}`);
  }
  if (o.alertOrStatus) segments.push(`alert/status appeared: "${o.alertOrStatus}"`);
  const observed = segments.length ? segments.join("; ") : "no meaningful page change";
  let text = `interaction receipt: observed after ${r.action || "action"}: ${observed}`;
  if (r.blockers && r.blockers.length) {
    text += `; blocked: ${r.blockers.map((b) => `${b.kind}: ${b.summary} Next: ${b.nextStep}`).join("; ")}`;
  }
  if (r.more) text += "; more facts were omitted; request the narrow target read named above";
  const limit = r.blockers && r.blockers.length ? FAILURE_LIMIT : SUCCESS_LIMIT;
  return clip(text, limit);
}

const GhostlightReceipt = { makeReceipt, renderReceipt };
if (typeof module !== "undefined" && module.exports) {
  module.exports = GhostlightReceipt;
} else {
  self.GhostlightReceipt = GhostlightReceipt;
}
})();
