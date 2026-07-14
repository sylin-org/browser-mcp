# LEDGER: closed-loop browser core (ADR-0078)

Durable progress. One task equals one commit. Update this file before and after each task.

## RESUME HERE

- ADR-0078 is accepted. The implementation batch is authored but no production code has changed.
- Start with C1, `C1-actionable-observations.md`.
- Re-read the live registry, pipeline, result, audit, and extension observation seams before editing.
- Cross-origin frame refs are out of scope and require a separate ADR.

## Task log

| Task | Commit | Status | Notes |
|------|--------|--------|-------|
| C1 actionable observations | -- | READY | Shared summary and ranked matcher |
| C2 interaction receipts | -- | READY | Bounded observation and recovery vocabulary |
| C3 act_on | -- | READY | Additive semantic interaction |
| C4 output provenance | -- | READY | Session nonce and page-text boundaries |
| C5 dialog control | -- | READY | Explicit dialog status and resolution |
| C6 tab control | -- | READY | Explicit owned-tab focus/reload/close |

## Batch checks

| Check | Status | Evidence |
|-------|--------|----------|
| Rust format, clippy, workspace tests | NOT RUN | -- |
| Extension syntax and tests | NOT RUN | -- |
| Lightbox all scenarios | NOT RUN | -- |
| Visible-browser verification | NOT RUN | See `LIVE-VERIFY.md` |
| Tool count and public docs synchronized | NOT RUN | -- |

## Deviations

None.
