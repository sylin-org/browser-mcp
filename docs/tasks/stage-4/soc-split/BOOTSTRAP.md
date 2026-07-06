# BOOTSTRAP: ADR-0033 — the inbound/outbound/manage SoC split

Cites: `docs/adr/0033-inbound-outbound-manage-zones.md` (the decision). Read it in full first.
Amends ADR-0030 Decision 5 and Decision 9; cashes the first slice of the deferred grant grammar.

## What this batch is

The honest split of `src/hub/webapi.rs` (which today fuses two bounded contexts — a WebSocket
tool-ingestion data plane and a loopback management UI — behind one listener, one gate, one
module) into three first-class zones: `inbound/` (ingestors that converge on the pipeline),
`outbound/` (executors), and `manage/` (the operator surface). Plus the rename of the grant axis
`channels` → `inbound`, and policy-controlled listener enablement (the "deny the adapter" case
ADR-0030 Decision 5 promised but the code never implemented).

No shims. No re-exports. Clean breaks, by mandate.

## Non-negotiable invariants (any phase red on these BLOCKS)

1. **Sacred tool surface**: the 13 trained MCP tool schemas + `explain`, byte-frozen
   (`src/transport/mcp/tools.rs` `TOOLS_JSON`, pinned by `tests/tool_schema_fidelity.rs`).
2. **Native-messaging wire** and the **extension-facing contract** (server-speaks-first,
   hello-free extension endpoint) — untouched.
3. **All-open output-identity**: a lone all-open session's client-visible output stays
   byte-identical (`tests/all_open_golden.rs`).
4. **The pipeline**: `transport::mcp::pipeline` / `serve_session` — the single governance
   chokepoint — is unchanged in behavior. It may be relocated/re-exported for module hygiene but
   its logic is not touched.
5. **The `a7` architecture test**: `src/governance/**` names no `browser`/`transport`/`mcp`/
   `native`/`tabId`/`token`/`socket`. The renamed `channels` axis stays in `src/governance/`,
   transport-agnostic.
6. **No behavioral gating, ever** (ADR-0028): the rename/move changes structure, not what a
   session is allowed to do.

## Sequencing rule

Phases run in strict order. Each phase ends GREEN on the four gates: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and the architecture
test. Do not start phase N+1 until phase N is green and committed. One commit per task,
matching the prior stage's discipline.

## Tree target (the destination, for reference while you migrate)

```
src/hub/
  inbound/
    mod.rs     the InboundChannel convergence; serve_session is the sink
    pipe.rs    the pipe adapter (relocated from transport::native::ipc's adapter path)
    web.rs     the WS/JSON-RPC tool adapter (the WS half of webapi.rs)
  outbound/
    browser.rs relocated from transport::executor (the Browser handle → chromium)
  manage/
    web.rs     the management plane HTTP routes (the Console half of webapi.rs)
    assets/    index.html, manage.css, manage.js (renamed from console.*)
    (cli.rs, instrumentation.rs: later, see phase 5)
  mod.rs       the composition root (ServiceContext, run_service_loop, run_mcp_server)
  pipeline     transport::mcp::pipeline stays where it is; the convergence point
  (handshake.rs, session.rs, supervisor.rs, role.rs, antisquat.rs: unchanged locations)
```

`webapi.rs` DELETES. `transport::executor.rs` MOVES to `outbound/browser.rs`. No re-exports.
