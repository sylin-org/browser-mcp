// SPDX-License-Identifier: Apache-2.0 OR MIT

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const directory = path.dirname(fileURLToPath(import.meta.url));
const fixture = readFileSync(path.join(directory, "free-surface-fixture.html"), "utf8");
const runner = readFileSync(path.join(directory, "run-smoke.mjs"), "utf8");

test("free-surface fixture is ASCII and exposes every pinned journey", () => {
  assert.doesNotMatch(fixture, /[^\x00-\x7F]/);
  for (const journey of ["toolbar", "form", "viewport", "product"]) {
    assert.match(fixture, new RegExp(`data-journey="${journey}"`));
  }
  for (const product of ["alpha", "beta", "gamma"]) {
    assert.match(fixture, new RegExp(`${product}: \\{`));
  }
});

test("fixture ids are unique and the dense toolbar stays dense", () => {
  const ids = [...fixture.matchAll(/\sid="([^"]+)"/g)].map((match) => match[1]);
  assert.equal(new Set(ids).size, ids.length, "fixture element ids must be unique");

  const toolbar = /<div class="toolbar"[\s\S]*?<\/div>/.exec(fixture);
  assert.ok(toolbar, "toolbar fixture is present");
  assert.equal((toolbar[0].match(/<button\b/g) || []).length, 8);
  assert.match(toolbar[0], /aria-label="Review changes"/);
});

test("baseline mode is opt-in and reports both candidate baselines", () => {
  assert.match(runner, /process\.argv\.includes\("--free-surface-baseline"\)/);
  assert.match(runner, /mode: "free-surface-baseline"/);
  assert.match(runner, /candidateA:/);
  assert.match(runner, /candidateB:/);
  assert.match(runner, /currentShape: "computer screenshot plus read_page"/);
  assert.match(runner, /currentShape: "numeric composite tab ids"/);
});
