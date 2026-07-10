# managed-5 batch: LEDGER

Single source of truth for batch progress. Update after EVERY task (BOOTSTRAP step 5). A fresh
executor resumes from RESUME HERE with no other context.

## RESUME HERE

Batch authored 2026-07-10; red-team re-read against the live tree completed the same day (T1/T2/
T3/T8 verified aligned; T4 caller-integration corrected -- print loop, not a lines vec; T6
precondition corrected -- multiple denial render sites exist, append at the pipeline emission
chokepoint; T7 anchors verified exactly and pinned). NOT started. Next task: T1.

## Status

| Task | Title | Status | Commit | Deviations |
| --- | --- | --- | --- | --- |
| T1 | Bundle `kind` discriminator | pending | - | - |
| T2 | ManagedStatus sidecar (single writer in managed::activate) | pending | - | - |
| T3 | Presentation validation (additive-only limits) | pending | - | - |
| T4 | doctor managed line (reads the sidecar) | pending | - | - |
| T5 | explain-tool Policy Passport section | pending | - | - |
| T6 | Denials-as-doors: org contact line | pending | - | - |
| T7 | Audit provenance: policy_seq on tool-call records | pending | - | - |
| T8 | Lightbox scenarios: passport-freshness + sidecar-propagation | pending | - | - |

Status values: `pending` | `in-progress` | `DONE` | `BLOCKED`.

## Log

One entry per task as it closes (or blocks). Number every deviation from the task file.
