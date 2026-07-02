//! Governance core -- the domain-agnostic policy layer.
//!
//! This bounded context (see docs/design/ghostlight-service-architecture.md section 3)
//! names no browser type. It owns the dispatch seam ([`dispatch`]), the typed config
//! registry ([`policy`]), and the policy-decision-point/policy-enforcement-point contract
//! ([`ports`]). The dependency direction is strictly inward: infra and the browser plugin
//! may depend on this module; this module depends only on std and serde. A fail-closed
//! arch-test (task A7) will enforce that.

pub mod dispatch;
pub mod policy;
pub mod ports;
