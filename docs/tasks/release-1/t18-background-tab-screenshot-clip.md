# T18: Screenshot of non-visible tabs via clip+scale single-pass capture

## Goal

Make the screenshot pipeline capture non-visible tabs (background tabs in the Browser MCP
group, or tabs in minimized windows) with a clipped, pre-scaled `Page.captureScreenshot`
that downscales in one pass inside the browser, instead of the canvas re-encode used today.
Visible tabs keep the current path unchanged. When background capture fails, the engine
falls back to the standard path and says so in the result text instead of silently
returning a possibly blank image.

## Project context

Browser MCP is governed browser automation. A single Rust binary is both the MCP server
(JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host; a thin
Manifest V3 extension executes CDP commands. Architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS IPC.

Files relevant to this task:

- `extension/service-worker.js`: CDP dispatch, the screenshot pipeline, the `computer`
  action switch, coordinate rescaling. This is the ONLY file you will modify.
- `extension/content.js` and `extension/agent-visual-indicator.js`: DOM reads and the
  phantom-cursor overlay. Do not touch them in this task.
- `src/mcp/schemas/tools.json`: byte-frozen official tool schemas. Never edit.
- `tests/tool_schema_fidelity.rs`: guard test that fails if the schema drifts.

Coordinate model background (already implemented, do not change it): each screenshot probes
the CSS viewport and devicePixelRatio, captures, downscales to a token budget, and records a
per-tab ScreenshotContext `{ vpW, vpH, shotW, shotH }`. Model-provided coordinates (read off
the downscaled screenshot) are rescaled back to CSS viewport pixels before input dispatch.
This task must keep that contract exactly: after your change, the context recorded for a
non-visible tab must hold the same CSS viewport dims and the same final pixel dims that the
visible path would have recorded for the same viewport.

Build and test: run `cargo test` from the repo root; all tests must pass. This task changes
only extension JavaScript, so no Rust rebuild is required, but run `cargo test` anyway to
confirm nothing else broke. Extension changes take effect after the user reloads the
extension at chrome://extensions. Binary or schema changes would require an MCP client
restart, but this task makes none. If you ever do need to rebuild and
`target/debug/browser-mcp.exe` is locked by a running session, rename it aside first
(for example: `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and
rebuild.

## Current behavior

All line numbers verified against the current tree; if other tasks have shifted them,
locate the same code by the function names given here.

The screenshot pipeline lives in `extension/service-worker.js`:

- Line 16: `const screenshotCtx = new Map();` maps tabId to `{ vpW, vpH, shotW, shotH }`,
  set on each screenshot. It is cleared on tab removal (line 132) and for a whole window on
  `resize_window` (lines 543-548).
- Line 70: budget constants:
  `const PX_PER_TOKEN = 28, MAX_TOKENS = 1568, MAX_SIDE = 1568, MAX_SCREENSHOT_B64 = 1100000;`
- `probeViewport(tabId)` at lines 72-80: one `Runtime.evaluate` with the expression
  `"({w:innerWidth,h:innerHeight,d:window.devicePixelRatio||1})"` and
  `returnByValue: true`; returns `{ vpW, vpH, dpr }` or throws
  `new Error("failed to probe viewport")` when the value is missing. Its only caller is
  `screenshot` (line 217).
- `targetDims(vpW, vpH)` at lines 82-89: shrinks the CSS viewport dims under the token
  budget (`Math.ceil(w / PX_PER_TOKEN) * Math.ceil(h / PX_PER_TOKEN) <= MAX_TOKENS`) and the
  longest-side cap (`MAX_SIDE`), returning `{ w, h }`. Never grows; when the viewport is
  already under budget it returns the viewport dims unchanged.
- `bytesFromBase64` (lines 90-94), `base64FromBytes` (lines 95-99), and
  `encodeJpeg(bitmap, w, h, quality)` (lines 100-106, OffscreenCanvas + `convertToBlob`)
  implement the canvas downscale.
- `rescaleCoord(tabId, x, y)` at lines 109-113 consumes the ScreenshotContext:
  `Math.round((x * c.vpW) / c.shotW)` per axis, passthrough when no context exists.
- `screenshot(tabId)` at lines 215-239 is the single capture entry point:
  1. `await ensureAttached(tabId);`
  2. `const { vpW, vpH, dpr } = await probeViewport(tabId);`
  3. Hides the phantom cursor: `await sendToTab(tabId, { type: "HIDE_FOR_TOOL_USE" });`
     then `await sleep(40);` (lines 219-220).
  4. Line 223, inside a try:
     `cap = await cdp(tabId, "Page.captureScreenshot", { format: "jpeg", quality: 80, captureBeyondViewport: false });`
     with a finally at line 225 that fires `sendToTab(tabId, { type: "SHOW_AFTER_TOOL_USE" });`
     (not awaited). Note: no `clip`, no explicit `fromSurface`.
  5. Line 227: `const { w, h } = targetDims(vpW, vpH);`
  6. Lines 229-236: defaults to the raw capture
     (`shotW = Math.round(vpW * dpr), shotH = Math.round(vpH * dpr)`), then tries the canvas
     downscale: `createImageBitmap` on the decoded JPEG, `encodeJpeg(bitmap, w, h, 0.55)`,
     re-encode at `0.3` when `base64.length > MAX_SCREENSHOT_B64`, then
     `shotW = w; shotH = h;`. On any canvas failure it keeps the raw capture.
  7. Line 237: `screenshotCtx.set(tabId, { vpW, vpH, shotW, shotH });`
  8. Returns the base64 string.
- Call sites of `screenshot(tabId)`, all in the `computer` function:
  - Line 365: `return textImage("Screenshot captured (jpeg).", await screenshot(tabId));`
  - Line 367: `` return textImage(`Zoom region ${JSON.stringify(a.region || [])} (jpeg).`, await screenshot(tabId)); ``
  - Line 414 (scroll): `` return textImage(`Scrolled ${dir} by ${amount}.`, await screenshot(tabId)); ``
- `sendToTab` at lines 246-248 is best-effort (swallows errors; content script may be absent
  on chrome:// pages).
- `dispatch` at lines 558-566 catches any handler exception and sends a native
  `tool_error` message with the string `` `${tool} failed: ${(e && e.message) || e}` ``.

The problems this task fixes:

- The capture never passes a `clip` and never checks whether the tab is visible. On a
  non-visible tab (a background tab of the MCP group, common in multi-tab flows, or a tab in
  a minimized window) `Page.captureScreenshot` can return a blank or stale frame, and the
  code happily canvas-downscales it and returns it as if it were fresh. Nothing in the
  result text warns the model.
- Every capture, even a perfectly good one, pays a decode + OffscreenCanvas + re-encode
  pass. For non-visible tabs the browser can do the downscale itself in the capture call
  (clip with a scale factor), skipping the canvas entirely.

There is no occurrence of `fromSurface` or `clip` anywhere in the extension today
(verified by search).

## Required behavior

Three changes to `extension/service-worker.js`, nothing else.

### 1. probeViewport also reports visibility

Extend the evaluated expression to
`"({w:innerWidth,h:innerHeight,d:window.devicePixelRatio||1,vis:document.visibilityState})"`
and change the return value to:

    { vpW: v.w, vpH: v.h, dpr: v.d || 1, visible: (v.vis || "visible") === "visible" }

The existing throw on a missing/invalid value stays exactly as is. Any `visibilityState`
other than `"visible"` (for example `"hidden"`) counts as not visible; a missing value
counts as visible so that pages without the API keep today's behavior. Chrome reports
`"hidden"` both for background tabs and for tabs in minimized windows, so one probe covers
both cases; this is also more accurate than `chrome.tabs.get(...).active`, which cannot see
window minimization.

### 2. screenshot() gains a clipped single-pass path and a truthful fallback

Change the return type of `screenshot(tabId)` from a base64 string to an object
`{ base64, note }`, where `note` is `""` on every clean path. The new algorithm, in order:

1. `await ensureAttached(tabId);`
2. `const { vpW, vpH, dpr, visible } = await probeViewport(tabId);`
3. `const { w, h } = targetDims(vpW, vpH);` (moved up; it is needed by both paths).
4. Hide the overlay exactly as today: `await sendToTab(tabId, { type: "HIDE_FOR_TOOL_USE" });`
   then `await sleep(40);`. Keep this for BOTH paths: the phantom cursor is a DOM element
   and would otherwise appear in a background capture too.
5. Capture phase, wrapped in a try whose finally fires
   `sendToTab(tabId, { type: "SHOW_AFTER_TOOL_USE" });` exactly once, not awaited, after the
   last capture attempt has settled and before any canvas work. This preserves the current
   semantics (the indicator returns as soon as pixels are captured, not after re-encoding).
   Inside the try:
   - If `visible` is false, attempt the clipped single-pass capture:
     - `const scale = w / vpW;` (always <= 1, because `targetDims` never grows).
     - `cap = await cdp(tabId, "Page.captureScreenshot", { format: "jpeg", quality: 55, clip: { x: 0, y: 0, width: vpW, height: vpH, scale }, fromSurface: true, captureBeyondViewport: false });`
       The clip rect is the CSS viewport; `scale` makes the browser emit the downscaled
       image directly, so no canvas pass is needed. `fromSurface: true` reads from the
       compositing surface, which is what makes capture of a non-presented tab work.
     - If `cap.data.length > MAX_SCREENSHOT_B64`, re-capture with the identical parameters
       except `quality: 30`. This mirrors the visible path's 0.55 -> 0.3 canvas quality
       ladder (CDP quality is an integer 0-100; canvas quality is a fraction 0-1).
     - On success: `screenshotCtx.set(tabId, { vpW, vpH, shotW: w, shotH: h });` and return
       `{ base64: cap.data, note: "" }`. Recording `w`/`h` is required so `rescaleCoord`
       maps coordinates identically to the visible path. The actual encoded image may
       differ from `w x h` by at most one pixel per axis due to scale rounding; accept
       this. Do NOT decode the image to measure it (that would reintroduce the canvas pass
       this task removes).
     - If ANY of these clipped-path CDP calls rejects (failure looks like a rejected
       promise from the `cdp` helper carrying the protocol error text, typically
       "Unable to capture screenshot"): remember the error message as
       `clipMsg = (e && e.message) || String(e)` and fall through to the standard capture
       below. Do not return yet; do not rethrow yet.
   - Standard capture (runs for visible tabs, and as the fallback after a clipped-path
     failure): `cap = await cdp(tabId, "Page.captureScreenshot", { format: "jpeg", quality: 80, captureBeyondViewport: false });`
     exactly as today. If THIS call rejects:
     - When it was the fallback (a clipped-path failure preceded it), throw
       `` new Error(`screenshot of non-visible tab failed: clipped capture: ${clipMsg}; fallback capture: ${fbMsg}`) ``
       where `fbMsg` is derived the same way as `clipMsg`. `dispatch` will surface this as
       `computer failed: screenshot of non-visible tab failed: ...`, which is the required
       truthful hard-failure: no image is returned at all.
     - When the tab was visible (no clipped attempt happened), let the rejection propagate
       unchanged, as today.
6. After the capture phase (standard-capture results only; the clipped path already
   returned): run the existing canvas downscale block byte-for-byte as it is today (raw
   default of `Math.round(vpW * dpr)` x `Math.round(vpH * dpr)`, `encodeJpeg` at 0.55,
   re-encode at 0.3 over the size cap, silent keep-raw on canvas failure), then
   `screenshotCtx.set(tabId, { vpW, vpH, shotW, shotH });` as today.
7. Return `{ base64, note }` where `note` is the exact string below when this standard
   capture was reached as the fallback from a failed clipped attempt, and `""` otherwise:

       Warning: this tab was not visible and direct background capture failed; the image was taken with the standard capture path and may be blank or stale.

   This is the honest middle outcome: the fallback capture technically succeeded, but on a
   non-visible tab its content cannot be trusted, and blank-frame detection is out of scope,
   so the engine flags it instead of pretending.

To summarize the three outcomes for a non-visible tab:

- Clipped capture works: normal image, empty note, no warning (nothing failed, nothing was
  substituted).
- Clipped capture fails, standard capture works: image plus the warning note above.
- Both fail: thrown error with both messages; the client sees a `tool_error`.

### 3. Callers append the note to their captions

Update EVERY call site of `screenshot(tabId)` in the file (currently the three listed in
Current behavior at lines 365, 367, and 414; if other release-1 tasks have added more
screenshot-returning paths, update those identically). The pattern at each site:

    const shot = await screenshot(tabId);
    return textImage(shot.note ? caption + " " + shot.note : caption, shot.base64);

where `caption` is the existing caption string for that site, byte-identical to today:
`"Screenshot captured (jpeg)."`, `` `Zoom region ${JSON.stringify(a.region || [])} (jpeg).` ``,
and `` `Scrolled ${dir} by ${amount}.` `` respectively. When the note is empty the emitted
text must be byte-identical to the current output. Example of a flagged result:

    Screenshot captured (jpeg). Warning: this tab was not visible and direct background capture failed; the image was taken with the standard capture path and may be blank or stale.

## Constraints

Hard rules, all non-negotiable:

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS. Visibility-aware capture is mechanism; it is allowed.
3. ASCII only in ALL code and docs: no em-dashes, no Unicode arrows, no curly quotes,
   anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text. The warning note and
   the combined error message above implement this; do not soften, reword, or drop them.
5. No new runtime dependencies. The extension stays vanilla JS: no bundler, no libraries.
6. Rust rules (2021 edition, thiserror, doc comments, rustfmt, clippy deny warnings) apply
   to any Rust you touch; this task should touch none.
7. Comments only for constraints the code cannot express; match the terse comment style
   already used in `extension/service-worker.js` (single-line, explaining WHY not WHAT).
8. Do NOT copy code from the official Anthropic extension or any other project; implement
   the behavior described here from scratch.

Task-specific constraints:

- Modify only `extension/service-worker.js`.
- Do not change `targetDims`, `PX_PER_TOKEN`, `MAX_TOKENS`, `MAX_SIDE`, or
  `MAX_SCREENSHOT_B64`. The clipped path must reuse `targetDims` output; do not invent a
  second budget computation.
- The ScreenshotContext shape stays `{ vpW, vpH, shotW, shotH }` and `rescaleCoord` stays
  untouched.
- Never call `chrome.tabs.update(..., { active: true })`, `chrome.windows.update(...,
  { focused: true })`, or anything else that changes tab or window focus to make a tab
  visible for capture. Stealing the user's focus is forbidden; that is exactly what the
  clipped path exists to avoid.
- The HIDE_FOR_TOOL_USE / SHOW_AFTER_TOOL_USE pairing must hold on every path, including
  both failure paths, exactly once per `screenshot()` call.
- The visible-tab pipeline (capture params, canvas quality ladder, raw-capture fallback,
  context bookkeeping) must be byte-for-byte what it is today apart from the mechanical
  restructuring required by steps above.

## Verification

1. `cargo test` from the repo root: all tests pass (nothing Rust-side changed, so this is a
   regression check, including `tests/tool_schema_fidelity.rs`).
2. Ask the user to reload the extension at chrome://extensions. No MCP client restart is
   needed for an extension-only change; the service worker reconnects to the native host on
   its own.
3. Manual scenarios (driven from an MCP client with the browser-mcp tools):
   a. Background tab capture: create two tabs in the Browser MCP group
      (`tabs_create_mcp` twice), navigate tab A to one visually distinct page and tab B to
      another (for example two different Wikipedia articles). Leave A as the active tab.
      Call `computer` with `action: "screenshot"` on tab B's tabId. Expect either a
      screenshot that clearly shows B's content with the plain caption
      `Screenshot captured (jpeg).` (clipped path worked), or the caption carrying the
      warning note (platform refused background capture and the fallback ran). Both are
      passes; a blank or A-content image with the plain caption is a failure.
   b. Coordinate integrity after a background capture: on tab B, use `find` to locate a
      link, then `computer` `left_click` with a `coordinate` read off the background
      screenshot from (a). After activating B manually, confirm the click landed where the
      screenshot showed it (the recorded context must map coordinates exactly as the
      visible path does).
   c. Visible tab regression: screenshot the active tab A. Expect the plain caption and an
      image identical in size and quality behavior to before the change.
   d. Scroll regression: `computer` `scroll` on the active tab still returns
      `Scrolled down by 3.` plus a screenshot; captions gain no note on clean paths.
   e. Resize regression: `resize_window` still clears contexts, and the next screenshot
      re-establishes correct coordinate mapping (click accuracy after resize).

## Out of scope

- Full-page or beyond-viewport capture. `captureBeyondViewport` stays `false` everywhere;
  the clip rect is always exactly the CSS viewport at origin `(0, 0)`.
- Any change to the token budget math (`targetDims`, `PX_PER_TOKEN`, `MAX_TOKENS`,
  `MAX_SIDE`) or to `MAX_SCREENSHOT_B64`.
- The `zoom` action's region semantics (T11). `zoom` keeps delegating to the same
  `screenshot()`; the only change at its call site is the note-plumbing pattern above.
- Blank-frame detection heuristics (decoding pixels to check for uniform color). The
  truthful warning note is the required mitigation; do not add image analysis.
- Activating, focusing, or un-minimizing anything to force visibility.
- Reintroducing `Emulation.setDeviceMetricsOverride` or any other device-metrics coordinate
  model; that approach is superseded.
- Retry loops, capture timeouts, settle-time tuning, or new logging.
- Any change to `extension/content.js`, `extension/agent-visual-indicator.js`, any Rust
  file, any schema, or any test.
