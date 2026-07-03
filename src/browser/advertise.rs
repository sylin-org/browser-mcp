//! Tool advertisement filtering (browser plugin; RECONCILIATION.md section 1, g14).
//!
//! `tools/list` membership is a domain-independent visibility optimization: with no manifest,
//! the full sacred fixture is advertised verbatim (all-open stays byte-identical); with a
//! manifest, a tool is kept only when the UNION over every grant could ever permit it (no tab
//! exists at `tools/list` time, so this can never be a per-domain decision). Per-call
//! enforcement (`governance::enforcement`) remains the sole authoritative check regardless of
//! what this module returns; hiding a tool here is not denying it, and nothing in this module
//! may claim otherwise. Schema TEXT is never altered -- a kept tool object is the fixture
//! object, cloned unchanged; only which tools appear in the array changes.
//!
//! Dynamic re-advertisement (emitting MCP `notifications/tools/list_changed` when a manifest
//! reload changes the permitted set, per RECONCILIATION.md section 3) is NOT implemented here:
//! it needs a manifest-hot-reload mechanism (re-parse-validate-swap, fail-closed on an invalid
//! reload) that does not exist yet anywhere in the codebase -- g12 already deferred it
//! explicitly, and g13 built grant enforcement on top of the same fixed-at-startup snapshot.
//! [`advertised_tools`] is called once, at connection time, from that static snapshot; wiring
//! live re-advertisement is a follow-up task, not a gap in this one.

use crate::browser::classify;
use crate::governance::manifest::document::{Access, Grant};
use crate::governance::ports::RwClass;
use serde_json::Value;

/// Compute the advertised `{ "tools": [...] }` object. `fixture` is the parsed sacred
/// tool-schema fixture (`transport::mcp::tools::TOOLS_JSON`, parsed by the caller so this
/// browser-plugin module never depends on the transport layer). `grants` is `None` for no
/// manifest (all-open): `fixture` is returned verbatim, byte-identical, no tool ever dropped,
/// reordered, or edited. `Some(grants)` (including an empty slice) filters to the union over
/// every grant, in fixture order: a tool is kept when at least one grant's access class AND
/// tool list would ever let it through (see [`grant_permits`]). An empty `grants` slice permits
/// nothing, so the result is an empty list -- not the full surface.
pub fn advertised_tools(fixture: &Value, grants: Option<&[Grant]>) -> Value {
    let Some(grants) = grants else {
        return fixture.clone();
    };
    let tools = fixture["tools"]
        .as_array()
        .expect("the fixture has a top-level 'tools' array");
    let kept: Vec<Value> = tools
        .iter()
        .filter(|tool| {
            let name = tool["name"]
                .as_str()
                .expect("every fixture tool object has a string 'name'");
            grants.iter().any(|g| grant_permits(g, name))
        })
        .cloned()
        .collect();
    serde_json::json!({ "tools": kept })
}

/// Whether grant `g` could ever permit `tool_name`: both the tool-list check and the
/// access-class check must pass (shared format section 4.3 / section 8).
fn grant_permits(g: &Grant, tool_name: &str) -> bool {
    tool_list_permits(g, tool_name) && access_class_permits(g, tool_name)
}

/// The tool-list half of [`grant_permits`] (shared format section 4.3, `tools`/`exclude_tools`
/// are mutually exclusive): a non-null `tools` array is an allow-list; otherwise a present
/// `exclude_tools` is a deny-list; otherwise every tool passes.
fn tool_list_permits(g: &Grant, tool_name: &str) -> bool {
    match &g.tools {
        Some(list) => list.iter().any(|t| t == tool_name),
        None => match &g.exclude_tools {
            Some(excluded) => !excluded.iter().any(|t| t == tool_name),
            None => true,
        },
    }
}

/// The access-class half of [`grant_permits`] (shared format section 8).
///
/// `computer` is special-cased BEFORE calling `classify` (g14 required behavior section 2
/// point 1): `classify` needs a sub-action to classify a `computer` call, and advertisement
/// has none. `computer` has both observe sub-actions (`screenshot`, `scroll`, `zoom`, `wait`,
/// `hover`, `scroll_to`) and mutate sub-actions (`left_click`, `right_click`, `double_click`,
/// `triple_click`, `type`, `key`, `left_click_drag`), so ANY access class (`read`, `write`, or
/// `all`) permits at least one of them -- the access-class test always passes for `computer`.
/// Advertisement is coarse by design: it lists the tool whenever ANY use of it is reachable,
/// and per-call enforcement then denies the specific sub-actions the grant does not permit.
/// `computer` is dropped from the advertised list only via [`tool_list_permits`] (every grant
/// excludes it, or no grant's positive `tools` list includes it).
fn access_class_permits(g: &Grant, tool_name: &str) -> bool {
    if tool_name == "computer" {
        return true;
    }
    match classify::classify(tool_name, None) {
        Some(RwClass::Observe) => matches!(g.access, Access::Read | Access::All),
        Some(RwClass::Mutate) => matches!(g.access, Access::Write | Access::All),
        // Unreachable for a fixture tool while g05's exhaustiveness tests pass; fail closed
        // (not advertised) rather than panic.
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mcp::tools::TOOLS_JSON;

    fn fixture() -> Value {
        serde_json::from_str(TOOLS_JSON).expect("TOOLS_JSON parses")
    }

    fn names_of(result: &Value) -> Vec<String> {
        result["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .map(|t| t["name"].as_str().expect("name").to_string())
            .collect()
    }

    fn grant(access: Access, tools: Option<&[&str]>, exclude_tools: Option<&[&str]>) -> Grant {
        Grant {
            id: "g".to_string(),
            domains: vec!["example.com".to_string()],
            access,
            tools: tools.map(|v| v.iter().map(|s| s.to_string()).collect()),
            exclude_tools: exclude_tools.map(|v| v.iter().map(|s| s.to_string()).collect()),
            description: None,
            mode: None,
        }
    }

    #[test]
    fn no_manifest_returns_the_fixture_verbatim() {
        let fx = fixture();
        assert_eq!(
            advertised_tools(&fx, None),
            fx,
            "byte-identical, not just same names"
        );
    }

    #[test]
    fn read_only_manifest_yields_the_exact_eight_tool_set_in_fixture_order() {
        let fx = fixture();
        let grants = vec![grant(Access::Read, None, None)];
        let result = advertised_tools(&fx, Some(&grants));
        assert_eq!(
            names_of(&result),
            vec![
                "tabs_context_mcp",
                "computer",
                "find",
                "get_page_text",
                "read_console_messages",
                "read_network_requests",
                "read_page",
                "update_plan",
            ]
        );
    }

    #[test]
    fn a_tool_excluded_by_every_grant_is_omitted() {
        let fx = fixture();
        let grants = vec![grant(Access::All, None, Some(&["javascript_tool"]))];
        let names = names_of(&advertised_tools(&fx, Some(&grants)));
        assert_eq!(names.len(), 12, "the other 12 tools remain: {names:?}");
        assert!(!names.contains(&"javascript_tool".to_string()));
    }

    #[test]
    fn a_positive_tools_list_yields_exactly_that_set() {
        let fx = fixture();
        let grants = vec![grant(Access::All, Some(&["read_page"]), None)];
        assert_eq!(
            names_of(&advertised_tools(&fx, Some(&grants))),
            vec!["read_page"]
        );
    }

    #[test]
    fn empty_grants_array_yields_an_empty_list() {
        let fx = fixture();
        let result = advertised_tools(&fx, Some(&[]));
        assert_eq!(result["tools"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn computer_present_under_read_only_and_write_only_absent_when_excluded_everywhere() {
        let fx = fixture();
        let read_only = vec![grant(Access::Read, None, None)];
        assert!(
            names_of(&advertised_tools(&fx, Some(&read_only))).contains(&"computer".to_string())
        );

        let write_only = vec![grant(Access::Write, None, None)];
        assert!(
            names_of(&advertised_tools(&fx, Some(&write_only))).contains(&"computer".to_string())
        );

        let excludes_computer = vec![grant(Access::All, None, Some(&["computer"]))];
        assert!(!names_of(&advertised_tools(&fx, Some(&excludes_computer)))
            .contains(&"computer".to_string()));
    }
}
