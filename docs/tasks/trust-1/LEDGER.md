# trust-1 batch: LEDGER

Single source of truth for batch progress. Update after EVERY task. A fresh executor resumes from
RESUME HERE with no other context.

## RESUME HERE

Batch authored 2026-07-10 (same session as the three-lane procurement research and the ADR-0057
Research-ratification amendment). W1 DONE. Next task: W2.

## Status

| Task | Title | Status | Commit | Deviations |
| --- | --- | --- | --- | --- |
| W1 | Trust-center skeleton: README index | DONE | (pending) | none |
| W2 | faq.md: the 22-question front door | pending | - | - |
| W3 | security-overview.md + data-flows.md | pending | - | - |
| W4 | sub-processors.md + continuity.md + supply-chain.md | pending | - | - |
| W5 | controls.md + questionnaire.md (CAIQ-shaped) | pending | - | - |
| W6 | support-policy.md + tiers.md + PLAN.md 3/2 sync | pending | - | - |
| W7 | msa.md + dpa.md (DRAFT -- pending counsel) | pending | - | - |
| W8 | SBOM in release CI + security-insights.yml + SECURITY.md alignment | pending | - | - |
| W9 | Red-team pass (over-claims) + cross-links | pending | - | - |

Status values: `pending` | `in-progress` | `DONE` | `BLOCKED`.

## Log

One entry per task as it closes (or blocks). Number every deviation from the task file.

### W1 -- Trust-center skeleton: README index (DONE)
- Wrote `docs/trust/README.md` with pinned H1 + H2s (How to read this, Documents, What we do
  not have), both verbatim sentences, a 12-row document table (14 markdown links total), and the
  footer.
- Verification: gated sentence 1 hit; `]\(` count 14 (>=12); em-dash 0; "open source" 0; footer
  present. Global gates clean.
- Deviations: none.
