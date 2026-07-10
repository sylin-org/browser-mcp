// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! License-specific signature material (ADR-0028 Decision 11): the embedded verifying-key TABLE
//! (Ghostlight's own key generations) and the licensing domain context. The composite Ed25519 +
//! ML-DSA-65 primitive itself lives in [`crate::governance::crypto`] (ADR-0055 Implementation
//! Decision 2); this module is the licensing-domain adapter over it, passing `ghostlight/license` as
//! the domain context, so licensing behavior is byte-identical to before the lift and its tests are
//! that refactor's regression guard.

use crate::governance::crypto::{self, GenKey};
use ed25519_dalek::{SigningKey, VerifyingKey};

pub use crate::governance::crypto::{ED_SIG_LEN, MLDSA_SIG_LEN};

/// The public development / evaluation seed (ADR-0028 Decision 2), exactly 32 bytes.
pub const DEV_SEED: &[u8; 32] = b"ghostlight development key gen0!";

/// ML-DSA domain-separation context for LICENSES; the same bytes bind sign and verify.
const LICENSE_CTX: &[u8] = b"ghostlight/license";

/// The embedded verifying-key table (ADR-0028 Decision 2). Generation 0 derives from the public
/// [`DEV_SEED`]; production generations are appended here as byte constants once the founder
/// generates them offline. Adding a generation is a localized edit to this one function.
pub(super) fn verifying_key(keygen: u32) -> Option<GenKey> {
    match keygen {
        0 => Some(GenKey::Ed25519(dev_verifying_key())),
        // Production generations (composite Ed25519 + ML-DSA-65) land here, e.g.:
        //   1 => Some(GenKey::Composite {
        //       ed: crypto::ed_verifying_key(&ED_GEN1)?,
        //       mldsa: Box::new(crypto::mldsa_verifying_key(&MLDSA_GEN1)?),
        //   }),
        _ => None,
    }
}

/// The Ed25519 verifying key of the public development generation (derived, not hardcoded, since the
/// seed is itself public).
fn dev_verifying_key() -> VerifyingKey {
    SigningKey::from_bytes(DEV_SEED).verifying_key()
}

/// Verify license signature material: delegates to the shared composite primitive with the licensing
/// domain context.
pub(super) fn verify(
    key: &GenKey,
    claims_bytes: &[u8],
    sig_ed: &[u8],
    sig_mldsa: Option<&[u8]>,
) -> bool {
    crypto::verify(key, LICENSE_CTX, claims_bytes, sig_ed, sig_mldsa)
}

/// Founder-only offline license-authoring primitives (ADR-0028 Decision 10). Feature-gated: only the
/// founder's air-gapped `license sign` / `license pubkey` build carries these; a release binary only
/// verifies. Also compiled under `#[cfg(test)]` so the round-trip tests can mint fixtures. Delegates
/// to the shared primitive with the licensing domain context.
#[cfg(any(feature = "license-admin", test))]
pub(super) mod admin {
    use super::LICENSE_CTX;
    use crate::governance::crypto::admin as gadmin;
    use crate::governance::crypto::{ED_SIG_LEN, MLDSA_PK_LEN, MLDSA_SIG_LEN};

    pub(crate) fn ed_sign(seed: &[u8; 32], claims_bytes: &[u8]) -> [u8; ED_SIG_LEN] {
        gadmin::ed_sign(seed, claims_bytes)
    }

    pub(crate) fn ed_public(seed: &[u8; 32]) -> [u8; 32] {
        gadmin::ed_public(seed)
    }

    pub(crate) fn mldsa_sign(seed: &[u8; 32], claims_bytes: &[u8]) -> [u8; MLDSA_SIG_LEN] {
        gadmin::mldsa_sign(seed, LICENSE_CTX, claims_bytes)
    }

    pub(crate) fn mldsa_public(seed: &[u8; 32]) -> [u8; MLDSA_PK_LEN] {
        gadmin::mldsa_public(seed)
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
    fn dev_generation_round_trips_through_the_licensing_context() {
        // The gen-0 dev key verifies a signature made over the same claims; this is the licensing
        // adapter's own guard that it wires the shared primitive with the right context.
        let claims = b"license claims bytes";
        let sig = admin::ed_sign(DEV_SEED, claims);
        let key = verifying_key(0).unwrap();
        assert!(verify(&key, claims, &sig, None));
        assert!(!verify(&key, b"tampered", &sig, None));
    }

    #[test]
    fn composite_wrappers_round_trip_under_the_licensing_context() {
        // A stand-in for a future production generation: the license admin wrappers (ed + ml-dsa)
        // must produce material the license `verify` accepts, i.e. the ml-dsa leg is bound to
        // `LICENSE_CTX`. This also keeps the licensing-domain wrappers exercised now that the
        // generic primitive's own round-trip lives in `governance::crypto`.
        let ed_seed = [5u8; 32];
        let mldsa_seed = [6u8; 32];
        let claims = b"composite license claims";
        let key = GenKey::Composite {
            ed: crypto::ed_verifying_key(&admin::ed_public(&ed_seed)).unwrap(),
            mldsa: Box::new(crypto::mldsa_verifying_key(&admin::mldsa_public(&mldsa_seed)).unwrap()),
        };
        let sig_ed = admin::ed_sign(&ed_seed, claims);
        let sig_mldsa = admin::mldsa_sign(&mldsa_seed, claims);
        assert!(verify(&key, claims, &sig_ed, Some(&sig_mldsa)));
        assert!(
            !verify(&key, claims, &sig_ed, None),
            "a composite key requires both legs"
        );
    }
}
