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
Steps:
1. Fully close Chrome (all windows, ensure no background Chrome process is running the
   extension).
2. With Chrome still closed, start a fresh MCP client session and issue any tool call, e.g.
   navigate to https://example.com.
Expect: the call takes about 5 seconds, then returns an error result whose text is exactly:
"Browser extension not connected after 5s. Check that Chrome is running with the extension
enabled; run with --debug and inspect the status files."
(No extra "Error: " prefix -- errorness is carried by isError, not by a text prefix.)

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
