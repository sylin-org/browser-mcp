// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- consequence digests (ADR-0037 Decision 2): after a mutating action the page is
// sampled for cheap, observable signals (URL change, title change, DOM mutation count, focus
// movement, newly appeared role=alert/status text, role=dialog presence). This module formats
// those signals into a single compact `observation:` block so the model learns what its action
// CAUSED without a separate verify read. Pure: no DOM, no chrome.*, no timers -- the caller
// (content.js) collects the raw signals and hands them here as a plain object.
//
// IIFE-wrapped and exposed as a single namespace per lib/constants.js's pattern (idempotent under
// MV3 worker re-evaluation; loadable as a content-script global via the manifest and under
// node --test).
(function () {
// The segment order is fixed (ADR-0037 D2, PINS.md SS10): url, title, mutations, focus, alert,
// status, dialog. Each present signal renders one segment; the whole is joined by "; " and
// prefixed "observation: ". When no signal is present the block is "observation: no observable
// change" -- a silent action is itself a signal the model needs. The rendered string is capped at
// 400 characters (truncated with "...").
function formatObservation(sig) {
  const s = sig || {};
  const segs = [];
  if (s.url) segs.push(`url changed to ${s.url}`);
  if (s.title) segs.push(`title changed to "${s.title}"`);
  if (s.mutations && s.mutations > 0) segs.push(`${s.mutations} DOM mutations`);
  if (s.focus) segs.push(`focus moved to "${s.focus}"`);
  if (s.alert) segs.push(`alert appeared: "${s.alert}"`);
  if (s.status) segs.push(`status appeared: "${s.status}"`);
  if (s.dialog) segs.push("dialog appeared");
  let out = segs.length ? `observation: ${segs.join("; ")}` : "observation: no observable change";
  if (out.length > 400) out = out.slice(0, 397) + "...";
  return out;
}

const GhostlightObservation = { formatObservation };
if (typeof module !== "undefined" && module.exports) {
  module.exports = GhostlightObservation;
} else {
  self.GhostlightObservation = GhostlightObservation;
}
})();
