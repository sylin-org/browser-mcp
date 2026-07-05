# 0031. Agent onboarding contract: tools.json as the single source of agent-facing truth

- Status: Accepted
- Date: 2026-07-05

## Relationship to other decisions

- AMENDS ADR-0007 (sacred tool surface): the trained-surface protection is scoped to the
  *trained* fields (`name`, `description`, `inputSchema`). The additive `example` and the
  top-level `agentGuide` section introduced here are NOT trained fields; they are ghostlight's
  own additive content and are out of scope of the byte-parity freeze. ADR-0007's fidelity test
  grows (see Decision 5) but its existing assertions are byte-stable.
- BUILDS ON ADR-0024 (tool registry and generic ingest pipeline): the directory already drives
  runtime dispatch. This ADR makes the schema fixture drive the agent contract too, and removes
  the directory's parallel prose so there is exactly one prose source per tool.
- ALIGNS WITH the NORTH-STAR Principle 3 (layered delight): agent onboarding is L0 base
  capability delight. The engine must be excellent to drive, not merely correct, for every model
  -- not only the one whose training surface ADR-0007 preserves.

## Context

Ghostlight's stated audience is any MCP client, any model (NORTH-STAR; SPEC 1.2). But the
agent-facing contract was inherited, unchanged, from a Claude-trained ecosystem: the schemas in
`src/transport/mcp/schemas/tools.json` are byte-identical to the official Claude-in-Chrome surface
(ADR-0007), and the `initialize` response emitted `serverInfo` but no `instructions` field at all.
A model trained on this surface (Claude) internalized the workflow at training time; a model that
was NOT trained on it (any other model) gets only the per-tool `inputSchema` and has to derive the
workflow -- "always get a tabId first, then navigate, then read" -- from description prose, and
gets it wrong on the first call.

This was observed directly during the security-1 verification pass: an untrained model called
`navigate({url})` without the required `tabId`, hit a generic schema-validation error, and had to
guess or re-fetch the schema to recover. The schema contract is necessary but not sufficient for
an untrained model; the workflow contract that a trained model carries in its weights has to be
served explicitly to everyone else.

A second drift problem exists independent of onboarding: `tools.json` and
`src/browser/directory.rs` BOTH carry a per-tool description string, and they have already
diverged (e.g. `navigate`: tools.json says "Navigate to a URL, or go forward/back in browser
history..."; directory says "Load a URL in a tab, or go back or forward in its history; a
top-level GET."). The directory's description is documentation-for-governance that nothing the
agent sees consumes. It is a second source with no test pinning it to the first.

## Decision

`tools.json` is the single source of truth for everything an agent sees. The service emits its
contents verbatim; it owns no agent-facing prose. Five parts.

### Decision 1: the workflow preamble lives in MCP `initialize.instructions`, sourced from a new `agentGuide` section of tools.json

The `initialize` response gains an `instructions` field (the native MCP home for "how to use this
server and its tools"). Its contents are NOT hand-written by the service -- they are a passthrough
of a new top-level `agentGuide` object in `tools.json`, with these fields:

- `summary`  -- one line: what ghostlight is and what it does.
- `workflow` -- the load-bearing rule: every tab-touching tool requires a `tabId`; get one from
  `tabs_context_mcp` (with `createIfEmpty: true`) or `tabs_create_mcp` before anything else; then
  `navigate`. Includes a `cost` line: screenshot/zoom return large images; prefer `read_page` or
  `get_page_text` for structure or text.
- `flow`     -- one line: the canonical sequence (`tabs_context_mcp -> navigate -> read -> act ->
  re-read`).
- `denials`  -- one line: a denial looks like `Denied (D-xxxxxxxx): ...`; call `explain` (no
  args) to see what is permitted.

The service constructs nothing; it parses `agentGuide` and emits it. Target size ~350-400 tokens,
served once at handshake (before any tool call), negligible against screenshot-bearing workflows.

### Decision 2: each tool carries an `example` field, additive and validated

Each tool entry in `tools.json` gains an optional `example` object:

```json
"example": {
  "call":    { "tabId": 0, "url": "https://example.com" },
  "returns": "navigated to https://example.com/"
}
```

Rules:

- `example.call` is REQUIRED on the 13 trained tools (highest value: a model will call them) and
  OPTIONAL on `explain` (argument-less, self-describing).
- `example.call` MUST be a complete, valid call -- never trimmed for readability. A trimmed
  example is worse than no example because a model trusts a concrete example over an abstract
  schema and will copy its omissions.
- `example.returns` is OPTIONAL per tool and used ONLY where the return shape is page-independent
  (`navigate` returns a status string; `tabs_context_mcp` returns the tab list; `explain` returns
  the policy text). For page-dependent returns (`read_page`, `get_page_text`, `find`,
  `computer screenshot`), `returns` is OMITTED -- a stale return-shape doc is worse than none, and
  the live response teaches the shape empirically after one call.
- EXCEPTION: `read_page` carries an `example.returns` NOT to document a full shape (page-dependent)
  but to pin one page-independent INVARIANT -- element refs of the form `ref_N`, addressable as
  `form_input.ref` and `computer.ref`. This closes the inference gap a model otherwise has to make
  on its first `form_input` call.

### Decision 3: WITHDRAWN -- the two description strings are distinct and both load-bearing

This decision was originally "delete the directory's per-variant `description` field; tools.json
is the only prose source." Implementation review (onboarding-1 planning) discovered this is wrong:
the directory's per-variant `description` is NOT parallel documentation to tools.json's
description. It is the production source for `explain_text()` (`src/browser/directory.rs:405-433`),
the `explain` tool's response body, and that output is golden-pinned (pinned by
`directory.rs:782-816` and exercised by every `policy explain` integration test).

The two description strings serve DIFFERENT consumers and are not duplicates of each other:

- `tools.json`'s per-tool `description` -- the agent-facing tool description served in
  `tools/list`. The trained-surface one (ADR-0007) on the 13 trained tools; ghostlight's own on
  `explain`.
- `directory.rs`'s per-variant `description` -- the governance-facing capability description
  rendered into the `explain` tool's response body (e.g. `"navigate: requires read. Load a URL
  in a tab, or go back or forward in its history; a top-level GET."`). One per `computer`
  sub-action; one per single-variant tool.

The revised rule: tools.json is the single source for the `tools/list` contract (name,
description, inputSchema, example); the directory is the single source for the `explain` body
(capability classification + per-variant description). Both are kept. The original "Venn overlap"
framing was a misread -- there is no overlap, because the two description strings target different
consumers and are consumed by different code paths. If governance later wants a human-facing label
for its own logging, it is still a different field (`governance_label`), never either description.

### Decision 4: hard-fail inputSchema validation with corrective errors, derived from the fixture

inputSchema violations at the `tools/call` entry point are REJECTED before dispatch (hard-fail),
returning a corrective `ToolError` in the same tool-result shape the "Unknown tool" path already
uses (`pipeline.rs:78-80`). This is a behavioral tightening: a missing `tabId` today silently
becomes `None` downstream and surfaces as an extension error with no corrective content; under
this decision it surfaces immediately as a corrective error naming the field and the example
shape. The tightening is strictly better for an untrained model and matches the existing
convention -- the codebase's `ToolError` taxonomy (`src/error.rs`) ALREADY carries a `next_step`
field on every variant (the `InvalidRequest`/`Binary`/`Ipc`/`Extension`/`Cdp`/`Page` builders,
`error.rs:133-184`, plus the `.next_step()` builder at `error.rs:188`). Decision 4 USES that
existing mechanism rather than inventing a parallel one.

Each validation failure returns a two-part message:

1. **What went wrong** -- the specific failure (missing field `tabId`; wrong type; unknown enum
   value; unknown tool name).
2. **What to try next** -- a concrete corrective suggestion, GENERATED from the fixture, when the
   fixture can produce one honestly.

The contract is "error + suggestion when viable," NOT "always attach a suggestion." A fabricated
suggestion is worse than none -- it sends a model down a wrong path confidently. The rule:

- ATTACH a suggestion when the fixture knows enough: missing required field (name the field, its
  type, and the example shape; for a missing `tabId` specifically, append "get one from
  `tabs_context_mcp` first"), wrong type (name the expected type), unknown enum value (list the
  valid values from `inputSchema.enum`), unknown tool name (list the advertised tools).
- DO NOT attach a suggestion for runtime/state failures (tab not found, extension disconnected),
  governance denials (already self-correcting via `Denied (D-xxxxxxxx):` + the `explain` tool --
  do not double it up), or internal errors. Report these cleanly with no invented suggestion.

The suggestion text is GENERATED, never hand-authored per tool: field name and expected type come
from `inputSchema`; enum alternatives come from `inputSchema.enum`; the example shape comes from
the tool's `example.call`; the "get a tabId first" hint is one hard-coded conditional in the error
formatter (the single piece of logic that does not come from the fixture, justified because it
would otherwise require a per-field `suggestion` annotation that is over-engineering for one
field).

The error path is the third consumer of the single source (after `initialize.instructions` and
`tools/list`). It stores no strings of its own and cannot drift.

### Decision 5: the fidelity test grows to pin the whole contract

`tests/tool_schema_fidelity.rs` adds:

1. (existing) the 14 names, the `computer.action` enum order, the `explain` shape -- byte-stable.
2. (new) every one of the 13 trained tools carries an `example.call` that VALIDATES AGAINST ITS
   OWN `inputSchema` (run the example through JSON-schema validation; an example missing a
   required field, or carrying an unknown enum value, fails CI). This makes "trimmed for
   readability" examples mechanically uncommittable -- the exact drift class that would otherwise
   erode the contract over time.
3. (new) `agentGuide` is present with non-empty `summary`, `workflow`, `flow`, `denials`.

Drift is not unlikely -- it is a CI failure.

## Consequences

- One file (`tools.json`) is the complete agent contract. A developer adding or changing a tool
  edits exactly one place; the `initialize` payload, `tools/list`, the validation-error messages,
  and the fidelity test all derive from it.
- A model of any training provenance gets the workflow contract at handshake, a valid example
  shape per tool in `tools/list`, and a self-correcting error when it gets the shape wrong. The
  first-call success rate for untrained models stops being a function of guessing.
- ADR-0007's trained-surface freeze is preserved: `name`, `description`, `inputSchema` on the 13
  trained tools stay byte-identical. The new fields (`example`, `agentGuide`) are additive
  ghostlight content, out of scope of the trained-surface protection, and a model trained on the
  old surface sees the same schemas it learned on (plus additive fields it can ignore).
- The directory's per-variant `description` is KEPT (Decision 3 withdrawn): it remains the source
  for the `explain` tool's response body. tools.json owns the `tools/list` contract; the directory
  owns the `explain` body. Two description strings, two consumers, no overlap.
- inputSchema validation becomes hard-fail at the `tools/call` entry point (Decision 4). Calls
  that today silently propagate malformed arguments to the extension now fail fast with a
  corrective `ToolError`. The cases this catches (missing `tabId`, wrong type, unknown enum
  value, unexpected property) are exactly those that fail downstream today with worse errors.
- Future tools, future guide sections (e.g. an enforced-mode appendix), and a possible future
  `discover` tool (if ever wanted, despite Decision 1 already serving the guide at handshake) all
  inherit the single-source property: add the entry to `tools.json`, everything else flows. No new
  mechanism per feature.
- Two explicit non-goals, recorded to resist future scope creep: NO per-tool "common mistakes"
  appendix (duplicates the workflow contract and the corrective errors; same drift class); NO
  per-tool "when to use vs alternative" matrix (the grouped-by-job layout in `agentGuide.workflow`
  already covers 80% at 10% of the maintenance).
