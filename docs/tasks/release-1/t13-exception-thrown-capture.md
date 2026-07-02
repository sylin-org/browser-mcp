# T13: Capture Runtime.exceptionThrown as console exception entries

## Goal

Uncaught page exceptions are currently invisible to agents: the extension ignores the CDP
`Runtime.exceptionThrown` event, so `read_console_messages` never shows them, even with
`onlyErrors: true`. Add a handler that turns each `Runtime.exceptionThrown` event into a
synthetic console entry with level `"exception"` and pushes it into the same per-tab console
buffer that `Runtime.consoleAPICalled` entries go into.

## Project context

Browser MCP is a governed browser automation system. A single Rust binary is both the MCP
server (JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host. A
thin Manifest V3 extension executes CDP commands. The chain is:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS IPC.

Files relevant to this task:

- `extension/service-worker.js`: the only file you will modify. Vanilla JS, Manifest V3
  service worker. Holds the CDP dispatch, the `chrome.debugger.onEvent` listener, the
  per-tab console and network buffers, and the tool handlers including
  `read_console_messages`.
- `src/mcp/schemas/tools.json`: SACRED. Byte-frozen official Claude-in-Chrome v1.0.78 tool
  schemas. Never edit it, never touch tool names, parameters, or description strings.
- `tests/tool_schema_fidelity.rs`: guard test over the sacred schemas. Must pass unchanged.
- `extension/content.js` and `extension/agent-visual-indicator.js`: not part of this task.

Build and test:

- Run `cargo test` from the repo root. This task changes no Rust code, so all existing tests
  must keep passing with zero modifications.
- Extension changes are picked up only after the user reloads the extension at
  `chrome://extensions`.
- If you ever need to rebuild the binary and `target/debug/browser-mcp.exe` is locked by a
  running session, rename it aside first (for example:
  `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and rebuild.
  Binary or schema changes require an MCP client restart to observe; this task should need
  neither.

## Current behavior

All line numbers refer to `extension/service-worker.js` as it stands now.

- Line 14 declares the console buffer: `const consoleBuffer = new Map();` with the shape
  comment `tabId -> [{ level, text }]`. The buffer is keyed by tabId only.
- Lines 137-154 hold the single CDP event listener,
  `chrome.debugger.onEvent.addListener((src, method, params) => { ... })`. It handles
  exactly three methods:
  - `Runtime.consoleAPICalled` (lines 139-145): joins `params.args` into a text string and
    calls `pushCapped(consoleBuffer, tabId, { level: params.type || "log", text })`.
  - `Network.requestWillBeSent` (lines 146-147) and `Network.responseReceived`
    (lines 148-152): maintain the network buffer.
  - `Runtime.exceptionThrown` is not handled anywhere in the file. The event is silently
    dropped, so uncaught page errors never reach the console buffer.
- Lines 155-160 define `pushCapped(map, tabId, item)`, which appends and trims the array to
  the most recent 1000 entries.
- Lines 125-133 delete the tab's `consoleBuffer` entry when the tab is removed.
- Lines 512-526 define the `read_console_messages` tool handler:
  - Line 516 lazily enables the Runtime CDP domain on first call:
    `await enableDomain(a.tabId, "Runtime");`. Console and exception events therefore only
    flow after the first `read_console_messages` call for that tab. Do not change this.
  - Line 518 applies the `onlyErrors` filter:
    `if (a.onlyErrors) msgs = msgs.filter((m) => ["error", "exception"].includes(m.level));`.
    It already treats `"exception"` as an error level; verified, no fix needed here.
  - Lines 519-522 apply the optional `pattern` filter against both `m.text` and `m.level`.
  - Line 523 applies `limit` (default 100), line 524 handles `clear`, and line 525 renders
    each entry as `[${m.level}] ${m.text}` joined by newlines.

## Required behavior

Modify `extension/service-worker.js` only.

1. Add a branch for `Runtime.exceptionThrown` to the existing
   `chrome.debugger.onEvent.addListener` callback (the listener at lines 137-154), as an
   `else if` alongside the `Runtime.consoleAPICalled` branch. The branch must:
   - Read `params.exceptionDetails`; if it is missing, treat it as an empty object and still
     record an entry (never crash the listener).
   - Build one buffer entry of exactly the shape the console buffer already uses:
     `{ level: "exception", text: <one-line string> }`.
   - Store it with the exact same call pattern the `Runtime.consoleAPICalled` branch uses:
     `pushCapped(consoleBuffer, tabId, entry)`. If a separate task has already changed how
     the consoleAPICalled branch keys or clears the buffer by the time you start, mirror
     whatever that branch does; the two entry kinds must always share one buffer with one
     keying scheme.

2. Add a small helper function (name it `exceptionText`, taking the exceptionDetails object
   and returning a string) placed next to the listener, and build the entry text with these
   exact rules. The result must be a single line (no newline characters). Compose it from up
   to three space-separated parts, in this order:

   Part 1, the base message (always present), chosen by the first match:
   - If `exceptionDetails.exception` exists and its `description` is a non-empty string:
     use only the first line of that description (split on `"\n"`, take index 0).
     Descriptions for Error objects embed a multi-line stack; only the first line
     ("Error: message") belongs here.
   - Else if `exceptionDetails.exception` exists and its `value` is not `undefined`:
     use `String(exceptionDetails.exception.value)`. This covers thrown primitives such as
     `throw "boom"`, where description is absent.
   - Else if `exceptionDetails.text` is a non-empty string: use it.
   - Else: use the literal string `Uncaught exception`.

   Part 2, the source location (only when `exceptionDetails.url` is a non-empty string):
   - `(URL:LINE)` where LINE is `exceptionDetails.lineNumber + 1` when `lineNumber` is a
     number, because CDP line numbers are 0-based.
   - If `lineNumber` is not a number, emit `(URL)` with no line suffix.

   Part 3, the compact stack (only when `exceptionDetails.stackTrace` exists and its
   `callFrames` is a non-empty array):
   - Take at most the first 3 call frames.
   - Render each frame as `NAME@URL:LINE` where NAME is `frame.functionName` or the literal
     `<anonymous>` when functionName is empty or missing, URL is `frame.url`, and LINE is
     `frame.lineNumber + 1`.
   - Join the rendered frames with `, ` and wrap the whole thing as `[at FRAME1, FRAME2, FRAME3]`.

   Worked example. For `throw new Error("boom")` fired from function `start` at line 10 of
   `http://example.com/app.js`, the stored entry is:

       { level: "exception", text: "Error: boom (http://example.com/app.js:10) [at start@http://example.com/app.js:10]" }

   and `read_console_messages` renders it, via the existing line-525 formatter, as:

       [exception] Error: boom (http://example.com/app.js:10) [at start@http://example.com/app.js:10]

3. Read side: confirm the `onlyErrors` filter in `read_console_messages` (line 518) still
   matches level `"exception"`. It already does; make no change to it unless it has drifted,
   in which case restore `"exception"` to the accepted set. Do not alter the rendering
   format, the `pattern` filter, the `limit` default, or the `clear` semantics.

4. Do not enable the Runtime domain anywhere new. Event flow continues to start on the first
   `read_console_messages` call per tab, exactly as today.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS.
3. ASCII only in all code and docs: no em-dashes, no arrows, no curly quotes, anywhere,
   including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on public
   items, rustfmt clean, clippy with deny warnings. (This task should touch no Rust; if you
   believe it must, stop and reconsider, because it must not.)
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style. A one-line comment noting that CDP line numbers are 0-based is
   acceptable; a paragraph is not.
8. Do NOT copy code from the official Anthropic extension or any other project; implement
   the behavior described above from scratch.

Task-specific:

9. The only file you may change is `extension/service-worker.js`.
10. Exception entries must reuse the existing buffer, cap (via `pushCapped`), tab-removal
    cleanup, and rendering paths. Do not add parallel storage, new Maps, or new fields on
    buffer entries.
11. The entry text must be a single line. Never store raw multi-line descriptions or raw
    stack traces.

## Verification

1. `cargo test` from the repo root: all tests pass with no test file edited.
2. Ask the user to reload the extension at `chrome://extensions` (extension changes are not
   picked up otherwise). No MCP client restart is needed for an extension-only change, but
   the tool calls below must run through a connected MCP client.
3. Manual end-to-end check, in this order (order matters because the Runtime domain is only
   enabled by the first read):
   a. Navigate a tab in the MCP tab group to any page.
   b. Call `read_console_messages` once for that tab (this enables the Runtime domain).
   c. Call `javascript_tool` with text
      `setTimeout(() => { throw new Error("t13 test"); }, 0); "scheduled"`.
      The setTimeout matters: an exception thrown directly inside the evaluated expression
      is returned in the evaluate response and never emits `Runtime.exceptionThrown`; the
      deferred throw becomes a genuine uncaught page exception.
   d. Call `read_console_messages` with `onlyErrors: true`. The output must contain a line
      beginning `[exception] Error: t13 test` followed by a `(url:line)` location and a
      compact `[at ...]` stack (frame details depend on the page).
   e. Call `javascript_tool` with `console.log("t13 plain")` and then
      `read_console_messages` without `onlyErrors`. Confirm `[log] t13 plain` appears
      exactly once (no double counting, no regression for ordinary levels) alongside the
      exception entry.
4. Confirm `pattern` filtering still works: `read_console_messages` with
   `pattern: "t13 test"` returns the exception line and nothing else.

## Out of scope

- Network events of any kind. `Network.loadingFailed` and other network-side failure
  visibility belong to T14, not this task. Do not touch the `Network.requestWillBeSent` or
  `Network.responseReceived` branches or the network buffer.
- Formatting changes for other console levels. The `Runtime.consoleAPICalled` branch, its
  arg-joining logic, and the `[level] text` render format stay byte-identical.
- Changing when or where the Runtime CDP domain is enabled (no eager enabling at attach
  time, no enabling from other tools).
- Enabling the deprecated Console CDP domain. The file deliberately uses only the Runtime
  domain to avoid double counting; leave that decision alone.
- Re-keying, domain-scoping, or navigation-clearing of the console buffer. That is a
  separate task (T12). If it has already landed, mirror its storage call; do not extend it.
- Changing buffer entry shape for existing levels, adding structured `url` or `stackTrace`
  fields to entries, or adding new Maps or caches.
- Any change to `extension/content.js`, `extension/agent-visual-indicator.js`,
  `extension/manifest.json`, any Rust source, any test, or `src/mcp/schemas/tools.json`.
