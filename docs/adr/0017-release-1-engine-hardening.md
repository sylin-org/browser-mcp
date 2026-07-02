# 0017. Release 1 engine hardening

- Status: Accepted
- Date: 2026-07

## Context

A five-lens study of agentic-assisted developer pain (browser-tool DX, frontend
feedback loop, trust and control, auth and session walls, token and latency
cost) mapped the evidenced failures of this product category: unbounded page
reads that blow the context window, a first-call race that surfaces as
"extension not connected", Manifest V3 service-worker death that strands long
sessions, opaque errors that name no failing component, input dispatch that
pages cannot hear, and observability tools that hide the very failures agents
are asked to debug.

All of the responses below are engine-stage work. None touch the sacred tool
surface (ADR-0007) and none entangle the staged governance layer (ADR-0013,
ADR-0018).

## Decision

1. **read_page pagination.** Default behavior up to a size threshold is
   unchanged. Above it, the tree is paginated structurally: subtrees that would
   exceed the budget collapse to a single line carrying their ref and element
   count, and the model expands them with `ref_id` and `depth` (both already in
   the schema). Backstops: a 10,000-element hard cap and the `max_chars` budget,
   each with an actionable guidance line. For `filter=interactive`, results are
   culled to the current viewport with a truthful note.
2. **get_page_text.** Largest-candidate `innerText` extraction with a
   `Source element:` header, `max_chars` honored (default 50000), and guidance
   messages for empty and over-limit outcomes.
3. **Connection warmup.** Channel establishment starts at MCP `initialize`; a
   tool call arriving before readiness waits up to a bounded window (constant
   5000 ms, slated to become config key `engine.connection.first_call_wait_ms`
   per ADR-0019) and truthfully notes any wait in the result.
4. **Service-worker recovery.** Durable session state (tab group id, managed
   tab ids) persists in `chrome.storage.session`; on restart the worker
   re-adopts its group, drops dead tabs, reattaches the debugger lazily, and
   the first read after a recovery notes that event buffers were reset.
5. **Hop-attributed errors.** Every tool-call failure names the failing hop
   (request, binary, ipc, extension, cdp, page) and one concrete next step.
6. **doctor subcommand.** One command fuses the debug state of both binary
   roles, the IPC transport, and extension last-seen into a health verdict
   with fix hints.
7. **Input fidelity.** `type` dispatches real keyDown/keyUp per character with
   newline mapped to Enter; double and triple clicks send an incrementing
   clickCount sequence; mouse events carry the buttons bitmask and force;
   scroll verifies movement and falls back to the nearest scrollable ancestor;
   zoom crops the requested region and updates the screenshot context so
   follow-up coordinates land correctly.
8. **Observability truth.** Console and network buffers reset when a tab
   changes domain; `Runtime.exceptionThrown` becomes a console entry;
   `Network.loadingFailed` marks the request failed instead of pending
   forever; empty results carry a note explaining lazy tracking and how to
   capture page-load events.
9. **Backstops.** `javascript_tool` gains REPL semantics (replMode plus an
   async-IIFE retry) and a 50 KB output cap; a shared effective-tabId helper
   falls back to the group's current tab and lists valid ids in errors;
   screenshots of non-visible tabs capture via clip and scale in one pass.

## Consequences

- Positive: the token story is bounded in the worst case, first-run
  reliability improves where the category most visibly fails, and agents can
  finally see the page failures they are asked to fix.
- Positive: everything is response and behavior shaping; the tool schemas do
  not move, so `tests/tool_schema_fidelity.rs` continues to pin the surface.
- Negative: structural pagination is our own design, not official parity; the
  divergence is deliberate (the official flat truncation gives no next step)
  and must stay honest about what was collapsed.
- Follow-up: each item ships as a self-contained implementation prompt under
  `docs/tasks/release-1/` for delegation to smaller models.
