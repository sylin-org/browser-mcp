// SPDX-License-Identifier: Apache-2.0 OR MIT
// Node unit tests for extension/lib/observation.js (ADR-0037 Decision 2: consequence digests),
// PINS.md SS10.

const { test } = require("node:test");
const assert = require("node:assert");
const { formatObservation } = require("../../extension/lib/observation.js");

test("empty signal -> no observable change", () => {
  assert.strictEqual(formatObservation({}), "observation: no observable change");
});

test("the pinned multi-signal oracle", () => {
  assert.strictEqual(
    formatObservation({ url: "/dashboard", mutations: 47, focus: "Search", alert: "Changes saved" }),
    'observation: url changed to /dashboard; 47 DOM mutations; focus moved to "Search"; alert appeared: "Changes saved"'
  );
});

test("a long alert is capped at 400 chars with a trailing ellipsis", () => {
  const big = "x".repeat(500);
  const out = formatObservation({ alert: big });
  assert.ok(out.length <= 400, `expected <= 400 chars, got ${out.length}`);
  assert.ok(out.endsWith("..."), `expected trailing "...", got: ${JSON.stringify(out.slice(-10))}`);
});

test("segment order: url, title, mutations, focus, alert, status, dialog", () => {
  const out = formatObservation({
    url: "/u",
    title: "T",
    mutations: 9,
    focus: "F",
    alert: "A",
    status: "S",
    dialog: true,
  });
  // The single expected string, in the fixed segment order.
  assert.strictEqual(
    out,
    'observation: url changed to /u; title changed to "T"; 9 DOM mutations; ' +
      'focus moved to "F"; alert appeared: "A"; status appeared: "S"; dialog appeared'
  );
});
