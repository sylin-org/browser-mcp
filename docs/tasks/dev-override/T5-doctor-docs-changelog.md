# T5 -- doctor's override line + docs + changelog (ADR-0048 D7)

## Goal

`ghostlight doctor` on the DEFAULT instance reports whether a live dev instance currently
shadows it for unpinned clients (one probe, one section), and the durable docs catch up:
DEV-LOOP's install step collapses to the plain default install, README stops requiring
`--extension-id`, and the CHANGELOG gains the ADR-0048 entries. Normative: ADR-0048 D7 (+ D5/D6
for the doc text). Oracles: PINS.md P5.

## Files this task owns (touch nothing else)

- `crates/core/src/hub/manage/doctor.rs`
- `docs/DEV-LOOP.md`
- `README.md`
- `CHANGELOG.md`
- `docs/tasks/dev-override/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ 3928a74 -- re-read before editing)

- doctor.rs: an `instance` binding is in scope from the `Instance:` section
  (`println!("  {:<9}{}", "name", instance.label());`); the IPC endpoint section ends with
  `println!("  {:<9}{}", "state", state_line(&probe));` followed by
  `let (log_dir, rows) = gather_sessions();`. The file already references `ipc::probe_endpoint`
  and `ipc::default_endpoint` through an existing `ipc` import path.
- DEV-LOOP.md has the section heading `## 2. Install the dev instance (once)` whose command line
  is `ghostlight --instance dev install --no-supervisor --debug --extension-id <your-unpacked-id>`.
- README.md has the step-4 extension-id lines and the
  `./target/release/ghostlight install --extension-id cjcmhepmagomefjggkcohdbfemacojoa` command
  block quoted in PINS P5.
- CHANGELOG.md's `## [Unreleased]` currently opens with `### Fixed` and also has a `### Changed`
  list (the ADR-0047 entries).

## STOP preconditions

- STOP if doctor.rs has no in-scope `instance` binding at the insertion point.
- STOP if T1 is not landed (`DEV_INSTANCE` must exist in transport::instance).
- STOP if the CHANGELOG has no `## [Unreleased]` heading.

## Changes (transcribe from PINS P5)

1. doctor.rs: insert the pinned `Development override:` section at the pinned anchor, matching
   the file's existing `ipc` import path for `probe_endpoint` / `adapter_endpoint_name` /
   `EndpointProbe`.
2. DEV-LOOP.md: replace section 2 with the pinned text; scrub any later `--extension-id` remnant
   per the pin.
3. README.md: the three pinned replacements (step-4 lines, the install command block, the
   Troubleshooting bullet tail) + the conditional flags-list bullet per the pin (verified at
   authoring: the flags list does not mention --extension-id, so nothing is added there).
4. CHANGELOG.md: the pinned `### Added` block above `### Fixed`; the two pinned bullets appended
   to `### Changed`.

## Verification (all green, in this order)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
node --test tests/extension/grouping.test.js
```

Plus the batch completion check (BOOTSTRAP): `git diff --name-only <base>..HEAD` contains NO
file from the NEVER list.

## Out of scope (fences)

- NO change to doctor's existing sections, verdict logic, or `--fix`.
- NO other README/docs edits beyond the pinned replacements.
- NO version bump; the CHANGELOG entries stay under `[Unreleased]`.

## Commit

Stage exactly the four named files. Pinned message (PINS P5):

```
feat(doctor): report the live development-override routing + ADR-0048 docs (ADR-0048 D7)
```

Then update LEDGER.md (RESUME HERE -> COMPLETE + the batch-complete note) and commit as
`docs(dev-override): ledger T5`.
