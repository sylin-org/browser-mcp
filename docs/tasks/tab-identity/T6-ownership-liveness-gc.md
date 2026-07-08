# T6 -- ownership liveness + group-map pruning + changelog (ADR-0047 D5)

## Goal

A tab owned by a session with NO live connection is adoptable by another session (dead-owner
reassignment, no timers), and the extension prunes `sessionGroups` entries whose Chrome group
died. Plus the CHANGELOG entry for the whole ADR-0047 batch. Normative: ADR-0047 D5. Oracles:
PINS.md P6.

## Files this task owns (touch nothing else)

- `crates/core/src/hub/session.rs`
- `crates/core/src/hub/mod.rs` (ServiceContext field)
- `crates/core/src/mcp/server.rs` (LiveGuidGuard + SessionSeat third field + gate switch)
- `crates/core/src/mcp/pipeline.rs` (the audit-test seat gains the third field; PINS P6)
- `tests/hub_isolation.rs` (ONE line: the new ServiceContext field initializer; PINS P6)
- `tests/hub_queue.rs` (ONE line: same)
- `extension/lib/grouping.js` (pruneDeadGroups)
- `extension/service-worker.js` (rehydrate call)
- `tests/extension/grouping.test.js` (require line + the pinned inline-fake test)
- `CHANGELOG.md`
- `docs/tasks/tab-identity/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ c49ee6d, T1-T5 landed -- re-read before editing)

- `session.rs` has `claim_tab` (three-arm match) and, from T4/T5, the response-claim caller in
  `server.rs` plus `check_tab_ownership`'s claim call.
- `server.rs` has `struct LiveSessionGuard(Arc<AtomicUsize>);` with `new` + `Drop`, constructed
  in `serve_session` as `let _live_guard = LiveSessionGuard::new(live_sessions);`.
- `rehydrate()` in `service-worker.js` restores `sessionGroupsState` with the loop anchor
  `for (const [guid, gid] of stored.sessionGroupsState) sessionGroups.set(guid, gid);`.
- `CHANGELOG.md` has an Unreleased (or newest-version) section at the top -- read its exact
  heading style before editing.

## STOP preconditions

- STOP if `claim_tab_live` or `LiveGuidGuard` or `pruneDeadGroups` already exist anywhere.
- STOP if `serve_session` no longer constructs `LiveSessionGuard` (anchor drift).

## Changes (transcribe from PINS P6)

1. `hub/mod.rs`: add `live_guids` to `ServiceContext` + init; add the pinned one-line
   initializer to the two test struct literals (P6).
2. `server.rs`: add `LiveGuidGuard` (pinned) beside `LiveSessionGuard`; construct it in
   `serve_session` right after `_live_guard` (bind the new context field explicitly in the
   destructure). Thread liveness by the ONE pinned mechanism (P6 THREADING): `SessionSeat`
   gains `live_guids`; `check_tab_ownership` gains a `live_guids` parameter; both gate sites
   switch from `claim_tab` to `claim_tab_live`; the pipeline.rs audit-test seat gains the third
   field.
3. `session.rs`: add `claim_tab_live` (pinned) + the two pinned tests
   (`dead_owner_tab_is_adoptable_by_a_live_session`, `live_owner_tab_stays_refused`).
   `claim_tab` / `owns_or_adopts_tab` and their tests stay UNCHANGED.
4. Extension: add `pruneDeadGroups` to `grouping.js` (pinned, exported); extend the worker's
   destructure line and the test file's require line with `pruneDeadGroups`; call it from
   `rehydrate()` right after the restore loop (pinned one-liner); add the pinned test
   `dead_groups_are_pruned_from_the_session_map` with its pinned INLINE fake (P6; do NOT modify
   the existing `fakeChrome` helper).
5. `CHANGELOG.md`: the top section is `## [0.3.0] - 2026-07-07` (already released; there is no
   Unreleased heading). CREATE a new `## [Unreleased]` section ABOVE it, matching the file's
   existing heading style, and place this block inside it:

```
### Fixed
- Tab tools no longer refuse tabs that sit in a per-session Ghostlight group: the extension's
  gate now recognizes every Ghostlight-managed group (ADR-0047 D1; the e2e F4 desync).
- A service-side read error in the agent adapter reconnects instead of exiting, so an abrupt
  service death never forces an MCP-client reload (ADR-0047 D6).

### Changed
- Session identity is stable across reconnects: the agent adapter re-presents one guid per
  process, so tab ownership and the session's Chrome tab group survive a service restart
  (ADR-0047 D2).
- New tabs are born directly in the calling session's tab group (no more about:blank bootstrap
  litter), and tabs_context_mcp reports that session's group (ADR-0047 D3).
- Tab groups are titled by the MCP client's name (for example "<ghost> Claude Code"), deduped
  across sessions, instead of a truncated session id (ADR-0047 D4).
- A tab owned by a session that is no longer connected can be adopted by a live session, and
  dead group-map entries are pruned on service-worker restart (ADR-0047 D5).
```

   (Replace `<ghost>` with the actual glyph escape rendering note as the CHANGELOG's existing
   style does for emoji -- if the CHANGELOG contains no emoji precedent, write the words
   "the ghost glyph followed by" instead; NEVER paste the literal emoji.)

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

Final batch check (BOOTSTRAP completion criteria):
`git diff --name-only <base>..HEAD` contains NO file from the BOOTSTRAP NEVER list.

## Out of scope (fences)

- NO owned_tabs garbage collection beyond dead-owner reassignment (no timers, no sweeps).
- NO changes to `groupSessionTabs`, the gate fns, or handlers.
- NO changes to mint quota, `SessionRegistry`, or idle-grace.

## Commit

Stage exactly the ten named files. Pinned message (PINS P6):

```
feat(session): ownership liveness -- dead-owner adoption + group-map pruning (ADR-0047 D5)
```

Then update LEDGER.md (RESUME HERE -> COMPLETE + the batch-complete note) and commit as
`docs(tab-identity): ledger T6`.
