# T2 -- relay down-classifier (ADR-0047 D6)

## Goal

The adapter's service-to-client relay direction classifies a SERVICE-side read error as
`ServiceClosed` (reconnect) instead of `ClientClosed` (exit), so an abrupt service death on
Windows (ERROR_BROKEN_PIPE on the read) never forces an MCP-client reload. Normative: ADR-0047
D6, amending ADR-0045. Oracles: PINS.md P2.

## Files this task owns (touch nothing else)

- `crates/transport/src/ipc.rs`
- `docs/adr/0045-resilient-reconnecting-adapter.md` (APPEND-only amendment)
- `docs/tasks/tab-identity/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ c49ee6d -- re-read before editing)

- `crates/transport/src/ipc.rs` contains `async fn relay_session<R, W, CO>(` whose `down` arm is:

```rust
    let down = async {
        match tokio::io::copy(ipc_read, client_out).await {
            Ok(_) => RelaySide::ServiceClosed, // service EOF
            Err(_) => RelaySide::ClientClosed, // writing to the client failed
        }
    };
```

- `RelaySide` is a two-variant enum (`ClientClosed`, `ServiceClosed`) in the same file.
- The file ends with (or contains) a `#[cfg(test)] mod tests` module. If it does NOT contain
  one, ADD `#[cfg(test)] mod tests { use super::*; ... }` at the end of the file (this is the
  one sanctioned structural addition).
- `docs/adr/0045-resilient-reconnecting-adapter.md` already ends with an
  `## Amendment (2026-07-08, pre-implementation of the split batch)` section.

## STOP preconditions

- STOP if the `down` arm's text differs from the block quoted above.
- STOP if a fn named `copy_service_to_client` already exists anywhere in the workspace
  (`grep -rn "copy_service_to_client" crates/ src/`).

## Changes (transcribe from PINS P2)

1. Add `copy_service_to_client` exactly as pinned (doc comment included); replace the `down` arm
   with `let down = copy_service_to_client(ipc_read, client_out);`.
2. Add the three pinned unit tests (`down_eof_classifies_service_closed`,
   `down_read_error_classifies_service_closed`,
   `down_client_write_error_classifies_client_closed`) with the pinned FailingReader /
   FailingWriter helpers local to the tests module. Each test is `#[tokio::test]`.
3. APPEND the pinned amendment section to ADR-0045 (never edit existing lines).

## Verification (all green)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ghostlight-transport --no-fail-fast
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

The two existing integration tests in `tests/adapter_reconnect.rs` MUST stay green (they prove
the reconnect path still works end-to-end with the new classifier).

## Out of scope (fences)

- NO change to `relay_session`'s `up` arm, `HandshakePreamble`, `relay_adapter`,
  `connect_and_handshake`, or `try_connect_once` (T3 owns the latter two).
- NO guid changes (T3).

## Commit

Stage exactly `crates/transport/src/ipc.rs` + the ADR-0045 file. Pinned message (PINS P2):

```
fix(transport): classify service-side read errors as reconnect, not client exit (ADR-0047 D6)
```

Then update LEDGER.md and commit as `docs(tab-identity): ledger T2`.
