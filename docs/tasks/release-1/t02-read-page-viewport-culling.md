# T02: read_page viewport culling for filter=interactive

## Goal

Make `read_page` with `filter=interactive` return only elements whose bounding rectangle
intersects the current viewport, matching the behavior of the official Claude-in-Chrome
extension. When culling actually removed at least one element, append one truthful note
line telling the model how to get the rest. `filter=all` output stays byte-identical.

## Project context

Browser MCP is a governed browser automation system. A single Rust binary is both the MCP
server (JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host.
A thin Manifest V3 extension executes CDP commands and DOM reads. Architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS
IPC. All policy, redaction, and audit decisions live in the Rust binary. The extension is
mechanism only.

Files involved in this task:

- `extension/content.js` (the ONLY file you will edit). Content script injected into
  pages. Implements the accessibility tree for `read_page`, plus `find`, `form_input`,
  and page text extraction. Vanilla JS, no bundler, no libraries.
- `extension/service-worker.js` (read for context, do NOT edit). Its `read_page` handler
  (around lines 479-483) checks tab-group membership, then forwards the raw tool
  arguments object as `options` in a `{ type: "accessibilityTree", options: a }` message
  to the content script via `chrome.tabs.sendMessage`.
- `src/mcp/schemas/tools.json` (NEVER edit). Byte-frozen official Claude-in-Chrome
  v1.0.78 tool schemas. The `read_page` schema defines `filter` as an enum of exactly
  `"interactive"` and `"all"`, defaulting to all elements when absent.
- `tests/tool_schema_fidelity.rs` (never edit). Guard test that must keep passing.
- `extension/agent-visual-indicator.js` (do not edit). Its overlay elements use
  `browser-mcp-*` ids and are already skipped by the DOM-read code.

Build and test: run `cargo test` from the repo root; all tests must pass. This task
changes only extension JS, so no Rust rebuild is required, but run `cargo test` anyway to
prove nothing regressed. Extension changes only take effect after the user reloads the
unpacked extension at chrome://extensions. Binary or schema changes (none expected here)
would require an MCP client restart. If you ever need to rebuild and
`target/debug/browser-mcp.exe` is locked by a running session, rename it aside first
(for example: `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and
rebuild.

## Current behavior

All facts below were verified by reading `extension/content.js` (313 lines) at the time
this prompt was written. Re-verify before editing; line numbers may have drifted.

- `visible(el)` (lines 97-101) is the only visibility gate. It returns false when
  `offsetParent` is null (except for `body` and `position: fixed` elements) or when
  computed `display` is `none` or `visibility` is `hidden`. It performs NO viewport or
  scroll-position check. An interactive element 40 screens below the fold is "visible".
- `accessibilityTree(options)` (lines 119-192) builds the `read_page` output.
  - Line 121: `const filter = options.filter || "all";`
  - Line 125: `let truncated = false;` next to the `out` accumulator; `add(s)` (lines
    126-135) enforces the `max_chars` budget.
  - `walk(el, depth, indent)` (lines 136-183) recurses through children and shadow
    roots. Line 138 skips our own `browser-mcp-*` overlay elements. Line 146 prunes
    non-interactive leaf elements in interactive mode:
    `if (filter === "interactive" && !isInteractive && !isContainer) return;`
  - Line 147 decides emission:
    `const show = ((filter === "all" && (r || n)) || (filter === "interactive" && isInteractive)) && isVisible;`
    There is no bounding-rect test anywhere in this path, so interactive elements are
    emitted regardless of scroll position.
  - Lines 165-174 emit `<select>` options as child lines inside the `if (show)` block.
  - Lines 178-182 descend into shadow root and light children; `nextIndent` indents one
    level only when the current element was shown.
  - Line 191 returns the result:

        return out + `\nViewport: ${window.innerWidth}x${window.innerHeight}`;
- The message handler (line 298) responds to `{ type: "accessibilityTree" }` by calling
  `accessibilityTree(msg.options)`.
- `find()` (lines 215-237) has its own separate loop and its own `visible()` check; it is
  unrelated to this task.

The official extension, for any `filter` value other than `"all"`, additionally skips
elements whose bounding rect does not intersect the current viewport. Ours does not,
which floods the model with off-screen elements on long pages.

## Required behavior

Edit `extension/content.js` only. Three changes, exactly as specified.

1. Add a viewport-intersection helper.

Place it directly after the `visible()` function (it belongs to the same
"Role / name / interactivity / visibility" section). Use exactly this logic:

    function intersectsViewport(el) {
      const rect = el.getBoundingClientRect();
      return rect.bottom > 0 && rect.right > 0 && rect.top < window.innerHeight && rect.left < window.innerWidth;
    }

Rationale you must preserve in the implementation (not necessarily as a comment):
`getBoundingClientRect()` is viewport-relative for every element, so this test is correct
at any scroll position and for `position: fixed` elements without special cases. Strict
inequalities mean partial intersection counts as in-viewport, while an element that only
touches the viewport edge with zero visible extent is culled. A zero-width or zero-height
rect located inside the viewport still passes (left < innerWidth and right > 0 both hold),
which is intentional: do not add any zero-size exclusion.

2. Apply culling in `walk()` for any filter other than `"all"`.

- Declare a `let culled = false;` flag inside `accessibilityTree`, next to the existing
  `let truncated = false;` declaration.
- Replace the single `show` computation (currently line 147) with:

      const wouldShow = ((filter === "all" && (r || n)) || (filter === "interactive" && isInteractive)) && isVisible;
      const show = wouldShow && (filter === "all" || intersectsViewport(el));
      if (wouldShow && !show) culled = true;

  Properties this shape guarantees; your implementation must keep all of them:
  - When `filter === "all"`, `show === wouldShow` and `intersectsViewport` is NEVER
    called (short-circuit), so `filter=all` output and layout-read behavior are
    byte-identical to today, and `culled` can never become true.
  - `intersectsViewport` runs only for elements that would otherwise be emitted, so no
    forced layout happens for elements already excluded by role, interactivity, or
    `visible()`.
  - `culled` becomes true only when the viewport test alone removed an element that
    every existing rule would have shown. Elements excluded for any other reason must
    not set `culled`.
- Do NOT touch the early-return prune at line 146
  (`filter === "interactive" && !isInteractive && !isContainer`). It must not gain a
  viewport check: an off-screen container can still have in-viewport descendants (for
  example absolutely positioned children), and the walk must keep descending.
- Do not change the children-descent code (lines 178-182), the `<select>` option
  emission, the overlay skip, or `nextIndent` handling. A culled element is simply not
  shown; its children are still walked at the unindented level, exactly like any other
  not-shown element today.
- Culling applies identically when `options.ref_id` scopes the walk to a subtree: an
  interactive read of a subtree still returns only the in-viewport part of it.

3. Append one truthful note line when culling removed something.

Change the return statement (currently line 191) so that when `culled` is true, exactly
one extra line is appended AFTER the existing `Viewport: WxH` line, with exactly this
text (ASCII, one line, no trailing whitespace):

    Note: interactive results are limited to the current viewport; scroll or use filter=all for the full document.

Concretely:

    let result = out + `\nViewport: ${window.innerWidth}x${window.innerHeight}`;
    if (culled) {
      result += "\nNote: interactive results are limited to the current viewport; scroll or use filter=all for the full document.";
    }
    return result;

When `culled` is false (including every `filter=all` call, every call where all
interactive elements happen to be in the viewport, and every call on an unknown filter
value where nothing would be shown anyway), the note must NOT appear and the returned
string must be byte-identical to today's output. The note is appended outside the
`add()`/`max_chars` budget, exactly like the existing `Viewport:` line; do not route it
through `add()`.

No other file changes. No new message types. No new options. The service worker already
passes `filter` through untouched.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS. Viewport culling is mechanism (it mirrors what the page currently
   shows), not policy; do not add any policy-flavored switches around it.
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly quotes, anywhere,
   including comments.
4. The engine is truthful: never fake success, never silently substitute behavior. That
   is why the note line exists; do not omit it, reword it, or emit it when nothing was
   culled.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no
   libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on
   public items, rustfmt clean, clippy with deny warnings. (No Rust changes are expected
   in this task; the rule applies if you touch any.)
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style. At most one short comment on the new helper (for example noting
   that getBoundingClientRect is viewport-relative) is acceptable; more is not.
8. Do NOT copy code from the official Anthropic extension or any other project;
   implement the behavior described above from scratch.

Task-specific:

9. `filter=all` must remain byte-identical in output AND must not gain any new
   `getBoundingClientRect` calls.
10. The exact note string given above is the contract; reproduce it character for
    character.
11. The `visible()` function must not be modified.

## Verification

1. `cargo test` from the repo root: all tests pass, including
   `tests/tool_schema_fidelity.rs`, with zero changes to Rust files.
2. Re-read your final `extension/content.js` and confirm: `intersectsViewport` exists
   with strict inequalities; `culled` is set only in the wouldShow-but-not-shown case;
   the note string matches the contract exactly; line 146's early return is untouched;
   `visible()` is untouched; the file is pure ASCII.
3. Manual end-to-end (requires the user, who must reload the extension at
   chrome://extensions after your edit; the binary and MCP client do not need a
   restart for a content-script-only change):
   - Navigate to a long page (for example a long Wikipedia article).
   - Call `read_page` with `filter=interactive`: output contains only elements currently
     on screen, and the final line is the Note line (a long page will always cull
     something).
   - Scroll down (computer tool, `scroll` action), call `read_page` with
     `filter=interactive` again: a different set of elements appears.
   - Call `read_page` with `filter=all`: full-document output, no Note line, identical
     shape to before this change.
   - Call `read_page` with `filter=interactive` on a short page that fits entirely in
     the viewport: no Note line.

## Out of scope

- Pagination, result caps, or any output-size management beyond what exists. That is
  task T01; do not implement or anticipate it here.
- Changing the visibility heuristics for `filter=all`, or changing `visible()` at all.
- Adding viewport culling to `find()`, `pageText()`, or any other content.js function.
- Any edit to `extension/service-worker.js`, `extension/agent-visual-indicator.js`,
  `src/mcp/schemas/tools.json`, or any Rust file.
- New `read_page` parameters, new message types, or new configuration keys.
- Sorting, deduplicating, or otherwise reordering the emitted tree.
- Changing the `Viewport: WxH` line, the truncation marker, or `max_chars` handling.
- Refactoring `walk()` or `accessibilityTree()` beyond the three changes specified.
