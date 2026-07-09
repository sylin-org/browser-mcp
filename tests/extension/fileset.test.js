// SPDX-License-Identifier: Apache-2.0 OR MIT
// Node unit tests for extension/lib/fileset.js (ADR-0050 Decision 2: file_upload byte decoding).
// "aGVsbG8=" is base64 "hello" (5 bytes); "d29ybGQ=" is "world" (5 bytes); "eA==" is "x".

const { test } = require("node:test");
const assert = require("node:assert");
const { decodeFiles } = require("../../extension/lib/fileset.js");

test("single file decodes to bytes with default mime type", () => {
  const r = decodeFiles([{ data: "aGVsbG8=", name: "hello.txt" }]);
  assert.strictEqual(r.ok, true);
  assert.strictEqual(r.decoded.length, 1);
  assert.strictEqual(r.decoded[0].name, "hello.txt");
  assert.strictEqual(r.decoded[0].type, "application/octet-stream");
  assert.strictEqual(r.totalBytes, 5);
  assert.strictEqual(String.fromCharCode(...r.decoded[0].bytes), "hello");
});

test("multiple files accumulate totalBytes", () => {
  const r = decodeFiles([
    { data: "aGVsbG8=", name: "a.txt" },
    { data: "d29ybGQ=", name: "b.txt" },
  ]);
  assert.strictEqual(r.decoded.length, 2);
  assert.strictEqual(r.totalBytes, 10);
});

test("a file missing its name is rejected", () => {
  const r = decodeFiles([{ data: "eA==" }]);
  assert.strictEqual(r.ok, false);
  assert.strictEqual(r.error, "each file must have `data` and `name`");
});

test("an explicit mimeType is preserved", () => {
  const r = decodeFiles([{ data: "aGVsbG8=", name: "c.png", mimeType: "image/png" }]);
  assert.strictEqual(r.decoded[0].type, "image/png");
});
