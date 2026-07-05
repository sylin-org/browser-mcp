# onboarding-1 batch: LEDGER

Durable progress for the onboarding-1 batch. One task = one commit. Update this file at the end
of every task. This is the single source of truth for "where are we"; a fresh executor resumes
from RESUME HERE with no other context.

## Baseline

- Branched from `dev` at 4d8f2de, with the ADR-0031 commit (207a7f3 on security-1) cherry-picked
  as the starting point (commit 43ec639 on this branch).
- 584 tests passing, clippy clean, fmt clean, working tree clean.

## RESUME HERE

**o02 (Add agentGuide + per-tool example to tools.json) is NEXT.** o01 landed; the ADR is
reconciled.

## o01 -- Reconcile ADR-0031

Status: DONE (this commit).

Files: `docs/adr/0031-agent-onboarding-contract.md` (Decision 3 rewritten as WITHDRAWN; Decision
4 sharpened to hard-fail with the ToolError discovery note; Consequences updated to match).

What landed: the ADR now matches the design the planning phase converged on. Decision 3 is
withdrawn -- the directory's per-variant description is load-bearing (it feeds `explain_text()`,
the `explain` tool's response body, golden-pinned), not parallel documentation. Decision 4 is
sharpened from "corrective errors" to "hard-fail inputSchema validation with corrective errors,"
with the discovery that the codebase's existing `ToolError` taxonomy already carries a `next_step`
field on every variant -- so Decision 4 USES the existing convention rather than inventing a
parallel mechanism.

Verification: docs-only task; no code or test change. The ADR's section structure (Decisions 1-5
+ Consequences) reads coherently after the rewrites.

## o02 -- Add agentGuide + per-tool example to tools.json

Status: TODO.

## o03 -- Emit initialize.instructions from agentGuide

Status: TODO.

## o04 -- Hard-fail schema validation with corrective errors (flagship)

Status: TODO.

## o05 -- Extend the fidelity test

Status: TODO.

## Deviation format

```
D<n>: <what the plan said> -> <what you actually did> because <the tree fact that forced it>.
     Impact on later tasks: <none | names the task + what it must now assume>.
```
