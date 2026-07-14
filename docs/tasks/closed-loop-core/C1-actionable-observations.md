# C1: Actionable observations

## Goal

Make `find` and targeted `read_page` return one compact, reusable element summary. Add deterministic
semantic ranking without adding a new tool or changing a trained schema.

## Read before editing

- ADR-0007, ADR-0034, ADR-0036, ADR-0038, ADR-0078
- `extension/content.js`
- `extension/service-worker.js`
- `crates/core/src/browser/directory.rs`
- current `find` and `read_page` tests in Rust and `extension/tests/`

## Implementation

1. Extract pure extension helpers for normalized matching, rank tiers, element state, bounded text,
   mechanical actions, and summary serialization. Keep policy words out of this module.
2. Make `find` use the shared matcher and summary. Preserve its current default and maximum result
   count. Ranking is PINS P1; document order breaks ties only for read output.
3. Make targeted `read_page(ref_id)` include the shared summary while retaining its bounded text.
   A full-page read remains compact and does not produce a structured DOM mirror.
4. Extend only additive output schemas and text renderers. Confirm the sacred schema snapshot is
   byte-identical.

## Tests

- Exact, prefix, token, and substring order; role filter; same-tier document order.
- Hidden, disabled, checked, selected, href, box, render serial, and secret-marker behavior.
- Bounds and omission of inapplicable fields.
- Top-document behavior, exclusion of iframe contents, and explicit guidance that framed targets
  are unsupported in this batch.
- Sacred tool schema fidelity unchanged.

## Commit

`feat(browser): make page observations actionable (ADR-0078)`
