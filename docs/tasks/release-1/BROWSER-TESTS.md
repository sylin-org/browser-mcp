# Deferred browser verification checklist

The unattended run cannot touch a live browser, so every verification step
that needs one accumulates here. Run this file top to bottom when you return.

## Before you start (once)

1. Close and restart the MCP client (Claude Code) so it launches the rebuilt
   binary from the release-1-hardening branch build.
   Note: if you want to test WITHOUT merging, build/install from the branch
   first; the registered binary path is what the client launches.
2. Reload the extension at chrome://extensions (Reload button on the dev
   extension).
3. Confirm basic liveness: ask the agent for a screenshot of any page. If
   that fails, run `browser-mcp doctor` (T07) and check --debug state files
   before proceeding.

## Format for entries (agent: follow exactly)

```
## T<NN>-<n>: <one-line purpose>
Changed: <what behavior changed, one sentence>
Steps:
1. <exact instruction, with URL and element>
2. ...
Expect: <the observable result that means PASS, per step where needed>
```

Entries below are appended by the unattended agent, in task order.

---

## T04-1: Fresh-session first-call warmup succeeds instead of racing the handshake
Changed: the binary now starts watching the extension channel at MCP `initialize`, and
`tools/call` waits up to 5s for the channel before failing (previously it failed instantly if
the handshake had not finished). A successful call that had to wait appends a trailing text
block: "(waited N.Ns for browser extension handshake)". This is a binary-only change (no
extension file touched); restarting the MCP client is required, no extension reload.
Steps:
1. Fully close Chrome (all windows), then close and relaunch the MCP client (Claude Code) so it
   starts a fresh mcp-server process.
2. Immediately (within a second or two of the client starting) launch Chrome with the extension
   enabled, and as soon as the MCP client is usable, issue a tool call, e.g. ask it to navigate
   to https://example.com.
Expect: the call succeeds (does not fail with "not connected"). If the handshake was still
settling when the call arrived, the tool result's last content block reads exactly like
"(waited 1.2s for browser extension handshake)" (digits vary). If the handshake had already
finished before the call, there is no such trailing note (the wait was 0, so `waited` stays
`None`).

## T04-2: Chrome fully closed -> exact bounded-timeout error text
Changed: same as T04-1; this exercises the failure path and its exact wording.
UPDATED by T06 (hop-attributed error reporting): the exact wording below supersedes the
original T04-2 text -- T06 replaced the ad hoc timeout message with the hop-attributed
`ToolError` contract; see T06-1 below for the fuller context.
Steps:
1. Fully close Chrome (all windows, ensure no background Chrome process is running the
   extension).
2. With Chrome still closed, start a fresh MCP client session and issue any tool call, e.g.
   navigate to https://example.com.
Expect: the call takes about 5 seconds, then returns an error result whose text is exactly:
"[hop: extension] Browser extension not connected. Next step: check chrome://extensions and
that Chrome is running."
(No extra "Error: " prefix -- errorness is carried by isError, not by a text prefix.)

## T06-1: Every tool-call failure names the hop that broke (binary only, no extension reload)
Changed: every tool-call failure text is now exactly
"[hop: <hop>] <message>. Next step: <next step>." where `<hop>` is one of invalid-request,
binary, ipc, extension, cdp, page. This replaces the old "Error: native messaging error: ..."
wrapper. This step needs only an MCP client restart (binary-only change).
Steps:
1. Close Chrome entirely (or otherwise ensure no extension is connected).
2. Restart the MCP client so it launches the rebuilt binary.
3. Call any tool, e.g. navigate to https://example.com.
Expect: after about 5s, the result text is exactly:
"[hop: extension] Browser extension not connected. Next step: check chrome://extensions and
that Chrome is running."
4. With the MCP client still running and Chrome still closed, call `tools/call` with a bogus
   tool name if your client lets you construct raw calls (otherwise skip this step; it is also
   covered by the automated test `unknown_tool_name_is_rejected_before_dispatch`).
Expect: an immediate (not ~5s) error result reading
"[hop: invalid-request] Unknown tool: <name>. Next step: call tools/list and use one of the
advertised tool names."

## T06-2: Stale `ref` on click / scroll_to / form_input is reported truthfully, not masked
Changed: previously a stale element `ref` (the page changed since `find`/`read_page` produced
it) either reported a misleading "coordinate or ref is required." success-shaped text, silently
substituted [0, 0] for `scroll`, reported a false "Scrolled to target." for `scroll_to`, or
returned form_input's content-script error as a SUCCESS text block prefixed "Error: ...". All
four now surface as a genuine `isError: true` result: "[hop: page] Element <ref> not found; the
page may have changed since it was read." (form_input instead echoes the content script's own
message verbatim, no added wording). Requires reloading the extension at chrome://extensions
AND restarting the MCP client.
Steps:
1. Reload the extension at chrome://extensions, then restart the MCP client.
2. Navigate a grouped tab to a simple static page (e.g. https://example.com) and call `find`
   with a query that matches the page heading; note the returned `ref` (e.g. `ref_1`).
3. Navigate the SAME tab away to a different URL (e.g. https://example.org) so the DOM the ref
   pointed at is gone.
4. Call `computer` with action `left_click` and the stale `ref` from step 2.
Expect: an `isError: true` result reading
"[hop: page] Element ref_1 not found; the page may have changed since it was read. Next step:
take a screenshot or call read_page to re-locate the element, then retry." (ref number varies).
5. Repeat steps 2-3, then call `computer` action `scroll_to` with the stale `ref`.
Expect: the same "[hop: page] Element ref_N not found; ..." error, NOT the previous "Scrolled to
target." success text.
6. Repeat steps 2-3 on a page with a form input, then call `form_input` with the stale `ref` and
   any `value`.
Expect: an `isError: true` result whose text is the content script's own message (e.g.
"Element ref_5 not found or was garbage-collected") with the hop prefix and next step appended,
NOT a "Error: ..." SUCCESS-shaped text block.

## T06-3: chrome:// page blocks content-script injection -> named page-hop failure
Changed: `read_page` (and other content-script-backed tools) on a page where script injection is
blocked (e.g. chrome:// pages) now fails with a named hop instead of an untagged rejection.
Requires reloading the extension AND restarting the MCP client.
Steps:
1. Reload the extension at chrome://extensions, then restart the MCP client.
2. Navigate a grouped tab to chrome://version.
3. Call `read_page` on that tab.
Expect: an `isError: true` result starting with either
"[hop: page] content script unavailable on this page (script injection blocked). Next step:
take a screenshot or call read_page to re-locate the element, then retry." or, if the debugger
attach itself is refused first, "[hop: cdp] debugger attach failed: ...". Either way the text
names a hop (page or cdp), never an untagged/opaque message.

## T06-4: Normal navigate + screenshot flow is unchanged
Changed: nothing on the success path; this is a regression check that hop-attributed error
plumbing did not disturb any success-text wording.
Steps:
1. With the extension reloaded and the MCP client restarted, navigate a grouped tab to
   https://example.com, then call `computer` action `screenshot` on that tab.
Expect: `navigate` returns "Navigated to https://example.com/." (or similar, unchanged wording);
`screenshot` returns the usual "Screenshot captured (jpeg)." text plus an image block, with no
"[hop: ...]" text anywhere and no `isError`.

## T04-3: Server stays responsive to `ping` while a tools/call is waiting
Changed: `tools/call` now runs on its own spawned task and no longer blocks the read loop, so
other protocol traffic (initialize, ping, subsequent calls) keeps flowing while one call is
waiting on the bounded 5s window.
Steps:
1. Start the mcp-server with `--debug` (or `BROWSER_MCP_DEBUG=1`) so the event log is available,
   with Chrome fully closed (so any call will hit the full 5s wait).
2. Pipeline two requests over stdio close together: a `tools/call` (which will wait ~5s), then a
   `ping`.
   (If your MCP client does not expose raw pipelining, this can also be checked by running
   `browser-mcp` directly and piping newline-delimited JSON-RPC requests into stdin by hand;
   see the requests shape used in tests/mcp_protocol.rs.)
Expect: the `ping` response arrives promptly (well under 5s), not only after the `tools/call`
response. Cross-check with `browser-mcp status --json` or the debug event log: the mcp_request
for `ping` is recorded and answered before the delayed `tools/call` response is written.
