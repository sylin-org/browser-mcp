//! Tool-call dispatch chokepoint -- the single Policy Enforcement Point (PEP).
//!
//! Every `tools/call` passes through [`Governance::decide`] exactly once, before the tool
//! executes, and through [`Governance::record_call`] exactly once after it resolves. The
//! [`Governance`] facade holds the governance ports (a
//! [`PolicyDecisionPoint`](crate::governance::ports::PolicyDecisionPoint), an
//! [`AuditSink`](crate::governance::ports::AuditSink), and later the browser plugin halves) and is
//! the one place the stage-2 overlay attaches.
//!
//! [`Governance::all_open`] is the ungoverned engine: its decide path is a literal STEP-0
//! short-circuit to [`Decision::Allow`](crate::governance::ports::Decision) that queries no port and
//! resolves no resource, so a session with no manifest and default config is byte-identical to
//! stage 1 (ADR-0013). Audit is orthogonal to that STEP-0 short-circuit (shared format doc
//! section 4.5: the flight recorder still records under all-open when `audit.enabled` is true), so
//! the audit sink is a field of `Governance` itself, not nested inside the governed-only state.
//!
//! `classify` is injected as a function pointer rather than named directly: this module lives in
//! the domain-agnostic governance core, and the concrete tool+action classification table is
//! browser-domain (`browser::classify`, g05's RECONCILIATION-driven placement; the a7 arch-test
//! forbids a `governance -> browser` edge). The crate-root binary supplies the browser plugin's
//! real classifier at construction.

use std::sync::{Arc, Mutex, PoisonError};

use crate::governance::ports::{
    AuditRecord, AuditSink, ClientInfo, Decision, DecisionRequest, EffectiveMode,
    GoverningResource, PolicyDecisionPoint, RwClass,
};

/// The governance facade held at the dispatch chokepoint: the Policy Enforcement Point.
///
/// One instance lives for the whole MCP session. The decision path is either the ungoverned
/// engine ([`Governance::all_open`], holding no decision port) or a governed overlay holding the
/// ports; the audit sink and client identity are held regardless, since recording is orthogonal
/// to whether a manifest is active.
pub struct Governance {
    mode: Mode,
    /// Always present; `NullSink` when audit is disabled. Recording is orthogonal to
    /// [`Mode`] (shared format doc section 4.5).
    audit: Arc<dyn AuditSink>,
    /// The browser plugin's tool+action -> observe/mutate table, injected so this core module
    /// never names the browser plugin directly.
    classify: fn(&str, Option<&str>) -> Option<RwClass>,
    /// The MCP client identity captured from the `initialize` request, first-wins for the
    /// whole session (shared format doc section 6.1 `client` field).
    client: Mutex<Option<ClientInfo>>,
}

/// The two shapes of the decision path. `AllOpen` holds nothing so its decide path is a
/// zero-cost short-circuit; `Governed` holds the decision port later tasks drive.
enum Mode {
    /// STEP-0: the ungoverned engine. No manifest, default config. Every call is `Allow`.
    AllOpen,
    /// The governed overlay. Populated by later stage-2 tasks; the pure/impure browser plugin
    /// halves (DomainPolicy classify/match, ResourceResolver) attach through builder methods added
    /// by G07/G13.
    Governed(GovernedState),
}

/// The decision port a governed facade holds. `dyn` here is deliberate: the decision point has
/// multiple impls (Noop today, Local in stage 2, a future Remote).
struct GovernedState {
    pdp: Box<dyn PolicyDecisionPoint>,
}

impl Governance {
    /// The ungoverned engine: a zero-port decision path whose decide path short-circuits to
    /// `Allow`, paired with an audit sink built independently from config (audit is orthogonal
    /// to all-open). This is the facade used in production until the manifest task lands.
    pub fn all_open(
        audit: Arc<dyn AuditSink>,
        classify: fn(&str, Option<&str>) -> Option<RwClass>,
    ) -> Self {
        Self {
            mode: Mode::AllOpen,
            audit,
            classify,
            client: Mutex::new(None),
        }
    }

    /// A governed facade over the given decision point, audit sink, and classifier. Not yet
    /// used by any production path; exercised by the facade unit tests. Later tasks add builder
    /// methods to attach the browser plugin's `DomainPolicy` (classify/match) and
    /// `ResourceResolver`.
    pub fn governed(
        pdp: Box<dyn PolicyDecisionPoint>,
        audit: Arc<dyn AuditSink>,
        classify: fn(&str, Option<&str>) -> Option<RwClass>,
    ) -> Self {
        Self {
            mode: Mode::Governed(GovernedState { pdp }),
            audit,
            classify,
            client: Mutex::new(None),
        }
    }

    /// The audit sink held by this facade. Always present (a disabled configuration holds a
    /// null sink); the audit recorder (G06) is what this points at in production.
    pub fn audit_sink(&self) -> &dyn AuditSink {
        self.audit.as_ref()
    }

    /// The single inbound governance decision for one tool call, taken at the dispatch chokepoint
    /// before the tool executes.
    ///
    /// Under [`Mode::AllOpen`] this is a literal STEP-0 short-circuit: it returns
    /// [`Decision::Allow`] without touching any port or resolving any resource, so all-open output
    /// is byte-identical to stage 1. Under [`Mode::Governed`] it asks the held decision point; the
    /// real pipeline (classify -> resolve resource -> grant check -> effective mode) is filled in by
    /// G07/G13/G15, and with the Noop decision point the result is still `Allow`.
    pub fn decide(&self, tool: &str) -> Decision {
        match &self.mode {
            Mode::AllOpen => Decision::Allow { grant_id: None },
            Mode::Governed(state) => {
                // Wiring stub. Placeholder request fields: the resolver task resolves the
                // governing resource, G12/G13 supply grants, G15 resolves the effective mode.
                // The Noop PDP ignores them and allows.
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

    /// Capture the MCP client identity from the `initialize` request's `clientInfo`
    /// (shared format doc section 6.1 `client` field). First capture wins for the whole
    /// session; a no-op if a client identity is already stored.
    pub fn set_client(&self, name: &str, version: &str) {
        let mut guard = self.client.lock().unwrap_or_else(PoisonError::into_inner);
        if guard.is_none() {
            *guard = Some(ClientInfo {
                name: name.to_string(),
                version: version.to_string(),
            });
        }
    }

    /// Build and record one audit record for a completed tool call (ADR-0018 step 1: the flight
    /// recorder). Called at the dispatch chokepoint after the call resolves, so the record
    /// carries the real duration. `action` is the `computer` sub-action when `tool == "computer"`,
    /// `None` otherwise.
    ///
    /// `identity`, `domain`, `grant_id`, `denial_id`, and `manifest` are always `None` until the
    /// manifest and enforcement tasks (G12/G13) land; `decision` is always `"allow"` until then
    /// (this task adds no enforcement). A classification miss (`self.classify` returns `None`:
    /// an unknown tool, or a `computer` call with a missing or unknown action) records
    /// [`RwClass::Mutate`]: the record vocabulary is only observe/mutate, and an unclassifiable
    /// call must never be presented as harmless observation.
    pub fn record_call(&self, tool: &str, action: Option<&str>, duration_ms: u64) {
        let rw = (self.classify)(tool, action).unwrap_or(RwClass::Mutate);
        let client = self
            .client
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let record = AuditRecord {
            event_id: uuid::Uuid::new_v4().to_string(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            identity: None,
            client,
            tool: tool.to_string(),
            action: action.map(str::to_string),
            rw,
            domain: None,
            decision: "allow",
            grant_id: None,
            denial_id: None,
            duration_ms,
            manifest: None,
        };
        self.audit.record(&record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::ports::NoopPdp;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn no_classification(_tool: &str, _action: Option<&str>) -> Option<RwClass> {
        None
    }

    /// A sink that counts records instead of dropping them, so tests can assert recording
    /// actually happened without pulling in the G06 file/stderr sinks.
    #[derive(Default)]
    struct CountingAuditSink {
        count: AtomicUsize,
    }
    impl AuditSink for CountingAuditSink {
        fn record(&self, _record: &AuditRecord) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A sink that keeps every record, so tests can assert on the actual built fields (`rw`,
    /// `action`, `client`) rather than just call count.
    #[derive(Default)]
    struct CapturingAuditSink {
        records: Mutex<Vec<AuditRecord>>,
    }
    impl AuditSink for CapturingAuditSink {
        fn record(&self, record: &AuditRecord) {
            self.records
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(record.clone());
        }
    }
    impl CapturingAuditSink {
        fn last(&self) -> AuditRecord {
            self.records
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .last()
                .cloned()
                .expect("at least one record was captured")
        }
    }

    /// A stand-in for the browser plugin's real classifier: `computer`/`screenshot` observes,
    /// `computer`/`left_click` mutates, `read_page` observes, everything else misses.
    fn sample_classify(tool: &str, action: Option<&str>) -> Option<RwClass> {
        match (tool, action) {
            ("computer", Some("screenshot")) => Some(RwClass::Observe),
            ("computer", Some("left_click")) => Some(RwClass::Mutate),
            ("read_page", None) => Some(RwClass::Observe),
            _ => None,
        }
    }

    #[test]
    fn all_open_decide_is_allow_and_still_records() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_classification);
        assert!(matches!(
            g.decide("navigate"),
            Decision::Allow { grant_id: None }
        ));
        g.record_call("navigate", None, 5);
        assert_eq!(sink.count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn governed_over_noop_still_allows_and_holds_the_sink() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::governed(Box::new(NoopPdp), sink.clone(), no_classification);
        assert!(matches!(g.decide("navigate"), Decision::Allow { .. }));
        g.record_call("navigate", None, 0);
        assert_eq!(sink.count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn classification_miss_records_mutate() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_classification);
        g.record_call("no_such_tool", None, 0);
        assert_eq!(sink.last().rw, RwClass::Mutate);
        g.record_call("computer", None, 0);
        assert_eq!(
            sink.last().rw,
            RwClass::Mutate,
            "a computer call with no action is also a classification miss"
        );
    }

    #[test]
    fn computer_action_classification_flows_into_rw() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), sample_classify);

        g.record_call("computer", Some("screenshot"), 0);
        let rec = sink.last();
        assert_eq!(rec.rw, RwClass::Observe);
        assert_eq!(rec.action.as_deref(), Some("screenshot"));

        g.record_call("computer", Some("left_click"), 0);
        assert_eq!(sink.last().rw, RwClass::Mutate);

        g.record_call("read_page", None, 0);
        let rec = sink.last();
        assert_eq!(rec.rw, RwClass::Observe);
        assert_eq!(rec.action, None);
    }

    #[test]
    fn set_client_first_capture_wins() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_classification);
        g.set_client("a", "1");
        g.set_client("b", "2");
        let stored = g.client.lock().unwrap();
        assert_eq!(stored.as_ref().unwrap().name, "a");
        assert_eq!(stored.as_ref().unwrap().version, "1");
        drop(stored);

        g.record_call("navigate", None, 0);
        let client = sink.last().client.expect("client info recorded");
        assert_eq!(client.name, "a");
        assert_eq!(client.version, "1");
    }
}
