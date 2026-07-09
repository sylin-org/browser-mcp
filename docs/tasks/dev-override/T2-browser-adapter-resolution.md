# T2 -- browser-adapter candidate resolution (ADR-0048 D4)

## Goal

An UNPINNED `ghostlight-adapter-browser` (the plain sibling binary Chrome launches through the
unified host) picks the first candidate whose endpoint EXISTS (probe-based, so a dead dev never
costs the ~30s connect patience) and falls to the default when all are absent. A
`ghostlight-adapter-browser-<n>` per-instance copy stays pinned via argv[0]. Resolution is per
adapter process, which IS per connect episode (Chrome respawns the host on every reconnect).
Normative: ADR-0048 D4. Oracles: PINS.md P2.

## Files this task owns (touch nothing else)

- `crates/transport/src/ipc.rs` (pick_native_host_endpoint + relay_native_host + tests)
- `crates/adapter-browser/src/main.rs` (resolve_selection + candidate threading)
- `docs/tasks/dev-override/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ 3928a74, T1 landed -- re-read before editing)

- `ipc.rs`: `pub async fn relay_native_host(endpoint: &str, debug: &...) -> Result<()>` opens
  with `let stream = connect(endpoint).await?;`; `connect()` retries internally (~30s on
  Windows); `EndpointProbe { Absent, Accepts, Rejects(String) }` and
  `pub fn probe_endpoint(endpoint: &str) -> EndpointProbe` exist for BOTH platforms (two cfg
  versions, same signature). After T1, `endpoint_candidates(&Selection)` exists.
- `crates/adapter-browser/src/main.rs`: `resolve_instance()` (env-wins, then argv[0] via
  `Instance::from_exe_stem_with_base(&exe, "ghostlight-adapter-browser")`, invalid env is
  non-fatal) and `ipc::relay_native_host(&ipc::default_endpoint(), &sink)`.

## STOP preconditions

- STOP if `pick_native_host_endpoint` already exists anywhere.
- STOP if `relay_native_host`'s first body line is not `let stream = connect(endpoint).await?;`.
- STOP if T1 is not landed (no `endpoint_candidates` in ipc.rs).

## Changes (transcribe from PINS P2)

1. ipc.rs: add `pick_native_host_endpoint` (pinned, probe-injected); `relay_native_host` gains
   the `endpoints: &[String]` signature and the pinned two-line body head; append the pinned
   doc-comment paragraph; add the two pinned unit tests.
2. adapter-browser main.rs: pinned `resolve_selection()` replaces `resolve_instance()`; main
   computes `ipc::endpoint_candidates(&selection)` and passes `&endpoints` to
   `relay_native_host`.

## Verification (all green, in this order)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test -p ghostlight-transport --no-fail-fast
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

## Out of scope (fences)

- NO change to `connect()`'s internal retry, `probe_endpoint`, or the frame-relay body of
  `relay_native_host` (only its signature + the two-line head).
- NO extension changes (T3). NO installer changes (T4).
- The argv[0] pinning path stays byte-equivalent for per-instance copies.

## Commit

Stage exactly the two named source files. Pinned message (PINS P2):

```
feat(transport): the browser adapter probes candidates and picks the first live service (ADR-0048 D4)
```

Then update LEDGER.md and commit as `docs(dev-override): ledger T2`.
