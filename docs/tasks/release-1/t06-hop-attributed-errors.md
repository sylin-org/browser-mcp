# T06: Hop-attributed error reporting across the full dispatch path

## Goal

Today every tool-call failure reaches the MCP client as an opaque string prefixed
"Error: native messaging error: ...", no matter where it actually broke. Introduce a
typed error classification in the binary and a small error-tagging convention in the
extension so that every failure names the hop that broke (invalid-request, binary, ipc,
extension, cdp, or page) and suggests one concrete next step. The success path is
untouched.

## Project context

Browser MCP is governed browser automation. A single Rust binary is BOTH the MCP server
(JSON-RPC 2.0 over stdio, hand-rolled, tokio) AND the Chrome native-messaging host; a
thin Manifest V3 extension executes CDP commands. The architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe
(Windows) / Unix-domain-socket IPC. A tool call flows: MCP client -> JSON-RPC line on
stdin -> `src/mcp/server.rs` -> `Browser::call` in `src/browser.rs` -> framed native
message over the IPC -> native-host process (`src/native/ipc.rs::relay_native_host`) ->
Chrome native messaging -> `extension/service-worker.js` -> CDP or content script ->
reply back the same way.

Files involved in this task:

- `src/error.rs`: the crate's typed error module (thiserror). You add the new
  classification here.
- `src/browser.rs`: the `Browser` handle. `Browser::call` sends a tool request and
  awaits the correlated reply; `Browser::attach` runs the reader/writer for one
  connected native-host stream; `route_reply` parses extension replies.
- `src/mcp/server.rs`: the JSON-RPC loop. `handle_tools_call` turns a `Browser::call`
  failure into an MCP tool error result (a `{ content: [...], isError: true }` result,
  not a JSON-RPC error).
- `src/mcp/tools.rs`: exposes `TOOLS_JSON`, the embedded sacred tool-schema fixture.
- `src/native/messages.rs`: doc-only module describing the binary <-> extension wire
  protocol. Its docs must be updated to match the new wire shape.
- `extension/service-worker.js`: CDP dispatch. `fail(id, error)` posts
  `{ id, type: "tool_error", error }` back to the binary; `dispatch(id, tool, args)`
  wraps every handler in try/catch.
- `extension/content.js`: DOM mechanism (read_page, find, form_input, refCoordinates).
  NOT modified in this task; you only react to values it already returns.

Build and test:

- `cargo test` from the repo root runs unit + integration tests. All must pass.
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must be clean.
- If `target/debug/browser-mcp.exe` is locked by a running session, rename it aside
  first (for example `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`)
  and rebuild.
- Extension changes require the user to reload the extension at chrome://extensions.
  Binary changes require an MCP client restart to observe.

## Current behavior

Verified against the working tree; correct these line numbers if they have drifted.

Binary side:

- `src/error.rs` defines `enum Error` (lines 9-58) with string-payload variants
  including `NativeMessaging(String)` (Display: `native messaging error: {0}`) and
  `Ipc(String)` (Display: `ipc error: {0}`). There is no per-hop classification.
- `src/browser.rs` line 25: `TOOL_TIMEOUT` is 60 seconds. Line 28:
  `type CallResult = std::result::Result<Value, String>;`.
- `Browser::call` (src/browser.rs lines 72-115) maps EVERY failure to
  `Error::NativeMessaging`:
  - no native-host connected, or the outgoing channel send fails (lines 90-96):
    message `browser extension is not connected`;
  - the extension replied `tool_error` (line 101): the extension's error string;
  - the oneshot channel closed before a reply (lines 102-104): message
    `extension disconnected before responding`;
  - timeout after 60s (lines 105-108): message `tool request timed out`.
- `Browser::attach` (src/browser.rs lines 120-150) reads replies in a
  `while let Ok(Some(payload))` loop (line 139), so a clean peer close (`Ok(None)`)
  and a framing/transport read error (`Err`) exit the loop indistinguishably; the
  drain at lines 147-149 fails every pending call with the string
  `extension disconnected`.
- `route_reply` (src/browser.rs lines 153-173) treats a reply with
  `"type": "tool_error"` as `Err(<error string>)`, defaulting to
  `tool execution failed` when the `error` field is missing (lines 164-169). It reads
  no other fields from an error reply.
- `handle_tools_call` (src/mcp/server.rs lines 116-155): a missing or non-string
  `name` returns JSON-RPC error -32602 `tools/call requires a string 'name'`
  (lines 122-124). A `Browser::call` failure becomes
  `text_content(format!("Error: {e}"))` with `isError: true` (lines 146-153). Through
  the `Error::NativeMessaging` Display, a CDP failure today reads for example:
  `Error: native messaging error: computer failed: Cannot access a chrome:// URL`.
  The user cannot tell whether the extension was down, CDP rejected the command, or
  the page changed.
- `Browser::call` is invoked in exactly one production site, src/mcp/server.rs
  line 135, plus tests in src/browser.rs (lines 219, 244, 253) and
  src/native/ipc.rs (line 336, success path only).
- src/browser.rs test `call_surfaces_a_tool_error` (lines 227-248) asserts the error
  string contains `boom`; `call_without_a_connection_fails_fast` (lines 251-255)
  asserts it contains `not connected`.
- tests/mcp_protocol.rs (lines 80-90) asserts a tool call with no extension connected
  yields `isError: true` and text containing `not connected`.
- `src/native/messages.rs` documents the extension -> binary error shape as
  `{ "id", "type": "tool_error", "error": "<message>" }` (lines 14-17).

Extension side (`extension/service-worker.js`):

- `fail(id, error)` (lines 49-51) posts
  `{ id, type: "tool_error", error: String(error) }`. No hop information exists.
- `dispatch(id, tool, args)` (lines 558-566): unknown tool -> `Unknown tool: ${tool}`
  (line 560); any handler throw -> `${tool} failed: ${(e && e.message) || e}`
  (line 564).
- `ensureAttached` (lines 54-64) calls `chrome.debugger.attach` (line 59); a rejection
  (for example DevTools already attached) propagates untagged.
- `cdp(tabId, method, params)` (lines 114-117) calls `chrome.debugger.sendCommand`;
  a rejection propagates untagged, so the CDP method name is lost.
- `probeViewport` (lines 72-80) throws plain `new Error("failed to probe viewport")`
  (line 78) when the evaluate result is unusable.
- `content(tabId, message)` (lines 197-204) retries `chrome.tabs.sendMessage` after
  injecting content.js; if the injection itself fails (chrome:// pages, the Chrome Web
  Store), the raw rejection propagates untagged.
- `resolveCoords` (lines 278-287): when `args.ref` is provided but the content script
  cannot resolve it (element gone), it returns `null`, and the click path (line 379)
  reports the misleading text `coordinate or ref is required.` as a SUCCESS result.
  The scroll case (lines 405-415) silently substitutes `[0, 0]`.
- `scroll_to` (lines 416-421) ignores the content script's boolean result, so a stale
  ref still reports `Scrolled to target.`.
- The `form_input` handler (lines 499-504) converts a content-script error
  (`{ error: ... }` from `setFormValue` in content.js, for example
  `Element ref_5 not found or was garbage-collected.`) into a SUCCESS text result
  `Error: ${r.result.error}` (line 502).

## Required behavior

### 1. The failure format contract

Every tool-call failure returned to the MCP client (the `isError: true` tool result
text) is exactly:

    [hop: <hop>] <message>. Next step: <next step>.

- `<hop>` is one of: `invalid-request`, `binary`, `ipc`, `extension`, `cdp`, `page`.
- `<message>` is one sentence, specific, no trailing period (the formatter adds it).
  It may contain embedded error text after a colon.
- `<next step>` is one imperative clause, no trailing period.
- Total length one to two sentences plus the next step. Never include stack traces,
  JSON dumps, or multi-line detail in the result text; verbose detail goes to
  `tracing::debug!` only.

Two canonical examples (produce these byte-for-byte in the scenarios described below):

    [hop: extension] Browser extension not connected. Next step: check chrome://extensions and that Chrome is running.
    [hop: cdp] Input.dispatchMouseEvent failed: <detail from Chrome>. Next step: retry after taking a screenshot to re-ground coordinates.

### 2. The typed classification (src/error.rs)

Add to `src/error.rs` (below the existing `Error` enum; do not change `Error`):

```rust
/// A tool-call failure attributed to the dispatch hop that broke. Rendered for the
/// MCP client as: "[hop: <hop>] <message>. Next step: <next step>."
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("[hop: invalid-request] {message}. Next step: {next_step}.")]
    InvalidRequest { message: String, next_step: String },
    #[error("[hop: binary] {message}. Next step: {next_step}.")]
    Binary { message: String, next_step: String },
    #[error("[hop: ipc] {message}. Next step: {next_step}.")]
    Ipc { message: String, next_step: String },
    #[error("[hop: extension] {message}. Next step: {next_step}.")]
    Extension { message: String, next_step: String },
    #[error("[hop: cdp] {message}. Next step: {next_step}.")]
    Cdp { message: String, next_step: String },
    #[error("[hop: page] {message}. Next step: {next_step}.")]
    Page { message: String, next_step: String },
}
```

Provide one constructor per variant, each taking `message: impl Into<String>` and
filling in that hop's default next step, named exactly: `invalid_request`, `binary`,
`ipc`, `extension`, `cdp`, `page`. The default next steps are these exact strings:

| hop             | default next step                                                        |
|-----------------|--------------------------------------------------------------------------|
| invalid-request | `fix the tool arguments to match the advertised schema and retry`        |
| binary          | `retry the call; if it keeps failing, restart the MCP client and report a bug` |
| ipc             | `restart the MCP client so both browser-mcp processes restart and reconnect` |
| extension       | `check chrome://extensions and that Chrome is running`                   |
| cdp             | `retry after taking a screenshot to re-ground coordinates`               |
| page            | `take a screenshot or call read_page to re-locate the element, then retry` |

Also provide:

- `pub fn next_step(self, step: impl Into<String>) -> Self`: consumes the error and
  returns a copy with the next step replaced (immutable builder style; do not mutate
  in place).
- `pub fn from_extension_wire(hop: Option<&str>, message: String) -> Self`: maps a
  wire-level extension error to a variant: `Some("cdp")` -> `Cdp`, `Some("page")` ->
  `Page`, anything else including `None` -> `Extension`. Each with its default next
  step.

Every public item gets a doc comment. Re-export the type from `src/lib.rs` alongside
the existing `pub use error::{Error, Result};` (make it
`pub use error::{Error, Result, ToolError};`).

### 3. Binary-side mapping (src/browser.rs)

Change `type CallResult` to `std::result::Result<Value, ToolError>` and change the
signature of `Browser::call` to
`pub async fn call(&self, tool: &str, args: &Value) -> std::result::Result<Value, ToolError>`.
Update its doc comment (it currently claims `Error::NativeMessaging` is returned).
Map each failure site as follows (exact messages; next steps via the constructor
defaults unless an override is shown):

| site in `call` / `attach`                                   | error produced |
|--------------------------------------------------------------|----------------|
| `serde_json::to_vec` or `host::encode` fails                 | `ToolError::binary(format!("failed to encode the tool request: {e}"))` |
| no native-host connected / outgoing send fails               | `ToolError::extension("Browser extension not connected")` |
| oneshot channel closed before a reply                        | `ToolError::extension("Browser extension disconnected before responding").next_step("retry the call; the extension reconnects automatically")` |
| timeout after `TOOL_TIMEOUT`                                 | `ToolError::extension("Tool request timed out after 60s").next_step("check that Chrome is running and responsive, then retry")` |
| extension replied `tool_error`                               | `ToolError::from_extension_wire(hop, message)` (see 4) |

In `attach`, distinguish WHY the read loop ended. Replace the
`while let Ok(Some(payload))` loop with a loop that matches on
`host::read_message(...)`:

- `Ok(Some(payload))`: route as today.
- `Ok(None)` (clean peer close): break; drain every pending call with
  `ToolError::extension("Browser extension disconnected before responding").next_step("retry the call; the extension reconnects automatically")`.
- `Err(e)` (framing or transport error): break; log it with `tracing::warn!` and drain
  every pending call with `ToolError::ipc(format!("IPC transport failed: {e}"))`.

In `route_reply`, for a `tool_error` reply also read the optional string fields
`hop` and `detail`. If `detail` is present, log it with
`tracing::debug!(detail, "extension error detail")` and do NOT include it in the
`ToolError` (it must never reach the tool result text). Build the error with
`ToolError::from_extension_wire(hop, message)` where `message` is the `error` field,
defaulting to `tool execution failed` when absent (as today).

The `self.debug.tool_end(...)` calls keep working: pass `&e.to_string()` for the
error case as today.

### 4. Extension-side structured errors (extension/service-worker.js)

The extension stays policy-free: it attributes MECHANISM failures (which layer threw),
never makes access or redaction decisions.

Add a tagging helper near `fail` (mechanism only; keep the surrounding comment
density):

```js
function hopError(hop, message, detail) {
  const err = new Error(message);
  err.hop = hop;
  if (detail) err.detail = String(detail);
  return err;
}
```

Change `fail(id, error)` to emit the structured wire shape. `error` on the wire stays
a string; `hop` and `detail` are optional extra fields:

```js
function fail(id, error) {
  const msg = { id, type: "tool_error", error: (error && error.message) || String(error) };
  if (error && error.hop) msg.hop = error.hop;
  if (error && error.detail) msg.detail = error.detail;
  try { nativePort && nativePort.postMessage(msg); } catch { /* port gone */ }
}
```

Change `dispatch`'s catch so tagged errors pass through as-is and untagged errors keep
the current tool-name prefix:

```js
} catch (e) {
  if (e && e.hop) fail(id, e);
  else fail(id, `${tool} failed: ${(e && e.message) || e}`);
}
```

The extension only ever sets `hop` to `"cdp"` or `"page"`; an absent `hop` means the
binary classifies it as `extension` (its own internal errors need no tag). Tag these
exact sites:

- `cdp(tabId, method, params)`: wrap the `chrome.debugger.sendCommand` call in
  try/catch and rethrow
  `hopError("cdp", `${method} failed: ${(e && e.message) || e}`)`. This is what makes
  the canonical `[hop: cdp] Input.dispatchMouseEvent failed: ...` message possible.
- `ensureAttached`: wrap `chrome.debugger.attach` and rethrow
  `hopError("cdp", `debugger attach failed: ${(e && e.message) || e}`)`.
- `probeViewport`: replace `throw new Error("failed to probe viewport")` with
  `throw hopError("page", "failed to probe viewport")`.
- `content(tabId, message)`: wrap the fallback branch (inject then re-send) in
  try/catch; on failure throw
  `hopError("page", "content script unavailable on this page (script injection blocked)", (e && e.message) || e)`.
  The original rejection text travels in `detail`, not in the message.
- `resolveCoords`: when `args.ref` is provided but the content script returns no
  coordinates, throw
  `hopError("page", `Element ${args.ref} not found; the page may have changed since it was read`)`
  instead of falling through to `null`. The neither-coordinate-nor-ref case keeps the
  existing `coordinate or ref is required.` text result unchanged. Note this also
  fixes the `scroll` action, which previously substituted `[0, 0]` for a stale ref;
  substituting a fake position violates the truthfulness rule.
- `scroll_to`: when `a.ref` is provided, check the content script's reply; if it
  reports the ref was not found (falsy result), throw
  `hopError("page", `Element ${a.ref} not found; the page may have changed since it was read`)`
  instead of reporting `Scrolled to target.`.
- `form_input` handler: replace
  `if (r && r.result && r.result.error) return text(`Error: ${r.result.error}`);` with
  `if (r && r.result && r.result.error) throw hopError("page", r.result.error);`.
  The content-script message (for example
  `Element ref_5 not found or was garbage-collected.`) becomes the hop message
  verbatim; strip nothing, add nothing. This is a failure that was previously
  masquerading as a success result, so converting it is in scope. If the content
  script message ends with a period, trim exactly one trailing period before passing
  it to `hopError` so the rendered text does not double the period.

Update the service worker's top-of-file comment (lines 4-6) to document the new reply
shape: `{ id, type: "tool_error", error, hop?, detail? }`.

### 5. Unknown-tool pre-check (src/mcp/server.rs + src/mcp/tools.rs)

Add to `src/mcp/tools.rs`:

```rust
/// True when `name` is one of the advertised tool names in the sacred fixture.
pub fn is_known_tool(name: &str) -> bool
```

Implement it by parsing `TOOLS_JSON` and checking the `name` field of each entry in
the `tools` array (read-only use of the fixture; the fixture itself is never edited).
A simple parse per call is fine (tools/list already re-parses per call).

In `handle_tools_call`, immediately after extracting `name` and `args` and BEFORE the
`dispatch::policy_check` line, return an `isError: true` tool result for unknown
names, built from
`ToolError::invalid_request(format!("Unknown tool: {name}")).next_step("call tools/list and use one of the advertised tool names")`.
This replaces the round trip that previously let the extension answer
`Unknown tool: ...` (or, worse, reported `not connected` when no extension was up).
Keep the extension's own unknown-tool guard in `dispatch` as a safety net; do not
remove it.

### 6. Rendering in the tool result (src/mcp/server.rs)

In the `Err(e)` branch of `handle_tools_call`, the result text becomes exactly
`e.to_string()` (the `[hop: ...]` format). Remove the `Error: ` prefix; keep the
`isError: true` mechanism unchanged. The success branch (including the read_page
redaction call) is byte-identical to today.

### 7. Wire protocol documentation (src/native/messages.rs)

Update the module doc's extension -> binary section to:

```json
{ "id": "<string>", "type": "tool_response", "result": { "content": [ ... ] } }
{ "id": "<string>", "type": "tool_error",    "error":  "<message>", "hop": "<cdp|page>", "detail": "<string>" }
```

with a sentence stating `hop` and `detail` are optional, `hop` is only ever `cdp` or
`page` (absent means the binary attributes the error to the extension itself), and
`detail` is debug-log-only material that must never appear in tool results.

### 8. Tests

All existing tests must keep passing; the two substring assertions that exist today
(`boom`, `not connected`) already survive the new format by construction. Add or
update the following:

- `src/error.rs` unit tests (`#[cfg(test)] mod tests`):
  - the Display of `ToolError::extension("Browser extension not connected")` equals
    exactly
    `[hop: extension] Browser extension not connected. Next step: check chrome://extensions and that Chrome is running.`
  - one Display assertion per remaining variant checking the `[hop: <name>]` prefix
    and the default next step;
  - `from_extension_wire`: `Some("cdp")` renders with prefix `[hop: cdp]`,
    `Some("page")` with `[hop: page]`, `None` and `Some("bogus")` with
    `[hop: extension]`;
  - `next_step(...)` replaces the default in the rendered string.
- `src/browser.rs` tests:
  - update `call_without_a_connection_fails_fast` to assert the message starts with
    `[hop: extension]` and still contains `not connected`;
  - update `call_surfaces_a_tool_error` (fake sends no hop) to assert the message
    starts with `[hop: extension]` and contains `boom`;
  - add a test where the fake extension replies
    `{ "id": ..., "type": "tool_error", "error": "Input.dispatchMouseEvent failed: no target", "hop": "cdp", "detail": "verbose internals" }`
    and assert the error string starts with `[hop: cdp]`, contains
    `Input.dispatchMouseEvent failed`, and does NOT contain `verbose internals`;
  - add a test with `"hop": "page"` asserting the `[hop: page]` prefix.
- `src/mcp/tools.rs` unit test: `is_known_tool("navigate")` is true,
  `is_known_tool("bogus_tool")` is false.
- `tests/mcp_protocol.rs`:
  - strengthen the existing no-extension assertion to also check the text starts with
    `[hop: extension]`;
  - add a test sending `tools/call` with name `bogus_tool` (after `initialize`) and
    assert the response has `isError: true` and text starting with
    `[hop: invalid-request]` and containing `Unknown tool: bogus_tool` (this must pass
    with no extension connected, proving the pre-check runs before dispatch).

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged. Reading tool names
   out of the embedded fixture is allowed; editing it is not.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS. Hop tagging is mechanism (which layer threw), not policy.
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly quotes,
   anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute behavior;
   when something failed or was recovered, say so in the tool result text. In
   particular, do not keep the `[0, 0]` substitution for a stale scroll ref and do
   not report `Scrolled to target.` when the target ref no longer resolves.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no
   libraries). The Rust side uses only crates already in Cargo.toml.
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on
   public items, rustfmt clean, clippy with deny warnings.
7. Comments only for constraints the code cannot express; match the surrounding
   comment density and style.
8. Do NOT copy code from the official Anthropic extension or any other project;
   implement the described behavior from scratch.
9. Task-specific: error messages in tool results are one to two sentences; stack
   traces and verbose detail go to `tracing::debug!`/`tracing::warn!` only. The
   `detail` wire field must never appear in a tool result.
10. Task-specific: the hop names, message strings, next-step strings, and the
    `[hop: <hop>] <message>. Next step: <next step>.` format are a contract; produce
    them exactly as specified above.
11. Task-specific: do not change `TOOL_TIMEOUT`, the reconnect/keepalive logic, the
    IPC transport, or `src/dispatch.rs` (the policy/audit seams stay as they are).

## Verification

1. `cargo fmt` then `cargo clippy --all-targets -- -D warnings` from the repo root:
   both clean.
2. `cargo test` from the repo root: all tests pass, including the unchanged
   `tests/tool_schema_fidelity.rs` and the new/updated tests listed above. If
   `target/debug/browser-mcp.exe` is locked by a running session, rename it aside
   (`mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and rebuild.
3. Manual, binary only (requires an MCP client restart to pick up the new binary):
   with Chrome closed, call any tool and confirm the result text is exactly
   `[hop: extension] Browser extension not connected. Next step: check chrome://extensions and that Chrome is running.`
4. Manual, full path (requires the user to reload the extension at
   chrome://extensions AND restart the MCP client): with a tab in the Browser MCP
   group,
   - `find` an element, navigate away, then `computer` `left_click` with the stale
     ref: expect a `[hop: page] Element ref_N not found; ...` result;
   - `form_input` with a stale ref: expect `[hop: page]` and the content-script
     message;
   - navigate the grouped tab to `chrome://version`, then `read_page`: expect
     `[hop: page] content script unavailable on this page (script injection blocked). ...`
     (or the equivalent cdp attach failure if the debugger refuses first; either way
     the hop must be named);
   - a normal `navigate` + `computer` `screenshot` flow: confirm all success texts
     are unchanged from before this task.

## Out of scope

- The `doctor` subcommand and any startup diagnostics (that is T07). Do not touch
  `src/install/` or `src/main.rs`.
- Retry logic of any kind. This task labels failures; it never retries them.
- Changing success-path result text. Every current success confirmation stays
  byte-identical. Specifically leave unchanged: the `coordinate or ref is required.`
  text when neither argument is given, the `Tab N is not in the ... group.` texts,
  `Invalid URL: ...`, the `javascript_tool` result text for page JS exceptions
  (`Error: ...` from `exceptionDetails`, which is the truthful result of running the
  JS), the `Could not read the page.` / `Could not extract page text.` fallbacks, and
  the read_page `ref_id` error text produced inside content.js.
- `extension/content.js` and `extension/agent-visual-indicator.js`: no changes.
- JSON-RPC protocol errors keep their current codes and messages: -32600 for missing
  method, -32601 for unknown methods, and -32602
  `tools/call requires a string 'name'` for a missing name. Only tool RESULTS carry
  the hop format.
- No new wire message types and no changes to the native-messaging framing or the
  IPC transport; the only wire change is the optional `hop` and `detail` fields on
  `tool_error`.
- No changes to `src/dispatch.rs`, `src/policy/`, `src/debug.rs`, or the redaction
  overlay call in the success branch.
- No renaming or restructuring of the existing `Error` enum in `src/error.rs`; the
  new `ToolError` sits beside it.
