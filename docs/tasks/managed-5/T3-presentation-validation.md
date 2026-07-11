# T3: Presentation validation -- additive-only limits (ADR-0055 D9 / Impl.8)

## Goal
Org voice can never spoof or crowd out truth-telling surfaces: a validly-signed bundle whose
presentation exceeds sane display limits or contains control characters is REJECTED at
verification (REACHABLE-BUT-BAD -> the reconcile keeps last-known-good; no new failure mode).

## Preconditions (verify, else STOP)
- `crates/core/src/governance/manifest/bundle.rs` has `pub struct Presentation { pub org_name:
  Option<String>, pub rationale: Option<String>, pub contacts: Vec<Contact> }` and `pub struct
  Contact { pub kind: String, pub value: String, pub label: Option<String> }`.
- `crates/core/src/governance/managed/mod.rs` `verify_and_parse` calls `bundle::verify_bundle`
  then `parse_manifest`.

## Required behavior
1. In bundle.rs add (public, doc-commented, cites ADR-0055 D9):
   `pub fn validate_presentation(p: &Presentation) -> Result<(), String>` enforcing EXACTLY:
   - org_name: chars() count <= 120; rationale <= 400; contacts.len() <= 8;
   - per Contact: kind <= 32, value <= 256, label <= 120 (chars() count);
   - EVERY present string field (org_name, rationale, kind, value, label): reject if any char
     `c < '\u{20}'` (control characters, incl. newline -- single-line display surfaces).
   - Error strings, verbatim format: `"presentation org_name exceeds 120 characters"`,
     `"presentation rationale exceeds 400 characters"`, `"presentation lists more than 8 contacts"`,
     `"presentation contact kind exceeds 32 characters"`, `"presentation contact value exceeds 256
     characters"`, `"presentation contact label exceeds 120 characters"`, `"presentation contains a
     control character"`.
2. In `verify_bundle`, after the T1 kind check and before returning `VerifiedBundle`: if
   `claims.presentation` is Some, run `validate_presentation`; on Err(msg) return
   `Err(BundleError::Claims(msg))` (reuses the existing variant; no new variant).

## Tests (bundle.rs `mod tests`; pinned)
- `oversized_org_name_is_rejected`: sign a bundle whose presentation org_name is
  `"x".repeat(121)`; assert verify_bundle returns
  `Err(BundleError::Claims("presentation org_name exceeds 120 characters".to_string()))`.
- `control_character_in_contact_is_rejected`: contact value `"mailto:a@b\n"`; assert
  `Err(BundleError::Claims("presentation contains a control character".to_string()))`.
- `valid_presentation_passes`: the existing `sample_presentation()` helper still verifies Ok.
- In managed/cache.rs tests add `bad_presentation_update_keeps_last_known_good`: seed a cache with
  a good bundle seq 5 (reuse `write_cache` + helpers), point the source file at a signed bundle
  seq 6 whose org_name is 121 chars, run `resolve_managed`, assert
  `freshness == Freshness::LastKnownGood(StaleReason::UpdateRejected)` and active seq == 5.

## Verification (literal)
- `cargo test -p ghostlight-core --lib -- governance::manifest::bundle governance::managed`
- Global verification per BOOTSTRAP.

## Out of scope
- No rendering/UI. No changes to Presentation's FIELDS (additive schema growth is a later ADR).

## Commit message (pinned)
`feat(managed): T3 additive-only presentation validation (ADR-0055 D9)`
