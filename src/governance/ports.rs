//! The governance seam -- the S4 policy-decision-point / policy-enforcement-point contract.
//!
//! The decision is a PURE, serializable function so it can run in-process today and
//! out-of-process later (the persistent-service direction, ADR-0021). The pure half
//! ([`DomainPolicy`]) travels WITH the decision; the impure half ([`ResourceResolver`])
//! stays at the enforcement point, since it needs live state. Single-impl ports
//! ([`DomainPolicy`], [`ResourceResolver`]) are consumed via generics/concrete types (zero
//! vtable); `dyn` is used only for [`PolicyDecisionPoint`] and [`AuditSink`], each of which
//! has more than one impl today ([`NoopPdp`]/a future Local PDP/a future out-of-process
//! Remote PDP) or a known future one (file/stderr/syslog sinks).

use serde::{Deserialize, Serialize};

use crate::governance::manifest::document::Grant;

// --- Supporting placeholder and axis types ---

/// Read/write classification of a tool call: the observe-vs-mutate axis (the core owns the
/// axis; g05 owns the tool+action -> class table in the browser plugin). `Observe` is an
/// observation; `Mutate` is a mutation. g05 maps each tool/action onto this and MAY extend
/// the type minimally when it lands. Distinct from a grant's `access` field (`read` | `write`
/// | `all`), which is a separate concept applied during enforcement (g13); see
/// RECONCILIATION.md section 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RwClass {
    Observe,
    Mutate,
}

impl RwClass {
    /// The audit `rw` field vocabulary (shared format doc section 6.1): exactly `"observe"`
    /// or `"mutate"`. Matches the `#[serde(rename_all = "snake_case")]` wire form but is
    /// provided directly so callers (the audit recorder, g06) do not need to round-trip
    /// through `serde_json` just to get the bare string.
    pub fn as_str(&self) -> &'static str {
        match self {
            RwClass::Observe => "observe",
            RwClass::Mutate => "mutate",
        }
    }
}

/// The effective enforcement mode for a call (g15 resolves it: per-grant > manifest >
/// `governance.mode`). `Observe` records a shadow denial but allows; `Enforce` blocks.
/// Wire names are `observe` / `enforce`, matching the `governance.mode` config enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveMode {
    Observe,
    Enforce,
}

impl EffectiveMode {
    /// The wire/status-surface vocabulary: exactly `"observe"` or `"enforce"`. Matches the
    /// `#[serde(rename_all = "snake_case")]` form, provided directly so callers (the doctor
    /// `Governance:` section, g15) do not need to round-trip through `serde_json`.
    pub fn as_str(&self) -> &'static str {
        match self {
            EffectiveMode::Observe => "observe",
            EffectiveMode::Enforce => "enforce",
        }
    }

    /// Parse the resolved `governance.mode` config value. The registry's own
    /// `EnumVariants(&["observe", "enforce"])` constraint (g01) guarantees a resolved config
    /// never carries any other string, so any other input is unreachable in practice; it
    /// fails closed to `Enforce` rather than panicking.
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "observe" => EffectiveMode::Observe,
            _ => EffectiveMode::Enforce,
        }
    }
}

/// One capability primitive of the ADR-0022 Decision 1 taxonomy. Capabilities classify
/// an operation by EPISTEMIC STATUS -- what the governor can PROVE about it -- never by
/// its (unknowable) downstream effect. `Read` is provably retrieval/observation only;
/// `Action` dispatches UI input whose effect is page-determined and unknowable; `Write`
/// is a declared mutation; `Execute` is unbounded arbitrary code. `Action` is NOT a
/// weaker `Write`: it encompasses the ability to CAUSE writes (a click can submit a
/// form). `Execute` is never implied by any other capability. Capabilities are
/// independent primitives, not ordered tiers. Wire/file names are lowercase: `"read"`,
/// `"action"`, `"write"`, `"execute"`. Nothing consumes this type yet: s05 wires it
/// into grants and enforcement; until then `RwClass` remains the classification in
/// force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Read,
    Action,
    Write,
    Execute,
}

impl Capability {
    /// The wire/file vocabulary (ADR-0022 Decision 1): exactly `"read"`, `"action"`,
    /// `"write"`, or `"execute"`. Matches the `#[serde(rename_all = "lowercase")]`
    /// form, provided directly so callers do not round-trip through `serde_json` for
    /// the bare string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::Read => "read",
            Capability::Action => "action",
            Capability::Write => "write",
            Capability::Execute => "execute",
        }
    }

    /// Parse one capability name. Exact lowercase only: any other casing, whitespace,
    /// or unknown word returns `None` (fail closed; the wire vocabulary is lowercase).
    pub fn from_name(name: &str) -> Option<Capability> {
        match name {
            "read" => Some(Capability::Read),
            "action" => Some(Capability::Action),
            "write" => Some(Capability::Write),
            "execute" => Some(Capability::Execute),
            _ => None,
        }
    }
}

/// True iff every element of `requires` appears in `allowed` -- the subset containment
/// that enforcement evaluates (ADR-0022 Decision 3). An empty `requires` is a subset of
/// everything, including an empty `allowed`. Duplicates in either slice do not change
/// the result. No capability implies another: `Execute` in `requires` is satisfied only
/// by `Execute` in `allowed`.
pub fn capability_subset(requires: &[Capability], allowed: &[Capability]) -> bool {
    requires.iter().all(|r| allowed.contains(r))
}

/// A tool identifier as advertised on the MCP surface. Placeholder newtype; g07/g14 flesh
/// out the tool-surface handling. The sacred tool schemas (ADR-0007) are the source of
/// truth for the actual names; this type never mutates them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolId(pub String);

/// A resource-matching pattern (a domain pattern for the browser plugin). Placeholder
/// newtype; g07 (the CVE-hardened matcher) and g12 (grant domains) flesh out the semantics.
/// Only syntax/shape is a wrapper here; no matching logic lives in the core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePattern(pub String);

/// A structured policy denial (shared format doc section 7). Carried by `Decision::Deny` and
/// `Decision::ShadowDeny`; its `denial_id` (via [`crate::governance::denial::denial_id`]) goes
/// into the audit record and its `message` is returned to the caller as a normal text tool
/// result -- a denial is a policy outcome to read and adapt to, never a transport or tool
/// failure. Grown by g08 from A2's two-field placeholder to the full shape; g13 (grant
/// enforcement) reuses it unchanged for the `unmatched_domain` / `access` / `tool` / `scheme`
/// denial rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Denial {
    /// Rule string per shared format doc section 7.1, e.g. `"sacred/*.mybank.com"`.
    pub rule: String,
    /// The resolving grant's id. Always `None` until the manifest engine (g12/g13) lands.
    pub grant_id: Option<String>,
    /// Stable denial id: `"D-"` plus 8 lowercase hex characters (shared format doc 7.1).
    pub denial_id: String,
    /// Parser-normalized host named in the message.
    pub domain: String,
    /// Full caller-facing message (shared format doc section 7.2 template). Names only the
    /// matched host and the denial id; never the rule, the pattern, or any other list entry.
    pub message: String,
}

/// The `identity` object of an audit record: `{ "principal": ..., "resolved_by": ... }`,
/// from the active manifest's `identity` block (shared format doc section 6.1). Always
/// `None` on [`AuditRecord`] until the manifest task (g12) lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identity {
    pub principal: String,
    pub resolved_by: String,
}

/// The `client` object of an audit record: `{ "name": ..., "version": ... }` from the MCP
/// `initialize` request's `clientInfo` (shared format doc section 6.1). Captured once per
/// session, first-wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// One audit record: exactly one JSON line per tool call (shared format doc section 6.1).
/// Field ORDER is part of the format; `serde_json` is built with `preserve_order`. Grown by
/// g06 from A2's single-field placeholder to the full shape, then by g10 (the `held`
/// field); reused unchanged by `policy simulate`, the activity ledger, and session recap
/// (later tasks).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditRecord {
    /// UUID v4, lowercase, hyphenated. Unique per record.
    pub event_id: String,
    /// RFC 3339 UTC timestamp, millisecond precision, e.g. `2026-07-02T14:32:15.003Z`.
    pub ts: String,
    /// From the active manifest's `identity` block; always `None` until the manifest task
    /// (g12) lands.
    pub identity: Option<Identity>,
    /// MCP client identity from the `initialize` request's `clientInfo`; `None` if the client
    /// did not provide it. Captured once per session.
    pub client: Option<ClientInfo>,
    /// MCP tool name as received.
    pub tool: String,
    /// The `computer` sub-action (e.g. `left_click`); `None` for every other tool.
    pub action: Option<String>,
    /// `Observe` or `Mutate` (shared format doc section 8; serializes as `"observe"` /
    /// `"mutate"` via `RwClass`'s own `snake_case` rename, so the record never hand-rolls a
    /// second copy of that vocabulary).
    pub rw: RwClass,
    /// Parser-normalized host of the current tab at decision time; always `None` until the
    /// enforcement task introduces current-tab tracking.
    pub domain: Option<String>,
    /// `"allow"`, `"deny"`, or `"shadow_deny"`. Always `"allow"` until enforcement (g13) and
    /// shadow mode (g15) land.
    pub decision: &'static str,
    /// Grant id that resolved the decision; always `None` until grants exist.
    pub grant_id: Option<String>,
    /// Stable denial id; always `None` until denials exist.
    pub denial_id: Option<String>,
    /// Wall time from dispatch entry to result, in milliseconds.
    pub duration_ms: u64,
    /// Active manifest identity; always `None` until the manifest task (g12) wires it in.
    /// Reuses [`crate::governance::manifest::identity::ManifestIdentity`] (g09) rather than a
    /// second `{name, version, hash}` shape.
    pub manifest: Option<crate::governance::manifest::identity::ManifestIdentity>,
    /// `true` when the call was answered with the take-the-wheel pause text instead of
    /// executing (a user hold, g10); on a held record `decision` is `"allow"` and
    /// `duration_ms` is `0`. `false` on every other record; always present, never omitted.
    pub held: bool,
}

/// A session EVENT record (shared format doc section 6, g11): additive to the tool-call
/// [`AuditRecord`] stream and deliberately distinguishable from it -- an `event` field, and
/// NONE of `tool`/`action`/`rw`/`domain`/`decision`/`grant_id`/`denial_id`/`duration_ms`. The
/// panic kill switch is the first (and, today, only) producer, with `event: "session_killed"`.
/// Field ORDER is part of the format; `serde_json` is built with `preserve_order`. Downstream
/// consumers that expect tool-call records (`policy simulate`, the activity ledger) must skip
/// any line carrying an `event` field.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionEventRecord {
    /// UUID v4, lowercase, hyphenated. Unique per record.
    pub event_id: String,
    /// RFC 3339 UTC timestamp, millisecond precision, e.g. `2026-07-02T14:32:15.003Z`.
    pub ts: String,
    /// From the active manifest's `identity` block; always `None` until the manifest task
    /// (g12) lands.
    pub identity: Option<Identity>,
    /// MCP client identity from the `initialize` request's `clientInfo`; `None` if the client
    /// did not provide it. Captured once per session.
    pub client: Option<ClientInfo>,
    /// The event discriminator. Always the literal `"session_killed"` today (g11); later
    /// session events, if any, would add their own string here, never a new record shape.
    pub event: &'static str,
    /// Active manifest identity; always `None` until the manifest task (g12) wires it in.
    pub manifest: Option<crate::governance::manifest::identity::ManifestIdentity>,
}

// --- The core decision types (serde is load-bearing) ---

/// A generic governing resource, so the decision core stays domain-agnostic. The browser
/// plugin fills `Resource(host)`; a filesystem module would fill `Resource(path)`.
/// `AlwaysAllow` is the resource-exempt case (browser: `about:blank`); `None` is a
/// resource-less call; `Indeterminate` means resolution failed and the decision must fail
/// closed under a manifest. g07/g12 refine how these are produced; the enum shape is stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoverningResource {
    /// A concrete governed resource (browser: a host such as `github.com`).
    Resource(String),
    /// The call targets an always-allowed resource (browser: `about:blank`).
    AlwaysAllow,
    /// The resource is outside the governed scope; carries a describing string.
    OutOfScope(String),
    /// The call has no governing resource (a resource-less tool).
    None,
    /// The resource could not be resolved; fail closed under a manifest.
    Indeterminate,
}

/// The complete, self-contained input to a policy decision. PURE and serde-serializable so
/// the decision can run in-process today and out-of-process later without a rewrite, and so
/// g17 (simulate) can replay a recorded request through the same decision function. Nothing
/// here references live state: resource resolution already happened (see `ResourceResolver`)
/// and its result is baked into `resource`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRequest {
    /// The grants in force for this subject (empty under all-open).
    pub grants: Vec<Grant>,
    /// The tool being called.
    pub tool: String,
    /// The `computer` sub-action, when `tool == "computer"`; `None` otherwise. Carried only
    /// for denial-message rendering (`computer (<action>)`, shared format doc section 7.2);
    /// grant `tools`/`exclude_tools` checks and rule strings use the bare `tool` name, never
    /// this field.
    pub action: Option<String>,
    /// The tool call's read/write classification.
    pub rw: RwClass,
    /// The resolved governing resource.
    pub resource: GoverningResource,
    /// The active manifest's own `mode` field (shared format 4.1), if it set one. `None` when
    /// no manifest is active or the manifest did not declare a `mode`. Part of the mode
    /// precedence (g15, shared format 3.4): a resolving grant's own `mode` (already carried
    /// on each `Grant` in `grants`) wins over this, which in turn wins over `config_mode`.
    pub manifest_mode: Option<EffectiveMode>,
    /// The resolved `governance.mode` config value (g01/g02), the last-resort fallback in
    /// the mode precedence above. Never optional: the layered resolver always defines
    /// `governance.mode` (the built-in Minimal preset is the floor), so resolution never
    /// fails to produce a mode.
    pub config_mode: EffectiveMode,
    /// The active manifest's content hash (g09), empty string when no manifest is active.
    /// Part of the request (not read from live state) so a denial id is fully reproducible
    /// from the request alone -- g17 (simulate) replays a recorded request through the same
    /// decision function and must get the same `D-...` id back.
    pub manifest_hash: String,
}

/// The outcome of a policy decision. `Allow` optionally names the grant that permitted the
/// call (for attribution/audit). `Deny` blocks; `ShadowDeny` would have blocked but the
/// mode is observe, so the call is allowed and the denial is recorded (g15). Serde-derived
/// so an out-of-process PDP can return it over the wire and g17 can compare replays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// The call is permitted; `grant_id` is the matching grant, if any.
    Allow { grant_id: Option<String> },
    /// The call is blocked.
    Deny(Denial),
    /// The call would be blocked under enforce; observe mode allows it and records the denial.
    ShadowDeny(Denial),
}

// --- The traits ---

/// The policy decision point: a PURE, relocatable function from a serializable request to a
/// decision. `dyn` because it has multiple impls (the `NoopPdp` here, a Local PDP in g13,
/// and a future out-of-process Remote PDP). Send + Sync so it can be shared across the
/// tokio runtime.
pub trait PolicyDecisionPoint: Send + Sync {
    /// Decide the outcome for a fully-resolved request. Must be pure: no I/O, no live state.
    fn decide(&self, req: &DecisionRequest) -> Decision;
}

/// The domain plugin's PURE half: classification, resource matching, sacred detection, and
/// the advertised tool surface. It travels WITH the decision (it can relocate out-of-process
/// with the PDP). Single-impl (the browser plugin); consumed via a concrete type or a
/// generic bound, never `dyn`. g05 provides `classify`, g07 provides `matches`, g08 provides
/// `is_sacred`, g07/g14 provide `tool_surface`; the trait MAY be minimally adjusted when they
/// land (for example splitting `classify`/`matches` into sub-traits if that reads cleaner).
pub trait DomainPolicy {
    /// Classify a tool (and optional sub-action) as read or write. `None` if unknown.
    fn classify(&self, tool: &str, action: Option<&str>) -> Option<RwClass>;
    /// True if `pattern` matches `resource` under the plugin's matching semantics.
    fn matches(&self, pattern: &ResourcePattern, resource: &GoverningResource) -> bool;
    /// True if `resource` is a sacred never-touch resource (always enforced).
    fn is_sacred(&self, resource: &GoverningResource) -> bool;
    /// The tools this plugin advertises on the MCP surface.
    fn tool_surface(&self) -> &[ToolId];
}

/// The domain plugin's IMPURE half: resolve the governing resource from live state (browser:
/// the active tab's URL). It stays at the enforcement point forever and NEVER relocates
/// out-of-process (it needs live state). Single-impl; consumed via a concrete type or a
/// generic bound, never `dyn`. Async because resolving the resource is I/O (a CDP round-trip
/// for the browser plugin). g07/g13 provide the browser impl.
///
/// This uses a native `async fn` in a trait (stable since Rust 1.75) rather than the
/// `async-trait` crate: the port is single-impl and consumed concretely, so it does not need
/// to be `dyn`-compatible, and avoiding `async-trait` keeps the dependency set lean (no
/// per-call boxing). The `async_fn_in_trait` lint is allowed for exactly this reason.
#[allow(async_fn_in_trait)]
pub trait ResourceResolver {
    /// Resolve the governing resource for a tool call from its arguments and live state.
    async fn governing_resource(&self, tool: &str, args: &serde_json::Value) -> GoverningResource;
}

/// A sink for audit records. `dyn` because it has multiple impls (the `NullSink` here, plus
/// file/stderr/syslog in g06). Send + Sync so it can be shared across the runtime. Recording
/// is fire-and-forget: it returns nothing and must not fail the call.
pub trait AuditSink: Send + Sync {
    /// Record one audit line. Must not panic and must not block the call path meaningfully.
    fn record(&self, record: &AuditRecord);
    /// Record one session-event line (g11: the panic kill switch is the first producer). Same
    /// destination and framing as [`Self::record`]; a distinct method because the two record
    /// shapes are deliberately different types, not a variant of one enum.
    fn record_session_event(&self, record: &SessionEventRecord);
}

// --- Zero-policy implementations ---

/// The no-op policy decision point: allows every call. This is the STEP-0 all-open PDP; the
/// facade (A3) uses it when there is no manifest, preserving byte-identical stage-1 behavior.
/// g13 provides the real (Local) PDP that runs the grant-check decision.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPdp;

impl PolicyDecisionPoint for NoopPdp {
    fn decide(&self, _req: &DecisionRequest) -> Decision {
        Decision::Allow { grant_id: None }
    }
}

/// An audit sink that drops every record. Used under all-open (audit disabled) so the audit
/// seam is always wired without emitting anything. g06 provides the file/stderr/syslog sinks.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSink;

impl AuditSink for NullSink {
    fn record(&self, _record: &AuditRecord) {}
    fn record_session_event(&self, _record: &SessionEventRecord) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_grant(id: &str) -> Grant {
        Grant {
            id: id.to_string(),
            domains: vec!["example.com".to_string()],
            access: crate::governance::manifest::document::Access::All,
            tools: None,
            exclude_tools: None,
            description: None,
            mode: None,
        }
    }

    fn sample_request(
        rw: RwClass,
        resource: GoverningResource,
        config_mode: EffectiveMode,
    ) -> DecisionRequest {
        DecisionRequest {
            grants: Vec::new(),
            tool: "navigate".to_string(),
            action: None,
            rw,
            resource,
            manifest_mode: None,
            config_mode,
            manifest_hash: String::new(),
        }
    }

    #[test]
    fn noop_pdp_allows_every_request() {
        let pdp = NoopPdp;
        let requests = [
            sample_request(
                RwClass::Observe,
                GoverningResource::None,
                EffectiveMode::Observe,
            ),
            sample_request(
                RwClass::Mutate,
                GoverningResource::Resource("example.com".to_string()),
                EffectiveMode::Enforce,
            ),
            DecisionRequest {
                grants: vec![sample_grant("g1")],
                tool: "computer".to_string(),
                action: Some("left_click".to_string()),
                rw: RwClass::Mutate,
                resource: GoverningResource::AlwaysAllow,
                manifest_mode: None,
                config_mode: EffectiveMode::Enforce,
                manifest_hash: String::new(),
            },
        ];
        for req in &requests {
            assert_eq!(pdp.decide(req), Decision::Allow { grant_id: None });
        }
    }

    /// A minimal, otherwise-null `AuditRecord` for tests that only need a concrete value to
    /// pass to `AuditSink::record`, not to inspect the record's own fields.
    fn sample_audit_record(tool: &str) -> AuditRecord {
        AuditRecord {
            event_id: "00000000-0000-4000-8000-000000000000".to_string(),
            ts: "2026-07-02T00:00:00.000Z".to_string(),
            identity: None,
            client: None,
            tool: tool.to_string(),
            action: None,
            rw: RwClass::Mutate,
            domain: None,
            decision: "allow",
            grant_id: None,
            denial_id: None,
            duration_ms: 0,
            manifest: None,
            held: false,
        }
    }

    #[test]
    fn null_sink_record_is_a_noop() {
        let sink = NullSink;
        sink.record(&sample_audit_record("navigate"));
    }

    /// A minimal, otherwise-null `SessionEventRecord` for tests that only need a concrete value.
    fn sample_session_event_record() -> SessionEventRecord {
        SessionEventRecord {
            event_id: "00000000-0000-4000-8000-000000000000".to_string(),
            ts: "2026-07-02T00:00:00.000Z".to_string(),
            identity: None,
            client: None,
            event: "session_killed",
            manifest: None,
        }
    }

    #[test]
    fn null_sink_record_session_event_is_a_noop() {
        let sink = NullSink;
        sink.record_session_event(&sample_session_event_record());
    }

    #[test]
    fn session_event_record_serializes_all_fields_in_order_with_no_tool_call_fields() {
        let record = sample_session_event_record();
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
        let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            vec!["event_id", "ts", "identity", "client", "event", "manifest"]
        );
        assert_eq!(v["event"], "session_killed");
        for field in [
            "tool",
            "action",
            "rw",
            "domain",
            "decision",
            "grant_id",
            "denial_id",
            "duration_ms",
        ] {
            assert!(
                v.get(field).is_none(),
                "{field} must not appear on a session event record"
            );
        }
    }

    #[test]
    fn record_serializes_all_fields_in_shared_format_order() {
        let record = sample_audit_record("navigate");
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
        let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            vec![
                "event_id",
                "ts",
                "identity",
                "client",
                "tool",
                "action",
                "rw",
                "domain",
                "decision",
                "grant_id",
                "denial_id",
                "duration_ms",
                "manifest",
                "held",
            ]
        );
    }

    #[test]
    fn held_defaults_false_and_serializes_as_a_boolean() {
        let record = sample_audit_record("navigate");
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
        assert_eq!(v["held"], false);
    }

    #[test]
    fn absent_values_serialize_as_null_not_omitted() {
        let record = sample_audit_record("navigate");
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
        for field in [
            "identity",
            "client",
            "action",
            "domain",
            "grant_id",
            "denial_id",
            "manifest",
        ] {
            assert!(v.get(field).is_some(), "{field} must be present");
            assert!(
                v[field].is_null(),
                "{field} must be null, got {:?}",
                v[field]
            );
        }
    }

    #[test]
    fn serialized_record_is_a_single_line() {
        let mut record = sample_audit_record("navigate");
        record.tool = "navigate\nwith embedded newline".to_string();
        let line = serde_json::to_string(&record).unwrap();
        assert!(!line.contains('\n'), "must contain no raw LF: {line}");
    }

    #[test]
    fn pdp_is_object_safe() {
        let pdp: Box<dyn PolicyDecisionPoint> = Box::new(NoopPdp);
        let req = sample_request(
            RwClass::Observe,
            GoverningResource::None,
            EffectiveMode::Observe,
        );
        assert_eq!(pdp.decide(&req), Decision::Allow { grant_id: None });
    }

    #[test]
    fn audit_sink_is_object_safe() {
        let sink: Box<dyn AuditSink> = Box::new(NullSink);
        sink.record(&sample_audit_record("read_page"));
        sink.record_session_event(&sample_session_event_record());
    }

    #[test]
    fn decision_request_round_trips_through_serde() {
        let req = DecisionRequest {
            grants: vec![sample_grant("servicenow-full")],
            tool: "navigate".to_string(),
            action: None,
            rw: RwClass::Mutate,
            resource: GoverningResource::Resource("example.com".to_string()),
            manifest_mode: Some(EffectiveMode::Observe),
            config_mode: EffectiveMode::Enforce,
            manifest_hash: "abc123".to_string(),
        };
        let json = serde_json::to_string(&req).expect("serializes");
        let round_tripped: DecisionRequest = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(req, round_tripped);
    }

    #[test]
    fn decision_round_trips_through_serde() {
        let denial = Denial {
            rule: "sacred/mybank.com".to_string(),
            grant_id: None,
            denial_id: "D-9f3a1c2e".to_string(),
            domain: "mybank.com".to_string(),
            message: "Denied (D-9f3a1c2e): mybank.com is on the user's never-touch list."
                .to_string(),
        };
        let variants = [
            Decision::Allow {
                grant_id: Some("servicenow-full".to_string()),
            },
            Decision::Allow { grant_id: None },
            Decision::Deny(denial.clone()),
            Decision::ShadowDeny(denial),
        ];
        for decision in variants {
            let json = serde_json::to_string(&decision).expect("serializes");
            let round_tripped: Decision = serde_json::from_str(&json).expect("deserializes");
            assert_eq!(decision, round_tripped);
        }
    }

    #[test]
    fn rw_and_mode_wire_names_are_lowercase() {
        assert_eq!(
            serde_json::to_string(&RwClass::Observe).unwrap(),
            "\"observe\""
        );
        assert_eq!(
            serde_json::to_string(&RwClass::Mutate).unwrap(),
            "\"mutate\""
        );
        assert_eq!(
            serde_json::to_string(&EffectiveMode::Observe).unwrap(),
            "\"observe\""
        );
        assert_eq!(
            serde_json::to_string(&EffectiveMode::Enforce).unwrap(),
            "\"enforce\""
        );
    }

    #[test]
    fn capability_wire_names_round_trip() {
        let pairs = [
            (Capability::Read, "read"),
            (Capability::Action, "action"),
            (Capability::Write, "write"),
            (Capability::Execute, "execute"),
        ];
        for (cap, name) in pairs {
            let expected_json = format!("\"{name}\"");
            assert_eq!(serde_json::to_string(&cap).unwrap(), expected_json);
            assert_eq!(
                serde_json::from_str::<Capability>(&expected_json).unwrap(),
                cap
            );
            assert_eq!(cap.as_str(), name);
            assert_eq!(Capability::from_name(name), Some(cap));
        }
    }

    #[test]
    fn capability_from_name_rejects_unknown_and_case_variants() {
        for name in ["Read", "READ", "", "all", "observe"] {
            assert_eq!(Capability::from_name(name), None);
        }
    }

    #[test]
    fn capability_subset_truth_table() {
        assert!(capability_subset(&[], &[]));
        assert!(capability_subset(
            &[],
            &[
                Capability::Read,
                Capability::Action,
                Capability::Write,
                Capability::Execute
            ]
        ));
        assert!(capability_subset(&[Capability::Read], &[Capability::Read]));
        assert!(!capability_subset(
            &[Capability::Read],
            &[Capability::Action, Capability::Write]
        ));
        assert!(!capability_subset(
            &[Capability::Execute],
            &[Capability::Read, Capability::Action, Capability::Write]
        ));
        assert!(!capability_subset(&[Capability::Execute], &[]));
        assert!(capability_subset(
            &[Capability::Action, Capability::Write],
            &[Capability::Read, Capability::Action, Capability::Write]
        ));
        assert!(!capability_subset(
            &[Capability::Action, Capability::Write],
            &[Capability::Action]
        ));
        assert!(capability_subset(
            &[Capability::Read, Capability::Read],
            &[Capability::Read]
        ));
        assert!(capability_subset(
            &[Capability::Read],
            &[Capability::Read, Capability::Read]
        ));
    }
}
