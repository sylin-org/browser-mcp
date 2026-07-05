# Ghostlight Console batch: LEDGER

Durable progress for the Console batch (ADR-0030 Decision 9). One task = one commit (landed as a
`feat(console): K<N> ...` code commit followed by a separate `docs(console): record K<N> commit
hash` ledger-update commit, per BOOTSTRAP.md's Environment facts). Update this file at the end of
every task, per BOOTSTRAP.md step 8. This is the single source of truth for "where are we"; a
fresh executor resumes from RESUME HERE with no other context.

## RESUME HERE

**K1 is NEXT (`K1-config-session-accessors.md`).** No task in this batch has started. Read
`docs/tasks/console/BOOTSTRAP.md` in full, then this file's Status table below (all rows
`pending`), then `K1-config-session-accessors.md` and the PINS.md sections it cites (CS6, CS7,
CS8, CS8.1, CS9). Follow the per-task procedure in BOOTSTRAP.md exactly.

## Status

| Task | Title | Status | Commit | Notes |
| --- | --- | --- | --- | --- |
| K1 | Config + session read accessors; shared config-write function | pending | -- | no HTTP, no UI; PINS.md CS6-CS9 |
| K2 | Console static GET routes in src/hub/webapi.rs | pending | -- | needs K1; PINS.md CS1, CS10, CS11 |
| K3 | GET /api/v1/config + config table UI | pending | -- | needs K2; PINS.md CS2 |
| K4 | GET /api/v1/sessions + sessions UI | pending | -- | needs K2; PINS.md CS3 |
| K5 | POST /api/v1/config/webapi-enable-remote + UI control | pending | -- | needs K1+K2; PINS.md CS4, CS5 |

Status values: `pending` | `in-progress` | `DONE` | `BLOCKED`.

## Log

One entry per task as it closes (or blocks). Number every deviation from the task file.

(No tasks have run yet.)

## Deviation format

When you deviate from a task file (a signature differs from as-of-authoring, a helper had to move,
an oracle needed pinning), record it under that task as:

```
D<n>: <what the task said> -> <what you actually did> because <the tree fact that forced it>.
     Impact on later tasks: <none | names the task + what it must now assume>.
```

A BLOCKED entry records instead: the failed assumption (with the file/symbol actually found), the
STOP precondition or fence that triggered, and what is needed to proceed. Then HALT.
