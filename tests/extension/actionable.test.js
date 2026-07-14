// SPDX-License-Identifier: Apache-2.0 OR MIT
// Node unit tests for extension/lib/actionable.js (ADR-0078 D2).

const { test } = require("node:test");
const assert = require("node:assert");
const { makeSummary, matchRank, rankCandidates } = require("../../extension/lib/actionable.js");

test("semantic ranks are exact, prefix, tokens, then substring", () => {
  assert.strictEqual(matchRank("save", { name: "Save" }), 0);
  assert.strictEqual(matchRank("save", { name: "Save changes" }), 1);
  assert.strictEqual(matchRank("changes save", { name: "Save all changes" }), 2);
  assert.strictEqual(matchRank("ave", { name: "Save" }), 3);
  assert.strictEqual(matchRank("missing", { name: "Save" }), null);
});

test("ranking preserves document order inside a tier and filters role", () => {
  const ranked = rankCandidates("save", [
    { ref: "ref_1", role: "link", name: "Save draft" },
    { ref: "ref_2", role: "button", name: "Save copy" },
    { ref: "ref_3", role: "button", name: "Save" },
  ], "button");
  assert.deepStrictEqual(ranked.map((item) => [item.ref, item.matchRank]), [
    ["ref_3", 0],
    ["ref_2", 1],
  ]);
});

test("summary is bounded, sparse, and keeps structural secret marking", () => {
  const summary = makeSummary({
    ref: "ref_9",
    role: "textbox",
    name: "n".repeat(200),
    visible: true,
    enabled: false,
    checked: false,
    value: "v".repeat(200),
    secret: true,
    href: `https://example.com/${"h".repeat(300)}`,
    box: { x: 1.4, y: 2.6, width: -4, height: 9.8 },
    renderSerial: 7,
    mechanicalActions: ["scroll_to"],
  });
  assert.strictEqual(summary.name.length, 120);
  assert.strictEqual(summary.value.length, 120);
  assert.strictEqual(summary.href.length, 240);
  assert.deepStrictEqual(summary.box, { x: 1, y: 3, width: 0, height: 10 });
  assert.strictEqual(summary.secret, true);
  assert.strictEqual(summary.checked, false);
  assert.ok(!Object.hasOwn(summary, "selected"));
});
