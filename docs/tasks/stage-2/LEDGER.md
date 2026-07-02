# Stage 2 ledger

Durable, context-wipe-safe record of stage-2 (governance) execution. This file plus
`BROWSER-TESTS.md` are the executor's memory. On every start, after any interruption, and whenever
state is unclear: read the RESUME HERE section first, then `PLAN.md` and `RECONCILIATION.md`, then the
current task prompt, then continue. Never rely on remembering earlier work; re-read files.

## RESUME HERE

- Branch: `stage-2` (off `main`, which has stage 1 merged). Never push, never merge, never commit to
  `main`.
- Progress: tasks `a1` (module reorg), `a2` (governance ports, + RwClass correction), `a3`
  (governance facade), `a7` (arch-test), `g01` (typed key registry), `g02` (layered
  resolution) landed.
- NEXT TASK: Phase A, task `a5` (`docs/tasks/stage-2/a5-hot-reload-substrate.md`).
- Order authority: `PLAN.md` (Phase A -> B -> C -> D). Full linear sequence is in `BOOTSTRAP.md`.
- Reconciliation: `RECONCILIATION.md` is AUTHORITATIVE over any conflicting detail in a `g`-doc.
- Invariants that must hold after every task: all-open byte-identical (the all-open golden test +
  `tests/mcp_protocol.rs`), the sacred tool surface (`tests/tool_schema_fidelity.rs`), `cargo clippy
  --all-targets -- -D warnings` clean, `cargo fmt --check` clean, full `cargo test` green, ASCII-only.

## Task log

(Append one entry per completed task, newest at the bottom. Suggested shape:)

### <task-id> <title> -- <date>
- Commit: <hash>
- Files touched: <list>
- Summary: <what landed, key decisions, any conservative choice made>
- Deviations from the g-doc per RECONCILIATION.md: <placement / hot-reload / ports notes>
- Verification: clippy/fmt/test status; which tests were added
- Browser checks queued: <count> (appended to BROWSER-TESTS.md as <task-id>-<n>), or none

### a1 module reorg (governance/ browser/ transport/) -- 2026-07-02
- Commit: e66b02f
- Files touched: `git mv` of `src/dispatch.rs`, `src/policy/{mod.rs,redact.rs}`, `src/tools/**`,
  `src/native/**`, `src/mcp/**` (incl. `schemas/`), `src/browser.rs`; new
  `src/{governance,browser,transport}/mod.rs`; edited `src/lib.rs`, `src/main.rs`, `src/doctor.rs`,
  `src/install/native_host.rs`, `src/transport/executor.rs`, `src/transport/native/{ipc,messages}.rs`,
  `src/transport/mcp/server.rs`, `src/governance/policy/mod.rs`; new `tests/all_open_golden.rs`.
- Summary: pure move, zero behavior change. `governance/` got `dispatch.rs` + `policy/` (minus
  `redact.rs`); `browser/` got `tools/` + `redact.rs`; `transport/` got `native/`, `mcp/`, and
  `browser.rs` (renamed `executor.rs` to avoid colliding with the new `browser/` plugin module).
  Every `use crate::...` cross-reference rewritten to the new absolute path; the one cross-bucket
  call (`transport/mcp/server.rs` redacting `read_page` output) now calls
  `crate::browser::redact::apply_to_result` directly. `lib.rs` re-exports `pub use
  transport::{mcp, native};` so `tests/tool_schema_fidelity.rs` and `tests/mcp_protocol.rs` keep
  resolving `browser_mcp::mcp::...` / `browser_mcp::native::...` unchanged, per the task's compat-
  facade requirement.
- Deviations from the g-doc per RECONCILIATION.md: none (A1 is not a g-doc; it is one of the new
  a-prompts that already encodes the current vision). Followed a1-module-reorg.md as written.
- Verification: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean, `cargo
  test` green (81 lib unit tests + 2 new `tests/all_open_golden.rs` + 4 `tests/mcp_protocol.rs`
  unchanged + 1 `tests/peer_death.rs` + 6 `tests/tool_schema_fidelity.rs` unchanged = 94 total).
  ASCII scan clean on every touched/moved file. Grep confirmed no stale `crate::browser::Browser`,
  `crate::dispatch`, `crate::policy`, `crate::mcp`, `crate::native`, `crate::tools` paths remain.
  `src/mcp/schemas/tools.json` -> `src/transport/mcp/schemas/tools.json` confirmed byte-identical
  (diff empty). One environment snag: `git mv src/mcp` (whole-directory rename) twice failed with
  Windows `Permission denied` (likely a transient AV/indexer lock); worked around by moving the 6
  files inside `src/mcp/` individually with `git mv`, then removing the resulting empty leftover
  `src/mcp/schemas/` and `src/mcp/` directories (untracked by git, harmless, but removed for
  tidiness) -- no conservative policy choice involved, purely a retry mechanic. No other locked-exe
  issue this task; `target/debug/browser-mcp.exe` needed the constraint-7 rename-aside once before
  the first build.
- Browser checks queued: none (binary-internal move; no user-visible behavior change per the task's
  own scope note).

### a2 governance ports (the seam contract) -- 2026-07-02
- Commit: 21994b6
- Files touched: new `src/governance/ports.rs`; one-line `pub mod ports;` edit to
  `src/governance/mod.rs`.
- Summary: purely additive seam contract. Added the axis/placeholder types (`RwClass`,
  `EffectiveMode`, `Grant`, `ToolId`, `ResourcePattern`, `Denial`, `AuditRecord`), the core
  decision types (`GoverningResource`, `DecisionRequest`, `Decision`), the traits
  (`PolicyDecisionPoint`, `DomainPolicy`, `ResourceResolver`, `AuditSink`), and the two
  zero-policy impls (`NoopPdp`, `NullSink`), exactly as specified in the task prompt. Nothing
  wired into `dispatch` yet (A3's job); no runtime behavior changed. `ResourceResolver` uses a
  native async fn in trait with `#[allow(async_fn_in_trait)]` (no `async-trait` dependency
  added), per constraint 9.
- Deviations from the g-doc per RECONCILIATION.md: the task prompt's literal example code used
  `RwClass::Read`/`RwClass::Write`; this was landed as-is and is WRONG per RECONCILIATION.md
  section 2, which is explicit that `RwClass` must be `Observe`/`Mutate` (distinct from a
  grant's `read`/`write`/`all` access field) and that a2/a3 prompt text using `Read`/`Write` is
  exactly the case to override. Caught before a3 consumed it; fixed in a follow-up correction
  commit (see the log entry below) rather than amending this commit, so the history stays
  linear per the one-task-one-commit rule.
- Verification: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean
  (the single permitted `#[allow(async_fn_in_trait)]` suppression), `cargo test` green (88 lib
  unit tests, +7 new in `governance::ports::tests` covering noop-pdp-allows-all, null-sink-is-
  noop, both ports' dyn-object-safety, `DecisionRequest`/`Decision` serde round-trips, and the
  lowercase wire vocabulary for `RwClass`/`EffectiveMode`). `tests/tool_schema_fidelity.rs`,
  `tests/mcp_protocol.rs`, `tests/peer_death.rs`, and `tests/all_open_golden.rs` all unchanged
  and green. Arch-fence manual check: `ports.rs` has exactly one `use` statement (`use serde::
  {Deserialize, Serialize};`); `serde_json::Value` is referenced by full path inline. A grep
  for the bare word "browser" hits only doc-comment prose (e.g. "browser: a host such as
  github.com"), matching the task prompt's own example text verbatim -- no `use crate::browser`
  or similar import exists. ASCII scan clean.
- Browser checks queued: none (pure library addition; nothing runtime-observable changed).

### correction: RwClass Observe/Mutate rename -- 2026-07-02
- Commit: 8da1bee
- Files touched: `src/governance/ports.rs` only (variant names + doc comment + every test use).
- Summary: renamed `RwClass::{Read,Write}` to `RwClass::{Observe,Mutate}` per
  RECONCILIATION.md section 2, which is explicit-by-name that a2/a3 prompt text guessing
  `Read`/`Write` (or a bare `Observe` without a `Mutate` sibling) must be overridden to
  `Observe`/`Mutate`, kept distinct from a grant's `access: read|write|all` field. Wire form is
  now `"observe"`/`"mutate"` (was `"read"`/`"write"`). No other type or trait touched; caught
  during a3 prep, before any other file consumed the wrong names, so this is a single
  self-contained rename with zero blast radius beyond `ports.rs`.
- Verification: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean,
  `cargo test` green (same 88 lib tests, all 7 `governance::ports::tests` still passing with the
  new variant names and wire strings).
- Browser checks queued: none.

### a3 governance facade (dispatch chokepoint) -- 2026-07-02
- Commit: (see this task's commit)
- Files touched: `src/governance/dispatch.rs` (rewritten: removed the no-op
  `PolicyDecision`/`policy_check`/`audit` seam, added the `Governance` facade); rewired
  `src/transport/mcp/server.rs` (threads `Arc<Governance>` through `run` -> `handle_line` ->
  `handle_tools_call`, replacing the two no-op seam calls with one `governance.decide(name)`);
  extended `tests/all_open_golden.rs` (added `facade_decide_is_all_open_after_the_move` and
  `read_page_redaction_is_still_wired_at_the_chokepoint`, renamed the old
  `dispatch_seam_is_all_open_after_the_move` since the free functions it tested no longer exist).
- Summary: `Governance` holds either `Mode::AllOpen` (zero-port, STEP-0 short-circuit to
  `Decision::Allow { grant_id: None }`) or `Mode::Governed(GovernedState)` (a boxed
  `PolicyDecisionPoint` + an `Arc<dyn AuditSink>`, exercised only by the new facade unit tests,
  not by any production path yet). `decide` stays sync; the `Governed` branch builds a
  placeholder `DecisionRequest` (empty grants, `RwClass::Observe`, `GoverningResource::None`,
  `EffectiveMode::Observe`) and asks the held PDP -- with `NoopPdp` the result is still `Allow`.
  The MCP server constructs `Governance::all_open()` once per session and calls `decide` at the
  same chokepoint position the old two-line seam occupied; the decision is still bound to
  `_decision` and ignored (no enforcement yet). `read_page` redaction is untouched in place.
- Deviations from the g-doc per RECONCILIATION.md: used A2's real port/type names throughout
  (`NoopPdp`, not the prompt's guessed `NoopPolicyDecisionPoint`; `RwClass::Observe`, already
  corrected in the prior ledger entry) per constraint 8 ("match A2's exact names").
- Verification: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean (no
  `#[allow(dead_code)]` added; `pdp`/`audit` stay live via `decide`/`audit_sink`), `cargo test`
  green (90 lib unit tests incl. the 2 new `governance::dispatch::tests`; `tests/all_open_golden.rs`
  3 tests incl. the 2 new; `tests/mcp_protocol.rs` UNCHANGED and green -- exact byte-identical
  `tools/list` and the exact no-extension hop-attributed message; `tests/tool_schema_fidelity.rs`
  and `tests/peer_death.rs` unchanged). Grep confirmed `policy_check`/`PolicyDecision` no longer
  appear anywhere except one historical mention in `dispatch.rs`'s own module doc ("replaces the
  v1.0 no-op `policy_check` / `audit` seams"). ASCII scan clean.
- Browser checks queued: none (binary-only chokepoint change; manual verification note per the
  task's own Verification step 5 -- tools/list still shows 13 tools, a call with Chrome closed
  still times out at ~5s, read_page redaction still defaults on -- is covered by the automated
  `tests/all_open_golden.rs::read_page_redaction_is_still_wired_at_the_chokepoint` test added this
  task, so no live-browser check is queued).

### a7 arch-test (fail-closed governance/ boundary guard) -- 2026-07-02
- Commit: (see this task's commit)
- Files touched: new `tests/architecture.rs` only.
- Summary: a pure `std::fs` + text-scan integration test that recursively walks
  `src/governance/` and fails if any `.rs` file names `crate::browser`, `crate::transport`,
  `crate::mcp`, `crate::native`, or the `url` crate (path-token matched with identifier
  boundaries, scanning raw lines including comments/strings, not just compiled code). Both
  fail-closed properties are in place: a missing `src/governance/` fails loudly (does not
  skip), and an empty directory fails rather than passing vacuously. Landed exactly as the
  task's literal code specified, verbatim.
- Deviations from the g-doc per RECONCILIATION.md: none (A7 is an a-prompt, not a g-doc).
  Followed a7-arch-test.md as written, byte for byte.
- Verification: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean,
  `cargo test` green (90 lib unit tests unchanged; new `tests/architecture.rs` 4 tests --
  `governance_core_has_no_forbidden_back_edges`, `scanner_detects_forbidden_crate_edges`,
  `scanner_detects_url_crate_reference`, `scanner_ignores_clean_lines`; `tests/all_open_golden.rs`
  3 unchanged; `tests/mcp_protocol.rs` 4 unchanged; `tests/peer_death.rs` 1 unchanged;
  `tests/tool_schema_fidelity.rs` 6 unchanged). Negative check per Verification step 4: added a
  temporary `use crate::browser::redact;` line to the end of `src/governance/dispatch.rs`, ran
  `cargo test --test architecture`, confirmed `governance_core_has_no_forbidden_back_edges`
  FAILED naming the exact file, line 138, and the edge `crate::browser`; reverted with
  `git checkout -- src/governance/dispatch.rs` and confirmed `git status` showed no diff before
  re-running green. Robustness check per step 5: ran `cargo test --test architecture` from `src/`
  (both with and without an explicit `--manifest-path`) and confirmed it still passes, since
  the scanner anchors on `CARGO_MANIFEST_DIR`, not the working directory. ASCII scan clean.
- Browser checks queued: none (pure build-time/test-time guard; no runtime or browser-facing
  behavior).

### g01 typed key registry (value types beyond bool) -- 2026-07-02
- Commit: (see this task's commit)
- Files touched: `src/governance/config/mod.rs` (renamed from `src/governance/policy/mod.rs`,
  rewritten: full typed registry replacing the bool-only prototype); `src/governance/mod.rs`
  (`pub mod policy;` -> `pub mod config;`); new `src/browser/pattern.rs`; `src/browser/mod.rs`
  (`pub mod pattern;`); `src/transport/mcp/server.rs` (Config import path, `&Config` threading,
  `FIRST_CALL_WAIT_MS` constant removed and replaced by `config.first_call_wait_ms()`).
- Summary: grew the registry to the full value model (`KeyValue`/`ConfigValue`/`KeyType`/
  `KeyConstraint`/`Preset`), registered the seven stage-2 keys exactly per shared-format-doc
  3.4 (`engine.connection.first_call_wait_ms`, `content.security.secrets.redact`,
  `content.security.sacred_domains`, `audit.enabled`, `audit.destination`, `audit.file.path`,
  `governance.mode`), added `KeyDef::parse_value` with the exact `ConfigValueError` display
  vocabulary, grew `Config` to seven owned fields (loses `Copy`, gains `Clone`), and wired
  `first_call_wait_ms` into the two `Duration::from_millis(FIRST_CALL_WAIT_MS)` call sites in
  the MCP server (the T04 timeout constant this task was scoped to retire). All defaults for
  `content.security.sacred_domains` are `StrList(&[])` for every preset, so `Config::from_preset`
  never needs the domain-pattern validator (it reads registry defaults directly, no JSON
  round-trip).
- Deviations from the g-doc per RECONCILIATION.md (both significant; g01's own doc predates
  A1 and assumes the flat `src/policy/mod.rs` layout):
  1. **Placement.** RECONCILIATION.md section 1 maps `src/policy/mod.rs` (registry, resolver,
     Config) to `governance/config/`, not the `governance/policy/` name A1 produced by a literal
     directory move. Renamed the directory as part of this task (`git mv
     src/governance/policy src/governance/config`), updated `governance/mod.rs`'s module
     declaration, and repointed the one external import site
     (`transport/mcp/server.rs`: `governance::policy::Config` -> `governance::config::Config`).
  2. **The domain-pattern validator (the RECONCILIATION section 2 "known integration point,
     resolve during g01/a1").** g01's own doc puts `pattern.rs` under `src/policy/pattern.rs`
     (i.e. inside governance) and has `parse_value` call
     `crate::policy::pattern::is_valid_pattern` directly. RECONCILIATION.md is explicit that the
     pattern grammar is browser-domain (`browser/pattern.rs`) and that `governance/config` must
     not name `browser::` (the a7 arch-test forbids it), offering two resolutions: inject a
     validator hook, or carry the domain-pattern key in a browser key catalog. Chose the
     injection hook (simpler than splitting `KEYS` into two composed catalogs, which would
     ripple into every later G02/G03/G04/G12 consumer of a single flat registry): `pattern.rs`
     landed in `src/browser/pattern.rs` (also the future home G07's matcher extends, per
     RECONCILIATION's own placement table), and `KeyDef::parse_value` gained a
     `domain_pattern_valid: fn(&str) -> bool` parameter, consulted only for the
     `DomainPatternList` constraint. `governance/config`'s own tests use a small test-local
     validator (duplicating the grammar) so they never depend on the browser plugin; the
     authoritative grammar and its exhaustive test list (part 5 of g01's doc) live in
     `browser/pattern.rs`'s own tests. Verified via `cargo test --test architecture`: zero
     forbidden edges.
  3. Minor: kept `audit.destination` / `audit.file.path` descriptions ending "Takes effect on
     restart" per g01's literal text -- RECONCILIATION.md section 3 says these should eventually
     drop that clause once hot-reload (A5) and the audit sink re-open (G06) exist, but neither
     has landed yet at this point in the task sequence (A5 is the very next task after G02), so
     the restart-only wording is still truthful today. Revisit when A5+G06 land.
- Verification: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean,
  `cargo test` green (104 lib unit tests, up from 90: +13 new in `governance::config::tests`,
  +2 new in `browser::pattern::tests`; `tests/all_open_golden.rs` 3 unchanged;
  `tests/architecture.rs` 4 unchanged and still green after the `governance/config` rename
  -- confirms zero forbidden edges introduced; `tests/mcp_protocol.rs` 4 unchanged;
  `tests/peer_death.rs` 1 unchanged; `tests/tool_schema_fidelity.rs` 6 unchanged). Grep confirmed
  `FIRST_CALL_WAIT_MS` and `minimal_default` no longer appear anywhere in `src/`. ASCII scan
  clean on every touched/new file.
- Browser checks queued: none (binary-only config/registry growth; the wired
  `first_call_wait_ms` value is 5000 under the Safe/Minimal preset, byte-identical to the
  retired constant, so no behavior changed).

### g02 layered configuration resolution and file loading -- 2026-07-02
- Commit: (see this task's commit)
- Files touched: new `src/governance/config/layers.rs` (the ADR-0019 five-layer resolver) and
  `src/governance/config/load.rs` (paths, file parsing, orchestration); `src/governance/config/mod.rs`
  (`pub mod layers;`/`pub mod load;`, `Config::from_resolution` + four `resolved_*` helpers);
  `src/error.rs` (one new variant, `Error::Config(String)`); `src/transport/mcp/server.rs`
  (startup now calls `load::load_and_resolve` + `Config::from_resolution` instead of
  `Config::default()`).
- Summary: `layers::resolve` walks `KEYS` and picks, for each key, the first of
  org_mandatory/user/org_recommended/preset/builtin that defines it, returning the shared-format
  2.1 triple (value/source/locked); `layers::validate_value` delegates to G01's
  `KeyDef::parse_value`. `load::user_config_path`/`org_policy_path` implement the exact
  shared-format 1.1/1.2 per-platform paths (Windows/macOS/Linux `cfg` branches, `ProgramData`
  env fallback). `load::parse_user_config` is lenient per entry (warn + skip unknown keys,
  invalid values, unknown presets, unknown top-level members; hard error only on structurally
  broken JSON). `load::parse_org_config` is strict everywhere (every violation --
  bad/missing schema, non-array config, unknown key, invalid value, bad level, duplicate key,
  unexpected member -- is a hard `Error::Config`). `load_and_resolve` reads both files
  (`ErrorKind::NotFound` -> absent/empty layer; any other I/O error -> hard error), logs
  warnings via `tracing::warn!`, and resolves. `Config::from_resolution` builds the typed
  session `Config` from a `Resolution`, with a `debug_assert!`-guarded fallback to the Safe
  preset default on an unreachable-by-construction shape mismatch (mirroring the `preset_*`
  helpers' panic-is-unreachable reasoning from G01, but non-panicking since a resolution is
  runtime-influenced by file content rather than purely compile-time).
- Deviations from the g-doc per RECONCILIATION.md / carried forward from G01's precedent: g02's
  own doc (written pre-A1/G01) specifies `validate_value(def, value) -> Result<(), String>` and
  `load_and_resolve() -> Result<Resolution>` with NO domain-pattern-validator parameter, and has
  `parse_user_config`/`parse_org_config` likewise take no such parameter. Since G01 threaded a
  `domain_pattern_valid: fn(&str) -> bool` into `KeyDef::parse_value` (the RECONCILIATION
  section 2 "known integration point": the governance core cannot name the browser plugin's
  pattern grammar directly), every function in this task that ultimately validates a
  `content.security.sacred_domains` value inherits that same extra parameter:
  `validate_value`, `parse_user_config`, `parse_org_config`, and `load_and_resolve` all gained
  a `domain_pattern_valid: fn(&str) -> bool` parameter, threaded from `transport/mcp/server.rs`
  (which supplies `browser::pattern::is_valid_pattern`, the real grammar) down to
  `layers::validate_value`'s call into `KeyDef::parse_value`. This is the same shape of
  deviation G01 already made and is not a new architectural decision, just its continuation.
  Placement: `layers.rs` and `load.rs` land in `governance/config/` (not a flat
  `src/policy/{layers,load}.rs`), per RECONCILIATION section 1's mapping, consistent with G01.
- Verification: `cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings` clean
  (fixed two lints along the way: a doc-comment line break that clippy's `doc_lazy_continuation`
  read as an unclosed markdown blockquote due to a mid-sentence `>` at a line wrap -- reworded
  to avoid `>` entirely; and `needless_return` in `org_policy_path`'s per-platform `cfg` blocks,
  restructured to `#[cfg(..)] let path = ...;` per-platform bindings ending in a single tail
  `path` expression instead of early `return`s under an `#[allow(unreachable_code)]`). `cargo
  test` green (119 lib unit tests, up from 104: +6 new in `governance::config::layers::tests`,
  +8 new in `governance::config::load::tests`, including a windows-`cfg`-gated
  `paths_follow_the_shared_format_locations`; `tests/all_open_golden.rs` 3 unchanged;
  `tests/architecture.rs` 4 unchanged and still green -- confirms `governance/config/{layers,load}.rs`
  introduce zero forbidden edges despite doing real file I/O and platform-path logic;
  `tests/mcp_protocol.rs` 4 unchanged, including the byte-identical `tools/list` assertion --
  proves the layered resolver with both files absent is byte-identical to the old
  `Config::default()` path; `tests/peer_death.rs` 1 unchanged; `tests/tool_schema_fidelity.rs`
  6 unchanged). Confirmed no stray `%APPDATA%\browser-mcp\config.json` or
  `%ProgramData%\browser-mcp\policy.json` exists on the dev machine before running (both
  `Test-Path` false), so the live binary spawned by `tests/mcp_protocol.rs` resolves through
  the builtin layer only, exactly as required by the task's own verification note. ASCII scan
  clean on every touched/new file.
- Browser checks queued: none (binary-only startup wiring; no browser-facing behavior changed).

## Reminders before running BROWSER-TESTS.md

Stage 2 is mostly unit-testable (pure governance logic), but several tasks have browser-facing
behavior that needs a real browser: the take-the-wheel pause (g10), the panic kill switch (g11), tool
advertisement filtering and `tools/list_changed` on hot-reload (g14), and end-to-end manifest
enforcement (g12/g13/g15). Accumulate those checks in `BROWSER-TESTS.md` as their tasks land; a human
runs them against a live browser after the code is in, exactly as release-1 did.
