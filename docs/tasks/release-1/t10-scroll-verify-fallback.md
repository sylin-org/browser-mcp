# T10: computer scroll: effectiveness verification and scrollable-ancestor fallback

## Goal

Make the `computer` tool's `scroll` action verify that the dispatched mouse wheel actually
moved something, and fall back to a direct `scrollBy` on the nearest scrollable ancestor when
it did not. Today the action fires one CDP wheel event and reports success blind, so scrolling
silently no-ops inside overflow containers, virtualized lists, and pages that intercept wheel
events. After this task the result text is always truthful about what moved.

## Project context

Browser MCP is governed browser automation. A single Rust binary is both the MCP server
(JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host; a thin
Manifest V3 extension executes CDP commands. Architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS IPC.

Files relevant to this task:

- `extension/service-worker.js`: CDP dispatch, the `computer` action switch, screenshot
  pipeline, input helpers. This is the ONLY file you will modify.
- `extension/content.js`: DOM reads (accessibility tree, find, form_input, page text). Do not
  touch it in this task.
- `src/mcp/schemas/tools.json`: byte-frozen official tool schemas. Never edit. The `scroll`
  action already has everything it needs: `coordinate` (line 71 of the schema file),
  `scroll_direction` (enum up/down/left/right, lines 100-103), `scroll_amount` (default 3,
  lines 105-109).
- `tests/tool_schema_fidelity.rs`: guard test that fails if the schema drifts.

Build and test: run `cargo test` from the repo root; all tests must pass. This task changes
only extension JavaScript, so no Rust rebuild is required, but run `cargo test` anyway to
confirm nothing else broke. Extension changes take effect after the user reloads the
extension at chrome://extensions. Binary or schema changes would require an MCP client
restart, but this task makes none. If you ever do need to rebuild and
`target/debug/browser-mcp.exe` is locked by a running session, rename it aside first
(for example: `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and
rebuild.

## Current behavior

All line numbers verified against the current tree.

The `scroll` case lives in the `computer` function in `extension/service-worker.js`,
lines 405-415:

- Line 406: `const c = (await resolveCoords(tabId, a)) || [0, 0];` resolves the target point
  (model coordinates are rescaled to CSS viewport px; `ref` coordinates come back already in
  CSS px). `resolveCoords` is defined at lines 278-287.
- Lines 407-410: direction defaults to `"down"`, amount is capped at 10
  (`Math.min(a.scroll_amount || 3, 10)`), and `deltaX`/`deltaY` are computed as
  `amount * 100` signed by direction (down = positive `deltaY`, right = positive `deltaX`).
- Line 411: `await moveCursor(tabId, c[0], c[1]);` moves the phantom cursor
  (helper at line 252, best-effort, resolves even when no content script is present).
- Line 412: dispatches one `Input.dispatchMouseEvent` of type `mouseWheel` at the point with
  the computed deltas and the `modifiers` bits computed at line 360.
- Line 413: `await sleep(250);` with no verification of any kind.
- Line 414: returns `textImage(`Scrolled ${dir} by ${amount}.`, await screenshot(tabId));`.

Supporting helpers you will reuse, all already present in `extension/service-worker.js`:

- `cdp(tabId, method, params)` at lines 114-117: ensures debugger attachment and sends a CDP
  command.
- `Runtime.evaluate` usage pattern with `returnByValue: true`: see `probeViewport` at
  lines 72-80 and the `javascript_tool` handler at lines 505-511. Note that a JS exception in
  the evaluated expression does not throw; it comes back as `r.exceptionDetails`.
- `sleep(ms)` at lines 242-244.
- `text(t)` at lines 207-209 and `textImage(t, base64)` at lines 210-212.
- `screenshot(tabId)` at lines 215-239.

The comment at line 356 records the screenshot contract: of the 13 computer actions, only
`screenshot`, `scroll`, and `zoom` return a screenshot. That contract must survive this task.

## Required behavior

Rewrite the `scroll` case (and only the `scroll` case) to this exact flow. Add two
module-level helper functions in the input-helpers region of `extension/service-worker.js`
(the block around lines 241-320, near `resolveCoords`), named `probeScrollState` and
`directScrollFallback`.

### Scrollable-ancestor predicate (used by both helpers, inside the evaluated snippets)

An element counts as scrollable when BOTH hold:

1. Its computed style has `overflow-y` equal to `"auto"` or `"scroll"`, OR `overflow-x`
   equal to `"auto"` or `"scroll"`.
2. `scrollHeight > clientHeight` OR `scrollWidth > clientWidth`.

The walk starts at `document.elementFromPoint(x, y)` (which may return null when the point is
outside the viewport) and follows `parentElement` upward until it finds a scrollable element
or runs out of ancestors. On plain pages the walk usually finds nothing, because `html` and
`body` normally have `overflow: visible`; that is fine, the window scroll position is tracked
separately.

### Helper 1: probeScrollState(tabId, x, y)

Runs one `cdp(tabId, "Runtime.evaluate", { expression, returnByValue: true })` where the
expression is an IIFE, with x and y interpolated as integers (`Math.round` them before
interpolation; never interpolate anything that is not a number). The IIFE performs the
ancestor walk above and returns an object with this exact shape:

    { winX: window.scrollX, winY: window.scrollY,
      hasEl: <boolean, true when a scrollable ancestor was found>,
      elX: <ancestor.scrollLeft or null>, elY: <ancestor.scrollTop or null> }

The helper resolves to that value, or to `null` on any failure: the `cdp` call rejected,
`r.exceptionDetails` is present, or `r.result.value` is missing. Never let this helper throw.

### Helper 2: directScrollFallback(tabId, x, y, dx, dy)

Runs one `Runtime.evaluate` (same interpolation and safety rules; dx and dy are the SAME
`deltaX`/`deltaY` values already computed for the wheel event, sign and magnitude unchanged).
The IIFE:

1. Repeats the ancestor walk from `document.elementFromPoint(x, y)`.
2. Picks the found ancestor as the scroll target, or `window` when none was found.
3. Reads the target's position before (element: `scrollLeft`/`scrollTop`; window:
   `window.scrollX`/`window.scrollY`).
4. Calls `target.scrollBy({ left: dx, top: dy, behavior: "instant" })`. The `"instant"`
   behavior is required so that pages with CSS `scroll-behavior: smooth` still move
   synchronously and the read in the next step is meaningful.
5. Reads the position again and returns:

       { moved: <true when either axis changed by more than 5>,
         usedWindow: <true when no scrollable ancestor was found> }

The helper resolves to that value, or to `null` on any failure (same failure conditions as
helper 1). Never let this helper throw.

### The new scroll case

Steps, in order. Steps 1, 2, 4, and 5 are byte-for-byte what the current code already does at
lines 406-412; keep them unchanged.

1. Resolve the point: `const c = (await resolveCoords(tabId, a)) || [0, 0];`
2. Compute `dir`, `amount`, `deltaX`, `deltaY` exactly as today (defaults, cap of 10, and
   the `amount * 100` magnitudes are all unchanged).
3. `const before = await probeScrollState(tabId, c[0], c[1]);`
4. `await moveCursor(tabId, c[0], c[1]);`
5. Dispatch the `mouseWheel` exactly as today (same params, including `modifiers`).
6. If `before` is `null` (verification unavailable, for example the page is mid-navigation):
   `await sleep(250);` and return result A below. This is the legacy path, identical in
   observable behavior to today.
7. Otherwise `await sleep(200);` then
   `const after = await probeScrollState(tabId, c[0], c[1]);`
8. If `after` is `null`: return result A. Do not run the fallback when the re-read failed;
   a blind fallback risks double-scrolling.
9. Compute effectiveness with a 5 px threshold on every axis:
   - `windowMoved` = `Math.abs(after.winX - before.winX) > 5 || Math.abs(after.winY - before.winY) > 5`
   - `elementMoved` = `before.hasEl && after.hasEl` AND the absolute difference of `elX` or
     of `elY` between the two probes is greater than 5 (treat null as 0 in the arithmetic).
   - If `windowMoved || elementMoved`: return result A. The wheel worked.
10. The wheel did nothing. Run
    `const fb = await directScrollFallback(tabId, c[0], c[1], deltaX, deltaY);`
    - If `fb` is `null`: return result D.
    - If `fb.moved` is true: return result B.
    - Otherwise: return result C.

### Result texts, verbatim

Every result, on every path, is
`textImage(<message>, await screenshot(tabId))`. Scroll stays one of the three
screenshot-returning actions no matter which path was taken.

- Result A (wheel effective, or verification unavailable):
  `` `Scrolled ${dir} by ${amount}.` ``
- Result B (wheel had no effect, fallback moved the target):
  `` `Scrolled ${dir} by ${amount} (mouse wheel had no effect; used direct scroll fallback).` ``
- Result C (wheel and fallback both moved nothing):
  `` `Scroll ${dir} had no effect at (${c[0]}, ${c[1]}); the page did not move at that position.` ``
- Result D (wheel had no effect and the fallback snippet failed to run):
  `` `Scroll ${dir} had no effect at (${c[0]}, ${c[1]}); the direct scroll fallback could not run.` ``

Result A on the unverifiable paths (steps 6 and 8) is acceptable because it makes exactly the
same claim the current code makes today; the engine only ever asserts that it scrolled when
it either verified movement or could not verify at all. Results B, C, and D exist because the
engine is truthful: recovery and failure must be visible in the result text.

## Constraints

Hard rules, all non-negotiable:

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description strings.
   `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in extension
   JS. Scroll verification is mechanism; it is allowed.
3. ASCII only in ALL code and docs: no em-dashes, no Unicode arrows, no curly quotes,
   anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text. The four result texts
   above implement this; do not soften or merge them.
5. No new runtime dependencies. The extension stays vanilla JS: no bundler, no libraries.
6. Rust rules (2021 edition, thiserror, doc comments, rustfmt, clippy deny warnings) apply to
   any Rust you touch; this task should touch none.
7. Comments only for constraints the code cannot express; match the terse comment style
   already used in `extension/service-worker.js` (single-line, lowercase after the marker,
   explaining WHY not WHAT).
8. Do NOT copy code from the official Anthropic extension or any other project; implement the
   behavior described here from scratch.

Task-specific constraints:

- Modify only `extension/service-worker.js`. No changes to `extension/content.js`,
  `extension/agent-visual-indicator.js`, any Rust file, or any schema or test file.
- Do not add new content-script message types; both snippets run through the existing `cdp`
  helper with `Runtime.evaluate`.
- Keep the existing delta computation (`amount * 100`), the cap of 10 ticks, the `[0, 0]`
  coordinate default, the `moveCursor` call, and the `modifiers` pass-through exactly as they
  are.
- Interpolate only rounded numbers into the evaluated expressions. Never interpolate strings.
- Both helpers must be exception-safe: they resolve to a value or `null`, they never reject.
- Use the literal threshold 5 in the comparisons; a single short comment noting it matches
  the "moved more than 5px" contract is acceptable.

## Verification

1. `cargo test` from the repo root: all tests pass (nothing Rust-side changed, so this is a
   regression check, including `tests/tool_schema_fidelity.rs`).
2. Ask the user to reload the extension at chrome://extensions. No MCP client restart is
   needed for an extension-only change; the service worker reconnects to the native host on
   its own.
3. Manual scenarios (driven from an MCP client with the browser-mcp tools):
   a. Normal page: navigate a grouped tab to a long article (any long Wikipedia page works),
      call `computer` with `action: "scroll"`, `scroll_direction: "down"`, coordinates near
      page center. Expect the text `Scrolled down by 3.` and a screenshot showing moved
      content.
   b. Wheel-blocked container (forces the fallback): have the user save this file and open it
      via file:// in a grouped tab:

          <!DOCTYPE html>
          <html><body style="margin:0">
          <div id="box" style="height:300px;width:400px;overflow-y:scroll;border:1px solid black">
            <div style="height:3000px;background:linear-gradient(red,blue)">tall content</div>
          </div>
          <script>
            document.getElementById("box").addEventListener(
              "wheel", (e) => e.preventDefault(), { passive: false });
          </script>
          </body></html>

      Scroll down at a coordinate inside the box. The wheel is swallowed by preventDefault,
      so expect the text
      `Scrolled down by 3 (mouse wheel had no effect; used direct scroll fallback).` and a
      screenshot where the gradient inside the box has visibly shifted.
   c. Nothing to scroll: on a short page whose content fits the viewport, scroll down
      anywhere. Expect
      `Scroll down had no effect at (x, y); the page did not move at that position.` with the
      actual coordinates, plus the screenshot.
   d. Regression: `left_click`, `type`, and `scroll_to` still behave exactly as before
      (text-only results for click/type/scroll_to; screenshot only on screenshot, scroll,
      zoom).

## Out of scope

- `scroll_to` (the ref-based case at lines 416-421). Do not touch it, do not add
  verification to it.
- Changing wheel delta magnitudes, tick caps, direction mapping, or dispatching more than
  one wheel event.
- Smooth-scroll animation of any kind. The `behavior: "instant"` in the fallback exists to
  DEFEAT page-level smooth scrolling during measurement, not to add animation.
- Per-frame targeting inside iframes. `Runtime.evaluate` and `elementFromPoint` here operate
  on the top document only; scrolling content inside a cross-document iframe is a separate
  future task. Do not add `Page.getFrameTree`, frame execution contexts, or content-script
  frame plumbing.
- Retrying the wheel, tuning the 200 ms settle time, or adding configuration for the 5 px
  threshold.
- Any change to the screenshot pipeline, `resolveCoords`, `moveCursor`, coordinate rescaling,
  or any other `computer` action.
- Any Rust change, schema change, or test change.
