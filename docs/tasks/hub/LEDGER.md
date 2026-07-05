# Ghostlight Hub batch: LEDGER

Durable progress for the Hub batch (ADR-0030). One task = one commit. Update this file at the end of
every task, per BOOTSTRAP step 8. This is the single source of truth for "where are we"; a fresh
executor resumes from RESUME HERE with no other context.

## RESUME HERE

**Next task: H3 (`H3-session-identity-guid.md`), RE-ISSUED 2026-07-04.** H0 landed (pure code move;
`src/hub` composition root extracted). H1 landed (transport-generic `serve_session<S>` +
`ServiceContext`, byte-identical single-session refactor). H2 landed (persistent SERVICE + thin
ADAPTER + genuine multiplex over the amended two-endpoint design; the kill-hook fan-out; ADR-0004
repealed at the MCP-client layer). H3 previously BLOCKED (see the H3 Log entry) because its own
Required Behavior assumed the ADAPTER/CONTROL accept loop lived in `src/hub/mod.rs`, when H2's
actual landing put it in `src/transport/native/ipc.rs`. The frontier author has since: (1) pinned
the corrected architecture in `docs/tasks/hub/PINS.md` SS9 (the single description H3/H4/H5/H7/H8
now cite) and re-authored H3's Required Behavior, STOP preconditions, and Tests to match; (2)
re-authored H4, H5, H7, and H8 too, since they shared the SAME stale assumption and would have
blocked in turn; (3) run two independent fresh-eyes verification passes against the LIVE, landed H2
code (not just the doc text) and closed 7 further gaps those passes found (missing `Hash`/`Clone`/
`PartialEq` derives on `PeerUser`/`SessionGuid`; a dead `server::run()` that would fail to compile
against the new `serve_session` signature; `relay_adapter`'s placeholder empty guid; H5's
screenshot-chunking location and mechanism; H8's web-session admission model; a malformed/empty-guid
parse-failure path at admission). `serve_session` now takes a plain `guid: SessionGuid` (not
`Option`) for every session, including the service's own lone one -- see PINS.md SS9 for why. The
tree is at the clean H2 baseline. Start H3 fresh against the re-authored task file, PINS.md SS1/SS8/
SS9, following the per-task procedure in `BOOTSTRAP.md`.

## Status

| Task | Title | Status | Commit | Notes |
| --- | --- | --- | --- | --- |
| H0 | Extract the HubCore composition root | DONE | a4e87b6 | |
| H1 | Transport-generic serve_session + ServiceContext | DONE | 4463b07 | |
| H2 | Persistent service + thin adapter + multiplex | DONE | 96a54fb | landed on the RE-ISSUED, two-endpoint-amended task; prior BLOCKED attempt superseded, see Log |
| H3 | Adapter-minted GUID identity + peer-cred binding | pending | -- | RE-ISSUED after PINS.md SS9 fix; prior BLOCKED in Log |
| H4 | Binary-authoritative cross-session tab isolation | pending | -- | |
| H5 | Reconnect grace window + honest bounded queue | pending | -- | orthogonal after H2 |
| H6 | Detached non-admin lifecycle + anti-squat | pending | -- | job-breakaway is the acceptance gate |
| H7 | Tab-group-per-session presentation | pending | -- | crosses the JS boundary |
| H8 | Local web API = TCP; bind per policy | pending | -- | needs H2+H3+H4; the corrected D2/D5 |

Status values: `pending` | `in-progress` | `DONE` | `BLOCKED`.

## Log

One entry per task as it closes (or blocks). Number every deviation from the task file.

### H0
- Verified all as-of-authoring facts in `H0-extract-hubcore.md` against the live tree: `main::run_server`
  (lines 442-547), `build_debug_sink` (lines 552-570, two callers), the `src/lib.rs` alphabetized module
  block, and the referenced `ipc::serve`/`ipc::default_endpoint`/`mcp::server::run`/`doctor::sweep_orphans`/
  `proc::parent`/`watchdog::wait_until_orphaned` signatures. All matched; no STOP precondition fired.
- Created `src/hub/mod.rs` hosting `run_mcp_server` (verbatim `run_server` body) and `build_debug_sink`
  (verbatim body, now `pub`). Added `pub mod hub;` to `src/lib.rs` between `governance` and `install`.
  Updated `src/main.rs`: the `command: None` arm now calls `ghostlight::hub::run_mcp_server`,
  `run_native_host_role` now calls `ghostlight::hub::build_debug_sink`; deleted the old `run_server` and
  `build_debug_sink` functions; narrowed/removed the imports the task named (`Context` narrowed off
  `anyhow::Result`; `browser::pattern`, `debug::DebugSink`, `governance::manifest::source`,
  `transport::executor::Browser` removed; `native::ipc` kept).
- No deviations from the task file. All four verification commands passed for real:
  `cargo build --all-targets`, `cargo test` (423 tests + the sacred/named suites, all ok), `cargo clippy
  --all-targets -- -D warnings` (clean), `cargo fmt --all -- --check` (clean after running `cargo fmt --all`
  once to normalize the new file's import order and a trailing blank line in `main.rs` -- whitespace/import
  ordering only, no semantic change; not logged as a numbered deviation since it does not alter any named
  fact, oracle, or assertion). Sacred tests (`tests/tool_schema_fidelity.rs`, `tests/all_open_golden.rs`,
  `tests/architecture.rs::governance_core_has_no_forbidden_back_edges`) green and byte-unmodified. Only
  `src/lib.rs`, `src/main.rs`, and the new `src/hub/mod.rs` changed; no NEVER-touch fence moved.
- Note: `cargo build`/`test`/`clippy` were run with `CARGO_TARGET_DIR` pointed at a scratch directory
  (not the repo's `target/`) because three live `ghostlight.exe` processes (this environment's own
  dogfooded MCP/native-host session) held the repo's `target/debug/ghostlight.exe` locked on Windows;
  this is a local build-artifact routing choice only, not a source or test change.

### H1
- Verified all as-of-authoring facts in `H1-serve-session-generic.md` against the live tree:
  `mcp::server::run` (lines 108-301, matching the task's line ranges within a few lines),
  `pipeline::handle_tools_call`'s signature (line 50, byte-identical to the task's quote), the
  `src/main.rs` call site (now `ghostlight::hub::run_mcp_server`, which itself calls
  `crate::mcp::server::run(browser, loaded_policy, user_source)` unchanged), and `LoadedPolicy`'s
  `#[derive(Debug, Clone, PartialEq)]`. All matched; no STOP precondition fired.
- D1: the STOP precondition reads "If `src/hub/mod.rs` does not exist or does not host `HubCore`,
  STOP." -> Re-read `H0-extract-hubcore.md` (the higher-priority per-task file for H0) and found
  its own "Required behavior" never mandates a literal Rust type/struct named `HubCore`: it only
  requires `pub fn run_mcp_server(...)` and `pub fn build_debug_sink(...)` inside `src/hub/mod.rs`,
  which is exactly what H0 landed (a4e87b6) and what the live tree contains today. `HubCore` is
  ADR-0030 Decision 2's and this task's own conceptual label for "the module hosting the
  composition root" (the module's doc comment self-identifies as that seam, citing Decision 2 by
  name), not a pinned identifier. Proceeded treating the existing `src/hub/mod.rs` (composition
  root present, doc-commented as the ServiceContext-attachment seam) as satisfying the
  precondition's substantive check, because reading it literally (no file/type may ever be named
  `HubCore`) would make the precondition permanently un-satisfiable even by H0 done correctly to
  its own letter -- which cannot be the intent of a linear, executable batch. Impact on later
  tasks: none functionally; H2/H3/H4/H5/H6/H8 task files use the same "hosts HubCore" phrasing to
  mean this module, and none of their own "Required behavior" sections require a literal `HubCore`
  struct either -- a future executor should read their STOP preconditions the same way (module
  presence + composition-root content, not a literal type name).
- Implemented per the task's exact prescriptions: added `ServiceContext` (fields `browser: Browser,
  store: Arc<ConfigStore>, recorder: Arc<Recorder>, initial_policy: LoadedPolicy`) and
  `ServiceContext::from_startup(browser, loaded_policy, user_source) -> crate::Result<Self>` to
  `src/hub/mod.rs`, moving the shared-lifetime setup (store load -> `spawn_watcher` -> recorder
  build -> recorder-reload subscription spawn), verbatim, out of `server::run`. Added
  `serve_session<S>(stream: S, ctx: ServiceContext) -> Result<()>` to
  `src/transport/mcp/server.rs`, moving the per-session setup (governance build, kill hook, writer
  task now writing to the split `write_half` instead of `tokio::io::stdout()`, policy-subscription
  task, read loop over `BufReader::new(read_half)`, ordered teardown), verbatim except for the
  stdout/stdin substitution the task itself specifies. `run` is now the thin wrapper:
  `ServiceContext::from_startup(...)` + `tokio::io::join(stdin, stdout)` + `serve_session(...)`,
  byte-identical signature, so `src/main.rs` (which calls `hub::run_mcp_server`, itself calling
  `mcp::server::run`) needed no edit.
- D2: mechanical import cleanup forced by the move, not called out by name in the task's "Imports
  today" note -> removed `use crate::governance::audit::Recorder;` and narrowed
  `use crate::browser::{advertise, pattern, polarity};` to `use crate::browser::{advertise,
  polarity};` in `server.rs` (both became unused once `Recorder::from_config` and
  `pattern::is_valid_pattern` moved into `ServiceContext::from_startup`); added `use
  crate::governance::audit::Recorder;`, `use crate::governance::config::reload::ConfigStore;`, `use
  crate::governance::manifest::source::LoadedPolicy;`, and `use std::sync::Arc;` to `src/hub/mod.rs`
  for the new struct/fn. Required for `cargo clippy --all-targets -- -D warnings` to pass (unused
  imports are hard errors under `-D warnings`). Impact on later tasks: none -- purely mechanical,
  covered by "the executor transcribes the mechanical relocation and import re-homing" latitude the
  task file itself grants for H0-style moves.
- OPTIONAL seam test (`serve_session_over_duplex_matches_stdio_initialize_reply`) SKIPPED per the
  task's own instruction ("SKIP it rather than improvise -- it is not required for the commit to be
  complete"); the kept-green suites (`tests/all_open_golden.rs`, `tests/mcp_protocol.rs`) already
  exercise `serve_session` over the real stdin/stdout join.
- All four verification commands passed for real: `cargo build --all-targets`; `cargo test` (423
  lib tests + every named integration suite -- `all_open_golden` 3/3,
  `architecture::governance_core_has_no_forbidden_back_edges` green, `audit_recorder` 2/2,
  `hot_reload` 1/1, `mcp_protocol` 6/6, `tool_schema_fidelity` 7/7, plus every other existing suite,
  all green); `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean
  (no reformatting needed). Sacred tests (`tests/tool_schema_fidelity.rs`,
  `tests/all_open_golden.rs`, `tests/architecture.rs::governance_core_has_no_forbidden_back_edges`)
  green and byte-unmodified (`git diff --stat` shows only `src/hub/mod.rs` and
  `src/transport/mcp/server.rs` changed). No NEVER-touch fence moved.
- Note: as in H0, `CARGO_TARGET_DIR` was pointed at a scratch directory (not the repo's `target/`)
  because a live `ghostlight.exe` held `target/debug/ghostlight.exe` locked on Windows (`Access is
  denied. (os error 5)` on the first build attempt); build-artifact routing only, not a source or
  test change.

### H2
- RE-ISSUED 2026-07-04 (frontier author). The BLOCKED entry below stands as provenance. Resolution:
  the design was AMENDED, not patched. ADR-0030 Decision 1 now specifies TWO local endpoints (a
  hello-free EXTENSION endpoint + an ADAPTER/CONTROL session-hello endpoint) instead of one
  role-demuxed endpoint; `ROLE_EXT` is deleted; the extension endpoint keeps its exact
  server-speaks-first contract so `tests/all_open_golden.rs` and `tests/mcp_protocol.rs` pass
  UNMODIFIED. PINS.md SS1 was rewritten, and H2 + H3 re-authored, to match. The first BLOCKED attempt
  was the golden test doing its job (it faithfully encodes the extension's spoken-to contract), not a
  stale double. Chosen over the three stopgaps the executor listed because those either edited a
  sacred file, invented an unpinned timeout, or bolted a second endpoint on WITHOUT removing the
  role-demux -- the amendment removes the role discriminator entirely (fewer, more meaningful parts).
- BLOCKED. Implemented the task in full (`src/hub/handshake.rs` new; `src/transport/native/ipc.rs`
  split `serve` into `claim_endpoint`/`serve_claimed` + added `relay_adapter` + a shared
  `handle_connection` hello-demux per PINS.md SS1; `src/transport/executor.rs` converted the
  single-consumer kill hook to the `kill_hooks`/`KillHookHandle` fan-out registry per Decision 7;
  `src/transport/mcp/server.rs` swapped `on_session_killed` for `register_session_kill_hook` in
  `serve_session`; `src/hub/mod.rs` rewired `run_mcp_server` to claim-or-adapt; new
  `tests/hub_multiplex.rs` with both named tests passing). `cargo build --all-targets` was clean
  and the new `tests/hub_multiplex.rs` (`two_sessions_route_replies_independently`,
  `one_kill_emits_one_audit_record_per_live_session`) passed. Then the task's own verification
  block (`cargo test --test mcp_protocol --test peer_death --test all_open_golden --test
  tool_schema_fidelity --test audit_recorder --test architecture`) surfaced a real, reproducible
  failure in a file this task's own NEVER-touch fence forbids editing, with no exception:
  `tests/all_open_golden.rs::read_page_redaction_is_still_wired_at_the_chokepoint` failed with
  `"[hop: extension] Browser extension not connected"` instead of succeeding. Root cause, traced
  and confirmed (not a hunch): the task's Required Behavior item 1 (PINS.md SS1) requires `serve`
  / `serve_claimed` to read the hub hello frame FIRST and demux BEFORE dispatching to
  `Browser::attach` (`"ext"`) or `serve_session` (`"adapter"`), with "an unknown or absent role
  fails the connection cleanly." `Browser::is_connected()` only becomes `true` INSIDE `attach()`,
  which under this design cannot run until a hello has been read from the peer. But
  `read_page_redaction_is_still_wired_at_the_chokepoint`'s fake extension (and
  `tests/mcp_protocol.rs::tools_call_waits_for_a_late_extension_and_notes_the_wait`'s, structurally
  identical -- confirmed failing the same way, though that file is only in the softer "Keep green"
  list, not the hard NEVER-touch one) connects via `ipc::connect` and calls
  `host::read_message` BEFORE ever writing anything -- it relies on the PRE-H2 behavior where the
  mcp-server can start writing a queued `tools/call`'s framed `tool_request` to a freshly accepted
  connection the instant `Browser::attach` claims it, with zero bytes required from the peer
  first. Under the hello-first gate, `attach()` never runs (the peer never sends first), so the
  pending `read_page` call's bounded `wait_connected(first_call_wait_ms, default 5000ms --
  src/governance/config/mod.rs `ENGINE_CONNECTION_FIRST_CALL_WAIT_MS`) window elapses with the
  extension never marked connected, and `pipeline::handle_tools_call` (src/transport/mcp/pipeline.rs:206)
  then calls `browser.call()` anyway, which fails fast with the exact "not connected" message
  observed -- matching the test's ~5s runtime exactly. This is a genuine, hello-first vs.
  receive-first-peer deadlock, not a coding mistake: I confirmed it by actually implementing the
  task, running the exact verification commands the task names, and tracing the failure to its
  root cause (both fake-extension tests reproduce it identically). I considered and rejected three
  workarounds because each either touches the NEVER-touch fence or invents an unpinned value: (a)
  editing the two tests' fake-extension helpers to send `{"hub":1,"role":"ext"}` first --
  forbidden for `tests/all_open_golden.rs` ("No exception"); (b) a bounded-timeout peek that
  defaults an as-yet-silent connection to `"ext"` -- contradicts the task's literal "absent role
  fails the connection cleanly" and requires inventing a timeout constant that is not pinned
  anywhere in PINS.md (the ORACLE RULE forbids deriving one); (c) a second, adapter-only endpoint
  so the original endpoint's `"ext"` path needs no hello at all -- deviates from PINS.md SS1's
  explicitly PINNED single-endpoint, hello-demuxed design, which is normative and cited, not mine
  to re-derive. Per BOOTSTRAP's Failure protocol ("a never-touch fence would have to move" /
  "verification cannot go green without violating a rule"), I reverted every H2 working-tree
  change (`git restore` on the four modified files; deleted the two new files) back to the clean
  H1 baseline, re-ran the sacred/named suite there to confirm it is green and byte-unmodified
  (`tests/tool_schema_fidelity.rs`, `tests/all_open_golden.rs`,
  `tests/architecture.rs::governance_core_has_no_forbidden_back_edges`, plus `mcp_protocol`,
  `peer_death`, `audit_recorder` -- all pass), and HALTED without attempting H2 again or any later
  task.
- RESOLVED (see the RE-ISSUED note at the top of this H2 section): the frontier author chose a variant
  of option (iii) -- a full two-endpoint split that DELETES the role-demux entirely, not merely a
  second endpoint bolted beside it. Options (i) and (ii) below were REJECTED (they edit a sacred file
  or invent an unpinned timeout) and the `role:"ext"` strings in them are historical, not live.
- What is needed to proceed (any one, decided by the frontier author, not by this executor):
  (i) amend `tests/all_open_golden.rs` (and likely `tests/mcp_protocol.rs`'s
  `tools_call_waits_for_a_late_extension_and_notes_the_wait`) to send the `{"hub":1,"role":"ext"}`
  hello from their fake-extension harness before their first read, and explicitly lift the
  NEVER-touch fence for that one mechanical accommodation; or (ii) re-pin the hello mechanism with
  an explicit, named, pinned sequencing rule for this exact race (e.g. a pinned bounded timeout
  after which an as-yet-silent connection defaults to `"ext"`, with the exact duration and
  fallback semantics stated in PINS.md so it is transcribed, not invented); or (iii) redesign the
  demux so it does not require a blocking pre-read gate on the shared endpoint (e.g. a second,
  adapter-only endpoint), with PINS.md SS1 and this task file re-authored to match. No deviation
  numbers logged (the implementation matched the task's Required Behavior to the letter; the
  conflict is between two of the task's own requirements, not a tree-fact mismatch this executor
  introduced).

**RE-ISSUED RUN (2026-07-04, DONE).** Verified all as-of-authoring facts in the re-authored
`H2-service-adapter-multiplex.md` and the amended `PINS.md` SS1 against the live tree: `src/hub`
and `HubCore`-equivalent composition root present (H0), `serve_session<S>(stream, ctx)` +
`ServiceContext` present (H1) with `next_id`/`pending` shared `Arc` fields on `Browser` confirmed
at their as-of-authoring locations, `on_session_killed`'s single-consumer replace doc confirmed
still in force, `Browser::attach`'s `AttachOutcome::AlreadyAttached` confirmed unchanged, and no
`run_server` in `main.rs` (H0 already moved it). No STOP precondition fired.

Implemented per the re-authored task + PINS.md SS1's two-endpoint split:
- `src/hub/handshake.rs` (new): `HUB_PROTO = 1`, `ROLE_ADAPTER = "adapter"`, `ROLE_CONTROL =
  "control"` -- no `ROLE_EXT`, per the amendment.
- `src/transport/native/ipc.rs`: added `adapter_endpoint_name` (base name + literal `-adapter`
  suffix, wrapped by the same `pipe_path`/`socket_path` helper); `AdapterListener` (cfg-split type
  alias, no unified `Listener` type); `claim_adapter_endpoint` (cfg-split, same bind-with-stale-heal
  `serve` already does, PINS.md SS1 pin 1); `serve_adapters(ctx, listener)` (accept-ahead +
  spawn-per-connection on the ALREADY-claimed listener, never re-claiming the name); the shared
  `handle_adapter_connection` (reads the framed hello INSIDE the spawned task via
  `host::read_message`, demuxes `"adapter"` into `transport::mcp::server::serve_session`,
  `"control"` cleanly refused, unknown/absent role refused, never a panic); `relay_adapter`
  (dials the adapter/control endpoint, sends the framed `{"hub":1,"role":"adapter","guid":""}`
  hello, then a RAW `tokio::io::copy` bidirectional relay -- PINS.md SS1 pin 3 -- mirroring
  `relay_native_host`'s lifecycle shape only, never its framing). The EXTENSION endpoint's `serve`,
  `connect`, `relay_native_host`, and every fake-extension test double are byte-for-byte unchanged.
- `src/transport/executor.rs` (the one sanctioned executor change, ADR-0030 Decision 7): replaced
  the single `kill_hook: Arc<Mutex<Option<KillHook>>>` with a `kill_hooks: Arc<Mutex<Vec<(u64,
  KillHook)>>>` fan-out registry plus `next_hook_id`; `on_session_killed` now APPENDS a permanent
  hook (doc comment updated from "replaces the first" to append semantics); added
  `register_session_kill_hook` returning a `#[must_use]` `KillHookHandle` whose `Drop` removes
  exactly its own entry; `handle_session_killed` now invokes every registered hook once per
  false->true transition. `Browser::attach`'s single-physical-link rejection is untouched.
- `src/transport/mcp/server.rs`: `serve_session`'s kill-hook registration swapped from
  `on_session_killed` to `register_session_kill_hook`, held as `_kill_handle` for the whole
  function body (session-scoped, deregisters on session end; `hold`/`killed`/`connected` stay
  global on the one shared `Browser`).
- `src/hub/mod.rs`: `ServiceContext` now `#[derive(Clone)]` (PINS.md SS1 pin 4; built ONCE via
  `from_startup`, cloned per session, never re-run per session). `run_mcp_server` now calls
  `ipc::claim_adapter_endpoint` FIRST; on win, `run_as_service` builds the `Browser`, spawns the
  UNCHANGED extension `ipc::serve`, builds the shared `ServiceContext` once, spawns
  `ipc::serve_adapters` over the already-claimed listener, and serves this process's own stdio as
  the first session over the shared context (byte-identical lone-client extension path); on loss
  (`Error::SessionBusy` from the adapter/control claim), `run_as_adapter` runs
  `ipc::relay_adapter` instead of the old reject-2nd degrade-and-continue arm, which no longer
  exists in this path (the loser never reaches the extension `serve` call at all).
- `tests/hub_multiplex.rs` (new): `two_sessions_route_replies_independently` (two `Browser::call`
  callers standing in for two sessions, per the task's own sanctioned lower-level alternative,
  share one `Browser`/one fake extension; asserts neither ever receives the other's reply);
  `one_kill_emits_one_audit_record_per_live_session` (three all-open `Governance`s with distinct
  client names, three file-backed `Recorder`s, one shared `Browser`; asserts exactly 3
  `session_killed` records, each with the 6-key `SessionEventRecord` order transcribed verbatim
  from ADR-0030's pinned oracle, each `client.name` matching its own session);
  `adapter_endpoint_two_phase_wire_round_trips` (spawns the real binary, connects to
  `<endpoint>-adapter` via `ipc::connect`, sends the framed hello then a RAW newline JSON-RPC
  `initialize` line, asserts a RAW newline-delimited reply with `id == 1` comes back -- fencing the
  PINS.md SS1 pin 3 framing trap).

D1: PINS.md SS1's "Pinned name: `ipc::relay_adapter(endpoint: &str, debug: &crate::debug::DebugSink)
    -> Result<()>` (the `endpoint` passed is the ADAPTER/CONTROL endpoint, not the extension
    endpoint)" -> implemented `relay_adapter` to take the SAME plain BASE endpoint every other
    call site threads (`ipc::default_endpoint()`), computing the `-adapter` suffix internally via
    the same `adapter_endpoint_name()` helper `claim_adapter_endpoint`/`serve_adapters` use, rather
    than requiring the caller to pre-suffix the argument -- because PINS.md SS1's own naming pin
    ("wrapped by the SAME `pipe_path`/socket-path helper") centralizes the derivation in one place,
    and every sibling adapter/control function already takes the base endpoint and suffixes
    internally; making `relay_adapter` alone expect a pre-suffixed argument would be an
    inconsistent, easy-to-misuse convention that no pinned test distinguishes from this reading
    either way (the resulting wire bytes and endpoint paths are identical). Impact on later tasks:
    none -- H6's spawn-on-demand call site should keep passing the plain base endpoint to
    `relay_adapter`, exactly as H2's own `run_as_adapter` does.
D2: the task's prose names the new acceptor `ipc::serve_adapters(ctx, listener)` (two arguments)
    -> implemented exactly that two-argument signature on both platforms (an earlier draft added a
    third `endpoint: &str` parameter so the Windows accept-ahead loop could re-create pipe
    instances, then was simplified to re-derive the same path via `default_endpoint()` internally,
    since that is already the single source of truth for the process's one endpoint name) --
    because the task's own text is closer to a two-argument shape than the explicitly-labeled
    "Pinned name:" bullet is for `relay_adapter`, and re-deriving avoids threading an extra
    parameter through every call site for no behavioral difference. Impact on later tasks: none --
    H6/H8 call sites should keep calling `ipc::serve_adapters(ctx, listener)` with no endpoint
    argument.

Verification: all four commands passed for real. `cargo build --all-targets` clean.
`cargo test --test hub_multiplex --test mcp_protocol --test peer_death --test all_open_golden
--test tool_schema_fidelity --test audit_recorder --test architecture` all green (26 tests across
the seven suites); `cargo test -p ghostlight --lib executor` green (17/17, including
`kill_hook_fires_exactly_once_per_transition` and
`a_second_attach_is_rejected_without_disturbing_the_live_session`); the full `cargo test` is green
(423 lib tests + every integration suite, 0 failed). `cargo clippy --all-targets -- -D warnings`
clean. `cargo fmt --all -- --check` clean (after running `cargo fmt --all` twice to normalize
wrapping introduced by the edits and by the D2 simplification -- whitespace only, no semantic
change, not logged as its own numbered deviation). Sacred tests
(`tests/tool_schema_fidelity.rs`, `tests/all_open_golden.rs`,
`tests/architecture.rs::governance_core_has_no_forbidden_back_edges`) green and byte-unmodified;
`git diff --stat` shows only `src/hub/mod.rs`, `src/transport/executor.rs`,
`src/transport/mcp/server.rs`, `src/transport/native/ipc.rs` modified plus the two new files
(`src/hub/handshake.rs`, `tests/hub_multiplex.rs`). No NEVER-touch fence moved; the sanctioned
kill-hook-fan-out exception to the executor fence, and the sanctioned two-endpoint-split scoping of
the extension fence, are the only fences touched, both as pinned.
- Note: as in H0/H1, `CARGO_TARGET_DIR` was pointed at a scratch directory (not the repo's
  `target/`) because three live `ghostlight.exe` processes (this environment's own dogfooded
  MCP/native-host session) held the repo's `target/debug/ghostlight.exe`; build-artifact routing
  only, not a source or test change.

### H3
- RE-ISSUED 2026-07-04 (frontier author). The BLOCKED entry below stands as provenance. Resolution:
  PINS.md SS9 pins the corrected architecture (accept/admission in `ipc.rs`, not `src/hub/mod.rs`;
  `ServiceContext` gains `session_registry`/`owned_tabs`/quota fields as siblings; `serve_session`
  gains a plain `guid: SessionGuid`, not `Option`). H3, H4, H5, H7, H8 were all re-authored to match
  (H4/H5/H7/H8 shared the exact same stale assumption and would have blocked in turn). Two further
  fresh-eyes passes against the live H2 code closed 7 more gaps (derives, dead code, the
  `relay_adapter` placeholder guid, H5's chunking mechanism, H8's admission model, a guid
  parse-failure path). See commits `9402312`-adjacent amendment history and `18746aa` for the full
  fix. Chosen over re-deriving each task's location independently, to guarantee cross-file pin
  agreement rather than risk 4 more independently-worded (and possibly inconsistent) corrections.
- BLOCKED at the per-task procedure's step 2 (RE-READ every source file the task names; verify
  each as-of-authoring fact), before writing any test or implementation code -- no working-tree
  changes exist to revert.
- Re-read the task's "Current-tree facts" bullet 1 verbatim: "`src/hub/` is created by H0-H2 (the
  composition root + `ServiceContext` + per-session state + `serve_session<S>(stream, ctx)` + the
  multiplex accept loop) ... this task adds a `guid` field to H2's per-session record and hooks the
  accept path; it does NOT invent the session record." And Required Behavior item 2: "The real OS
  capture (Windows `GetNamedPipeClientProcessId` + token SID; Unix `SO_PEERCRED` / `getpeereid`)
  happens in the accept path in `src/hub/mod.rs` on the raw pipe/UDS handle H2 already owns." And
  item 3 ("Service routing"): "In `src/hub/mod.rs`, after H2's handshake reads the presented GUID
  and the accept layer captures the `PeerCred`, call `SessionRegistry::admit` ... On `Admitted`,
  key the H2 per-session record (its `Governance` facade + owned-handle set) by the GUID's
  canonical string."
- Verified against the live tree: `src/hub/` currently contains exactly two files, `handshake.rs`
  and `mod.rs` (confirmed via directory listing). `src/hub/mod.rs` (H2's actual landed shape) holds
  only `run_mcp_server`, `run_as_service`, `run_as_adapter`, `build_debug_sink`, and
  `ServiceContext` -- `run_as_service` never itself loops over connections or touches a raw
  platform handle; it builds the `Browser`/`ServiceContext` once and SPAWNS
  `ipc::serve_adapters(ctx, adapter_listener)`, a function living in
  `src/transport/native/ipc.rs`, not `src/hub`. The actual ADAPTER/CONTROL accept loop, the
  session-hello read, and the concrete platform types (`AdapterListener` = `NamedPipeServer` on
  Windows / `UnixListener` + the accepted `UnixStream` on Unix) live entirely inside
  `src/transport/native/ipc.rs`'s `serve_adapters`/`handle_adapter_connection`; by the time
  `handle_adapter_connection` runs, the stream is already type-erased to a generic
  `S: AsyncRead + AsyncWrite + Send + Unpin + 'static` (its own signature) -- the concrete OS
  handle (`GetNamedPipeClientProcessId`/`SO_PEERCRED`-capable) is only reachable at the call sites
  inside `serve_adapters`, before that erasure, never from anything `src/hub` owns. There is also
  NO per-session record type anywhere in the tree (grepped `SessionGuid|PeerCred|owned_handle|
  SessionRecord` across all of `src/`: zero matches) holding "the `Governance` facade + owned-handle
  set" for item 3 to key by GUID -- `serve_session` (`src/transport/mcp/server.rs`) builds its
  per-session `Arc<Mutex<Arc<Governance>>>` as a local variable inside the function body, not as a
  `src/hub`-owned record H3 could add a `guid` field to.
- STOP precondition triggered (transcribed verbatim from `H3-session-identity-guid.md`): "If the
  accept layer in `src/hub` has NO access to the connecting peer's raw pipe/UDS handle to read its
  OS credential, STOP -- the peer-cred capture seam belongs to the transport/accept in `src/hub`;
  build it there, never by reaching into `src/governance`." Per BOOTSTRAP's authority order item 4
  ("If the tree contradicts a task's load-bearing assumption, follow that task's STOP precondition;
  do NOT improvise around it") and the per-task procedure's "keep the change inside the files the
  task names", satisfying Required Behavior items 2/3 as written would require either editing
  `src/transport/native/ipc.rs` (a file this task does not name, and the only file where the
  concrete platform handle and the real accept loop actually exist) or inventing a per-session
  record type the task explicitly says H3 must not invent ("it does NOT invent the session
  record"). Neither is a sanctioned move under this task's scope, so no code was written and
  nothing was reverted.
- What is needed to proceed (decided by the frontier author, not by this executor): re-author H3
  (or insert a small intermediate step) to reconcile with H2's ACTUAL landed shape -- either (i)
  name `src/transport/native/ipc.rs` as an in-scope file for the peer-credential capture (e.g.
  capture `PeerCred` inside `serve_adapters` immediately after `listener.accept()` /
  `server.connect()`, where the concrete `UnixStream`/`NamedPipeServer` handle is still live,
  thread it into `handle_adapter_connection`, and call `SessionRegistry::admit` there before
  dispatching to `transport::mcp::server::serve_session`); or (ii) explicitly defer the live wiring
  (item 2's OS-capture code and item 3's routing/keying) to whichever task first introduces a
  per-session record type (H4, which builds the owned-handle set), re-scoping H3 itself to the
  pure `SessionGuid`/`PeerCred`/`SessionRegistry` types plus the role marker (item 6) and the a7
  scanner extension (item 5) -- with PINS.md and the task file updated to say so explicitly, so a
  future executor does not re-hit this same STOP. No deviation numbers logged: this is a tree-fact
  mismatch in the task's own authoring assumptions (H2 was re-authored 2026-07-04 for the
  two-endpoint split after H3 was first drafted), not a choice this executor made.

### H4
- (not started)

### H5
- (not started)

### H6
- (not started)

### H7
- (not started)

### H8
- (not started)

## Deviation format

When you deviate from a task file (a signature differs from as-of-authoring, a helper had to move,
an oracle needed pinning), record it under that task as:

```
D<n>: <what the task said> -> <what you actually did> because <the tree fact that forced it>.
     Impact on later tasks: <none | names the task + what it must now assume>.
```

A BLOCKED entry records instead: the failed assumption (with the file/symbol actually found), the
STOP precondition or fence that triggered, and what is needed to proceed. Then HALT.
