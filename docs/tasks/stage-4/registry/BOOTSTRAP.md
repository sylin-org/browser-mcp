# BOOTSTRAP: ADR-0034 — The Capability & Transport Registry

Cites: `docs/adr/0034-capability-transport-registry.md` (the decision). Read it in full first.
Amends ADR-0007 (deprecates the byte-frozen mandate) and ADR-0030 (the matrix becomes code).

## What this batch is

The matrix realized: discrete inbound transports (pipe, web) and outbound capabilities (browser
today; shell/fs/desktop future) as first-class, trait-bearing zone modules behind one
composition-root registry. Each capability owns its tool declarations + RAWX requirements + agent
guidance in code; the registry aggregates them. A capability manifest at MCP handshake makes the
surface self-describing for any model.

Six phases, each green on the four gates (fmt, clippy, test, architecture) before the next starts.
One commit per phase.

## Non-negotiable invariants (any phase red on these BLOCKS)

1. **All-open output-identity**: a lone all-open session's client-visible output stays
   byte-identical (`tests/all_open_golden.rs`).
2. **The native-messaging wire** and the **extension-facing contract** (server-speaks-first,
   hello-free extension endpoint) -- untouched in behavior.
3. **The dispatch stage order** (`transport/mcp/pipeline.rs` stage comments are the pin).
4. **The governance core boundary** (a7 arch test): `src/governance/**` names no
   `browser`/`transport`/`mcp`/`native`/`tabId`/`token`/`socket`.
5. **No behavioral gating** (ADR-0028): the registry changes structure, not what a session may do.
6. **Code reads greenfield**: no "renamed from"/"formerly"/ADR-number markers in `src/` or
   `tests/`. History lives in ADRs only.

## Tree target (the destination)

```
src/hub/
  inbound/
    mod.rs        ITransport trait + the registry of transports
    pipe.rs       the pipe/adapter-control ingestion adapter (lifted from ipc.rs)
    web.rs        the WS/HTTP ingestion adapter (already discrete)
  outbound/
    mod.rs        ICapability trait + the registry of capabilities
    capability.rs (optional) shared types: ToolDeclaration, CapabilityGuide
    browser/
      mod.rs      the browser capability: holds Browser, owns its directory + guide, attach()
      tools.rs    the tool declarations (name, schema, RAWX, example, guidance) -- replaces tools.json
      backend.rs  (the current outbound/browser.rs -- the Browser handle + extension link)
  manage/         unchanged from ADR-0033
  ...
src/transport/    keeps wire-level primitives only (framing, dial, relay, default_endpoint)
```

`tools.json` DELETES. `transport/mcp/schemas/` DELETES. The aggregated directory is composed at
runtime from the registry; `tools/list`, `explain`, enforcement, and the validator all consume it.

## Sequencing rule

Phases run in strict order. Each phase ends GREEN on the four gates. Do not start phase N+1 until
phase N is green and committed. One commit per phase.
