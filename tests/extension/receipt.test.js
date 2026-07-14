// SPDX-License-Identifier: Apache-2.0 OR MIT
// Node unit tests for extension/lib/receipt.js (ADR-0078 D1/D4).

const { test } = require("node:test");
const assert = require("node:assert");
const { makeReceipt, renderReceipt } = require("../../extension/lib/receipt.js");

test("receipt reports bounded observed facts without causal language", () => {
  const receipt = makeReceipt({
    tabId: 7,
    action: "left_click",
    targetAssurance: "ref",
    target: { ref: "ref_1", role: "button", name: "Save" },
    before: { url: "https://example.com/edit", title: "Edit", renderSerial: 2 },
    after: {
      url: "https://example.com/done",
      title: "Done",
      mutations: 12,
      renderSerial: 3,
      alert: "Saved",
      changedElements: [{ ref: "ref_2", role: "status", name: "Saved" }],
    },
  });
  assert.strictEqual(receipt.page.origin, "https://example.com");
  assert.strictEqual(receipt.targetAssurance, "ref");
  assert.strictEqual(receipt.observedAfter.renderAdvanced, true);
  const text = renderReceipt(receipt);
  assert.match(text, /^interaction receipt: observed after left_click:/);
  assert.ok(!/caused|committed|completed|verified/i.test(text), text);
  assert.ok(text.length <= 800);
});

test("dialog becomes a bounded progressive blocker", () => {
  const receipt = makeReceipt({
    tabId: 1,
    action: "left_click",
    before: { url: "https://example.com", title: "A", renderSerial: 1 },
    after: { url: "https://example.com", title: "A", renderSerial: 1, mutations: 0, dialogOpened: true },
  });
  assert.deepStrictEqual(receipt.blockers.map((item) => item.kind), ["dialog_open"]);
  const text = renderReceipt(receipt);
  assert.match(text, /resolve the dialog explicitly/);
  assert.ok(text.length <= 1200);
});

test("receipt caps changed elements and blocker payloads", () => {
  const many = Array.from({ length: 8 }, (_, i) => ({ ref: `ref_${i}`, role: "button", name: "x".repeat(300) }));
  const receipt = makeReceipt({
    after: { changedElements: many },
    blockers: Array.from({ length: 8 }, () => ({ kind: "covered_target", summary: "s".repeat(400), nextStep: "n".repeat(400) })),
  });
  assert.strictEqual(receipt.observedAfter.changedElements.length, 3);
  assert.strictEqual(receipt.blockers.length, 3);
  assert.ok(receipt.blockers.every((item) => item.summary.length <= 200 && item.nextStep.length <= 200));
});
