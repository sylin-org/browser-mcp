# H6: Always-ready standalone service + thin-only adapters + anti-squat + idle-grace

> Batch: Ghostlight Hub. Normative: docs/adr/0030-ghostlight-hub-orchestrator.md (Decision 8 AS
> AMENDED 2026-07-04; Decision 1 for the argv role topology and the role marker; Provenance
> "always-ready-service amendment" and "the sacred surface is user DELIGHT"; "Preserved invariants"
> for the fences). Oracle values: docs/tasks/hub/PINS.md SS5 (rewritten) and SS8. One task = one
> commit. Facts below are as-of-authoring 2026-07-04 -- RE-READ the named files before relying on any
> line number.

## Goal

Land the amended ADR-0030 Decision 8 lifecycle. The persistent SERVICE is a STANDALONE process
started by argv (`ghostlight service`); it owns both endpoints and the extension link, multiplexes
adapter sessions, runs NO parent-death watchdog, and shuts down on an idle-grace window. EVERY MCP
invocation is a THIN ADAPTER: it connects, relays, and dies with its editor; if the service is down
it asks the OS supervisor to start it (self-heal) and otherwise reports a clear message. The SERVICE
proves possession of a per-install secret on connect (anti-squat) before relaying. The ADR-0029
parent-death watchdog + reaper are re-scoped to the ADAPTER; the core's proc-identity ROLE is
deleted. There is NO on-demand in-editor spawn and NO job breakaway: that mechanism is DELETED, not
built (amended Decision 8; Windows in-job breakaway is unreliable and was the H6 block).

This task RESHAPES H2's landed election (`run_mcp_server` claim-win-becomes-in-process-service):
after H6, `run_mcp_server` is ALWAYS the adapter and a new `run_service` (the `service` subcommand)
is the standalone service. H2's multiplex, H3's identity, H4's isolation, and H5's queue are
UNCHANGED in substance; only WHO hosts the service and HOW it is reached change.

## Authority

1. docs/adr/0030-ghostlight-hub-orchestrator.md (amended Decision 8; Decision 1; Provenance;
   Preserved invariants) -- NORMATIVE. Cite it; never restate its semantics.
2. docs/tasks/hub/PINS.md SS5 (rewritten) + SS8 -- the pinned oracle values. Transcribe them; never
   derive.
3. BOOTSTRAP.md ground rules.
4. This task file.

If they conflict, the higher wins.

## Current-tree facts (as-of-authoring 2026-07-04; RE-READ before relying)

STANDING ORDER: every line number/signature below is a snapshot. RE-READ the named file first. If a
STOP precondition's assumption is absent, STOP -- do not improvise around a broken assumption.

- `src/hub/mod.rs` (H2-H5 landed): `run_mcp_server(manifest, debug_on)` claims the adapter/control
  endpoint FIRST and, on WIN, calls `run_as_service` (which builds the `Browser`, spawns the
  extension `ipc::serve`, builds the shared `ServiceContext` ONCE, spawns `ipc::serve_adapters`, AND
  serves THIS process's own stdio as the first session via `serve_session` in a `tokio::select!`
  against a parent-death `shutdown`); on LOSE (`Error::SessionBusy`) calls `run_as_adapter`
  (`ipc::relay_adapter`). `run_as_service` currently ALSO spawns
  `transport::watchdog::wait_until_orphaned(parent)`. `ServiceContext` is `#[derive(Clone)]` with
  fields `browser`/`store`/`recorder`/`initial_policy`/`session_registry`/`owned_tabs`/`mint_quota`.
  H6 reshapes this per SS5.1 (delete the election + the service's own stdio session + the service's
  watchdog; `run_mcp_server` becomes the pure adapter; add `run_service`).
- `src/main.rs`: role detection (`chrome-extension://` -> `run_native_host_role`) precedes clap; the
  `Command` enum has Install/Uninstall/Doctor/Status/Config/Policy (NO `Service`); the
  `command: None` arm calls `ghostlight::hub::run_mcp_server(manifest, debug_flag || debug_env)`.
  H6 adds the `Service` subcommand + arm per SS5.1. `run_native_host_role` (the RELAY) stays
  connect-only -- do NOT add any spawn path to it.
- `src/transport/native/ipc.rs`: `default_endpoint()`; `adapter_endpoint_name` (base + `-adapter`);
  `relay_adapter(endpoint, debug)` (mints a GUID, sends the framed hello, then a RAW bidirectional
  relay); `handle_adapter_connection<S>(ctx, stream, peer_cred)` (reads the hello, admits via
  `SessionRegistry`, calls `serve_session`); `serve_adapters(ctx, listener)`;
  `claim_adapter_endpoint(endpoint)`; `capture_peer_cred(...)` per platform; `serve(browser,
  endpoint)` (extension endpoint, UNCHANGED). H6 inserts the anti-squat proof into
  `handle_adapter_connection` (service side) and `relay_adapter` (adapter side), and the self-heal
  dial-retry into `relay_adapter`, per SS5.2/SS5.3.
- `src/hub/handshake.rs`: `HUB_PROTO`, `ROLE_ADAPTER`, `ROLE_CONTROL`. H6 adds
  `ROLE_SERVICE_PROOF = "service-proof"` (SS5.3).
- `src/hub/role.rs` (H3): `Role`, `set_role`, `role`, `assert_role`, `assert_service_role`,
  `assert_adapter_role`. H6 calls `assert_adapter_role("start_service")` (SS5.2/SS8); do NOT redefine
  these.
- `src/transport/mcp/server.rs`: `serve_session<S>(stream, ctx, guid: SessionGuid)` -- its first
  line already `assert_service_role`s; its read loop runs H4's ownership gate and H5's `write_chunked`.
  H6 adds the SS5.4 `live_sessions` increment/decrement guard at entry/exit. Do NOT touch the frozen
  `notifications/tools/list_changed` line.
- `src/debug.rs::log_dir() -> Option<PathBuf>` resolves the PER-USER `dirs::data_local_dir()/ghostlight`
  (`%LOCALAPPDATA%\ghostlight` on Windows). The `hub-key` lives there (SS5.3); RE-READ it; do NOT
  invent a new dir. The debug snapshot carries a `role` field and `extension_connected: bool`, keyed
  by pid in `debug-state-<pid>.json`.
- `src/doctor.rs`: reap targets orphaned `"mcp-server"` sessions today (filters as-of-authoring at
  doctor.rs:561/609, text at :147, doc at :20); the health anchor / display find `"mcp-server"` at
  :86/:465/:481. H6 re-scopes ONLY the REAP filters/text to `"adapter"` per SS5.5; the health anchor
  stays `"mcp-server"` (= the standalone service). `sweep_orphans` stays in the adapter entry.
- `src/proc.rs`, `src/transport/watchdog.rs`: platform liveness + the generic watchdog. H6 updates
  ONLY their module docs ("adapter role", not "mcp-server role"); NO API change; keep every inline
  test green.
- `docs/adr/0029-process-lifecycle-hygiene.md`: AMEND (append a short "Superseded/amended by ADR-0030
  Decision 8 (2026-07-04): the parent-death watchdog + reaper are re-scoped to the ADAPTER; the
  standalone service uses idle-grace" note near the top; keep the historical body).

## Required behavior (cite the ADR; transcribe SS5/SS8 values)

1. Argv role dispatch (Decision 1 amendment; SS5.1). Add the `Service` subcommand; `run_mcp_server`
   becomes the thin ADAPTER (`Role::Adapter`, `"adapter"` debug label, `sweep_orphans`, parent
   capture + watchdog, relay, no policy/Browser/ServiceContext); add `run_service` (`Role::Service`,
   `"mcp-server"` label, loads policy, claims the endpoint as a single-instance guard, owns both
   endpoints + `ServiceContext`, NO parent/watchdog/own-stdio-session, idle-grace).

2. Thin adapter + supervisor self-heal (Decision 8 amendment; SS5.2). The adapter, on a failed first
   dial, calls `supervisor::start_service()` (which `assert_adapter_role`s first) then retries the
   dial within the pinned window; on exhaustion it logs the pinned self-heal message and exits
   non-zero. NEVER spawn an in-job child; NEVER run governance in the adapter. The RELAY
   (`run_native_host_role`) gains NO spawn path (Decision 1: relay only connects).

3. Idle-grace shutdown (Decision 8: "shuts down on an idle-grace window ... never on parent-death";
   SS5.4). `ServiceContext` gains `live_sessions`; `serve_session` counts every session; `run_service`
   exits after `IDLE_GRACE` of continuous (zero sessions AND extension link gone). The service NEVER
   calls `watchdog::wait_until_orphaned` and NEVER captures a parent.

4. Parent-death watchdog + reaper re-scoped to the ADAPTER (Decision 8; SS5.5). The adapter keeps
   `proc::parent()` + `watchdog::wait_until_orphaned` + `sweep_orphans`; the service runs none.
   `doctor::reap` reaps orphaned `"adapter"` sessions, never the service.

5. Anti-squat (Decision 8; SS5.3). Per-install 32-byte `hub-key` at `debug::log_dir()/hub-key`
   (per-user; 0600 on Unix), created on first service start. On connect the service sends a framed
   `service-proof` (HMAC-SHA256 over the adapter's hello bytes); the adapter verifies (constant-time)
   and ABORTS with the pinned text on any mismatch, before relaying. It is transport admission, not a
   denial-id, and a no-op for the wire once verified.

6. Delete the core's proc-identity/liveness ROLE (Decision 8 + Consequences; Decision 4: "the
   governance core gains NO concept of pid / ancestor / creation-time"). The SERVICE gains no
   pid/parent concept. `src/proc.rs` is retained ONLY for the adapter watchdog and the doctor reap
   (do NOT delete `ProcId`/`parent`/`orphaned`/`pid_exists`/`is_alive`/`creation_time`/`terminate`).

MUST stay byte-identical: `TOOLS_JSON`; the native-messaging framing; the MCP JSON-RPC wire +
`notifications/tools/list_changed`; a lone all-open session's CLIENT-VISIBLE output; the a7
core/back-edge boundary (all lifecycle/anti-squat/self-heal code lands in `src/hub` or the binary
shell, NEVER in `src/governance/**`).

## Tests (BY NAME; assertions pinned)

Per the "only delight is sacred" provenance, `tests/peer_death.rs`, `tests/mcp_protocol.rs`, and the
one spawning test in `tests/all_open_golden.rs` are MOVABLE HARNESS at H6: update their spawn
choreography to the standalone-service + thin-adapter topology, PRESERVING every existing assertion
verbatim. `tests/tool_schema_fidelity.rs` and the all-open CLIENT-VISIBLE assertions remain frozen.

- Shared test support (author it; pin the contract). A helper module (e.g. `tests/support/mod.rs`,
  or inline per file) exposing:
  - `fn spawn_service(endpoint: &str) -> Child`: spawns `CARGO_BIN_EXE_ghostlight` with the `service`
    subcommand + `GHOSTLIGHT_ENDPOINT=endpoint` + `GHOSTLIGHT_DEBUG=1` + a unique `GHOSTLIGHT_LOG_DIR`
    (so the hub-key + debug files are test-isolated), stdout/stderr null, and BLOCKS until the
    service's debug snapshot exists / the endpoint accepts a connection (poll up to ~15s). Returns the
    Child (the test kills it in teardown; do NOT wait out idle-grace).
  - `fn spawn_adapter(endpoint: &str) -> Child` with piped stdin/stdout (a bare invocation): the thin
    adapter that relays to the service on `endpoint`.
  Because the service is spawned FIRST and awaited-ready, the adapter's first dial succeeds and the
  self-heal path is never taken (correct: tests must not touch a real OS supervisor).

- `tests/mcp_protocol.rs` (movable harness): change `drive`/`drive_with_manifest` to
  `spawn_service(endpoint)` (for `drive_with_manifest`, pass `--manifest` to the SERVICE, since the
  adapter ignores it -- SS5.1) THEN `spawn_adapter(endpoint)`, write requests to the ADAPTER's stdin,
  read the ADAPTER's stdout, and kill the service in teardown. EVERY assertion stays verbatim
  (`initialize`/`tools/list` == fixture, the 14 tools, the `[hop: extension] Browser extension not
  connected...` exact text, `explain` byte-identical across postures, the invalid-request pre-check,
  jsonrpc rules, the late-extension wait note). The fake extension keeps connecting to the SAME
  `endpoint` (the extension endpoint the SERVICE owns).

- `tests/all_open_golden.rs` (movable harness -- ONLY the spawning test): the two pure/in-process
  tests (`tools_list_is_byte_stable_through_the_move`, `facade_decide_is_all_open_after_the_move`) do
  NOT spawn and are UNCHANGED. `read_page_redaction_is_still_wired_at_the_chokepoint` spawns a bare
  binary today; change it to `spawn_service(endpoint)` + `spawn_adapter(endpoint)`, drive the adapter,
  fake ext on `endpoint`, kill the service in teardown. The redaction assertions
  (`value="[value redacted]"`, no `secret_value=`, no `hunter2`) stay verbatim -- they are the
  invariant.

- `tests/peer_death.rs` (movable harness): its intent is "a native host exits when its real IPC peer
  dies." The real peer is now the SERVICE. Rewrite the scenario: `spawn_service(endpoint)` (the peer),
  spawn the native-host relay (`chrome-extension://...` arg) on the same endpoint, wait for the
  service's `"extension_connected": true` snapshot, KILL THE SERVICE, and assert the native-host exits
  within 5s. Keep the pinned assertions (`connected`, `exited`) verbatim; only the process that is
  spawned-and-killed changes (service, not a bare invocation).

- ADD `tests/hub_lifecycle.rs`:
  - `service_survives_the_spawning_adapter_exit`: `spawn_service(ep)`; `spawn_adapter(ep)`; confirm the
    service is up (endpoint accepts / snapshot present) AND the adapter connected; KILL THE ADAPTER
    and reap it; assert the SERVICE process still reads alive (`ghostlight::proc::pid_exists(service_pid)
    == true`) shortly after (well within `IDLE_GRACE` = 30s; SS5.4). Directly exercises Decision 8:
    the service's lifetime is independent of any client. Kill the service in teardown.
  - `adapter_cannot_complete_handshake_with_an_impostor_service`: stand up an IMPOSTOR listener on the
    adapter/control endpoint (same user) that does NOT hold the real `hub-key` (point it at an empty /
    different `GHOSTLIGHT_LOG_DIR`, or have it send a bogus `service-proof`); run the adapter handshake
    against it; assert the adapter REFUSES past the handshake (no relay) and surfaces the PINNED text
    `refusing to connect: the Ghostlight service on this endpoint is not the one this user installed`
    (SS5.3). Exercises Decision 8 anti-squat.
  - `supervisor_start_asserts_adapter_role` (SS8, text-scan, NOT a live-process test): assert the
    source of `src/hub/supervisor.rs` (resolve via `env!("CARGO_MANIFEST_DIR")` join, like
    `tests/architecture.rs`'s `governance_dir()`) contains the literal substring `assert_adapter_role`.
    Guards the SS8 wiring; `src/hub/role.rs`'s own unit tests (H3) guard the assertion LOGIC.
  - Unit-test `supervisor::supervisor_start_command()` (pure): assert it returns the pinned
    program+args for the current platform (SS5.2), e.g. on Windows `("schtasks", ["/run","/tn","Ghostlight Service"])`.
    NEVER execute the command.

## Verification (literal commands)

```
cargo build --all-targets
cargo test --test hub_lifecycle
cargo test --test peer_death
cargo test --test mcp_protocol
cargo test --test all_open_golden --test tool_schema_fidelity --test architecture
cargo test --lib proc
cargo test --lib watchdog
cargo test --lib -- hub::supervisor
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

Then the FULL `cargo test` must be green.

## STOP preconditions

- If `src/hub/mod.rs` (the H0 composition root) or `src/hub/role.rs` (H3; `assert_adapter_role`) is
  absent, STOP: an earlier task has not landed.
- If `serve_session`'s signature is not `serve_session<S>(stream, ctx, guid: SessionGuid)` or it does
  not already `assert_service_role`, STOP: H3 diverged (SS9/SS8).
- If `ipc::relay_adapter` / `handle_adapter_connection` / `serve_adapters` / `claim_adapter_endpoint`
  are not present under those names in `src/transport/native/ipc.rs`, STOP: H2 diverged (SS1/SS9).
- If `src/debug.rs::log_dir()` does not resolve a per-user data dir, STOP (SS5.3 relies on it; do not
  invent a dir).
- If any AUTHOR-MUST-PIN value is still unpinned in PINS.md SS5 (idle-grace, self-heal window/message,
  supervisor identifiers, anti-squat secret/proof shape, `service-proof` role), STOP: transcribe, do
  not derive.
- If landing this task would require moving a NEVER-touch fence below, STOP.
- NOTE: the old H6 "if breakaway cannot be verified, STOP/BLOCK" precondition is REMOVED -- the amended
  Decision 8 builds no in-job breakaway; the strong guarantee comes from the OS supervisor (H9).

## NEVER touch (this task)

Delight-protecting fences (frozen; the sacred surface is user delight, ADR-0030 Provenance):
- `src/transport/mcp/tools.rs` (`TOOLS_JSON`) and `tests/tool_schema_fidelity.rs` -- byte-frozen. No
  exception, ever (Claude's trained behavior IS the delight).
- `src/transport/native/host.rs` framing (4-byte LE prefix, `MAX_MESSAGE_LEN`, `encode`/`read_message`)
  -- the extension wire. No exception. The anti-squat proof frame RIDES this framing; it does NOT
  change it.
- The MCP JSON-RPC wire + the `notifications/tools/list_changed` line (`server.rs`). The adapter is a
  byte relay, never a rewriter.
- `tests/architecture.rs` a7 (`governance_core_has_no_forbidden_back_edges`): `src/governance/**` names
  no browser/transport/mcp/native/url and no tabId/token/socket type. ALL lifecycle / anti-squat /
  self-heal / supervisor code lands in `src/hub` or the binary shell -- NEVER `src/governance/**`. The
  H8-only `channels.webapi.from` exception does not apply here.
- `Browser::attach` single-EXTENSION-link rejection (`AttachOutcome::AlreadyAttached`). Retained.
- All-open CLIENT-VISIBLE output byte-identity: the new paths are no-ops for a lone all-open session's
  wire bytes.

Movable at H6 (sanctioned; preserve every assertion verbatim, update only the spawn choreography, per
the "only delight is sacred" provenance): `tests/peer_death.rs`, `tests/mcp_protocol.rs`, and the one
spawning test in `tests/all_open_golden.rs`. (The two pure/in-process `all_open_golden.rs` tests are
NOT touched.)

Task-specific fences:
- The SERVICE role must NEVER call `watchdog::wait_until_orphaned` and NEVER capture a parent to die
  with. Its only shutdown trigger is idle-grace.
- Add NO spawn path to `run_native_host_role` (the relay). The relay only connects (Decision 1).
- Do NOT spawn the service elevated / as SYSTEM (Decision 8). Do NOT build any `CREATE_BREAKAWAY_FROM_JOB`
  / `IsProcessInJob` / detached-child-spawn code -- that mechanism is DELETED (amended Decision 8).
- Do NOT delete `ProcId`/`parent`/`orphaned`/`pid_exists`/`is_alive`/`creation_time`/`terminate` from
  `src/proc.rs`: the adapter watchdog + `doctor::reap` depend on them.
