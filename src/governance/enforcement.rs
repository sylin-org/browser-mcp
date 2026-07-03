//! Per-call grant enforcement (ADR-0018 step 3, g13): the pure decision core. This IS the
//! `PolicyDecisionPoint::decide` the a2 seam anticipated (RECONCILIATION.md section 2: "g13
//! `check_call` IS the pure `PolicyDecisionPoint::decide` over a serializable
//! `DecisionRequest`"); [`LocalPdp`] is the concrete, in-process implementation
//! `Governance::governed` uses once a manifest is active, alongside the `NoopPdp` (a2)
//! all-open placeholder.
//!
//! Pure: no I/O, no async, no clock. Grant-domain matching is injected as a function pointer
//! (`domain_matches: fn(pattern, host) -> bool`), supplied by the composition root using the
//! browser plugin's real G07 matcher, so this core module never names `browser::` directly
//! (the a7 arch-test forbids it) -- the same "known integration point" shape already used for
//! `classify` and `domain_pattern_valid` elsewhere in `governance/`.

use crate::governance::denial;
use crate::governance::manifest::document::{Access, Grant};
use crate::governance::ports::{
    Decision, DecisionRequest, Denial, EffectiveMode, GoverningResource, PolicyDecisionPoint,
    RwClass,
};

/// The in-process policy decision point wrapping [`check_call`]. `Governance::governed` uses
/// this once a manifest is active.
pub struct LocalPdp {
    domain_matches: fn(&str, &str) -> bool,
}

impl LocalPdp {
    /// `domain_matches(pattern, host)`: true when the ALREADY-VALIDATED grant domain `pattern`
    /// matches the ALREADY-NORMALIZED `host` (the browser plugin's real G07 matcher).
    pub fn new(domain_matches: fn(&str, &str) -> bool) -> Self {
        Self { domain_matches }
    }
}

impl PolicyDecisionPoint for LocalPdp {
    fn decide(&self, req: &DecisionRequest) -> Decision {
        check_call(
            &req.grants,
            &req.tool,
            req.action.as_deref(),
            req.rw,
            &req.resource,
            &req.manifest_hash,
            self.domain_matches,
            req.manifest_mode,
            req.config_mode,
        )
    }
}

/// Resolve the effective enforcement mode of one decision (shared format doc section 3.4,
/// g15): a resolving grant's own `mode` wins when set, else the manifest-level `mode`, else
/// the resolved `governance.mode`. `config` is never optional: the layered resolver always
/// defines `governance.mode` (the built-in Minimal preset is the floor), so resolution never
/// fails to produce a mode.
pub fn effective_mode(
    grant: Option<EffectiveMode>,
    manifest: Option<EffectiveMode>,
    config: EffectiveMode,
) -> EffectiveMode {
    grant.or(manifest).unwrap_or(config)
}

/// Wrap a raw `check_call` verdict into its final form (g15, ADR-0020 commitment 4): `Allow`
/// passes through unchanged (there is nothing to shadow); a `Deny` becomes `ShadowDeny` when
/// the effective mode resolves to `Observe`, else stays `Deny`. The resolving grant's own
/// `mode`, if any, is looked up by the denial's own `grant_id` -- `check_call` never needs to
/// thread a second grant reference through its internal helpers for this. Sacred-domain
/// denials never reach this function at all (they are a separate, always-on code path at the
/// dispatch chokepoint that never touches `Decision`/`check_call`), so every `Deny` this
/// function ever sees is eligible for the mode switch; there is no `sacred` rule to carve out
/// here.
pub(crate) fn apply_mode(
    decision: Decision,
    grants: &[Grant],
    manifest_mode: Option<EffectiveMode>,
    config_mode: EffectiveMode,
) -> Decision {
    let Decision::Deny(denial) = decision else {
        return decision;
    };
    let grant_mode = denial
        .grant_id
        .as_deref()
        .and_then(|id| grants.iter().find(|g| g.id == id))
        .and_then(|g| g.mode);
    match effective_mode(grant_mode, manifest_mode, config_mode) {
        EffectiveMode::Enforce => Decision::Deny(denial),
        EffectiveMode::Observe => Decision::ShadowDeny(denial),
    }
}

/// Render a tool's label for a denial message (shared format doc section 7.2): `computer`
/// calls render as `computer (<action>)`; every other tool renders its bare name.
fn tool_label(tool: &str, action: Option<&str>) -> String {
    match (tool, action) {
        ("computer", Some(action)) => format!("computer ({action})"),
        _ => tool.to_string(),
    }
}

/// The pure per-call grant-resolution decision (shared format doc sections 4.3, 4.5, 7, 8).
/// STEP 0 (no manifest -> allow) lives at the caller ([`crate::governance::dispatch::Governance::decide`]);
/// this function always assumes a manifest is active. Order is load-bearing (the denial id
/// depends on the rule string, so the first failing rule must be deterministic): resource-kind
/// dispatch first (`AlwaysAllow`/`OutOfScope`/`Indeterminate`/`None`/`Resource`), then, for a
/// resolved host, grant resolution, THEN the tool-list check, THEN the access check.
#[allow(clippy::too_many_arguments)]
pub fn check_call(
    grants: &[Grant],
    tool: &str,
    action: Option<&str>,
    rw: RwClass,
    resource: &GoverningResource,
    manifest_hash: &str,
    domain_matches: fn(&str, &str) -> bool,
    manifest_mode: Option<EffectiveMode>,
    config_mode: EffectiveMode,
) -> Decision {
    let raw = match resource {
        GoverningResource::AlwaysAllow => Decision::Allow { grant_id: None },
        GoverningResource::OutOfScope(scheme) => {
            Decision::Deny(scheme_denial(scheme, manifest_hash))
        }
        GoverningResource::Indeterminate => {
            Decision::Deny(unmatched_domain_denial("(unknown)", manifest_hash))
        }
        GoverningResource::Resource(host) => decide_for_host(
            grants,
            tool,
            action,
            rw,
            host,
            manifest_hash,
            domain_matches,
        ),
        GoverningResource::None => decide_no_page(grants, tool, action, rw, manifest_hash),
    };
    apply_mode(raw, grants, manifest_mode, config_mode)
}

fn decide_for_host(
    grants: &[Grant],
    tool: &str,
    action: Option<&str>,
    rw: RwClass,
    host: &str,
    manifest_hash: &str,
    domain_matches: fn(&str, &str) -> bool,
) -> Decision {
    let Some(grant) = first_matching_grant(grants, host, domain_matches) else {
        return Decision::Deny(unmatched_domain_denial(host, manifest_hash));
    };
    if let Some(denial) = tool_list_denial(grant, tool, action, host, manifest_hash) {
        return Decision::Deny(denial);
    }
    if !access_covers(grant.access, rw) {
        return Decision::Deny(access_denial(grant, tool, action, rw, host, manifest_hash));
    }
    Decision::Allow {
        grant_id: Some(grant.id.clone()),
    }
}

/// The `NoPage` union rule (shared format doc section 4.3, mirroring G14's advertisement
/// membership test so per-call is never more permissive than advertisement): candidates are
/// the grants passing the tool-list check, in manifest order; allow if any candidate's access
/// covers `rw`; else deny `access` attributed to the first candidate; no candidates at all ->
/// deny `tool/<tool>` with no grant id.
fn decide_no_page(
    grants: &[Grant],
    tool: &str,
    action: Option<&str>,
    rw: RwClass,
    manifest_hash: &str,
) -> Decision {
    let candidates: Vec<&Grant> = grants
        .iter()
        .filter(|g| tool_list_denial(g, tool, action, "(unknown)", manifest_hash).is_none())
        .collect();
    let Some(first) = candidates.first() else {
        return Decision::Deny(tool_denial(tool, action, None, "(unknown)", manifest_hash));
    };
    if let Some(grant) = candidates.iter().find(|g| access_covers(g.access, rw)) {
        return Decision::Allow {
            grant_id: Some(grant.id.clone()),
        };
    }
    Decision::Deny(access_denial(
        first,
        tool,
        action,
        rw,
        "(unknown)",
        manifest_hash,
    ))
}

/// First grant, in manifest order, with any domain pattern matching `host` (shared format doc
/// section 4.3: first match wins).
fn first_matching_grant<'a>(
    grants: &'a [Grant],
    host: &str,
    domain_matches: fn(&str, &str) -> bool,
) -> Option<&'a Grant> {
    grants.iter().find(|g| {
        g.domains
            .iter()
            .any(|pattern| domain_matches(pattern, host))
    })
}

/// `None` when `tool` passes the grant's `tools`/`exclude_tools` check (shared format doc
/// section 4.3: a non-null `tools` list is an allow-list; otherwise `exclude_tools`, if
/// present, is a deny-list). Checked as the literal tool name (`"computer"`, never an action).
fn tool_list_denial(
    grant: &Grant,
    tool: &str,
    action: Option<&str>,
    domain: &str,
    manifest_hash: &str,
) -> Option<Denial> {
    let allowed = match &grant.tools {
        Some(list) => list.iter().any(|t| t == tool),
        None => match &grant.exclude_tools {
            Some(excluded) => !excluded.iter().any(|t| t == tool),
            None => true,
        },
    };
    if allowed {
        None
    } else {
        Some(tool_denial(
            tool,
            action,
            Some(grant),
            domain,
            manifest_hash,
        ))
    }
}

/// Whether `access` authorizes a call of class `rw` (shared format doc section 8): `Observe`
/// requires `read` or `all`; `Mutate` requires `write` or `all` (`write` does NOT imply
/// `read`).
fn access_covers(access: Access, rw: RwClass) -> bool {
    matches!(
        (access, rw),
        (Access::All, _) | (Access::Read, RwClass::Observe) | (Access::Write, RwClass::Mutate)
    )
}

fn unmatched_domain_denial(domain: &str, manifest_hash: &str) -> Denial {
    let rule = "unmatched_domain".to_string();
    let denial_id = denial::denial_id(manifest_hash, "", &rule);
    let message = format!(
        "Denied ({denial_id}): no grant covers {domain}. Tool use is limited to domains your \
         policy grants. Give this denial id to your administrator if access to {domain} is \
         needed."
    );
    Denial {
        rule,
        grant_id: None,
        denial_id,
        domain: domain.to_string(),
        message,
    }
}

fn scheme_denial(scheme: &str, manifest_hash: &str) -> Denial {
    let rule = format!("scheme/{scheme}");
    let denial_id = denial::denial_id(manifest_hash, "", &rule);
    let message = format!(
        "Denied ({denial_id}): the URL scheme '{scheme}:' is not permitted under the active \
         policy. Only http and https pages can be automated."
    );
    Denial {
        rule,
        grant_id: None,
        denial_id,
        domain: String::new(),
        message,
    }
}

/// The denial for a call whose read/write class could not be determined (`classify` returned
/// `None`: an unknown tool, or a `computer` call with a missing/unknown action). Under a
/// manifest, an unclassifiable call is never authorized. Public: [`crate::governance::dispatch
/// ::Governance::decide`] (the caller) builds this BEFORE constructing a `DecisionRequest`,
/// since without a resolved `RwClass` there is no request to build.
pub fn unclassifiable_denial(tool: &str, action: Option<&str>, manifest_hash: &str) -> Denial {
    tool_denial(tool, action, None, "(unknown)", manifest_hash)
}

fn tool_denial(
    tool: &str,
    action: Option<&str>,
    grant: Option<&Grant>,
    domain: &str,
    manifest_hash: &str,
) -> Denial {
    let rule = format!("tool/{tool}");
    let grant_id_str = grant.map(|g| g.id.as_str()).unwrap_or("");
    let denial_id = denial::denial_id(manifest_hash, grant_id_str, &rule);
    let label = tool_label(tool, action);
    let message = match grant {
        Some(g) => format!(
            "Denied ({denial_id}): grant '{grant_id}' does not permit '{label}' on {domain}. \
             Other tools in your access class remain available. Give this denial id to your \
             administrator to request '{label}'.",
            grant_id = g.id
        ),
        None => format!(
            "Denied ({denial_id}): no grant permits '{label}'. Give this denial id to your \
             administrator to request '{label}'."
        ),
    };
    Denial {
        rule,
        grant_id: grant.map(|g| g.id.clone()),
        denial_id,
        domain: domain.to_string(),
        message,
    }
}

fn access_denial(
    grant: &Grant,
    tool: &str,
    action: Option<&str>,
    rw: RwClass,
    domain: &str,
    manifest_hash: &str,
) -> Denial {
    let rule = "access".to_string();
    let denial_id = denial::denial_id(manifest_hash, &grant.id, &rule);
    let label = tool_label(tool, action);
    let message = match rw {
        RwClass::Mutate => format!(
            "Denied ({denial_id}): '{label}' needs write access on {domain}, and grant \
             '{grant_id}' allows read only. Observation tools (read_page, get_page_text, find, \
             screenshot) remain available. Give this denial id to your administrator to \
             request write access.",
            grant_id = grant.id
        ),
        RwClass::Observe => format!(
            "Denied ({denial_id}): '{label}' needs read access on {domain}, and grant \
             '{grant_id}' allows write only. Give this denial id to your administrator.",
            grant_id = grant.id
        ),
    };
    Denial {
        rule,
        grant_id: Some(grant.id.clone()),
        denial_id,
        domain: domain.to_string(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for the real G07 matcher: exact string equality, or a `*.` prefix meaning
    /// "any host ending in `.suffix`" -- just enough grammar for these pure tests, never the
    /// authoritative grammar (that lives in `browser::pattern`'s own exhaustive tests).
    fn stub_domain_matches(pattern: &str, host: &str) -> bool {
        match pattern.strip_prefix("*.") {
            Some(suffix) => host.ends_with(&format!(".{suffix}")),
            None => pattern == host,
        }
    }

    fn grant(id: &str, domains: &[&str], access: Access) -> Grant {
        Grant {
            id: id.to_string(),
            domains: domains.iter().map(|d| d.to_string()).collect(),
            access,
            tools: None,
            exclude_tools: None,
            description: None,
            mode: None,
        }
    }

    /// The g13-era convenience wrapper: always `manifest_mode: None, config_mode: Enforce`, so
    /// every pre-g15 test keeps asserting `Deny` for a would-deny exactly as before. Tests that
    /// specifically exercise the g15 mode switch use [`check_with_mode`] instead.
    fn check(grants: &[Grant], tool: &str, rw: RwClass, resource: &GoverningResource) -> Decision {
        check_with_mode(grants, tool, rw, resource, None, EffectiveMode::Enforce)
    }

    #[allow(clippy::too_many_arguments)]
    fn check_with_mode(
        grants: &[Grant],
        tool: &str,
        rw: RwClass,
        resource: &GoverningResource,
        manifest_mode: Option<EffectiveMode>,
        config_mode: EffectiveMode,
    ) -> Decision {
        check_call(
            grants,
            tool,
            None,
            rw,
            resource,
            "hash",
            stub_domain_matches,
            manifest_mode,
            config_mode,
        )
    }

    fn host(h: &str) -> GoverningResource {
        GoverningResource::Resource(h.to_string())
    }

    #[test]
    fn first_matching_grant_wins() {
        let grants = vec![
            grant("first", &["example.com"], Access::Read),
            grant("second", &["example.com"], Access::All),
        ];
        // rw Mutate: "first" (read-only) would deny access; "second" (all) would allow. Since
        // "first" resolves (it is earlier), the outcome must be a deny attributed to "first".
        match check(&grants, "form_input", RwClass::Mutate, &host("example.com")) {
            Decision::Deny(d) => assert_eq!(d.grant_id.as_deref(), Some("first")),
            other => panic!("expected a deny attributed to the first grant, got {other:?}"),
        }
    }

    #[test]
    fn unmatched_domain_denies() {
        let grants = vec![grant("g1", &["example.com"], Access::All)];
        match check(&grants, "form_input", RwClass::Mutate, &host("evil.com")) {
            Decision::Deny(d) => {
                assert_eq!(d.rule, "unmatched_domain");
                assert_eq!(d.grant_id, None);
            }
            other => panic!("expected unmatched_domain, got {other:?}"),
        }
    }

    #[test]
    fn access_rules() {
        let read_grant = vec![grant("r", &["example.com"], Access::Read)];
        let write_grant = vec![grant("w", &["example.com"], Access::Write)];
        let all_grant = vec![grant("a", &["example.com"], Access::All)];

        match check(
            &read_grant,
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
        ) {
            Decision::Deny(d) => {
                assert_eq!(d.rule, "access");
                assert_eq!(d.grant_id.as_deref(), Some("r"));
            }
            other => panic!("expected access deny, got {other:?}"),
        }
        match check(
            &write_grant,
            "read_page",
            RwClass::Observe,
            &host("example.com"),
        ) {
            Decision::Deny(d) => assert_eq!(d.rule, "access"),
            other => panic!("expected access deny (write does not imply read), got {other:?}"),
        }
        assert!(matches!(
            check(
                &read_grant,
                "read_page",
                RwClass::Observe,
                &host("example.com")
            ),
            Decision::Allow { .. }
        ));
        assert!(matches!(
            check(
                &write_grant,
                "form_input",
                RwClass::Mutate,
                &host("example.com")
            ),
            Decision::Allow { .. }
        ));
        assert!(matches!(
            check(
                &all_grant,
                "form_input",
                RwClass::Mutate,
                &host("example.com")
            ),
            Decision::Allow { .. }
        ));
        assert!(matches!(
            check(
                &all_grant,
                "read_page",
                RwClass::Observe,
                &host("example.com")
            ),
            Decision::Allow { .. }
        ));
    }

    #[test]
    fn tool_list_rules() {
        let mut allow_list_grant = grant("g", &["example.com"], Access::All);
        allow_list_grant.tools = Some(vec!["read_page".to_string()]);
        match check(
            &[allow_list_grant],
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
        ) {
            Decision::Deny(d) => assert_eq!(d.rule, "tool/form_input"),
            other => panic!("expected tool/form_input deny, got {other:?}"),
        }

        let mut exclude_grant = grant("g", &["example.com"], Access::All);
        exclude_grant.exclude_tools = Some(vec!["javascript_tool".to_string()]);
        match check(
            &[exclude_grant],
            "javascript_tool",
            RwClass::Mutate,
            &host("example.com"),
        ) {
            Decision::Deny(d) => assert_eq!(d.rule, "tool/javascript_tool"),
            other => panic!("expected tool/javascript_tool deny, got {other:?}"),
        }

        // A computer call is checked as the string "computer" regardless of action.
        let mut computer_excluded = grant("g", &["example.com"], Access::All);
        computer_excluded.exclude_tools = Some(vec!["computer".to_string()]);
        let decision = check_call(
            &[computer_excluded],
            "computer",
            Some("left_click"),
            RwClass::Mutate,
            &host("example.com"),
            "hash",
            stub_domain_matches,
            None,
            EffectiveMode::Enforce,
        );
        match decision {
            Decision::Deny(d) => assert_eq!(d.rule, "tool/computer"),
            other => panic!("expected tool/computer deny, got {other:?}"),
        }
    }

    #[test]
    fn tool_check_precedes_access_check() {
        // A grant that both excludes the tool AND lacks the class must deny with rule
        // "tool/...", not "access".
        let mut g = grant("g", &["example.com"], Access::Read);
        g.exclude_tools = Some(vec!["form_input".to_string()]);
        match check(&[g], "form_input", RwClass::Mutate, &host("example.com")) {
            Decision::Deny(d) => assert_eq!(d.rule, "tool/form_input"),
            other => panic!("expected tool/form_input (not access), got {other:?}"),
        }
    }

    #[test]
    fn computer_subactions_split() {
        let read_grant = vec![grant("r", &["example.com"], Access::Read)];
        let all_grant = vec![grant("a", &["example.com"], Access::All)];

        let allow = check_call(
            &read_grant,
            "computer",
            Some("screenshot"),
            RwClass::Observe,
            &host("example.com"),
            "hash",
            stub_domain_matches,
            None,
            EffectiveMode::Enforce,
        );
        assert!(matches!(allow, Decision::Allow { .. }));

        let deny = check_call(
            &read_grant,
            "computer",
            Some("left_click"),
            RwClass::Mutate,
            &host("example.com"),
            "hash",
            stub_domain_matches,
            None,
            EffectiveMode::Enforce,
        );
        match deny {
            Decision::Deny(d) => assert_eq!(d.rule, "access"),
            other => panic!("expected access deny, got {other:?}"),
        }

        for (action, rw) in [
            ("screenshot", RwClass::Observe),
            ("left_click", RwClass::Mutate),
        ] {
            let allow = check_call(
                &all_grant,
                "computer",
                Some(action),
                rw,
                &host("example.com"),
                "hash",
                stub_domain_matches,
                None,
                EffectiveMode::Enforce,
            );
            assert!(matches!(allow, Decision::Allow { .. }), "action {action}");
        }
    }

    #[test]
    fn scheme_and_about_blank() {
        let grants = vec![grant("g", &["example.com"], Access::All)];
        for scheme in ["chrome", "file", "javascript"] {
            match check(
                &grants,
                "navigate",
                RwClass::Observe,
                &GoverningResource::OutOfScope(scheme.to_string()),
            ) {
                Decision::Deny(d) => assert_eq!(d.rule, format!("scheme/{scheme}")),
                other => panic!("expected scheme deny, got {other:?}"),
            }
        }
        assert_eq!(
            check(
                &grants,
                "navigate",
                RwClass::Observe,
                &GoverningResource::AlwaysAllow
            ),
            Decision::Allow { grant_id: None }
        );
    }

    #[test]
    fn unknown_fails_closed() {
        let grants = vec![grant("g", &["example.com"], Access::All)];
        match check(
            &grants,
            "read_page",
            RwClass::Observe,
            &GoverningResource::Indeterminate,
        ) {
            Decision::Deny(d) => {
                assert_eq!(d.rule, "unmatched_domain");
                assert_eq!(d.grant_id, None);
            }
            other => panic!("expected unmatched_domain deny, got {other:?}"),
        }
    }

    #[test]
    fn no_page_union_rule() {
        let read_grant = vec![grant("r1", &["example.com"], Access::Read)];
        assert!(matches!(
            check(&read_grant, "tabs_context_mcp", RwClass::Observe, &GoverningResource::None),
            Decision::Allow { grant_id: Some(ref g) } if g == "r1"
        ));
        match check(
            &read_grant,
            "tabs_create_mcp",
            RwClass::Mutate,
            &GoverningResource::None,
        ) {
            Decision::Deny(d) => assert_eq!(d.rule, "access"),
            other => panic!("expected access deny, got {other:?}"),
        }

        let all_grant = vec![grant("a1", &["example.com"], Access::All)];
        assert!(matches!(
            check(
                &all_grant,
                "tabs_create_mcp",
                RwClass::Mutate,
                &GoverningResource::None
            ),
            Decision::Allow { .. }
        ));

        let mut excluding = grant("e1", &["example.com"], Access::All);
        excluding.exclude_tools = Some(vec!["tabs_create_mcp".to_string()]);
        match check(
            &[excluding],
            "tabs_create_mcp",
            RwClass::Mutate,
            &GoverningResource::None,
        ) {
            Decision::Deny(d) => {
                assert_eq!(d.rule, "tool/tabs_create_mcp");
                assert_eq!(d.grant_id, None);
            }
            other => panic!("expected tool/tabs_create_mcp deny with no grant id, got {other:?}"),
        }
    }

    #[test]
    fn unclassifiable_denies_via_the_tool_rule() {
        // g13 leaves the "classify returned None" branch to the caller (Governance::decide);
        // this pins that the tool/<name> denial shape it must produce is available and
        // correctly formed, using the same tool_denial building block check_call itself uses.
        let denial = tool_denial("no_such_tool", None, None, "(unknown)", "hash");
        assert_eq!(denial.rule, "tool/no_such_tool");
        assert_eq!(denial.grant_id, None);
        assert!(denial.message.starts_with("Denied (D-"));
    }

    // --- g15: shadow enforcement (the mode switch) ---

    #[test]
    fn effective_mode_precedence_covers_every_combination() {
        // Grant wins when set, regardless of manifest/config.
        assert_eq!(
            effective_mode(
                Some(EffectiveMode::Observe),
                Some(EffectiveMode::Enforce),
                EffectiveMode::Enforce
            ),
            EffectiveMode::Observe
        );
        assert_eq!(
            effective_mode(
                Some(EffectiveMode::Enforce),
                Some(EffectiveMode::Observe),
                EffectiveMode::Observe
            ),
            EffectiveMode::Enforce
        );
        // Manifest wins when grant is None.
        assert_eq!(
            effective_mode(None, Some(EffectiveMode::Observe), EffectiveMode::Enforce),
            EffectiveMode::Observe
        );
        assert_eq!(
            effective_mode(None, Some(EffectiveMode::Enforce), EffectiveMode::Observe),
            EffectiveMode::Enforce
        );
        // Config wins when both grant and manifest are None.
        assert_eq!(
            effective_mode(None, None, EffectiveMode::Observe),
            EffectiveMode::Observe
        );
        assert_eq!(
            effective_mode(None, None, EffectiveMode::Enforce),
            EffectiveMode::Enforce
        );
    }

    #[test]
    fn mode_switch_yields_shadow_deny_under_observe_with_the_identical_grant_and_denial_id() {
        // access, tool, and unmatched_domain denials all go through the same apply_mode wrap;
        // exercise all three (scheme is covered by scheme_and_about_blank's own case already).
        let read_grant = vec![grant("r", &["example.com"], Access::Read)];

        let enforce = check_with_mode(
            &read_grant,
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
            None,
            EffectiveMode::Enforce,
        );
        let observe = check_with_mode(
            &read_grant,
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
            None,
            EffectiveMode::Observe,
        );
        match (enforce, observe) {
            (Decision::Deny(d_enforce), Decision::ShadowDeny(d_observe)) => {
                assert_eq!(d_enforce.rule, "access");
                assert_eq!(d_enforce.grant_id, d_observe.grant_id);
                assert_eq!(d_enforce.denial_id, d_observe.denial_id);
            }
            other => panic!("expected (Deny, ShadowDeny) for the access rule, got {other:?}"),
        }

        let mut excluding = grant("g", &["example.com"], Access::All);
        excluding.exclude_tools = Some(vec!["form_input".to_string()]);
        let enforce = check_with_mode(
            &[excluding.clone()],
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
            None,
            EffectiveMode::Enforce,
        );
        let observe = check_with_mode(
            &[excluding],
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
            None,
            EffectiveMode::Observe,
        );
        match (enforce, observe) {
            (Decision::Deny(d_enforce), Decision::ShadowDeny(d_observe)) => {
                assert_eq!(d_enforce.rule, "tool/form_input");
                assert_eq!(d_enforce.denial_id, d_observe.denial_id);
            }
            other => panic!("expected (Deny, ShadowDeny) for the tool rule, got {other:?}"),
        }

        let enforce = check_with_mode(
            &read_grant,
            "form_input",
            RwClass::Mutate,
            &host("evil.com"),
            None,
            EffectiveMode::Enforce,
        );
        let observe = check_with_mode(
            &read_grant,
            "form_input",
            RwClass::Mutate,
            &host("evil.com"),
            None,
            EffectiveMode::Observe,
        );
        match (enforce, observe) {
            (Decision::Deny(d_enforce), Decision::ShadowDeny(d_observe)) => {
                assert_eq!(d_enforce.rule, "unmatched_domain");
                assert_eq!(d_enforce.grant_id, None);
                assert_eq!(d_enforce.grant_id, d_observe.grant_id);
                assert_eq!(d_enforce.denial_id, d_observe.denial_id);
            }
            other => panic!("expected (Deny, ShadowDeny) for unmatched_domain, got {other:?}"),
        }
    }

    #[test]
    fn mode_switch_never_touches_an_allow() {
        let all_grant = vec![grant("a", &["example.com"], Access::All)];
        let observe = check_with_mode(
            &all_grant,
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
            None,
            EffectiveMode::Observe,
        );
        assert!(matches!(observe, Decision::Allow { .. }));
    }

    #[test]
    fn grant_level_mode_overrides_manifest_and_config() {
        let mut observe_grant = grant("g", &["example.com"], Access::Read);
        observe_grant.mode = Some(EffectiveMode::Observe);
        let decision = check_with_mode(
            &[observe_grant],
            "form_input",
            RwClass::Mutate,
            &host("example.com"),
            Some(EffectiveMode::Enforce),
            EffectiveMode::Enforce,
        );
        assert!(
            matches!(decision, Decision::ShadowDeny(_)),
            "the grant's own observe mode must win over an enforcing manifest and config: {decision:?}"
        );
    }

    #[test]
    fn unclassifiable_call_goes_through_the_same_mode_switch() {
        // Governance::decide applies apply_mode to the classification-miss denial too (it is an
        // ordinary tool/<name> rule, not a sacred one); pin the building block's own denial
        // shape stays eligible (the wrapping itself is exercised at the Governance::decide level).
        let denial = unclassifiable_denial("no_such_tool", None, "hash");
        let grants: Vec<Grant> = Vec::new();
        let shadowed = apply_mode(
            Decision::Deny(denial.clone()),
            &grants,
            None,
            EffectiveMode::Observe,
        );
        assert!(matches!(shadowed, Decision::ShadowDeny(d) if d.denial_id == denial.denial_id));
        let enforced = apply_mode(
            Decision::Deny(denial),
            &grants,
            None,
            EffectiveMode::Enforce,
        );
        assert!(matches!(enforced, Decision::Deny(_)));
    }
}
