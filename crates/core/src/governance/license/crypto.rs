// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Composite license-signature crypto (ADR-0028 Decision 11).
//!
//! Every production license carries TWO signatures over the exact same claims bytes: Ed25519 (the
//! classical guard) and ML-DSA-65 / FIPS 204 (the post-quantum leg). Verification for a composite
//! generation requires BOTH to pass (AND-composition), so a forger must break both -- the scheme is
//! only ever as strong as the STRONGER algorithm, whichever way history breaks (Ed25519 falls to a
//! quantum computer; ML-DSA's exposure is its youth). This mirrors the IETF LAMPS composite-signature
//! direction and the hybrid pattern TLS and SSH adopted for their post-quantum rollouts.
//!
//! Generation 0 is the deliberately PUBLIC development/evaluation key (Ed25519 only): anyone can
//! self-sign an evaluation license, which is exactly why a gen-0 license is capped to the evaluation
//! tier (enforced in [`super::resolve_bytes`]) and always stamped -- it can never masquerade as a paid
//! production license. Production generations (1+) use the founder's air-gapped composite keys and
//! are appended to [`verifying_key`] as byte constants once generated (ADR-0028 Decision 10; the
//! signing seeds never enter this repo or CI).
//!
//! Both algorithms are pure Rust: the four-target cross-compile matrix must never grow a C toolchain.

use ed25519_dalek::{Signature as EdSignature, VerifyingKey};
use fips204::ml_dsa_65;
use fips204::traits::{SerDes as _, Verifier as _};

/// The public development / evaluation seed (ADR-0028 Decision 2), exactly 32 bytes.
pub const DEV_SEED: &[u8; 32] = b"ghostlight development key gen0!";

/// ML-DSA domain-separation context; the same bytes bind sign and verify.
const LICENSE_CTX: &[u8] = b"ghostlight/license";

/// Ed25519 signature length.
pub const ED_SIG_LEN: usize = 64;
/// ML-DSA-65 signature length (FIPS 204, `ml_dsa_65::SIG_LEN`).
pub const MLDSA_SIG_LEN: usize = 3309;
/// ML-DSA-65 public-key length (FIPS 204, `ml_dsa_65::PK_LEN`).
pub const MLDSA_PK_LEN: usize = 1952;

/// A verifying key for one generation. Generation 0 is Ed25519-only (the public dev key); production
/// generations are composite (Ed25519 + ML-DSA-65).
pub(super) enum GenKey {
    Ed25519(VerifyingKey),
    // Constructed once the first production (composite) generation is embedded in `verifying_key`;
    // until then only the Ed25519 dev generation exists, so the variant is dead in a release build.
    // The verify path already handles it, so embedding a production key is a localized change.
    #[allow(dead_code)]
    Composite {
        ed: VerifyingKey,
        mldsa: Box<ml_dsa_65::PublicKey>,
    },
}

/// The embedded verifying-key table (ADR-0028 Decision 2). Generation 0 derives from the public
/// [`DEV_SEED`]; production generations are appended here as byte constants once the founder
/// generates them offline. Adding a generation is a localized edit to this one function.
pub(super) fn verifying_key(keygen: u32) -> Option<GenKey> {
    match keygen {
        0 => Some(GenKey::Ed25519(dev_verifying_key())),
        // Production generations (composite Ed25519 + ML-DSA-65) land here, e.g.:
        //   1 => Some(GenKey::Composite {
        //       ed: ed_verifying_key(&ED_GEN1),
        //       mldsa: Box::new(mldsa_verifying_key(&MLDSA_GEN1)?),
        //   }),
        _ => None,
    }
}

/// The Ed25519 verifying key of the public development generation (derived, not hardcoded, since the
/// seed is itself public).
fn dev_verifying_key() -> VerifyingKey {
    ed25519_dalek::SigningKey::from_bytes(DEV_SEED).verifying_key()
}

/// Reconstruct an Ed25519 verifying key from 32 embedded bytes (production generations).
#[allow(dead_code)] // used once the first production generation is embedded
fn ed_verifying_key(bytes: &[u8; 32]) -> Option<VerifyingKey> {
    VerifyingKey::from_bytes(bytes).ok()
}

/// Reconstruct an ML-DSA-65 verifying key from its embedded bytes (production generations).
#[allow(dead_code)] // used once the first production generation is embedded
fn mldsa_verifying_key(bytes: &[u8; MLDSA_PK_LEN]) -> Option<ml_dsa_65::PublicKey> {
    ml_dsa_65::PublicKey::try_from_bytes(*bytes).ok()
}

/// Verify the signature material for a generation. An Ed25519-only generation requires the Ed25519
/// signature and REJECTS any ML-DSA material (a stray `sig_mldsa` on a gen that has no lattice key
/// is a malformed envelope). A composite generation requires BOTH signatures to pass. Never panics.
pub(super) fn verify(
    key: &GenKey,
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
            ed_ok(ed) && mldsa.verify(claims_bytes, &sig_arr, LICENSE_CTX)
        }
    }
}

/// Offline license-authoring primitives (ADR-0028 Decision 10). Feature-gated: only the founder's
/// air-gapped `license sign` / `license pubkey` build carries these; a release binary only verifies.
/// Also compiled under `#[cfg(test)]` so the round-trip tests can mint fixtures.
#[cfg(any(feature = "license-admin", test))]
pub(super) mod admin {
    use super::*;
    use ed25519_dalek::{Signer as _, SigningKey};
    use fips204::traits::{KeyGen as _, Signer as _};

    /// Ed25519 signature over `claims_bytes` from a 32-byte seed (deterministic).
    pub(crate) fn ed_sign(seed: &[u8; 32], claims_bytes: &[u8]) -> [u8; ED_SIG_LEN] {
        SigningKey::from_bytes(seed).sign(claims_bytes).to_bytes()
    }

    /// The Ed25519 public key bytes for a seed (for embedding in [`verifying_key`]).
    pub(crate) fn ed_public(seed: &[u8; 32]) -> [u8; 32] {
        SigningKey::from_bytes(seed).verifying_key().to_bytes()
    }

    /// ML-DSA-65 signature over `claims_bytes` from a 32-byte seed. The hedge seed is fixed so the
    /// signature is reproducible; it is not secret (it guards fault attacks only, per FIPS 204).
    pub(crate) fn mldsa_sign(seed: &[u8; 32], claims_bytes: &[u8]) -> [u8; MLDSA_SIG_LEN] {
        let (_pk, sk) = ml_dsa_65::KG::keygen_from_seed(seed);
        sk.try_sign_with_seed(&[0u8; 32], claims_bytes, LICENSE_CTX)
            .expect("ml-dsa-65 signing over in-memory claims never fails")
    }

    /// The ML-DSA-65 public key bytes for a seed (for embedding in [`verifying_key`]).
    pub(crate) fn mldsa_public(seed: &[u8; 32]) -> [u8; MLDSA_PK_LEN] {
        let (pk, _sk) = ml_dsa_65::KG::keygen_from_seed(seed);
        pk.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_seed_is_exactly_32_bytes() {
        assert_eq!(DEV_SEED.len(), 32);
    }

    #[test]
    fn ed25519_round_trip_and_tamper() {
        let claims = b"the exact claims bytes";
        let sig = admin::ed_sign(DEV_SEED, claims);
        let key = verifying_key(0).unwrap();
        assert!(
            verify(&key, claims, &sig, None),
            "genuine signature verifies"
        );
        assert!(
            !verify(&key, b"tampered claims bytes", &sig, None),
            "tampered claims fail"
        );
        // A stray ML-DSA signature on the Ed25519 dev generation is rejected.
        assert!(
            !verify(&key, claims, &sig, Some(&[0u8; MLDSA_SIG_LEN])),
            "stray mldsa material on an ed25519 generation is rejected"
        );
    }

    #[test]
    fn composite_round_trip_requires_both() {
        // A locally-built composite key stands in for a future production generation.
        let ed_seed = [7u8; 32];
        let mldsa_seed = [9u8; 32];
        let claims = b"composite claims";
        let ed_pub = VerifyingKey::from_bytes(&admin::ed_public(&ed_seed)).unwrap();
        let mldsa_pub =
            ml_dsa_65::PublicKey::try_from_bytes(admin::mldsa_public(&mldsa_seed)).unwrap();
        let key = GenKey::Composite {
            ed: ed_pub,
            mldsa: Box::new(mldsa_pub),
        };
        assert!(matches!(key, GenKey::Composite { .. }));

        let sig_ed = admin::ed_sign(&ed_seed, claims);
        let sig_mldsa = admin::mldsa_sign(&mldsa_seed, claims);

        assert!(
            verify(&key, claims, &sig_ed, Some(&sig_mldsa)),
            "both signatures present and valid"
        );
        assert!(
            !verify(&key, claims, &sig_ed, None),
            "composite generation rejects a missing ml-dsa signature"
        );
        // Break either leg -> the composite fails (AND-composition).
        let bad_ed = admin::ed_sign(&[1u8; 32], claims);
        assert!(
            !verify(&key, claims, &bad_ed, Some(&sig_mldsa)),
            "wrong ed25519 signature fails the composite"
        );
        let bad_mldsa = admin::mldsa_sign(&[2u8; 32], claims);
        assert!(
            !verify(&key, claims, &sig_ed, Some(&bad_mldsa)),
            "wrong ml-dsa signature fails the composite"
        );
    }

    #[test]
    fn mldsa_lengths_match_fips204() {
        assert_eq!(MLDSA_SIG_LEN, ml_dsa_65::SIG_LEN);
        assert_eq!(MLDSA_PK_LEN, ml_dsa_65::PK_LEN);
    }
}
