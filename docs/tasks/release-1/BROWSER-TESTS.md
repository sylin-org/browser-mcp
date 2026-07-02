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

## T07-1: `browser-mcp doctor` with no MCP session running reports the no-server problem, exit 1
Changed: `doctor` is now a fused, one-shot diagnosis (Binary / Browsers / MCP clients / IPC
endpoint / Debug sessions / Verdict sections) instead of registration-state-only output, and it
now returns a truthful exit code (0 healthy, 1 any problem found). Binary-only change; rebuild
the binary first (rename `target/debug/browser-mcp.exe` aside if a running session holds it
locked, then rebuild). No extension reload needed for this step (no MCP client needs to be
running at all).
Steps:
1. Ensure no MCP client / mcp-server process is running (close the MCP client, or otherwise make
   sure nothing owns the `org.sylin.browser_mcp.v1` IPC endpoint).
2. Run `browser-mcp doctor` from a shell.
3. Check the exit code (`echo $?` in bash, `$LASTEXITCODE` in PowerShell).
Expect: the report shows all six sections in order (Binary, Browsers, MCP clients, IPC endpoint,
Debug sessions, Verdict). The IPC endpoint `state` line reads
"absent (no mcp-server currently owns it)". The Verdict section has at least one
"  problem: no mcp-server is running (the IPC endpoint does not exist): ..." line. The exit code
is 1.

## T07-2: `browser-mcp doctor` during a healthy debug session reports OK, exit 0
Changed: same fusion as T07-1; this exercises the healthy path, including the new clientInfo
capture (Part B) and the extension-connected signal. Requires the dev install to register the
server with `BROWSER_MCP_DEBUG=1` (or manually restart the MCP client with `BROWSER_MCP_DEBUG=1`
set in its environment) and the extension reloaded/attached at least once.
Steps:
1. Restart the MCP client so it launches the rebuilt binary with debug mode on (`--debug` or
   `BROWSER_MCP_DEBUG=1`).
2. Reload the extension at chrome://extensions if it was not already loaded, and make one tool
   call (e.g. navigate to https://example.com) so the extension attaches.
3. Run `browser-mcp doctor` from a shell.
4. Check the exit code.
Expect: IPC endpoint `state` reads
"accepts connections (doctor made one brief probe connection)". Under "Debug sessions", the
newest `mcp-server` row shows `client <name> <version>` where `<name>`/`<version>` match what the
MCP client reports in its `initialize` request (e.g. "claude-code" and its version), NOT
"(not recorded)", and `extension connected` (not "not connected"). The Verdict section is exactly
one line: "  OK: mcp-server (pid <pid>) is running, the extension is connected, and the IPC
endpoint accepts connections." Exit code is 0.

## T07-3: `browser-mcp doctor` catches a disconnected extension, then recovers
Changed: same fusion; this exercises Verdict rule 6 (extension disconnected from a live
mcp-server). No rebuild needed beyond T07-2's if already done.
Steps:
1. With the debug session from T07-2 still running (mcp-server up, extension was connected),
   disable the extension at chrome://extensions (or otherwise stop its service worker) and wait a
   few seconds for the mcp-server to observe the disconnect.
2. Run `browser-mcp doctor`.
3. Re-enable the extension at chrome://extensions, make one more tool call so it reattaches, then
   run `browser-mcp doctor` again.
Expect step 2: a Verdict problem line naming the mcp-server's pid, either
"the extension is disconnected from the mcp-server (pid <pid>; it connected <n> time(s) earlier
in this session): ..." (if it had connected before) -- exit code 1.
Expect step 3: doctor returns to the single "OK: ..." Verdict line and exit code 0.

## T07-4: `browser-mcp doctor --verbose` shows every session with its counters
Changed: `--verbose` (previously ignored by the installer's doctor) now lifts the 6-row display
cap on the Debug sessions section and prints a `counters:` line under every row.
Steps:
1. With at least one debug session on record (from T07-2/T07-3), run
   `browser-mcp doctor --verbose`.
Expect: every session row (not just the newest 6) is shown, with no
"(and N older; use --verbose to show all)" line, and each session row is immediately followed by
a line reading
"      counters: requests=<n> tools=<n> errors=<n> frames_out=<n> frames_in=<n> connects=<n>
disconnects=<n>" with real (non-placeholder) numbers.

## T07-5: `browser-mcp status` still works during a debug session (role filtering regression check)
Changed: `status_report()` is now role-aware (only reports mcp-server sessions); this confirms
that filtering did not silently break the existing `status` command.
Steps:
1. With the debug session from T07-2 running, run `browser-mcp status` (no flags).
Expect: the usual formatted report (pid, uptime, extension connected/not, counters, in-flight,
recent events) renders exactly as before this change -- no "no mcp-server debug state" message
while a real session is live.

## T07-6 (optional): native-host debug state file and the extension-last-seen line
Changed: the native-host role now writes its own `debug-state-<pid>.json` / `debug-events-<pid>.jsonl`
files, but only when Chrome itself was launched with `BROWSER_MCP_DEBUG=1` set in its environment
(Chrome does not pass `--debug` to the process it spawns, so this is opt-in and its absence is
normal -- do not treat a missing native-host row as a problem).
Steps:
1. Fully close Chrome.
2. Launch Chrome from a shell with `BROWSER_MCP_DEBUG=1` set in that shell's environment (so the
   native-host process Chrome spawns inherits it), with the extension enabled.
3. Make one tool call from the MCP client so the extension attaches.
4. Run `browser-mcp doctor` (with the mcp-server also in debug mode, per T07-2, for the fullest
   picture).
Expect: the Debug sessions section includes a `native-host` row
("  native-host   pid <pid>  started <S> ago  active <A> ago", no client/extension fields on that
row), and, after the session rows, a line reading
"  extension last seen <A> ago (native-host pid <pid>)". Separately, confirm launching Chrome
WITHOUT `BROWSER_MCP_DEBUG=1` (the normal case) produces no native-host row and no problem line
about its absence.

## T01-1: Small page still renders byte-identical read_page output
Changed: `accessibilityTree` in extension/content.js was rewritten from a serialize-as-you-walk
design to a two-pass measure/emit design (structural pagination). When output fits the character
budget the intent is byte-identical output to before this change (same lines, same order, same
refs, no markers, no summary line). Extension-only change: requires reloading the extension at
chrome://extensions; no MCP client restart needed.
Steps:
1. Reload the extension at chrome://extensions.
2. Navigate a grouped tab to https://example.com.
3. Call `read_page` with only `tabId` set (all other args default: filter="all", depth=15,
   max_chars=50000).
Expect: a short accessibility tree (heading, paragraph, link, etc.), each shown line ending in
`[ref_N]`, no lines containing "[subtree collapsed:", no line starting with "[element cap
reached:" or "[showing", no "... (truncated)" anywhere, and the output ends with a blank line
then "Viewport: WxH". If you have a pre-change capture of this exact call, diff them: they should
be identical.

## T01-2: Large page triggers structural pagination with collapse markers and a summary line
Changed: same as T01-1; this exercises the over-budget path.
Steps:
1. With the extension reloaded, navigate a grouped tab to
   https://en.wikipedia.org/wiki/Web_browser.
2. Call `read_page` with `max_chars: 2000` (defaults for everything else).
Expect: only complete lines (no line is cut mid-word, no "... (truncated)" string anywhere), one
or more lines matching exactly
"<indent>  [subtree collapsed: <N> elements; call read_page with ref_id=ref_<M> to expand]"
(N and M vary), followed near the end by a line matching exactly
"[showing <M> of <T> elements; expand a collapsed subtree with ref_id, or narrow with
filter=\"interactive\" or a smaller depth]" (M <= T, both plausible integers), then the usual
"Viewport: WxH" trailer as the last line.

## T01-3: Expanding a collapsed subtree via ref_id gets a fresh budget rooted there
Changed: same as T01-1/T01-2; exercises re-rooting the walk at a collapsed subtree's ref.
Steps:
1. Take a `ref_<M>` value from a collapse marker line produced in T01-2 (same tab, same page,
   same session -- refs are WeakRef-backed and only valid while the page/tab is unchanged).
2. Call `read_page` on the same tab with `ref_id: "ref_<M>"` and default `max_chars`.
Expect: the output is rooted at that element's subtree (its own lines and descendants, own fresh
"[showing ...]" or unmarked output depending on its own size), not the whole page again.

## T01-4: filter="interactive" and depth still shrink output as before
Changed: none to this behavior; regression check that pagination did not disturb filter/depth
handling (both were already honored and are explicitly out of scope for structural changes).
Steps:
1. On https://en.wikipedia.org/wiki/Web_browser, call `read_page` with `filter: "interactive"`
   and default depth/max_chars.
2. Separately, call `read_page` with `depth: 3` and default filter/max_chars.
Expect: step 1 shows substantially fewer lines than the "all" filter, only interactive elements
(links, buttons, inputs, etc.) and their containers. Step 2 shows a shallower tree (no lines more
than 3 levels of indent below the root). Neither call should be required to trigger a collapse
marker unless the shrunk output still exceeds max_chars (50000 default; unlikely at depth 3 or
filter=interactive on this page, but not a failure if it does -- markers are correct behavior at
any size).

## T01-5 (synthetic): the 10000-element cap fires and reports an exact count
Changed: new hard backstop (`MAX_ELEMENTS = 10000`) with a dedicated cap line ahead of the
summary line when it fires.
Steps:
1. Navigate a grouped tab to any simple page (e.g. https://example.com).
2. Call `javascript_tool` on that tab with the expression:
   `document.body.innerHTML = Array.from({length: 12000}, (_, i) => "<span>item " + i + "</span>").join(""); "ok"`
3. Call `read_page` on the same tab with `max_chars: 2000000` (large enough that the character
   budget is not the limiting factor).
Expect: exactly 10000 `span "item <n>" [ref_N]`-shaped lines, then a line reading exactly
"[element cap reached: output stopped after 10000 elements; use filter=\"interactive\", a ref_id
subtree, or a smaller depth]", then a line reading exactly
"[showing 10000 of 12000 elements; expand a collapsed subtree with ref_id, or narrow with
filter=\"interactive\" or a smaller depth]", then the "Viewport: WxH" trailer.

## T01-6: Stale ref_id still returns the unchanged error string
Changed: nothing (regression check); the stale-ref error path was explicitly preserved verbatim.
Steps:
1. On any grouped tab, call `read_page` with `ref_id: "ref_99999"` (a ref number that was never
   assigned in this page session).
Expect: the result text is exactly
`Error: ref_id "ref_99999" not found or was garbage-collected.`
(no markers, no summary line, no viewport trailer -- this is a plain string return, unchanged from
before this task).

## T02-1: filter=interactive only shows on-screen elements, with the Note line
Changed: `read_page` with `filter: "interactive"` now culls elements whose bounding rect does not
intersect the current viewport (via `getBoundingClientRect`), and appends one extra trailer line
when culling removed anything.
Steps:
1. With the extension reloaded, navigate a grouped tab to
   https://en.wikipedia.org/wiki/Web_browser (a long page, scrolled to the top).
2. Call `read_page` with `filter: "interactive"` (defaults for everything else).
Expect: every emitted element line corresponds to something currently visible on screen (no
off-screen links/buttons from far down the article). The very last line of the output is exactly
"Note: interactive results are limited to the current viewport; scroll or use filter=all for the
full document." (this line comes after the "Viewport: WxH" line).

## T02-2: Scrolling changes which interactive elements appear
Changed: same as T02-1; exercises that culling is scroll-position-relative, not a one-time compute.
Steps:
1. Same tab as T02-1, already scrolled to the top with a prior `filter: "interactive"` result
   recorded.
2. Call `computer` with action `scroll` to scroll down several screens (e.g. scroll down by a
   large amount, or use `scroll_to` on a ref far down the page from a `filter: "all"` call).
3. Call `read_page` with `filter: "interactive"` again on the same tab.
Expect: the set of `ref_N` interactive elements returned in step 3 differs from the set returned in
step 2's precursor (T02-1) -- new links/buttons that are now on screen appear, and elements that
were on screen before but have scrolled off no longer appear. The trailing Note line is still
present (still a long page with more off-screen content).

## T02-3: filter=all is unaffected -- no Note line, full document
Changed: nothing observable for `filter=all`; regression check that culling never applies there.
Steps:
1. Same tab as T02-1/T02-2 (any scroll position).
2. Call `read_page` with `filter: "all"` (or omit `filter` entirely -- "all" is the default).
Expect: the output includes off-screen elements from elsewhere in the document (not just what is
currently on screen), and the output ends with the "Viewport: WxH" line with nothing after it --
no "Note: interactive results are limited..." line, regardless of scroll position.

## T02-4: A short page that fits the viewport produces no Note line
Changed: same mechanism as T02-1; exercises the "nothing was culled" branch of the new note logic.
Steps:
1. Navigate a grouped tab to a short page whose interactive elements (if any) all fit within one
   screen without scrolling, for example https://example.com (it has exactly one link, "More
   information...", near the top).
2. Call `read_page` with `filter: "interactive"`.
Expect: the output ends with the "Viewport: WxH" line and nothing after it -- no Note line, since
nothing was off-screen to cull.

## T03-1: get_page_text picks the largest-innerText candidate, with the Source element header
Changed: `get_page_text` no longer picks the first matching selector or reads `textContent` off a
cloned node; it now scans every element matching any of twelve candidate selectors, picks the one
with the strictly largest `innerText`, and prefixes the output with "Source element: <selector>".
Paragraph breaks now survive (innerText preserves layout line breaks; the old textContent path
collapsed all whitespace to single spaces). Extension-only change: requires reloading the
extension at chrome://extensions; no MCP client restart needed.
Steps:
1. Reload the extension at chrome://extensions.
2. Navigate a grouped tab to a text-heavy Wikipedia article, for example
   https://en.wikipedia.org/wiki/Web_browser.
3. Call `get_page_text` with only `tabId` set (no `max_chars`).
Expect: the output starts with "Source element: " followed by one of the twelve candidate
selectors (for example "main" or ".content"), and the body text below it is broken into multiple
paragraphs separated by blank lines (not one giant single-spaced line). No "Title:" or "URL:"
line appears anywhere.

## T03-2: max_chars truncates with the exact bracketed notice
Changed: `max_chars` (previously ignored end to end) now bounds the normalized body text; the
service worker forwards it unchanged and the content script floors/validates it, defaulting to
50000 for anything absent or invalid.
Steps:
1. On the same tab as T03-1 (extension already reloaded), call `get_page_text` with
   `max_chars: 500`.
Expect: the output is "Source element: <selector>", a blank line, roughly 500 characters of body
text, a blank line, then a line reading exactly
"[Truncated at 500 characters. Retry with a larger max_chars, or use read_page to get a
structured view with element refs.]" (the number matches the `max_chars` you passed).

## T03-3: No readable text produces the actionable no-content message
Changed: previously an empty/near-empty page silently returned "Title: ...\nURL: ...\n\n" with no
text; now it returns a single actionable line naming the source element and suggesting
`read_page`.
Steps:
1. Navigate a grouped tab to about:blank.
2. Call `get_page_text` with only `tabId` set.
Expect: the output is exactly one line:
"No readable text content found (source element: body). The page may be mostly visual or may
render text dynamically. Use read_page to inspect the page structure instead."
(No "Source element:" header, no blank body.)

## T03-4: Hidden text (display:none) is excluded, unlike the old textContent implementation
Changed: switching from `textContent` on a cloned node to `innerText` means CSS-hidden text (for
example `display:none` banners or collapsed sections) is no longer included in the output. This
is a direct regression check against the old behavior.
Steps:
1. Navigate a grouped tab to a simple page, for example https://example.com.
2. Call `javascript_tool` on that tab with the expression:
   `const d = document.createElement("div"); d.style.display = "none"; d.textContent =
   "HIDDEN_MARKER_XYZ"; document.body.appendChild(d); "ok"`
3. Call `get_page_text` on the same tab with only `tabId` set.
Expect: the output does NOT contain the string "HIDDEN_MARKER_XYZ" anywhere. (The pre-T03
`textContent`-based implementation would have included it; this confirms the switch to
`innerText` actually excludes CSS-hidden content.)

## T12-1: Cross-domain navigation clears network requests
Changed: the network buffer is now keyed to the tab's current hostname; navigating a tab to a
different hostname replaces its buffer with a fresh empty one owned by the new hostname, so a
read after a cross-domain navigation never returns the old domain's requests. Extension-only
change: requires reloading the extension at chrome://extensions; no MCP client restart needed.
Steps:
1. Reload the extension at chrome://extensions.
2. Create a tab in the group and navigate it to https://example.com/.
3. Call `read_network_requests` on that tab (this enables Network tracking for the first time).
4. Navigate the same tab to https://example.com/ again (reload) to capture some traffic.
5. Call `read_network_requests` again.
Expect (step 5): the output contains example.com request lines (URLs starting with
"https://example.com/" or "http://example.com/").
6. Navigate the SAME tab to a different domain, for example https://www.iana.org/.
7. Call `read_network_requests`.
Expect (step 7): the output contains no example.com URLs anywhere. It is fine (and expected per
the accepted CDP-race limitation) if the very first iana.org document request is missing or
appears as a response-only "? https://www.iana.org/ -> 200" style line; seeing any example.com
traffic here is a failure.

## T12-2: Same-hostname navigation retains earlier requests
Changed: same as T12-1; this checks the non-reset side of the same rule (an unchanged hostname
must NOT reset the buffer), including SPA-style same-hostname URL changes.
Steps:
1. Continuing from T12-1 (tab currently on https://www.iana.org/, buffer already has iana.org
   entries from step 7 above), navigate the same tab to https://www.iana.org/domains (a
   different path, same hostname).
2. Call `read_network_requests`.
Expect: the output contains BOTH the request(s) captured on the earlier "/" page from T12-1 step
7 AND the new requests from /domains -- nothing was dropped by the path-only navigation.

## T12-3: Console messages are domain-scoped the same way
Changed: the console buffer follows the identical per-hostname ownership rule as the network
buffer.
Steps:
1. Navigate a grouped tab to https://example.com/.
2. Call `read_console_messages` on that tab once (enables Runtime tracking for the first time;
   ignore its output).
3. Call `javascript_tool` on the same tab with the expression: `console.log("marker-A"); "ok"`.
4. Call `read_console_messages` on the same tab.
Expect (step 4): the output contains the line "[log] marker-A".
5. Navigate the same tab to a different domain, for example https://www.iana.org/.
6. Call `read_console_messages` on the same tab.
Expect (step 6): the output does NOT contain "marker-A" (either "No console messages matching the
pattern." if nothing else was logged yet, or only iana.org-originated messages if the page itself
logs something).
7. Call `javascript_tool` on the same tab with the expression: `console.log("marker-B"); "ok"`.
8. Call `read_console_messages` on the same tab.
Expect (step 8): the output contains "[log] marker-B" and still does NOT contain "marker-A".

## T12-4: clear still works after the per-domain change
Changed: `read_network_requests`'s `clear: true` parameter still empties the buffer as before,
now via the new `{ host, items: [] }` shape.
Steps:
1. On a grouped tab with some captured network traffic (for example continuing from T12-1/T12-2),
   call `read_network_requests` with `clear: true`.
2. Perform a page action that generates at least one new request (for example a reload).
3. Call `read_network_requests` again (no `clear`).
Expect (step 3): the output shows only requests made after the clear in step 1 -- none of the
pre-clear requests reappear.

## T12-5: Tab close cleanup runs without errors
Changed: the `chrome.tabs.onRemoved` listener now also deletes the tab's `tabHost` entry
alongside the existing buffer/context cleanup.
Steps:
1. With a grouped tab that has an attached debugger and some buffered console/network entries
   (any tab used in T12-1 through T12-4 qualifies), close that tab.
2. Open the extension's service worker console at chrome://extensions (click "service worker"
   under the Browser MCP extension) and check for errors logged around the time of the close.
Expect: no errors appear in the service worker console from the tab-removal cleanup path.

## T13-1: Deferred uncaught exception appears as a console entry with level "exception"
Changed: `chrome.debugger.onEvent` now handles `Runtime.exceptionThrown` (previously silently
dropped) and pushes a synthetic `{ level: "exception", text }` entry into the same per-tab
console buffer `Runtime.consoleAPICalled` writes to. `read_console_messages`'s `onlyErrors`
filter already accepted `"exception"`, so no change was needed there. Order matters below: the
Runtime CDP domain is only enabled by the first `read_console_messages` call for a tab.
Steps:
1. Navigate a tab in the MCP tab group to any page (for example https://example.com).
2. Call `read_console_messages` once for that tab (this enables the Runtime domain; the result
   will likely be "No console messages matching the pattern.", which is fine).
3. Call `javascript_tool` with text:
   `setTimeout(() => { throw new Error("t13 test"); }, 0); "scheduled"`
   (the setTimeout matters: throwing directly inside the evaluated expression surfaces in the
   evaluate response itself and never emits `Runtime.exceptionThrown`; the deferred throw is a
   genuine uncaught page exception).
4. Call `read_console_messages` with `onlyErrors: true`.
Expect (step 4): the output contains a line beginning `[exception] Error: t13 test` followed by
a `(url:line)` location and a compact `[at ...]` stack, for example something like
`[exception] Error: t13 test (https://example.com/:1) [at <anonymous>@https://example.com/:1]`
(exact frame names/URLs depend on the page).

## T13-2: Ordinary console levels are unaffected (no double counting)
Changed: same as T13-1; this confirms the new branch does not disturb the existing
`Runtime.consoleAPICalled` path.
Steps:
1. Continuing from T13-1 (same tab, Runtime domain already enabled), call `javascript_tool`
   with text `console.log("t13 plain")`.
2. Call `read_console_messages` without `onlyErrors`.
Expect: `[log] t13 plain` appears exactly once, alongside the `[exception] Error: t13 test` line
from T13-1 (both present, neither duplicated).

## T13-3: pattern filtering matches the exception text
Changed: same as T13-1; confirms the new entry participates in the existing `pattern` filter
like any other console entry.
Steps:
1. Continuing from T13-1/T13-2, call `read_console_messages` with `pattern: "t13 test"`.
Expect: the output contains only the `[exception] Error: t13 test ...` line and nothing else
(not the `[log] t13 plain` line).
