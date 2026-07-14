# LEDGER: closed-loop browser core (ADR-0078)

Durable progress. One task equals one commit. Update this file before and after each task.

## RESUME HERE

- ADR-0078 is accepted and C1-C4 are complete.
- Start with C5, `C5-dialog-control.md`.
- Re-read the live session creation, registry, pipeline, result, and page-output seams before editing.
- Cross-origin frame refs are out of scope and require a separate ADR.

## Task log

| Task | Commit | Status | Notes |
|------|--------|--------|-------|
| C1 actionable observations | a5a2391 | DONE | Shared summary, ranked matcher, structured secret redaction; all gates green |
| C2 interaction receipts | 50d87e2 | DONE | Bounded observed-after receipt, target assurance, dialog blocker; all gates green |
| C3 act_on | 9c2901b | DONE | Semantic targeting, dynamic RAWX, bounded recovery, adaptive wait, minimized audit; all gates green |
| C4 output provenance | this commit | DONE | Session nonce, page-text boundaries, structured provenance, and final service-side budgets; all gates green |
| C5 dialog control | -- | READY | Explicit dialog status and resolution |
| C6 tab control | -- | READY | Explicit owned-tab focus/reload/close |

## Batch checks

| Check | Status | Evidence |
|-------|--------|----------|
| Rust format, clippy, workspace tests | PASS (C1-C4) | 653 core unit tests plus workspace integration/doc tests |
| Extension syntax and tests | PASS (C1-C4) | 81 Node tests; changed JS passes `node --check` |
| Lightbox all scenarios | NOT RUN | -- |
| Visible-browser verification | NOT RUN | See `LIVE-VERIFY.md` |
| Tool count and public docs synchronized | NOT RUN | -- |

## Deviations

1. The authored bootstrap said to run `node --test` from `extension/`, but extension tests live in
   `tests/extension/`. C1 corrected the command to the repository's real test location.
