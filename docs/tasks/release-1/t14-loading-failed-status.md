# T14: Network.loadingFailed marks requests failed instead of eternally pending

## Goal

The extension buffers network events per tab, but it only listens for
`Network.requestWillBeSent` and `Network.responseReceived`. A request that
fails (DNS failure, blocked by an ad blocker, aborted by the page) never gets
a status, so `read_network_requests` renders it as `(pending)` forever and the
agent cannot tell that a fetch actually died. Add a `Network.loadingFailed`
handler that marks the matching request with status 503 and records the CDP
error text, and extend the renderer to show that error text.

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

- `src/mcp/server.rs`: JSON-RPC loop in the binary.
- `src/mcp/schemas/tools.json`: SACRED. Byte-frozen official tool schemas.
  Never edit. Guarded by `tests/tool_schema_fidelity.rs`.
- `extension/service-worker.js`: CDP dispatch, screenshot pipeline,
  console/network buffers, keyboard/mouse dispatch. This is the ONLY file
  this task touches.
- `extension/content.js`: accessibility tree, find, form_input, page text.
  Not touched by this task.
- `extension/agent-visual-indicator.js`: phantom cursor and glow overlays.
  Not touched by this task.

Build and test: run `cargo test` from the repo root; all tests must pass.
This task changes only extension JavaScript, so no Rust rebuild is required,
but run `cargo test` anyway to prove nothing regressed. Extension changes
require the user to reload the extension at chrome://extensions to take
effect; the extension reconnects to the native host on its own after a
reload. If you ever need to rebuild the binary and
`target/debug/browser-mcp.exe` is locked by a running session, rename it
aside first (for example:
`mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and
rebuild.

## Current behavior

All facts below were verified by reading `extension/service-worker.js`.

- Line 15 declares the per-tab network buffer with a shape comment:

  ```js
  const networkBuffer = new Map(); // tabId -> [{ requestId, method, url, status, mimeType }]
  ```

- Lines 137-154: a single `chrome.debugger.onEvent.addListener` callback
  handles three events:
  - `Runtime.consoleAPICalled` (line 139) pushes into `consoleBuffer`.
  - `Network.requestWillBeSent` (lines 146-147) pushes
    `{ requestId, method, url, status: 0 }` via `pushCapped`.
  - `Network.responseReceived` (lines 148-152) finds the entry by
    `requestId` with `arr.find((r) => r.requestId === params.requestId)` and
    sets `status` and `mimeType` on it; if no entry exists it pushes a
    fallback entry with `method: "?"`.
  - There is NO branch for `Network.loadingFailed`. Failed requests keep
    `status: 0` forever.

- Lines 155-160: `pushCapped(map, tabId, item)` appends and caps each
  per-tab array at 1000 entries.

- Lines 527-536: the `read_network_requests` tool handler. It checks group
  membership, ensures attachment, enables the `Network` CDP domain, filters
  by `a.urlPattern` (substring match on `r.url`), slices to the last
  `a.limit || 100` entries, optionally clears on `a.clear`, and renders each
  entry on line 535 as:

  ```js
  `${r.method || "?"} ${r.url} ${r.status ? "-> " + r.status : "(pending)"}`
  ```

  When the filtered list is empty it returns the text
  `No network requests matching the pattern.`

- Line 131: the buffer for a tab is deleted when the tab is removed.

Net effect: `fetch("https://no-such-host.invalid/")` produces a
`Network.requestWillBeSent` event and then a `Network.loadingFailed` event
with `errorText: "net::ERR_NAME_NOT_RESOLVED"`, but our buffer never sees the
failure, so the line renders as
`GET https://no-such-host.invalid/ (pending)` indefinitely.

## Required behavior

Three changes, all in `extension/service-worker.js`.

1. Handle `Network.loadingFailed` in the `chrome.debugger.onEvent` listener.
   Add a new `else if` branch after the existing `Network.responseReceived`
   branch (after line 152), guarded the same way the sibling branches are:

   ```js
   } else if (method === "Network.loadingFailed" && params.requestId) {
   ```

   Inside the branch:
   - Look up the entry exactly like the `responseReceived` branch does:
     get the tab's array (`networkBuffer.get(tabId) || []`) and
     `find((r) => r.requestId === params.requestId)`.
   - If an entry is found:
     - Set `existing.status = 503;` unconditionally, even if a response was
       already received (a failure after headers still means the fetch
       died). 503 is the stand-in status the official extension uses so the
       model can tell a dead request from an in-flight one.
     - Set `existing.errorText = params.errorText;` but only when
       `params.errorText` is a non-empty string; otherwise leave the field
       unset. Example values: `net::ERR_BLOCKED_BY_CLIENT`,
       `net::ERR_NAME_NOT_RESOLVED`, `net::ERR_ABORTED`.
     - Set `existing.canceled = !!params.canceled;` (the CDP event carries a
       boolean `canceled` flag when the page aborted the request).
   - If no entry is found: do nothing. `Network.loadingFailed` carries no
     URL, so a synthetic entry could not render a useful line. Do NOT push a
     new entry in this branch.

2. Extend the renderer in `read_network_requests` (line 535). Exact output
   per entry:
   - Status set and `errorText` present:
     `<METHOD> <URL> -> <STATUS> (<ERRORTEXT>)`
     Example: `GET https://no-such-host.invalid/ -> 503 (net::ERR_NAME_NOT_RESOLVED)`
   - Status set, no `errorText`: unchanged.
     Example: `GET https://example.com/api -> 200`
   - Status still 0 (never completed, never failed): unchanged.
     Example: `GET https://example.com/slow (pending)`

   One correct implementation of the template expression:

   ```js
   `${r.method || "?"} ${r.url} ${r.status ? "-> " + r.status + (r.errorText ? " (" + r.errorText + ")" : "") : "(pending)"}`
   ```

   Everything else in the handler stays exactly as it is: the group check,
   `ensureAttached`, `enableDomain(a.tabId, "Network")`, the `urlPattern`
   substring filter, the `limit` slice, the `clear` behavior, the join with
   `"\n"`, and the empty-result message
   `No network requests matching the pattern.`

3. Update the shape comment on line 15 so it stays truthful. New comment:

   ```js
   const networkBuffer = new Map(); // tabId -> [{ requestId, method, url, status, mimeType, errorText, canceled }]
   ```

The `canceled` flag is stored for data fidelity but is not rendered; only
`errorText` appears in the output. Do not invent additional rendering for it.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or
   description strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction
   decisions in extension JS.
3. ASCII only in ALL code and docs: no em-dashes, no unicode arrows, no curly
   quotes, anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute
   behavior; when something failed or was recovered, say so in the tool
   result text. That is the point of this task: a dead request must be
   visibly dead, and the error text must accompany the stand-in 503 whenever
   the event provided one, so the agent does not mistake it for a real
   server 503.
5. No new runtime dependencies. Extension stays vanilla JS (no bundler, no
   libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc
   comments on public items, rustfmt clean, clippy with deny warnings. (No
   Rust changes are expected in this task.)
7. Comments only for constraints the code cannot express; match the
   surrounding comment density and style. The only comment change required
   is the buffer shape comment on line 15.
8. Do NOT copy code from the official Anthropic extension or any other
   project; implement the described behavior from scratch.

Task-specific:

9. Only `extension/service-worker.js` changes. No other file.
10. Do not touch the `Network.requestWillBeSent` or `Network.responseReceived`
    branches beyond adding the new sibling branch after them.
11. Do not rename `networkBuffer`, `pushCapped`, or any existing field.

## Verification

1. Run `cargo test` from the repo root. All tests must pass (this task adds
   no Rust code, so this confirms nothing regressed).
2. Ask the user to reload the extension at chrome://extensions. No MCP client
   restart is needed because the binary is unchanged; the reloaded extension
   reconnects to the native host on its own.
3. Manual end-to-end check through the MCP client:
   - Create or navigate a tab in the Browser MCP group to any page (for
     example https://example.com).
   - Call `read_network_requests` once first. This matters: network events
     are only buffered after the Network CDP domain is enabled, and the
     first call is what enables it.
   - Run via `javascript_tool`:
     `fetch("https://no-such-host-t14.invalid/").catch(() => "failed")`
   - Wait a moment, then call `read_network_requests` again. Expect a line
     of the form:
     `GET https://no-such-host-t14.invalid/ -> 503 (net::ERR_NAME_NOT_RESOLVED)`
   - Confirm ordinary successful requests still render like
     `GET https://example.com/ -> 200`.
   - Optional canceled check via `javascript_tool`:
     `(() => { const c = new AbortController(); fetch("https://example.com/", { signal: c.signal }).catch(() => "aborted"); c.abort(); return "ok"; })()`
     then expect the aborted request to render with `-> 503 (net::ERR_ABORTED)`.
   - Confirm a request that is genuinely still in flight (for example a
     fetch to a slow endpoint checked immediately) still renders with
     `(pending)`.

## Out of scope

- Response body capture. Do not call `Network.getResponseBody` or store
  bodies anywhere.
- Retry logic of any kind. The extension reports the failure; it never
  re-issues a request.
- Buffer keying changes. Task T12 owns how entries are keyed; do not change
  the find-by-requestId matching model, deduplication, the `pushCapped` cap,
  or how the buffer is keyed by tabId.
- No changes to console buffering, `read_console_messages`, or any other
  tool handler.
- No changes to the Rust binary, the tool schemas, `extension/content.js`,
  `extension/agent-visual-indicator.js`, or `extension/manifest.json`.
- Do not store or render `blockedReason`, `corsErrorStatus`, `type`, or any
  other `Network.loadingFailed` field beyond `errorText` and `canceled`.
- Do not change the `(pending)` wording, the `->` separator, or the
  empty-result message.
