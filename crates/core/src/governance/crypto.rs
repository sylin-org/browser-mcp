// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Shared composite-signature crypto: the Ed25519 + ML-DSA-65 (FIPS 204) AND-composition primitive
//! (ADR-0028 Decision 11, generalized per ADR-0055 Implementation Decision 2).
//!
//! Two bounded contexts sign under this one primitive, each with its OWN domain-separation context
//! so a signature minted in one can never verify in the other:
//!   - licensing ([`crate::governance::license`], context `ghostlight/license`): Ghostlight signs a
//!     license with its embedded key generations.
//!   - managed policy ([`crate::governance::manifest::bundle`], context `ghostlight/policy`,
//!     ADR-0055): the CUSTOMER'S ORG signs its own policy bundle with its own keypair; the endpoint
//!     verifies against the org public key provisioned via MDM. Ghostlight embeds no policy key.
//!
//! A composite key requires BOTH signatures to pass (AND-composition), so a forger must break both;
//! the scheme is only ever as strong as the STRONGER algorithm, whichever way history breaks
//! (Ed25519 falls to a quantum computer; ML-DSA's exposure is its youth). Both algorithms are pure
//! Rust: the four-target cross-compile matrix must never grow a C toolchain.
//!
//! The domain context binds only the ML-DSA leg (FIPS 204 takes a context string natively); the
//! Ed25519 leg signs the bare claims, exactly as the original licensing primitive did, so this lift
//! preserves licensing behavior byte-for-byte. Cross-domain replay is additionally prevented by the
//! disjoint claims schemas (a license `Claims` never parses as a policy bundle, or vice versa) and,
//! decisively, by disjoint keys (an org policy key is never a Ghostlight license key).

use ed25519_dalek::{Signature as EdSignature, VerifyingKey};
use fips204::ml_dsa_65;
use fips204::traits::{SerDes as _, Verifier as _};

/// Ed25519 signature length.
pub const ED_SIG_LEN: usize = 64;
/// ML-DSA-65 signature length (FIPS 204, `ml_dsa_65::SIG_LEN`).
pub const MLDSA_SIG_LEN: usize = 3309;
/// ML-DSA-65 public-key length (FIPS 204, `ml_dsa_65::PK_LEN`).
pub const MLDSA_PK_LEN: usize = 1952;

/// A verifying key: either Ed25519-only (the public licensing dev generation, or an evaluation-grade
/// org policy key) or composite (Ed25519 + ML-DSA-65, used by production license generations and by
/// production org policy keys).
pub enum GenKey {
    Ed25519(VerifyingKey),
    Composite {
        ed: VerifyingKey,
        mldsa: Box<ml_dsa_65::PublicKey>,
    },
}

/// Reconstruct an Ed25519 verifying key from 32 bytes.
pub fn ed_verifying_key(bytes: &[u8; 32]) -> Option<VerifyingKey> {
    VerifyingKey::from_bytes(bytes).ok()
}

/// Reconstruct an ML-DSA-65 verifying key from its `MLDSA_PK_LEN` bytes.
pub fn mldsa_verifying_key(bytes: &[u8; MLDSA_PK_LEN]) -> Option<ml_dsa_65::PublicKey> {
    ml_dsa_65::PublicKey::try_from_bytes(*bytes).ok()
}

/// Verify signature material for `key` over `claims_bytes`, the ML-DSA leg bound to the
/// domain-separation `ctx`. An Ed25519-only key requires the Ed25519 signature and REJECTS any
/// ML-DSA material (a stray `sig_mldsa` on a key with no lattice leg is a malformed envelope). A
/// composite key requires BOTH signatures to pass. Never panics on any input.
pub fn verify(
    key: &GenKey,
    ctx: &[u8],
    claims_bytes: &[u8],
    sig_ed: &[u8],
    sig_mldsa: Option<&[u8]>,
) -> bool {
    let ed_ok = |vk: &VerifyingKey| -> bool {
        let Ok(arr) = <[u8; ED_SIG_LEN]>::try_from(sig_ed) else {
            return false;
        };
        vk.verify_strict(claims_bytes, &EdSignature::from_bytes(&arr))
            .is_ok()
    };
    match key {
        GenKey::Ed25519(vk) => sig_mldsa.is_none() && ed_ok(vk),
        GenKey::Composite { ed, mldsa } => {
            let Some(mldsa_sig) = sig_mldsa else {
                return false;
            };
            let Ok(sig_arr) = <[u8; MLDSA_SIG_LEN]>::try_from(mldsa_sig) else {
                return false;
            };
            ed_ok(ed) && mldsa.verify(claims_bytes, &sig_arr, ctx)
        }
    }
}

/// Offline signing primitives. Gated identically to the founder's licensing authoring build for now
/// (ADR-0028 Decision 10); ADR-0055 Phase 1d ungates these once the CUSTOMER-facing `ghostlight
/// policy sign` command needs them in a normal build (having the sign code compiled does not enable
/// forgery -- a forger still needs the private seed). Also compiled under `#[cfg(test)]` so the
/// round-trip tests can mint fixtures.
#[cfg(any(feature = "license-admin", test))]
pub mod admin {
    use super::*;
    use ed25519_dalek::{Signer as _, SigningKey};
    use fips204::traits::{KeyGen as _, Signer as _};

    /// Ed25519 signature over `claims_bytes` from a 32-byte seed (deterministic; no domain context,
    /// matching the classical leg's original behavior).
    pub fn ed_sign(seed: &[u8; 32], claims_bytes: &[u8]) -> [u8; ED_SIG_LEN] {
        SigningKey::from_bytes(seed).sign(claims_bytes).to_bytes()
    }

    /// The Ed25519 public key bytes for a seed (for embedding or for an org public key).
    pub fn ed_public(seed: &[u8; 32]) -> [u8; 32] {
        SigningKey::from_bytes(seed).verifying_key().to_bytes()
    }

    /// ML-DSA-65 signature over `claims_bytes` from a 32-byte seed, bound to the domain `ctx`. The
    /// hedge seed is fixed so the signature is reproducible; it is not secret (FIPS 204 fault-attack
    /// guard only).
    pub fn mldsa_sign(seed: &[u8; 32], ctx: &[u8], claims_bytes: &[u8]) -> [u8; MLDSA_SIG_LEN] {
        let (_pk, sk) = ml_dsa_65::KG::keygen_from_seed(seed);
        sk.try_sign_with_seed(&[0u8; 32], claims_bytes, ctx)
            .expect("ml-dsa-65 signing over in-memory claims never fails")
    }

    /// The ML-DSA-65 public key bytes for a seed (for embedding or for an org public key).
    pub fn mldsa_public(seed: &[u8; 32]) -> [u8; MLDSA_PK_LEN] {
        let (pk, _sk) = ml_dsa_65::KG::keygen_from_seed(seed);
        pk.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_round_trip_and_tamper() {
        let ctx = b"ghostlight/test";
        let seed = [3u8; 32];
        let claims = b"the exact claims bytes";
        let sig = admin::ed_sign(&seed, claims);
        let key = GenKey::Ed25519(ed_verifying_key(&admin::ed_public(&seed)).unwrap());
        assert!(verify(&key, ctx, claims, &sig, None), "genuine signature verifies");
        assert!(
            !verify(&key, ctx, b"tampered claims bytes", &sig, None),
            "tampered claims fail"
        );
        // A stray ML-DSA signature on an Ed25519-only key is rejected.
        assert!(
            !verify(&key, ctx, claims, &sig, Some(&[0u8; MLDSA_SIG_LEN])),
            "stray mldsa material on an ed25519 key is rejected"
        );
    }

    #[test]
    fn composite_round_trip_requires_both() {
        let ctx = b"ghostlight/policy";
        let ed_seed = [7u8; 32];
        let mldsa_seed = [9u8; 32];
        let claims = b"composite claims";
        let key = GenKey::Composite {
            ed: ed_verifying_key(&admin::ed_public(&ed_seed)).unwrap(),
            mldsa: Box::new(mldsa_verifying_key(&admin::mldsa_public(&mldsa_seed)).unwrap()),
        };

        let sig_ed = admin::ed_sign(&ed_seed, claims);
        let sig_mldsa = admin::mldsa_sign(&mldsa_seed, ctx, claims);

        assert!(
            verify(&key, ctx, claims, &sig_ed, Some(&sig_mldsa)),
            "both signatures present and valid"
        );
        assert!(
            !verify(&key, ctx, claims, &sig_ed, None),
            "composite key rejects a missing ml-dsa signature"
        );
        // Break either leg -> the composite fails (AND-composition).
        let bad_ed = admin::ed_sign(&[1u8; 32], claims);
        assert!(
            !verify(&key, ctx, claims, &bad_ed, Some(&sig_mldsa)),
            "wrong ed25519 signature fails the composite"
        );
        let bad_mldsa = admin::mldsa_sign(&[2u8; 32], ctx, claims);
        assert!(
            !verify(&key, ctx, claims, &sig_ed, Some(&bad_mldsa)),
            "wrong ml-dsa signature fails the composite"
        );
    }

    #[test]
    fn mldsa_context_domain_separates() {
        // A composite signature minted under one context must NOT verify under another -- the
        // property that keeps a license signature from ever validating as a policy bundle.
        let ed_seed = [11u8; 32];
        let mldsa_seed = [13u8; 32];
        let claims = b"same bytes, different domain";
        let key = GenKey::Composite {
            ed: ed_verifying_key(&admin::ed_public(&ed_seed)).unwrap(),
            mldsa: Box::new(mldsa_verifying_key(&admin::mldsa_public(&mldsa_seed)).unwrap()),
        };
        let sig_ed = admin::ed_sign(&ed_seed, claims);
        let sig_license = admin::mldsa_sign(&mldsa_seed, b"ghostlight/license", claims);
        assert!(
            verify(&key, b"ghostlight/license", claims, &sig_ed, Some(&sig_license)),
            "verifies under its own context"
        );
        assert!(
            !verify(&key, b"ghostlight/policy", claims, &sig_ed, Some(&sig_license)),
            "the same signature is rejected under a different context"
        );
    }

    #[test]
    fn mldsa_lengths_match_fips204() {
        assert_eq!(MLDSA_SIG_LEN, ml_dsa_65::SIG_LEN);
        assert_eq!(MLDSA_PK_LEN, ml_dsa_65::PK_LEN);
    }
}
