# Bootstrap: unattended execution of the maturity-1 prompts

You are executing a prepared batch of tasks in the Ghostlight repository at
`f:\Replica\NAS\Files\repo\github\sylin-org\browser-mcp`. Everything you need
is in this directory (docs/tasks/maturity-1/). You resolve nothing by judgment:
where a prompt is silent, the answer is in 00-design.md or the cited ADR; where
none of them answer, STOP and record a BLOCKED entry.

## Ground rules

1. Authority order: this file, then the task prompt, then 00-design.md, then
   the cited ADRs (docs/adr/0026, 0027), then the tree. A MATERIAL conflict (the
   two authorities require different concrete actions) is a STOP (BLOCKED entry,
   move on), not a judgment call. NOT a conflict: 00-design.md and a prompt are
   MORE SPECIFIC than the ADRs and govern this batch's scope. In particular,
   00-design.md's "Excluded from this batch" list intentionally implements a
   subset of an ADR's decisions, and 00-design.md refines ADR wording (for
   example `cargo clippy --all-targets -- -D warnings`); following the more
   specific instruction is correct, not a conflict.
2. One task = one commit. Conventional commit format; each prompt pins its
   commit subject. Never amend, never rebase, never push, never merge.
3. Work on branch `maturity-1`, created from the `dev` tip you find. Record
   the base commit in the ledger before m01.
4. Before m01: run `cargo test` and record the baseline count in the ledger.
   If the baseline is red, STOP entirely (BLOCKED, no tasks run). EXCEPTION: a
   `cargo` error of the form "failed to remove file '...ghostlight.exe' Access
   is denied (os error 5)" is NOT a red baseline; it means a Ghostlight process
   holds the build output. Close any running ghostlight.exe, or run every
   `cargo test`/`cargo build` in this batch with an isolated target dir
   (`CARGO_TARGET_DIR=target/it cargo test`, Git Bash) and record that you did.
5. ASCII only in code you write; docs you write use no em-dashes and no smart
   quotes. Scan your changes INCLUDING new files (stage first, because
   `git diff` without `--cached` omits untracked files), under Git Bash:
   `git add -A && git diff --cached -U0 | grep "^+" | rg -n "[^\x00-\x7F]"`
   must be empty for every task (the ghost glyph exists only as an escape). If
   you must express a non-ASCII or control test input, write it as a JS or
   Rust escape (the six characters backslash u 0 0 e 9; or backslash u 0 0 0
   1), never a literal byte.
6. Never touch the files in the Never touch list except where a prompt is
   named there as the sanctioned owner.
7. Re-read the tree before editing: every prompt's Current behavior section is
   as-of authoring (2026-07-03). LINE NUMBERS ARE ADVISORY: m02 inserts a header
   at line 1 of every .rs and .js file, so every cited line number in a later
   prompt is off by one once m02 has run. That drift is expected and is NOT a
   contradiction. STOP only when the CONTENT a prompt quotes (a function body, a
   string, an enum) is genuinely absent or different; locate it by content (rg
   for the symbol), never by line number.
8. Verbatim means verbatim: where a prompt pins text, YAML, or code, transcribe
   it byte-for-byte. Do not improve, reformat, or reorder it.
9. Tests are pinned by the prompts. Do not derive your own expected values; if
   a pinned expectation appears wrong, that is a STOP, not a fix.
10. Run every verification command listed in the prompt, in order, and record
    the outcomes in the ledger entry.
11. Golden files regenerate only via the sanctioned commands in 00-design.md,
    run under Git Bash, followed by a hand review of the diff.
12. If a tool or dependency is missing (rg, node, cargo, network for npm),
    record it and STOP that task; do not substitute tooling.

## Environment facts

- Windows 11; PowerShell 7 is the primary shell, Git Bash is available (use
  Git Bash for anything involving output redirection to files).
- Rust stable toolchain; `cargo test` green at batch authoring (475 tests run;
  477 test declarations; 2 are platform-gated off Windows). A Ghostlight process
  running from `target/debug/ghostlight.exe` locks that file and makes `cargo
  test`/`cargo build` fail with os error 5; see ground rule 4.
- Node v24 installed; network access works (npm install has succeeded on this
  machine recently).
- The repo is a single crate (`ghostlight`), publish = false. No .github/
  directory exists yet. No package.json exists anywhere outside reference/.
- ripgrep (`rg`) is available.

## Task sequence

Linear; every prefix leaves a coherent, buildable, green tree:

1. m01-ledger-correction.md (docs only, tiny)
2. m02-spdx-headers.md (mechanical, all .rs/.js headers)
3. m03-ci-workflows.md (creates .github/workflows/)
4. m04-audit-syslog-none.md (Rust: destinations + config key + goldens)
5. m05-extension-lib-extraction.md (extension: lib/ + node tests)
6. m06-headless-smoke.md (tests/e2e/ harness + CI job; riskiest, last)

## Never touch

Absolute (no sanctioned exception in this batch):

- src/transport/mcp/schemas/tools.json and tests/tool_schema_fidelity.rs (the
  sacred tool surface; m02 explicitly skips the test file)
- LICENSE, LICENSE-APACHE, LICENSE-MIT, LICENSE-GOVERNANCE, LICENSING.md
- docs/adr/** (read-only authority)
- docs/SPEC.md (its rewrite is out of batch)
- extension/manifest.json (m05 needs no manifest change; if you believe a task
  requires one, that is a STOP)
- .dev-key.pem; any credential or key material
- The prompts, 00-design.md, and this BOOTSTRAP.md in docs/tasks/maturity-1/
  (your own instructions; the only file here you write to is LEDGER.md)
- git push, merge, rebase, force operations, --no-verify

Owned exceptions (single sanctioned owner):

- docs/tasks/stage-4/LEDGER.md: append-only, owner m01
- src/governance/audit/** and src/governance/config/**: m02 adds ONLY the SPDX
  header line; m04 owns all other edits (only the files its prompt names)
- tests/golden/config-schema.json, tests/golden/config-keys.md: owner m04,
  regeneration only
- README.md: owner m04, only the single pinned bullet edit
- extension/service-worker.js: m02 adds ONLY the SPDX header line; m05 owns the
  refactor
- .github/workflows/ci.yml: created by m03; m05 and m06 each APPEND only the
  job their prompt pins (distinct job names, appended at end of file; order
  between m05 and m06 does not matter)
- .gitignore: owner m06 (the two pinned entries)
- This directory's LEDGER.md: every task appends its own entry and updates
  RESUME HERE in place

## Per-task procedure

Before m01 only: create the `maturity-1` branch from the `dev` tip, then fill
the RESUME HERE block's Branch, Base commit, and Baseline fields (from ground
rule 4). Those RESUME HERE edits are committed as part of m01's single commit,
not on their own.

Per task:

1. Read the task prompt fully. Check its Depends on / STOP preconditions.
2. Re-verify the prompt's Current behavior CONTENT against the tree (rule 7).
3. Implement exactly the Required behavior. Nothing more.
4. Add the tests the prompt names, with the pinned assertions.
5. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test` (plus the prompt's own verification commands).
6. Run the ASCII scan (ground rule 5).
7. Update docs/tasks/maturity-1/LEDGER.md: append this task's entry (template in
   that file, with numbered deviations or "none") and update RESUME HERE in
   place (Progress, NEXT TASK).
8. Commit EVERYTHING the task changed, including the ledger update, in ONE
   commit with the pinned subject. The tree is clean after the commit (one task
   = one commit, ledger included).

## Failure protocol

After two focused attempts at a failing step: restore the working tree to the
last commit. Run `git status` first. Restore tracked files with `git restore .`.
Remove ONLY the specific new files this task created, BY NAME (for example
`rm .github/workflows/ci.yml`); NEVER run a bare `git clean -fd` (it would
delete this batch directory and any other untracked work). Then write a BLOCKED
ledger entry quoting the failure evidence, update RESUME HERE, commit that
ledger entry alone (subject `docs(tasks): maturity-1 <task> BLOCKED`), and SKIP
to the next task. Exception: if m03 is BLOCKED, m05 and m06 still run but must
STOP at their ci.yml edit step and record that sub-step as blocked-by-m03.
Never leave the tree dirty between tasks.

## Completion

All six tasks committed or BLOCKED; ledger RUN SUMMARY written (tests
before/after, tasks landed, deviations rolled up); RESUME HERE marked complete;
tree clean; branch `maturity-1` left unpushed for a human to review and merge.
