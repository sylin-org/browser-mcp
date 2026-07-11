// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! The signed policy bundle (ADR-0055): a manifest wrapped with a monotonic publish sequence and
//! optional org-authored presentation, signed by the CUSTOMER'S ORG with its own composite keypair.
//!
//! Unlike a license (signed by Ghostlight's embedded keys), a `managed://` policy is signed by the
//! org and verified against the org's PUBLIC key, which the endpoint receives out of band over MDM
//! (ADR-0055 Implementation Decision 1). Ghostlight embeds no policy key. The envelope mirrors the
//! license envelope shape (ADR-0055 Impl. Decision 3) but has NO `keygen` field: the single trust
//! anchor is the configured org key, so whether both signature legs are required is decided by that
//! key's type (Ed25519-only vs composite), not by a table lookup.
//!
//! Because authenticity lives in the SIGNATURE and not the transport (ADR-0055 D7), the exact same
//! bundle bytes verify identically whether fetched over the network, read from a local file, or
//! carried on a USB stick. This module is pure: it defines the format, verifies a bundle against a
//! given org key under the `ghostlight/policy` domain context, and (for the customer signing tool)
//! mints one. It never touches the filesystem, the network, or config; the manifest inside is
//! returned as raw JSON for the existing loader ([`super::document::parse_manifest`]) to validate.

use serde::{Deserialize, Serialize};

use crate::governance::crypto::{self, GenKey};

/// ML-DSA domain-separation context for POLICY bundles; distinct from `ghostlight/license` so a
/// signature minted in one domain can never verify in the other (see [`crate::governance::crypto`]).
const POLICY_CTX: &[u8] = b"ghostlight/policy";

/// Org-authored presentation, additive-only (ADR-0055 D9): the org may add its own voice (name,
/// rationale, how to reach a human) to the Policy Passport and to denials, but never remove any
/// truth-telling surface. The signature covers these fields, so an attacker cannot swap the org's
/// contact for a phishing address without breaking it. Deliberately NOT `deny_unknown_fields`: the
/// presentation format grows additively, and older endpoints ignore fields they do not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Presentation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contacts: Vec<Contact>,
}

/// One org contact channel (email / chat / ticket URL). `kind` is a free string, not an enum, so an
/// org can name a channel type this build does not know without being rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub kind: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// The signed content of a policy bundle: the manifest document, the monotonic publish sequence
/// (the ADR-0055 D6 anti-rollback field), and the optional presentation. Serialized to the exact
/// bytes that are signed and verified. Forward-compatible (no `deny_unknown_fields`): a bundle the
/// org signs with a newer claims field still verifies on an older endpoint, which ignores it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BundleClaims {
    /// Bundle kind discriminator (ADR-0055 Impl.9a): future-proofs the envelope for governed
    /// content beyond policy (saved scripts, break-glass). Serde-defaults to `"policy"` so bundles
    /// signed before this field verify unchanged; verification rejects any other kind.
    #[serde(default = "default_kind")]
    kind: String,
    /// Monotonic publish sequence. The endpoint refuses a bundle whose `seq` is below the one it
    /// already holds (the anti-rollback check lives in the cache, ADR-0055 Phase 2).
    seq: u64,
    /// The manifest document (schema 3), carried as a nested JSON value and validated downstream by
    /// [`super::document::parse_manifest`].
    manifest: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    presentation: Option<Presentation>,
}

/// The default bundle `kind` (ADR-0055 Impl.9a): every bundle this build mints is a `"policy"`
/// bundle, and any bundle predating the `kind` field is treated as one.
fn default_kind() -> String {
    "policy".to_string()
}

/// The on-disk / on-wire envelope: `v` version, base64 signed `claims`, and the signature legs.
/// `sig_mldsa` is absent for an Ed25519-only (evaluation-grade) org key and present for a composite
/// (production) one. Permissive on unknown fields for forward compatibility.
#[derive(Debug, Deserialize)]
struct Envelope {
    v: u32,
    claims: String,
    sig: String,
    #[serde(default)]
    sig_mldsa: Option<String>,
}

/// A verified policy bundle: the publish sequence, the manifest as raw JSON (for the existing
/// loader to parse and validate), and the org-authored presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedBundle {
    pub seq: u64,
    pub manifest_json: String,
    pub presentation: Option<Presentation>,
}

/// Why a policy bundle could not be parsed or verified. Every failure is precise; none panics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BundleError {
    #[error("not a policy bundle envelope: {0}")]
    Envelope(String),
    #[error("unsupported policy bundle version {0}")]
    Version(u32),
    #[error("policy bundle field '{field}' is not valid base64")]
    Base64 { field: &'static str },
    #[error("ed25519 signature has the wrong length")]
    EdSigLen,
    #[error("ml-dsa signature has the wrong length")]
    MldsaSigLen,
    #[error("policy bundle signature verification failed")]
    BadSignature,
    #[error("malformed policy bundle claims: {0}")]
    Claims(String),
    #[error("unsupported policy bundle kind '{0}'")]
    Kind(String),
}

/// Build a policy verifying key from an org's public key bytes: Ed25519-only, or composite when an
/// ML-DSA-65 public key is also provided. `None` if either key's bytes are not a valid point.
pub fn org_key(
    ed_bytes: &[u8; 32],
    mldsa_bytes: Option<&[u8; crypto::MLDSA_PK_LEN]>,
) -> Option<GenKey> {
    let ed = crypto::ed_verifying_key(ed_bytes)?;
    match mldsa_bytes {
        Some(m) => Some(GenKey::Composite {
            ed,
            mldsa: Box::new(crypto::mldsa_verifying_key(m)?),
        }),
        None => Some(GenKey::Ed25519(ed)),
    }
}

/// Enforce additive-only display limits on org-authored presentation (ADR-0055 D9): org voice may
/// add a name, rationale, and contacts, but can never spoof or crowd out truth-telling surfaces. A
/// validly-signed bundle whose presentation exceeds these limits or carries a control character
/// (which could forge extra display lines) is rejected at verification. Character counts are Unicode
/// scalar values (`chars()`), not bytes. Every present string field is swept for control characters
/// (`c < '\u{20}'`, newline included: these surfaces are single-line).
pub fn validate_presentation(p: &Presentation) -> Result<(), String> {
    if p.org_name.as_ref().is_some_and(|s| s.chars().count() > 120) {
        return Err("presentation org_name exceeds 120 characters".to_string());
    }
    if p.rationale.as_ref().is_some_and(|s| s.chars().count() > 400) {
        return Err("presentation rationale exceeds 400 characters".to_string());
    }
    if p.contacts.len() > 8 {
        return Err("presentation lists more than 8 contacts".to_string());
    }
    for c in &p.contacts {
        if c.kind.chars().count() > 32 {
            return Err("presentation contact kind exceeds 32 characters".to_string());
        }
        if c.value.chars().count() > 256 {
            return Err("presentation contact value exceeds 256 characters".to_string());
        }
        if c.label.as_ref().is_some_and(|s| s.chars().count() > 120) {
            return Err("presentation contact label exceeds 120 characters".to_string());
        }
    }
    let mut fields: Vec<&str> = Vec::new();
    if let Some(s) = &p.org_name {
        fields.push(s);
    }
    if let Some(s) = &p.rationale {
        fields.push(s);
    }
    for c in &p.contacts {
        fields.push(&c.kind);
        fields.push(&c.value);
        if let Some(s) = &c.label {
            fields.push(s);
        }
    }
    if fields.iter().any(|s| s.chars().any(|c| c < '\u{20}')) {
        return Err("presentation contains a control character".to_string());
    }
    Ok(())
}

/// Parse and verify one policy bundle's bytes against `key` under the `ghostlight/policy` context.
/// A composite `key` requires both signature legs; an Ed25519-only key requires the ed leg and
/// rejects a stray ml-dsa leg (see [`crate::governance::crypto::verify`]). Never panics on any
/// input; every failure is a precise [`BundleError`].
pub fn verify_bundle(bytes: &[u8], key: &GenKey) -> Result<VerifiedBundle, BundleError> {
    let env: Envelope =
        serde_json::from_slice(bytes).map_err(|e| BundleError::Envelope(e.to_string()))?;
    if env.v != 1 {
        return Err(BundleError::Version(env.v));
    }
    let claims_bytes =
        crate::b64::decode(&env.claims).ok_or(BundleError::Base64 { field: "claims" })?;
    let sig = crate::b64::decode(&env.sig).ok_or(BundleError::Base64 { field: "sig" })?;
    if sig.len() != crypto::ED_SIG_LEN {
        return Err(BundleError::EdSigLen);
    }
    let sig_mldsa = match &env.sig_mldsa {
        Some(s) => {
            let d = crate::b64::decode(s).ok_or(BundleError::Base64 { field: "sig_mldsa" })?;
            if d.len() != crypto::MLDSA_SIG_LEN {
                return Err(BundleError::MldsaSigLen);
            }
            Some(d)
        }
        None => None,
    };
    if !crypto::verify(key, POLICY_CTX, &claims_bytes, &sig, sig_mldsa.as_deref()) {
        return Err(BundleError::BadSignature);
    }
    let claims: BundleClaims =
        serde_json::from_slice(&claims_bytes).map_err(|e| BundleError::Claims(e.to_string()))?;
    if claims.kind != "policy" {
        return Err(BundleError::Kind(claims.kind));
    }
    if let Some(p) = &claims.presentation {
        validate_presentation(p).map_err(BundleError::Claims)?;
    }
    let manifest_json =
        serde_json::to_string(&claims.manifest).map_err(|e| BundleError::Claims(e.to_string()))?;
    Ok(VerifiedBundle {
        seq: claims.seq,
        manifest_json,
        presentation: claims.presentation,
    })
}

// --- ASCII-armored block (the transport-agnostic copy/paste + sneakernet form of ADR-0055 D7; the
// generic wrapper is `crate::armor`, shared with the license) --------------------------------------

/// The armor label for policy bundles.
const ARMOR_LABEL: &str = "GHOSTLIGHT POLICY";

/// Wrap envelope JSON bytes as an ASCII-armored policy block. The armored payload decodes to the
/// EXACT envelope bytes, so both forms verify identically.
pub fn armor(envelope_json: &[u8]) -> String {
    crate::armor::wrap(ARMOR_LABEL, envelope_json)
}

/// Extract envelope JSON bytes from an ASCII-armored policy block, or `None` if the markers are
/// absent or the body is not valid base64. Whitespace between the markers is ignored.
pub fn dearmor(block: &str) -> Option<Vec<u8>> {
    crate::armor::unwrap(ARMOR_LABEL, block)
}

/// True when the input looks like an armored policy block (vs. a raw JSON envelope).
pub fn is_armored(s: &str) -> bool {
    crate::armor::is_armored(ARMOR_LABEL, s)
}

/// Mint a signed policy bundle over `manifest` at publish sequence `seq`. Ed25519-only when
/// `mldsa_seed` is `None`, composite when present. Always compiled (ADR-0055 Phase 1d): the
/// customer-facing `ghostlight policy sign` command mints bundles in a normal build.
pub fn sign_bundle(
    ed_seed: &[u8; 32],
    mldsa_seed: Option<&[u8; 32]>,
    seq: u64,
    manifest: serde_json::Value,
    presentation: Option<Presentation>,
) -> Vec<u8> {
    let claims = BundleClaims {
        kind: default_kind(),
        seq,
        manifest,
        presentation,
    };
    let claims_bytes = serde_json::to_vec(&claims).expect("BundleClaims serializes");
    let sig = crypto::admin::ed_sign(ed_seed, &claims_bytes);
    let mut env = serde_json::Map::new();
    env.insert("v".into(), serde_json::json!(1));
    env.insert(
        "claims".into(),
        serde_json::json!(crate::b64::encode(&claims_bytes)),
    );
    env.insert("sig".into(), serde_json::json!(crate::b64::encode(&sig)));
    if let Some(mseed) = mldsa_seed {
        let sig_mldsa = crypto::admin::mldsa_sign(mseed, POLICY_CTX, &claims_bytes);
        env.insert(
            "sig_mldsa".into(),
            serde_json::json!(crate::b64::encode(&sig_mldsa)),
        );
    }
    serde_json::to_vec(&env).expect("envelope serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> serde_json::Value {
        serde_json::json!({
            "schema": 3,
            "name": "acme-baseline",
            "version": "1",
            "grants": []
        })
    }

    fn sample_presentation() -> Presentation {
        Presentation {
            org_name: Some("Acme Security".into()),
            rationale: Some("Baseline browser automation policy.".into()),
            contacts: vec![Contact {
                kind: "email".into(),
                value: "security@acme.example".into(),
                label: None,
            }],
        }
    }

    /// Assemble an Ed25519-only envelope over arbitrary hand-built claims bytes, exactly as
    /// `sign_bundle` frames it (used to forge legacy / unknown-kind claims the signer never mints).
    fn ed_envelope_from_claims(ed_seed: &[u8; 32], claims_bytes: &[u8]) -> Vec<u8> {
        let sig = crypto::admin::ed_sign(ed_seed, claims_bytes);
        let mut env = serde_json::Map::new();
        env.insert("v".into(), serde_json::json!(1));
        env.insert(
            "claims".into(),
            serde_json::json!(crate::b64::encode(claims_bytes)),
        );
        env.insert("sig".into(), serde_json::json!(crate::b64::encode(&sig)));
        serde_json::to_vec(&env).expect("envelope serializes")
    }

    #[test]
    fn kind_defaults_to_policy_for_old_claims() {
        let ed_seed = [31u8; 32];
        let claims = serde_json::json!({ "seq": 1, "manifest": sample_manifest() });
        let claims_bytes = serde_json::to_vec(&claims).unwrap();
        let bytes = ed_envelope_from_claims(&ed_seed, &claims_bytes);
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        assert!(verify_bundle(&bytes, &key).is_ok());
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let ed_seed = [32u8; 32];
        let claims =
            serde_json::json!({ "kind": "script", "seq": 1, "manifest": sample_manifest() });
        let claims_bytes = serde_json::to_vec(&claims).unwrap();
        let bytes = ed_envelope_from_claims(&ed_seed, &claims_bytes);
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        assert_eq!(
            verify_bundle(&bytes, &key),
            Err(BundleError::Kind("script".to_string()))
        );
    }

    #[test]
    fn oversized_org_name_is_rejected() {
        let ed_seed = [51u8; 32];
        let p = Presentation {
            org_name: Some("x".repeat(121)),
            rationale: None,
            contacts: vec![],
        };
        let bytes = sign_bundle(&ed_seed, None, 1, sample_manifest(), Some(p));
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        assert_eq!(
            verify_bundle(&bytes, &key),
            Err(BundleError::Claims(
                "presentation org_name exceeds 120 characters".to_string()
            ))
        );
    }

    #[test]
    fn control_character_in_contact_is_rejected() {
        let ed_seed = [52u8; 32];
        let p = Presentation {
            org_name: Some("Acme Security".into()),
            rationale: None,
            contacts: vec![Contact {
                kind: "email".into(),
                value: "mailto:a@b\n".into(),
                label: None,
            }],
        };
        let bytes = sign_bundle(&ed_seed, None, 1, sample_manifest(), Some(p));
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        assert_eq!(
            verify_bundle(&bytes, &key),
            Err(BundleError::Claims(
                "presentation contains a control character".to_string()
            ))
        );
    }

    #[test]
    fn valid_presentation_passes() {
        let ed_seed = [53u8; 32];
        let bytes = sign_bundle(&ed_seed, None, 1, sample_manifest(), Some(sample_presentation()));
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        assert!(verify_bundle(&bytes, &key).is_ok());
    }

    #[test]
    fn ed_only_bundle_round_trips() {
        let ed_seed = [21u8; 32];
        let bytes = sign_bundle(&ed_seed, None, 7, sample_manifest(), Some(sample_presentation()));
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        let v = verify_bundle(&bytes, &key).expect("verifies");
        assert_eq!(v.seq, 7);
        assert_eq!(v.presentation.as_ref().unwrap().org_name.as_deref(), Some("Acme Security"));
        assert!(v.manifest_json.contains("acme-baseline"));
    }

    #[test]
    fn composite_bundle_round_trips() {
        let ed_seed = [22u8; 32];
        let mldsa_seed = [23u8; 32];
        let bytes = sign_bundle(&ed_seed, Some(&mldsa_seed), 42, sample_manifest(), None);
        let key = org_key(
            &crypto::admin::ed_public(&ed_seed),
            Some(&crypto::admin::mldsa_public(&mldsa_seed)),
        )
        .unwrap();
        let v = verify_bundle(&bytes, &key).expect("verifies");
        assert_eq!(v.seq, 42);
        assert!(v.presentation.is_none());
    }

    #[test]
    fn wrong_org_key_is_rejected() {
        let bytes = sign_bundle(&[1u8; 32], None, 1, sample_manifest(), None);
        let other = org_key(&crypto::admin::ed_public(&[2u8; 32]), None).unwrap();
        assert_eq!(verify_bundle(&bytes, &other), Err(BundleError::BadSignature));
    }

    #[test]
    fn tampered_claims_fail_verification() {
        let ed_seed = [24u8; 32];
        let bytes = sign_bundle(&ed_seed, None, 1, sample_manifest(), None);
        // Flip one character inside the base64 claims VALUE to a different valid base64 char, so it
        // still decodes but the signed bytes differ -> the signature no longer matches. (Mutating
        // the envelope via serde targets the value precisely, not the surrounding JSON structure.)
        let mut env: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let mut chars: Vec<char> = env["claims"].as_str().unwrap().chars().collect();
        let mid = chars.len() / 2;
        chars[mid] = if chars[mid] == 'A' { 'B' } else { 'A' };
        env["claims"] = serde_json::Value::String(chars.into_iter().collect());
        let tampered = serde_json::to_vec(&env).unwrap();
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        assert_eq!(verify_bundle(&tampered, &key), Err(BundleError::BadSignature));
    }

    #[test]
    fn ed_only_bundle_under_a_composite_key_is_rejected() {
        // An org that provisioned a composite (production) key must not accept an ed-only bundle:
        // the missing ml-dsa leg fails the AND-composition.
        let ed_seed = [25u8; 32];
        let mldsa_seed = [26u8; 32];
        let bytes = sign_bundle(&ed_seed, None, 1, sample_manifest(), None);
        let composite = org_key(
            &crypto::admin::ed_public(&ed_seed),
            Some(&crypto::admin::mldsa_public(&mldsa_seed)),
        )
        .unwrap();
        assert_eq!(verify_bundle(&bytes, &composite), Err(BundleError::BadSignature));
    }

    #[test]
    fn armor_round_trips_through_verification() {
        let ed_seed = [27u8; 32];
        let bytes = sign_bundle(&ed_seed, None, 3, sample_manifest(), None);
        let block = armor(&bytes);
        assert!(is_armored(&block));
        let recovered = dearmor(&block).expect("dearmor");
        assert_eq!(recovered, bytes, "armored payload is the exact envelope");
        let key = org_key(&crypto::admin::ed_public(&ed_seed), None).unwrap();
        assert!(verify_bundle(&recovered, &key).is_ok());
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        let key = org_key(&crypto::admin::ed_public(&[9u8; 32]), None).unwrap();
        assert!(matches!(
            verify_bundle(b"not json", &key),
            Err(BundleError::Envelope(_))
        ));
        assert!(matches!(
            verify_bundle(b"{}", &key),
            Err(BundleError::Envelope(_))
        ));
    }

    #[test]
    fn wrong_version_is_rejected() {
        let env = serde_json::json!({ "v": 2, "claims": "AA", "sig": "AA" });
        let key = org_key(&crypto::admin::ed_public(&[9u8; 32]), None).unwrap();
        let bytes = serde_json::to_vec(&env).unwrap();
        assert_eq!(verify_bundle(&bytes, &key), Err(BundleError::Version(2)));
    }
}
