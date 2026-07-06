# 0035. The `script` tool: sequential multi-tool composition

- Status: Accepted
- Date: 2026-07-06

## Relationship to other decisions

- BUILDS ON ADR-0034 (the capability & transport registry): `script` is a browser-capability tool
  that orchestrates the browser's own primitives. It is declared in the browser capability's
  directory alongside `navigate`, `read_page`, etc. — it is a first-class tool the model
  discovers in the capability manifest at handshake.
- BUILDS ON ADR-0024 (the generic ingest pipeline): `script`'s per-step execution calls the
  existing `pipeline::handle_tools_call` for each step — the SAME governance chokepoint every
  individual tool call enters. Each step is independently authorized, audited, and
  post-processed. `script` adds no parallel dispatch path.
- PRESERVES ADR-0030 Decision 3 (the honest singleton queue): each step enqueues an independent
  frame on the single extension port; the existing `write_chunked` + `TOOL_TIMEOUT` fairness
  guarantees are not bypassed. A 20-step script is 20 independent calls, each with its own 60s
  timeout — not one 20-minute bulk primitive.
- AMENDS ADR-0007 (sacred tool surface, deprecated by ADR-0034): `script` is a new browser tool,
  additive to the directory. The 13 primitive tools + `explain` stay; `script` joins them.

## Context

Browser automation is inherently multi-step: navigate → wait → read → find → interact → verify.
Today each step is one MCP `tools/call`, which costs one full inference round-trip — the model
generates a response, the client sends the request, the server executes, the client returns the
result, the model generates the next response. A 10-step form-filling workflow costs 10
inference passes, each adding latency, token consumption, and context-window bloat from
intermediate results.

Claude's reference implementation (and the broader MCP ecosystem) solves this at the model/client
layer (parallel tool calling, programmatic tool calling / "code mode"). But these solutions are
model-specific: Claude supports them; other models (GLM, Llama, Mistral) and other MCP clients
(Cursor, ZCode, custom integrations) generally do not. Ghostlight's mandate is to make life
easier for ANY model, not just Claude.

The `script` tool is a server-side composition primitive that works with every MCP client: it
takes an ordered array of tool calls, executes them sequentially (each step after the prior one
completes), supports data flow between steps via a lightweight reference syntax, and returns a
compact result array. One `tools/call` to `script` replaces N individual `tools/call`s. No client
changes needed; no model-specific features required.

## Decision

### Decision 1: `script` is a first-class browser-capability tool

Declared in the browser capability's directory alongside `navigate`, `read_page`, `computer`,
etc. It appears in `tools/list`, the capability manifest, and `explain` like any other tool.
The model discovers it at handshake and uses it naturally.

### Decision 2: sequential execution with data flow

Steps execute in order — step N+1 starts only after step N completes. This is the default and
the common case: browser workflows are sequential chains with dependencies (navigate → wait →
read → interact). Parallel execution is explicitly deferred (a future `mode: "parallel"` flag
for the rare case of independent calls on different tabs).

Data flows between steps via a reference syntax in step arguments:

- **`$prev.field`** — the common case: reference a field from the immediately preceding step's
  result. The model doesn't count steps; it says "from the previous step, take `ref_1`."
- **`$N.field`** — reach back to step N's result (1-indexed). The escape hatch for
  non-adjacent dependencies: "from step 2's `read_page` result, take `ref_3`" (after a wait or
  an intermediate step in between).

The resolver is JSONPath-lite: a string value starting with `$` in any step's `args` is
substituted with the referenced field from the prior step's parsed JSON result before that step
executes. Non-`$` values pass through unchanged.

### Decision 3: the input shape

```json
{
  "tool": "script",
  "args": {
    "tabId": 0,
    "steps": [
      { "tool": "navigate", "args": { "url": "https://example.com" } },
      { "tool": "read_page", "args": { "filter": "interactive" } },
      { "tool": "form_input", "args": { "ref": "$prev.ref_1", "value": "hello" } },
      { "tool": "computer", "args": { "action": "left_click", "ref": "$prev.ref_3" } }
    ],
    "onError": "stop"
  }
}
```

- `tabId` — the tab context (passed to each step that needs it; steps may omit it and inherit
  the script-level value).
- `steps` — the ordered array. Each step has a `tool` name and an `args` object.
  - `minItems: 1`, `maxItems: 20` (prevents a runaway script from monopolizing the extension
    port; the 60s per-step `TOOL_TIMEOUT` still applies).
- `onError` — `"stop"` (default, halt on first error) or `"continue"` (run remaining steps).
  Under `"stop"`, prior step results are still returned — the model sees what succeeded before
  the failure.

### Decision 4: compact results

The result is a structured array, not the raw concatenation of every step's full output:

```json
{
  "results": [
    { "step": 1, "tool": "navigate", "ok": true },
    { "step": 2, "tool": "read_page", "ok": true, "result": "ref_1: Search\nref_2: Button" },
    { "step": 3, "tool": "form_input", "ok": true },
    { "step": 4, "tool": "computer", "ok": false, "error": "[hop: page] ref_3 not found" }
  ],
  "summary": "3/4 steps succeeded (step 4 failed)",
  "duration_ms": 3400
}
```

- **Text results are included inline** (truncated if very long). The model sees what it needs to
  reason about the next step.
- **Images are NOT inlined.** A screenshot step returns `{ "step": 3, "ok": true, "imageId":
  "img_abc", "note": "screenshot captured; use computer(zoom) to inspect region" }`. The model
  can then explicitly retrieve the image if it needs to see it. This prevents a single
  screenshot from bloating the context with a 200KB base64 blob in the middle of a script
  result.

### Decision 5: each step goes through the full pipeline

`script` does not bypass governance. Each step's tool call enters the existing
`pipeline::handle_tools_call` — the SAME chokepoint every individual tool call enters today:
config snapshot, registry lookup, schema validation, governance `begin`/`authorize`, hold check,
sacred check, dispatch, audit `complete`. Each step is independently:

- **Authorized** (a manifest can deny step 3 while allowing steps 1-2).
- **Audited** (one audit record per step, carrying the step number and the `script` parent in
  its shape — see Decision 7).
- **Post-processed** (`read_page`'s secret redaction, `navigate`'s landing re-check — all
  per-step, unchanged).

A `script` of 5 steps produces 5 audit records, 5 governance decisions, and 5 extension frames.
No shortcuts, no bulk primitives, no bypasses.

### Decision 6: `script` is a `Handler::Local`

`script` is entirely service-side logic. It does not forward to the extension as a single frame;
it calls the pipeline for each step (which in turn dispatches to the extension individually).
Its `Handler` is `Handler::Local` with a function that:

1. Parses the steps array and the `onError` mode.
2. Resolves `$prev`/`$N` references in each step's args before execution.
3. Calls `pipeline::handle_tools_call` for each step, passing the shared `Browser`/`Governance`/
   `ConfigStore` handles.
4. Collects results, formats the compact output, returns.

### Decision 7: audit shape

Each step's audit record carries the standard 14-key shape (plus the new `transport` and
`capability_origin` fields from ADR-0034), with the tool name being the STEP's tool name (e.g.,
`navigate`, `read_page`), not `script`. The parent `script` call itself is NOT audited as a
separate tool call — its steps ARE the audit records. This keeps the audit stream honest: each
record represents one actual tool execution, not the orchestration wrapper.

## Consequences

### Fixed

- Any model on any MCP client can express multi-step workflows in one call. A 10-step
  form-filling flow becomes 1 inference pass instead of 10. This is the single biggest
  round-trip reducer for untrained models.
- Data flow between steps (`$prev`, `$N`) eliminates the "call read_page, see the refs, call
  form_input with the refs" two-phase pattern for find→interact workflows.
- Compact results keep intermediate output out of the context window (images referenced, not
  inlined; text truncated if very long).

### Cost

- One new tool declaration in the browser capability's directory.
- The `$prev`/`$N` resolver (~50 lines: recursive walk over args JSON, substitute `$`-prefixed
  strings with referenced fields).
- The script interpreter (~100 lines: iterate steps, call the pipeline per step, collect
  results, format compact output).
- The compact result formatter (image-id generation, text truncation).
- Per-step `tabId` inheritance (steps may omit `tabId` and inherit the script-level value).

### Preserved invariants

- All-open output-identity: `script`'s per-step execution is byte-identical to calling the step
  individually. The compact result is the only new output shape, and it's additive (the model
  sees the same data it would from individual calls, just structured).
- The dispatch stage order (pipeline.rs pins): each step enters the same pipeline stages.
- The honest singleton queue (ADR-0030 D3): no bulk primitive, no bypass of the per-call timeout
  or the chunked-write fairness.
- No behavioral gating: `script` is available in all modes (all-open, safe, restricted). A
  manifest can deny individual steps within a script, never the `script` tool itself on the basis
  of its being a composition.

## Open questions (deferred)

- **Parallel branches** (`mode: "parallel"`): for independent calls on different tabs. Deferred;
  the sequential default covers 90%+ of real workflows.
- **Named steps** (`{ "id": "page", "tool": "read_page", ... }` then `$page.ref_1`): nicer for
  readability in long scripts. Deferred until real scripts hit 15+ steps and counting becomes
  painful; `$prev` + `$N` covers 95% today.
- **Conditional steps** (`{ "if": "$prev.ok", "tool": "computer", ... }`): branching within a
  script. Deferred; if a workflow needs branching, it's complex enough to warrant its own
  semantic helper (like `form_fill`, ADR-0036) rather than generic conditional logic in `script`.
- **Saved scripts / macros** (name a script, call it by name later): the stepping stone to a
  "shortcuts" feature. Deferred; `script` is the unnamed, ephemeral v1.
