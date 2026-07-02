# Stage 2 ledger

Durable, context-wipe-safe record of stage-2 (governance) execution. This file plus
`BROWSER-TESTS.md` are the executor's memory. On every start, after any interruption, and whenever
state is unclear: read the RESUME HERE section first, then `PLAN.md` and `RECONCILIATION.md`, then the
current task prompt, then continue. Never rely on remembering earlier work; re-read files.

## RESUME HERE

- Branch: `stage-2` (off `main`, which has stage 1 merged). Never push, never merge, never commit to
  `main`.
- Progress: NOTHING executed yet. Stage-2 begins here.
- NEXT TASK: Phase A, task `a1` (`docs/tasks/stage-2/a1-module-reorg.md`). Read `BOOTSTRAP.md` first if
  you have not.
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

## Reminders before running BROWSER-TESTS.md

Stage 2 is mostly unit-testable (pure governance logic), but several tasks have browser-facing
behavior that needs a real browser: the take-the-wheel pause (g10), the panic kill switch (g11), tool
advertisement filtering and `tools/list_changed` on hot-reload (g14), and end-to-end manifest
enforcement (g12/g13/g15). Accumulate those checks in `BROWSER-TESTS.md` as their tasks land; a human
runs them against a live browser after the code is in, exactly as release-1 did.
