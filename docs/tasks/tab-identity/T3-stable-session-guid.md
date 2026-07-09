# T3 -- stable per-process SessionGuid (ADR-0047 D2)

## Goal

`ghostlight-adapter-agent` mints its `SessionGuid` once per process and re-presents the SAME
guid on every reconnect, so the service-side ownership map and the extension's persisted
`sessionGroups` map keep working across a service restart. Normative: ADR-0047 D2 (supersedes
the "fresh guid per (re)connect ... exactly right" posture). Oracles: PINS.md P3.

## Files this task owns (touch nothing else)

- `crates/transport/src/ipc.rs`
- `tests/adapter_reconnect.rs`
- `docs/adr/0045-resilient-reconnecting-adapter.md` (APPEND-only amendment)
- `docs/tasks/tab-identity/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ c49ee6d -- re-read before editing)

- `crates/transport/src/ipc.rs`:
  - `pub async fn relay_adapter(endpoint: &str, debug: &crate::observability::DebugSink)` holds
    the reconnect loop; its doc comment contains the sentence beginning "A fresh `SessionGuid`
    is minted per (re)connect".
  - `async fn try_connect_once(adapter_endpoint: &str)` contains
    `let guid = crate::session_guid::SessionGuid::mint();` and builds the hello via
    `json!({ "hub": ..., "role": ..., "guid": guid.as_str() })`.
  - `connect_and_handshake(adapter_endpoint: &str, reconnect: bool)` calls `try_connect_once`
    twice (fast path + retry loop).
  - Debug notes in `relay_adapter` use `debug.ipc_note("...")`.
- `tests/adapter_reconnect.rs` contains
  `adapter_reconnects_across_a_service_restart_without_a_client_reload`, which builds a
  per-run `log_dir`; the adapter runs with `GHOSTLIGHT_DEBUG=1` inherited? -- NO: check the
  `spawn_adapter` fn. STOP precondition below covers it.

## STOP preconditions

- STOP if any anchor above is absent.
- Check `spawn_adapter` in `tests/adapter_reconnect.rs`: the adapter child must run with debug
  events enabled and writing into `log_dir` (it already sets `GHOSTLIGHT_LOG_DIR`; it must ALSO
  set `GHOSTLIGHT_DEBUG=1` -- if it does not, ADD `.env("GHOSTLIGHT_DEBUG", "1")` to
  `spawn_adapter` as part of this task; this is the one sanctioned edit beyond the assertion
  block).
- STOP if `ipc.rs` already has a fn named `adapter_hello`.

## Changes (transcribe from PINS P3)

1. Extract `fn adapter_hello(guid: &crate::session_guid::SessionGuid) -> serde_json::Value`
   (private, same file) building the exact existing hello JSON; `try_connect_once` gains the
   `guid` parameter and uses `adapter_hello(guid)`.
2. `connect_and_handshake` gains the `guid` parameter, threads it to both call sites.
3. `relay_adapter` mints once before the loop and passes `&session_guid` into
   `connect_and_handshake`; add the single pinned debug note
   `"session identity minted (stable for this adapter process)"` BEFORE the loop.
4. Rewrite the two stale doc-comment passages exactly as directed in P3 (cite ADR-0047 D2).
5. Add unit test `hello_carries_the_caller_guid` (PINS P3).
6. Extend the restart integration test with the pinned debug-events assertions (mint-note count
   == 1 across all `debug-events-*.jsonl` in `log_dir`; reconnect-note count >= 1).
7. APPEND the pinned ADR-0045 amendment section.

## Verification (all green)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ghostlight-transport --no-fail-fast
cargo test --test adapter_reconnect --no-fail-fast
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

## Out of scope (fences)

- NO change to the hello WIRE SHAPE (`{hub, role, guid}` -- same keys, same values shape).
- NO change to `verify_service_proof`, `dial_once`, the retry windows, or `relay_session`.
- NO service-side (core) changes: `SessionRegistry::admit` already sanctions same-user
  re-presentation; if you believe a core change is needed, that belief is wrong per ADR-0047 D2
  -- BLOCK and record instead of editing core.

## Commit

Stage exactly the three named files. Pinned message (PINS P3):

```
feat(transport): stable per-process session guid -- reconnects resume identity (ADR-0047 D2)
```

Then update LEDGER.md and commit as `docs(tab-identity): ledger T3`.
