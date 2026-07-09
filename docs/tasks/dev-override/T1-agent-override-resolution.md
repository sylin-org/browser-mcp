# T1 -- Selection tri-state + agent-side override resolution (ADR-0048 D1/D2/D3)

## Goal

An UNPINNED `ghostlight-adapter-agent` (no `--instance`, no `GHOSTLIGHT_INSTANCE`) walks the
ordered candidate list `[dev, default]` on every connect episode -- first connect AND every
reconnect tick -- so a live dev service shadows the default and a dead one fails over to it, with
the MCP handshake replayed (ADR-0045) and the same per-process guid re-presented (ADR-0047 D2).
Pinning (`--instance <n>`, `--instance default`, `GHOSTLIGHT_ENDPOINT`) keeps exactly one
candidate. Normative: ADR-0048 D1/D2/D3. Oracles: PINS.md P1.

## Files this task owns (touch nothing else)

- `crates/transport/src/instance.rs` (Selection + DEV_INSTANCE + tests)
- `crates/transport/src/ipc.rs` (candidates + relay_adapter + connect_and_handshake + tests)
- `crates/adapter-agent/src/main.rs` (resolve_selection + candidate threading)
- `tests/hub_identity.rs` (ONE call site: the relay_adapter slice fix; PINS P1)
- `tests/adapter_override.rs` (NEW)
- `docs/tasks/dev-override/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ 3928a74 -- re-read every one before editing)

- `instance.rs`: `Instance::validate` REJECTS the name `default` (reserved) -- Selection's
  `default` handling therefore lives in `classify`, BEFORE `from_name`. `Instance::from_name`,
  `ENV_VAR`, `endpoint()` exist as used by the pins.
- `ipc.rs`: `pub fn default_endpoint() -> String` reads `GHOSTLIGHT_ENDPOINT` else
  `Instance::resolve().endpoint()`; `adapter_endpoint_name(endpoint)` appends `-adapter`;
  `pub async fn relay_adapter(endpoint: &str, debug: &crate::observability::DebugSink)` computes
  `let adapter_endpoint = adapter_endpoint_name(endpoint);` and loops
  `connect_and_handshake(&adapter_endpoint, !first, &session_guid)`;
  `async fn connect_and_handshake(adapter_endpoint: &str, reconnect: bool, guid: &...)` does one
  fast `try_connect_once`, then `crate::supervisor::start_service()`, then the interval/window
  retry loop. `try_connect_once(ep, guid)` dials ONCE (an absent pipe fails instantly).
- `crates/adapter-agent/src/main.rs`: `resolve_instance()` + `instance_flag_value()` +
  `let endpoint = ipc::default_endpoint();` + `relay_with_watchdog(&endpoint, ...)` +
  `ipc::relay_adapter(endpoint, &debug_sink)`.
- `tests/hub_identity.rs` contains
  `let _ = ghostlight::native::ipc::relay_adapter(&relay_endpoint, &debug).await;` inside a
  `tokio::spawn(async move { ... })`.
- `tests/adapter_reconnect.rs` (NEVER edited) holds the helper shapes to transcribe: `static
  SEQ`, `bin`, `adapter_bin`, `service_cmd`, `wait_for_state`, `spawn_adapter`, `send`, `recv`,
  and an inline adapter-stdout reader thread in the restart test.
- The MCP `initialize` result carries `result.serverInfo.name == "ghostlight-<instance>"` (the
  instance's `mcp_server_name`), which the new integration tests use as the
  which-service-answered oracle.

## STOP preconditions

- STOP if `Selection`, `DEV_INSTANCE`, `endpoint_candidates`, or `candidates_from` already exist
  anywhere (`grep -rn "enum Selection\|DEV_INSTANCE\|endpoint_candidates" crates/transport/src/`
  -- note `install::Selection` in core is a DIFFERENT type and does not count).
- STOP if `relay_adapter`'s signature or `connect_and_handshake`'s shape differs materially from
  the facts above.
- STOP if `tests/adapter_override.rs` already exists.
- STOP if `grep -n "GHOSTLIGHT_ENDPOINTS" crates/ src/ tests/ -r` matches anything (the plural
  seam must be new).

## Changes (transcribe from PINS P1; order within the task)

1. instance.rs: add `DEV_INSTANCE`, `Selection` (`classify`, `resolve_from`, `candidates`) and
   the two pinned tests.
2. ipc.rs: add `candidates_from` + `endpoint_candidates` (+ pinned precedence test); convert
   `relay_adapter` and `connect_and_handshake` to the pinned multi-candidate shapes (doc-comment
   appends included); add the pinned override note emission.
3. adapter-agent main.rs: pinned `resolve_selection()` replaces `resolve_instance()`; thread
   `endpoint_candidates(&selection)` through `relay_with_watchdog` into `relay_adapter`.
4. tests/hub_identity.rs: the pinned one-line slice fix.
5. tests/adapter_override.rs: the pinned new file (helpers transcribed from adapter_reconnect.rs
   with the pinned deltas; both pinned tests verbatim).

## Verification (all green, in this order)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test -p ghostlight-transport --no-fail-fast
cargo test --test adapter_override --no-fail-fast
cargo test --test adapter_reconnect --no-fail-fast
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

`adapter_reconnect` MUST pass UNCHANGED (the single-candidate regression guard).

## Out of scope (fences)

- NO change to `relay_native_host` (T2), the extension (T3), the installer (T4), doctor (T5).
- NO change to `try_connect_once`, `adapter_hello`, `verify_service_proof`, `dial_once`, the
  retry constants, or the supervisor.
- NO change to `default_endpoint` (the service and doctor keep it).
- NO change to `Instance::validate` or any existing instance.rs test.

## Commit

Stage exactly the five named source files. Pinned message (PINS P1):

```
feat(transport): the development override -- unpinned adapters resolve dev-first (ADR-0048 D1/D2/D3)
```

Then update LEDGER.md and commit as `docs(dev-override): ledger T1`.
