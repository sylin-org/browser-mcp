# T17: Effective-tabId resolution: current-tab fallback + valid-ID error listing

## Goal

Every tabId-bearing tool handler in the extension requires an explicit tabId
and, when the id is stale or foreign, returns a generic "not in the group"
message that forces the model into a blind `tabs_context_mcp` round trip.
Introduce one shared helper that (a) falls back to the group's current tab
when tabId is omitted or null and (b) lists the valid tab IDs in the error
when a wrong id is passed, and use it in every tabId-bearing handler.

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
and redaction decisions live in the Rust binary. Managing which tab a tool
call lands on is mechanism (tab-group lifecycle), so it belongs in the
extension.

Key files:

- `src/mcp/server.rs`: JSON-RPC loop in the binary. Read-only for this task.
- `src/mcp/schemas/tools.json`: SACRED. Byte-frozen official tool schemas.
  Never edit. Guarded by `tests/tool_schema_fidelity.rs`.
- `src/browser.rs`: routes extension replies back to MCP callers. Read-only
  for this task.
- `extension/service-worker.js`: CDP dispatch, tab-group management, tool
  handlers. This is the ONLY file this task touches.
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

All facts below were verified by reading the named files.

In `extension/service-worker.js` (568 lines):

- Line 9: `const GROUP_TITLE = "Browser MCP";`
- Lines 163-174: `ensureGroup(create)` recovers the module-level `groupId`
  from live state (by querying tab groups titled `GROUP_TITLE`) and only
  creates a new window plus group when `create` is truthy.
- Lines 175-177: `groupTabs()` returns `chrome.tabs.query({ groupId })`, or
  `[]` when `groupId` is null.
- Lines 178-190: `inGroup(tabId)` consults live state via
  `chrome.tabs.get(tabId)`, recovers a stale `groupId` by group title, and
  returns false when the tab does not exist or is not in the group.
- Every tabId-bearing handler requires `a.tabId` and starts with an
  `inGroup` check. Two message variants exist today:
  - The fuller variant, using `GROUP_TITLE`:
    - `computer` (lines 358-359):
      ```js
      const tabId = a.tabId;
      if (!(await inGroup(tabId))) return text(`Tab ${tabId} is not in the ${GROUP_TITLE} group.`);
      ```
    - `navigate` (line 461): same message; the handler then uses `a.tabId`
      directly on lines 463, 465, 472, 474, 475.
  - The short variant, `` `Tab ${a.tabId} is not in the group.` ``:
    - `read_page` (line 480; `a.tabId` used on 481)
    - `get_page_text` (line 485; used on 486)
    - `find` (line 490; used on 491)
    - `form_input` (line 500; used on 501)
    - `javascript_tool` (line 506; used on 507)
    - `read_console_messages` (line 513; used on 514, 516, 517, 524)
    - `read_network_requests` (line 528; used on 529, 530, 531, 534)
    - `resize_window` (line 538; used on 539; note the handler also has an
      unrelated inner loop variable named `tabId` on lines 543-547)
- Neither variant lists the valid tab IDs, and no handler falls back when
  tabId is missing: a missing tabId reaches `chrome.tabs.get(undefined)`
  inside `inGroup`, which throws, so `inGroup` returns false and the model
  sees `Tab undefined is not in the ... group.`
- `tabs_context_mcp` (lines 447-451), `tabs_create_mcp` (lines 452-459), and
  `update_plan` (lines 551-555) take no tabId.
- These membership refusals are returned via `text(...)` as a normal
  `tool_response` result, NOT via `fail(...)`. Lines 558-566, `dispatch`:

  ```js
  async function dispatch(id, tool, args) {
    const handler = handlers[tool];
    if (!handler) return fail(id, `Unknown tool: ${tool}`);
    try {
      reply(id, await handler(args));
    } catch (e) {
      fail(id, `${tool} failed: ${(e && e.message) || e}`);
    }
  }
  ```

  A thrown error therefore becomes a `tool_error` with a
  `<tool> failed: ` prefix.

In the Rust binary (context for why the error channel matters; do not edit):

- `src/browser.rs`, `route_reply` (lines 153-173): a `tool_error` reply
  becomes `Err(<error string>)`.
- `src/mcp/server.rs`, `handle_tools_call` (lines 116-155): the `Err` branch
  (lines 147-153) renders `Error: <string>` with `isError: true`. The success
  branch passes the extension's result through. Also note lines 125-128: the
  binary forwards `arguments` verbatim with NO schema validation, so a null
  or omitted tabId can reach the extension even though the schema marks
  tabId required.

In `src/mcp/schemas/tools.json` (SACRED, do not edit): `tabId` appears in
the `required` list of all ten tabId-bearing tools: navigate (line 47),
computer (123), find (142), form_input (165), get_page_text (184),
javascript_tool (207), read_console_messages (238), read_network_requests
(265), read_page (297), resize_window (320). This stays exactly as is; the
fallback below is defense in depth for arguments that arrive anyway.

`extension/manifest.json` declares a classic (non-module) service worker
with `minimum_chrome_version: "116"`. Top-level functions and classes are
reachable from the inspected service worker console, and
`chrome.tabs.Tab.lastAccessed` may be undefined on older Chrome versions, so
the helper must tolerate a missing `lastAccessed`.

## Required behavior

All changes are in `extension/service-worker.js`.

### 1. Add `TabAccessError` and `effectiveTabId`

Insert both immediately after `inGroup` (after line 190), inside the
tab-group section. Use these exact names; `dispatch` and every handler will
reference them.

```js
// Thrown when a tool call names a tab outside the group or the group has no usable tab.
// dispatch() converts it to a plain text tool result so the message reaches the model
// verbatim, matching how group-membership refusals are delivered today.
class TabAccessError extends Error {}

// Resolve the tab a tool call acts on. A provided tabId must be in the group; an omitted or
// null tabId falls back to the group's active tab, else its most recently accessed tab.
async function effectiveTabId(rawTabId) {
  if (rawTabId !== undefined && rawTabId !== null) {
    if (await inGroup(rawTabId)) return rawTabId;
    await ensureGroup(false);
    const tabs = await groupTabs();
    if (!tabs.length) {
      throw new TabAccessError(`Tab ${rawTabId} is not in the ${GROUP_TITLE} group. The group has no tabs; use tabs_create_mcp to open one.`);
    }
    throw new TabAccessError(`Tab ${rawTabId} is not in the ${GROUP_TITLE} group. Valid tab IDs are: ${tabs.map((t) => t.id).join(", ")}.`);
  }
  await ensureGroup(false);
  const tabs = await groupTabs();
  if (!tabs.length) {
    throw new TabAccessError(`No tabs in the ${GROUP_TITLE} group. Use tabs_create_mcp to open one, or tabs_context_mcp with createIfEmpty: true.`);
  }
  const active = tabs.filter((t) => t.active);
  const pool = active.length ? active : tabs;
  let best = pool[0];
  for (const t of pool) {
    if ((t.lastAccessed || 0) > (best.lastAccessed || 0)) best = t;
  }
  return best.id;
}
```

Behavior this encodes, spelled out:

- Provided tabId that is in the group: returned unchanged. The happy path
  costs the same one `inGroup` call the handlers make today.
- Provided tabId that is stale or foreign (tab closed, tab in another group,
  tab that never existed, or a non-integer that makes `chrome.tabs.get`
  throw): recover the group id from live state with `ensureGroup(false)`
  (recover only; never create), then throw `TabAccessError`. Rendered
  message when the group has tabs 987654321 and 987654322:

  ```
  Tab 123456 is not in the Browser MCP group. Valid tab IDs are: 987654321, 987654322.
  ```

  Rendered message when the group is empty or absent:

  ```
  Tab 123456 is not in the Browser MCP group. The group has no tabs; use tabs_create_mcp to open one.
  ```

  The prefix keeps the fuller of the two phrasings used today (the
  `computer`/`navigate` variant built from `GROUP_TITLE`); the short
  `is not in the group.` variant disappears. Build the messages with the
  `GROUP_TITLE` constant in a template literal, never a hardcoded
  "Browser MCP" string. The id list is the group's tab ids in
  `chrome.tabs.query` order, joined with `", "`, with a trailing period.
- Omitted or null tabId with at least one tab in the group: prefer tabs with
  `active === true`; among that pool (or among all group tabs when none is
  active) pick the highest `lastAccessed`, treating a missing `lastAccessed`
  as 0; on ties keep the first in query order. Return that tab's id.
- Omitted or null tabId with no group or an empty group: throw
  `TabAccessError` with exactly

  ```
  No tabs in the Browser MCP group. Use tabs_create_mcp to open one, or tabs_context_mcp with createIfEmpty: true.
  ```

  Never create a tab, a group, or a window from this helper. Erroring
  truthfully is the required behavior.
- `rawTabId` of `0` counts as provided (only `undefined` and `null` trigger
  the fallback). Do not coerce strings to numbers; a string id fails
  `inGroup` and takes the stale-id path.

### 2. Convert `TabAccessError` to a text result in `dispatch`

Replace the `dispatch` function (lines 558-566) with:

```js
async function dispatch(id, tool, args) {
  const handler = handlers[tool];
  if (!handler) return fail(id, `Unknown tool: ${tool}`);
  try {
    reply(id, await handler(args));
  } catch (e) {
    if (e instanceof TabAccessError) return reply(id, text(e.message));
    fail(id, `${tool} failed: ${(e && e.message) || e}`);
  }
}
```

Rationale you must preserve: today's membership refusals travel as normal
text results. Letting `TabAccessError` fall through to `fail(...)` would
change the wire shape to a `tool_error`, which the binary renders as
`Error: <tool> failed: <message>` with `isError: true`. That is a behavior
change beyond this task; the new branch keeps the exact message and the
exact delivery channel.

### 3. Use the helper in every tabId-bearing handler

Ten handlers change. In each, the resolved id is a local
`const tabId = await effectiveTabId(a.tabId);` and every later use of
`a.tabId` in that handler body becomes `tabId`. Do not assign back onto `a`.

- `computer` (lines 357-359): replace

  ```js
  const tabId = a.tabId;
  if (!(await inGroup(tabId))) return text(`Tab ${tabId} is not in the ${GROUP_TITLE} group.`);
  ```

  with

  ```js
  const tabId = await effectiveTabId(a.tabId);
  ```

  The rest of the function already uses the local `tabId`; leave it alone.

- `navigate` (line 461): replace the `inGroup` check line with the
  `const tabId = ...` line, then change `a.tabId` to `tabId` on lines 463,
  465, 472, 474, and 475. Nothing else in the handler changes (the
  back/forward branches, the URL normalization, `waitForLoad`, and the
  result text stay as they are).

- `read_page` (line 480), `get_page_text` (485), `find` (490), `form_input`
  (500), `javascript_tool` (506): same replacement; the single `content` or
  `cdp` call below each check uses `tabId`. In `read_page`, keep the
  content-script payload `{ type: "accessibilityTree", options: a }`
  untouched.

- `read_console_messages` (line 513): replace the check line; use `tabId` on
  the lines that today read `a.tabId` (514, 516, 517, 524). Filters
  (`a.onlyErrors`, `a.pattern`, `a.limit`, `a.clear`) keep reading from `a`.

- `read_network_requests` (line 528): replace the check line; use `tabId` on
  lines 529, 530, 531, 534. `a.urlPattern`, `a.limit`, `a.clear` keep
  reading from `a`.

- `resize_window` (line 538): replace the check line; line 539 becomes
  `const tab = await chrome.tabs.get(tabId);`. The inner loop on lines
  543-547 declares its own `tabId` over `attached.keys()`, which would
  shadow the new outer const; rename that loop variable to `attachedId`
  (three occurrences: the `for` header, `chrome.tabs.get(attachedId)`, and
  `screenshotCtx.delete(attachedId)`).

`tabs_context_mcp`, `tabs_create_mcp`, and `update_plan` take no tabId and
do not change. The `tabs_context_mcp` message on line 449
(`No Browser MCP tab group. Call with createIfEmpty: true.`) stays exactly
as it is.

### 4. What does not change

- `src/mcp/schemas/tools.json` is untouched: `tabId` stays `required` in
  every schema. This task is behavior only.
- `inGroup`, `ensureGroup`, and `groupTabs` are called, not modified.
- All success-path result texts of every handler are byte-identical to
  today's.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or
   description strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction
   decisions in extension JS. Effective-tab resolution is tab-group
   mechanism and belongs here; do not add any domain or permission logic to
   it.
3. ASCII only in ALL code and docs: no em-dashes, no unicode arrows, no
   curly quotes, anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute
   behavior. The fallback is not silent substitution because the schema
   defines tabId as the target selector and the group's current tab is the
   documented default target; but when NO tab can be resolved you must error
   with the exact messages above, and you must never create tabs, groups, or
   windows to make a call succeed.
5. No new runtime dependencies. Extension stays vanilla JS (no bundler, no
   libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc
   comments on public items, rustfmt clean, clippy with deny warnings. (No
   Rust changes are expected in this task.)
7. Comments only for constraints the code cannot express; match the
   surrounding comment density and style. The two short comments shown on
   `TabAccessError` and `effectiveTabId` are the only new comments needed.
8. Do NOT copy code from the official Anthropic extension or any other
   project; implement the described behavior from scratch.

Task-specific:

9. Only `extension/service-worker.js` changes. No other file.
10. Use the exact names `TabAccessError` and `effectiveTabId`.
11. Do not mutate the incoming `args` object (no `a.tabId = ...`); bind the
    resolved id to a local `const tabId`.
12. Membership refusals must remain plain text tool results (via the new
    `dispatch` branch), never `tool_error` frames.

## Verification

1. Run `cargo test` from the repo root. All tests must pass (this task adds
   no Rust code, so this confirms nothing regressed, including
   `tests/tool_schema_fidelity.rs`).
2. Ask the user to reload the extension at chrome://extensions. No MCP
   client restart is needed because the binary is unchanged; the reloaded
   extension reconnects to the native host on its own.
3. Manual end-to-end check through the MCP client:
   - Call `tabs_context_mcp` with `createIfEmpty: true` and note the real
     tab ids in the group.
   - Call `get_page_text` with `tabId: 999999`. Expect exactly:
     `Tab 999999 is not in the Browser MCP group. Valid tab IDs are: <the real ids, comma separated>.`
   - Call `navigate` and `computer` (action `screenshot`) with a valid
     tabId. Both must behave exactly as before.
4. Fallback checks from the inspected service worker console
   (chrome://extensions, Browser MCP, "service worker" link). The worker is
   a classic script, so the helper is on the global scope:
   - `effectiveTabId(null).then(console.log)` resolves to the id of the
     group's active tab (or its most recently accessed tab if none is
     active).
   - `effectiveTabId(999999).catch((e) => console.log(e.message))` prints
     the valid-IDs message.
   - Close every tab in the group (the group disappears with its last tab),
     then:
     - `effectiveTabId(null).catch((e) => console.log(e.message))` prints
       `No tabs in the Browser MCP group. Use tabs_create_mcp to open one, or tabs_context_mcp with createIfEmpty: true.`
     - `effectiveTabId(999999).catch((e) => console.log(e.message))` prints
       `Tab 999999 is not in the Browser MCP group. The group has no tabs; use tabs_create_mcp to open one.`
   - Confirm no new tab, group, or window appeared during any of these
     failure checks.

## Out of scope

- `tabs_context_mcp` and `tabs_create_mcp` semantics. Their handlers,
  messages, and `createIfEmpty` behavior do not change.
- Creating tabs, groups, or windows when the fallback finds nothing. The
  helper errors truthfully; it never provisions.
- Any edit to `src/mcp/schemas/tools.json` or any Rust file (`src/mcp/`,
  `src/browser.rs`, `src/dispatch.rs`, `src/native/`, tests). tabId stays
  `required` in every schema even though the extension now tolerates its
  absence.
- Changing `inGroup`, `ensureGroup`, or `groupTabs` internals.
- Coercing or validating tabId types (no string-to-number conversion, no
  range checks).
- Converting membership refusals to `tool_error` / `isError` results, or
  adding an `isError` flag anywhere in the extension.
- Touching `extension/content.js`, `extension/agent-visual-indicator.js`,
  or `extension/manifest.json`.
- Rewording any success-path result text, the `Unknown tool:` message, the
  `<tool> failed:` prefix, or the `tabs_context_mcp` no-group message on
  line 449.
