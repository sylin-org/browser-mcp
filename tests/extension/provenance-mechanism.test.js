// SPDX-License-Identifier: Apache-2.0 OR MIT
// Extension contract for ADR-0078 D5: mechanism reports page facts; service owns trust markers.

const { test } = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const content = fs.readFileSync(path.join(root, "extension/content.js"), "utf8");
const worker = fs.readFileSync(path.join(root, "extension/service-worker.js"), "utf8");

test("extension reports origin and render serial without minting trust boundaries", () => {
  assert.match(content, /function currentPageMeta\(\)/);
  assert.match(content, /origin: location\.origin/);
  assert.match(content, /renderSerial/);
  assert.match(worker, /async function pageMeta\(tabId\)/);
  for (const source of [content, worker]) {
    assert.doesNotMatch(source, /GHOSTLIGHT PAGE CONTENT/);
    assert.doesNotMatch(source, /sessionNonce/);
    assert.doesNotMatch(source, /pageSourced/);
  }
});
