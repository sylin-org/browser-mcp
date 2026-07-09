# T1 -- managed-surface predicate (ADR-0047 D1)

## Goal

The extension's tool gate recognizes a tab as in-surface when it sits in ANY Ghostlight-managed
group (the legacy global group OR any per-session group in `sessionGroups`), via a pure,
unit-tested predicate in `extension/lib/grouping.js`. This kills the e2e F4 failure ("Tab N is
not in the ... group" on a tab that is visibly grouped) at its root. Normative: ADR-0047 D1.
Oracles: PINS.md P1.

## Files this task owns (touch nothing else)

- `extension/lib/grouping.js`
- `extension/service-worker.js`
- `tests/extension/grouping.test.js`
- `docs/tasks/tab-identity/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ c49ee6d -- re-read each anchor before editing)

- `extension/lib/grouping.js` is an IIFE exporting
  `const GhostlightGrouping = { groupSessionTabs };` with both a `module.exports` and a
  `self.GhostlightGrouping` branch.
- `extension/service-worker.js` contains, near the top:
  `const { groupSessionTabs } = self.GhostlightGrouping;`
- `service-worker.js` contains these anchors (approx lines 593-611):
  - `async function groupTabs() {` with body
    `return groupId === null ? [] : chrome.tabs.query({ groupId });`
  - `async function inGroup(tabId) {` ending its try block with
    `return tab.groupId === groupId;`
- Above `const sessionGroups = new Map();` sits a comment block whose last lines read
  "so that check cannot become session-aware. `sessionGroups` backs ONLY the group_request".
- `tests/extension/grouping.test.js` requires `../../extension/lib/grouping.js` and uses
  `node:test` + `node:assert`.

## STOP preconditions

- STOP if any anchor above is absent or materially different.
- STOP if `GhostlightGrouping` already exports a `managedGroupIds` or `isManagedGroupId` symbol.

## Changes (transcribe from PINS P1)

1. Add `managedGroupIds` + `isManagedGroupId` to `grouping.js`, exactly as pinned; extend the
   export object. Also rewrite the stale sentence in `grouping.js`'s module header comment that
   claims the module is "ADDITIVE to (never a replacement of) the existing single-group ...
   mechanism ... which this module does not touch or call" -- after this task the gate CONSULTS
   this module's predicate; state that and cite ADR-0047 D1 (ADR-0047's Context records why the
   old claim was false at the Chrome API level).
2. Rewire `service-worker.js`: the destructure line, `inGroup`'s final membership line,
   `groupTabs`'s body, and the stale `sessionGroups` comment -- all exactly as pinned in P1.
   Nothing else in the worker changes (error strings stay; `effectiveTabId` stays; `ensureGroup`
   stays).
3. Append the two pinned tests to `tests/extension/grouping.test.js`:
   `managed_surface_accepts_global_and_session_groups`,
   `managed_surface_rejects_foreign_and_ungrouped`.

## Verification (all green)

```
node --check extension/service-worker.js
node --check extension/lib/grouping.js
node --test tests/extension/grouping.test.js
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

(The cargo commands are unaffected by this task but pin that the tree stays green.)

## Out of scope (fences)

- NO change to error message strings (T5 owns them).
- NO change to `tabs_create_mcp` / `tabs_context_mcp` handlers (T4 owns them).
- NO change to `groupSessionTabs`, `persistSessionState`, `rehydrate`, `ensureGroup`.
- NO Rust changes.

## Commit

Stage exactly the three extension/test files. Pinned message (PINS P1):

```
fix(extension): managed-surface tab gate -- recognize every Ghostlight-managed group (ADR-0047 D1)
```

Then update LEDGER.md (status, hash, deviations) and commit as
`docs(tab-identity): ledger T1`.
