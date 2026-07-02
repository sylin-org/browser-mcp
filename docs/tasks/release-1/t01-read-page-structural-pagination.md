# T01: read_page structural pagination with element and char caps

## Goal

Today `read_page` output is cut at a raw character limit: the accessibility
tree serializer stops mid-line, appends `... (truncated)`, and gives the model
no way to know what was lost or how to get it. Replace that with structural
pagination: when the tree exceeds the character budget, collapse whole
subtrees behind their parent's ref with an explicit marker the model can act
on, add a hard 10000-element backstop, and end oversized output with a
truthful summary line. Output that fits the budget must remain byte-identical
to today.

## Project context

Browser MCP is a governed browser automation system. A single Rust binary is
both the MCP server (JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the
Chrome native-messaging host; a thin Manifest V3 extension executes CDP
commands. Architecture:

```
MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser
```

The two binary roles run as separate OS processes bridged by tokio-native
named-pipe/UDS IPC. The extension holds mechanism only: all policy, access,
and redaction decisions live in the Rust binary.

Key files:

- `src/mcp/server.rs`: JSON-RPC loop in the binary. Not touched by this task.
- `src/mcp/schemas/tools.json`: SACRED. Byte-frozen official tool schemas.
  Never edit. Guarded by `tests/tool_schema_fidelity.rs`.
- `extension/service-worker.js`: CDP dispatch, tool handlers, screenshot
  pipeline. Its `read_page` handler already forwards all arguments to the
  content script; it is NOT touched by this task.
- `extension/content.js`: accessibility tree generation, element-ref mapping,
  find, form_input, page text extraction. This is the ONLY file this task
  touches.
- `extension/agent-visual-indicator.js`: phantom cursor and glow overlays.
  Overlay elements use `browser-mcp-*` ids and are skipped by DOM reads. Not
  touched by this task.

Build and test: run `cargo test` from the repo root; all tests must pass.
This task changes only extension JavaScript, so no Rust rebuild is required,
but run `cargo test` anyway to prove nothing regressed. Extension changes
require the user to reload the extension at chrome://extensions to take
effect; the extension reconnects to the native host on its own after a
reload. No MCP client restart is needed because the binary is unchanged. If
you ever need to rebuild the binary and `target/debug/browser-mcp.exe` is
locked by a running session, rename it aside first (for example:
`mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and
rebuild.

## Current behavior

All facts below were verified by reading the named files.

`extension/content.js`, function `accessibilityTree(options)`, lines 119-192:

- Line 121: `const filter = options.filter || "all";`
- Line 122: `const maxDepth = options.depth || 15;` The `depth` argument IS
  honored: line 137 bails out of `walk` when `depth > maxDepth`, with the
  walk root at depth 0.
- Line 123: `const maxChars = options.max_chars || 50000;`
- Lines 126-135: the `add(s)` helper appends to the output string `out`.
  When `out.length + s.length > maxChars` it appends a PARTIAL slice of `s`
  followed by the literal marker `\n... (truncated)`, sets a `truncated`
  flag, and every later `add` is a no-op. This is the only overflow handling
  today: a mid-line cut, no element cap, no guidance to the model.
- Lines 136-183: `walk(el, depth, indent)` does a single top-down pass that
  serializes as it goes:
  - Line 137: returns if `truncated || depth > maxDepth || !el ||
    el.nodeType !== 1`.
  - Line 138: skips elements whose id starts with `browser-mcp-` (our own
    visual-indicator overlay).
  - Line 140: skips `script`, `style`, `noscript`, `template`.
  - Lines 141-145: computes `r = role(el)`, `n = accessibleName(el)`,
    `isInteractive`, `isVisible`, `isContainer`.
  - Line 146: with `filter === "interactive"`, prunes elements that are
    neither interactive nor containers.
  - Line 147: `show` is true when
    `((filter === "all" && (r || n)) || (filter === "interactive" && isInteractive)) && isVisible`.
  - Lines 148-163: when `show`, builds the element line: indent, then
    `r || tag`, then the accessible name in double quotes sliced to 100
    chars, then ` [ref_N]` via `refFor(el)` (EVERY shown line carries a
    ref), then `href` for anchors, then `value` or `secret_value` sliced to
    80 chars for input/textarea, then `type` for inputs, then `placeholder`,
    then `disabled`. Emitted with `add(line + "\n")` on line 163.
  - Lines 164-174: for a `<select>`, each option is emitted as an indented
    child line through individual `add` calls, so today the option list can
    be cut mid-list by the char budget.
  - Lines 176-182: for non-select elements, recurses into
    `el.shadowRoot.children` first, then `el.children`, with
    `nextIndent = show ? indent + "  " : indent` (line 179). Elements with
    `show === false` emit no line but their children are still walked at the
    same indent.
- Lines 184-189: `ref_id` IS honored: it re-roots the walk at the deref'd
  element; when the ref is stale the function returns exactly
  `Error: ref_id "${options.ref_id}" not found or was garbage-collected.`
  (line 187).
- Line 190: `walk(root, 0, "");`
- Line 191: returns
  `out + `\nViewport: ${window.innerWidth}x${window.innerHeight}``. Because
  every emitted line ends with `\n`, non-empty output has a blank line
  before the `Viewport:` trailer. The trailer is NOT counted against
  `maxChars`.

Ref bookkeeping (lines 17-35): `refFor(el)` assigns `ref_1`, `ref_2`, ... in
first-seen order via a module-level `refSeq` counter and memoizes per element,
so ref numbering is determined by traversal order.

`extension/service-worker.js`:

- Lines 479-483: the `read_page` tool handler checks group membership via
  `inGroup(a.tabId)`, then calls
  `content(a.tabId, { type: "accessibilityTree", options: a })` and returns
  `text((r && r.result) || "Could not read the page.")`. The WHOLE argument
  object is forwarded as `options`, so `filter`, `depth`, `ref_id`, and
  `max_chars` already reach the content script. No service worker change is
  needed.
- Lines 197-204: `content()` sends the message and injects `content.js` on
  failure, then retries.

`src/mcp/schemas/tools.json`, lines 270-300: the frozen `read_page` schema
has `tabId` (required), `filter` (enum `interactive`/`all`), `depth`
(described default 15), `ref_id`, and `max_chars` (described default 50000).
The frozen description says an over-limit read returns "an error asking you
to specify a smaller depth or focus on a specific element using ref_id".
That text describes the official extension. Our implementation deliberately
does something more useful and still truthful: structural pagination. Do NOT
implement an error response for overflow, and do NOT touch the description.

## Required behavior

Rewrite the inside of `accessibilityTree(options)` in `extension/content.js`
from a serialize-as-you-walk design to a two-pass design: pass 1 builds a
render tree with per-subtree measurements; pass 2 emits within the budget,
collapsing subtrees that do not fit. The function signature, the message
handler case on line 298, and all other functions in the file stay as they
are.

Definitions used throughout:

- An "element" is one emitted element line: a DOM node whose `show`
  condition (current line 147 semantics, unchanged) is true. Select option
  lines are NOT elements; they ride along with their select's line.
- A node's "unit" is the exact text the current code would emit for that
  node alone: the element line built by current lines 148-162 plus, for a
  `<select>`, all of its option lines built by current lines 164-174, each
  line ending with `\n`. Nodes with `show === false` have an empty unit.
- `MAX_ELEMENTS` is a new constant equal to `10000`. Declare it as a `const`
  at the top of `accessibilityTree` (or module scope next to the function).

### Pass 1: measure

Walk the DOM exactly as `walk` does today: same entry guards (element nodes
only, `depth > maxDepth` bail with root at depth 0), same `browser-mcp-` id
skip, same script/style/noscript/template skip, same
`filter === "interactive"` pruning, same `show` computation, same recursion
order (shadow root children first, then light children), same
`nextIndent = show ? indent + "  " : indent`, same select-is-a-leaf rule
(never descend into a `<select>`). But instead of appending to a string,
build one record per visited node:

- `unit`: the node's unit string (empty for `show === false` nodes). Build
  it by moving the CURRENT line-construction code (lines 148-162 and the
  select-option loop, lines 164-174) into this pass unchanged, minus the
  `add` calls. Do not reword, reorder, or reformat anything in the line;
  same slicing (100-char names, 80-char values), same attribute order, same
  quoting.
- `ref`: for shown nodes, the value `refFor(el)` returned while building the
  unit (needed later for collapse markers). `refFor` must be called exactly
  once per shown node, in the same traversal order as today, so ref
  numbering is identical to the current implementation for the same page.
  Never call `refFor` for nodes that are not shown.
- `indent`: the indent string the unit was built with.
- `children`: the child records, in traversal order.
- `unitChars`: `unit.length`.
- `subtreeChars`: `unitChars` plus the sum of all children's `subtreeChars`.
- `elements`: `(show ? 1 : 0)` plus the sum of all children's `elements`.

Root selection is unchanged: `document.body`, or the `ref_id` element via
`deref`, with the stale-ref error string on current line 187 returned
verbatim. Let `total` be the root record's `elements`.

### Pass 2: emit

Walk the render tree top-down with mutable state: `out = ""`,
`remaining = maxChars`, `shown = 0`, and boolean flags `collapsed = false`
(a collapse marker was emitted), `stopped = false` (the walk halted because
even a collapsed form did not fit), `capped = false` (the element cap was
reached). Rules, applied per record:

1. If `stopped` or `capped`, do nothing (the whole emit halts).
2. If the record is NOT shown (empty unit): recurse into each child in
   order, applying these same rules. Pass-through nodes never collapse; only
   nodes that own a line (and therefore a ref) can.
3. If the record IS shown and `subtreeChars <= remaining`: the whole subtree
   fits. Append `unit` to `out`, subtract `unitChars` from `remaining`,
   increment `shown`; if `shown >= MAX_ELEMENTS`, set `capped = true`. Then
   recurse into each child in order (rule 1 halts the recursion if the cap
   just fired).
4. If the record IS shown and `subtreeChars > remaining`: the subtree does
   not fit. Build the collapse marker line:

   ```
   <indent>  [subtree collapsed: <N> elements; call read_page with ref_id=<ref> to expand]
   ```

   followed by `\n`, where `<indent>  ` is the record's indent plus two
   spaces (the position its children would occupy), `<N>` is
   `elements - 1` written as a decimal integer, and `<ref>` is the record's
   ref (for example `ref_42`), bare, no quotes. Then:
   - If `unitChars + markerLine.length <= remaining`: append `unit`, then
     the marker line; subtract both lengths from `remaining`; increment
     `shown` (the parent line counts; the collapsed descendants do not); if
     `shown >= MAX_ELEMENTS`, set `capped = true`; set `collapsed = true`.
     Do NOT recurse into children.
   - Otherwise: set `stopped = true` and emit nothing for this record. The
     entire emit pass ends here; later siblings are not emitted. The
     summary line (below) discloses the shortfall. This whole-stop rule is
     deliberate: output is always a prefix of document order plus markers,
     never a sequence with silent gaps in the middle.

   Invariant, no special case needed: whenever the marker branch is taken,
   `N >= 1`. (`subtreeChars > remaining >= unitChars + markerLine.length`
   implies descendants contributed characters, and only shown descendants
   contribute characters.)

Note the consequence of rule 3: inside a subtree that fits, every descendant
also fits, so markers only appear at the frontier where the budget runs out.
Siblings after a collapsed subtree are still emitted while budget remains;
that is the breadth-over-depth property this design exists for.

A `<select>` is a leaf record (no children), so it can never emit a marker;
if its unit does not fit the remaining budget, rule 4 takes the `stopped`
branch. This means an option list is now atomic: it is either emitted whole
or not at all. That is an intentional over-threshold improvement over
today's mid-list cut.

### Trailing lines

After the emit pass, compute `omitted = total - shown`, then append in this
order (these lines do NOT count against `maxChars`, matching how the
viewport trailer is treated today):

1. If `capped` AND `omitted > 0`, append exactly:

   ```
   [element cap reached: output stopped after 10000 elements; use filter="interactive", a ref_id subtree, or a smaller depth]
   ```

   followed by `\n`. (If the cap fired on the very last element and nothing
   was actually omitted, the line would be a lie; the `omitted > 0` guard
   prevents that.)

2. If `omitted > 0`, append exactly:

   ```
   [showing <M> of <T> elements; expand a collapsed subtree with ref_id, or narrow with filter="interactive" or a smaller depth]
   ```

   followed by `\n`, where `<M>` is `shown` and `<T>` is `total`, both
   decimal integers. `total` counts only elements inside the CURRENT view
   (this root, this filter, this depth), so an expansion call with `ref_id`
   gets a fresh budget and fresh depth measured from that root. Note that
   `collapsed` or `stopped` each imply `omitted > 0`, so this one condition
   covers every degraded outcome.

3. The existing return expression from line 191, unchanged:
   `out + `\nViewport: ${window.innerWidth}x${window.innerHeight}``.

### Removals and invariants

- The `add(s)` mid-line cutting logic and the literal marker
  `... (truncated)` are removed. That string must never appear in any output
  again.
- When the whole tree fits (`subtreeChars` of the root `<= maxChars`) and
  `total <= MAX_ELEMENTS`, the output must be byte-for-byte identical to
  what the current implementation produces: same lines, same order, same
  refs, no markers, no summary, same viewport trailer. This falls out of the
  design if pass 1 reuses the existing line-construction code verbatim.
- `depth` and `ref_id` were verified to already work (current lines 122,
  137, 184-189); preserve their exact semantics. No new plumbing is needed
  in `extension/service-worker.js`: its `read_page` handler (lines 479-483)
  already forwards the full argument object.
- Keep the exact default coercions: `options.filter || "all"`,
  `options.depth || 15`, `options.max_chars || 50000`.
- Keep the stale-ref error string on current line 187 verbatim.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or
   description strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction
   decisions in extension JS. Pagination is mechanism (a size budget), not
   policy; do not add any content-based filtering while you are in there.
3. ASCII only in ALL code and docs: no em-dashes, no unicode arrows, no
   curly quotes, anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute
   behavior. Every collapse is disclosed by a marker, every shortfall by the
   summary line; nothing is dropped silently. Counts in markers and the
   summary must be exact, not estimates.
5. No new runtime dependencies. Extension stays vanilla JS (no bundler, no
   libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc
   comments on public items, rustfmt clean, clippy with deny warnings. (No
   Rust changes are expected in this task.)
7. Comments only for constraints the code cannot express; match the
   surrounding comment density and style. The existing comments inside
   `accessibilityTree` (the select-leaf comment, the truthful-value comment)
   must survive the restructuring next to the code they describe.
8. Do NOT copy code from the official Anthropic extension or any other
   project; implement the described behavior from scratch.

Task-specific:

9. Only `extension/content.js` changes; within it, only the body of
   `accessibilityTree` (plus the `MAX_ELEMENTS` constant). No changes to
   `refFor`, `deref`, `role`, `accessibleName`, `interactive`, `visible`,
   `sensitive`, `pageText`, `find`, `setFormValue`, `refCoordinates`, or the
   message handler.
10. Never change the per-line element format for expanded nodes: same
    fields, same order, same slicing, same quoting as today.
11. Ref numbering must stay identical to today's for the same page and
    options: `refFor` called once per shown node, in traversal order, shown
    nodes only.
12. The three new line formats (collapse marker, cap line, summary line)
    must match the templates above character for character, including
    bracket placement, punctuation, and the quoted `filter="interactive"`.

## Verification

1. Run `cargo test` from the repo root. All tests must pass (this task adds
   no Rust code, so this confirms the schema guard and everything else is
   untouched).
2. Ask the user to reload the extension at chrome://extensions. No MCP
   client restart is needed because the binary is unchanged.
3. Manual end-to-end checks through the MCP client:
   - Small page, byte-identical path: navigate a group tab to
     https://example.com and call `read_page` with defaults. Expect no
     collapse markers, no cap line, no summary line, and the usual
     `Viewport: WxH` trailer. If you captured the pre-change output, diff
     them: identical.
   - Pagination path: navigate to a large page (for example
     https://en.wikipedia.org/wiki/Web_browser) and call `read_page` with
     `max_chars: 2000`. Expect: complete lines only (no mid-line cuts, no
     `... (truncated)` anywhere), one or more collapse markers matching the
     template, a final summary line with plausible `M of T`, then the
     viewport trailer.
   - Expansion path: take a `<ref>` from any collapse marker and call
     `read_page` with `ref_id` set to it. Expect that subtree, rooted at
     that element, with its own fresh budget.
   - `filter: "interactive"` and `depth: 3` still shrink output as before.
   - Element cap (optional, synthetic): via `javascript_tool` run
     `document.body.innerHTML = Array.from({length: 12000}, (_, i) => "<span>item " + i + "</span>").join(""); "ok"`
     then call `read_page` with `max_chars: 2000000`. Expect exactly 10000
     `span` lines, then the cap line, then a summary reading
     `[showing 10000 of 12000 elements; ...]`, then the viewport trailer.
   - Stale ref: call `read_page` with `ref_id: "ref_99999"`. Expect the
     unchanged error string
     `Error: ref_id "ref_99999" not found or was garbage-collected.`

## Out of scope

- Viewport culling (that is task T02). Do not change which elements are
  considered visible; do not touch `visible()` or add any
  position-in-viewport logic.
- Any change to `role()`, `accessibleName()`, `interactive()`,
  `sensitive()`, or the `show` condition semantics.
- Any schema edit. The frozen `read_page` description mentions returning an
  error when output exceeds the limit; do NOT implement that error and do
  NOT edit the description to match the new behavior.
- No changes to `extension/service-worker.js` (its handler already forwards
  the arguments), `extension/agent-visual-indicator.js`,
  `extension/manifest.json`, or any Rust code.
- No changes to `find`, `get_page_text`, `form_input`, or their content.js
  functions.
- No caching of the render tree between calls; every `read_page` call walks
  the live DOM fresh.
- No new tool arguments beyond the frozen schema (no page tokens, no cursor
  parameters, no continuation ids).
- Do not add configuration for the 10000-element cap or the marker wording;
  they are fixed by this task.
