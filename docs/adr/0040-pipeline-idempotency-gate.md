# 0040. Pipeline-level idempotency gate

- Status: Proposed (direction ratified 2026-07-06; implementation deferred until the
  composition batch, ADR-0035..0038, is complete and live-verified)

## Relationship to other decisions

- SUPERSEDES ADR-0035 Decision 9 (the two-tool `idempotency_key` cache, not taken in v1): the
  implementation pass's critique -- partial coverage teaches false confidence; the correct
  home is the pipeline -- is accepted as the design input here.
- BUILDS ON ADR-0035 Decision 8 as landed: dry-run proved the pattern of a per-call flag
  threaded through `run_tool_call` that changes what happens at a pipeline boundary without a
  parallel dispatch path. The dedup gate is the same move at the pre-decision boundary.
- SERVES ADR-0036 (`form_fill`): a retried submit is the single most consequential duplicate
  in the product.

## Context

MCP delivery of intent is at-least-once (clients time out and models retry, correctly, by
design); browser side effects need at-most-once execution. The gap is real: a long `script` or
a submitting `form_fill` can outlive a client's patience, the server keeps executing, the
model retries, and an irreversible action fires twice. `script.budget_ms` narrows the window;
nothing closes it. The v1 cache design guarded only two tools; this ADR re-homes the mechanism
where the implementation critique said it belongs: one gate, every tool call.

## Decision (direction-level)

1. **A pre-decision deduplication gate in the pipeline.** `run_tool_call` checks an optional
   idempotency key BEFORE `governance.begin`: a duplicate of a completed recent call returns
   the stored outcome (marked as a replay); a duplicate of an IN-FLIGHT call joins it and
   returns the original's outcome when it lands. Replays write no audit records; the join is
   the true double-fire protection.
2. **Mechanism universal, exposure additive.** The gate keys on `(tool, key)` for ANY call
   that carries a key. In v1 the `idempotency_key` parameter is surfaced only on `script` and
   `form_fill` (additive schema fields on the two new tools; trained schemas untouched).
   Exposure can grow per ADR-0034 Decision 7's additive-optional-parameter sanction if
   warranted.
3. **Service-scoped, bounded, ephemeral.** One cache per service process (every session drives
   the same browser and user), LRU-bounded, minutes-scale TTL, never persisted. Exact
   constants are pinned at implementation time.

## Open questions (deferred to implementation)

- Constants (entry cap, TTL) and the replay marker's exact result shape.
- Whether a dry-run call ever consults the gate (presumptively no: nothing to protect).
- Interaction with the take-the-wheel hold (presumptively: a held original is joinable like
  any in-flight call; the join returns the hold text).
- Whether `explain`/read-only tools should ignore keys silently or reject them.
