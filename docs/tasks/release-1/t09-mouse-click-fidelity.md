# T09: Mouse fidelity: incrementing clickCount sequence, buttons bitmask, force

## Goal

Make synthetic mouse input indistinguishable from real Chrome mouse input for pages that
inspect click sequences and button state. Double and triple clicks must be dispatched as a
sequence of press/release pairs with an incrementing clickCount (1, then 2, then 3), and
every mouse event on the click and drag paths must carry an explicit `buttons` bitmask and
a `force` value that reflect whether a button is held.

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

- `extension/service-worker.js`: CDP dispatch for the `computer` tool. This is the ONLY
  file you will modify.
- `src/mcp/schemas/tools.json`: byte-frozen official tool schemas. Never touched.
- `tests/tool_schema_fidelity.rs`: guard test that must keep passing.
- `extension/content.js`, `extension/agent-visual-indicator.js`: DOM reads and the phantom
  cursor overlay. Not touched by this task.

How mouse input flows today: a `computer` tool call arrives at `dispatch`, which routes to
the `computer(a)` handler in `extension/service-worker.js`. Click-family actions resolve
coordinates (model coordinates are rescaled from screenshot pixels to CSS viewport pixels
by `rescaleCoord`; ref-derived coordinates are already CSS pixels), move the phantom
cursor, then dispatch `Input.dispatchMouseEvent` through the `cdp(tabId, method, params)`
helper (lines 114-117), which auto-attaches the debugger.

Build and test: the extension is vanilla JS loaded unpacked; after editing it, the user
must reload the extension at chrome://extensions. The Rust binary is not rebuilt for this
task, but run `cargo test` from the repo root to confirm all tests still pass. If you do
rebuild for any reason and `target/debug/browser-mcp.exe` is locked by a running session,
rename it aside first (for example `mv target/debug/browser-mcp.exe
target/debug/browser-mcp.exe.old-1`) and rebuild.

## Current behavior

All facts below were verified in `extension/service-worker.js` (568 lines).

- `sleep(ms)` helper at lines 242-244.
- `modifierBits(str)` at lines 260-269 converts a modifier string to the CDP modifiers
  integer. Unchanged by this task.
- `click(tabId, x, y, opts)` at lines 270-277 dispatches exactly three events:
  `mouseMoved` (line 272), `sleep(40)`, `mousePressed` (line 274), `sleep(40)`,
  `mouseReleased` (line 276). The pressed and released events carry `button`,
  `clickCount`, and `modifiers` only. For a double click the pair is sent ONCE with
  `clickCount: 2`; for a triple click ONCE with `clickCount: 3`. No event on this path
  carries `buttons` or `force`.
- The `computer(a)` handler's click branch at lines 373-389: `resolveCoords` (line 378),
  `moveCursor` (line 380), then for hover a bare `mouseMoved` (line 382). For clicks it
  computes `button` ("right" for right_click, else "left") at line 385 and `clickCount`
  (2 for double_click, 3 for triple_click, else 1) at line 386, then calls `click(...)`
  at line 387. Line 387 is the only call site of `click()`.
- `left_click_drag` at lines 422-439 dispatches: `mouseMoved` at the start point (line
  428), `sleep(40)`, `mousePressed` with `button: "left"` (line 430), `sleep(40)`, ten
  interpolated `mouseMoved` events 16 ms apart (lines 432-435), then `mouseReleased` with
  `button: "left"` (line 437). None of these events carry `buttons` or `force`. The drag
  press/release events do not carry `clickCount` at all (CDP defaults it to 0).
- The scroll action dispatches `mouseWheel` at line 412. Out of scope here (see below).

Consequence: pages that track real click sequences (word selection on double click,
paragraph selection on triple click, custom dblclick handlers, drag handlers that read
`event.buttons`) do not register our synthetic input correctly.

## Required behavior

Real Chrome emits an N-click as N press/release pairs with clickCount incrementing 1..N,
and every mouse event carries a `buttons` bitmask plus a `force` value. Reproduce that.

### 1. New module-level constants

Add these two constants in the "Input helpers" section of `extension/service-worker.js`
(near `KEY_MAP`, which starts at line 253):

    const BUTTON_BITS = { left: 1, right: 2, middle: 4 };
    const CLICK_GAP_MS = 40;

`BUTTON_BITS` is the DOM MouseEvent.buttons bitmask per button name. `CLICK_GAP_MS` is the
single delay constant used between press and release and between click iterations (40 ms,
matching the existing rhythm of this file; the acceptable range was 10-50 ms and 40 is the
chosen constant).

### 2. buttons and force semantics (applies to every event you touch)

- `buttons` is the bitmask of mouse buttons held AFTER the event takes effect, exactly as
  the DOM `MouseEvent.buttons` property behaves: 0 on a move before any press, the
  button's bit (from `BUTTON_BITS`) on `mousePressed` and on every `mouseMoved` while the
  button is held, and 0 on `mouseReleased` (the release clears the bit; no other button is
  ever held concurrently in this codebase).
- `force` is `0.5` exactly when that event's `buttons` value is nonzero, otherwise `0`.
  So: pressed events and held-moves carry `force: 0.5`; released events and unpressed
  moves carry `force: 0`.

Both fields must be set explicitly on every event listed in sections 3 and 4, even where
the value is 0 (do not rely on CDP defaults).

### 3. Rework `click()` (lines 270-277)

Replace the body of `click(tabId, x, y, opts)` with the following exact event sequence.
Keep the same signature and keep reading `modifiers`, `button` (default "left"), and
`clickCount` (default 1) from `opts` as today. Let N be the requested clickCount and let
`bit = BUTTON_BITS[button] || 0`.

1. `mouseMoved` at (x, y) with `modifiers`, `buttons: 0`, `force: 0`.
2. `await sleep(CLICK_GAP_MS)`.
3. For i = 1 to N (inclusive), in order:
   a. `mousePressed` at (x, y) with `button`, `clickCount: i`, `modifiers`,
      `buttons: bit`, `force: 0.5`.
   b. `await sleep(CLICK_GAP_MS)`.
   c. `mouseReleased` at (x, y) with `button`, `clickCount: i`, `modifiers`,
      `buttons: 0`, `force: 0`.
   d. If i < N: `await sleep(CLICK_GAP_MS)` before the next iteration.

So a single click (N=1) produces exactly one pressed/released pair with `clickCount: 1`; a
double click produces two pairs (`clickCount: 1` then `clickCount: 2`); a triple click
produces three pairs (`clickCount: 1`, `2`, `3`). Never dispatch a pair whose first
clickCount is 2 or 3.

All events are dispatched via the existing `cdp(tabId, "Input.dispatchMouseEvent", {...})`
helper. CDP's `Input.dispatchMouseEvent` accepts `buttons` (integer) and `force` (number);
pass them as plain JSON fields alongside the existing ones.

Do not change the call site at line 387 or the `clickCount` computation at line 386: the
handler still passes N (1, 2, or 3) into `click()`, and `click()` now expands it into the
incrementing loop.

### 4. Add buttons and force to the drag path (lines 422-439)

Modify only the four dispatch statements; everything else in the `left_click_drag` case
(coordinate rescaling, `moveCursor` calls, the 10-step interpolation, the 16 ms step
delay, the 40 ms sleeps, the result text) stays byte-identical:

- Start `mouseMoved` (line 428): add `buttons: 0, force: 0`.
- `mousePressed` (line 430): add `buttons: BUTTON_BITS.left, force: 0.5`.
- Each interpolated `mouseMoved` (line 433): add `buttons: BUTTON_BITS.left, force: 0.5`.
- Final `mouseReleased` (line 437): add `buttons: 0, force: 0`.

Do NOT add or change `clickCount` on any drag event; the drag press/release continue to
omit it exactly as today.

### 5. Unchanged output

Tool result texts are unchanged: the click branch still returns
`` `${a.action} at (${c[0]}, ${c[1]}).` `` and the drag case still returns
`` `Dragged (${sx}, ${sy}) -> (${ex}, ${ey}).` ``. No new result text, no new logging.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS.
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly quotes, anywhere,
   including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on public
   items, rustfmt clean, clippy with deny warnings. (This task should not require touching
   Rust at all.)
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style. One short comment explaining the incrementing clickCount loop (real
   N-clicks are N pairs with clickCount 1..N) is appropriate; do not comment each field.
8. Do NOT copy code from the official Anthropic extension or any other project; implement
   the behavior described above from scratch.

Task-specific:

9. The only file modified is `extension/service-worker.js`.
10. Exactly one new delay constant (`CLICK_GAP_MS = 40`) and one new bitmask table
    (`BUTTON_BITS`). No other new module-level state.
11. `BUTTON_BITS` includes `middle: 4` as a constant only; do not add any middle-click
    action, parameter, or code path.

## Verification

1. Run `cargo test` from the repo root. All tests must pass (this change is extension-only
   but the schema guard must stay green).
2. Ask the user to reload the extension at chrome://extensions (extension changes are not
   picked up otherwise). Binary and schema were not changed, so no MCP client restart is
   needed.
3. Manual checks through an MCP client driving the extension, on a text-heavy page (for
   example a Wikipedia article):
   - `computer` `double_click` on a word: the word becomes selected (this only works when
     the page sees clickCount 1 then 2).
   - `computer` `triple_click` on a paragraph: the paragraph or line becomes selected.
   - `computer` `left_click` on a link or button: exactly one click, normal activation, no
     accidental double-click side effects.
   - `computer` `right_click`: behaves as before (context-menu-suppressing pages see
     `buttons: 2` while pressed).
   - `computer` `left_click_drag` across text: a selection range is created; pages reading
     `event.buttons` during the drag see 1.
4. Open the service worker console (chrome://extensions, "Inspect views: service worker")
   and confirm no errors are thrown during the actions above.

## Out of scope

- The hover branch: leave the bare `mouseMoved` at line 382 exactly as it is.
- Scroll: do not touch the `mouseWheel` dispatch at line 412 or anything in the scroll or
  scroll_to cases (that is task T10).
- Coordinate rescaling (`rescaleCoord`, `resolveCoords`): already correct; do not modify.
- Middle-click support beyond the `middle: 4` entry in `BUTTON_BITS`: no new actions,
  schema fields, or dispatch paths for the middle button.
- Do not add `clickCount` to the drag path events.
- Do not change the phantom cursor / visual indicator calls (`moveCursor`,
  `showActivity`), the keyboard paths (`pressKey`, `type`), timings other than specified,
  tool result strings, or anything in `extension/content.js`,
  `extension/agent-visual-indicator.js`, or any Rust source.
- Do not refactor `click()` beyond what section 3 requires, and do not rename existing
  functions or constants.
