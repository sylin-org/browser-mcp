# managed-5 batch: LEDGER

Single source of truth for batch progress. Update after EVERY task (BOOTSTRAP step 5). A fresh
executor resumes from RESUME HERE with no other context.

## RESUME HERE

Batch authored 2026-07-10; red-team re-read against the live tree completed the same day (T1/T2/
T3/T8 verified aligned; T4 caller-integration corrected -- print loop, not a lines vec; T6
precondition corrected -- multiple denial render sites exist, append at the pipeline emission
chokepoint; T7 anchors verified exactly and pinned). T1 DONE (5a02aaa). Next task: T2.

## Status

| Task | Title | Status | Commit | Deviations |
| --- | --- | --- | --- | --- |
| T1 | Bundle `kind` discriminator | DONE | 5a02aaa | none |
| T2 | ManagedStatus sidecar (single writer in managed::activate) | pending | - | - |
| T3 | Presentation validation (additive-only limits) | pending | - | - |
| T4 | doctor managed line (reads the sidecar) | pending | - | - |
| T5 | explain-tool Policy Passport section | pending | - | - |
| T6 | Denials-as-doors: org contact line | pending | - | - |
| T7 | Audit provenance: policy_seq on tool-call records | pending | - | - |
| T8 | Lightbox scenarios: passport-freshness + sidecar-propagation | pending | - | - |

Status values: `pending` | `in-progress` | `DONE` | `BLOCKED`.

## Log

One entry per task as it closes (or blocks). Number every deviation from the task file.

### T1 -- Bundle `kind` discriminator (5a02aaa)
- Preconditions verified: BundleClaims had exactly seq/manifest/presentation (no kind); BundleError
  had the 7 named variants; verify_bundle + sign_bundle present.
- Implemented per spec: `kind` field first in BundleClaims (serde default_kind), `default_kind()`
  beside the struct, `BundleError::Kind(String)`, kind check in verify_bundle after claims parse,
  `kind: default_kind()` in sign_bundle. Added a `ed_envelope_from_claims` test helper to forge
  legacy/unknown-kind claims the signer never mints.
- Tests `kind_defaults_to_policy_for_old_claims` + `unknown_kind_is_rejected` pass; all 10 bundle
  tests green. Global gates: workspace tests pass, clippy clean, lightbox 7/7 ok.
- Deviations: none.
