# C4: Output provenance

## Goal

Give page-sourced output a uniform structured provenance marker and a service-authored text boundary
without treating content as policy or persisting it.

## Read before editing

- ADR-0038, ADR-0042, ADR-0078
- `crates/core/src/mcp/pipeline.rs`
- `crates/core/src/mcp/outcome.rs`
- session creation and dependency-injection seams
- registry output schemas and page-reading result tests

## Implementation

1. Mint one memory-only session nonce through an injectable source. Do not use a global nonce and do
   not persist it.
2. Mark registry descriptors/results that contain page-sourced output. Add PINS P4 structured
   provenance at the shared service post-processing seam.
3. Wrap only page-authored text with the exact PINS P4 markers after it returns from the browser.
   Keep service confirmations, validation, policy messages, and audit output outside the boundary.
4. Ensure mixed results distinguish service-authored receipt labels from bounded page-authored text.

## Tests

- Stable snapshots with an injected nonce and correct origin/render metadata.
- Two sessions receive different nonces; one session is stable for its lifetime.
- Page content containing fake Ghostlight markers cannot select the real nonce or terminate the
  service-authored boundary.
- Non-page outputs are unchanged.
- No nonce or page payload enters audit or disk.

## Commit

`feat(mcp): mark untrusted page-sourced output (ADR-0078)`
