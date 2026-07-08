# tab-identity batch -- LEDGER

Durable execution record. One task = one code commit + one ledger commit. Update after EVERY
task (or block); this file is the single source of truth for batch progress.

## RESUME HERE

Next task: **T2** (`T2-transport-down-classifier.md`). Base: T1 landed at `31049f2`.

## Task table

| Task | Status | Code commit | Notes |
|---|---|---|---|
| T1 managed-surface predicate | done | 31049f2 | |
| T2 down-classifier | pending | - | |
| T3 stable session guid | pending | - | |
| T4 envelope guid + session ops | pending | - | |
| T5 client-name titles + errors | pending | - | |
| T6 liveness + pruning + changelog | pending | - | |

## Per-task log

(Append one entry per task: commit hash, verification results, and EVERY deviation from the task
file/PINS, numbered. A BLOCKED entry carries the failed precondition or error text verbatim and
your reasoning, then the batch HALTS per BOOTSTRAP.)

### T1 -- managed-surface predicate (ADR-0047 D1) -- DONE

- Code commit: `31049f2`.
- STOP preconditions: both passed (all anchors present verbatim; `GhostlightGrouping` did not yet
  export `managedGroupIds`/`isManagedGroupId` -- grep found no matches anywhere under extension/).
- Changes made exactly per PINS P1: added `managedGroupIds` + `isManagedGroupId` pure fns and
  extended the export object in `grouping.js`; rewrote the stale "additive/never touched" header
  claim and the stale `sessionGroups` comment to cite ADR-0047 D1; rewired `service-worker.js`
  destructure line, `groupTabs` body (union over managed ids), and `inGroup`'s final membership
  line to `isManagedGroupId(...)`; require line + two pinned tests appended to the test file.
- Verification (V-ALL, all green): `node --check` on both JS files OK; `node --test
  tests/extension/grouping.test.js` = 3 pass (the 2 new + the pre-existing); `cargo fmt --check`
  OK; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace
  --no-fail-fast` = 43 suites `test result: ok`, 0 failed; `cargo check --target
  x86_64-unknown-linux-gnu --workspace --all-targets` OK. All three edited files verified pure
  ASCII.
- Deviations from task/PINS: NONE.
- Note (not a deviation): git emitted the usual "CRLF will be replaced by LF" advisory for
  `service-worker.js` -- a pre-existing repo line-ending condition, no content impact.
