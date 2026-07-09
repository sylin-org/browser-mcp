// SPDX-License-Identifier: MIT
// Tests for the vendored NeuQuant quantizer (extension/lib/neuquant.js). NeuQuant is the reference
// standard (gif.js's TypedNeuQuant); these pin the contract gifenc.js relies on: a deterministic
// 256-entry palette, lookup indices in range, and that a dominant color is represented in the palette.

const test = require("node:test");
const assert = require("node:assert");
const { NeuQuant } = require("../../extension/lib/neuquant.js");

// Build a packed RGB buffer (3 bytes/pixel) from a list of [r,g,b] repeated `reps` times.
function rgb(colors, reps) {
  const buf = new Uint8Array(colors.length * reps * 3);
  let k = 0;
  for (let n = 0; n < reps; n++) {
    for (const [r, g, b] of colors) {
      buf[k++] = r; buf[k++] = g; buf[k++] = b;
    }
  }
  return buf;
}

test("buildColormap yields a 256-entry palette", () => {
  const nq = new NeuQuant(rgb([[10, 20, 30], [200, 100, 50]], 300), 10);
  nq.buildColormap();
  const map = nq.getColormap();
  assert.strictEqual(map.length, 256 * 3, "768 channel values");
  for (const v of map) assert.ok(v >= 0 && v <= 255, "channel in 0..255");
});

test("is deterministic: identical input -> identical palette", () => {
  const pixels = rgb([[255, 0, 0], [0, 128, 255], [12, 200, 64]], 500);
  const a = new NeuQuant(pixels, 10); a.buildColormap();
  const b = new NeuQuant(pixels, 10); b.buildColormap();
  assert.deepStrictEqual(a.getColormap(), b.getColormap());
});

test("lookupRGB returns an in-range index for a trained color", () => {
  const nq = new NeuQuant(rgb([[240, 16, 16]], 1000), 10);
  nq.buildColormap();
  const idx = nq.lookupRGB(240, 16, 16);
  assert.ok(Number.isInteger(idx) && idx >= 0 && idx < 256, "index in 0..255");
  const map = nq.getColormap();
  const dr = map[idx * 3] - 240, dg = map[idx * 3 + 1] - 16, db = map[idx * 3 + 2] - 16;
  assert.ok(Math.sqrt(dr * dr + dg * dg + db * db) < 96, "nearest palette entry is close to the trained color");
});

test("tolerates an empty training buffer (valid default palette, no crash)", () => {
  const nq = new NeuQuant(new Uint8Array(0), 10);
  nq.buildColormap();
  assert.strictEqual(nq.getColormap().length, 256 * 3);
  const idx = nq.lookupRGB(0, 0, 0);
  assert.ok(idx >= 0 && idx < 256);
});
