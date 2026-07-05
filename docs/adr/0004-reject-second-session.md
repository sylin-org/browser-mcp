# 0004. Reject a second concurrent session

- Status: Accepted
- Date: 2026-07

## Context

The governance model is single-subject: one binary, one identity, one manifest, one
browser profile (SPEC 10). The reference implementation instead shares one browser across
many sessions via a primary/client TCP relay, which multiplexes several clients onto the
same browser. That multiplexing has no place in a model where exactly one manifest
authorizes exactly one subject at a time, and it is listed as an explicit v1 exclusion
(SPEC 10, multi-user / multi-session multiplexing row).

## Decision

There is one active session. The first instance to acquire the IPC endpoint owns the
browser (SPEC 2.1); a second concurrent mcp-server is rejected cleanly ("another session
owns the browser") rather than sharing the browser via a primary/client relay
(SPEC 10). The rejection is enforced at endpoint acquisition and surfaces as `SessionBusy`
(mechanism per ADR-0003). Shared machines use separate OS profiles. Primary/client
sharing is deferred, not designed away.

## Consequences

- No multiplexing machinery: no cross-session state, no primary/client relay to build, no
  question of which manifest applies to which client: the single manifest binds the one
  session.
- A clean, legible failure ("another session owns the browser") instead of two agents
  silently contending for the same tabs.
- Negative: two agents cannot drive the same browser at once; a crashed or stale session
  must release the endpoint before a new one can bind. Prompt native-host exit (ADR-0003)
  and stale-socket cleanup on Unix keep that window short.
- Follow-up: primary/client sharing remains a possible future extension if demand appears.

## Amendment (2026-07-04, ADR-0030)

ADR-0030 (Ghostlight Hub) REPEALS this decision at the MCP-client layer: a persistent SERVICE
now multiplexes any number of MCP clients as sessions instead of rejecting the second one. This
is a documentation cross-reference only -- this ADR's Status and its retained
single-physical-extension-link invariant (`Browser::attach`'s `AttachOutcome::AlreadyAttached`,
the one Chrome-spawned extension connection) are UNCHANGED: ADR-0030 repeals the CLIENT-facing
rejection this ADR documents, never the one-extension-link rule. See ADR-0030's "Relationship to
other decisions" section and Decision 3 ("D1 -- the honest singleton queue": the single MV3
service worker plus the single native port is an accepted, truthful serialization bottleneck, not
a hidden limitation) for the full scope of the repeal.
