//! Tool-call dispatch chokepoint -- the single Policy Enforcement Point (PEP).
//!
//! Every `tools/call` passes through [`Governance::decide`] exactly once, before the tool
//! executes. The [`Governance`] facade holds the governance ports (a
//! [`PolicyDecisionPoint`](crate::governance::ports::PolicyDecisionPoint), an
//! [`AuditSink`](crate::governance::ports::AuditSink), and later the browser plugin halves) and is
//! the one place the stage-2 overlay attaches. It replaces the v1.0 no-op `policy_check` / `audit`
//! seams.
//!
//! [`Governance::all_open`] is the ungoverned engine: its decide path is a literal STEP-0
//! short-circuit to [`Decision::Allow`](crate::governance::ports::Decision) that queries no port and
//! resolves no resource, so a session with no manifest and default config is byte-identical to
//! stage 1 (ADR-0013).

use std::sync::Arc;

use crate::governance::ports::{
    AuditSink, Decision, DecisionRequest, EffectiveMode, GoverningResource, PolicyDecisionPoint,
    RwClass,
};

/// The governance facade held at the dispatch chokepoint: the Policy Enforcement Point.
///
/// One instance lives for the whole MCP session. It is either the ungoverned engine
/// ([`Governance::all_open`], holding no port) or a governed overlay holding the ports. The MCP
/// server calls [`Governance::decide`] once per tool call.
pub struct Governance {
    mode: Mode,
}

/// The two shapes of the facade. `AllOpen` holds nothing so its decide path is a zero-cost
/// short-circuit; `Governed` holds the ports that later tasks drive.
enum Mode {
    /// STEP-0: the ungoverned engine. No manifest, default config. Every call is `Allow`.
    AllOpen,
    /// The governed overlay. Populated by later stage-2 tasks; the pure/impure browser plugin
    /// halves (DomainPolicy classify/match, ResourceResolver) attach through builder methods added
    /// by G05/G07/G13.
    Governed(GovernedState),
}

/// The ports a governed facade holds. `dyn` here is deliberate: the decision point has multiple
/// impls (Noop today, Local in stage 2, a future Remote), and the audit sink has multiple impls
/// (file/stderr/syslog, added by G06). Single-impl domain ports stay concrete/generic and attach
/// later, so they are not fields yet (keeping this facade free of unread state).
struct GovernedState {
    pdp: Box<dyn PolicyDecisionPoint>,
    audit: Arc<dyn AuditSink>,
}

impl Governance {
    /// The ungoverned engine: a zero-port facade whose decide path short-circuits to `Allow`.
    /// This is the only facade used in production until the manifest/config tasks land, and it
    /// preserves byte-identical all-open behavior (ADR-0013).
    pub fn all_open() -> Self {
        Self {
            mode: Mode::AllOpen,
        }
    }

    /// A governed facade over the given decision point and audit sink. Not yet used by any
    /// production path; exercised by the facade unit tests. Later tasks add builder methods to
    /// attach the browser plugin's `DomainPolicy` (classify/match) and `ResourceResolver`.
    pub fn governed(pdp: Box<dyn PolicyDecisionPoint>, audit: Arc<dyn AuditSink>) -> Self {
        Self {
            mode: Mode::Governed(GovernedState { pdp, audit }),
        }
    }

    /// The audit sink held by a governed facade, or `None` under all-open. The audit recorder (G06)
    /// emits one record per call through this; this task only holds it.
    pub fn audit_sink(&self) -> Option<&dyn AuditSink> {
        match &self.mode {
            Mode::AllOpen => None,
            Mode::Governed(state) => Some(state.audit.as_ref()),
        }
    }

    /// The single inbound governance decision for one tool call, taken at the dispatch chokepoint
    /// before the tool executes.
    ///
    /// Under [`Mode::AllOpen`] this is a literal STEP-0 short-circuit: it returns
    /// [`Decision::Allow`] without touching any port or resolving any resource, so all-open output
    /// is byte-identical to stage 1. Under [`Mode::Governed`] it asks the held decision point; the
    /// real pipeline (classify -> resolve resource -> grant check -> effective mode) is filled in by
    /// G05/G07/G13/G15, and with the Noop decision point the result is still `Allow`.
    pub fn decide(&self, tool: &str) -> Decision {
        match &self.mode {
            Mode::AllOpen => Decision::Allow { grant_id: None },
            Mode::Governed(state) => {
                // Wiring stub. Placeholder request fields: G05 classifies for a real `rw`, the
                // resolver task resolves the governing resource, G12/G13 supply grants, G15 resolves
                // the effective mode. The Noop PDP ignores them and allows.
                let req = DecisionRequest {
                    grants: Vec::new(),
                    tool: tool.to_string(),
                    rw: RwClass::Observe,
                    resource: GoverningResource::None,
                    mode: EffectiveMode::Observe,
                };
                state.pdp.decide(&req)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::ports::{AuditRecord, NoopPdp};

    /// A sink that drops every record. Lets the tests build a governed facade without pulling in
    /// the G06 sinks. Its `record` is never called here; it exists to satisfy the trait.
    struct NullAuditSink;
    impl AuditSink for NullAuditSink {
        fn record(&self, _record: &AuditRecord) {}
    }

    #[test]
    fn all_open_decide_is_allow_with_no_grant_and_no_sink() {
        let g = Governance::all_open();
        assert!(matches!(
            g.decide("navigate"),
            Decision::Allow { grant_id: None }
        ));
        assert!(g.audit_sink().is_none());
    }

    #[test]
    fn governed_over_noop_still_allows_and_holds_the_sink() {
        let g = Governance::governed(Box::new(NoopPdp), Arc::new(NullAuditSink));
        assert!(matches!(g.decide("navigate"), Decision::Allow { .. }));
        assert!(g.audit_sink().is_some());
    }
}
