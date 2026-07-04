# Bootstrap: unattended execution of the licensing-1 prompts

You are executing a prepared batch of tasks in the Ghostlight repository at
`f:\Replica\NAS\Files\repo\github\sylin-org\browser-mcp`. Everything you need is in this
directory (docs/tasks/licensing-1/). You resolve nothing by judgment: where a prompt is
silent, the answer is in 00-design.md or ADR-0028; where none of them answer, STOP and
record a BLOCKED entry.

## Ground rules

1. Authority order: this file, then the task prompt, then 00-design.md, then ADR-0028,
   then the tree. A MATERIAL conflict (two authorities require different concrete actions)
   is a STOP (BLOCKED entry, move on), not a judgment call. 00-design.md and the prompts
   are MORE SPECIFIC than the ADR and govern this batch's scope.
2. One task = one commit. Conventional commit format; each prompt pins its commit
   subject. Never amend, never rebase, never push, never merge.
3. Work on branch `licensing-1`, created from the `dev` tip you find. Record the base
   commit in the ledger before l01.
4. Before l01: run `cargo test` and record the baseline count in the ledger. If the
   baseline is red, STOP entirely (BLOCKED, no tasks run). EXCEPTION: a `cargo` error of
   the form "failed to remove file '...ghostlight.exe' Access is denied (os error 5)" is
   NOT a red baseline; it means a Ghostlight process holds the build output. Close any
   running ghostlight.exe, or run every `cargo test`/`cargo build`/`cargo run` in this
   batch with an isolated target dir (`CARGO_TARGET_DIR=target/it cargo test`, Git Bash)
   and record that you did.
5. ASCII only in code you write; docs you write use no em-dashes and no smart quotes.
   Scan your changes INCLUDING new files (stage first, because `git diff` without
   `--cached` omits untracked files), under Git Bash:
   `git add -A && git diff --cached -U0 | grep "^+" | rg -n "[^\x00-\x7F]"`
   must be empty for every task.
6. Never touch the files in the Never touch list except where a prompt is named there as
   the sanctioned owner.
7. Re-read the tree before editing: every prompt's Current behavior section is as-of
   authoring (2026-07-03). Line numbers are advisory; locate cited code by CONTENT (rg
   for the symbol), never by line number. STOP only when quoted CONTENT is genuinely
   absent or different.
8. Verbatim means verbatim: where a prompt pins text, YAML, or code, transcribe it
   byte-for-byte. Do not improve, reformat, or reorder it.
9. Tests are pinned by the prompts. Do not derive your own expected values; if a pinned
   expectation appears wrong, that is a STOP, not a fix.
10. Run every verification command listed in the prompt, in order, and record the
    outcomes in the ledger entry.
11. If a tool or dependency is missing (rg, cargo, network for the crates.io fetch of the
    two new dependencies), record it and STOP that task; do not substitute tooling.
12. ADR-0028 Decision 1 is an invariant: if any step you are about to take would make
    ANY behavior conditional on license state, you have misread the prompt. STOP.

## Environment facts

- Windows 11; PowerShell 7 primary, Git Bash available (use Git Bash for anything
  involving output redirection to files).
- Rust stable toolchain; `cargo test` green at batch authoring (479 tests pass on the
  authoring machine; 2 more are platform-gated off Windows). A running ghostlight.exe
  locks target/debug/ghostlight.exe; see ground rule 4.
- Network access works (crates.io fetch of ed25519-dalek and base64 in l01 needs it).
- ripgrep (`rg`) is available.
- The repo is a single crate `ghostlight`, publish = false, edition 2021, no
  `[features]` table in Cargo.toml yet.

## Task sequence

Linear; every prefix leaves a coherent, buildable, green tree:

1. l01-license-core.md (deps + src/governance/license.rs + unit tests)
2. l02-disk-resolution-and-recorder-stamp.md (paths + Recorder stamp mechanics)
3. l03-license-cli-and-fixture.md (CLI subcommands + dev fixture + integration tests)
4. l04-startup-wiring-and-doctor.md (server.rs wiring + doctor section)
5. l05-security-md-and-sbom.md (release.yml SBOM step ONLY; SECURITY.md landed outside
   the batch and is never-touch)
6. l06-business-templates.md (docs/business/templates/, transcription only)

l05 and l06 are independent of l01-l04: if an earlier task is BLOCKED, still run them.

## Never touch

Absolute (no sanctioned exception in this batch):

- src/transport/mcp/schemas/tools.json and tests/tool_schema_fidelity.rs
- src/governance/explain.rs and every golden under tests/golden/ (this batch regenerates
  NO goldens; if a golden test fails, STOP)
- tests/audit_recorder.rs (the stamp design keeps it green unchanged; a failure is a STOP)
- LICENSE, LICENSE-APACHE, LICENSE-MIT, LICENSE-GOVERNANCE, LICENSING.md
- SECURITY.md, PRICING.md, README.md, docs/guides/**, docs/COMPARISON.md (public content
  owned outside this batch)
- docs/adr/** (read-only authority)
- docs/SPEC.md, extension/** (no extension change in this batch)
- .github/workflows/ci.yml (this batch touches only release.yml, owner l05)
- .dev-key.pem; any credential or key material outside the deliberately-public DEV_SEED
- The prompts, 00-design.md, and this BOOTSTRAP.md (the only file here you write to is
  LEDGER.md)
- git push, merge, rebase, force operations, --no-verify

Owned exceptions (single sanctioned owner):

- Cargo.toml: owner l01 (the two dependencies + the [features] table, nothing else)
- src/governance/license.rs: created by l01; l02 appends the disk-resolution functions;
  l03 appends the state_row helper and its unit test
- src/governance/mod.rs: owner l01 (one `pub mod license;` line)
- src/governance/audit/mod.rs: owner l02 (stamp field, setter, write_serialized change,
  new tests; nothing else)
- src/main.rs: owner l03 (the License command variant and its wiring)
- src/transport/mcp/server.rs: owner l04 (the pinned startup lines only)
- src/doctor.rs: owner l04 (the License section only)
- .github/workflows/release.yml: owner l05 (the pinned SBOM additions only)
- docs/business/templates/: created by l06
- This directory's LEDGER.md: every task appends its entry and updates RESUME HERE

## Per-task procedure

Before l01 only: create the `licensing-1` branch from the `dev` tip, then fill the RESUME
HERE block's Branch, Base commit, and Baseline fields. Those edits are committed as part
of l01's single commit.

Per task:

1. Read the task prompt fully. Check its Depends on / STOP preconditions.
2. Re-verify the prompt's Current behavior CONTENT against the tree (rule 7).
3. Implement exactly the Required behavior. Nothing more.
4. Add the tests the prompt names, with the pinned assertions.
5. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
   (plus the prompt's own verification commands). For l01-l04 also run
   `cargo clippy --all-targets --features license-admin -- -D warnings` so the gated code
   is linted too.
6. Run the ASCII scan (ground rule 5).
7. Update docs/tasks/licensing-1/LEDGER.md: append this task's entry and update RESUME
   HERE in place (Progress, NEXT TASK).
8. Commit EVERYTHING the task changed, including the ledger update, in ONE commit with
   the pinned subject.

## Failure protocol

After two focused attempts at a failing step: restore the working tree to the last
commit. Run `git status` first. Restore tracked files with `git restore .`. Remove ONLY
the specific new files this task created, BY NAME; NEVER run a bare `git clean -fd`.
Then write a BLOCKED ledger entry quoting the failure evidence, update RESUME HERE,
commit that ledger entry alone (subject `docs(tasks): licensing-1 <task> BLOCKED`), and
SKIP to the next task. Never leave the tree dirty between tasks.

## Completion

All six tasks committed or BLOCKED; ledger RUN SUMMARY written (tests before/after,
tasks landed, deviations rolled up); RESUME HERE marked complete; tree clean; branch
`licensing-1` left unpushed for a human to review and merge.
