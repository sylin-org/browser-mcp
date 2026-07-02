# T11: computer zoom: real region crop with coordinate-context update

## Goal

Make the `computer` tool's `zoom` action actually capture the requested region instead of
returning a full-viewport screenshot. The region is validated, rescaled from screenshot
space to CSS pixels, clamped to the visible viewport, captured magnified under the
existing token budget, and recorded in the per-tab ScreenshotContext so that coordinates
read off the zoomed image map back to the correct CSS position.

## Project context

Browser MCP is governed browser automation. A single Rust binary is both the MCP server
(JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host. A thin
Manifest V3 extension executes CDP commands. Architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS
IPC. The extension holds mechanism only: it receives `{ id, type: "tool_request", tool,
args }` over the native port, executes CDP commands, and replies. All policy, access, and
audit decisions live in the Rust binary.

Files relevant to this task:

- `extension/service-worker.js`: CDP dispatch, screenshot pipeline, coordinate rescaling.
  This is the ONLY file you will modify.
- `src/mcp/schemas/tools.json`: byte-frozen official tool schemas. Never touched. The
  `region` parameter already exists there (lines 87-93): an array of 4 numbers, described
  as "(x0, y0, x1, y1)" from top-left to bottom-right in pixels from the viewport origin,
  required for `zoom`.
- `tests/tool_schema_fidelity.rs`: guard test that must keep passing.
- `extension/content.js`, `extension/agent-visual-indicator.js`: DOM reads and the phantom
  cursor overlay. Not touched by this task.

Coordinate model background (comment at lines 65-69 of the service worker): there is NO
device-metrics override. Each screenshot probes the CSS viewport and DPR, captures at
native resolution, downscales to a token budget, and records a per-tab ScreenshotContext.
Model-provided coordinates are read off that downscaled image, so they are rescaled back
to CSS viewport pixels before input dispatch. Coordinates derived from element refs come
from getBoundingClientRect and are already CSS pixels; they are never rescaled.

Build and test: the extension is vanilla JS loaded unpacked; after editing it, the user
must reload the extension at chrome://extensions. The Rust binary is not rebuilt for this
task, but run `cargo test` from the repo root to confirm all tests still pass. If you do
rebuild for any reason and `target/debug/browser-mcp.exe` is locked by a running session,
rename it aside first (for example `mv target/debug/browser-mcp.exe
target/debug/browser-mcp.exe.old-1`) and rebuild.

## Current behavior

All facts below were verified in `extension/service-worker.js` (568 lines).

- Line 16: `const screenshotCtx = new Map();` with the comment
  `// tabId -> { vpW, vpH, shotW, shotH } (set on each screenshot)`.
- Line 70: budget constants
  `const PX_PER_TOKEN = 28, MAX_TOKENS = 1568, MAX_SIDE = 1568, MAX_SCREENSHOT_B64 = 1100000;`.
- `probeViewport(tabId)` at lines 72-80 evaluates
  `({w:innerWidth,h:innerHeight,d:window.devicePixelRatio||1})` and returns
  `{ vpW, vpH, dpr }`; it throws `Error("failed to probe viewport")` when the probe
  returns nothing usable. It does NOT return scroll offsets.
- `targetDims(vpW, vpH)` at lines 82-89 only shrinks: it returns the input dims unless the
  token count `Math.ceil(w / 28) * Math.ceil(h / 28)` exceeds `MAX_TOKENS` or the longest
  side exceeds `MAX_SIDE`. It never magnifies, so it cannot be called directly for zoom.
- `encodeJpeg(bitmap, w, h, quality)` at lines 100-106 draws the bitmap into an
  OffscreenCanvas of w x h and returns base64 JPEG.
- `rescaleCoord(tabId, x, y)` at lines 107-113: passthrough (rounded) when no context
  exists, otherwise `[Math.round((x * c.vpW) / c.shotW), Math.round((y * c.vpH) / c.shotH)]`.
  It has no notion of a region offset.
- `screenshot(tabId)` at lines 214-239: probes the viewport, hides the phantom cursor via
  `sendToTab(tabId, { type: "HIDE_FOR_TOOL_USE" })` plus `sleep(40)` (lines 219-220),
  captures `Page.captureScreenshot { format: "jpeg", quality: 80, captureBeyondViewport:
  false }` in a try whose finally sends `{ type: "SHOW_AFTER_TOOL_USE" }` (lines 221-226),
  then decodes with `createImageBitmap`, re-encodes at quality 0.55 with a 0.3 fallback
  when the base64 exceeds `MAX_SCREENSHOT_B64` (lines 230-236; raw capture kept if canvas
  APIs are unavailable), and records the context at line 237:
  `screenshotCtx.set(tabId, { vpW, vpH, shotW, shotH });`.
- The `computer(a)` handler starts at line 357. The zoom case is lines 366-367:

      case "zoom":
        return textImage(`Zoom region ${JSON.stringify(a.region || [])} (jpeg).`, await screenshot(tabId));

  It echoes the region but ignores it completely and returns a full-viewport screenshot,
  which defeats the tool: the model asked for a close-up and gets the same image again.
- Other `rescaleCoord` consumers: `resolveCoords` at line 280 and the drag endpoints at
  lines 425-426. `resize_window` (lines 537-550) deletes the ScreenshotContext for tabs in
  the resized window; `chrome.tabs.onRemoved` deletes it at line 132.
- Result helpers: `text(t)` at lines 207-209, `textImage(t, base64)` at lines 210-212.
- The CDP helper is `cdp(tabId, method, params)` at lines 114-117 (auto-attaches);
  `ensureAttached` is at lines 55-64; `sendToTab` at lines 246-248; `sleep` at lines
  242-244.

## Required behavior

Rework the zoom action so it captures exactly the requested region, magnified to make best
use of the token budget, and so that coordinates read off the zoomed image are mapped back
correctly by the existing coordinate machinery. Zoom region coordinates arrive in the
coordinate space of the LAST screenshot returned for that tab (full or zoomed), exactly
like click coordinates do.

### 1. Validation in the zoom case of `computer(a)`

Replace the body of the zoom case (lines 366-367) with:

    case "zoom": {
      const r = a.region;
      if (!Array.isArray(r) || r.length !== 4 || !r.every((v) => Number.isFinite(v)))
        return text("region [x0, y0, x1, y1] is required for zoom.");
      if (!(r[2] > r[0]) || !(r[3] > r[1]))
        return text("zoom region is empty: x1 must be greater than x0 and y1 must be greater than y0.");
      const z = await zoomScreenshot(tabId, r);
      if (z.error) return text(z.error);
      return textImage(`Zoom region (${z.x0}, ${z.y0}) -> (${z.x1}, ${z.y1}) captured (jpeg${z.clamped ? "; clamped to the visible viewport" : ""}).`, z.base64);
    }

The three error strings and the success string are exact; do not reword them. The echoed
x0/y0/x1/y1 are the final CSS-pixel integers that were actually captured (after rescale
and clamp), so the result text is truthful about what the image shows. Validation errors
are returned as normal text results (matching the existing "text is required for type."
style), not thrown.

### 2. New helper `zoomScale(w, h)`

Add a small function next to `targetDims` (after line 89). Given the region's CSS
dimensions, it returns the largest capture scale that keeps the output inside the existing
budget: token count `ceil(outW / 28) * ceil(outH / 28) <= 1568` and longest side
`<= 1568`. Reuse the existing constants; introduce no new numeric constants except the
0.98 correction factor.

    function zoomScale(w, h) {
      let s = Math.min(MAX_SIDE / Math.max(w, h), Math.sqrt((MAX_TOKENS * PX_PER_TOKEN * PX_PER_TOKEN) / (w * h)));
      while (s > 0 && Math.ceil(Math.round(w * s) / PX_PER_TOKEN) * Math.ceil(Math.round(h * s) / PX_PER_TOKEN) > MAX_TOKENS) s *= 0.98;
      return s;
    }

The while loop corrects the continuous estimate for the ceil() granularity; it terminates
because s decreases monotonically. For a small region this magnifies (s well above 1); for
a region near full viewport size on a large display it downscales (s below 1). Both are
correct.

### 3. New function `zoomScreenshot(tabId, region)`

Add it directly after `screenshot(tabId)` (after line 239). It returns either
`{ error: "..." }` or `{ base64, x0, y0, x1, y1, clamped }`. Steps, in order:

1. `await ensureAttached(tabId);`
2. Probe viewport AND scroll offsets in one evaluate (do not modify `probeViewport`):
   evaluate the expression
   `({w:innerWidth,h:innerHeight,sx:window.scrollX||0,sy:window.scrollY||0})` via
   `Runtime.evaluate` with `returnByValue: true`. If the result has no usable `w`/`h`,
   throw `new Error("failed to probe viewport")` (same guard style as `probeViewport`).
   Call the results `vpW`, `vpH`, `sx`, `sy`.
3. Rescale the region corners from screenshot space to CSS pixels using the tab's CURRENT
   ScreenshotContext, exactly like click coordinates:
   `const [rx0, ry0] = rescaleCoord(tabId, region[0], region[1]);` and
   `const [rx1, ry1] = rescaleCoord(tabId, region[2], region[3]);`.
   This MUST happen before the context is overwritten in step 9, so that a zoom issued
   against a previous zoomed screenshot composes correctly (chained zooms). When no
   screenshot has ever been taken for the tab, `rescaleCoord` passes the values through
   rounded, which treats them as CSS pixels; that existing behavior is kept.
4. Clamp to the viewport: x values to `[0, vpW]`, y values to `[0, vpH]`, producing
   `x0, y0, x1, y1`. Set `clamped` to true when any clamped value differs from its
   rescaled input.
5. `const w = x1 - x0, h = y1 - y0;` If `w < 1 || h < 1`, return
   `{ error: "zoom region is empty or entirely outside the visible viewport." }` (exact
   string). This covers regions fully outside the viewport and regions that collapse to
   nothing after rescale.
6. `const s = zoomScale(w, h);`
7. Hide the overlays and capture, mirroring the full-screenshot rhythm: send
   `{ type: "HIDE_FOR_TOOL_USE" }` via `sendToTab`, `await sleep(40)`, then inside a try
   whose finally sends `{ type: "SHOW_AFTER_TOOL_USE" }` (not awaited, same as line 225),
   call:

       cap = await cdp(tabId, "Page.captureScreenshot", {
         format: "jpeg", quality: 80,
         clip: { x: sx + x0, y: sy + y0, width: w, height: h, scale: s },
         captureBeyondViewport: false,
       });

   The clip coordinates for `Page.captureScreenshot` are document-relative CSS pixels,
   not viewport-relative, so the scroll offsets `sx`/`sy` must be added. The `scale`
   field multiplies CSS pixels to output pixels, so the output image is approximately
   `round(w * s)` x `round(h * s)` pixels.
8. Re-encode under the size budget, mirroring lines 230-236: compute fallback dims
   `let shotW = Math.max(1, Math.round(w * s)), shotH = Math.max(1, Math.round(h * s));`
   and `let base64 = cap.data;`. Then in a try: `createImageBitmap` from the capture
   bytes (reuse `bytesFromBase64`), re-encode with
   `encodeJpeg(bitmap, bitmap.width, bitmap.height, 0.55)`, fall back to quality 0.3 when
   the base64 length exceeds `MAX_SCREENSHOT_B64`, set `shotW`/`shotH` from the bitmap's
   actual `width`/`height`, and close the bitmap. On catch, keep the raw capture and the
   fallback dims (OffscreenCanvas/createImageBitmap unavailable), as the full-screenshot
   path does.
9. Record the zoomed context:
   `screenshotCtx.set(tabId, { vpW, vpH, shotW, shotH, offX: x0, offY: y0, regionW: w, regionH: h });`
10. Return `{ base64, x0, y0, x1, y1, clamped }`.

### 4. Extend `rescaleCoord` (lines 107-113)

Generalize the mapping so a coordinate read off a zoomed screenshot maps back through the
region offset. Replace the function body with:

    function rescaleCoord(tabId, x, y) {
      const c = screenshotCtx.get(tabId);
      if (!c || !c.shotW || !c.shotH) return [Math.round(x), Math.round(y)];
      const rw = c.regionW || c.vpW, rh = c.regionH || c.vpH;
      return [Math.round((c.offX || 0) + (x * rw) / c.shotW), Math.round((c.offY || 0) + (y * rh) / c.shotH)];
    }

For a full screenshot (offset 0, region = viewport) this is numerically identical to the
old formula, so clicks after a full screenshot behave exactly as before. Update the
comment above the function (lines 107-108) with one added line noting that zoomed captures
carry a region offset. The `|| c.vpW` fallbacks keep the function safe against a context
object without the new fields.

### 5. Full-screenshot bookkeeping (the only change to `screenshot()`)

Change line 237 to store the new fields with the offset reset to zero:

    screenshotCtx.set(tabId, { vpW, vpH, shotW, shotH, offX: 0, offY: 0, regionW: vpW, regionH: vpH });

This is what makes a subsequent full screenshot (including the one returned by the scroll
action at line 414) reset the zoom offset. Nothing else in `screenshot()` changes. Also
update the map-shape comment on line 16 to
`// tabId -> { vpW, vpH, shotW, shotH, offX, offY, regionW, regionH } (set on each screenshot/zoom)`.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged. The `region` parameter
   already exists in the schema; no schema work is needed or allowed.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS.
3. ASCII only in ALL code and docs: no em-dashes, no arrows other than the ASCII "->"
   already used in result strings, no curly quotes, anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text. This is why the
   result text echoes the clamped CSS region and carries the "clamped to the visible
   viewport" note, and why invalid regions produce explicit error texts instead of a
   silent full-viewport screenshot.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on public
   items, rustfmt clean, clippy with deny warnings. (This task should not require touching
   Rust at all.)
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style. Appropriate here: one short comment on `zoomScale` (magnify or
   shrink to the token budget), one on the clip scroll-offset addition (clip is
   document-relative), and the one-line additions to the line 16 and `rescaleCoord`
   comments. Do not comment every step.
8. Do NOT copy code from the official Anthropic extension or any other project; implement
   the behavior described above from scratch.

Task-specific:

9. The only file modified is `extension/service-worker.js`.
10. Exactly two new module-level functions: `zoomScale` and `zoomScreenshot`. No new
    module-level state, no new constants beyond what section 2 specifies.
11. Do not modify `probeViewport`, `targetDims`, `encodeJpeg`, `bytesFromBase64`,
    `base64FromBytes`, or any capture parameter of the full-screenshot path.
12. The rescale of the incoming region (step 3) must use the context as it was BEFORE this
    zoom, and the new context must only be written after a successful capture.

## Verification

1. Run `cargo test` from the repo root. All tests must pass (this change is
   extension-only, but the schema guard must stay green).
2. Ask the user to reload the extension at chrome://extensions (extension changes are not
   picked up otherwise). Binary and schema were not changed, so no MCP client restart is
   needed.
3. Manual checks through an MCP client driving the extension, on a content-rich page (for
   example a Wikipedia article):
   - `computer` `screenshot`, then `computer` `zoom` with a region around a small element
     (coordinates read off that screenshot): the returned image shows ONLY that region,
     visibly magnified, not the full viewport.
   - `zoom` without a region: exact text
     "region [x0, y0, x1, y1] is required for zoom."
   - `zoom` with `region: [200, 200, 100, 300]`: exact text
     "zoom region is empty: x1 must be greater than x0 and y1 must be greater than y0."
   - `zoom` with a region far outside the page (for example
     `[9000, 9000, 9500, 9500]`): exact text
     "zoom region is empty or entirely outside the visible viewport."
   - `zoom` with a region that straddles the viewport edge: image is captured and the
     result text ends with "(jpeg; clamped to the visible viewport)." and echoes the
     clamped coordinates.
   - After a zoom, `left_click` with a coordinate read off the ZOOMED image: the click
     lands on the element visible at that point in the zoomed image (offset mapping).
   - Zoom again with a region read off the zoomed image (chained zoom): the new image is
     the correct sub-region.
   - `computer` `screenshot` (full), then click with a coordinate read off the full image:
     mapping is back to normal (offset reset).
   - Scroll the page down first, then screenshot and zoom: the zoomed image matches what
     is visible on screen (scroll offset handled in the clip).
4. Open the service worker console (chrome://extensions, "Inspect views: service worker")
   and confirm no errors are thrown during the actions above.

## Out of scope

- The full-screenshot path: no changes to `screenshot()` other than the single
  `screenshotCtx.set` line in section 5, and no changes to `probeViewport`, `targetDims`,
  capture quality values, or `captureBeyondViewport` on that path.
- Any schema edit: `region` is already in `src/mcp/schemas/tools.json`; the schema file,
  tool names, and descriptions are frozen.
- No device-metrics or emulation overrides (`Emulation.setDeviceMetricsOverride`); the
  probe-and-rescale coordinate model stands.
- No zooming beyond the visible viewport: do not set `captureBeyondViewport: true` and do
  not scroll the page to reach an off-screen region.
- No caching or cropping of previous captures; every zoom performs a fresh
  `Page.captureScreenshot` with a clip.
- Do not change `resolveCoords`, the drag path, the scroll action's trailing screenshot,
  `resize_window`'s context clearing, or any result text other than the zoom case's.
- Do not touch `extension/content.js`, `extension/agent-visual-indicator.js`, or any Rust
  source.
- Do not rename existing functions or constants.
