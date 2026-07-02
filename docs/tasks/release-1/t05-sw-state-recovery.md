# T05: Service-worker death recovery (rehydrate tab group, reattach lazily)

## Goal

Manifest V3 service workers are killed by the browser (extension reload, browser update,
crash, manual stop), and every in-memory variable dies with them: the MCP tab group id, the
per-tab console/network buffers, screenshot contexts, and debugger attachment records. Make
the extension survive that: persist the minimal durable session state to
`chrome.storage.session`, rehydrate it on service-worker startup, reattach the debugger
lazily on the next command, and tell the truth about lost event buffers in the next read.

## Project context

Browser MCP is a governed browser automation system. A single Rust binary is both the MCP
server (JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host. A
thin Manifest V3 extension executes CDP commands. The chain is:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS IPC.

The extension side works like this: the service worker opens a native-messaging port to the
host `org.sylin.browser_mcp`, receives `{ id, type: "tool_request", tool, args }` messages,
runs the matching tool handler, and replies `{ id, type: "tool_response", result }` or
`{ id, type: "tool_error", error }`. Managed tabs live in a Chrome tab group titled
"Browser MCP"; membership in that group is the extension's only notion of "managed tab".

Files relevant to this task:

- `extension/service-worker.js`: the only file you will modify. Vanilla JS, Manifest V3
  service worker. Holds the native port lifecycle, the keepalive alarm, the tab group
  management, the per-tab buffers, the debugger attach logic, and all tool handlers.
- `extension/manifest.json`: read-only for this task. It already declares the `storage` and
  `alarms` permissions (lines 15-16) and `minimum_chrome_version: "116"` (line 6). Do not
  edit it.
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

Background you must know about MV3 lifecycles:

- `chrome.storage.session` survives service-worker restarts but is cleared when the browser
  itself restarts. That is exactly the scope we want: after a full browser restart there is
  genuinely no prior session, so no stale state and no false recovery notice.
- Since Chrome 116 an open native-messaging port extends the service worker's lifetime, so
  in practice the worker dies on extension reload, browser update, crash, or a manual stop,
  not on ordinary idle. Recovery must work for all of those.
- Top-level code in the service worker runs again on every restart, but only listeners
  registered synchronously at the top level are guaranteed to fire. Keep all
  `addListener` calls synchronous at the top level, as they are today.

## Current behavior

All line numbers refer to `extension/service-worker.js` as it stands now (568 lines).

- Lines 8-9 declare `NATIVE_HOST = "org.sylin.browser_mcp"` and
  `GROUP_TITLE = "Browser MCP"`.
- Lines 11-16 declare all session state as plain module-level variables, none persisted:
  `nativePort`, `groupId`, `attached` (Map, tabId to `{ domains: Set }`), `consoleBuffer`,
  `networkBuffer`, `screenshotCtx`. A grep for `chrome.storage` over `extension/` finds
  nothing; the extension currently persists no state at all.
- Lines 22-25: a `"keepalive"` alarm is created at top level with
  `periodInMinutes: 0.4`; its handler calls `connect()` whenever `nativePort` is null.
- Lines 27-44: `connect()` opens the native port, wires `onMessage` to `dispatch`, and on
  `onDisconnect` (or a throw) nulls the port and retries via `setTimeout(connect, 2000)`.
- Line 54: `attaching` (Map) holds in-flight attach promises. Lines 55-64:
  `ensureAttached(tabId)` returns early if `attached.has(tabId)`, otherwise calls
  `chrome.debugger.attach({ tabId }, "1.3")` and records
  `attached.set(tabId, { domains: new Set() })`. There is no handling for the attach call
  rejecting because a previous attachment survived a service-worker restart; such a
  rejection propagates as a tool error.
- Lines 114-117: `cdp(tabId, method, params)` awaits `ensureAttached` before every
  `chrome.debugger.sendCommand`, so attachment is already on-demand per command.
- Lines 118-124: `enableDomain(tabId, domain)` issues `<domain>.enable` once per tab,
  tracked in the per-tab `domains` set. Re-issuing an enable after a restart is harmless;
  CDP enables are idempotent.
- Lines 125-133: `chrome.tabs.onRemoved` detaches the debugger and deletes the tab's
  entries from `attached`, `consoleBuffer`, `networkBuffer`, and `screenshotCtx`.
  Line 134: `chrome.debugger.onDetach` prunes `attached`.
- Lines 137-154 buffer console and network events; lines 155-160 cap each buffer at 1000
  entries via `pushCapped`.
- Lines 163-174: `ensureGroup(create)` first validates the in-memory `groupId` with
  `chrome.tabGroups.get`, clearing it if stale; then queries
  `chrome.tabGroups.query({ title: GROUP_TITLE })` and adopts the first match; only then,
  if `create` is true, makes a new window, groups its tab, and titles the group
  "Browser MCP" with color blue. So there is already a live-state recovery path, but it
  depends entirely on the group title still being "Browser MCP". If the user renames the
  group, recovery by title fails and a duplicate group gets created.
- Lines 178-190: `inGroup(tabId)` consults live tab state and re-adopts `groupId` when it
  finds the tab in a group whose title matches `GROUP_TITLE` (again title-dependent).
- Lines 191-194: `tabContext` renders `{ mcpGroupId, tabs }` as JSON for the tabs tools.
- Lines 447-451: `tabs_context_mcp` calls `ensureGroup(a.createIfEmpty)`. Lines 452-459:
  `tabs_create_mcp` calls `ensureGroup(true)`, creates a tab, and groups it (line 455).
- Lines 512-526: `read_console_messages` renders buffered entries or the fallback string
  `"No console messages matching the pattern."` (line 525). Lines 527-536:
  `read_network_requests` renders buffered entries or
  `"No network requests matching the pattern."` (line 535). Neither gives any hint that the
  buffers may have been wiped by a service-worker restart, so after a restart the model
  silently sees an empty log and can wrongly conclude the page emitted nothing.
- Lines 558-566: `dispatch(id, tool, args)` looks up the handler, awaits it, and replies or
  fails. Line 568: top-level `connect()` runs on every service-worker start.

Summary of the gap: the group recovers only if its title is untouched, the managed-tab list
exists nowhere durable, a surviving debugger attachment can make re-attach fail, and buffer
loss is silent.

## Required behavior

Modify `extension/service-worker.js` only.

1. Persistence helper. Add an async function `persistSessionState()` that writes the
   minimal durable state to `chrome.storage.session` under the key `"sessionState"` with
   exactly this shape:

       { groupId: <number or null>, tabIds: <array of numbers> }

   Derivation rules:
   - `groupId` is the current module-level `groupId`.
   - `tabIds` is derived live: when `groupId` is not null, it is the ids from
     `await chrome.tabs.query({ groupId })`; when `groupId` is null (or the query throws
     because the group vanished between check and query), it is `[]`.
   - Wrap the storage write in try/catch; if `chrome.storage.session.set` fails, swallow
     the error (recovery then degrades to the existing title-based fallback; do not crash a
     tool call over a persistence failure).
   - Do NOT persist `consoleBuffer`, `networkBuffer`, `screenshotCtx`, or `attached`.
     Buffers and screenshot contexts are deliberately re-derived from live activity;
     debugger attachment is re-established lazily (point 4).

2. Persistence points. Call `persistSessionState()` (fire-and-forget or awaited, your
   choice, but never let its failure fail the caller) at exactly these moments:
   - At the end of `ensureGroup(create)`, on every call, after `groupId` has settled
     (covers the adopt-by-title branch, the create branch, and the stale-id-cleared case).
   - Inside `inGroup(tabId)`, only when it actually re-adopts `groupId` (the branch at
     lines 182-185 where `groupId` transitions from null to the found group's id).
   - In `tabs_create_mcp`, after the new tab has been grouped (after the
     `chrome.tabs.group` call at line 455).
   - At the end of the `chrome.tabs.onRemoved` listener (lines 125-133), so a closed tab
     drops out of the stored `tabIds`.
   Do not add persistence calls anywhere else; `navigate`, `computer`, and the read tools
   do not change group membership.

3. Startup rehydration. Add an async function `rehydrate()` and, at the top level next to
   the existing `connect()` call (line 568), capture its promise in a module-level
   variable named `ready`:

       const ready = rehydrate();

   Wait: `ready` must be declared before `dispatch` can run, and `dispatch` must begin with
   `await ready;` so no tool executes against un-rehydrated state (the native port can
   deliver a request within milliseconds of startup). `rehydrate()` must never reject:
   wrap its whole body so that any internal error resolves the promise anyway; a
   rehydration failure must degrade to the current cold-start behavior, never wedge
   dispatch.

   `rehydrate()` does the following, in order:
   a. Read `chrome.storage.session.get("sessionState")` and take the `sessionState`
      property. If there is no stored value, return; this is a genuinely fresh start
      (first install or first run after a browser restart) and nothing below applies.
   b. Decide whether a prior session existed: it did when the stored `groupId` is not null
      or the stored `tabIds` is a non-empty array. If a prior session existed, set both
      module-level notice flags from point 6 (`consoleResetNotice` and
      `networkResetNotice`) to true, because whatever those buffers held is gone.
   c. If the stored `groupId` is not null, verify the group still exists with
      `chrome.tabGroups.get(storedGroupId)`. If the call succeeds, adopt it: assign the
      module-level `groupId` from storage. Do NOT compare titles here and do NOT rename or
      recolor the group; the stored id is authoritative even if the user renamed the group.
      If the call throws, the group is gone: leave `groupId` null (the existing
      title-query fallback in `ensureGroup` remains the next recovery layer, and after
      that, creation).
   d. Call `persistSessionState()`. Because that helper re-derives `tabIds` from a live
      query of the adopted group (or stores `[]` when the group is gone), this single call
      prunes ids of tabs that no longer exist and drops tabs the user pulled out of the
      group. No separate per-tab `chrome.tabs.get` loop is needed.

   Note what rehydration must NOT do: it must not create a group, must not create tabs,
   must not attach the debugger to anything, must not enable CDP domains, and must not
   touch the buffers. It only restores `groupId` and refreshes the stored record.

4. Lazy debugger reattach. Keep the on-demand model exactly as it is: after a restart the
   `attached` map is empty and the next `cdp()` call re-attaches via `ensureAttached`.
   Add one piece of resilience inside `ensureAttached`'s attach promise: when
   `chrome.debugger.attach({ tabId }, "1.3")` rejects AND the error message matches
   `/already attached/i`, the previous service-worker instance's attachment may have
   survived. In that case call `chrome.debugger.getTargets()` and look for a target whose
   `tabId` equals this tab and whose `attached` flag is true:
   - If found, adopt the surviving attachment: record
     `attached.set(tabId, { domains: new Set() })` and resolve as attached. The fresh
     empty `domains` set means `enableDomain` will re-issue `Runtime.enable` or
     `Network.enable` on next use, which is idempotent and safe.
   - If not found, rethrow the original attach error unchanged so the tool call fails
     truthfully.
   Caveat you must preserve: `getTargets` cannot tell whose debugger is attached. If the
   surviving attachment actually belongs to DevTools rather than to us, the adoption will
   lead to a `sendCommand` failure, which propagates truthfully to the model through the
   existing catch in `dispatch` (lines 561-565). That is the correct outcome; do not mask
   it, do not retry, do not force-detach someone else's debugger.

5. Native port and keepalive: verify, do not change. The top-level `connect()` call
   (line 568) already re-establishes the native-messaging port on every service-worker
   start; the `"keepalive"` alarm (lines 22-25, period 0.4 minutes) already reconnects when
   the port is null; `onDisconnect` already retries after 2000 ms (lines 36-39). Re-read
   these paths to confirm they still hold, then leave them byte-identical. The only change
   near startup is the added `const ready = rehydrate();` line.

6. Buffer-loss truthfulness. Add two module-level booleans, `consoleResetNotice` and
   `networkResetNotice`, both initialized false and both set true only by `rehydrate()`
   step b. Then:
   - In `read_console_messages`, after composing the final output string (whether it is the
     joined entries or the `"No console messages matching the pattern."` fallback), if
     `consoleResetNotice` is true, append a newline plus this exact line, then set
     `consoleResetNotice = false`:

         Note: console event buffer was reset by a browser service-worker restart; tracking resumed from that point.

   - In `read_network_requests`, same mechanic with `networkResetNotice` and this exact
     line:

         Note: network event buffer was reset by a browser service-worker restart; tracking resumed from that point.

   Rules: the note is appended at most once per tool per service-worker lifetime (the flag
   is consumed on first successful read); it is appended only when the handler actually
   reaches its normal return (the early `inGroup` rejection at the top of each handler must
   not consume the flag); the two flags are independent (a console read must not consume
   the network notice or vice versa); and the existing output format above the note stays
   byte-identical.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS. Everything in this task is mechanism (state survival and honest
   reporting); keep it that way.
3. ASCII only in all code and docs: no em-dashes, no arrows, no curly quotes, anywhere,
   including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text. The notice lines in
   point 6 exist for exactly this reason; do not soften or drop them.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on public
   items, rustfmt clean, clippy with deny warnings. (This task should touch no Rust; if you
   believe it must, stop and reconsider, because it must not.)
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style. One short comment on why `getTargets` adoption exists and one on the
   storage.session lifetime are acceptable.
8. Do NOT copy code from the official Anthropic extension or any other project; implement
   the behavior described above from scratch.

Task-specific:

9. The only file you may change is `extension/service-worker.js`.
10. Use `chrome.storage.session` only. Not `chrome.storage.local`, not `chrome.storage.sync`.
    The browser-restart-clears semantics of the session area are load-bearing.
11. The stored record is exactly `{ groupId, tabIds }` under the key `"sessionState"`.
    Do not persist buffers, screenshot contexts, attachment records, or anything else.
12. All `addListener` registrations stay synchronous at the top level, as they are now.
13. Do not change the keepalive alarm name or period, the 2000 ms reconnect delay, the
    group title or color, buffer caps, or any existing tool result text except the two
    appended notice lines.

## Verification

1. `cargo test` from the repo root: all tests pass with no test file edited (this task
   touches no Rust, so this is a pure regression gate).
2. Ask the user to reload the extension at `chrome://extensions` (Developer mode on).
   No MCP client restart is needed for an extension-only change, but the tool calls below
   must run through a connected MCP client.
3. Establish a session:
   a. Call `tabs_context_mcp` with `createIfEmpty: true`; note the `mcpGroupId` value in
      the JSON output.
   b. Call `tabs_create_mcp`, then `navigate` the new tab to `https://example.com`.
   c. Call `read_console_messages` for that tab once (enables the Runtime domain and, on a
      fresh install, must NOT show any reset note; there was no prior session).
   d. Optionally, from the service worker's DevTools console (chrome://extensions, click
      the "service worker" link), run
      `chrome.storage.session.get("sessionState").then(console.log)` and confirm the
      stored `groupId` matches `mcpGroupId` and `tabIds` lists the group's tabs.
4. Kill the service worker without touching the tabs. Two working ways: open Chrome's task
   manager (Shift+Esc), select the "Extension: Browser MCP" row, and End Process; or open
   `chrome://serviceworker-internals`, find the extension's worker, and press Stop. Closing
   the worker's DevTools window first helps it actually die.
5. Recovery checks, in order:
   a. Call `tabs_context_mcp` (no `createIfEmpty`). Expect the SAME `mcpGroupId` as in
      step 3a, the same tabs listed, and no duplicate "Browser MCP" group visible in the
      browser.
   b. Call `read_console_messages` for the managed tab. Expect the console reset note as
      the last line. Call it again: the note must be gone.
   c. Call `read_network_requests` for the managed tab. Expect the network reset note as
      the last line, independent of the console reads. Call again: gone.
   d. Call `computer` with `action: "screenshot"` on the managed tab. Expect a successful
      screenshot; the debugger infobar reappears if it had dropped (lazy reattach worked).
6. Rename resilience (the improvement over the title-based fallback): rename the tab group
   in the browser UI to anything else, kill the worker again as in step 4, then call
   `tabs_context_mcp`. Expect the renamed group to be re-adopted by its stored id (same
   `mcpGroupId`), with its user-chosen name left alone and no new group created.
7. Closed-tab pruning: close one managed tab, then check
   `chrome.storage.session.get("sessionState")` in the worker console; its id must be gone
   from `tabIds`.
8. Full browser restart: quit and relaunch Chrome, connect the MCP client, call
   `read_console_messages` on a fresh group tab. `chrome.storage.session` is cleared by the
   browser restart, so no reset note may appear (there is no recovered session to report
   about).

## Out of scope

- Binary-side (Rust) reconnect or restart handling. No changes to anything under `src/`,
  `tests/`, or `Cargo.toml`.
- Persisting console/network buffer contents, screenshot contexts, or debugger attachment
  records. Buffers and contexts are re-derived by design; only `{ groupId, tabIds }` is
  durable.
- Any change to tab-group naming or color behavior beyond re-adoption: no renaming a
  recovered group back to "Browser MCP", no recoloring, no re-grouping of orphaned tabs
  into a new group when the old group is gone.
- Changing the keepalive strategy: alarm name, alarm period, reconnect delay, or the
  Chrome-116 native-port lifetime assumption.
- Adding manifest permissions or editing `extension/manifest.json` in any way (`storage`
  and `alarms` are already declared).
- Eager debugger attachment or eager CDP domain enabling during rehydration; attachment
  stays strictly on-demand.
- Any change to `extension/content.js`, `extension/agent-visual-indicator.js`, or
  `src/mcp/schemas/tools.json`.
- New tools, new tool parameters, or any change to existing tool result texts other than
  the two exact notice lines defined above.
