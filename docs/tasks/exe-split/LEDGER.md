# exe-split LEDGER

Durable batch progress. One task = one CODE commit + one ledger commit = one log entry
(BOOTSTRAP per-task procedure). Update after EVERY task, before starting the next.

## RESUME HERE

- Next task: **S3** (`S3-move-wire-and-handshake.md`)
- Base commit: `fccca60` on `dev` (tree green at batch authoring; later docs-only commits carry
  the batch itself)
- Batch state: IN PROGRESS (S1, S2 complete)

## Task table

| Task | Title | Status | Commit |
|---|---|---|---|
| S1 | Workspace + transport crate skeleton | done | 14a8bd0 |
| S2 | Move leaf utilities to transport | done | bbb02da |
| S3 | Move wire + handshake to transport | pending | - |
| S4 | Create ghostlight-core; root becomes facade | pending | - |
| S5 | ghostlight-adapter-agent bin + rewire clients + test harness | pending | - |
| S6 | ghostlight-adapter-browser bin + host install rework | pending | - |
| S7 | Retire roles from the ghostlight bin | pending | - |
| S8 | Reconnect patience (120s) + ADR-0045 amendment | pending | - |
| S9 | --no-supervisor + DEV-LOOP.md | pending | - |
| S10 | Packaging + distribution sweep | pending | - |

## Log

(Append one entry per finished task:)

```
### S<n> -- <title>
- Commit: <hash>
- Verification: fmt OK / clippy OK / test --workspace OK / linux cross-check OK
- Deviations:
  1. <none | numbered list, one line each>
```

### S1 -- Workspace + transport crate skeleton
- Commit: 14a8bd0
- Verification: fmt OK / clippy OK / test --workspace OK (524 root unit + full integration suite pass; new ghostlight-transport crate builds, 0 tests) / linux cross-check OK
- Deviations:
  1. none. (Git reported routine CRLF->LF normalization on Cargo.toml; committed blobs are LF per repo convention -- no content or requirement change.)

### S2 -- Move leaf utilities to transport
- Commit: bbb02da
- Verification: fmt OK / clippy OK / test --workspace OK (full suite green; ghostlight-transport now runs 36 moved unit tests, 0 failed) / linux cross-check OK
- Deviations:
  1. Promoted three observability fns from `pub(crate)` to `pub` -- `now_ms`, `fmt_ms`, `session_state_files`. Reason: `src/hub/manage/doctor.rs` (root crate) calls all three, and once observability moved to ghostlight-transport the `pub(crate)` visibility scoped them to transport, breaking the cross-crate calls. SPEC section 2 sanctions exactly this ("items that were `pub(crate)` or private and are now needed across the crate boundary become `pub`"). No behavior change; not a governance/tool-surface item.
  2. watchdog.rs keeps a now-dangling rustdoc intra-doc link `[`crate::main`]` (the lib-only transport crate has no `main`). Left unmodified per the mechanical-move rule (not in the SPEC section 2 rewrite list). Harmless to SPEC-12 verification: intra-doc links are a rustdoc lint, not built by clippy/test/check, and CI runs no `cargo doc`.

## Blocked

(Only if the failure protocol fired: task id, exact failing step/error text, one-paragraph
diagnosis. The batch HALTS here.)
