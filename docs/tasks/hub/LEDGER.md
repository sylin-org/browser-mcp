# Ghostlight Hub batch: LEDGER

Durable progress for the Hub batch (ADR-0030). One task = one commit. Update this file at the end of
every task, per BOOTSTRAP step 8. This is the single source of truth for "where are we"; a fresh
executor resumes from RESUME HERE with no other context.

## RESUME HERE

**Next task: H2 (`H2-service-adapter-multiplex.md`).**
H0 landed (pure code move; `src/hub` composition root extracted). H1 landed (transport-generic
`serve_session<S>` + `ServiceContext`, byte-identical single-session refactor). Start at H2,
follow the per-task procedure in `BOOTSTRAP.md`.

## Status

| Task | Title | Status | Commit | Notes |
| --- | --- | --- | --- | --- |
| H0 | Extract the HubCore composition root | DONE | a4e87b6 | |
| H1 | Transport-generic serve_session + ServiceContext | DONE | pending-hash | |
| H2 | Persistent service + thin adapter + multiplex | pending | -- | the one large coupled commit |
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
- (not started)

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
