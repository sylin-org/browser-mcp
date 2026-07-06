# t03: Split `webapi.rs`; move `Browser` to `outbound/`; introduce `inbound/` + `outbound/` zones

Cites: ADR-0033 Decision 1, Decision 6, Decision 7. Needs t01 + t02 DONE (consumes renamed axis
+ `enabled` gate). Read ADR-0033's "Decision 6" tree target and "Decision 7" routing model first.

## What this task is

The structural break: `src/hub/webapi.rs` deletes; its two halves (the WS tool-ingestion data
plane, and the management-UI routes) move into separate modules. `transport::executor::Browser`
moves to `outbound/browser.rs`. The `inbound/` and `outbound/` zones materialize. No re-exports,
no shims — every consumer takes the new path directly.

The management-plane routes stay co-located on the same loopback listener as the web ingestion
adapter for now (Decision 7: one listener, two gated routing contexts), but the routing function
and its gate are split so the Console (`manage.web`) and the WS adapter (`inbound.web`) become
independently enableable. The `manage/` zone's full extraction (own capability, own routing
context, loopback-lock) lands in t04; this task does the physical file split + the Browser move +
the InboundChannel convergence scaffold.

## Why third

Depends on t01 (renamed axis) and t02 (`enabled` keys + bind gate). Sequenced before t04 so the
management-plane extraction has clean halves to extract from.

## Current-tree facts (re-verify)

- `src/hub/webapi.rs` is one file holding: `run()` (the listener loop), `handle_connection`
  (classifies WS-upgrade vs Console route), the WS handshake + `WsStream` + RFC 6455 primitives
  (these are the `inbound.web` half), and `route_console_request` + `write_config_response` +
  `write_sessions_response` + `write_enable_remote_response` + `write_asset` (these are the
  `manage.web` half). Plus pure payload builders (`config_payload`, `sessions_payload`) split out
  per ADR-0032 — these are reusable from either side.
- `transport::executor::Browser` is in `src/transport/executor.rs`. Consumers in tests (grep
  `transport::executor::Browser`): `tests/audit_recorder.rs:120`,
  `tests/hub_isolation.rs:32`, `tests/hub_multiplex.rs:30`, `tests/hub_queue.rs:28`.
- `transport::mod.rs` declares `pub mod executor;`.
- `serve_session` lives at `src/transport/mcp/server.rs:181` — the convergence point. It stays.
- `lib.rs` re-exports: `pub use transport::{mcp, native};` (compatibility facade for the sacred
  fidelity guard). The Browser move does NOT get a re-export — update callers directly.

## What changes

1. **Create `src/hub/inbound/mod.rs`**: declares the zone. For now it hosts a thin
   `serve_inbound_web(ctx, listener)` entry point extracted from `webapi::run`'s accept loop,
   parameterized so the listener is passed in (the bind decision stays in `run_service_loop`,
   which now resolves `inbound.web.enabled` per t02 before binding). This is the seed of the
   `InboundChannel` abstraction (a policy-enabled listener that converges on `serve_session`); a
   formal `trait InboundChannel` is NOT required this task — a function with the right shape is
   enough, and a later task can promote it.
2. **Create `src/hub/inbound/web.rs`**: the WS half of `webapi.rs`. Moves verbatim:
   `handle_connection`'s WS-upgrade branch, `WsStream`, the RFC 6455 handshake/encode/decode
   primitives, `compute_accept_key`, `sha1`, `base64_encode`, `decode_frame`, `encode_frame`,
   `host_is_expected`, `origin_hostname`, the `inbound_source` decision (post-t01 name). Calls
   `serve_session` exactly as today (line 278 of the old file).
3. **Create `src/hub/outbound/browser.rs`**: move `src/transport/executor.rs` here verbatim.
   Update `src/transport/mod.rs` to drop `pub mod executor;`. Update every consumer's `use` path
   (4 test files + any in-crate references — grep `transport::executor`).
4. **Update `src/hub/mod.rs`**: declare `pub mod inbound; pub mod outbound;` (and `pub mod manage;`
   once t04 lands its first file; for this task `manage/` may be empty or hold the relocated
   `console_assets.rs` pending t04). `run_service_loop`'s web spawn now calls
   `inbound::web::serve_inbound_web(...)` (or whatever the extracted entry is named) instead of
   `webapi::run`. The bind gate (t02) runs before this call.
5. **Leave `manage.web`'s routes in place on the same listener for now**: the routing partition
   (today's `is_ws_attempt` check ahead of the Console router) moves into `inbound/web.rs`'s
   `handle_connection`, but it still delegates non-WS requests to the management routes. Those
   routes (`route_console_request` etc.) stay in their current location (a `manage/web.rs`
   stub or the old `webapi.rs` until t04 moves them). The KEY change this task: the WS-side gate
   consults `inbound.web.enabled`/`from`, and the Console-side gate consults
   `manage.web.enabled`/`from` — two separate decisions on the two routing branches, even though
   they share one listener. That is the Decision 7 model.

6. **DELETE `src/hub/webapi.rs`** once both halves have new homes. Update any remaining `mod`
   declaration and imports (grep `hub::webapi`).

## Tests

- `tests/webapi_auth.rs`: update `use ghostlight::hub::webapi::{...}` → the new paths. The
  `resolve_bind` / `*_BIND` constants move to `inbound::web`; `builtin_webapi_from` (renamed in
  t01 to `builtin_inbound_web_from`) lives there too. This file is the compile-break sentinel for
  the WS half — fix its imports first.
- `tests/audit_recorder.rs`, `tests/hub_isolation.rs`, `tests/hub_multiplex.rs`,
  `tests/hub_queue.rs`: one-line `use` updates each (`transport::executor::Browser` →
  `outbound::browser::Browser`). Plus `hub_queue.rs`'s doc-comment reference.
- `tests/console_static_routes.rs::a_real_ws_upgrade_request_is_unaffected` now exercises
  `inbound::web`'s handshake path — it should pass unchanged (the wire is identical). This test is
  the cross-zone boundary sentinel.
- `tests/hub_role_wiring.rs`: text-scan asserts `serve_session` calls `assert_service_role`.
  `serve_session` did not move (it stays in `transport::mcp::server`), so this stays green. Re-read
  to confirm during the move.
- No behavioral assertions change. The wire, the routes, the responses — all identical.

## Verification

- All four gates green.
- `find src/hub -name webapi.rs` returns nothing.
- `grep -rn "transport::executor" .` returns nothing in `src/` (tests updated to the new path).
- `tests/all_open_golden.rs` passes byte-identical (invariant #3).
- `tests/tool_schema_fidelity.rs` passes unchanged (invariant #1).
- The architecture test passes — `outbound/` is in `src/hub`, not `src/governance`, so the `a7`
  boundary is unaffected.

## Out of scope

- The `manage/` zone's own module + assets rename — t04 (this task leaves the Console routes
  wherever they landed in step 5, possibly a `manage/web.rs` stub with the old `console_assets`
  path; t04 completes the extraction and renames the assets).
- Formalizing `trait InboundChannel` / `trait OutboundCapability` — a later coherence task; this
  task just establishes the file structure and the function shapes.
- `manage/cli.rs` and `manage/instrumentation.rs` — phase 5.
