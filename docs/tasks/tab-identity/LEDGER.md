# tab-identity batch -- LEDGER

Durable execution record. One task = one code commit + one ledger commit. Update after EVERY
task (or block); this file is the single source of truth for batch progress.

## RESUME HERE

Next task: **T1** (`T1-managed-surface-predicate.md`). Base: the bundle-introducing docs commit
on dev (source anchors verified at its parent, c49ee6d).

## Task table

| Task | Status | Code commit | Notes |
|---|---|---|---|
| T1 managed-surface predicate | pending | - | |
| T2 down-classifier | pending | - | |
| T3 stable session guid | pending | - | |
| T4 envelope guid + session ops | pending | - | |
| T5 client-name titles + errors | pending | - | |
| T6 liveness + pruning + changelog | pending | - | |

## Per-task log

(Append one entry per task: commit hash, verification results, and EVERY deviation from the task
file/PINS, numbered. A BLOCKED entry carries the failed precondition or error text verbatim and
your reasoning, then the batch HALTS per BOOTSTRAP.)
