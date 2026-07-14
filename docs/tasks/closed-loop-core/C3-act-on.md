# C3: `act_on`

## Goal

Add one semantic browser interaction that resolves, authorizes, acts, observes, and optionally waits
in one MCP roundtrip without hiding ambiguity.

## Read before editing

- ADR-0022, ADR-0034, ADR-0036, ADR-0037, ADR-0038, ADR-0075, ADR-0078
- `crates/core/src/browser/directory.rs`
- `crates/core/src/mcp/pipeline.rs`
- `crates/core/src/mcp/outcome.rs`
- `crates/core/src/mcp/form_fill.rs`
- `crates/core/src/mcp/script.rs`
- `extension/content.js`, `extension/service-worker.js`, and C1/C2 modules
- audit record, builder, and correlation tests under `crates/core/src/governance/`

## Implementation

1. Add `mcp/act_on.rs` with typed nested validation for PINS P3. Register the additive tool using a
   const JSON schema and output schema. Do not touch a trained schema.
2. Reuse the registry's generic `action_key` mechanism for additive multi-variant tools, update its
   stale computer-only comments, then derive the complete RAWX requirements before dispatch. Use
   one parent decision and correlate internal resolution/action/observation steps following
   `form_fill`.
3. Resolve through C1 ranking. Refuse a tied best rank with no mutation and a C2 candidate capsule.
4. Dispatch the pinned actions. Render a short target glow/caption through the existing policy-free
   visual mechanism before action. Do not add a confirmation modal.
5. Take the normal observation. After meaningful activity only, reuse the settle detector for at
   most five seconds. When `expect` exists, evaluate it with the existing `wait_for` semantics.
6. Append content-free `target_assurance` and outcome category to the parent audit record. Add
   setters rather than exposing audit fields. Respect PINS P5 exactly.
7. Update advertised-surface, agent-guide, script/batch, and fidelity oracles additively.

## Tests

- Nested schema correction: target exclusivity, set-value value rule, expect exclusivity/bounds.
- Generic registry action-variant selection, including unknown-action correction, without a
  tool-name-specific pipeline branch for each new action tool.
- RAWX matrix and one parent governance decision for every target/action/expect combination.
- Unique match acts; best-tier tie does not dispatch; stale and framed targets recover truthfully.
- Adaptive settle starts only after meaningful activity and never exceeds five seconds.
- Expect met and expect timeout outcomes.
- Visual target mechanism is pointer-safe, excluded from read/find/capture, and policy-free.
- Script/browser_batch correlation and structured result flow.
- Audit includes assurance/category and excludes every PINS P5 payload.
- Tool count increases by one and the trained snapshot remains byte-stable.
- PINS P7 low-level versus closed-loop journey comparison.

## Commit

`feat(mcp): add governed semantic act_on tool (ADR-0078)`
