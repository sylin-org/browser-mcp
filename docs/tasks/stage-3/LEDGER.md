# Stage 3 ledger

Durable, context-wipe-safe record of stage-3 (capability model, ADR-0022) execution. This file
plus `docs/tasks/stage-2/BROWSER-TESTS.md` are the executor's memory. On every start, after any
interruption, and whenever state is unclear: read the RESUME HERE section first, then
`BOOTSTRAP.md` and ADR-0022, then the current task prompt, then continue. Never rely on
remembering earlier work; re-read files.

## RESUME HERE

- Branch: `stage-3` (created from `stage-2`; create it if absent). Never push, never merge,
  never commit to `main` or `stage-2`.
- Progress: `s01`, `s02` landed.
- NEXT TASK: `s03` (`docs/tasks/stage-3/s03-action-directory.md`).
- Authority: ADR-0022 (`docs/adr/0022-intent-calibrated-capabilities.md`) over task prompts over
  the stage-2 shared-format doc (superseded in sections 4.3 / 6.1-rw / 8) over SPEC.
- Invariants after every task: tree green (`cargo test`, `clippy -D warnings`, `fmt --check`),
  `tests/architecture.rs` passing, all-open byte-identical, the 13 trained tool schemas
  byte-identical (s07 adds the one sanctioned 14th; no other tools.json change ever),
  ASCII-only, no new dependencies, superseded code deleted in the task that supersedes it.

## Task log

(Append one entry per completed task, newest at the bottom. Shape:)

### <task-id> <title> -- <date>
- Commit: (see this task's commit)
- Files touched: <list>
- Summary: <what landed, key decisions, any conservative choice made>
- Deviations from the prompt/ADR: <numbered, each with reasoning; "none" if none>
- Verification: <clippy/fmt/test status; test counts before -> after; which suites unchanged>
- Browser checks queued: <count and ids appended to BROWSER-TESTS.md, or "none">

### s01 navigate is read -- 2026-07-03
- Commit: (see this task's commit, `fix(governance): s01 navigate is read`)
- Files touched: `src/browser/classify.rs`, `src/browser/advertise.rs`,
  `src/governance/enforcement.rs`, `src/governance/simulate.rs`,
  `src/transport/mcp/server.rs`, `tests/tool_advertisement.rs`, `tests/tool_enforcement.rs`,
  `tests/shadow_mode.rs`, `tests/policy_simulate.rs`, `docs/tasks/stage-2/BROWSER-TESTS.md`,
  `docs/tasks/stage-3/LEDGER.md`.
- Summary: flipped the single `("navigate", RwClass::Mutate)` row in
  `src/browser/classify.rs` to `RwClass::Observe` (navigate is provably a GET, per
  ADR-0022 Context/Decision 2), with the exact banner comment the prompt pinned. Updated
  every dependent expectation per the prompt's per-file instructions: the read-only
  advertisement fixture grows from 8 to 9 tools (navigate now included, in fixture order,
  in both `advertise.rs` and `tests/tool_advertisement.rs`); `tests/tool_enforcement.rs`'s
  mutate-on-read-grant example moved from `navigate` to the domain-less `tabs_create_mcp`
  union-rule path (a local `research-read` grant, not the shared
  `EXAMPLE_FULL_AND_RESEARCH_READ` constant, so the all-access `example-full` grant cannot
  mask the denial), and a new test `navigate_is_permitted_on_a_read_only_grant` pins the
  bugfix end to end (allow, `grant_id: research-read`, `rw: observe`, correct audit domain);
  `tests/shadow_mode.rs`'s would-deny call moved to `tabs_create_mcp` for the same reason;
  `src/transport/mcp/server.rs`'s two inline tests updated (`rw` expectation to
  `"observe"`; the shadow-deny pair's shared call switched to `tabs_create_mcp` with a
  matching fake-extension response of `"created"`); `tests/policy_simulate.rs`'s golden
  totals moved from 3/6/4 to 4/5/4 (13 total unchanged) and the now-stale
  `docs-read`/`navigate`/`access` group line was deleted, leaving three groups in the
  pinned order. Renamed stub-driven `"navigate"`+`RwClass::Mutate` pairings in
  `enforcement.rs` (11 call sites across 8 tests) to `"form_input"` (with the derived
  `tool/navigate` -> `tool/form_input` rule strings and the two
  `exclude_tools: ["navigate"]` -> `["form_input"]`), except the deliberate exception in
  `scheme_and_about_blank`, which keeps the `"navigate"` literal and flips its two
  `RwClass::Mutate` arguments to `RwClass::Observe` per the prompt. Did the equivalent
  truthfulness rename in `simulate.rs`'s `stub_classify` (navigate entry to `Observe`) and
  its three consuming tests (two `navigate` replay lines switched to `javascript_tool`,
  which was already `Mutate` in the stub, per the prompt's exact instruction);
  `totals_arithmetic_holds` left untouched as instructed (its navigate line now flips to
  allow, but the sum invariant it checks still holds).
- Deviations from the prompt/ADR: none. Every literal, table, rename, and test name
  transcribed as pinned by the prompt; no ADR/prompt conflict encountered.
- Verification: `cargo fmt` (reformatted 2 files, whitespace/wrapping only, re-verified
  with a full re-run of `cargo test` afterward -- unchanged pass count) then
  `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo
  test` 430 -> 431 (one net new test, `navigate_is_permitted_on_a_read_only_grant`;
  `tests/tool_enforcement.rs` 7 -> 8), all passing, 0 failed. Confirmed byte-unchanged and
  green: `tests/architecture.rs` (4 tests), `tests/all_open_golden.rs` (3 tests),
  `tests/mcp_protocol.rs` (4 tests), `tests/tool_schema_fidelity.rs` (6 tests),
  `tests/audit_recorder.rs` (2 tests) -- none of these five files were touched by this
  task's diff (`git diff --stat` confirms). ASCII scan on every touched file (`rg -n
  "[^\x00-\x7F]" <files>`) printed nothing.
- Browser checks queued: 1 (`s01-1` appended to `docs/tasks/stage-2/BROWSER-TESTS.md`, the
  exact text pinned by the task prompt).

### s02 capability vocabulary in the governance core -- 2026-07-03
- Commit: (see this task's commit, `feat(governance): s02 capability vocabulary in the
  governance core`)
- Files touched: `src/governance/ports.rs`, `docs/tasks/stage-3/LEDGER.md`.
- Summary: added the ADR-0022 Decision 1 capability taxonomy as a pure, additive type in
  the governance core: the `Capability` enum (`Read`, `Action`, `Write`, `Execute`,
  `#[serde(rename_all = "lowercase")]`), its `as_str`/`from_name` helpers, and the
  free-standing `capability_subset(requires, allowed)` containment helper, inserted
  verbatim from the task prompt immediately after the `impl EffectiveMode` block and
  before `ToolId`, doc comments included. Nothing consumes the new type in this task
  (s05 wires it in); `RwClass` is untouched and stays the classification in force until
  s06. The diff is additive-only: every pre-existing line in `ports.rs` is byte-unchanged
  (`git diff --stat` and manual read confirm only inserted lines).
- Deviations from the prompt/ADR: none. The enum, helpers, and all three named tests were
  transcribed verbatim from the prompt; no ADR/prompt conflict encountered.
- Verification: `cargo fmt` (no changes beyond what was written) then `cargo fmt --check`
  clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo test` 431 -> 434 (three
  net new tests: `capability_wire_names_round_trip`,
  `capability_from_name_rejects_unknown_and_case_variants`,
  `capability_subset_truth_table`, all in `src/governance/ports.rs`'s `mod tests`, which is
  part of the lib unit-test binary, 370 -> 373), all passing, 0 failed. Baseline of 431 was
  independently reconfirmed by stashing this task's diff and re-running the full suite
  before restoring it. Confirmed unchanged and green: `tests/architecture.rs` (3 tests),
  `tests/all_open_golden.rs` (4 tests), `tests/mcp_protocol.rs` (4 tests),
  `tests/tool_schema_fidelity.rs` (6 tests) -- none of these four files appear in this
  task's `git diff --stat` (only `src/governance/ports.rs` and this ledger changed). ASCII
  scan (`rg -n "[^\x00-\x7F]" src/governance/ports.rs`) printed nothing.
- Browser checks queued: none (a pure type addition; no BROWSER-TESTS.md entry, per the
  task prompt's Verification section).
