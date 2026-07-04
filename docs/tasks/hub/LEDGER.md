# Ghostlight Hub batch: LEDGER

Durable progress for the Hub batch (ADR-0030). One task = one commit. Update this file at the end of
every task, per BOOTSTRAP step 8. This is the single source of truth for "where are we"; a fresh
executor resumes from RESUME HERE with no other context.

## RESUME HERE

**Next task: H2 (`H2-service-adapter-multiplex.md`), RE-ISSUED 2026-07-04.** H0 landed (pure code
move; `src/hub` composition root extracted). H1 landed (transport-generic `serve_session<S>` +
`ServiceContext`, byte-identical single-session refactor). H2 previously BLOCKED (see the H2 Log
entry) on a hello-first-vs-server-speaks-first deadlock; the frontier author has since AMENDED the
design (ADR-0030 Decision 1 two-endpoint split; PINS.md SS1 rewritten; H2 + H3 re-authored;
`ROLE_EXT` deleted; the extension endpoint stays hello-free and its tests pass UNMODIFIED). The tree
is at the clean H1 baseline. Start H2 fresh against the re-authored task file and amended PINS.md
SS1, following the per-task procedure in `BOOTSTRAP.md`.

## Status

| Task | Title | Status | Commit | Notes |
| --- | --- | --- | --- | --- |
| H0 | Extract the HubCore composition root | DONE | a4e87b6 | |
| H1 | Transport-generic serve_session + ServiceContext | DONE | 4463b07 | |
| H2 | Persistent service + thin adapter + multiplex | pending | -- | RE-ISSUED after 2026-07-04 two-endpoint amendment; prior BLOCKED in Log |
| H3 | Adapter-minted GUID identity + peer-cred binding | pending | -- | |
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

### H3
- (not started)

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
