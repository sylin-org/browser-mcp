# T4 -- guid on the tool envelope + session-scoped tab operations (ADR-0047 D3)

## Goal

Every `tool_request` carries the calling session's guid (additive envelope field); the extension
births `tabs_create_mcp` tabs DIRECTLY into the calling session's group (no born-global churn,
no about:blank litter) and scopes `tabs_context_mcp` to that group; the service claims a
session-created tab from the response so no other session can first-touch-steal it. Normative:
ADR-0047 D3. Oracles: PINS.md P4. This is the batch's largest task; the envelope change and its
consumers are one atomic unit (accepted coupling; do not split).

## Files this task owns (touch nothing else)

- `crates/core/src/hub/outbound/browser.rs` (Browser::call signature + envelope + its own tests)
- `crates/core/src/hub/endpoint.rs` (its test only)
- `crates/core/src/mcp/pipeline.rs` (guid threading + its own tests)
- `crates/core/src/mcp/script.rs` (re-entry threading)
- `crates/core/src/mcp/form_fill.rs` (guid threading through `run` to its three browser.call sites)
- `crates/core/src/mcp/outcome.rs` (LocalCtx field)
- `crates/core/src/mcp/server.rs` (SessionSeat, handle_line, tabs_create response claim)
- `tests/hub_multiplex.rs` (two .call sites gain the pinned guid args)
- `extension/service-worker.js` (dispatch guid; handlers; createTabInSessionGroup; tabContext)
- `docs/tasks/tab-identity/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ c49ee6d -- re-read every one before editing)

- `browser.rs`: `pub async fn call(&self, tool: &str, args: &Value)` builds
  `json!({ "id": id, "type": "tool_request", "tool": tool, "args": args })`.
- PRODUCTION 2-arg `.call(` sites on `Browser` across the workspace: two in `pipeline.rs`
  (`let outcome = browser.call(name, args).await;` + one in the navigate-landing region;
  re-locate with `grep -n "\.call(" crates/core/src/mcp/pipeline.rs`) and three in
  `form_fill.rs` (`form_structure_internal`, `form_input`, `computer`). Test-code sites exist in
  `browser.rs`'s own tests, `endpoint.rs`'s test, and `tests/hub_multiplex.rs` -- covered by the
  P4 pins + the P4 BLANKET TEST RULE.
- `pipeline.rs`: `run_tool_call(browser, store, governance, name, &args, None, false)` is called
  from `handle_tools_call`; `script.rs` re-enters via `handle.block_on(run_tool_call(`.
- `outcome.rs`: `pub struct LocalCtx<'a>` has exactly the fields
  `browser, store, governance, config, args`.
- `server.rs`: `handle_line(&browser, &capabilities, &store, &governance, line, &tx)` is called
  from `serve_session`'s read loop; its `"tools/call"` arm spawns
  `pipeline::handle_tools_call(&browser, &store, &governance, id, params.as_ref())`.
- `server.rs`: `check_tab_ownership(line, &owned_tabs, &guid, &governance, &browser)` exists and
  its Adopted arm calls `emit_group_request(browser, owned_tabs, guid)`.
- A second caller of `handle_line` exists (a test named
  `tools_call_produces_one_audit_record_with_client_identity`, per the fn's doc comment) --
  locate it with `grep -rn "handle_line(" crates/core/src/` and update its call.
- `endpoint.rs` test `serve_bridges_a_tool_call_over_the_real_ipc` calls
  `browser.call("navigate", &json!({}))` and asserts `result["echoed"]`.
- Extension anchors: `dispatch(msg.id, msg.tool, msg.args || {});` (tool_request handler);
  `async function dispatch(id, tool, args)`; `reply(id, await handler(args));`;
  `async tabs_context_mcp(a) {`; `async tabs_create_mcp() {`;
  `function tabContext(tabs) {` using `mcpGroupId: groupId`.

## STOP preconditions

- STOP if `grep -rn "\"guid\"" crates/core/src/hub/outbound/browser.rs` already matches inside
  the tool_request envelope (someone landed this already).
- STOP if `LocalCtx` has fields other than the five listed.
- STOP if `grep -rn "structuredContent" crates/core/src/mcp/server.rs` already matches
  (server.rs must not yet read structuredContent; this task adds its first read. NOTE:
  `script.rs` DOES already parse structuredContent for ref resolution -- that is expected and
  is NOT a stop condition).
- STOP if `JsonRpcResponse`'s `result` field (crates/core/src/mcp/types.rs) is not a public,
  value-inspectable `Option<Value>`-like field (see PINS P4).
- STOP if the extension handlers' anchors differ materially (T1 changed only the gate fns).

## Changes (transcribe from PINS P4; order within the task)

1. Core: `Browser::call(&self, guid: &str, tool: &str, args: &Value)`; envelope gains
   `"guid": guid`. Fix both pipeline call sites by threading a new `guid: &str` parameter
   through `run_tool_call` (position pinned in P4) and `handle_tools_call`; add
   `pub guid: &'a str` to `LocalCtx` (thread at its construction sites -- find them with
   `grep -rn "LocalCtx {" crates/core/src/`); `script.rs` passes `ctx.guid`; `form_fill.rs`
   threads per P4 (its `run` gains `guid: &str`; three call sites); `tests/hub_multiplex.rs`
   uses the pinned `"session-a"` / `"session-b"`. Every remaining compile-flagged TEST site
   takes `"test-guid"` per the P4 BLANKET TEST RULE.
2. Core: add `SessionSeat` (PINS P4) in `server.rs`; `serve_session` builds it and passes
   `&seat` to `handle_line`; `handle_line` gains the parameter; the tools/call spawn clones
   what it needs; after `handle_tools_call` returns, when `name == "tabs_create_mcp"`, parse
   `structuredContent.tabId` (i64) from the response's `result` and run
   `crate::hub::session::claim_tab(&seat.owned_tabs, &seat.guid, tab_id)`; on `Adopted`, call
   `emit_group_request(&browser, &seat.owned_tabs, &seat.guid)`. Update the OTHER
   `handle_line` caller (the audit test) with a locally built seat.
3. Core test: update `serve_bridges_a_tool_call_over_the_real_ipc` per PINS P4
   (`"test-guid"` + the `v["guid"]` assertion).
4. Extension: thread `msg.guid` -> `dispatch(id, tool, args, guid)` -> `handler(args, guid)`;
   add `createTabInSessionGroup` (pinned); move the two current handler bodies verbatim to the
   module-level `tabsCreateLegacy` / `tabsContextLegacy` functions and rework the handlers into
   the pinned guid paths (PINS P4 -- the legacy fns are module-level, NEVER handlers-object
   members); widen `tabContext(tabs, reportGroupId)` (pinned).

## Verification (all green)

```
node --check extension/service-worker.js
node --test tests/extension/grouping.test.js
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

`cargo test --workspace` MUST include the updated `serve_bridges_a_tool_call_over_the_real_ipc`
green (the envelope oracle) and the untouched `tests/all_open_golden.rs` green (byte-identity:
initialize/tools/list replies carry no envelope change; if the golden breaks, you changed
something out of scope -- BLOCK).

## Out of scope (fences)

- NO title changes (T5): `emit_group_request` keeps its CURRENT signature and `group_title`.
- NO liveness changes (T6): the response claim uses `claim_tab` (T6 switches it).
- NO change to `check_tab_ownership`'s signature or logic.
- NO change to `tab_url`, `request_group`, or any other native message type.
- NO change to `directory.rs`, error strings, `effectiveTabId`, `ensureGroup`,
  `groupSessionTabs`.

## Commit

Stage exactly the seven named source files. Pinned message (PINS P4):

```
feat(session): guid on the tool envelope + session-scoped tab operations (ADR-0047 D3)
```

Then update LEDGER.md and commit as `docs(tab-identity): ledger T4`.
