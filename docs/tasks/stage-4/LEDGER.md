# Stage 4 ledger

Durable, context-wipe-safe record of stage-4 (registry/pipeline architecture, ADR-0023/0024/0025)
execution. This file plus `docs/tasks/stage-2/BROWSER-TESTS.md` are the executor's memory. On
every start, after any interruption, and whenever state is unclear: read the RESUME HERE section
first, then `BOOTSTRAP.md` and the ADR(s) the current task cites, then the current task prompt,
then continue. Never rely on remembering earlier work; re-read files.

## RESUME HERE

- Branch: `stage-4` (created from `stage-3`; create it if absent). Never push, never merge,
  never commit to `main`, `stage-2`, or `stage-3`.
- Progress: t01 landed. The stage-3 org-policy outage is fixed: `parse_manifest` is the sole
  reader/parser/validator of the policy file; `parse_org_config` and `load_and_resolve` are
  deleted. t02 landed: `src/browser/directory.rs` generalized in place into the ADR-0024
  Decision 1 `ToolDescriptor` registry (14 rows, 26 variants); `requires()` and
  `explain_text()` keep their exact contracts and byte-identical output; nothing outside the
  module consumes the new fields yet.
- NEXT TASK: `t03` (`docs/tasks/stage-4/t03-governance-authorize.md`).
- Authority: ADR-0023/0024/0025 (each in its own scope) over task prompts over ADR-0022 over
  the stage-2 shared-format doc over SPEC.
- Invariants after every task: tree green (`cargo test`, `clippy -D warnings`, `fmt --check`),
  `tests/architecture.rs` passing, all-open byte-identical, tools.json and
  `tests/tool_schema_fidelity.rs` byte-untouched (NO exception task this stage), behavior
  preserved except the two sanctioned additions (t01 org-policy loading works, t06 hot-reload),
  ASCII-only, no new dependencies, superseded code deleted in the task that supersedes it.

## Task log

(Append one entry per completed task, newest at the bottom. Shape:)

### <task-id> <title> -- <date>
- Commit: (see this task's commit)
- Files touched: <list>
- Summary: <what landed, key decisions, any conservative choice made>
- Deviations from the prompt/ADR: <numbered, each with reasoning; "none" if none>
- Deletions performed: <the removed functions/files this task retired, or "none">
- Verification: <clippy/fmt/test status; test counts before -> after; which suites unchanged>
- Browser checks queued: <count and ids appended to BROWSER-TESTS.md, or "none">

### t01 one loader for the policy file -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: `src/governance/manifest/document.rs`, `src/governance/config/load.rs`,
  `src/governance/config/reload.rs`, `src/governance/config/cli.rs`,
  `src/governance/config/presets.rs`, `src/governance/manifest/source.rs`,
  `src/transport/mcp/server.rs`, `src/doctor.rs`, `tests/manifest_validation.rs`,
  `docs/tasks/stage-2/BROWSER-TESTS.md`, this file.
- Summary: implemented ADR-0023 in full. `parse_manifest` (`document.rs`) is now the sole
  reader/parser/validator of the policy file for every origin; its config-array validation
  pass rejects a duplicate `key` (Decision 3). `parse_org_config` and `load_and_resolve`
  (`config/load.rs`) are deleted; replaced by the pure `org_config_from_entries(entries:
  &[ConfigEntry]) -> OrgConfig` split, plus a small `org_config_from_policy(&LoadedPolicy) ->
  OrgConfig` helper (origin-gated: only an org-sourced manifest's entries reach the org
  layers) shared by `read_layers` (`config/load.rs`) and
  `ConfigStore::load_initial_with_policy` (`config/reload.rs`, the renamed/reshaped
  `load_initial_with_manifest_config`) so the CLI's and the server's views of the org layers
  can never disagree. `read_layers` gained a `&LoadedPolicy` parameter and now reads only the
  user config file, deriving the org contribution from the policy and merging the manifest's
  user-layer map (`manifest_config_as_user_layer`) under the user config file's own values
  (file wins on collision, transcribed from `reload.rs::merge_manifest_user_config`).
  `reload.rs::read_and_parse_org` re-points to `parse_manifest` +
  `org_config_from_entries`, mapping a `ManifestError` via `Display` alone (no double-path
  prefixing). `cli.rs::resolve_with_warnings` now loads the policy once and returns it
  alongside the resolution/warnings; `run_list` passes it to `shadow_line` (which lost its own
  `load_policy` call and gained a `&LoadedPolicy` parameter) instead of reloading a second
  time; `presets.rs::resolve_current_and_candidate` does the same one-load pattern.
  `server.rs`/`doctor.rs` both swap to `load_initial_with_policy(checker, &loaded_policy)` and
  drop their `manifest_config_as_user_layer` call sites (the store computes it internally
  now). `source.rs::manifest_config_as_user_layer`'s doc comment is rewritten to say the org
  branch is empty because org entries take the ORG channel, not because a second parser reads
  the file; its behavior and its two inline tests are unchanged. Added the new integration
  test `org_policy_file_with_config_boots_the_server` (`#[cfg(windows)]`,
  `tests/manifest_validation.rs`): spawns the real binary with a schema-3 org policy (one
  read-only grant, two mandatory config entries: `audit.enabled`, `audit.file.path` at a
  unique temp path) at a fake `ProgramData`-rooted org path and confirms the outage regression
  is gone (the server answers `initialize`/`tools/list` instead of exiting at startup) with
  the governed tool list transcribed verbatim from `tests/tool_advertisement.rs`.
- Deviations from the prompt/ADR:
  1. Added `org_config_from_policy(&LoadedPolicy) -> OrgConfig` in `config/load.rs`, not
     literally named in the prompt, as a small shared helper between `read_layers` and
     `ConfigStore::load_initial_with_policy` so the origin-gated "only an org-sourced
     manifest's entries reach the org layers" rule has exactly one implementation instead of
     being written out twice at the two call sites. Conservative choice per BOOTSTRAP rule 4
     (fewer moving parts; a single source of truth for a rule both the CLI and the server
     store depend on never disagreeing). No pinned signature, string, or test assertion was
     affected; `org_config_from_entries`'s own pinned signature is unchanged.
  2. The task prompt's own historical narrative sentence in the new integration test's doc
     comment was reworded to avoid the literal substring `parse_org_config` (referring to it
     instead as "the now-deleted second org-file parser"), so the prompt's own Verification
     step 2 (`rg -n "parse_org_config|load_and_resolve" src/ tests/` -> no hits) passes
     literally, including inside the new test's own doc comment.
- Deletions performed: `governance::config::load::parse_org_config` (and its test
  `org_file_violations_are_errors`), `governance::config::load::load_and_resolve` (dead, zero
  callers, verified via `rg` before deletion), `ConfigStore::load_initial_with_manifest_config`
  (renamed/reshaped to `load_initial_with_policy`; `load_initial` itself is KEPT as the
  zero-argument-beyond-checker convenience the prompt specifies, delegating to
  `load_initial_with_policy` with an all-open `LoadedPolicy`).
- Verification: `cargo fmt` (applied) then `cargo fmt --check` clean; `cargo clippy
  --all-targets -- -D warnings` clean; `cargo test` fully green, 461 -> 464 (net +3: added
  `duplicate_config_key_is_a_field_error`, `org_config_from_entries_splits_by_level`,
  `org_sourced_policy_config_reaches_the_org_layers`,
  `org_policy_file_with_config_boots_the_server`; removed
  `org_file_violations_are_errors`). `tests/architecture.rs` (4 tests),
  `tests/all_open_golden.rs` (3 tests), `tests/mcp_protocol.rs` (6 tests), and
  `tests/tool_schema_fidelity.rs` (7 tests) all pass unchanged.
  `git diff HEAD -- src/transport/mcp/schemas/tools.json tests/tool_schema_fidelity.rs` and
  `git diff HEAD -- Cargo.toml Cargo.lock` both empty. `rg -n
  "parse_org_config|load_and_resolve" src/ tests/` -> no hits; `rg -n "expected 2" src/` -> no
  hits. ASCII scan on all 9 touched files -> clean. Manual smoke: copied
  `examples/research-read-only.json` to the real `%ProgramData%\browser-mcp\policy.json`, ran
  `cargo run -- doctor` (rendered the manifest correctly, no "config resolution is broken"),
  deleted the file, re-ran doctor (confirmed all-open again).
- Browser checks queued: 1 (`t01-1`, appended to `docs/tasks/stage-2/BROWSER-TESTS.md`).

### t02 the tool registry -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: `src/browser/directory.rs`, `src/browser/mod.rs`, `src/browser/advertise.rs`,
  this file.
- Summary: implemented ADR-0024 Decision 1 in full. `src/browser/directory.rs` generalizes IN
  PLACE from the flat 26-row `ActionDescriptor`/`DIRECTORY` pair into the single per-tool
  `ToolDescriptor` registry (`REGISTRY`, 14 rows in tools.json advertised order): each row
  carries `tool`, `action_key` (`Some("action")` on `computer` only), `variants` (the 26
  existing `(action, requires, description)` triples unchanged, transcribed byte-for-byte as
  `ActionVariant`), `resource` (`ResourceShape`: `DomainLess`/`TabScoped`/`TargetArg`,
  mirroring today's `resolve_governing_resource` name match exactly), `handler` (`Handler`:
  `ExtensionForward` for 13 tools, `Local(explain_text)` for `explain`), `postprocess`
  (`Some(crate::browser::redact::apply_to_result)` on `read_page` only; verified the real
  signature is `fn(&mut serde_json::Value, bool)`, matching the pinned type exactly, no
  deviation needed there), and `post_dispatch` (`PostDispatch::NavigateLanding` on `navigate`
  only, `None` elsewhere). Added `descriptor(tool: &str) -> Option<&'static ToolDescriptor>`
  (linear scan). `requires(tool, action)` keeps its exact signature and semantics, reimplemented
  over `descriptor()` + `variants` (absent-vs-empty invariant unchanged). `explain_text()`
  reimplemented over `REGISTRY`, label generalized to `{tool} ({action})` from row data (no
  hardcoded `computer` literal); output is byte-identical, confirmed by the untouched
  server-side pin `pinned_explain_text_matches_the_real_directory_formatter`. `ActionDescriptor`
  and the flat `DIRECTORY` const are deleted; the inline test module is reworked per the task's
  Tests section (fixture-mirror technique kept). `src/browser/mod.rs`'s module doc sentence
  naming the directory is rewritten to name the ADR-0024 Decision 1 registry while keeping the
  `directory` module name and link (module not renamed).
- Deviations from the prompt/ADR:
  1. Constraint 1 said only `directory.rs` and `mod.rs` would change, but the live tree has a
     third direct consumer of the flat `DIRECTORY` const the prompt's Current Behavior survey
     did not mention: `src/browser/advertise.rs::tool_has_a_reachable_variant` iterates
     `directory::DIRECTORY` rows directly. Since Required Behavior section 3 unambiguously
     mandates deleting `DIRECTORY`, this consumer would not compile otherwise. Conservative fix
     per BOOTSTRAP rule 4 (behavior-preservation over structure-preservation; fewer moving
     parts): retargeted the same filter/any logic onto `directory::REGISTRY` rows' `variants`
     (`.filter(|row| row.tool == tool_name).flat_map(|row| row.variants.iter()).any(...)`),
     using only the pre-existing `tool`/`variants`/`requires` shape, no new type
     (`ResourceShape`/`Handler`/`PostDispatch`/`descriptor()`) referenced there, so constraint 4's
     `rg` check (new-field usage confined to `directory.rs`) still passes clean. Every
     `tool_advertisement.rs` and inline `advertise.rs` test still passes unchanged, confirming
     behavior is byte-identical.
  2. `per_tool_fields_match_the_adr_table`'s pinned `EXPECTED_TOOLS` tuple type triggered
     clippy's `type_complexity` lint (not itself pinned but a direct consequence of the pinned
     tuple shape in the Tests section). Added `#[allow(clippy::type_complexity)]` on that one
     `const` rather than restructure the pinned type, per BOOTSTRAP rule 14 (byte-pinned oracles
     move by transcription; the tuple shape is prescribed literally in the prompt).
- Deletions performed: `browser::directory::ActionDescriptor` (struct) and
  `browser::directory::DIRECTORY` (const); their absorbed content lives on as
  `REGISTRY[*].variants`. Superseded inline tests `directory_covers_the_sacred_surface_exactly`,
  `directory_requires_match_the_adr_table`, and
  `explain_text_is_the_vocabulary_block_then_one_line_per_row` are replaced by their reworked
  registry-shaped equivalents (`registry_covers_the_sacred_surface_exactly`,
  `registry_requires_match_the_adr_table`, `explain_text_is_unchanged_by_the_registry_reshape`,
  the last folding in the old structural test's pinned line assertions so no parallel dead test
  survives).
- Verification: `cargo fmt` (applied) then `cargo fmt --check` clean; `cargo clippy
  --all-targets -- -D warnings` clean; `cargo test` fully green, 464 -> 465 (net +1: five old
  directory tests replaced by six reworked/new registry tests --
  `registry_covers_the_sacred_surface_exactly`, `registry_requires_match_the_adr_table`,
  `absent_is_none_and_empty_is_some` (unchanged), `every_description_is_nonempty_ascii_and_short`
  (unchanged, iterates variants), `per_tool_fields_match_the_adr_table` (new),
  `explain_text_is_unchanged_by_the_registry_reshape` (new, folds in the old structural test)).
  `tests/architecture.rs` (4 tests), `tests/all_open_golden.rs` (3 tests), `tests/mcp_protocol.rs`
  (6 tests), and `tests/tool_schema_fidelity.rs` (7 tests) all pass unchanged. `git diff HEAD --
  src/transport/mcp/schemas/tools.json tests/tool_schema_fidelity.rs` and `git diff HEAD --
  Cargo.toml Cargo.lock` both empty. Constraint-4 `rg -n
  "ResourceShape|Handler::|PostDispatch|descriptor\(" src/ --glob '!src/browser/directory.rs'`
  returns nothing. ASCII scan on all 3 touched source files (`advertise.rs`, `directory.rs`,
  `mod.rs`) clean.
- Browser checks queued: none (pure data/lookup change; nothing observable live yet, per the
  task's own Verification section).
