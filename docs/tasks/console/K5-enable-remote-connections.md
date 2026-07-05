# K5: POST /api/v1/config/webapi-enable-remote + its UI control

Cites: `docs/adr/0030-ghostlight-hub-orchestrator.md` Decision 5, Decision 9; `docs/tasks/console/
PINS.md` CS1 (route table), CS4 (`config_changed` audit event), CS5 (request/response shapes,
disclaimer text). Needs K1 (`set_user_value`, `CHANNELS_WEBAPI_FROM`) and K2 (router + shell)
DONE. Read `docs/tasks/console/BOOTSTRAP.md` in full first, especially its "must NEVER write
anything except the single ... key" and "must NEVER implement token mint/revoke" fences.

## What this task is

The Console's ONE write action: "Enable remote connections," which writes the single user-layer
`channels.webapi.from` key to `["*"]` (never a caller-supplied value -- the POST body is ignored),
refuses cleanly under an org-mandatory lock, and records exactly one new `config_changed` audit
event on success. This is the ENTIRE write surface this batch ships; no other key, no manifest
edit, no token issuance.

## Current-tree facts

- K1 landed `crate::governance::config::cli::set_user_value(key: &str, value: serde_json::Value,
  domain_pattern_valid: fn(&str) -> bool) -> crate::Result<std::path::PathBuf>` (`pub(crate)`,
  reachable from `src/hub`) and the `CHANNELS_WEBAPI_FROM` constant.
- `ServiceContext.recorder: Arc<Recorder>` already implements `governance::ports::AuditSink`
  (`record_session_event` writes one JSONL line via the SAME path
  `Governance::record_session_killed` etc. ultimately use: `self.audit.record_session_event(
  &record)`).
- `set_user_value` internally calls `resolve_with_warnings`, which reads the REAL, PLATFORM-FIXED
  user config path (`governance::config::load::user_config_path()`, backed by the `dirs` crate's
  `config_dir()` -- e.g. `%APPDATA%\ghostlight\config.json` on Windows) and the REAL, PLATFORM-
  FIXED org policy path (`load::org_policy_path()`, e.g. `%ProgramData%\ghostlight\...` on
  Windows) -- NEITHER has a `GHOSTLIGHT_*`-style env override in `load.rs` today (confirm this
  yourself: `grep -n "env::var" src/governance/config/load.rs`). This is a REAL hazard for this
  task's own tests: a naive real-spawned-service-plus-real-POST test would write to the ACTUAL
  test-runner machine's real Ghostlight config file, exactly the kind of unauthorized real-machine
  side effect this batch's tests must never cause (mirroring the Hub batch's own H9 rule: never
  actually invoke a real OS-level side-effecting path from a test).
- `tests/support/mod.rs`'s `spawn_service_with_program_data(endpoint, program_data_dir)` already
  establishes the EXACT precedent needed here: it isolates `load::org_policy_path()`'s resolution
  for a spawned test service by setting the `ProgramData` environment variable on the CHILD
  PROCESS ONLY (never touching this machine's real `ProgramData`), because `org_policy_path()`'s
  own platform resolution reads that env var. The SAME technique, applied to whichever env
  variable this platform's `dirs::config_dir()` actually reads (Windows: `APPDATA`; verify this
  is really what `dirs::config_dir()` resolves from on this build -- if the `dirs` crate calls a
  raw WinAPI that does NOT respect an overridden `APPDATA` env var for the CURRENT process, this
  precedent does not transfer and you must STOP, see below), would isolate `user_config_path()`
  for a spawned test service the exact same way, with ZERO source changes to `load.rs`.

## STOP preconditions

- Before writing any test that spawns a real service and exercises a REAL successful write, PROVE
  the isolation actually works: spawn a real service with a candidate env override (e.g. `APPDATA`
  pointed at a fresh temp directory) pointing somewhere OBVIOUSLY different from this machine's
  real path, drive a successful `set_user_value`-equivalent write through it, and confirm the
  written file landed INSIDE the temp directory, never in the real path. If it does not (the `dirs`
  crate ignores the env var for this process on this platform), STOP: do not write a test that
  risks touching the real path "just this once" to see if it works, and do not silently skip
  testing the success path. Instead, mark K5 BLOCKED with this exact finding (which env var was
  tried, what `dirs::config_dir()` actually resolved to) so the frontier author can decide between
  (a) adding a genuine, precedented env-override to `load.rs` itself (a small, explicit, NEW
  sanctioned change to a file this task does not currently name) or (b) testing the write action's
  logic without a real spawned-process file write (e.g. a lower-level test of the HTTP handler
  function against an injected write function). Do NOT invent a third option that touches the real
  path even transiently.
- If `set_user_value`'s error variants are not all `crate::Error::Config(String)` (i.e. if some
  other error type can come back), STOP and report the actual variant found rather than assuming
  CS5's uniform "every failure is 409" rule still holds.

## Required behavior

1. Add ONE row to CS1's table: `POST /api/v1/config/webapi-enable-remote`, gated by the SAME
   `channels.webapi.from` decision every other Console route uses. The request body is NEVER read
   or parsed (CS5) -- the handler must not attempt to consume the connection's body at all for
   this route in this batch.
2. On success (`Ok(path)` from `set_user_value(CHANNELS_WEBAPI_FROM, json!(["*"]),
   is_valid_pattern)`): respond `200 OK` with the EXACT JSON shape CS5 pins (`key`, `value`,
   `written_to`, `note` -- `written_to` is the returned path's `Display` string, not itself
   asserted byte-for-byte; the other three fields ARE asserted verbatim), then record ONE
   `config_changed` `SessionEventRecord` via `ctx.recorder.record_session_event(&record)` with
   every field exactly as PINS.md CS4 specifies (`identity: None`, `client: None`, `event:
   "config_changed"`, `manifest: None`).
3. On a locked-key `Err`: respond `409 Conflict` with `{"error": "<the exact lock-refusal message
   string>"}` (CS5's transcribed message). On any OTHER `Err` from `set_user_value`: also `409
   Conflict`, `{"error": "<the exact message>"}` -- no separate 5xx branch in this batch. Record NO
   audit event on any refusal (mirrors `record_manifest_reload`'s own "only on success" rule).
4. Update the Console page (K2's shell) to add the enable-remote control: a button/toggle plus the
   PINNED disclaimer text from CS5, rendered verbatim, visible before the action can be triggered
   (not just in a tooltip nobody reads).

## Tests to write FIRST

Reuse K2's spawn/port-uniqueness helpers; add to `tests/console_static_routes.rs` or a focused new
`tests/console_enable_remote.rs`:

- (Precondition proof, not itself a named pinned test, but must exist and pass before the tests
  below are trusted): confirm the env-override isolation approach described above actually works
  on this platform, per the STOP precondition. If it does not, this task is BLOCKED and none of
  the tests below get written against a real file write.
- `enable_remote_writes_the_pinned_value_and_records_one_config_changed_event`: spawn a real
  service with the proven isolation override AND an audit destination pointed at a test-local file
  (reuse whichever existing pattern `tests/audit_recorder.rs` or the Hub batch's audit tests
  already use to get a real, readable audit JSONL out of a spawned process -- verify the exact
  mechanism before assuming one), POST to `/api/v1/config/webapi-enable-remote` with an empty
  body, assert `200 OK` and the EXACT `key`/`value`/`note` literals from CS5, then read the
  isolated user config file and assert it now contains `"channels.webapi.from": ["*"]`, then read
  the audit file and assert exactly one NEW line with `event == "config_changed"`,
  `identity == null`, `client == null`, `manifest == null`.
- `enable_remote_refuses_cleanly_under_an_org_mandatory_lock`: spawn a real service with an
  org-mandatory lock on `channels.webapi.from` (reuse the SAME org-override mechanism K3's
  locked-key test uses), POST to the same route, assert `409 Conflict` with the EXACT transcribed
  lock-refusal message in the `error` field, and assert the isolated user config file was NOT
  created or modified (no file, or byte-identical to before the POST) and no NEW audit line
  appeared.
- `enable_remote_ignores_the_request_body`: POST a nonsense body (e.g. `{"value":
  ["evil.example.com"]}`) and assert the WRITTEN value is still the PINNED `["*"]`, never the
  caller-supplied one -- this is the single most important test in this task, since it is the
  concrete proof of the "Console never lets an HTTP caller choose an arbitrary value" fence.

## Out of scope

- No "disable remote" route (out of scope per PINS.md CS5; not part of this batch).
- No token mint/revoke of any kind (BOOTSTRAP's fence). If any test or implementation step here
  starts to need a principal/credential concept, STOP.
- No live TCP re-bind of the web API listener on this config change -- CS5's disclaimer text
  ("takes effect the next time the Ghostlight service restarts") is the pinned, accepted
  limitation; do not attempt to rebind the listener from this handler.
