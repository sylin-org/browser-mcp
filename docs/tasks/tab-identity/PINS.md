# tab-identity batch -- PINS (the oracles)

Computed by the author against dev @ c49ee6d. Executors TRANSCRIBE these; never re-derive an
expectation (an executor-derived expectation validates its own bugs). Anchors quote current tree
text; re-locate by anchor, not line number. Semantics live in ADR-0047; this file pins wire
shapes, names, strings, signatures, and test assertions only.

## P1 -- managed-surface predicate (T1; ADR-0047 D1)

- New pure functions in `extension/lib/grouping.js`, exported on the existing
  `GhostlightGrouping` namespace object alongside `groupSessionTabs`:

```js
// The managed surface (ADR-0047 D1): every Chrome tab-group id this extension manages -- the
// legacy global group (when set) plus every per-session group it created on service request.
function managedGroupIds(globalGroupId, sessionGroups) {
  const ids = new Set();
  if (globalGroupId !== null && globalGroupId !== undefined) ids.add(globalGroupId);
  for (const gid of sessionGroups.values()) ids.add(gid);
  return ids;
}

// True iff `groupId` (a chrome tab's .groupId; -1 means ungrouped) is a managed group.
function isManagedGroupId(groupId, globalGroupId, sessionGroups) {
  if (groupId === -1 || groupId === null || groupId === undefined) return false;
  return managedGroupIds(globalGroupId, sessionGroups).has(groupId);
}
```

  Export shape: `const GhostlightGrouping = { groupSessionTabs, managedGroupIds, isManagedGroupId };`

- `service-worker.js` gate rewiring (anchors are current source text):
  - `inGroup`: keep the existing title self-heal branch verbatim; replace ONLY the final
    membership line `return tab.groupId === groupId;` with
    `return isManagedGroupId(tab.groupId, groupId, sessionGroups);`
    and destructure the two new fns at the top alongside the existing
    `const { groupSessionTabs } = self.GhostlightGrouping;` ->
    `const { groupSessionTabs, managedGroupIds, isManagedGroupId } = self.GhostlightGrouping;`
  - `groupTabs`: replace the body (anchor: `return groupId === null ? [] : chrome.tabs.query({ groupId });`)
    with the union over managed ids, global group first, then `sessionGroups` insertion order:

```js
async function groupTabs() {
  const ids = managedGroupIds(groupId, sessionGroups);
  const all = [];
  for (const gid of ids) {
    try {
      all.push(...(await chrome.tabs.query({ groupId: gid })));
    } catch { /* a vanished group contributes no tabs */ }
  }
  return all;
}
```

  - The stale comment above `sessionGroups` (anchor: "so that check cannot become session-aware.
    `sessionGroups` backs ONLY the group_request") is rewritten to state the ADR-0047 D1 reality:
    the gate consults `sessionGroups` through the managed-surface predicate; cite ADR-0047 D1.
  - `effectiveTabId`, `ensureGroup`, error strings: UNCHANGED in T1 (T5 owns the strings).

- The test file's require line (currently
  `const { groupSessionTabs } = require("../../extension/lib/grouping.js");`) becomes
  `const { groupSessionTabs, managedGroupIds, isManagedGroupId } = require("../../extension/lib/grouping.js");`
- New tests appended to `tests/extension/grouping.test.js` (node:test, same style):
  - `test("managed_surface_accepts_global_and_session_groups", ...)` with EXACTLY these
    assertions (note the numeric sort comparator; a bare `.sort()` sorts lexicographically and
    fails):

```js
  const m = new Map([["S", 9], ["T", 12]]);
  assert.deepStrictEqual(
    Array.from(managedGroupIds(7, m)).sort((a, b) => a - b),
    [7, 9, 12]
  );
  assert.strictEqual(isManagedGroupId(9, 7, m), true);
  assert.strictEqual(isManagedGroupId(7, 7, m), true);
```

  - `test("managed_surface_rejects_foreign_and_ungrouped", ...)`:

```js
  const m = new Map([["S", 9], ["T", 12]]);
  assert.strictEqual(isManagedGroupId(8, 7, m), false);
  assert.strictEqual(isManagedGroupId(-1, 7, m), false);
  assert.strictEqual(isManagedGroupId(5, null, new Map()), false);
  assert.strictEqual(managedGroupIds(null, new Map()).size, 0);
```

- Pinned commit message (T1):
  `fix(extension): managed-surface tab gate -- recognize every Ghostlight-managed group (ADR-0047 D1)`

## P2 -- relay down-classifier (T2; ADR-0047 D6)

- In `crates/transport/src/ipc.rs`, replace the `down` arm of `relay_session` (anchor:
  `let down = async {` ... `tokio::io::copy(ipc_read, client_out)`) with a call to a new
  private async fn (same file):

```rust
/// The service->client relay direction (ADR-0047 D6, amending ADR-0045): a manual copy loop so
/// the two failure sides classify differently. Reading 0 bytes OR a read error from the service
/// pipe is the SERVICE side ending (reconnect); only a failed write toward the client is the
/// CLIENT side ending (exit). The pre-0047 `tokio::io::copy` arm collapsed both error kinds into
/// ClientClosed, which on Windows (an abrupt service death often surfaces as ERROR_BROKEN_PIPE
/// on the read) exited the adapter and forced the client reload ADR-0045 exists to prevent.
async fn copy_service_to_client<R, W>(ipc_read: &mut R, client_out: &mut W) -> RelaySide
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = [0u8; 8192];
    loop {
        match ipc_read.read(&mut buf).await {
            Ok(0) => return RelaySide::ServiceClosed, // service EOF
            Ok(n) => {
                if client_out.write_all(&buf[..n]).await.is_err()
                    || client_out.flush().await.is_err()
                {
                    return RelaySide::ClientClosed; // writing to the client failed
                }
            }
            Err(_) => return RelaySide::ServiceClosed, // service read error (e.g. broken pipe)
        }
    }
}
```

  The `down` arm becomes `let down = copy_service_to_client(ipc_read, client_out);` (note: no
  `async {}` wrapper needed; it is already a future).
- New unit tests in `crates/transport/src/ipc.rs`'s existing `#[cfg(test)] mod tests` (add one if
  the module has none -- STOP precondition in the task confirms which):
  - `down_eof_classifies_service_closed`: `tokio::io::duplex(64)`; drop the ENTIRE service-side
    `DuplexStream` (the whole second half of the pair -- dropping only a split WriteHalf does
    NOT produce EOF and the read pends forever); assert `copy_service_to_client(...)` returns
    `RelaySide::ServiceClosed`.
  - `down_read_error_classifies_service_closed`: a 6-line local `struct FailingReader;`
    implementing `AsyncRead` whose `poll_read` returns
    `Poll::Ready(Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe)))`; assert
    `ServiceClosed`.
  - `down_client_write_error_classifies_client_closed`: service side = a duplex carrying one
    pending byte (write `b"x"` then keep the handle alive); client side = a `struct
    FailingWriter;` whose `poll_write` returns `Poll::Ready(Err(BrokenPipe))`; assert
    `ClientClosed`.
- ADR-0045 amendment: APPEND (never edit existing text) to
  `docs/adr/0045-resilient-reconnecting-adapter.md` a section titled exactly
  `## Amendment (2026-07-08, ADR-0047 D6): down-relay error classification` with 3-6 lines
  stating the reclassification (read-side error -> reconnect) and citing ADR-0047 D6.
- Pinned commit message (T2):
  `fix(transport): classify service-side read errors as reconnect, not client exit (ADR-0047 D6)`

## P3 -- stable per-process SessionGuid (T3; ADR-0047 D2)

- `crates/transport/src/ipc.rs`:
  - `relay_adapter`: mint ONCE before the reconnect loop:
    `let session_guid = crate::session_guid::SessionGuid::mint();`
    and add, immediately after the existing first-connect debug note branch is set up (before
    the loop), exactly one debug event:
    `debug.ipc_note("session identity minted (stable for this adapter process)");`
  - `try_connect_once` signature becomes
    `async fn try_connect_once(adapter_endpoint: &str, guid: &crate::session_guid::SessionGuid)`
    and uses the passed guid in the hello (delete its local `SessionGuid::mint()` line). The
    hello JSON shape is UNCHANGED: `{ "hub": .., "role": "adapter", "guid": .. }`.
  - `connect_and_handshake` gains the same `guid: &crate::session_guid::SessionGuid` parameter
    and threads it into both `try_connect_once` call sites.
  - REWRITE the stale doc-comment sentence on `relay_adapter` (anchor: "A fresh `SessionGuid` is
    minted per (re)connect: a reconnect is a NEW session to the service (the old one's slot
    freed when its connection dropped), which is exactly right.") to state ADR-0047 D2: one guid
    per adapter process, re-presented on every reconnect via the registry's sanctioned same-user
    reuse path, so ownership and the session's Chrome group survive the gap; cite ADR-0047 D2.
    Also rewrite `try_connect_once`'s "(a fresh `SessionGuid` per attempt)" phrase to
    "(the caller's stable per-process `SessionGuid`)".
- New unit test in `crates/transport/src/ipc.rs` tests module:
  - `hello_carries_the_caller_guid`: build the hello `serde_json::json!` VALUE exactly as
    `try_connect_once` does (extract a tiny private helper
    `fn adapter_hello(guid: &crate::session_guid::SessionGuid) -> serde_json::Value` used by
    `try_connect_once`, and test THAT): assert `hello["guid"] == guid.as_str()`,
    `hello["role"] == "adapter"`, `hello["hub"] == 1`, and that calling it twice with the same
    guid yields identical values.
- Integration pin, extend `tests/adapter_reconnect.rs`
  `adapter_reconnects_across_a_service_restart_without_a_client_reload` ONLY (leave the 5s-gap
  test untouched): after the existing `list2` assertions, read every `debug-events-*.jsonl`
  under `log_dir`, concatenate their raw text, and assert by SUBSTRING COUNT (the pinned,
  sufficient check -- no JSON parsing):
  - occurrences of `session identity minted (stable for this adapter process)` == 1
  - occurrences of `service restart detected; reconnected` >= 1
- ADR-0045 amendment: APPEND a section titled exactly
  `## Amendment (2026-07-08, ADR-0047 D2): stable session identity across reconnects` (3-8
  lines; supersedes the fresh-guid-per-reconnect posture; cite ADR-0047 D2).
- Pinned commit message (T3):
  `feat(transport): stable per-process session guid -- reconnects resume identity (ADR-0047 D2)`

## P4 -- guid on the tool envelope + session-scoped tab operations (T4; ADR-0047 D3)

Wire pin (additive; every existing field byte-identical):

```
{ "id": "<n>", "type": "tool_request", "tool": "<name>", "args": { ... }, "guid": "<session guid>" }
```

Core-side signature pins (thread the guid; the compiler finds every call site -- the known ones
are listed in the task):

- `Browser::call(&self, guid: &str, tool: &str, args: &Value)` (guid FIRST). The envelope gains
  `"guid": guid`. `Browser::tab_url` is UNCHANGED.
- `pipeline::run_tool_call(browser, store, governance, guid: &str, name, args, orchestration, dry_run)`
  (guid after governance).
- `pipeline::handle_tools_call(browser, store, governance, guid: &str, id, params)`.
- `LocalCtx` gains `pub guid: &'a str,` (after `governance`); `script.rs`'s re-entry
  (`handle.block_on(run_tool_call(` anchor) passes `ctx.guid` through.
- `form_fill.rs` (three 2-arg `browser.call(` sites: `form_structure_internal`, `form_input`,
  `computer`): its `run` fn gains `guid: &str` after `governance`
  (`run(browser, governance, guid, args)`); its `Handler::Local` closure passes `ctx.guid`; all
  three calls become `browser.call(guid, ...)`.
- `tests/hub_multiplex.rs` (two 2-arg `.call(` sites on session handles): pass `"session-a"` and
  `"session-b"` respectively as the new first argument.
- BLANKET TEST RULE: every OTHER test call site the compiler flags after these signature changes
  (the `browser.rs` `#[cfg(test)]` module's own `browser.call(...)` sites, `handle_tools_call`
  test invocations, `LocalCtx { ... }` test literals, and any similar) uses the literal guid
  `"test-guid"`. This rule is the sanctioned fix; no site needs individual pinning.
- `server::handle_line` gains a `seat: &SessionSeat` parameter, positioned after `governance`
  and before `line` (`handle_line(&browser, &capabilities, &store, &governance, &seat, line, &tx)`),
  where (in `server.rs`):

```rust
/// One session's identity + shared-state handles (ADR-0047 D3), threaded from `serve_session`
/// into the dispatch arms so a spawned tools/call can claim tabs the session creates.
pub(super) struct SessionSeat {
    pub(super) guid: SessionGuid,
    pub(super) owned_tabs: Arc<Mutex<HashMap<i64, SessionGuid>>>,
}
```

  `serve_session` constructs it once (cloning the Arc it already destructured) and passes
  `&seat` to `handle_line`; `check_tab_ownership` keeps its current parameters (it already
  receives `owned_tabs` and `guid` separately -- leave it).
- The tools/call arm's spawned task (anchor: `pipeline::handle_tools_call(&browser, &store,
  &governance, id, params.as_ref())`): BEFORE the spawn, extract the called tool name once --
  `let tool_name = params.as_ref().and_then(|p| p.get("name")).and_then(Value::as_str).map(str::to_string);`
  -- and clone `seat.guid` + `seat.owned_tabs` into the spawn. After obtaining `resp`, when
  `tool_name.as_deref() == Some("tabs_create_mcp")`, parse the created tab id from the
  response's `result` -- `resp.result.as_ref().and_then(|r| r.get("structuredContent")).and_then(|s| s.get("tabId")).and_then(Value::as_i64)`
  -- and, when present, run `claim_tab(...)`; on `TabClaim::Adopted` call
  `emit_group_request(...)`. A missing/unparseable tabId is a silent no-op (the extension may
  have failed the call). (STOP precondition: verify `JsonRpcResponse`'s `result` field is a
  public `Option<Value>` -- read `crates/core/src/mcp/types.rs`; if its shape differs, adapt the
  accessor and log the deviation, or BLOCK if it is not value-inspectable.)
- The OTHER `handle_line` caller (`tools_call_produces_one_audit_record_with_client_identity`
  in `pipeline.rs`'s tests) builds a local seat:
  `let seat = crate::mcp::server::SessionSeat { guid: SessionGuid::mint(), owned_tabs: Arc::new(Mutex::new(HashMap::new())) };`
  (`pub(super)` = `pub(in crate::mcp)`, visible from `mcp::pipeline`'s tests.)
- `mcp::server`'s one existing `Browser::call` test-companion in `crates/core/src/hub/endpoint.rs`
  test `serve_bridges_a_tool_call_over_the_real_ipc` updates its call to
  `browser.call("test-guid", "navigate", &json!({}))` and its fake native-host assertion reads
  `v["guid"] == "test-guid"` in addition to the existing `v["tool"]` echo.

Extension-side pins:

- The tool_request handler (anchor: `dispatch(msg.id, msg.tool, msg.args || {});`) becomes
  `dispatch(msg.id, msg.tool, msg.args || {}, msg.guid);`
- `async function dispatch(id, tool, args)` gains a 4th parameter `guid`; the handler invocation
  (anchor: `reply(id, await handler(args));`) becomes `reply(id, await handler(args, guid));`
  (extra args are harmless to one-parameter handlers).
- The two LEGACY bodies move to MODULE-LEVEL functions (placed directly above the `handlers`
  object), NOT members of the handlers object -- `dispatch` invokes handlers as bare function
  references (no `this` binding), and a handlers-object member would also become dispatchable
  as a tool name:

```js
// Pre-0047 tabs_create_mcp behavior, kept verbatim for guid-less legacy/native callers
// (ADR-0047 D3): global-group birth via ensureGroup(true).
async function tabsCreateLegacy() { ...the CURRENT tabs_create_mcp body, moved verbatim... }

// Pre-0047 tabs_context_mcp behavior, kept verbatim for guid-less legacy/native callers
// (ADR-0047 D3): the global group's view.
async function tabsContextLegacy(a) { ...the CURRENT tabs_context_mcp body, moved verbatim... }
```

- `tabs_create_mcp` handler (replace whole body; anchor `async tabs_create_mcp() {`):

```js
  async tabs_create_mcp(_a, guid) {
    if (typeof guid !== "string" || !guid) return tabsCreateLegacy();
    const { tab, gid } = await createTabInSessionGroup(guid);
    await persistSessionState();
    const r = tabContext(await chrome.tabs.query({ groupId: gid }), gid);
    r.content[0].text = `Created tab ${tab.id}.\n` + r.content[0].text;
    r.structuredContent = { tabId: tab.id, tabs: r.structuredContent.tabs };
    return r;
  },
```

- `tabs_context_mcp` handler (replace whole body; anchor `async tabs_context_mcp(a) {`):

```js
  async tabs_context_mcp(a, guid) {
    if (typeof guid !== "string" || !guid) return tabsContextLegacy(a);
    let gid = sessionGroups.has(guid) ? sessionGroups.get(guid) : null;
    if (gid !== null) {
      try { await chrome.tabGroups.get(gid); } catch { gid = null; }
    }
    if (gid === null) {
      if (!a.createIfEmpty) {
        return text("No Ghostlight tab group for this session. Call tabs_context_mcp with createIfEmpty: true, or create a tab with tabs_create_mcp.");
      }
      gid = (await createTabInSessionGroup(guid)).gid;
      await persistSessionState();
    }
    return tabContext(await chrome.tabs.query({ groupId: gid }), gid);
  },
```

- New helper (place beside `ensureGroup`; the ONE place session-group birth happens):

```js
// Session-group birth (ADR-0047 D3): create a tab directly inside `guid`'s group. First tab of
// a session: one focused window whose single fresh tab becomes the group (no about:blank
// litter); later tabs: a tab in the group's window, grouped immediately. The GROUP_TITLE
// placeholder is retitled by the service's next group_request (client-name title, ADR-0047 D4).
async function createTabInSessionGroup(guid) {
  let gid = sessionGroups.has(guid) ? sessionGroups.get(guid) : null;
  if (gid !== null) {
    try { await chrome.tabGroups.get(gid); } catch { gid = null; }
  }
  let tab;
  if (gid === null) {
    const win = await chrome.windows.create({ focused: true });
    tab = win.tabs[0];
    gid = await chrome.tabs.group({ tabIds: [tab.id] });
    await chrome.tabGroups.update(gid, { title: GROUP_TITLE, color: "blue" });
  } else {
    const group = await chrome.tabGroups.get(gid);
    tab = await chrome.tabs.create({ active: true, windowId: group.windowId });
    await chrome.tabs.group({ tabIds: [tab.id], groupId: gid });
  }
  sessionGroups.set(guid, gid);
  return { tab, gid };
}
```

- `tabContext(tabs)` gains a second parameter: `function tabContext(tabs, reportGroupId)` with
  `const gid = reportGroupId === undefined ? groupId : reportGroupId;` used for BOTH the JSON
  text and `structuredContent.mcpGroupId`; the two legacy callers pass nothing (unchanged
  behavior), the session-scoped callers pass their `gid`.

Rust-side test pin (new file `tests/tool_envelope_guid.rs` is NOT created; instead):
- extend `crates/core/src/hub/endpoint.rs`'s existing `serve_bridges_a_tool_call_over_the_real_ipc`
  as pinned above (that IS the envelope-carries-guid oracle over the real IPC).

Pinned commit message (T4):
`feat(session): guid on the tool envelope + session-scoped tab operations (ADR-0047 D3)`

## P5 -- client-name titles + recovery-steering errors (T5; ADR-0047 D4)

- `crates/core/src/hub/session.rs`: DELETE `group_title` and its test
  `group_title_matches_the_pinned_format` (superseded by ADR-0047 D4; PINS.md SS6 of the hub
  batch is history, not edited). ADD:

```rust
/// The per-session Chrome group title (ADR-0047 D4, superseding the hub batch's SS6 pin):
/// `"\u{1F47B} <client name>"`, deduplicated with `" (2)"`, `" (3)"`, ... when another session
/// already holds the same base title, falling back to the literal name `Ghostlight` when no
/// clientInfo was captured. Computed once per guid in the service-lifetime `titles` registry and
/// reused for every later request for the SAME guid (stable across reconnects, ADR-0047 D2).
pub fn session_title(
    titles: &Mutex<HashMap<String, String>>,
    guid: &SessionGuid,
    client_name: Option<&str>,
) -> String {
    let mut map = titles.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(existing) = map.get(guid.as_str()) {
        return existing.clone();
    }
    let name = client_name.unwrap_or("Ghostlight");
    let base = format!("\u{1F47B} {name}");
    let mut candidate = base.clone();
    let mut n = 1u32;
    while map.values().any(|t| t == &candidate) {
        n += 1;
        candidate = format!("{base} ({n})");
    }
    map.insert(guid.as_str().to_string(), candidate.clone());
    candidate
}
```

- New pinned test in `session.rs` tests module, `session_title_uses_client_name_with_dedupe_and_fallback`:
  with a fresh `Mutex<HashMap>` and three minted guids:
  - `session_title(&t, &g1, Some("Claude Code"))` == `"\u{1F47B} Claude Code"`
  - `session_title(&t, &g2, Some("Claude Code"))` == `"\u{1F47B} Claude Code (2)"`
  - `session_title(&t, &g1, Some("Claude Code"))` (repeat call, same guid) ==
    `"\u{1F47B} Claude Code"` (cached, not `(3)`)
  - `session_title(&t, &g3, None)` == `"\u{1F47B} Ghostlight"`
- `ServiceContext` gains `pub session_titles: Arc<std::sync::Mutex<HashMap<String, String>>>,`
  (after `owned_tabs`), initialized `Arc::new(Mutex::new(HashMap::new()))` in `from_startup`.
  TWO integration tests build `ServiceContext` literals field-by-field and get an E0063 missing-
  field error from the new field: `tests/hub_isolation.rs` and `tests/hub_queue.rs`. Add the
  one-line initializer `session_titles: Arc::new(Mutex::new(HashMap::new())),` to each literal
  (matching each file's existing import style). Both files are in T5's owned list for exactly
  this one line each.
- `emit_group_request` signature becomes
  `fn emit_group_request(browser: &Browser, owned_tabs: &..., titles: &Mutex<HashMap<String,String>>, governance: &Governance, guid: &SessionGuid)`
  and builds the title via
  `session_title(titles, guid, governance.current_client().as_ref().map(|c| c.name.as_str()))`.
  (Verified at authoring: `Governance::current_client` (dispatch.rs) is
  `pub(crate) fn current_client(&self) -> Option<ClientInfo>` with
  `ClientInfo { pub name: String, pub version: String }`; the pinned expression compiles. The
  task still re-verifies as its STOP precondition.) Both call sites (`check_tab_ownership`'s
  Adopted arm; T4's tabs_create response claim) thread the new arguments; `serve_session`
  destructures `ServiceContext` FIELD-BY-FIELD with no `..`, so the new field MUST be bound
  explicitly there (omitting it is a compile error that points at the exact spot).
- Extension error strings (T5's second half). Each replacement is the WHOLE STATEMENT and MUST
  stay a `throw new TabAccessError(...)` (dispatch's `instanceof TabAccessError` branch converts
  these to plain text tool results; a bare Error would silently become a tool_error instead):
  - anchor `is not in the ${GROUP_TITLE} group. The group has no tabs` -- statement becomes:
    `` throw new TabAccessError(`Tab ${rawTabId} is not a tab Ghostlight manages, and there are no managed tabs yet. Create one with tabs_create_mcp.`); ``
  - anchor `is not in the ${GROUP_TITLE} group. Valid tab IDs are:` -- statement becomes:
    `` throw new TabAccessError(`Tab ${rawTabId} is not a tab Ghostlight manages. Valid tab IDs: ${tabs.map((t) => t.id).join(", ")}. List them with tabs_context_mcp.`); ``
  - anchor `No tabs in the ${GROUP_TITLE} group. Use tabs_create_mcp` -- statement becomes:
    `` throw new TabAccessError(`No Ghostlight tabs yet. Create one with tabs_create_mcp, or call tabs_context_mcp with createIfEmpty: true.`); ``
  - The legacy `tabsContextLegacy` body's `No ${GROUP_TITLE} tab group. Call with
    createIfEmpty: true.` string stays VERBATIM (legacy path, frozen).
- Pinned commit message (T5):
  `feat(session): client-name tab-group titles + recovery-steering tab errors (ADR-0047 D4)`

## P6 -- ownership liveness + pruning (T6; ADR-0047 D5)

- `ServiceContext` gains `pub live_guids: Arc<std::sync::Mutex<HashMap<String, usize>>>,`
  (after `live_sessions`), initialized empty in `from_startup`. As in P5, add the one-line
  initializer `live_guids: Arc::new(Mutex::new(HashMap::new())),` to the `ServiceContext`
  literals in `tests/hub_isolation.rs` and `tests/hub_queue.rs` (both in T6's owned list).
- `server.rs`: a second RAII guard beside `LiveSessionGuard` (same file, same pattern):

```rust
/// Marks this session's guid live for the ownership gate (ADR-0047 D5): a tab owned by a guid
/// with NO live session is adoptable by another session. Counted (not boolean) because a
/// reconnect's new connection can briefly overlap the old one's teardown.
struct LiveGuidGuard {
    live_guids: Arc<Mutex<HashMap<String, usize>>>,
    guid: String,
}
```

  `new(live_guids, guid)` increments the entry; `Drop` decrements and removes at zero.
  `serve_session` constructs it right after `_live_guard`.
- `crates/core/src/hub/session.rs` new fn (claim_tab and owns_or_adopts_tab stay UNCHANGED):

```rust
/// ADR-0047 D5: [`claim_tab`] with liveness-aware refusal -- a DIFFERENT owner only refuses the
/// claim while that owner has a live session; a dead session's tab is reassigned to the claimer
/// (first-touch adoption from the dead, reported as `Adopted` so the group request fires).
pub fn claim_tab_live(
    owned_tabs: &Mutex<HashMap<i64, SessionGuid>>,
    live_guids: &Mutex<HashMap<String, usize>>,
    guid: &SessionGuid,
    tab_id: i64,
) -> TabClaim {
    let mut map = owned_tabs.lock().unwrap_or_else(PoisonError::into_inner);
    match map.get(&tab_id) {
        Some(owner) if owner == guid => TabClaim::Owned,
        Some(owner) => {
            let live = live_guids
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(owner.as_str())
                .copied()
                .unwrap_or(0)
                > 0;
            if live {
                TabClaim::Refused
            } else {
                map.insert(tab_id, guid.clone());
                TabClaim::Adopted
            }
        }
        None => {
            map.insert(tab_id, guid.clone());
            TabClaim::Adopted
        }
    }
}
```

- THREADING (the one pinned mechanism; supersedes P4's two-field `SessionSeat` shape):
  `SessionSeat` gains `pub(super) live_guids: Arc<Mutex<HashMap<String, usize>>>,` (bound from
  the context in `serve_session`), and `check_tab_ownership` gains a
  `live_guids: &Mutex<HashMap<String, usize>>` parameter (after `owned_tabs`). Both gate sites
  -- `check_tab_ownership` and T4's tabs_create response claim (which reads `seat.live_guids`)
  -- switch from `claim_tab` to `claim_tab_live`; every OTHER `claim_tab` caller (tests,
  `owns_or_adopts_tab`) stays on the old fn. The pipeline.rs audit-test seat gains the third
  field (`live_guids: Arc::new(Mutex::new(HashMap::new()))`).
- New pinned tests in `session.rs`:
  - `dead_owner_tab_is_adoptable_by_a_live_session`: empty live map; A claims 5 (Adopted via
    `claim_tab_live`); B claims 5 with A NOT live -> `Adopted`; A re-claims 5 with B live
    (insert B's guid -> 1 in the live map) -> `Refused`.
  - `live_owner_tab_stays_refused`: A live (count 1); A claims 5; B claims 5 -> `Refused`.
- Extension pruning, pure fn in `extension/lib/grouping.js` (exported on the namespace):

```js
// ADR-0047 D5 hygiene: drop sessionGroups entries whose Chrome group no longer exists. Returns
// true when anything was removed (the caller persists). Probes group liveness only; reads no
// tab or group content.
async function pruneDeadGroups(chrome, sessionGroups) {
  let changed = false;
  for (const [guid, gid] of Array.from(sessionGroups.entries())) {
    try {
      await chrome.tabGroups.get(gid);
    } catch {
      sessionGroups.delete(guid);
      changed = true;
    }
  }
  return changed;
}
```

  Called from `rehydrate()` right after the `sessionGroupsState` restore loop:
  `if (await pruneDeadGroups(chrome, sessionGroups)) await persistSessionState();`
- The test file's require line gains `pruneDeadGroups` (T6 extends the T1 destructure).
- New test in `tests/extension/grouping.test.js`,
  `dead_groups_are_pruned_from_the_session_map`, using a small INLINE fake (do NOT modify the
  existing `fakeChrome` helper; its `liveGroupIds` set is populated only via `chrome.tabs.group`
  and cannot express a pre-existing live group):

```js
test("dead_groups_are_pruned_from_the_session_map", async () => {
  const chrome = {
    tabGroups: {
      async get(groupId) {
        if (groupId !== 9) throw new Error(`no such group ${groupId}`);
        return { id: 9 };
      },
    },
  };
  const sessionGroups = new Map([["S", 9], ["T", 12]]);
  assert.strictEqual(await pruneDeadGroups(chrome, sessionGroups), true);
  assert.deepStrictEqual(Array.from(sessionGroups.entries()), [["S", 9]]);
  assert.strictEqual(await pruneDeadGroups(chrome, sessionGroups), false);
});
```
- CHANGELOG.md: add an `### Fixed` / `### Changed` entry block for ADR-0047 under the
  Unreleased heading (the task pins the exact lines).
- Pinned commit message (T6):
  `feat(session): ownership liveness -- dead-owner adoption + group-map pruning (ADR-0047 D5)`

## Cross-cutting pins

- Ledger commit message per task: `docs(tab-identity): ledger T<n>` (T1..T6).
- The ghost glyph is ALWAYS the `\u{1F47B}` escape in source (Rust and JS); never the literal.
- No task edits `crates/core/src/browser/directory.rs`, the two adapters, or anything on the
  BOOTSTRAP NEVER list.
- After T4, `Browser::call` has exactly 3 params everywhere; a leftover 2-arg call is a compile
  error the executor fixes by threading the real session guid in PRODUCTION code (form_fill via
  `ctx.guid` is the pinned example) and by the P4 BLANKET TEST RULE (`"test-guid"`, or the
  pinned per-file values for `hub_multiplex.rs`) in test code.
