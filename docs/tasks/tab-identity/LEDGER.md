# tab-identity batch -- LEDGER

Durable execution record. One task = one code commit + one ledger commit. Update after EVERY
task (or block); this file is the single source of truth for batch progress.

## RESUME HERE

Next task: **T4** (`T4-session-scoped-tab-operations.md`). Base: T3 landed at `fb88795`.

## Task table

| Task | Status | Code commit | Notes |
|---|---|---|---|
| T1 managed-surface predicate | done | 31049f2 | |
| T2 down-classifier | done | 293dfd1 | |
| T3 stable session guid | done | fb88795 | build-order note (deviation 1) |
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

### T2 -- relay down-classifier (ADR-0047 D6) -- DONE

- Code commit: `293dfd1`.
- STOP preconditions: both passed. The `down` arm text matched the quoted block verbatim
  (`let down = async { match tokio::io::copy(ipc_read, client_out).await { ... } }`);
  `grep -rn "copy_service_to_client" crates/ src/` returned nothing. `RelaySide` confirmed a
  two-variant enum; a `#[cfg(test)] mod tests` already existed (no structural addition needed).
- Changes exactly per PINS P2: added the private `copy_service_to_client` async fn (doc comment
  verbatim) right after `relay_session`; replaced the `down` arm with
  `let down = copy_service_to_client(ipc_read, client_out);`; appended the three pinned
  `#[tokio::test]`s with local `FailingReader`/`FailingWriter`; APPENDED the ADR-0045 amendment
  section `## Amendment (2026-07-08, ADR-0047 D6): down-relay error classification` (existing
  lines untouched).
- Test-assertion mechanism: used `assert!(matches!(..., RelaySide::ServiceClosed))` /
  `RelaySide::ClientClosed` rather than `assert_eq!`, since `RelaySide` derives neither
  `PartialEq` nor `Debug` and the task fences forbid unrelated changes; `matches!` satisfies the
  pinned "returns RelaySide::X" assertion without touching the enum. (Judgment-free: the pin
  states the expected variant, not the macro.)
- Verification (all green): `cargo fmt --check` OK; `cargo clippy --workspace --all-targets --
  -D warnings` exit 0 (verified via exit code, not just tail); `cargo test -p
  ghostlight-transport` = 60 passed incl. the 3 new; `cargo test --workspace --no-fail-fast` = 43
  `test result: ok`, 0 failed, `adapter_reconnects_across_a_service_restart_without_a_client_reload`
  green; `cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets` OK. Both files
  pure ASCII.
- Deviations from task/PINS: NONE beyond the `matches!` choice noted above (a transcription
  choice, not a semantic deviation).

### T3 -- stable per-process SessionGuid (ADR-0047 D2) -- DONE

- Code commit: `fb88795`.
- STOP preconditions: all passed. Every anchor present; `grep "fn adapter_hello"` empty;
  `spawn_adapter` did NOT set `GHOSTLIGHT_DEBUG` (the line-60 `GHOSTLIGHT_DEBUG` belongs to
  `service_cmd`), so per the task's sanctioned edit I added `.env("GHOSTLIGHT_DEBUG", "1")` to
  `spawn_adapter`. Confirmed constants for the pinned test: `HUB_PROTO: u32 = 1`,
  `ROLE_ADAPTER = "adapter"`.
- Changes exactly per PINS P3: extracted `adapter_hello(guid)`; `try_connect_once` gained the
  `guid` param and dropped its local mint; `connect_and_handshake` gained the `guid` param and
  threads it to both call sites; `relay_adapter` mints ONE guid before the loop, emits the pinned
  note, and passes `&session_guid` into `connect_and_handshake`; rewrote the two stale
  doc-comment passages to cite ADR-0047 D2; added `hello_carries_the_caller_guid`; extended the
  restart integration test (mint-note count == 1, reconnect-note count >= 1) leaving the 5s-gap
  test untouched; APPENDED the ADR-0045 D2 amendment.
- DEVIATION 1 (verification-recipe gap, worked around; NOT a code change): the pinned T3
  verification lists `cargo test --test adapter_reconnect` and `cargo test --workspace` but NONE
  of the pinned commands rebuild the DELIVERABLE `target/debug/ghostlight-adapter-agent.exe` that
  `adapter_bin()` spawns by PATH (it is not referenced via `CARGO_BIN_EXE_*`, unlike the
  `ghostlight` bin). `cargo test --workspace` builds each crate's TEST harness, not the sibling
  deliverable bin, so the reconnect test first ran a stale (pre-T3) adapter and my new mint-note
  assertion failed (observed left:0 right:1; the surviving log_dir's adapter events file had the
  old notes but not the mint note; the on-disk exe was timestamped 16:10 and did not embed the
  new string). Fix: ran `cargo build --workspace` to refresh the deliverable bins, after which
  `cargo test --test adapter_reconnect` = 2 passed and `cargo test --workspace` = all green. The
  code is correct as pinned; only an extra `cargo build --workspace` step is needed before the
  reconnect test. RECOMMENDATION for the batch author: add `cargo build --workspace` to the T3
  verification block ahead of the reconnect test.
- Verification (all green after the build step): `cargo fmt --check` OK; clippy exit 0;
  `cargo test -p ghostlight-transport` = 61 passed incl. `hello_carries_the_caller_guid`;
  `cargo test --test adapter_reconnect` = 2 passed (mint-note + reconnect assertions live);
  `cargo test --workspace --no-fail-fast` = 43 `test result: ok`, 0 failed;
  `cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets` OK. All three files
  pure ASCII.
