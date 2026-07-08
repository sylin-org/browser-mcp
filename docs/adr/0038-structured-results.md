# 0038. Structured results (`structuredContent`) and cost-aware guidance

- Status: Accepted
- Date: 2026-07-06

## Relationship to other decisions

- SUBSTRATE FOR ADR-0035 (`script`): `$prev`/`$N` references resolve exclusively against the
  structured results this ADR defines. This ADR lands before ADR-0035's interpreter.
- BUILDS ON ADR-0034 (declarations in code): each tool declaration gains an optional
  structured-result vocabulary and `outputSchema`; the capability guide gains cost notes
  (Decision 5) under the same "self-describing surface" mandate as ADR-0034 Decision 6.
- SERVES ADR-0036 (`form_fill`) and ADR-0037 (`wait_for`, digests): their result objects are
  delivered under this ADR's mechanism.
- RE-PINS the all-open output-identity invariant: the TEXT content of every existing tool
  result stays byte-identical; `structuredContent` is a sanctioned additive sibling field in
  the result envelope. The invariant's precise wording going forward: text content
  byte-identical; structured content additive.

## Context

The structure exists and is thrown away. The content script's `find` builds
`{ results: [{ ref, role, name, x, y }], more }` and the service worker flattens it to prose
(`Found N element(s): [ref_5] button "Search" at (x, y)`) before the binary ever sees it.
`tabs_context_mcp` stringifies JSON into a text block. Models (and orchestrators) then parse
prose to recover what was a typed object two hops earlier.

Meanwhile MCP grew first-class support for exactly this: tool results may carry
`structuredContent` alongside `content`, and tool declarations may advertise an
`outputSchema`. Modern clients consume the structure; older clients ignore it and read the
text. Adopting it is not a compatibility risk; it is the missing half of the wire contract.

For ADR-0035 specifically this is load-bearing: a `$prev.results.0.ref` reference needs an
addressable object, and parsing rendered prose would validate its own bugs.

## Decision

### Decision 1: the extension preserves structure across its boundary

For tools with a declared structured vocabulary, the extension's response carries a
`structured` field alongside the rendered text. The service maps it verbatim into the MCP
result's `structuredContent`. The rendered text is produced exactly as today, byte-identical.
The renderer and the structure are two views of one source object, built in one place per tool
in the service worker -- never re-derived from each other.

### Decision 2: the v1 structured vocabulary, pinned per tool

| Tool | `structuredContent` shape |
|---|---|
| `find` | `{ "results": [{ "ref", "role", "name", "x", "y" }], "more": bool }` |
| `tabs_context_mcp` | `{ "mcpGroupId", "tabs": [{ "tabId", "title", "url" }] }` |
| `tabs_create_mcp` | `{ "tabId", "tabs": [{ "tabId", "title", "url" }] }` (`tabId` = the created tab) |
| `navigate` | `{ "tabId", "url", "title" }` (final, post-redirect landing) |
| `wait_for` | `{ "found", "elapsed_ms", "ref"?, "settled"?, "peak_mutations"?, "final_rate"? }` (ADR-0037 Decisions 1, 5, 6) |
| `form_fill` | its full result object (ADR-0036 Decision 3) |
| `script` | its compact result object (ADR-0035 Decision 4) |
| mutating `computer` actions / `form_input` | the digest twin: `{ "url_changed"?, "title_changed"?, "focus"?, "mutations", "alert"?, "dialog_appeared"? }` (ADR-0037 Decision 2) |

`read_page` and `get_page_text` deliberately expose NO structured vocabulary in v1: their
structure IS the text, a structured accessibility index would be enormous, and the
read-then-act workflow belongs to `form_fill` and `find`. An index variant is an open question.

### Decision 3: `outputSchema` in the declarations

Tools with a structured vocabulary advertise the matching `outputSchema` in their code
declarations (ADR-0034 Decision 4 co-location: inputSchema, requires, guidance, and now
outputSchema in one place). Clients and models that read it get typed results; the fidelity
snapshot pins it against drift like every other declaration field.

### Decision 4: references resolve against structure only

Restating ADR-0035 Decision 2 from this side of the contract: `$prev`/`$N` resolution reads
`structuredContent` and nothing else. A step whose tool declares no vocabulary offers nothing
to reference; a reference into it fails with the corrective error, which names the tools that
DO carry structure. Prose is never parsed. This is the oracle-hygiene rule applied to the wire:
if the structure is worth referencing, it is worth declaring.

### Decision 5: cost-aware guidance in the capability guide

Models cannot see costs until after paying them. The capability guide and per-tool guidance
(ADR-0034 Decisions 4 and 6) gain static cost notes, pinned v1 wording per tool, for example:

- `get_page_text`: "can return tens of thousands of tokens on document-heavy pages; prefer
  `find` for targeted lookups and `read_page` filter=interactive for form work."
- `computer` `screenshot`: "costs roughly 1,600 tokens per image; prefer `read_page` or
  `find` when you need targets rather than appearance."
- `read_page` full: "large on complex pages; `filter: 'interactive'` is dramatically smaller;
  `diff: true` returns only changes since your last read."
- `script`: "each step still costs a browser round-trip internally; keep scripts under the
  step budget and use `wait_for` between navigation and reads."

A host that warns the model before the expensive call is a host the model can drive well;
this is the cheapest lever in this ADR and ships with the guide text alone.

## Consequences

### Fixed

- Orchestrators (`script`) and models get typed, addressable results; prose parsing dies.
- ADR-0035's reference semantics stand on a real substrate with a pinned per-tool vocabulary.
- Modern MCP clients get `structuredContent`/`outputSchema`; older clients lose nothing.
- Models get told what tools cost before paying, in the same guide that teaches usage.

### Cost

- One source object per structured tool in the service worker, with renderer + structure as
  two views (a refactor of `find`, tabs, and navigate result construction).
- The `structured` field on the extension wire and its passthrough in the binary.
- `outputSchema` fields in the declarations and their fidelity-snapshot coverage.
- The guide cost-note text and its upkeep.

### Preserved invariants

- Text content of every existing tool result: byte-identical. The envelope grows only the
  additive `structuredContent` sibling, and only for tools that declare a vocabulary.
- The extension stays policy-free: `structured` is mechanism, carrying the same facts as the
  text.
- Declarations-in-code as the single source of truth (ADR-0034): outputSchema lives in the
  same declaration row, snapshot-tested with everything else.

## Open questions (deferred)

- **Measured per-domain cost hints** (the service learns that `get_page_text` on a given site
  averages 80KB and coaches accordingly): deferred; static notes first, telemetry never
  leaves the machine regardless (ADR-0028 posture).
- **A structured index variant of `read_page`** (refs + roles + names without the prose
  tree): deferred until a consumer besides curiosity exists.
- **Extension-side schema validation of `structured` payloads** (defense against a drifting
  service worker): deferred; the fidelity snapshot covers declaration drift, and the binary
  treats the payload as opaque.
