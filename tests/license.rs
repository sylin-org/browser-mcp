// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Integration coverage for the licensing engine (ADR-0028): a real, committed, signed license
//! fixture must resolve to a valid evaluation license, and both on-disk forms must agree. This runs
//! under default features (no `license-admin`), so it guards against the verify path or the claims
//! serialization drifting away from the committed signature.

use ghostlight::governance::license::{self, LicenseState};

const FIXTURE: &str = "tests/fixtures/license/dev-license.json";

#[test]
fn committed_dev_fixture_is_a_valid_evaluation_license() {
    let bytes = std::fs::read(FIXTURE).expect("the committed dev-license fixture is present");
    let state = license::resolve_bytes(&bytes);
    match &state {
        LicenseState::Valid { claims, keygen } => {
            assert_eq!(*keygen, 0, "signed by the public development generation");
            assert_eq!(claims.tier, "evaluation");
            assert_eq!(claims.licensee, "Ghostlight Development");
            assert!(claims.products.iter().any(|p| p == "browser"));
        }
        other => panic!("expected a valid evaluation license, got {other:?}"),
    }
    // A gen-0 / evaluation license always carries the evaluation stamp -- it can never look like a
    // paid production deployment.
    assert_eq!(license::stamp_for(&state), Some("evaluation"));
}

#[test]
fn fixture_armors_and_dearmors_to_the_same_bytes() {
    let bytes = std::fs::read(FIXTURE).expect("fixture present");
    let block = license::armor(&bytes);
    assert!(license::is_armored(&block));
    let recovered = license::dearmor(&block).expect("dearmor an armored block");
    assert_eq!(
        recovered, bytes,
        "the armored payload is the exact envelope"
    );
    assert!(matches!(
        license::resolve_bytes(&recovered),
        LicenseState::Valid { .. }
    ));
}

#[test]
fn a_single_flipped_byte_invalidates_the_signature() {
    let mut bytes = std::fs::read(FIXTURE).expect("fixture present");
    // Corrupt a byte inside the base64 claims field; verification must fail closed.
    let pos = bytes
        .iter()
        .position(|&b| b == b'e')
        .expect("a base64 char");
    bytes[pos] ^= 0x01;
    assert!(matches!(
        license::resolve_bytes(&bytes),
        LicenseState::Invalid(_)
    ));
}
