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
//! `requires` is injected as a function pointer rather than named directly: this module lives
//! in the domain-agnostic governance core, and the concrete action directory is browser-domain
//! (`browser::directory::requires`, ADR-0022 Decision 2; the a7 arch-test forbids a
//! `governance -> browser` edge). The crate-root binary supplies the browser plugin's real
//! implementation at construction. The audit `capability` field (ADR-0022 Decision 8) is
//! derived from the SAME `requires` slice the caller looked up for `decide`, threaded in by
//! every public record function -- there is no second, browser-supplied fn pointer for audit.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::governance::manifest::document::Grant;
use crate::governance::ports::{
    AuditRecord, AuditSink, Capability, ClientInfo, Decision, DecisionRequest, Denial,
    EffectiveMode, GoverningResource, PolicyDecisionPoint, SessionEventRecord,
};

/// How long a take-the-wheel hold may last before [`hold_message`] appends the resume hint
/// (g10, ADR-0018 step 2). A constant for now; a future registry key
/// (`engine.hold.hint_after_ms`) may make it configurable -- not this task's job.
pub const HOLD_HINT_AFTER: Duration = Duration::from_secs(120);

/// The take-the-wheel pause reply for a held tool call (g10, ADR-0018 step 2): a plain,
/// truthful statement that the call was NOT executed, why, and what the agent should do
/// (stop and wait, never retry-spin), rendered as a normal successful MCP text result --
/// never an error, never a hint that the action happened. `action` is the `computer`
/// sub-action, rendering the label `computer (<action>)`; every other tool renders its bare
/// name (mirrors the denial-format convention, shared format doc section 7.2). Past
/// [`HOLD_HINT_AFTER`], a second sentence names the only way to resume: the user, from the
/// extension.
pub fn hold_message(tool: &str, action: Option<&str>, held_for: Duration) -> String {
    let label = match (tool, action) {
        ("computer", Some(action)) => format!("computer ({action})"),
        _ => tool.to_string(),
    };
    let mut message = format!(
        "Paused: the user has taken control of the browser (take-the-wheel). The '{label}' \
         call was NOT executed. This is not an error, and retrying will not help: every \
         browser tool call receives this same reply until the user resumes. Stop issuing \
         browser tool calls, tell the user the session is paused and you are waiting, and \
         continue only after the user says they have resumed."
    );
    if held_for >= HOLD_HINT_AFTER {
        message.push(' ');
        message.push_str(
            "This session has been paused for more than 2 minutes. Only the user can resume \
             it, from the Browser MCP extension: the popup Pause/Resume button or the toggle \
             keyboard shortcut.",
        );
    }
    message
}

/// The status-surface governance summary (g15, shared format doc section 9.2): the
/// manifest-level effective mode (`manifest_mode.unwrap_or(config_mode)`) and whether shadow
/// enforcement is active. Rendered by `get_status`'s `governance` object and the doctor
/// `Governance:` section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernanceStatus {
    pub mode: EffectiveMode,
    pub shadow: bool,
}

/// The pure computation behind [`GovernanceStatus`] (g15): `mode` is the manifest-level
/// effective mode; `shadow` is true only when `grants` is non-empty AND that mode is
/// `Observe` -- per-grant overrides never change this top-level flag, and an empty `grants`
/// array (a manifest with no policy content yet) is deliberately reported as non-shadow even
/// though an individual would-deny call under it would still be classified `shadow_deny` by
/// [`crate::governance::enforcement::apply_mode`] (the badge describes whether a MEANINGFUL
/// policy is being observed, not the literal per-call decision vocabulary). A free function
/// (not a `Governance` method) so a standalone caller with no live session -- `browser-mcp
/// doctor`, which resolves its own manifest independently -- computes the identical summary
/// [`Governance::governance_status`] does, from the same three inputs.
pub fn governance_status(
    grants: &[Grant],
    manifest_mode: Option<EffectiveMode>,
    config_mode: EffectiveMode,
) -> GovernanceStatus {
    let mode = manifest_mode.unwrap_or(config_mode);
    let shadow = !grants.is_empty() && mode == EffectiveMode::Observe;
    GovernanceStatus { mode, shadow }
}

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
    /// The browser plugin's action directory lookup (ADR-0022 Decision 2), injected so this
    /// core module never names the browser plugin directly. `None` is a directory miss (fail
    /// closed); `Some(&[])` is an unconditionally-allowed action; `Some(reqs)` is the bound
    /// capability requirement set a `DecisionRequest` carries. Consumed by [`Self::decide`].
    /// This is the ONLY browser-supplied fn pointer `Governance` holds: every public record
    /// function now takes its own `requires: &[Capability]` parameter (the caller's own
    /// lookup through this same table, ADR-0022 Decision 8), rather than `Governance` looking
    /// it up a second time for audit purposes.
    requires: fn(&str, Option<&str>) -> Option<&'static [Capability]>,
    /// The MCP client identity captured from the `initialize` request, first-wins for the
    /// whole session (shared format doc section 6.1 `client` field).
    client: Mutex<Option<ClientInfo>>,
}

/// The two shapes of the decision path. `AllOpen` holds nothing so its decide path is a
/// zero-cost short-circuit; `Governed` holds the decision port plus the active manifest's
/// grants and content hash (g13).
enum Mode {
    /// STEP-0: the ungoverned engine. No manifest, default config. Every call is `Allow`.
    AllOpen,
    /// The governed overlay, active once a manifest is loaded (g13).
    Governed(GovernedState),
}

/// The decision port a governed facade holds, plus the request fields that come from the
/// active manifest itself rather than from any one call (g13): the resolved grants (in
/// manifest order; the pure decision core re-resolves the matching grant per call), the
/// manifest's content hash (denial ids are computed from it, shared format doc section 7.1),
/// and the manifest-level `mode` (g15, shared format 4.1: the mode precedence's middle tier,
/// between a resolving grant's own `mode` and the resolved `governance.mode`). `dyn` on the
/// PDP is deliberate: the decision point has multiple impls (Noop today, `LocalPdp` in stage
/// 2, a future Remote).
struct GovernedState {
    pdp: Box<dyn PolicyDecisionPoint>,
    grants: Vec<Grant>,
    manifest_hash: String,
    manifest_mode: Option<EffectiveMode>,
}

impl Governance {
    /// The ungoverned engine: a zero-port decision path whose decide path short-circuits to
    /// `Allow`, paired with an audit sink built independently from config (audit is orthogonal
    /// to all-open). This is the facade used in production until the manifest task lands.
    pub fn all_open(
        audit: Arc<dyn AuditSink>,
        requires: fn(&str, Option<&str>) -> Option<&'static [Capability]>,
    ) -> Self {
        Self {
            mode: Mode::AllOpen,
            audit,
            requires,
            client: Mutex::new(None),
        }
    }

    /// A governed facade over the given decision point, audit sink, action directory lookup,
    /// the active manifest's resolved grants, its content hash (g13), and its own `mode` field,
    /// if any (g15). `transport::mcp::server::run` constructs this with a `LocalPdp` once a
    /// manifest is active; `all_open` stays the facade for a session with no manifest.
    pub fn governed(
        pdp: Box<dyn PolicyDecisionPoint>,
        audit: Arc<dyn AuditSink>,
        requires: fn(&str, Option<&str>) -> Option<&'static [Capability]>,
        grants: Vec<Grant>,
        manifest_hash: String,
        manifest_mode: Option<EffectiveMode>,
    ) -> Self {
        Self {
            mode: Mode::Governed(GovernedState {
                pdp,
                grants,
                manifest_hash,
                manifest_mode,
            }),
            audit,
            requires,
            client: Mutex::new(None),
        }
    }

    /// The audit sink held by this facade. Always present (a disabled configuration holds a
    /// null sink); the audit recorder (G06) is what this points at in production.
    pub fn audit_sink(&self) -> &dyn AuditSink {
        self.audit.as_ref()
    }

    /// True when a manifest is active ([`Mode::Governed`]); false under all-open. The dispatch
    /// chokepoint (`transport::mcp::server`) uses this to skip grant-resource resolution --
    /// including every extension tab-URL round trip it would otherwise make -- entirely under
    /// all-open (g13 constraint 3: STEP 0 must add zero new frames and zero new latency).
    pub fn is_governed(&self) -> bool {
        matches!(self.mode, Mode::Governed(_))
    }

    /// The active manifest's resolved grants (g14, tool advertisement filtering): `None` under
    /// all-open, `Some(&state.grants)` once a manifest is active. Read-only; a static snapshot
    /// captured once at construction, same as everything else `GovernedState` holds -- there is
    /// no live re-resolution yet (see `browser::advertise`'s module doc).
    pub fn grants(&self) -> Option<&[Grant]> {
        match &self.mode {
            Mode::AllOpen => None,
            Mode::Governed(state) => Some(&state.grants),
        }
    }

    /// The status-surface governance summary (g15, shared format doc section 9.2): `None`
    /// under all-open; `Some(governance_status(...))` once a manifest is active, computed from
    /// this facade's own held grants and manifest-level mode. Delegates to the free function
    /// [`governance_status`] so a standalone reader with no live `Governance` instance
    /// (`browser-mcp doctor`, which resolves its own manifest independently) computes the
    /// IDENTICAL summary from the same inputs -- the two surfaces can never disagree (g15
    /// constraint 12).
    pub fn governance_status(&self, config_mode: EffectiveMode) -> Option<GovernanceStatus> {
        match &self.mode {
            Mode::AllOpen => None,
            Mode::Governed(state) => Some(governance_status(
                &state.grants,
                state.manifest_mode,
                config_mode,
            )),
        }
    }

    /// The single inbound governance decision for one tool call, taken at the dispatch chokepoint
    /// before the tool executes.
    ///
    /// Under [`Mode::AllOpen`] this is a literal STEP-0 short-circuit: it returns
    /// [`Decision::Allow`] without touching any port or resolving any resource, so all-open output
    /// is byte-identical to stage 1. Under [`Mode::Governed`] the call's bound capability
    /// requirement set is looked up first (`action` is the `computer` sub-action, `None` for
    /// every other tool; ADR-0022 Decision 2): a directory miss (`None`) denies via the
    /// `unknown_action` rule (`enforcement::unknown_action_denial`), then passes through the SAME
    /// mode switch (g15, `enforcement::apply_mode`) a classified would-deny does, since a
    /// directory miss is an ordinary rule, not a sacred one -- it is just as eligible for shadow
    /// enforcement as any other would-deny. `Some(&[])` short-circuits to `Allow` immediately,
    /// without building a `DecisionRequest` (ADR-0022 Decision 5 step 2: no resource resolution,
    /// no grant scan). `Some(reqs)` builds the full [`DecisionRequest`] from the held grants,
    /// manifest hash, and manifest-level mode, plus the caller-resolved `resource` and
    /// `config_mode`, and delegates to the held decision point (which applies the same mode
    /// switch internally, `LocalPdp`/`check_call`, g15).
    pub fn decide(
        &self,
        tool: &str,
        action: Option<&str>,
        resource: GoverningResource,
        config_mode: EffectiveMode,
    ) -> Decision {
        match &self.mode {
            Mode::AllOpen => Decision::Allow { grant_id: None },
            Mode::Governed(state) => {
                let Some(reqs) = (self.requires)(tool, action) else {
                    let denial = crate::governance::enforcement::unknown_action_denial(
                        tool,
                        action,
                        &state.manifest_hash,
                    );
                    return crate::governance::enforcement::apply_mode(
                        Decision::Deny(denial),
                        &state.grants,
                        state.manifest_mode,
                        config_mode,
                    );
                };
                if reqs.is_empty() {
                    return Decision::Allow { grant_id: None };
                }
                let req = DecisionRequest {
                    grants: state.grants.clone(),
                    tool: tool.to_string(),
                    action: action.map(str::to_string),
                    requires: reqs.to_vec(),
                    resource,
                    manifest_mode: state.manifest_mode,
                    config_mode,
                    manifest_hash: state.manifest_hash.clone(),
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

    /// Build and record one audit record for a completed, ALLOWED tool call (ADR-0018 step 1:
    /// the flight recorder). Called at the dispatch chokepoint after the call resolves, so the
    /// record carries the real duration. `action` is the `computer` sub-action when
    /// `tool == "computer"`, `None` otherwise. `domain` is the current tab's host at decision
    /// time when the sacred-domains check (g08) or the grant machinery (g13) resolved one,
    /// `None` otherwise (shared format doc section 6.1: `domain` is a decision-time fact, not
    /// derived from tool arguments). `grant_id` is the resolving grant's id under a manifest
    /// (from `Decision::Allow { grant_id }`, g13), `None` under all-open or when no grant
    /// participates (`AlwaysAllow`, the `NoPage` union rule with no candidate... every allow
    /// path that reaches this far always has one, but the type stays optional to mirror
    /// [`Decision::Allow`] exactly).
    ///
    /// `identity` and `manifest` are always `None` until the identity/manifest-audit tasks land;
    /// `decision` is always `"allow"` (a denied call goes through [`Self::record_deny`]
    /// instead). `requires` is the call's bound capability requirement set (ADR-0022 Decision
    /// 2), looked up by the caller from the same action directory `decide` consulted; a
    /// directory miss maps to an empty requires slice at the call site and records `"none"`;
    /// the `decision` and denial-rule fields carry the deny story.
    pub fn record_call(
        &self,
        tool: &str,
        action: Option<&str>,
        requires: &[Capability],
        duration_ms: u64,
        domain: Option<&str>,
        grant_id: Option<&str>,
    ) {
        let record = self.build_record(
            tool,
            action,
            requires,
            domain,
            "allow",
            grant_id.map(str::to_string),
            None,
            duration_ms,
            false,
        );
        self.audit.record(&record);
    }

    /// Build and record one audit record for a call DENIED before dispatch (the sacred-domains
    /// rule, g08; later the grant-enforcement rules, g13). No tool call ever ran, so
    /// `duration_ms` is `0` per shared format doc section 6.1. `action` is the `computer`
    /// sub-action when `tool == "computer"`, `None` otherwise. `requires` is the call's bound
    /// capability requirement set (ADR-0022 Decision 2); a denial is still recorded with its
    /// true requirement set, since the record's `capability` field is about the call's nature,
    /// not its outcome. `domain` is the current tab's host at decision time when a current-tab
    /// check resolved one, `None` otherwise -- this is independent of which host the denial
    /// itself names (`denial.domain`): a navigate-target denial with an unresolvable current
    /// tab still records `domain: null` even though the denial message names the target
    /// (shared format doc section 6.1).
    pub fn record_deny(
        &self,
        tool: &str,
        action: Option<&str>,
        requires: &[Capability],
        denial: &Denial,
        domain: Option<&str>,
    ) {
        let record = self.build_record(
            tool,
            action,
            requires,
            domain,
            "deny",
            denial.grant_id.clone(),
            Some(denial.denial_id.clone()),
            0,
            false,
        );
        self.audit.record(&record);
    }

    /// Build and record one audit record for `navigate`'s point-5 post-landing denial (g13,
    /// shared format doc section 6.1): unlike [`Self::record_deny`], the call DID dispatch and
    /// the browser actually navigated before landing off-grant, so `duration_ms` is the real
    /// elapsed time, not `0`. Always `tool: "navigate"`. `domain` is the FINAL (post-redirect)
    /// host the tab landed on, or `None` for a non-host landing (a scheme, or an unresolvable
    /// re-query) -- never the denial message's `(unknown)` placeholder.
    pub fn record_navigate_landing_deny(
        &self,
        action: Option<&str>,
        requires: &[Capability],
        denial: &Denial,
        domain: Option<&str>,
        duration_ms: u64,
    ) {
        let record = self.build_record(
            "navigate",
            action,
            requires,
            domain,
            "deny",
            denial.grant_id.clone(),
            Some(denial.denial_id.clone()),
            duration_ms,
            false,
        );
        self.audit.record(&record);
    }

    /// Build and record one audit record for a call that WOULD have been denied under enforce
    /// but ran because the effective mode resolved to observe (g15, shadow enforcement,
    /// ADR-0020 commitment 4): the tool executed exactly as an allow would, so `decision` is
    /// `"shadow_deny"` with the SAME `grant_id`/`denial_id` an enforce-mode deny of the
    /// identical call would carry (they are derived from the same `Denial`, never recomputed),
    /// and `duration_ms` is the real elapsed time, never the pre-dispatch `0`. The agent's
    /// response carries no denial text; only this record tells the truth about what would have
    /// happened under enforce.
    pub fn record_shadow_deny(
        &self,
        tool: &str,
        action: Option<&str>,
        requires: &[Capability],
        denial: &Denial,
        domain: Option<&str>,
        duration_ms: u64,
    ) {
        let record = self.build_record(
            tool,
            action,
            requires,
            domain,
            "shadow_deny",
            denial.grant_id.clone(),
            Some(denial.denial_id.clone()),
            duration_ms,
            false,
        );
        self.audit.record(&record);
    }

    /// Build and record one audit record for a call answered with the take-the-wheel pause
    /// text instead of executing (a user hold, g10). The call was not policy-denied (policy
    /// was never consulted) and no tool ran, so `decision` is `"allow"` and `duration_ms` is
    /// `0`, exactly like [`Self::record_deny`]'s zero-duration convention -- but `held` is
    /// `true` and `grant_id`/`denial_id` stay `None`. `domain` is always `None`: a held call
    /// must not touch the extension, so no current-tab host is ever resolved for it.
    pub fn record_held(&self, tool: &str, action: Option<&str>, requires: &[Capability]) {
        let record = self.build_record(tool, action, requires, None, "allow", None, None, 0, true);
        self.audit.record(&record);
    }

    #[allow(clippy::too_many_arguments)]
    fn build_record(
        &self,
        tool: &str,
        action: Option<&str>,
        requires: &[Capability],
        domain: Option<&str>,
        decision: &'static str,
        grant_id: Option<String>,
        denial_id: Option<String>,
        duration_ms: u64,
        held: bool,
    ) -> AuditRecord {
        let capability = requires.first().map(Capability::as_str).unwrap_or("none");
        AuditRecord {
            event_id: uuid::Uuid::new_v4().to_string(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            identity: None,
            client: self.current_client(),
            tool: tool.to_string(),
            action: action.map(str::to_string),
            capability,
            domain: domain.map(str::to_string),
            decision,
            grant_id,
            denial_id,
            duration_ms,
            manifest: None,
            held,
        }
    }

    fn current_client(&self) -> Option<ClientInfo> {
        self.client
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Record the panic kill switch's session event (g11): the user severed the session. A
    /// session event, not a tool call -- carries no
    /// `tool`/`action`/`capability`/`domain`/`decision`/`grant_id`/`denial_id`/`duration_ms`,
    /// only the shared `event_id`/`ts`/`identity`/`client`/`manifest` fields plus
    /// `event: "session_killed"`. Called from the
    /// `Browser::on_session_killed` hook, registered once at session startup; the extension
    /// signals the event at most once per kill (the flag transition is idempotent), so this
    /// fires at most once per kill too.
    pub fn record_session_killed(&self) {
        let record = SessionEventRecord {
            event_id: uuid::Uuid::new_v4().to_string(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            identity: None,
            client: self.current_client(),
            event: "session_killed",
            manifest: None,
        };
        self.audit.record_session_event(&record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::ports::NoopPdp;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn no_requires(_tool: &str, _action: Option<&str>) -> Option<&'static [Capability]> {
        None
    }

    /// A stand-in for the browser plugin's real action directory: `computer`/`screenshot` and
    /// `read_page` require `read`; `computer`/`left_click` requires `action`; `tabs_create_mcp`
    /// requires nothing (ADR-0022 `requires: []`); everything else misses.
    fn sample_requires(tool: &str, action: Option<&str>) -> Option<&'static [Capability]> {
        match (tool, action) {
            ("computer", Some("screenshot")) => Some(&[Capability::Read]),
            ("computer", Some("left_click")) => Some(&[Capability::Action]),
            ("read_page", None) => Some(&[Capability::Read]),
            ("tabs_create_mcp", None) => Some(&[]),
            _ => None,
        }
    }

    /// A PDP that always denies, so a test built on it can prove a call NEVER reached it
    /// (ADR-0022 Decision 5 step 2: a `requires: []` action short-circuits to `Allow` before any
    /// decision-point consultation).
    struct AlwaysDenyPdp;
    impl PolicyDecisionPoint for AlwaysDenyPdp {
        fn decide(&self, _req: &DecisionRequest) -> Decision {
            Decision::Deny(Denial {
                rule: "would-have-fired".to_string(),
                grant_id: None,
                denial_id: "D-00000000".to_string(),
                domain: String::new(),
                message: "the PDP was consulted when it should not have been".to_string(),
            })
        }
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
        fn record_session_event(&self, _record: &SessionEventRecord) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A sink that keeps every record, so tests can assert on the actual built fields
    /// (`capability`, `action`, `client`) rather than just call count.
    #[derive(Default)]
    struct CapturingAuditSink {
        records: Mutex<Vec<AuditRecord>>,
        session_events: Mutex<Vec<SessionEventRecord>>,
    }
    impl AuditSink for CapturingAuditSink {
        fn record(&self, record: &AuditRecord) {
            self.records
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(record.clone());
        }
        fn record_session_event(&self, record: &SessionEventRecord) {
            self.session_events
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

        fn last_session_event(&self) -> SessionEventRecord {
            self.session_events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .last()
                .cloned()
                .expect("at least one session event was captured")
        }
    }

    #[test]
    fn all_open_decide_is_allow_and_still_records() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_requires);
        assert!(matches!(
            g.decide(
                "navigate",
                None,
                GoverningResource::None,
                EffectiveMode::Enforce
            ),
            Decision::Allow { grant_id: None }
        ));
        g.record_call("navigate", None, &[], 5, None, None);
        assert_eq!(sink.count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn governed_over_noop_still_allows_and_holds_the_sink() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::governed(
            Box::new(NoopPdp),
            sink.clone(),
            sample_requires,
            Vec::new(),
            String::new(),
            None,
        );
        assert!(matches!(
            g.decide(
                "read_page",
                None,
                GoverningResource::None,
                EffectiveMode::Enforce
            ),
            Decision::Allow { .. }
        ));
        g.record_call("navigate", None, &[], 0, None, None);
        assert_eq!(sink.count.load(Ordering::SeqCst), 1);
    }

    /// A directory miss (`requires` returns `None`) denies via the `unknown_action` rule, and
    /// that denial goes through the SAME mode switch a classified would-deny does (ADR-0022;
    /// `enforcement::unknown_action_denial`, `enforcement::apply_mode`).
    #[test]
    fn directory_miss_denies_via_unknown_action_through_the_mode_switch() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::governed(
            Box::new(NoopPdp),
            sink,
            no_requires,
            Vec::new(),
            "hash".to_string(),
            None,
        );
        match g.decide(
            "no_such_tool",
            None,
            GoverningResource::None,
            EffectiveMode::Enforce,
        ) {
            Decision::Deny(d) => assert_eq!(d.rule, "unknown_action"),
            other => panic!("expected an unknown_action deny, got {other:?}"),
        }
        match g.decide(
            "no_such_tool",
            None,
            GoverningResource::None,
            EffectiveMode::Observe,
        ) {
            Decision::ShadowDeny(d) => assert_eq!(d.rule, "unknown_action"),
            other => panic!("expected an unknown_action shadow deny, got {other:?}"),
        }
    }

    /// `requires: []` allows immediately, with no grant id, WITHOUT ever consulting the decision
    /// point (ADR-0022 Decision 5 step 2): proven here by wiring a PDP that always denies and
    /// showing the call still allows.
    #[test]
    fn requires_empty_allows_without_consulting_the_pdp() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::governed(
            Box::new(AlwaysDenyPdp),
            sink,
            sample_requires,
            Vec::new(),
            "hash".to_string(),
            None,
        );
        assert_eq!(
            g.decide(
                "tabs_create_mcp",
                None,
                GoverningResource::None,
                EffectiveMode::Enforce
            ),
            Decision::Allow { grant_id: None }
        );
    }

    #[test]
    fn computer_action_requires_flows_into_capability() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), sample_requires);

        g.record_call(
            "computer",
            Some("screenshot"),
            &[Capability::Read],
            0,
            None,
            None,
        );
        let rec = sink.last();
        assert_eq!(rec.capability, "read");
        assert_eq!(rec.action.as_deref(), Some("screenshot"));

        g.record_call(
            "computer",
            Some("left_click"),
            &[Capability::Action],
            0,
            None,
            None,
        );
        assert_eq!(sink.last().capability, "action");

        g.record_call("read_page", None, &[Capability::Read], 0, None, None);
        let rec = sink.last();
        assert_eq!(rec.capability, "read");
        assert_eq!(rec.action, None);
    }

    /// `requires_empty_records_capability_none`: a directory-less/empty-requires call records
    /// `capability: "none"` and `decision: "allow"` (ADR-0022 Decision 8).
    #[test]
    fn requires_empty_records_capability_none() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_requires);
        g.record_call("tabs_create_mcp", None, &[], 0, None, None);
        let rec = sink.last();
        assert_eq!(rec.capability, "none");
        assert_eq!(rec.decision, "allow");
    }

    /// `deny_record_carries_the_capability_of_the_denied_call`: a deny record's `capability`
    /// reflects the call's own requirement set, not the outcome.
    #[test]
    fn deny_record_carries_the_capability_of_the_denied_call() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_requires);
        let denial = Denial {
            rule: "capability".to_string(),
            grant_id: None,
            denial_id: "D-00000001".to_string(),
            domain: String::new(),
            message: "Denied (D-00000001): javascript_tool needs the 'execute' capability."
                .to_string(),
        };
        g.record_deny(
            "javascript_tool",
            None,
            &[Capability::Execute],
            &denial,
            None,
        );
        let rec = sink.last();
        assert_eq!(rec.capability, "execute");
        assert_eq!(rec.decision, "deny");
    }

    #[test]
    fn set_client_first_capture_wins() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_requires);
        g.set_client("a", "1");
        g.set_client("b", "2");
        let stored = g.client.lock().unwrap();
        assert_eq!(stored.as_ref().unwrap().name, "a");
        assert_eq!(stored.as_ref().unwrap().version, "1");
        drop(stored);

        g.record_call("navigate", None, &[], 0, None, None);
        let client = sink.last().client.expect("client info recorded");
        assert_eq!(client.name, "a");
        assert_eq!(client.version, "1");
    }

    #[test]
    fn record_call_passes_the_resolved_domain_through() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_requires);
        g.record_call("read_page", None, &[], 0, Some("www.mybank.com"), None);
        assert_eq!(sink.last().domain.as_deref(), Some("www.mybank.com"));
    }

    #[test]
    fn record_deny_writes_a_zero_duration_deny_record() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), sample_requires);
        let denial = Denial {
            rule: "sacred/*.mybank.com".to_string(),
            grant_id: None,
            denial_id: "D-af6633ec".to_string(),
            domain: "www.mybank.com".to_string(),
            message: "Denied (D-af6633ec): www.mybank.com is on the user's never-touch list."
                .to_string(),
        };
        g.record_deny(
            "read_page",
            None,
            &[Capability::Read],
            &denial,
            Some("www.mybank.com"),
        );
        let rec = sink.last();
        assert_eq!(rec.decision, "deny");
        assert_eq!(rec.denial_id.as_deref(), Some("D-af6633ec"));
        assert_eq!(rec.grant_id, None);
        assert_eq!(rec.duration_ms, 0);
        assert_eq!(rec.domain.as_deref(), Some("www.mybank.com"));
        assert_eq!(rec.capability, "read");
    }

    #[test]
    fn record_held_writes_an_allow_record_with_held_true_and_no_domain() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), sample_requires);
        g.record_held("computer", Some("screenshot"), &[Capability::Read]);
        let rec = sink.last();
        assert_eq!(rec.decision, "allow");
        assert!(rec.held);
        assert_eq!(rec.duration_ms, 0);
        assert_eq!(rec.domain, None);
        assert_eq!(rec.grant_id, None);
        assert_eq!(rec.denial_id, None);
        assert_eq!(rec.capability, "read");
        assert_eq!(rec.action.as_deref(), Some("screenshot"));
    }

    #[test]
    fn record_call_and_record_deny_leave_held_false() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_requires);
        g.record_call("navigate", None, &[], 5, None, None);
        assert!(!sink.last().held);

        let denial = Denial {
            rule: "sacred/mybank.com".to_string(),
            grant_id: None,
            denial_id: "D-171052e3".to_string(),
            domain: "mybank.com".to_string(),
            message: "Denied (D-171052e3): mybank.com is on the user's never-touch list."
                .to_string(),
        };
        g.record_deny("navigate", None, &[], &denial, None);
        assert!(!sink.last().held);
    }

    #[test]
    fn hold_message_states_not_executed_with_no_hint_below_the_threshold() {
        let msg = hold_message("navigate", None, Duration::from_secs(1));
        assert!(msg.starts_with("Paused:"));
        assert!(msg.contains("NOT executed"));
        assert!(msg.contains("'navigate' call"));
        assert!(!msg.contains("2 minutes"));
    }

    #[test]
    fn hold_message_appends_the_hint_at_and_above_the_threshold() {
        let at_threshold = hold_message("navigate", None, HOLD_HINT_AFTER);
        assert!(at_threshold.contains("2 minutes"));
        assert!(at_threshold.contains("Only the user can resume it"));

        let above_threshold =
            hold_message("navigate", None, HOLD_HINT_AFTER + Duration::from_secs(1));
        assert!(above_threshold.contains("2 minutes"));

        let below_threshold =
            hold_message("navigate", None, HOLD_HINT_AFTER - Duration::from_secs(1));
        assert!(!below_threshold.contains("2 minutes"));
    }

    #[test]
    fn hold_message_renders_computer_action_label() {
        let msg = hold_message("computer", Some("left_click"), Duration::from_secs(0));
        assert!(msg.contains("'computer (left_click)' call"));

        let plain = hold_message("read_page", None, Duration::from_secs(0));
        assert!(plain.contains("'read_page' call"));
    }

    #[test]
    fn record_session_killed_writes_a_session_event_with_no_tool_call_fields() {
        let sink = Arc::new(CapturingAuditSink::default());
        let g = Governance::all_open(sink.clone(), no_requires);
        g.set_client("claude-code", "2.1.0");
        g.record_session_killed();
        let rec = sink.last_session_event();
        assert_eq!(rec.event, "session_killed");
        assert_eq!(rec.client.as_ref().unwrap().name, "claude-code");
        assert_eq!(rec.identity, None);
        assert_eq!(rec.manifest, None);
    }

    // --- g15: the governance_status badge resolver ---

    fn one_grant() -> Grant {
        Grant {
            id: "g1".to_string(),
            hosts: crate::governance::manifest::document::HostRules {
                allow: vec!["example.com".to_string()],
                deny: Vec::new(),
            },
            allowed: vec![Capability::Read, Capability::Action, Capability::Write],
            description: None,
            mode: None,
        }
    }

    #[test]
    fn governance_status_is_none_under_all_open() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::all_open(sink, no_requires);
        assert_eq!(g.governance_status(EffectiveMode::Enforce), None);
    }

    #[test]
    fn governance_status_reports_shadow_true_with_grants_under_observe() {
        assert_eq!(
            governance_status(
                &[one_grant()],
                Some(EffectiveMode::Observe),
                EffectiveMode::Enforce
            ),
            GovernanceStatus {
                mode: EffectiveMode::Observe,
                shadow: true,
            }
        );
        // The manifest's own mode wins; config alone would have said enforce here.
        assert_eq!(
            governance_status(&[one_grant()], None, EffectiveMode::Observe),
            GovernanceStatus {
                mode: EffectiveMode::Observe,
                shadow: true,
            }
        );
    }

    #[test]
    fn governance_status_reports_shadow_false_under_enforce() {
        assert_eq!(
            governance_status(
                &[one_grant()],
                Some(EffectiveMode::Enforce),
                EffectiveMode::Observe
            ),
            GovernanceStatus {
                mode: EffectiveMode::Enforce,
                shadow: false,
            }
        );
    }

    #[test]
    fn governance_status_never_shadows_with_empty_grants_even_under_observe() {
        assert_eq!(
            governance_status(&[], Some(EffectiveMode::Observe), EffectiveMode::Enforce),
            GovernanceStatus {
                mode: EffectiveMode::Observe,
                shadow: false,
            }
        );
    }

    #[test]
    fn governance_status_via_the_live_facade_matches_the_free_function() {
        let sink = Arc::new(CountingAuditSink::default());
        let g = Governance::governed(
            Box::new(NoopPdp),
            sink,
            no_requires,
            vec![one_grant()],
            String::new(),
            Some(EffectiveMode::Observe),
        );
        assert_eq!(
            g.governance_status(EffectiveMode::Enforce),
            Some(GovernanceStatus {
                mode: EffectiveMode::Observe,
                shadow: true,
            })
        );
    }
}
