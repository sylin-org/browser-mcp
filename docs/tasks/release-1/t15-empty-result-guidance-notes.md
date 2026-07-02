# T15: Empty-result guidance notes for read_console_messages and read_network_requests

## Goal

When `read_console_messages` or `read_network_requests` finds zero entries, the tool result must explain why (tracking starts on first use of the tool for a tab) and tell the agent how to get data (reload the page or trigger requests). The result must also distinguish truthfully between an empty buffer and a filter that excluded everything. Today both tools return a bare one-line no-matches string, so agents wrongly conclude "no logs" on the very first call instead of reloading.

## Project context

Browser MCP is governed browser automation. A single Rust binary is both the MCP server (JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host; a thin Manifest V3 extension executes CDP commands. Architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS IPC. The binary relays tool calls to the extension and relays the extension's results back to the MCP client unchanged.

Files relevant to this task:

- `extension/service-worker.js`: CDP dispatch, console/network event buffers, and the tool handlers you will edit. This is the ONLY file you will modify.
- `src/mcp/schemas/tools.json`: the sacred, byte-frozen official tool schemas. Never edit. The `read_console_messages` schema (parameters `tabId`, `pattern`, `limit`, `onlyErrors`, `clear`) is at lines 211-241; the `read_network_requests` schema (parameters `tabId`, `urlPattern`, `limit`, `clear`) is at lines 242-268.
- `tests/tool_schema_fidelity.rs`: guard test that must keep passing unchanged.

Build and test: run `cargo test` from the repo root. This task changes only extension JavaScript, so no Rust rebuild is needed, but run `cargo test` anyway to confirm nothing regressed. Extension changes take effect only after the user reloads the extension at `chrome://extensions`. If you did need to rebuild the binary and `target/debug/browser-mcp.exe` is locked by a running session, rename it aside first (for example `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and rebuild; binary or schema changes require an MCP client restart to observe. Neither applies here.

## Current behavior

All line numbers verified against `extension/service-worker.js` as of this writing.

Buffering (background, do not touch):

- `consoleBuffer` and `networkBuffer` are `Map`s of `tabId -> array`, declared at lines 14-15.
- A `chrome.debugger.onEvent` listener (lines 137-154) pushes `Runtime.consoleAPICalled` events into `consoleBuffer` and `Network.requestWillBeSent` / `Network.responseReceived` events into `networkBuffer`, via `pushCapped` (lines 155-160, cap 1000 entries per tab).
- Both buffers are deleted when the tab is removed (lines 130-131).
- CDP domains are enabled lazily: `enableDomain` (lines 118-124) enables `Runtime` or `Network` only when the read tool is first called for that tab. Events emitted before that first call were never delivered, so the buffer is empty. This is why the first call on a tab almost always returns zero entries; the fix for that experience is the guidance note this task adds, not any change to the buffering.

The two handlers you will edit:

- `read_console_messages(a)` at lines 512-526. It checks group membership (line 513), attaches and enables `Runtime` (lines 514-516), reads `consoleBuffer.get(a.tabId) || []` (line 517), applies the `onlyErrors` filter (line 518), applies the `pattern` filter as a case-insensitive regex with a plain-substring fallback when the regex fails to compile (lines 519-521), applies `msgs.slice(-(a.limit || 100))` (line 523), clears the buffer when `a.clear` is set (line 524), and returns at line 525:

      return text(msgs.length ? msgs.map((m) => `[${m.level}] ${m.text}`).join("\n") : "No console messages matching the pattern.");

- `read_network_requests(a)` at lines 527-536. Same shape: group check (line 528), attach and enable `Network` (lines 529-530), reads `networkBuffer.get(a.tabId) || []` (line 531), applies the `urlPattern` substring filter (line 532), applies the limit slice (line 533), clears when `a.clear` is set (line 534), and returns at line 535:

      return text(reqs.length ? reqs.map((r) => `${r.method || "?"} ${r.url} ${r.status ? "-> " + r.status : "(pending)"}`).join("\n") : "No network requests matching the pattern.");

- `text(t)` (lines 207-209) wraps a string into `{ content: [{ type: "text", text: t }] }`. Use it exactly as the handlers already do.

The problem: the empty-result strings "No console messages matching the pattern." and "No network requests matching the pattern." are returned even when no pattern was given, and they say nothing about the lazy-enable behavior. The agent has no cue to reload the page.

## Required behavior

Modify only the zero-result return paths of the two handlers. Non-empty results must remain byte-for-byte identical to today.

In each handler, capture the pre-filter buffer length in a local variable at the point where the buffer is first read (line 517 for console, line 531 for network), before any filter or limit is applied. Call it `total` (or similar). Do not move or change the filter, limit, or clear logic; `clear` must still empty the buffer even when zero entries matched, exactly as it does now.

When the filtered list is empty, return a two-line result: a primary line, then a newline (`\n`), then a note line. The exact strings follow.

`read_console_messages`, zero entries after filtering:

- If `total` is 0 (buffer empty), the primary line is:

      No console messages recorded for this tab.

- If `total` is greater than 0 (entries existed but the filter excluded all of them), the primary line is:

      ${total} console message(s) recorded for this tab, but none matched your filter.

  where `${total}` is the pre-filter count as a plain integer. Use the literal `(s)` suffix; do not add pluralization logic.

- In both cases, the second line is exactly:

      Note: console tracking begins when this tool is first used on a tab. Reload the page to capture messages emitted during page load.

Example full result text for a first call on a fresh tab:

    No console messages recorded for this tab.
    Note: console tracking begins when this tool is first used on a tab. Reload the page to capture messages emitted during page load.

Example full result text when 14 messages exist but the pattern matched none:

    14 console message(s) recorded for this tab, but none matched your filter.
    Note: console tracking begins when this tool is first used on a tab. Reload the page to capture messages emitted during page load.

`read_network_requests`, zero entries after filtering:

- If `total` is 0, the primary line is:

      No network requests recorded for this tab.

- If `total` is greater than 0, the primary line is:

      ${total} network request(s) recorded for this tab, but none matched your filter.

- In both cases, the second line is exactly:

      Note: network tracking begins when this tool is first used on a tab. Reload the page to capture requests made during page load, or interact with the page to trigger new requests.

Wrap the assembled string with the existing `text(...)` helper, as the handlers do today. The non-empty branches (the `msgs.map(...)` and `reqs.map(...)` joins) must not change in any way.

Match the surrounding code style: compact vanilla JS, double quotes, template literals, semicolons, no new helper functions unless a tiny local one genuinely reduces duplication inside `service-worker.js` (a small shared function taking the two primary-line variants and the note string is acceptable; a copy in each handler is also acceptable).

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in extension JS. The guidance note is a factual statement about mechanism, not a policy decision.
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly quotes, anywhere, including comments. The existing `"-> "` in the network output line is plain ASCII hyphen-greater-than and stays as is.
4. The engine is truthful: never fake success, never silently substitute behavior. That is the point of this task: "buffer empty" and "N entries did not match your filter" are different truths and must be reported as such.
5. No new runtime dependencies. The extension stays vanilla JS: no bundler, no libraries.
6. Rust rules (2021 edition, thiserror, doc comments, rustfmt, clippy deny warnings) apply to any Rust you touch; this task touches none.
7. Comments only for constraints the code cannot express; match the surrounding comment density and style. The zero-result strings are self-explanatory and need no comment.
8. Do NOT copy code from the official Anthropic extension or any other project; implement the behavior described above from scratch.

Task-specific constraints:

9. The only file you may edit is `extension/service-worker.js`, and within it only the bodies of `read_console_messages` and `read_network_requests`.
10. The strings given in Required behavior are exact. Do not reword, reorder, repunctuate, or add trailing whitespace or a trailing newline.
11. Do not change the order of operations in the handlers (group check, attach, enable, read, filter, limit, clear, return); only capture `total` and replace the zero-result string.

## Verification

1. Run `cargo test` from the repo root. All tests must pass, including `tool_schema_fidelity`.
2. Ask the user to reload the extension at `chrome://extensions` (extension-only change; no binary rebuild, no MCP client restart needed).
3. Manual checks through an MCP client (Claude Code) after reload:
   - Open a fresh tab into the group, then call `read_console_messages` with only `tabId`. Expect exactly the two-line "No console messages recorded for this tab." result with the console note.
   - Reload that page, call `read_console_messages` again. If the page logs anything during load, expect the normal `[level] text` lines, unchanged in format.
   - With messages present, call `read_console_messages` with `pattern` set to a string that cannot match (for example `"zzz_no_such_pattern"`). Expect "N console message(s) recorded for this tab, but none matched your filter." followed by the console note, where N is the buffered count.
   - Repeat the same three checks for `read_network_requests` using `urlPattern` for the no-match filter case; expect the network variants of the strings.
   - Call `read_network_requests` with `clear: true` when zero entries match a filter; the next unfiltered call must show the buffer was cleared (returns the buffer-empty variant), confirming `clear` still fires on the zero-match path.
4. Confirm no other output changed: a call that returns entries must produce byte-identical text to before the change.

## Out of scope

- Buffer mechanics of any kind: the `chrome.debugger.onEvent` listener, `pushCapped`, the 1000-entry cap, per-tab buffer lifecycle, `enableDomain`, `ensureAttached`, or making tracking eager instead of lazy. Those belong to T12/T13/T14, not this task.
- Any change to non-empty output: line formats, joins, ordering, limits, or filter semantics.
- Any change to the `clear`, `limit`, `pattern`, `onlyErrors`, or `urlPattern` behavior, including edge-case hardening of `limit`.
- Any change to other tool handlers, `extension/content.js`, `extension/agent-visual-indicator.js`, the Rust binary, or `src/mcp/schemas/tools.json`.
- Any auto-reload of the page on behalf of the agent. The note tells the agent what to do; the tool must not do it for them.
- New helpers shared across files, new files, or refactors beyond the two return paths.
