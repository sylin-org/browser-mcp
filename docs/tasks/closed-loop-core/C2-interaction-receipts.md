# C2: Interaction receipts

## Goal

Replace the narrow consequence digest with one bounded receipt and recovery vocabulary while
preserving current low-level action latency.

## Read before editing

- ADR-0037, ADR-0038, ADR-0042, ADR-0078
- `extension/lib/observation.js`
- `extension/service-worker.js`
- `crates/core/src/browser/directory.rs`
- `crates/core/src/mcp/outcome.rs`
- the browser typed-error path and existing observation tests

## Implementation

1. Extend the pure observation module to produce PINS P2 from before/after facts and C1 summaries.
2. Keep the fixed approximately 300 ms observation for existing low-level mutating calls.
3. Add bounded success rendering and typed recovery capsules for the pinned blocker kinds.
4. Carry structured receipts through `structuredContent` and pinned output schemas. Preserve
   corrective typed failures rather than flattening all blockers into success.
5. Add target-assurance and outcome categories through neutral result vocabulary. Audit persistence
   is completed in C3 after the semantic path exists.

## Tests

- Every length/count budget in PINS P2.
- URL/title/render/changed-element and alert/status observations.
- No causal or transaction language.
- Ambiguous, stale, covered, dialog, timeout, frame, and missing-target recovery rendering.
- Existing low-level calls perform one sample and do not enter the five-second settle loop.

## Commit

`feat(browser): return bounded interaction receipts (ADR-0078)`
