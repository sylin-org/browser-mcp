# managed-5 batch: LEDGER

Single source of truth for batch progress. Update after EVERY task (BOOTSTRAP step 5). A fresh
executor resumes from RESUME HERE with no other context.

## RESUME HERE

Batch authored 2026-07-10; red-team re-read against the live tree completed the same day (T1/T2/
T3/T8 verified aligned; T4 caller-integration corrected -- print loop, not a lines vec; T6
precondition corrected -- multiple denial render sites exist, append at the pipeline emission
chokepoint; T7 anchors verified exactly and pinned). T1 DONE (5a02aaa), T2 DONE (c395c42),
T3 DONE (3a64c8f). Next task: T4.

## Status

| Task | Title | Status | Commit | Deviations |
| --- | --- | --- | --- | --- |
| T1 | Bundle `kind` discriminator | DONE | 5a02aaa | none |
| T2 | ManagedStatus sidecar (single writer in managed::activate) | DONE | c395c42 | none |
| T3 | Presentation validation (additive-only limits) | DONE | 3a64c8f | none |
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

### T2 -- ManagedStatus sidecar (c395c42)
- Preconditions verified: activate signature + resolve_managed call; Reconciled/Freshness/
  StaleReason/write_cache present in cache.rs; paths.managed_cache: Option<PathBuf>; chrono is a
  core dep with the clock feature.
- New crates/core/src/governance/managed/status.rs: ManagedStatus struct (v/freshness/stale_reason/
  seq/fetched_at/source/presentation/last_error), from_reconciled with the exact snake_case mapping,
  sidecar_path, write_sidecar (reuses cache::write_cache atomic temp+rename), read_sidecar (None on
  absent/garbage). `pub mod status;` added; activate now best-effort writes the sidecar after
  resolve_managed (warn-and-continue on failure).
- Tests: snake_case_mapping_is_exact, sidecar_round_trips, read_sidecar_absent_or_garbage_is_none;
  extended activate_resolves_a_configured_local_bundle to assert freshness=="fresh", seq==Some(4).
  31 managed tests green (default) + 29 green (--no-default-features air-gap; status.rs touches no
  ureq/rustls). Global gates: workspace tests pass, clippy clean, lightbox 7/7 ok.
- Deviations: none.

### T3 -- Presentation validation (3a64c8f)
- Preconditions verified: Presentation{org_name,rationale,contacts} + Contact{kind,value,label} in
  bundle.rs; verify_and_parse calls verify_bundle then parse_manifest.
- bundle.rs: pub fn validate_presentation with the exact limits (org_name<=120, rationale<=400,
  contacts<=8, kind<=32, value<=256, label<=120 via chars()) and a control-character sweep
  (c<'\u{20}') across every present string field, verbatim error strings. verify_bundle runs it on
  Some(presentation) after the T1 kind check, mapping Err(msg)->BundleError::Claims(msg).
- Tests: oversized_org_name_is_rejected, control_character_in_contact_is_rejected,
  valid_presentation_passes (bundle.rs); bad_presentation_update_keeps_last_known_good (cache.rs,
  seq-6 bad-presentation update refused -> LastKnownGood(UpdateRejected), active seq==5). 45 bundle+
  managed tests green. Global gates: workspace tests pass, clippy clean, lightbox 7/7 ok.
- Deviations: none.
