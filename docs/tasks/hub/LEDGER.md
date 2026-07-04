# Ghostlight Hub batch: LEDGER

Durable progress for the Hub batch (ADR-0030). One task = one commit. Update this file at the end of
every task, per BOOTSTRAP step 8. This is the single source of truth for "where are we"; a fresh
executor resumes from RESUME HERE with no other context.

## RESUME HERE

**Next task: H0 (`H0-extract-hubcore.md`).**
Nothing landed yet. Start at H0, follow the per-task procedure in `BOOTSTRAP.md`.

## Status

| Task | Title | Status | Commit | Notes |
| --- | --- | --- | --- | --- |
| H0 | Extract the HubCore composition root | pending | -- | |
| H1 | Transport-generic serve_session + ServiceContext | pending | -- | |
| H2 | Persistent service + thin adapter + multiplex | pending | -- | the one large coupled commit |
| H3 | Adapter-minted GUID identity + peer-cred binding | pending | -- | |
| H4 | Binary-authoritative cross-session tab isolation | pending | -- | |
| H5 | Reconnect grace window + honest bounded queue | pending | -- | orthogonal after H2 |
| H6 | Detached non-admin lifecycle + anti-squat | pending | -- | job-breakaway is the acceptance gate |
| H7 | Tab-group-per-session presentation | pending | -- | crosses the JS boundary |
| H8 | Local web API = TCP; bind per policy | pending | -- | needs H2+H3+H4; the corrected D2/D5 |

Status values: `pending` | `in-progress` | `DONE` | `BLOCKED`.

## Log

One entry per task as it closes (or blocks). Number every deviation from the task file.

### H0
- (not started)

### H1
- (not started)

### H2
- (not started)

### H3
- (not started)

### H4
- (not started)

### H5
- (not started)

### H6
- (not started)

### H7
- (not started)

### H8
- (not started)

## Deviation format

When you deviate from a task file (a signature differs from as-of-authoring, a helper had to move,
an oracle needed pinning), record it under that task as:

```
D<n>: <what the task said> -> <what you actually did> because <the tree fact that forced it>.
     Impact on later tasks: <none | names the task + what it must now assume>.
```

A BLOCKED entry records instead: the failed assumption (with the file/symbol actually found), the
STOP precondition or fence that triggered, and what is needed to proceed. Then HALT.
