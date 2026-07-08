# exe-split LEDGER

Durable batch progress. One task = one CODE commit + one ledger commit = one log entry
(BOOTSTRAP per-task procedure). Update after EVERY task, before starting the next.

## RESUME HERE

- Next task: **S5** (`S5-adapter-agent-bin.md`)
- Base commit: `fccca60` on `dev` (tree green at batch authoring; later docs-only commits carry
  the batch itself)
- Batch state: IN PROGRESS (S1, S2, S3, S4 complete)

## Task table

| Task | Title | Status | Commit |
|---|---|---|---|
| S1 | Workspace + transport crate skeleton | done | 14a8bd0 |
| S2 | Move leaf utilities to transport | done | bbb02da |
| S3 | Move wire + handshake to transport | done | a48c136 |
| S4 | Create ghostlight-core; root becomes facade | done | 4d8767a |
| S5 | ghostlight-adapter-agent bin + rewire clients + test harness | pending | - |
| S6 | ghostlight-adapter-browser bin + host install rework | pending | - |
| S7 | Retire roles from the ghostlight bin | pending | - |
| S8 | Reconnect patience (120s) + ADR-0045 amendment | pending | - |
| S9 | --no-supervisor + DEV-LOOP.md | pending | - |
| S10 | Packaging + distribution sweep | pending | - |

## Log

(Append one entry per finished task:)

```
### S<n> -- <title>
- Commit: <hash>
- Verification: fmt OK / clippy OK / test --workspace OK / linux cross-check OK
- Deviations:
  1. <none | numbered list, one line each>
```

### S1 -- Workspace + transport crate skeleton
- Commit: 14a8bd0
- Verification: fmt OK / clippy OK / test --workspace OK (524 root unit + full integration suite pass; new ghostlight-transport crate builds, 0 tests) / linux cross-check OK
- Deviations:
  1. none. (Git reported routine CRLF->LF normalization on Cargo.toml; committed blobs are LF per repo convention -- no content or requirement change.)

### S2 -- Move leaf utilities to transport
- Commit: bbb02da
- Verification: fmt OK / clippy OK / test --workspace OK (full suite green; ghostlight-transport now runs 36 moved unit tests, 0 failed) / linux cross-check OK
- Deviations:
  1. Promoted three observability fns from `pub(crate)` to `pub` -- `now_ms`, `fmt_ms`, `session_state_files`. Reason: `src/hub/manage/doctor.rs` (root crate) calls all three, and once observability moved to ghostlight-transport the `pub(crate)` visibility scoped them to transport, breaking the cross-crate calls. SPEC section 2 sanctions exactly this ("items that were `pub(crate)` or private and are now needed across the crate boundary become `pub`"). No behavior change; not a governance/tool-surface item.
  2. watchdog.rs keeps a now-dangling rustdoc intra-doc link `[`crate::main`]` (the lib-only transport crate has no `main`). Left unmodified per the mechanical-move rule (not in the SPEC section 2 rewrite list). Harmless to SPEC-12 verification: intra-doc links are a rustdoc lint, not built by clippy/test/check, and CI runs no `cargo doc`.

### S3 -- Move wire + handshake to transport
- Commit: a48c136
- Verification: fmt OK / clippy OK / test --workspace OK (full suite green; ghostlight-transport now runs 56 unit tests, 0 failed; the 3 adapter-side ipc tests + 2 service-side ipc tests both run in their new homes) / linux cross-check OK. Merge shim confirmed at src/transport/native/ipc.rs:43.
- Method note: the root ipc.rs adapter/service split was done with a checked Python script (scratchpad) that extracts service-half line ranges by number with a boundary assertion on every range, so the delicate unsafe FFI (capture_peer_cred, win_security) is preserved byte-exact rather than retyped. The adapter half (transport/src/ipc.rs) was written fresh with the SPEC section 2 path rewrites.
  1. transport/src/ipc.rs doc-prose adjustments (3), all to avoid dangling rustdoc links that would point OUTSIDE the transport crate (a core dep transport must never take) or at a renamed item: (a) the module doc carries the original's general pre-endpoint paragraphs plus a new one-line ADR-0046 split note, dropping the endpoint-enumeration paragraph whose links name the now-relocated serve/claim/serve_adapters/handle_adapter_connection/send_service_proof; (b) two `[`crate::hub::outbound::browser::Browser::attach`]` links in the probe_endpoint docs became the prose "the browser executor" (Browser is a core type); (c) a stale `[`dial_with_self_heal`]` link became `[`connect_and_handshake`]` (its current name). No code/behavior change.
  2. root ipc.rs tests module gained `use tokio::time::{sleep, Duration};`. The module-level tokio::time import was dropped because the service half's non-test code never uses sleep/Duration (only the two service tests do); the import moved into the tests module so clippy -D warnings stays clean either way.
  3. hub/mod.rs: role/antisquat/handshake/supervisor consolidated into ONE re-export line `pub use ghostlight_transport::{antisquat, handshake, role, supervisor};` (rather than a separate `role` line plus a new three-item line). Same effect, one fewer line; fmt keeps the alphabetical order.
  4. Carried forward: transport now also has a dangling rustdoc link in handshake.rs (`[`crate::transport::mcp::server::serve_session`]`), same rustdoc-only class as the S2 watchdog note; harmless to SPEC-12.

### S4 -- Create ghostlight-core; root becomes facade
- Commit: 4d8767a
- Verification: fmt OK / clippy OK / test --workspace OK (40 test-result lines, 0 FAILED; ghostlight-core runs 468 unit tests, 0 failed) / linux cross-check OK. tests/ diff since HEAD = EXACTLY tests/architecture.rs + tests/hub_role_wiring.rs (SPEC section 12 pin met).
- Method note: the SPEC section 3 path rewrites were applied by a checked Python script (13 patterns across crates/core/src; 18 files, e.g. crate::transport::mcp -> crate::mcp x32, crate::instance -> ghostlight_transport::instance x12). The ipc references were fixed BY HAND (not the script) for the adapter-vs-service split: doctor.rs -> ghostlight_transport::ipc (all-adapter), pipe.rs -> `use crate::hub::endpoint as ipc` (all-service), hub/mod.rs split into `use ghostlight_transport::ipc` (default_endpoint/relay_adapter) + child module `endpoint::` for serve/claim. core lib.rs (SPEC 3) and root facade (SPEC 6) written exactly as pinned.
  1. Kept `anyhow` in ghostlight-core's [dependencies] though SPEC section 3's minus-list drops it: crates/core/src/hub/mod.rs uses `anyhow::{Context, Result}`, so the compiler demands it (SPEC section 3: "the compiler is the referee; log every kept-but-questionable dep"). getrandom/hmac/tracing-subscriber/clap were dropped as SPEC directs (0 refs in the moved tree); sha2/uuid/chrono/url kept (in use).
  2. Root [dependencies] keeps dirs, uuid, chrono, url beyond SPEC section 4's list (ghostlight-core, ghostlight-transport, clap, anyhow, tokio, tracing, serde_json). Reason: the integration tests in tests/ use those crates directly (dirs x2 files, chrono x1, uuid x1, url x1, serde_json x23, tokio x8) and the package has no [dev-dependencies] section, so [dependencies] is the only place they can live. main.rs itself needs only clap/anyhow/tokio/tracing. Removed serde, tracing-subscriber, thiserror, sha2, hmac, getrandom, winreg, windows-sys, libc from root.
  3. Straggler fix (compiler-demanded): crates/core/src/governance/templates.rs `include_str!` paths went `../../examples/` -> `../../../../examples/` (the file is two directory levels deeper after the move; examples/ stays at the repo root, unmoved). Path-only; the embedded template bytes and all governance semantics are byte-unchanged. The a7 governance-purity test (tests/architecture.rs) still passes because `ghostlight_transport::...` does NOT contain the forbidden token `crate::transport` (the ban is on the `crate::`-prefixed path edge).
  4. Straggler fix: removed the `use crate::hub::endpoint;` I first added to hub/mod.rs -- it collided (E0255) with the `pub mod endpoint;` child-module declaration; the child module already puts `endpoint` in scope, so `endpoint::serve` / `endpoint::claim_adapter_endpoint` resolve directly.
  5. endpoint.rs (moved service half): the S3 merge shim became a plain `use ghostlight_transport::ipc::*;` (was `pub use`) -- the root facade (SPEC 6) re-exports both ipc halves under `ghostlight::native::ipc`, so a `pub use` here would double-export. Module doc collapsed to the SPEC section 3 one-liner.

## Blocked

(Only if the failure protocol fired: task id, exact failing step/error text, one-paragraph
diagnosis. The batch HALTS here.)
