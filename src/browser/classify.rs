//! The authoritative read/write classification of the sacred tool surface (shared format doc
//! section 8, ADR-0018 step 1). This supersedes SPEC 3.1/3.3/5.4's older three-tier
//! Observe/Mutate/Manage model; consult only shared format doc section 8 for class
//! assignments, never the SPEC. Both the audit recorder (`rw` field, g06) and grant
//! enforcement (`access: read | write | all`, later tasks) consume [`classify`].
//!
//! This is the browser PLUGIN half of classification: the observe/mutate axis type itself
//! ([`crate::governance::ports::RwClass`]) is domain-agnostic core; the concrete 13-tool table
//! is browser-domain data (RECONCILIATION.md section 1) that implements the plugin side of
//! [`crate::governance::ports::DomainPolicy::classify`].
//!
//! The module is pure: no I/O, no allocation beyond what slice iteration needs, no
//! dependencies beyond `core`/`std`.

use crate::governance::ports::RwClass;

/// One entry per tool EXCEPT `computer` (12 entries), in the tools.json advertised order.
/// `computer` is deliberately absent: it is classified per sub-action via
/// [`COMPUTER_ACTION_CLASSES`], and a test asserts its absence here.
pub const TOOL_CLASSES: &[(&str, RwClass)] = &[
    ("tabs_context_mcp", RwClass::Observe),
    ("tabs_create_mcp", RwClass::Mutate),
    // navigate is Observe: provably a GET (top-level document load), per ADR-0022
    // (Context + Decision 2). Reclassified by s01; supersedes the shared format doc
    // section 8 row (bannered in s08). Navigation remains the domain-enforcement point
    // (pre-dispatch target check + landing check); those are host checks, not class checks.
    ("navigate", RwClass::Observe),
    ("find", RwClass::Observe),
    ("form_input", RwClass::Mutate),
    ("get_page_text", RwClass::Observe),
    ("javascript_tool", RwClass::Mutate),
    ("read_console_messages", RwClass::Observe),
    ("read_network_requests", RwClass::Observe),
    ("read_page", RwClass::Observe),
    ("resize_window", RwClass::Mutate),
    ("update_plan", RwClass::Observe),
];

/// One entry per `computer` sub-action (13 entries), in the tools.json `action` enum order.
///
/// Rationale (shared format doc section 8): the observe set reads or reveals page state
/// without committing input that changes application state. `scroll`, `hover`, and
/// `scroll_to` dispatch input events but only move the viewport or pointer; a read-only grant
/// that cannot scroll cannot read a page below the fold, which would make read access useless
/// in practice. This deliberately supersedes SPEC 3.3's "scroll is mutate because it dispatches
/// input" rationale. Observe: `screenshot`, `scroll`, `zoom`, `wait`, `hover`, `scroll_to` (6).
/// Mutate: `left_click`, `right_click`, `double_click`, `triple_click`, `type`, `key`,
/// `left_click_drag` (7).
pub const COMPUTER_ACTION_CLASSES: &[(&str, RwClass)] = &[
    ("left_click", RwClass::Mutate),
    ("right_click", RwClass::Mutate),
    ("type", RwClass::Mutate),
    ("screenshot", RwClass::Observe),
    ("wait", RwClass::Observe),
    ("scroll", RwClass::Observe),
    ("key", RwClass::Mutate),
    ("left_click_drag", RwClass::Mutate),
    ("double_click", RwClass::Mutate),
    ("triple_click", RwClass::Mutate),
    ("zoom", RwClass::Observe),
    ("scroll_to", RwClass::Observe),
    ("hover", RwClass::Observe),
];

/// Classify one tool call. `action` is consulted only when `tool` is `"computer"`; for every
/// other tool it is ignored. Returns `None` for a tool name not on the sacred surface, and for
/// a `computer` call whose action is absent or unknown. `None` is a classification miss, not a
/// denial; what callers do with it is decided by the consuming tasks, not here.
///
/// Note for future consumers: grant-level `tools` / `exclude_tools` checks match the literal
/// tool name `"computer"`, never an action name (shared format doc section 4.3); this function
/// is for the observe/mutate axis only.
pub fn classify(tool: &str, action: Option<&str>) -> Option<RwClass> {
    if tool == "computer" {
        let action = action?;
        return COMPUTER_ACTION_CLASSES
            .iter()
            .find(|(a, _)| *a == action)
            .map(|(_, class)| *class);
    }
    TOOL_CLASSES
        .iter()
        .find(|(t, _)| *t == tool)
        .map(|(_, class)| *class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mcp::tools::TOOLS_JSON;
    use std::collections::HashSet;

    fn sacred_tool_names() -> HashSet<String> {
        let v: serde_json::Value = serde_json::from_str(TOOLS_JSON).unwrap();
        v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    }

    fn sacred_computer_actions() -> HashSet<String> {
        let v: serde_json::Value = serde_json::from_str(TOOLS_JSON).unwrap();
        let computer = v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "computer")
            .expect("computer tool present");
        computer["inputSchema"]["properties"]["action"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn tool_table_matches_the_sacred_surface() {
        let sacred = sacred_tool_names();
        let mut expected: HashSet<String> = sacred.clone();
        expected.remove("computer");

        let table_names: HashSet<String> =
            TOOL_CLASSES.iter().map(|(t, _)| t.to_string()).collect();
        assert_eq!(table_names, expected, "no gaps, no stale entries");

        assert!(
            !TOOL_CLASSES.iter().any(|(t, _)| *t == "computer"),
            "computer must not appear in TOOL_CLASSES"
        );
        let mut seen = HashSet::new();
        for (t, _) in TOOL_CLASSES {
            assert!(seen.insert(*t), "duplicate entry: {t}");
        }
    }

    #[test]
    fn computer_action_table_matches_the_sacred_enum() {
        let sacred_actions = sacred_computer_actions();
        let table_actions: HashSet<String> = COMPUTER_ACTION_CLASSES
            .iter()
            .map(|(a, _)| a.to_string())
            .collect();
        assert_eq!(table_actions, sacred_actions);
        assert_eq!(COMPUTER_ACTION_CLASSES.len(), 13);
        let mut seen = HashSet::new();
        for (a, _) in COMPUTER_ACTION_CLASSES {
            assert!(seen.insert(*a), "duplicate action: {a}");
        }
    }

    #[test]
    fn classification_matches_the_shared_format_table() {
        assert_eq!(classify("tabs_context_mcp", None), Some(RwClass::Observe));
        assert_eq!(classify("tabs_create_mcp", None), Some(RwClass::Mutate));
        assert_eq!(classify("navigate", None), Some(RwClass::Observe));
        assert_eq!(classify("find", None), Some(RwClass::Observe));
        assert_eq!(classify("form_input", None), Some(RwClass::Mutate));
        assert_eq!(classify("get_page_text", None), Some(RwClass::Observe));
        assert_eq!(classify("javascript_tool", None), Some(RwClass::Mutate));
        assert_eq!(
            classify("read_console_messages", None),
            Some(RwClass::Observe)
        );
        assert_eq!(
            classify("read_network_requests", None),
            Some(RwClass::Observe)
        );
        assert_eq!(classify("read_page", None), Some(RwClass::Observe));
        assert_eq!(classify("resize_window", None), Some(RwClass::Mutate));
        assert_eq!(classify("update_plan", None), Some(RwClass::Observe));

        assert_eq!(
            classify("computer", Some("left_click")),
            Some(RwClass::Mutate)
        );
        assert_eq!(
            classify("computer", Some("right_click")),
            Some(RwClass::Mutate)
        );
        assert_eq!(classify("computer", Some("type")), Some(RwClass::Mutate));
        assert_eq!(
            classify("computer", Some("screenshot")),
            Some(RwClass::Observe)
        );
        assert_eq!(classify("computer", Some("wait")), Some(RwClass::Observe));
        assert_eq!(classify("computer", Some("scroll")), Some(RwClass::Observe));
        assert_eq!(classify("computer", Some("key")), Some(RwClass::Mutate));
        assert_eq!(
            classify("computer", Some("left_click_drag")),
            Some(RwClass::Mutate)
        );
        assert_eq!(
            classify("computer", Some("double_click")),
            Some(RwClass::Mutate)
        );
        assert_eq!(
            classify("computer", Some("triple_click")),
            Some(RwClass::Mutate)
        );
        assert_eq!(classify("computer", Some("zoom")), Some(RwClass::Observe));
        assert_eq!(
            classify("computer", Some("scroll_to")),
            Some(RwClass::Observe)
        );
        assert_eq!(classify("computer", Some("hover")), Some(RwClass::Observe));
    }

    #[test]
    fn unclassified_inputs_return_none() {
        assert_eq!(classify("no_such_tool", None), None);
        assert_eq!(classify("computer", None), None);
        assert_eq!(classify("computer", Some("no_such_action")), None);
        assert_eq!(
            classify("read_page", Some("left_click")),
            Some(RwClass::Observe),
            "action is ignored for non-computer tools"
        );
    }

    #[test]
    fn rw_class_strings_match_the_audit_vocabulary() {
        assert_eq!(RwClass::Observe.as_str(), "observe");
        assert_eq!(RwClass::Mutate.as_str(), "mutate");
    }
}
