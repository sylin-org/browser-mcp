// SPDX-License-Identifier: Apache-2.0 OR MIT
//! H8 (`docs/tasks/hub/H8-web-api-loopback-policy.md`, ADR-0030 Decision 5/9): the
//! `inbound.web.from` decision is produced by `PolicyDecisionPoint::decide` (the PDP), never
//! by any transport-layer check. Drives the pure decision directly, no listener involved.

use ghostlight::governance::inbound::InboundPdp;
use ghostlight::governance::ports::{
    Decision, DecisionRequest, EffectiveMode, GoverningResource, PolicyDecisionPoint,
};

fn request(inbound_source: &str) -> DecisionRequest {
    DecisionRequest {
        grants: Vec::new(),
        tool: String::new(),
        action: None,
        requires: Vec::new(),
        resource: GoverningResource::None,
        manifest_mode: None,
        config_mode: EffectiveMode::Enforce,
        manifest_hash: String::new(),
        inbound_source: Some(inbound_source.to_string()),
    }
}

#[test]
fn inbound_web_from_is_decided_in_the_pdp_on_the_subject() {
    let pdp = InboundPdp::new(vec!["localhost".to_string()]);

    // A member of the allowlist is allowed.
    assert_eq!(
        pdp.decide(&request("localhost")),
        Decision::Allow { grant_id: None }
    );

    // A source that is NOT a member is denied, by the pure PDP `decide`, PINNED rule label and
    // denial_id shape (docs/tasks/hub/PINS.md SS7).
    match pdp.decide(&request("203.0.113.7")) {
        Decision::Deny(denial) => {
            assert_eq!(denial.rule, "inbound/web_from");
            assert!(
                denial.denial_id.starts_with("D-"),
                "denial_id: {}",
                denial.denial_id
            );
            assert_eq!(
                denial.denial_id.len(),
                10,
                "\"D-\" plus 8 lowercase hex: {}",
                denial.denial_id
            );
            assert!(
                denial.denial_id[2..]
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "denial_id: {}",
                denial.denial_id
            );
        }
        other => panic!("expected Decision::Deny, got {other:?}"),
    }
}
