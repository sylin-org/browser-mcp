// SPDX-License-Identifier: Apache-2.0 OR MIT
// Node unit tests for extension/lib/keys.js (key/input tables and modifier bits).

const { test } = require("node:test");
const assert = require("node:assert");
const { modifierBits, vkCode, keyCode, charKeyInfo } = require("../../extension/lib/keys.js");

test("modifier bits match CDP values", () => {
  assert.strictEqual(modifierBits("ctrl"), 2);
  assert.strictEqual(modifierBits("alt"), 1);
  assert.strictEqual(modifierBits("shift"), 8);
  assert.strictEqual(modifierBits("meta"), 4);
  assert.strictEqual(modifierBits("ctrl+shift"), 10);
});

test("named virtual key codes", () => {
  assert.strictEqual(vkCode("Enter"), 13);
  assert.strictEqual(vkCode("Tab"), 9);
});

test("punctuation maps", () => {
  assert.strictEqual(vkCode(";"), 186);
  assert.strictEqual(keyCode(";"), "Semicolon");
});

test("charKeyInfo maps newline to Enter", () => {
  assert.strictEqual(charKeyInfo("\n").key, "Enter");
  assert.strictEqual(charKeyInfo("\r").key, "Enter");
});

test("charKeyInfo rejects control and non-ASCII", () => {
  assert.strictEqual(charKeyInfo("\u0001"), null);
  assert.strictEqual(charKeyInfo("\u00e9"), null);
});
