# Maturity-1 ledger

Durable execution record for the m01-m06 batch. The Task log is append-only
(one entry per task, newest at the bottom); the RESUME HERE block below is
updated in place each task. Each task commits its own ledger changes as part of
that task's single commit.

## RESUME HERE

- Branch: `maturity-1` (created from `dev` tip, base commit
  f66fbf02ae4a3b54c8b9cf92a8f448519be0662a)
- Baseline: `cargo test` (via `CARGO_TARGET_DIR=target/it`, see deviation in
  m01 entry) = 475 passed, 0 failed
- Progress: m01 done
- NEXT TASK: m02 (docs/tasks/maturity-1/m02-spdx-headers.md)
- Authority: BOOTSTRAP.md, then the task prompt, then 00-design.md, then
  ADR-0026/0027
- Invariants: tree green and clean between tasks; no push; ASCII diff scan per
  task

## Task log

(Append one entry per completed task. Shape:)

### <task-id> <title> -- <date>
- Commit: (see this task's commit)
- Files touched: <list>
- Summary: <what landed, key decisions, any conservative choice made>
- Deviations from the prompt/design: <numbered, each with reasoning; "none" if none>
- Verification: <fmt/clippy/test status; test counts before -> after; the
  prompt's own verification command outcomes>
- Notes for the reviewer: <anything a human should double-check, or "none">

### m01 stage-4 ledger post-run correction -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: docs/tasks/stage-4/LEDGER.md, docs/tasks/maturity-1/LEDGER.md
- Summary: Appended the pinned POST-RUN CORRECTION block to the end of
  docs/tasks/stage-4/LEDGER.md, verbatim per m01's Required behavior, noting
  that the t-live-1 consolidated live pass (commit 44db1f3) has since run and
  passed, while still owed: g13-1 steps 4-5, g13-3's governed half, g15-1/g15-2,
  and macOS/Linux live checks. No existing byte of the file changed. Filled in
  the maturity-1 LEDGER.md RESUME HERE block (branch, base commit, baseline).
- Deviations from the prompt/design: 1. Three ghostlight.exe processes were
  running (target/debug/ghostlight.exe locked), so all `cargo test` runs in
  this batch use `CARGO_TARGET_DIR=target/it` per BOOTSTRAP ground rule 4
  rather than closing the running processes (one is this session's own
  connected MCP server). 2. The appended block was written via a small Python
  one-liner (not the Edit tool) to guarantee byte-exact CRLF line endings
  matching the rest of the file, since the repo has `core.autocrlf=true` and
  no .gitattributes.
- Verification: `rg -c "POST-RUN CORRECTION"` -> 1; `rg -c "44db1f3"` -> 1;
  `rg -c "PLAIN STATEMENT"` -> 1 (unchanged); `git diff` shows only appended
  lines (no existing line changed). ASCII diff scan on staged changes: empty
  (clean). Baseline `cargo test` (isolated target dir): 475 passed, 0 failed.
  Spot-run `cargo test --test hot_reload`: 1 passed (org_policy_hot_swap_end_to_end).
- Notes for the reviewer: none.

## RUN SUMMARY

(Write after the last task: tasks landed vs BLOCKED, test counts baseline ->
final, deviations rolled up, anything left for a human.)
