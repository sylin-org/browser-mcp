# T16: javascript_tool REPL semantics and 50KB output cap

## Goal

Make the `javascript_tool` handler in the extension deliver the REPL semantics its schema
already promises: top-level `await` works, and the value of the last expression is returned.
Add a one-shot async-IIFE retry for code that uses a top-level `return`, and cap the
stringified result at 50KB so a huge return value cannot flood the model's context window.

## Project context

Browser MCP is a governed browser automation system. A single Rust binary is BOTH the MCP
server (JSON-RPC 2.0 over stdio, hand-rolled on tokio) AND the Chrome native-messaging host;
a thin Manifest V3 extension executes CDP commands. The two binary roles run as separate OS
processes bridged by tokio-native named-pipe/UDS IPC.

```
MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser
```

Call flow for this task: the MCP client calls the `javascript_tool` tool; the binary relays
`{ id, type: "tool_request", tool: "javascript_tool", args }` to the extension over native
messaging; in `extension/service-worker.js` the `dispatch()` function (line 558) looks up
`handlers.javascript_tool` (line 505) and sends back either
`{ id, type: "tool_response", result }` or `{ id, type: "tool_error", error }`. Tool results
use the MCP content envelope built by the `text()` helper (lines 207-209):
`{ content: [{ type: "text", text: t }] }`.

Files involved:

- `extension/service-worker.js` -- the ONLY file you will change. Vanilla JS, no build step,
  no bundler, no libraries. The `javascript_tool` handler is at lines 505-511.
- `src/mcp/schemas/tools.json` -- SACRED, byte-frozen official Claude-in-Chrome v1.0.78 tool
  schemas. NEVER edit. The `javascript_tool` entry (lines 188-210) is the contract this task
  implements.
- `tests/tool_schema_fidelity.rs` -- guard test that must keep passing unchanged.
- `src/browser.rs` -- read-only for this task; it holds the per-call timeout you must not
  touch (see Current behavior).

Build and test:

- `cargo test` from the repo root must pass (no Rust changes in this task; running it
  confirms you did not accidentally touch the schema or Rust code).
- Extension changes are picked up only after the user reloads the extension at
  `chrome://extensions`. No MCP client restart is needed for an extension-only change; the
  service worker reconnects to the native host on its own (keepalive alarm).
- If you ever need to rebuild the binary and `target/debug/browser-mcp.exe` is locked by a
  running session, rename it aside first (for example
  `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and rebuild. This
  task should not require a rebuild.

## Current behavior

Verified in `extension/service-worker.js`:

- Lines 505-511, the current handler:

```js
async javascript_tool(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    const r = await cdp(a.tabId, "Runtime.evaluate", { expression: a.text, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) return text(`Error: ${r.exceptionDetails.text || "exception"}`);
    const v = r.result;
    return text(v.value !== undefined ? JSON.stringify(v.value) : (v.description || String(v.type)));
  },
```

- Line 507 calls `Runtime.evaluate` with `returnByValue: true` and `awaitPromise: true` but
  WITHOUT `replMode`, and there is no fallback of any kind. Consequences:
  - Top-level `await` is a SyntaxError in a plain (non-REPL) `Runtime.evaluate`, so code like
    `await fetch(url).then(r => r.json())` fails even though the schema explicitly tells the
    model to write exactly that.
  - A top-level `return 7` fails with an "Illegal return statement" SyntaxError and the
    handler gives up.
- Lines 509-510: the success path stringifies the result with no size limit. A large value
  (for example `document.documentElement.outerHTML` on a heavy page) is returned whole and
  floods the context window.
- Line 508: the exception path returns `Error: <exceptionDetails.text or "exception">`.
- The schema (`src/mcp/schemas/tools.json`, entry at lines 188-210) promises in the `text`
  parameter description: "Evaluated in the page context with REPL semantics: top-level
  `await` works, and the result of the last expression is returned automatically". The
  current handler does not honor this.
- Timeout: the extension sets NO timeout on `Runtime.evaluate`. The only timeout is in the
  binary: `src/browser.rs` line 25 defines `const TOOL_TIMEOUT: Duration =
  Duration::from_secs(60);`, applied in `Browser::call` at line 99 via
  `tokio::time::timeout`; on expiry the MCP client receives the error
  "tool request timed out". This behavior stays exactly as it is.
- `dispatch()` (lines 558-565) wraps any exception thrown by a handler as a `tool_error`
  with message `<tool> failed: <message>`. This also stays as it is.
- The `cdp()` helper (line 114) attaches the debugger if needed and forwards to
  `chrome.debugger.sendCommand`.

## Required behavior

Rewrite the body of `handlers.javascript_tool` in `extension/service-worker.js` to the
following exact logic. Everything not listed here stays byte-identical.

1. Keep the `inGroup` guard on the first line exactly as it is today.

2. First attempt: call `Runtime.evaluate` with `replMode: true` added to the existing
   parameters. The full parameter object is:

```js
{ expression: a.text, returnByValue: true, awaitPromise: true, replMode: true }
```

   `replMode` gives both promised semantics: top-level `await` compiles, and the completion
   value of the last expression is returned. `awaitPromise: true` must stay because REPL-mode
   evaluation produces a promise.

3. Illegal-return fallback: if the first attempt's response has `exceptionDetails`, build a
   probe string by concatenating `r.exceptionDetails.text` and, when present,
   `r.exceptionDetails.exception.description` (either field can carry the message depending
   on how the error surfaced; tolerate both being missing). If that probe string contains the
   substring `Illegal return statement`, retry EXACTLY ONCE with the user code wrapped in an
   async IIFE. The retry expression is built with newlines around the user code so a trailing
   line comment cannot swallow the closing tokens:

```js
"(async () => {\n" + a.text + "\n})()"
```

   Retry parameters: `{ expression: <wrapped>, returnByValue: true, awaitPromise: true }`.
   Do NOT pass `replMode` on the retry (the IIFE itself provides `await` support and makes
   `return` legal). The retry's response replaces the first response for all further steps.
   Never retry more than once, even if the retry itself also reports
   "Illegal return statement".

4. Exception result: if the final response (first attempt when no retry fired, otherwise the
   retry) has `exceptionDetails`, return exactly what the handler returns today:

```js
text(`Error: ${r.exceptionDetails.text || "exception"}`)
```

5. Success result: compute the output string exactly as today:

```js
const v = r.result;
let out = v.value !== undefined ? JSON.stringify(v.value) : (v.description || String(v.type));
```

   Then apply the 50KB cap: if `out.length > 50 * 1024` (51200, measured with the string's
   `.length`, that is UTF-16 code units; do not add byte-level accounting), truncate and
   append the marker so the final string is:

```js
out.slice(0, 50 * 1024) + "\n[OUTPUT TRUNCATED: Exceeded 50KB limit]"
```

   The marker text `[OUTPUT TRUNCATED: Exceeded 50KB limit]` must appear verbatim,
   preceded by a single newline. Return `text(out)`.

6. On success after a fallback retry, do NOT append any note about the retry. The retry is
   how the advertised contract is delivered, not a substitute behavior; the truthfulness rule
   is satisfied because the code the model wrote ran and its value is returned. Genuine
   failures still surface through step 4.

7. Timeout: unchanged. Do not pass a `timeout` parameter to `Runtime.evaluate`, do not add
   any timer in the extension, and do not touch `TOOL_TIMEOUT` in `src/browser.rs`. The
   binary's 60-second whole-call timeout covers the worst case fine: an
   "Illegal return statement" failure is a compile-time error that returns immediately, so
   the retry has essentially the full window.

You may extract the shared evaluate-and-inspect step into one small helper function placed
next to the handler if it keeps the handler under control, but the observable behavior must
match the steps above exactly.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS. The 50KB cap is a mechanical size limit, not content inspection; do not
   filter or rewrite output based on what it contains.
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly quotes, anywhere,
   including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text. For this task the
   exception path (step 4) is that truth channel; keep it intact.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on public
   items, rustfmt clean, clippy with deny warnings. (This task should not touch Rust at
   all; the rule applies if you find yourself editing Rust, which you must not.)
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style (this file uses sparse `//` comments).
8. Do NOT copy code from the official Anthropic extension or any other project; implement
   the described behavior from scratch.

Task-specific constraints:

9. Change ONLY the `javascript_tool` handler (plus at most one small adjacent helper) in
   `extension/service-worker.js`. Do not touch any other handler, the screenshot pipeline,
   the buffers, `dispatch()`, `text()`, or `cdp()`.
10. Keep the handler inside the `handlers` object literal and keep it returning the `text()`
    envelope. Errors thrown out of the handler must still propagate to `dispatch()`'s catch.

## Verification

1. `cargo test` from the repo root: all tests pass (proves the schema and Rust side are
   untouched).
2. Optional parse check of the changed file: `node --check extension/service-worker.js`
   (parse only; `chrome` and `self` are runtime globals and do not matter here).
3. Manual end-to-end (the user must reload the extension at `chrome://extensions` first; no
   MCP client restart is needed). From an MCP client, on a normal web page tab in the group,
   run `javascript_tool` with each of these `text` values and confirm:
   - `1 + 1` returns `2`.
   - `const a = { x: 1 }; a.x + 41` returns `42` (last-expression value).
   - `await new Promise(r => setTimeout(() => r("done"), 100))` returns `"done"`
     (top-level await now works).
   - `return 7` returns `7` (async-IIFE fallback fired; no extra note in the output).
   - `"x".repeat(200000)` returns a string of about 51.2K characters ending with
     `[OUTPUT TRUNCATED: Exceeded 50KB limit]`.
   - `nosuchvariable.foo` returns a message starting with `Error: ` (exception path
     unchanged).
   - `document.title` still returns the page title (regression check for the plain path).

## Out of scope

- Output sanitization, redaction, DLP, or any content inspection of the result. The spec
  explicitly excludes content inspection, and redaction decisions belong to the binary's
  policy layer, never to the extension.
- Console capture: do not collect `console.log` output produced by the evaluated code and
  do not attach console messages to the `javascript_tool` result. The separate
  `read_console_messages` tool owns that.
- Schema text: the `javascript_tool` entry in `src/mcp/schemas/tools.json` already
  describes the required semantics. Do not edit it or any other schema entry.
- Changing the exception message format beyond step 4, adding stack traces, or improving
  error wording. Keep `Error: <text or "exception">` as is.
- Adding a `timeout` parameter to `Runtime.evaluate`, extension-side timers, or changes to
  `TOOL_TIMEOUT` in `src/browser.rs`.
- Execution-context changes: no isolated worlds, no `contextId` selection, no
  `Page.createIsolatedWorld`. Evaluation stays in the page's main world as today.
- Any edits to `extension/content.js`, `extension/agent-visual-indicator.js`, other
  handlers in `service-worker.js`, or any Rust file.
