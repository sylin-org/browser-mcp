# MCP spec currency: the 2026-07-28 revision vs the Ghostlight tree

Date: 2026-07-07 (final revision publishes 2026-07-28; release candidate locked 2026-05-21).
Standing rule per ADR-0041 Decision 5: refresh this note when a new MCP revision publishes, and
check it before any new transport or adapter work. Tree facts below are as of dev @ 656259c.

## What the 2026-07-28 revision changes (from the RC)

1. **Stateless protocol core.** Protocol-level sessions are removed; the `Mcp-Session-Id`
   header is gone from the Streamable HTTP transport.
2. **Multi Round-Trip Requests (MRTR)** replace server-initiated sampling and elicitation: a
   server returns an input-required result carrying `inputRequests` plus opaque `requestState`;
   the client re-issues the call with answers. All state rides the payload.
3. **Tasks**: a first-class primitive for long-running work.
4. **Extensions framework**: namespaced optional protocol extensions.
5. **Authorization hardening**: six SEPs aligning the auth spec with deployed OAuth 2.0/OIDC
   practice (HTTP transports).
6. **Deprecation policy**: deprecated features live at least 12 months before removal.

Sources: blog.modelcontextprotocol.io (2026-07-28 release candidate post); see
docs/research/14 for the discovery context.

## Audit of the tree against it

- **Transport exposure.** Ghostlight's MCP is hand-rolled JSON-RPC 2.0 over stdio
  (`src/transport/mcp/server.rs`); the hub's local web API tunnels the same byte stream over a
  hand-rolled WebSocket into the same `serve_session` chokepoint. Neither uses Streamable
  HTTP, protocol-level sessions, or `Mcp-Session-Id` (verified: no occurrence in `src/`).
  The stateless-core change therefore costs nothing today, and the web adapter's session
  model (per-connection `SessionGuid`, an internal concept) is unaffected because it was never
  a protocol session.
- **Server-initiated calls.** Ghostlight issues none (no sampling, no elicitation), so the
  MRTR replacement is a no-op. If a future hold/approval flow ever wants client interaction,
  MRTR is the shape to use; do not build on elicitation, which is now legacy.
- **Structured results.** `structuredContent` + `outputSchema` (ADR-0038) match the current
  published shape and are untouched by the RC.
- **Authorization.** stdio + a loopback-default local web API; the OAuth SEPs target remote
  HTTP deployments and do not apply. If the web adapter ever accepts non-loopback peers as an
  MCP endpoint, the then-current auth spec becomes mandatory reading (the channels allowlist
  is a Ghostlight policy, not a substitute for protocol auth).
- **THE finding -- protocol version pinning.** `src/transport/mcp/server.rs:160` pins
  `pub const PROTOCOL_VERSION: &str = "2024-11-05"` and `initialize_result` echoes it
  unconditionally, ignoring the client's requested version. Meanwhile the server emits
  `structuredContent`/`outputSchema`, which entered the spec in the 2025-06-18 revision. The
  declared version and the emitted surface disagree, and there is no version negotiation.
  Clients tolerate it today (unknown fields are ignored per spec), but it is the one place the
  tree is not spec-truthful. **Disposition: landscape-1 task L2** adds version negotiation
  over a supported set (`2024-11-05`, `2025-03-26`, `2025-06-18`): echo the requested version
  when supported, else answer the latest supported. Declaring `2025-06-18` is honest because
  every optional feature is capability-gated and Ghostlight declares only `tools`.
- **Tasks (direction, not action).** `script` executions and future saved-script runs
  (ADR-0039) map naturally onto the Tasks primitive if long-running browser workflows ever
  outgrow a single request/response. Evaluate ONLY against the final published revision, after
  2026-07-28; recorded as direction in ADR-0041 Decision 5.

## Bottom line

The tree is effectively insulated by its stdio-first design. One real fix (version
negotiation, landscape-1 L2), one legacy trap to avoid (elicitation), one future mapping to
revisit after publication (Tasks). No architectural change required.
