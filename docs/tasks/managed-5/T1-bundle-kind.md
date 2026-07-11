# T1: Bundle `kind` discriminator (ADR-0055 Impl.9a)

## Goal
The signed bundle claims gain `kind` (string, serde-default `"policy"`); verification REJECTS any
kind other than `"policy"` with a precise error. This future-proofs the envelope for governed
content (saved scripts, break-glass) without changing any existing behavior.

## Preconditions (verify, else STOP)
- `crates/core/src/governance/manifest/bundle.rs` contains `struct BundleClaims` with exactly the
  fields `seq: u64`, `manifest: serde_json::Value`, `presentation: Option<Presentation>` and NO
  `kind` field. (grep: `rg -n "struct BundleClaims" -A 12 crates/core/src/governance/manifest/bundle.rs`)
- Same file has `pub enum BundleError` with variants `Envelope, Version, Base64, EdSigLen,
  MldsaSigLen, BadSignature, Claims` and `pub fn verify_bundle`, `pub fn sign_bundle`.

## Required behavior (as-of-authoring facts; re-read the file first)
1. Add to `BundleClaims`: `#[serde(default = "default_kind")] kind: String,` placed FIRST in the
   struct; add `fn default_kind() -> String { "policy".to_string() }` beside the struct.
2. `sign_bundle` sets `kind: default_kind()` when building `BundleClaims` (no new parameter).
3. Add `BundleError` variant, verbatim:
   `#[error("unsupported policy bundle kind '{0}'")]\n    Kind(String),`
4. In `verify_bundle`, AFTER parsing `BundleClaims` and BEFORE building `VerifiedBundle`:
   `if claims.kind != "policy" { return Err(BundleError::Kind(claims.kind)); }`

## Tests (add in bundle.rs `mod tests`; names and assertions pinned)
- `kind_defaults_to_policy_for_old_claims`: build claims JSON WITHOUT `kind` by hand (mirror the
  existing test helpers: serialize `{"seq":1,"manifest":{...minimal schema-3...}}`), ed-sign it via
  `crypto::admin::ed_sign`, assemble the envelope exactly as `sign_bundle` does, then
  `verify_bundle(..)` and assert `.is_ok()`.
- `unknown_kind_is_rejected`: same construction but claims include `"kind":"script"`; assert
  `verify_bundle(..) == Err(BundleError::Kind("script".to_string()))` (derive PartialEq already
  present on BundleError).
- Existing tests must pass UNCHANGED (the default fills `kind` on the sign path).

## Verification (literal)
- `cargo test -p ghostlight-core --lib -- governance::manifest::bundle` -> all pass, 0 failed.
- Global verification per BOOTSTRAP.

## Out of scope
- NO new `kind` values, NO plumbing of kind into VerifiedBundle/managed (a later batch consumes it).
- Do not touch cli.rs, managed/, http.rs.

## Commit message (pinned)
`feat(managed): T1 bundle kind discriminator (default policy, reject unknown) (ADR-0055 Impl.9a)`
