# K4: GET /api/v1/sessions + the sessions/groups UI

Cites: `docs/adr/0030-ghostlight-hub-orchestrator.md` Decision 4, Decision 6, Decision 9;
`docs/tasks/console/PINS.md` CS1 (route table), CS3 (response shape), CS9 (the accessor K1
landed). Needs K1 (`live_session_summaries`) and K2 (router + shell) DONE. Read
`docs/tasks/console/BOOTSTRAP.md` in full first.

## What this task is

The "live sessions/groups" half of the Console's control plane (ADR-0030 Decision 9). Read only.
Shows how many sessions are live right now and, for adapter sessions admitted since the service
started, a TRUNCATED (never full) GUID prefix, OS pid, and owned tabIds -- never a manifest,
never a full session identity, never anything web/WS sessions do not yet expose (H8's own forward
guidance: a web session never calls `SessionRegistry::admit`, so it never appears in
`adapter_bindings`; only `live_session_count` reflects it).

## Current-tree facts

- K1 landed `session::live_session_summaries(registry: &Mutex<SessionRegistry>, owned_tabs:
  &Mutex<HashMap<i64, SessionGuid>>) -> Vec<SessionSummary>` with `SessionSummary { guid: String
  (8 chars), pid: u32, owned_tab_ids: Vec<i64> }`. `ServiceContext` already exposes
  `session_registry: Arc<Mutex<SessionRegistry>>`, `owned_tabs: Arc<Mutex<HashMap<i64,
  SessionGuid>>>`, and `live_sessions: Arc<AtomicUsize>` as `pub` fields.
- K2 landed the router this task adds one more row to.

## STOP preconditions

- If `SessionSummary`'s `guid` field is anything other than exactly 8 ASCII characters (re-verify
  K1 actually landed the truncation, not the full canonical string), STOP -- do not serve a full
  GUID from this route under any circumstance; this is a hard security invariant (ADR-0030
  Decision 4), not a style preference.

## Required behavior

1. Add ONE row to CS1's table: `GET /api/v1/sessions`, gated by the SAME `channels.webapi.from`
   decision every other Console route already uses.
2. The handler builds the EXACT JSON shape PINS.md CS3 pins: `live_session_count` from
   `ctx.live_sessions.load(Ordering::Relaxed)`, `adapter_bindings` from
   `session::live_session_summaries(&ctx.session_registry, &ctx.owned_tabs)` (each entry
   `{"guid", "pid", "owned_tab_ids"}`), and the literal `note` string from CS3 verbatim (the
   honest-limitation disclosure belongs in the API response itself, not just a doc comment).
3. Update the Console page (K2's shell) to render a sessions section: a live count and a list of
   adapter bindings (guid prefix, pid, tab count or ids). No specific byte-for-byte markup is
   pinned.

## Tests to write FIRST

Same file convention as K3 (reuse `tests/console_static_routes.rs` or a focused new
`tests/console_sessions_api.rs`, reusing K2's port-uniqueness helper, never duplicating it):

- `sessions_api_reports_a_live_adapter_session_with_truncated_guid`: spawn a real service
  (`support::spawn_service_with_webapi_port`) and a real adapter (`support::spawn_adapter`), drive
  one `tools/call` naming a `tabId` through the adapter (mirroring
  `tests/hub_completion_criteria.rs`'s own pattern for touching a tab through a real adapter --
  reuse its shape, including a fake extension standing in for Chrome if the tool call needs one to
  complete, or a tool/tabId combination that does not require a real browser response if you find
  one that reaches `SessionRegistry::admit`/`owned_tabs` without needing the extension connected --
  verify which is true by reading `check_tab_ownership`'s actual gate order before assuming),
  then fetch `GET /api/v1/sessions` over real TCP and assert: `live_session_count >= 1`,
  `adapter_bindings` contains an entry whose `guid` is exactly 8 hex-looking lowercase characters
  (never the full 36-character canonical form -- assert the LENGTH is 8, not just "looks
  truncated"), and whose `owned_tab_ids` contains the tabId the adapter touched.
- `sessions_api_never_serves_a_full_guid`: a direct unit-level test (no HTTP needed) asserting
  `session::live_session_summaries`' own output never contains a 36-character or hyphen-bearing
  string in its `guid` field, for a registry seeded with a real minted `SessionGuid` (this may
  already be covered by K1's own unit test of `live_session_summaries` -- if so, do not duplicate
  it; just confirm it exists and is sufficient, and skip adding a second one).

## Out of scope

- No write action (K5).
- No config data (K3).
- No web/WS session identity tracking (out of scope per CS3's own documented limitation; do not
  add a new tracking mechanism to make web sessions appear in `adapter_bindings`).
