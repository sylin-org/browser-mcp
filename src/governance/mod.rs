//! Governance core -- the domain-agnostic policy layer.
//!
//! This bounded context (see docs/design/ghostlight-service-architecture.md section 3)
//! names no browser type. It owns the dispatch seam ([`dispatch`]), the typed config
//! registry ([`config`]), the policy manifest ([`manifest`]), the audit flight recorder
//! ([`audit`]), and the policy-decision-point/policy-enforcement-point contract ([`ports`]).
//! The dependency direction is strictly inward: infra and the browser plugin may depend on
//! this module; this module depends only on std and serde (plus `uuid`/`chrono`/`sha2` for
//! audit and manifest identity). A fail-closed arch-test (task A7) enforces that.

pub mod audit;
pub mod config;
pub mod dispatch;
pub mod manifest;
pub mod ports;
