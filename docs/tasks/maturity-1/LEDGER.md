# Maturity-1 ledger

Durable execution record for the m01-m06 batch. The Task log is append-only
(one entry per task, newest at the bottom); the RESUME HERE block below is
updated in place each task. Each task commits its own ledger changes as part of
that task's single commit.

## RESUME HERE

- Branch: `maturity-1` (create from the `dev` tip; record the base commit here)
- Baseline: (record `cargo test` count before m01)
- Progress: not started
- NEXT TASK: m01 (docs/tasks/maturity-1/m01-ledger-correction.md)
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

## RUN SUMMARY

(Write after the last task: tasks landed vs BLOCKED, test counts baseline ->
final, deviations rolled up, anything left for a human.)
