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

// --- Supporting placeholder and axis types ---

/// Read/write classification of a tool call: the observe-vs-mutate axis (the core owns the
/// axis; g05 owns the tool+action -> class table in the browser plugin). `Read` is an
/// observation; `Write` is a mutation. g05 maps each tool/action onto this and MAY extend
/// the type minimally when it lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RwClass {
    Read,
    Write,
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

/// One resolved manifest grant. Placeholder: g12 (manifest engine) fleshes this out to
/// `{ domains, access, tools, mode }`. Only `id` is defined now, so `Decision::Allow` can
/// attribute the matching grant (g13). Kept minimal and serde-round-trippable on purpose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    /// Stable identifier of this grant, used for allow-attribution and audit.
    pub id: String,
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

/// A structured denial. Placeholder: g08 introduces the stable denial-id scheme and g13 the
/// full reason set. Two fields now, both serde-round-trippable, so `Decision::Deny` and
/// `Decision::ShadowDeny` carry something meaningful before g08 lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Denial {
    /// Stable denial identifier (g08 pins the scheme).
    pub denial_id: String,
    /// Human-readable reason surfaced to the caller.
    pub reason: String,
}

/// One audit record: the flight-recorder line for a single tool call. Placeholder: g06
/// fleshes out the full record (identity, client, tool, action, rw, domain, decision,
/// timing). Only `tool` is defined now so `AuditSink` has a concrete argument type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// The tool that was called.
    pub tool: String,
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
    /// The tool call's read/write classification.
    pub rw: RwClass,
    /// The resolved governing resource.
    pub resource: GoverningResource,
    /// The effective enforcement mode.
    pub mode: EffectiveMode,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request(
        rw: RwClass,
        resource: GoverningResource,
        mode: EffectiveMode,
    ) -> DecisionRequest {
        DecisionRequest {
            grants: Vec::new(),
            tool: "navigate".to_string(),
            rw,
            resource,
            mode,
        }
    }

    #[test]
    fn noop_pdp_allows_every_request() {
        let pdp = NoopPdp;
        let requests = [
            sample_request(
                RwClass::Read,
                GoverningResource::None,
                EffectiveMode::Observe,
            ),
            sample_request(
                RwClass::Write,
                GoverningResource::Resource("example.com".to_string()),
                EffectiveMode::Enforce,
            ),
            DecisionRequest {
                grants: vec![Grant {
                    id: "g1".to_string(),
                }],
                tool: "computer".to_string(),
                rw: RwClass::Write,
                resource: GoverningResource::AlwaysAllow,
                mode: EffectiveMode::Enforce,
            },
        ];
        for req in &requests {
            assert_eq!(pdp.decide(req), Decision::Allow { grant_id: None });
        }
    }

    #[test]
    fn null_sink_record_is_a_noop() {
        let sink = NullSink;
        sink.record(&AuditRecord {
            tool: "navigate".to_string(),
        });
    }

    #[test]
    fn pdp_is_object_safe() {
        let pdp: Box<dyn PolicyDecisionPoint> = Box::new(NoopPdp);
        let req = sample_request(
            RwClass::Read,
            GoverningResource::None,
            EffectiveMode::Observe,
        );
        assert_eq!(pdp.decide(&req), Decision::Allow { grant_id: None });
    }

    #[test]
    fn audit_sink_is_object_safe() {
        let sink: Box<dyn AuditSink> = Box::new(NullSink);
        sink.record(&AuditRecord {
            tool: "read_page".to_string(),
        });
    }

    #[test]
    fn decision_request_round_trips_through_serde() {
        let req = DecisionRequest {
            grants: vec![Grant {
                id: "servicenow-full".to_string(),
            }],
            tool: "navigate".to_string(),
            rw: RwClass::Write,
            resource: GoverningResource::Resource("example.com".to_string()),
            mode: EffectiveMode::Enforce,
        };
        let json = serde_json::to_string(&req).expect("serializes");
        let round_tripped: DecisionRequest = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(req, round_tripped);
    }

    #[test]
    fn decision_round_trips_through_serde() {
        let denial = Denial {
            denial_id: "D-9f3a1c2e".to_string(),
            reason: "no grant covers this domain".to_string(),
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
        assert_eq!(serde_json::to_string(&RwClass::Read).unwrap(), "\"read\"");
        assert_eq!(serde_json::to_string(&RwClass::Write).unwrap(), "\"write\"");
        assert_eq!(
            serde_json::to_string(&EffectiveMode::Observe).unwrap(),
            "\"observe\""
        );
        assert_eq!(
            serde_json::to_string(&EffectiveMode::Enforce).unwrap(),
            "\"enforce\""
        );
    }
}
