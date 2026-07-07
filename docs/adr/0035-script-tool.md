# 0035. The `script` tool: sequential multi-tool composition

- Status: Accepted
- Date: 2026-07-06 (amended same day: pre-implementation correction pass against the live
  pipeline -- reference resolution re-grounded on structured results, denial/hold visibility
  fixed, parent audit flipped, dry-run + idempotency + budget added. Re-amended same day,
  post-implementation: Decision 8 re-grounded as a pipeline-level parameter matching the landed
  code; Decision 9 not taken in v1 and superseded by ADR-0040, Proposed)

## Relationship to other decisions

- BUILDS ON ADR-0034 (the capability & transport registry): `script` is a browser-capability tool
  that orchestrates the browser's own primitives. It is declared in the browser capability's
  directory alongside `navigate`, `read_page`, etc. -- a first-class tool the model discovers in
  the capability manifest at handshake.
- DEPENDS ON ADR-0038 (structured results): `$prev`/`$N` references resolve against a step's
  `structuredContent`, never against rendered text. Without ADR-0038 there is nothing
  machine-addressable to reference; ADR-0038 lands first.
- DEPENDS ON ADR-0037 (`wait_for`): sequential scripts against dynamic pages need a condition
  wait between navigate and read, or step 2 reads a skeleton. `wait_for` is an ordinary step.
- BUILDS ON ADR-0024 (the generic ingest pipeline): each step is executed through the SAME
  governance chokepoint every individual tool call enters. Each step is independently
  authorized, audited, and post-processed. `script` adds no parallel dispatch path.
- PRESERVES ADR-0030 Decision 3 (the honest singleton queue): each step enqueues an independent
  frame on the single extension port; the existing `write_chunked` + `TOOL_TIMEOUT` fairness
  guarantees are not bypassed. A 20-step script is 20 independent calls, each with its own 60s
  timeout -- not one bulk primitive.
- AMENDS ADR-0007 (sacred tool surface, deprecated by ADR-0034): `script` is a new browser tool,
  additive to the directory. The 13 primitive tools + `explain` stay; `script` joins them.
- FEEDS ADR-0039 (saved scripts, Proposed): the `batch_id` audit correlation pinned here is the
  recording substrate for named, governed, replayable workflows.

## Context

Browser automation is inherently multi-step: navigate, wait, read, find, interact, verify.
Today each step is one MCP `tools/call`, which costs one full inference round-trip. A 10-step
workflow costs 10 inference passes, each adding latency, token consumption, and context-window
bloat from intermediate results.

Claude's ecosystem solves this at the model/client layer (parallel tool calling, programmatic
tool calling). But those solutions are model-specific. Ghostlight's mandate is to make life
easier for ANY model on ANY MCP client. The `script` tool is a server-side composition
primitive: one `tools/call` carrying an ordered array of tool calls, executed sequentially, with
data flow between steps and a compact result. No client changes, no model-specific features.

The amendment pass corrected the original against the live tree:

- The original `$prev.ref_1` flagship example referenced a field that exists in no result shape
  (`read_page` renders prose; `ref_N` tokens are embedded text, not fields), and presumed the
  model could know which ref is which without seeing the page -- the exact round trip `script`
  claims to remove. References now resolve against ADR-0038 structured results, and the
  read-then-fill workflow is explicitly `form_fill`'s job (ADR-0036), not `script`'s.
- Governance denials, sacred denials, and take-the-wheel holds return as SUCCESSFUL text results
  by design (pipeline.rs), so an orchestrator reading the MCP envelope cannot distinguish a
  denied step from a completed one. The pipeline core now returns a structured outcome
  (Decision 6) and the compact result carries an honest per-step status (Decision 4).
- `Handler::Local` today is `Local(fn() -> String)`: synchronous, argument-less, answered in the
  free-action arm. Both this tool and `form_fill` need async, argument-bearing, re-entrant local
  handlers; the honest cost is stated in Decision 6, not discovered mid-implementation.

## Decision

### Decision 1: `script` is a first-class browser-capability tool

Declared in the browser capability's directory alongside `navigate`, `read_page`, `computer`,
etc. It appears in `tools/list`, the capability manifest, and `explain` like any other tool.

Its directory row: `requires: []` (the steps carry their own requirements; the wrapper itself
touches nothing), no `action_key`, `Handler::Local`, no postprocess, no post-dispatch marker.

Step tool names may be any tool in the aggregated directory EXCEPT `script` itself -- no
nesting, no recursion, enforced at schema-validation time with a corrective error. `form_fill`
(ADR-0036) is a legal step: it is bounded and internally budgeted.

### Decision 2: sequential execution with data flow over structured results

Steps execute in order -- step N+1 starts only after step N completes. Parallel execution is
explicitly deferred (a future `mode: "parallel"` flag for independent calls on different tabs).

Data flows between steps via references in step arguments, resolved against the referenced
step's STRUCTURED result (ADR-0038 `structuredContent`), never against rendered text:

- **`$prev.path`** -- a field from the immediately preceding step's structured result.
- **`$N.path`** -- reach back to step N's structured result (1-indexed).
- **Path grammar:** dot-separated segments after the head; a numeric segment is an array index.
  `$prev.results.0.ref` is "first match's ref from the previous `find`". A bare `$prev` / `$N`
  (no path) substitutes the whole structured result.
- **Escape:** a leading `$$` produces a literal `$` and ends reference processing
  (`"$$1.50"` becomes the string `"$1.50"`). A `$`-string that does not match the reference
  grammar (`$prev` or `$<digits>`, optionally followed by `.path`) passes through unchanged.
  Note the sharp case this grammar creates: a literal money value like `"$1.50"` DOES parse as
  step-1, path `50`; it will fail resolution with a corrective error that names the `$$` escape.
- **Failure semantics:** an unresolvable reference (no such step, step not run, step failed,
  step has no structured result, path miss) fails THAT step before any dispatch, with a
  corrective error naming the reference, the available keys, and the `$$` escape hint. The
  failure respects `onError` like any other step failure.

The honest v1 showcase chains are `find` then act (`$prev.results.0.ref`), `tabs_create_mcp`
then use the new tab (`$prev.tabId`), and `wait_for` then act on the matched element
(`$prev.ref`). `read_page` deliberately exposes no structured reference surface in v1: if the
workflow is "read the form, then fill it", the answer is `form_fill` (ADR-0036), not a script.

### Decision 3: the input shape

```json
{
  "tool": "script",
  "args": {
    "tabId": 0,
    "steps": [
      { "tool": "navigate", "args": { "url": "https://example.com" } },
      { "tool": "wait_for", "args": { "text": "Results", "state": "visible" } },
      { "tool": "find", "args": { "query": "download report button" } },
      { "tool": "computer", "args": { "action": "left_click", "ref": "$prev.results.0.ref" } }
    ],
    "onError": "stop",
    "dry_run": false,
    "budget_ms": 90000
  }
}
```

- `tabId` -- the script-level tab context. Steps may omit `tabId` and inherit it; a step may
  override it (including with a reference, e.g. `"tabId": "$prev.tabId"` after `tabs_create_mcp`).
- `steps` -- the ordered array; each step has `tool` and `args`. `minItems: 1`, `maxItems: 20`.
- `onError` -- `"stop"` (default) or `"continue"`. Under `"stop"`, prior results are returned.
- `dry_run` -- Decision 8. Default false.
- `budget_ms` -- total wall-clock budget for the whole script. Registered in the typed config
  registry as `script.budget_ms` (default 120000; hard cap 480000; org-lockable like any key);
  the argument may lower but never exceed the configured value. On exhaustion, the current step
  finishes its own `TOOL_TIMEOUT` window, remaining steps report `not_run`, and the compact
  result returns what completed. Rationale: 20 steps x 60s is a 20-minute single `tools/call`;
  MCP clients time out far earlier, and a server still executing Write steps after the client
  gave up invites a retry and a double-submit. The budget narrows that hazard; closing it fully
  is ADR-0040's mandate (Proposed).

### Decision 4: compact results with honest per-step status

```json
{
  "results": [
    { "step": 1, "tool": "navigate", "status": "ok" },
    { "step": 2, "tool": "wait_for", "status": "ok", "result": "found after 640ms" },
    { "step": 3, "tool": "find", "status": "ok",
      "result": "Found 2 element(s): [ref_12] button \"Download report\" ..." },
    { "step": 4, "tool": "computer", "status": "denied",
      "result": "Write denied on example.com by grant g-14 (manifest acme.json)" }
  ],
  "summary": "3/4 steps completed; step 4 denied",
  "duration_ms": 3400
}
```

- **`status`** is one of `ok | error | denied | held | not_run`. This is the load-bearing fix:
  denials and holds are successful TEXT envelopes on the wire (deliberately, so models read
  them), so a boolean `ok` derived from `isError` would report a denied navigate as success.
  Status comes from the pipeline's structured outcome (Decision 6), not from envelope sniffing.
  The denial/hold text is always included verbatim -- it is the model's corrective guidance.
- **Text results are included inline**, truncated at 2000 chars per step and 25000 chars for
  the whole result (marked `(truncated)`); a step's structured result (ADR-0038) rides along
  under `structured` when its declaration defines one.
- **A hold stops the script unconditionally**, regardless of `onError`: the held step reports
  `held`, every remaining step reports `not_run`, and the script returns immediately. The user
  grabbed the wheel; burning through 16 more steps that each individually answer with hold text
  would be technically correct and humanly wrong.
- **Screenshots are not inlined and, in v1, not stored.** A screenshot-producing step returns
  `{ "status": "ok", "note": "screenshot captured and discarded inside script; call
  computer(screenshot) directly if you need to see it" }`. The original draft promised an
  `imageId`, but no retrieval tool exists and none is sanctioned here; a dangling identifier is
  worse than an honest note. An image store + retrieval is an open question.

### Decision 5: each step goes through the full pipeline

`script` does not bypass governance. Each step enters the existing pipeline chokepoint: config
snapshot, registry lookup, schema validation, governance `begin`/`authorize`, hold check, sacred
check, dispatch, audit `complete`. Each step is independently:

- **Authorized** (a manifest can deny step 3 while allowing steps 1-2). Unlike `form_fill`'s
  internals (ADR-0036 Decision 5), script steps are independent model-authored intents, so each
  one gets its own full governance decision. That distinction is deliberate and pinned:
  composition of arbitrary calls = per-step decisions; mechanism of a single semantic intent =
  one decision at the parent.
- **Audited** (one record per step; Decision 7).
- **Post-processed** (`read_page` secret redaction, `navigate` landing re-check -- per-step,
  unchanged).
- **Snapshot-per-step:** each step takes its own config snapshot at entry, exactly as an
  individual call would. A hot-reload mid-script applies from the next step. This is the honest
  reading of "20 independent calls".

### Decision 6: the structured-outcome refactor and the new `Handler::Local`

Two pipeline changes this ADR owns and prices honestly:

1. **`CallOutcome`.** The pipeline core splits into `run_tool_call(...) -> CallOutcome` and an
   MCP-edge renderer. `CallOutcome` variants: `Success { result, structured }`,
   `Failure { ToolError }`, `Denied { message, source: Policy | Sacred }`, `Held { message }`.
   The edge renders each variant into today's envelopes BYTE-IDENTICALLY (denials and holds
   stay successful text results on the wire; all-open output-identity holds). Orchestrators
   (`script`, `form_fill`) consume `CallOutcome`, which is the only honest way to know what
   actually happened to a step.
2. **`Handler::Local` grows up.** From `Local(fn() -> String)` to an async handler receiving a
   context (Browser, ConfigStore, Governance, the call's args, session identity) and returning
   `CallOutcome`. Async recursion (pipeline -> local handler -> pipeline) is boxed
   (`Box::pin`). `explain` migrates mechanically. Dispatch position: a Local tool with
   `requires: []` answers in the free-action arm exactly where `explain` answers today
   (`script` itself does); a Local tool with non-empty requires (`form_fill`) dispatches in the
   ExtensionForward stage position, AFTER grant enforcement. Both positions are pinned so the
   stage-order tests stay meaningful.

### Decision 7: audit shape -- the parent IS audited, steps carry correlation

The original draft suppressed the parent record ("the steps ARE the audit records"). Flipped,
for two reasons: every pipeline entry produces a record today (including `Handler::Local`;
`explain` is audited), so suppression would be a special case sitting oddly next to "no
shortcuts, no bypasses"; and ADR-0034 Decision 8 argues audit exists to make `policy simulate`
replay-faithful -- omitting the one `tools/call` the wire actually received makes replay LESS
faithful.

- **The parent `script` call gets one record**: tool `script`, requirement-free allow, with a
  fresh `batch_id` (GUID).
- **Each step's record** carries the step's own tool name and the standard shape, plus three
  additive keys appended at the END of the record (after `held` today; ADR-0034 Decision 8's
  `transport`/`capability_origin` had not landed as of this amendment and append after these
  whenever they do; old-record byte-order preserved):
  `orchestrator: "script"`, `batch_id` (the parent's GUID), `step` (1-indexed).
- One script call is therefore fully reconstructable from the stream: one parent + N correlated
  step records, each step record still representing one actual tool execution.

### Decision 8: `dry_run` -- per-step governance verdicts without execution

As re-grounded by the implementation (the landed design is BETTER than the original
script-layer evaluator, and this amendment ratifies it): dry-run is a PIPELINE parameter, not a
script-internal simulation. `run_tool_call` carries a `dry_run` flag; a dry step runs the REAL
decision path -- registry lookup, schema validation, hold check, sacred check, and the genuine
governance verdict (via the audit-free decision port, so no step record is written) -- and at
the dispatch boundary returns the verdict instead of sending a tool frame. Verdicts cannot
drift from live behavior because they ARE live behavior; a denial's text is the exact text a
live call would produce.

- Statuses: `would_allow` | `would_deny`. There is no `indeterminate`: every verdict is the
  real pre-dispatch decision. The one execution-time dependency is named instead of encoded: a
  dry verdict for a tool with a landing re-check (`navigate`) carries the suffix
  `(pre-dispatch verdict; the post-redirect landing is checked live)` so the pre-flight map
  never over-promises.
- Under dry-run the interpreter never halts on a non-ok verdict: the point is the FULL map.
- Dry-run may probe tab URLs (read-only tab metadata) but never sends a tool frame.
- Audit: the parent record is written with `dry_run: true` (stamped via the same side-channel
  pattern as `batch_id`); no step records (nothing executed -- the absence is the honest
  signal, and the parent's flag disambiguates "dry run" from "script that ran nothing").
- The flag originates from the script tool's own arguments and flows to its steps;
  `handle_tools_call` always passes `dry_run: false` for top-level calls.

The strategic point: the worst governance experience is not denial, it is denial at step 7 of
10 with the world half-mutated. Dry-run turns denial from a landmine into a map, and it is
`policy simulate` exposed to the model -- the governance pillar and the delight pillar in one
feature. No competitor can copy it without first having the governance layer.

### Decision 9: retry safety -- NOT TAKEN in v1; superseded by ADR-0040 (Proposed)

The originally ratified design (an `idempotency_key` argument on `script`/`form_fill` backed by
a service-scoped LRU with in-flight join) was NOT implemented, and the argument does not exist
in any shipped schema. The implementation pass recorded the reasons (composition batch LEDGER,
C8): a two-tool cache protects `script` and `form_fill` while every direct
`computer(left_click)` and `form_input` carries the identical double-submit hazard unprotected
-- partial coverage that teaches false confidence; and the correct placement is a pre-decision
gate in the pipeline, covering EVERY tool call, exactly as Decision 8's dry-run flag proved
out.

That critique is accepted as being about PLACEMENT, not about whether retry safety matters:
the client-timeout -> automatic-retry -> duplicate-submit sequence remains a real, open hazard
(narrowed but not closed by `budget_ms`). ADR-0040 (Proposed) owns the rebuild as a
pipeline-level deduplication gate. Until it lands, an interrupted mutating call fires once and
a re-fire is the model's or user's explicit choice.

## Consequences

### Fixed

- Any model on any MCP client expresses multi-step workflows in one call; round-trips collapse.
- Data flow works against real, addressable shapes (`find`, `tabs_create_mcp`, `wait_for`),
  with escaping, and fails loudly and correctively instead of silently misfiring.
- Denied/held steps are visible AS denied/held in the compact result; a script can never
  report success for work governance stopped.
- The client-timeout/retry double-submit hazard is narrowed (budget); closing it is ADR-0040.
- Dry-run gives models a pre-flight map of what a script would be allowed to do, from the real
  decision path.

### Cost

- The `CallOutcome` split and the `Handler::Local` signature change (the real cost center of
  this ADR: a pipeline refactor touching the edge renderer, the Local dispatch arms, and
  `explain`'s migration -- priced here, not discovered later).
- The reference resolver (grammar, path walk, array indexing, `$$` escape, corrective errors).
- The script interpreter (iterate, resolve, call `run_tool_call`, collect, budget accounting).
- The compact formatter (status mapping, truncation budgets, structured passthrough).
- The `script.budget_ms` config key and the dry-run plumbing through `run_tool_call`.
- Three additive audit keys plus `dry_run`, and their shared-format doc section.

### Preserved invariants

- All-open output-identity for individual calls: the `CallOutcome` edge renders today's
  envelopes byte-identically; `structuredContent` additions are sanctioned separately by
  ADR-0038.
- The dispatch stage order (pipeline.rs pins), with the two Local dispatch positions now pinned
  explicitly.
- The honest singleton queue (ADR-0030 D3): per-step frames, per-step `TOOL_TIMEOUT`, no bulk
  primitive.
- No behavioral gating: `script` is available in all modes. A manifest can deny individual
  steps, never the `script` tool itself on the basis of its being a composition.

## Open questions (deferred)

- **Parallel branches** (`mode: "parallel"`): deferred; sequential covers 90%+ of workflows.
- **Named steps** (`{ "id": "page", ... }` then `$page.ref`): deferred until real scripts hit
  15+ steps; `$prev` + `$N` covers today.
- **Conditional steps**: deferred; a workflow that needs branching warrants its own semantic
  helper (the ADR-0036 pattern), not generic conditionals in `script`.
- **Saved scripts / macros**: now ADR-0039 (Proposed) -- named, parameterized, governed,
  advertisable workflow artifacts recorded via this ADR's `batch_id`.
- **Screenshot store + retrieval** for mid-script captures (v1 discards with a note).
- **Naming**: `batch` / `sequence` were considered against `script` (which sits one shelf away
  from `javascript_tool` and may invite models to pass JS source). Kept `script` for v1; the
  schema validator's corrective error ("steps is an array of tool calls, not source code")
  mitigates. Revisit only if live traffic shows real confusion.
