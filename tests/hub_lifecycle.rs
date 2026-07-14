// SPDX-License-Identifier: Apache-2.0 OR MIT
//! H6 role-wiring guard (ADR-0030 Decision 8). Process-lifecycle and anti-squat coverage lives in
//! the ADR-0056 Lightbox scenario library.

use std::path::Path;

/// PINS.md SS8 wiring guard (text-scan, NOT a live-process test; mirrors
/// `tests/hub_role_wiring.rs`'s own pattern): `start_service`'s SS5.2 role assertion must actually
/// be present in the source. `src/hub/role.rs`'s own unit tests guard the assertion LOGIC.
#[test]
fn supervisor_start_asserts_adapter_role() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates")
        .join("transport")
        .join("src")
        .join("supervisor.rs");
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        source.contains("assert_adapter_role"),
        "src/hub/supervisor.rs must call assert_adapter_role (PINS.md SS8 wiring guard)"
    );
}
