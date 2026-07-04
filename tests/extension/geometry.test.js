// SPDX-License-Identifier: Apache-2.0 OR MIT
// Node unit tests for extension/lib/geometry.js (screenshot sizing and coordinate rescaling).

const { test } = require("node:test");
const assert = require("node:assert");
const { targetDims, zoomScale, rescaleCtxCoord } = require("../../extension/lib/geometry.js");

test("targetDims passes small viewports through", () => {
  assert.deepStrictEqual(targetDims(1280, 720), { w: 1280, h: 720 });
});

test("targetDims shrinks to the token budget", () => {
  assert.deepStrictEqual(targetDims(1920, 1080), { w: 1466, h: 824 });
});

test("targetDims clamps the longest side", () => {
  assert.deepStrictEqual(targetDims(4000, 100), { w: 1568, h: 39 });
});

test("targetDims never returns zero", () => {
  assert.deepStrictEqual(targetDims(1, 1), { w: 1, h: 1 });
});

test("zoomScale magnifies a small region within budget", () => {
  const s = zoomScale(100, 100);
  assert.ok(10.8 < s && s < 10.9, `s = ${s}`);
  assert.ok(Math.ceil(Math.round(100 * s) / 28) ** 2 <= 1568);
});

test("zoomScale shrinks a large region to the budget edge", () => {
  const s = zoomScale(2000, 1000);
  assert.strictEqual(Math.round(2000 * s), 1568);
  assert.strictEqual(Math.round(1000 * s), 784);
});

test("rescaleCtxCoord passthrough without context", () => {
  assert.deepStrictEqual(rescaleCtxCoord(null, 10.4, 20.6), [10, 21]);
});

test("rescaleCtxCoord maps screenshot px to viewport px", () => {
  assert.deepStrictEqual(
    rescaleCtxCoord({ vpW: 1280, vpH: 720, shotW: 1024, shotH: 576 }, 512, 288),
    [640, 360]
  );
});

test("rescaleCtxCoord adds zoom region offsets", () => {
  assert.deepStrictEqual(
    rescaleCtxCoord(
      { vpW: 1280, vpH: 720, shotW: 800, shotH: 600, offX: 100, offY: 50, regionW: 400, regionH: 300 },
      400,
      300
    ),
    [300, 200]
  );
});
