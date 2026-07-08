# T5 -- client-name titles + recovery-steering errors (ADR-0047 D4)

## Goal

Per-session Chrome group titles come from the MCP client's name (`\u{1F47B} Claude Code`, deduped
`(2)`, fallback `\u{1F47B} Ghostlight`) instead of a truncated guid, and the extension's tab
errors steer the agent to the cheap recovery (`tabs_context_mcp`) instead of into making litter.
Normative: ADR-0047 D4 (supersedes the hub batch's SS6 title pin). Oracles: PINS.md P5.

## Files this task owns (touch nothing else)

- `crates/core/src/hub/session.rs`
- `crates/core/src/hub/mod.rs` (ServiceContext field)
- `crates/core/src/mcp/server.rs` (emit_group_request + its callers)
- `tests/hub_isolation.rs` (ONE line: the new ServiceContext field initializer; PINS P5)
- `tests/hub_queue.rs` (ONE line: same)
- `extension/service-worker.js` (three error strings ONLY)
- `docs/tasks/tab-identity/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ c49ee6d, T1-T4 landed -- re-read before editing)

- `session.rs` has `pub fn group_title(guid: &SessionGuid) -> String` returning
  `format!("\u{1F47B} Ghostlight {}", &guid.as_str()[..8])`, and the test
  `group_title_matches_the_pinned_format`.
- `hub/mod.rs` `pub struct ServiceContext` fields end with
  `live_sessions`, `debug_sink` (plus `session_titles` does NOT exist yet).
- `server.rs` `emit_group_request` builds `let title = crate::hub::session::group_title(guid);`.
  After T4 it has exactly two callers: `check_tab_ownership`'s Adopted arm and the tabs_create
  response claim in the tools/call spawn.
- `Governance` exposes `current_client()` (used in server.rs's policy-subscription task:
  `let client = outgoing.current_client();`). VERIFY its return type exposes a public `name`
  string (locate the fn with `grep -rn "fn current_client" crates/core/src/governance/` and
  read its signature + the returned struct).
- Extension error-string anchors (all inside `effectiveTabId`):
  - `is not in the ${GROUP_TITLE} group. The group has no tabs`
  - `is not in the ${GROUP_TITLE} group. Valid tab IDs are:`
  - `No tabs in the ${GROUP_TITLE} group. Use tabs_create_mcp`

## STOP preconditions

- STOP if `current_client()`'s returned type does not expose a usable client name string
  (verified at authoring: `Option<ClientInfo>` with `pub name: String`).
- STOP if `grep -rn --exclude-dir=node_modules "is not in the" tests/ crates/ src/` matches
  anything (a test pinning the OLD error strings would break silently; as of authoring the only
  matches under tests/ are vendored playwright files inside `tests/e2e/node_modules/`, which
  the exclude removes -- re-verify with the exclude flag exactly as written).
- STOP if `serve_session` does not destructure `ServiceContext` field-by-field (the new field
  must be explicitly bound, not swallowed).
- STOP if `tests/hub_isolation.rs` / `tests/hub_queue.rs` no longer build `ServiceContext`
  struct literals (the one-line addition would then be wrong).

## Changes (transcribe from PINS P5)

1. `session.rs`: delete `group_title` + its test; add `session_title` + the pinned test
   `session_title_uses_client_name_with_dedupe_and_fallback`.
2. `hub/mod.rs`: add `session_titles` to `ServiceContext` + `from_startup` init; add the pinned
   one-line initializer to the two test struct literals (P5).
3. `server.rs`: `emit_group_request` gains `titles` + `governance` parameters and builds the
   title via `session_title(...)` (PINS P5); thread at both callers; `serve_session` binds the
   new field in its destructure and includes it in the seat/spawn clones as needed.
4. `extension/service-worker.js`: replace the three pinned error strings (P5) verbatim. The
   legacy `tabsContextLegacy` body's string stays untouched.

## Verification (all green)

```
node --check extension/service-worker.js
node --test tests/extension/grouping.test.js
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

`cargo test -p ghostlight-core session_title` must show the new test passing;
`grep -rn "group_title" crates/ src/ tests/` must return ZERO matches after the change.

## Out of scope (fences)

- NO changes to `docs/tasks/hub/PINS.md` (history; ADR-0047 carries the supersession).
- NO liveness changes (T6).
- NO other extension string changes; NO handler changes.
- The ghost glyph stays the `\u{1F47B}` escape in both languages.

## Commit

Stage exactly the six named source files. Pinned message (PINS P5):

```
feat(session): client-name tab-group titles + recovery-steering tab errors (ADR-0047 D4)
```

Then update LEDGER.md and commit as `docs(tab-identity): ledger T5`.
