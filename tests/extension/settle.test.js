// SPDX-License-Identifier: Apache-2.0 OR MIT
// Node unit tests for extension/lib/settle.js (ADR-0037 Decision 5: the adaptive settle
// detector), PINS.md SS9.

const { test } = require("node:test");
const assert = require("node:assert");
const { settleThreshold, createSettleDetector } = require("../../extension/lib/settle.js");

test("settleThreshold adapts to the observed peak, floored at 3", () => {
  assert.strictEqual(settleThreshold(400), 20);
  assert.strictEqual(settleThreshold(100), 5);
  assert.strictEqual(settleThreshold(80), 4);
  assert.strictEqual(settleThreshold(61), 3);
  assert.strictEqual(settleThreshold(60), 3);
  assert.strictEqual(settleThreshold(59), 3);
  assert.strictEqual(settleThreshold(30), 3);
  assert.strictEqual(settleThreshold(0), 3);
});

// Feeds every count in order and reports the detector's final state: `settled` is the LAST
// push()'s return value (not necessarily the FIRST push that returned true -- once the
// consecutive-quiet run reaches 3 it only grows on further quiet windows, so later pushes in a
// still-quiet tail report the same true), alongside the trivially-final peak/lastRate/windows.
function feed(counts) {
  const d = createSettleDetector();
  let settled = false;
  for (const c of counts) {
    settled = d.push(c);
  }
  return { settled, peak: d.peak, lastRate: d.lastRate, windows: d.windows };
}

test("settles after 3 consecutive quiet windows (first window never a candidate)", () => {
  const r = feed([400, 200, 80, 15, 10, 2]);
  assert.strictEqual(r.settled, true);
  assert.strictEqual(r.peak, 400);
  assert.strictEqual(r.lastRate, 2);
  assert.strictEqual(r.windows, 6);
});

test("settles on a light, mostly-quiet page", () => {
  const r = feed([5, 1, 0, 0]);
  assert.strictEqual(r.settled, true);
  assert.strictEqual(r.peak, 5);
  assert.strictEqual(r.lastRate, 0);
  assert.strictEqual(r.windows, 4);
});

test("never settles under a sustained mutation rate", () => {
  const r = feed([10, 4, 4, 4, 4, 4, 4, 4]);
  assert.strictEqual(r.settled, false);
  assert.strictEqual(r.peak, 10);
  assert.strictEqual(r.lastRate, 4);
  assert.strictEqual(r.windows, 8);
});

test("recovers after a mid-run spike resets the quiet streak", () => {
  const r = feed([300, 2, 2, 100, 50, 10, 5, 2, 1]);
  assert.strictEqual(r.settled, true);
  assert.strictEqual(r.peak, 300);
  assert.strictEqual(r.lastRate, 1);
  assert.strictEqual(r.windows, 9);
});

test("settles on an entirely quiet page", () => {
  const r = feed([0, 0, 0, 0]);
  assert.strictEqual(r.settled, true);
  assert.strictEqual(r.peak, 0);
  assert.strictEqual(r.lastRate, 0);
  assert.strictEqual(r.windows, 4);
});
