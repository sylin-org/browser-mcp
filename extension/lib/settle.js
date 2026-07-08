// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- the adaptive settle detector (ADR-0037 Decision 5): a rate-of-change decay
// evaluator over a per-window DOM-mutation counter. Pure: no DOM, no timers, no chrome.* -- the
// caller (content.js) bins its own MutationObserver counter into 500ms windows and feeds each
// window's count through push(); this module only tracks state and decides when the page has
// gone quiet.
//
// IIFE-wrapped and exposed as a single namespace per lib/constants.js's pattern (idempotent
// under MV3 worker re-evaluation; loadable both as a content-script global and under node --test).
(function () {
// The adaptive threshold (ADR-0037 Decision 5): a heavy page (high peak) tolerates a higher
// mutation rate before being called quiet; a light page's floor is 3 (absorbs low-frequency
// background updates like an SSE tick adding 1-2 nodes every window).
function settleThreshold(peak) {
  return Math.max(Math.floor(peak * 0.05), 3);
}

// A fresh detector: no windows observed yet. `push(count)` feeds one window's mutation count
// (in arrival order) and returns whether the page is settled AS OF THIS PUSH: the mutation rate
// has been below the adaptive threshold for 3 consecutive windows. The very first pushed window
// never counts as a settlement candidate (there is nothing to compare a rate decay against
// yet); any non-quiet window resets the consecutive-quiet count to zero.
function createSettleDetector() {
  let peak = 0;
  let lastRate = 0;
  let windows = 0;
  let quietRun = 0;
  return {
    push(count) {
      windows += 1;
      lastRate = count;
      if (count > peak) peak = count;
      if (windows === 1) {
        quietRun = 0;
        return false;
      }
      if (count < settleThreshold(peak)) {
        quietRun += 1;
      } else {
        quietRun = 0;
      }
      return quietRun >= 3;
    },
    get peak() {
      return peak;
    },
    get lastRate() {
      return lastRate;
    },
    get windows() {
      return windows;
    },
  };
}

const GhostlightSettle = { settleThreshold, createSettleDetector };
if (typeof module !== "undefined" && module.exports) {
  module.exports = GhostlightSettle;
} else {
  self.GhostlightSettle = GhostlightSettle;
}
})();
