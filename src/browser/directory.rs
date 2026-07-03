//! The action directory of ADR-0022 Decision 2: a per-action bound capability requirement
//! set plus a curt, agent-targeted description, compiled in as static browser-domain data.
//! This is additive alongside [`crate::browser::classify`], which remains the enforcement
//! and audit authority until the s05/s06 switch moves consumers over to this table.
//!
//! Absent-vs-empty invariant (ADR-0022 Decision 2): [`requires`] returning `None` is a
//! classification MISS -- the action has no directory entry, and callers must deny it (fail
//! closed). `Some(&[])` means the action's bound requirement set is empty -- it is
//! unconditionally allowed, no resource resolution or grant scan needed. The two states are
//! never to be conflated: `None` and `Some(&[])` are distinct outcomes with opposite
//! consequences.
//!
//! The module is pure: no I/O, no allocation beyond what slice iteration needs, no
//! dependencies beyond `core`/`std`.

use crate::governance::ports::Capability;

/// One row of the action directory: an action's bound capability requirement set and its
/// agent-targeted description.
#[derive(Debug, Clone, Copy)]
pub struct ActionDescriptor {
    pub tool: &'static str,
    pub action: Option<&'static str>,
    pub requires: &'static [Capability],
    pub description: &'static str,
}

/// The action directory (ADR-0022 Decision 2): 12 tools + 13 `computer` sub-actions = 25
/// rows, in tools.json advertised order with `computer` expanded in place into its 13
/// action rows in tools.json `action` enum order. The `explain` tool's row is added by s07.
pub const DIRECTORY: &[ActionDescriptor] = &[
    ActionDescriptor {
        tool: "tabs_context_mcp",
        action: None,
        requires: &[Capability::Read],
        description:
            "List the MCP tab group: the ids, URLs, and titles of the tabs this server controls.",
    },
    ActionDescriptor {
        tool: "tabs_create_mcp",
        action: None,
        requires: &[],
        description: "Open a new empty tab in the MCP tab group; touches no page and no server.",
    },
    ActionDescriptor {
        tool: "navigate",
        action: None,
        requires: &[Capability::Read],
        description: "Load a URL in a tab, or go back or forward in its history; a top-level GET.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("left_click"),
        requires: &[Capability::Action],
        description:
            "Left-click at coordinates; commits an activation whose effect the page decides.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("right_click"),
        requires: &[Capability::Action],
        description: "Right-click at coordinates; commits an activation.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("type"),
        requires: &[Capability::Action],
        description: "Type text into the focused element; commits data to page handlers.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("screenshot"),
        requires: &[Capability::Read],
        description: "Capture a screenshot of the visible viewport.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("wait"),
        requires: &[],
        description: "Pause for a duration; touches no page and no server.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("scroll"),
        requires: &[Capability::Read],
        description: "Scroll the viewport; moves the view without committing input to the page.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("key"),
        requires: &[Capability::Action],
        description: "Press a key or key combination; commits input to page handlers.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("left_click_drag"),
        requires: &[Capability::Action],
        description: "Click and drag between two points; commits pointer input to the page.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("double_click"),
        requires: &[Capability::Action],
        description: "Double-click at coordinates; commits an activation.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("triple_click"),
        requires: &[Capability::Action],
        description: "Triple-click at coordinates; commits an activation.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("zoom"),
        requires: &[Capability::Read],
        description: "Capture a zoomed screenshot of a page region.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("scroll_to"),
        requires: &[Capability::Read],
        description: "Scroll an element into view; moves the viewport without committing input.",
    },
    ActionDescriptor {
        tool: "computer",
        action: Some("hover"),
        requires: &[Capability::Read],
        description: "Move the pointer over a point; commits no activation and no data.",
    },
    ActionDescriptor {
        tool: "find",
        action: None,
        requires: &[Capability::Read],
        description: "Search the page for elements matching a natural-language description.",
    },
    ActionDescriptor {
        tool: "form_input",
        action: None,
        requires: &[Capability::Write],
        description: "Fill or set values in form fields; a declared, state-changing write.",
    },
    ActionDescriptor {
        tool: "get_page_text",
        action: None,
        requires: &[Capability::Read],
        description: "Extract the page's readable text content, article-first, without HTML.",
    },
    ActionDescriptor {
        tool: "javascript_tool",
        action: None,
        requires: &[Capability::Execute],
        description:
            "Run arbitrary JavaScript in the page; unbounded, and can bypass the UI entirely.",
    },
    ActionDescriptor {
        tool: "read_console_messages",
        action: None,
        requires: &[Capability::Read],
        description: "Read buffered browser console messages from a tab.",
    },
    ActionDescriptor {
        tool: "read_network_requests",
        action: None,
        requires: &[Capability::Read],
        description: "Read buffered HTTP network requests observed in a tab.",
    },
    ActionDescriptor {
        tool: "read_page",
        action: None,
        requires: &[Capability::Read],
        description: "Read the page as an accessibility tree of elements with reference ids.",
    },
    ActionDescriptor {
        tool: "resize_window",
        action: None,
        requires: &[],
        description: "Resize the browser window; browser state only, touches no page content.",
    },
    ActionDescriptor {
        tool: "update_plan",
        action: None,
        requires: &[],
        description: "Present a plan of intended actions to the user; informational only.",
    },
];

/// Look up the bound capability requirement set for one action. `action` is consulted only
/// when `tool` is `"computer"`; for every other tool it is ignored.
///
/// Returns `None` when the (tool, action) pair has no directory entry -- a classification
/// MISS, which callers must treat as a denial (fail closed), never as "no requirements".
/// Returns `Some(&[])` when the action's bound requirement set is genuinely empty -- the
/// action is unconditionally allowed. See the module doc comment for the absent-vs-empty
/// invariant (ADR-0022 Decision 2).
pub fn requires(tool: &str, action: Option<&str>) -> Option<&'static [Capability]> {
    if tool == "computer" {
        let action = action?;
        return DIRECTORY
            .iter()
            .find(|row| row.tool == "computer" && row.action == Some(action))
            .map(|row| row.requires);
    }
    DIRECTORY
        .iter()
        .find(|row| row.tool == tool && row.action.is_none())
        .map(|row| row.requires)
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
    fn directory_covers_the_sacred_surface_exactly() {
        let sacred = sacred_tool_names();
        let mut expected_non_computer: HashSet<String> = sacred.clone();
        expected_non_computer.remove("computer");

        let none_action_tool_names: HashSet<String> = DIRECTORY
            .iter()
            .filter(|row| row.action.is_none())
            .map(|row| row.tool.to_string())
            .collect();
        assert_eq!(
            none_action_tool_names, expected_non_computer,
            "no gaps, no stale entries among action:None rows"
        );

        assert!(
            !DIRECTORY
                .iter()
                .any(|row| row.action.is_none() && row.tool == "computer"),
            "no action:None row may have tool computer"
        );
        assert!(
            DIRECTORY
                .iter()
                .filter(|row| row.action.is_some())
                .all(|row| row.tool == "computer"),
            "every row with a Some action must have tool computer"
        );

        let sacred_actions = sacred_computer_actions();
        let table_actions: HashSet<String> = DIRECTORY
            .iter()
            .filter_map(|row| row.action.map(|a| a.to_string()))
            .collect();
        assert_eq!(table_actions, sacred_actions);
        assert_eq!(table_actions.len(), 13);

        assert_eq!(DIRECTORY.len(), 25);

        let mut seen = HashSet::new();
        for row in DIRECTORY {
            assert!(
                seen.insert((row.tool, row.action)),
                "duplicate row: {row:?}"
            );
        }
    }

    #[test]
    fn directory_requires_match_the_adr_table() {
        const EXPECTED: &[(&str, Option<&str>, &[Capability])] = &[
            ("tabs_context_mcp", None, &[Capability::Read]),
            ("tabs_create_mcp", None, &[]),
            ("navigate", None, &[Capability::Read]),
            ("computer", Some("left_click"), &[Capability::Action]),
            ("computer", Some("right_click"), &[Capability::Action]),
            ("computer", Some("type"), &[Capability::Action]),
            ("computer", Some("screenshot"), &[Capability::Read]),
            ("computer", Some("wait"), &[]),
            ("computer", Some("scroll"), &[Capability::Read]),
            ("computer", Some("key"), &[Capability::Action]),
            ("computer", Some("left_click_drag"), &[Capability::Action]),
            ("computer", Some("double_click"), &[Capability::Action]),
            ("computer", Some("triple_click"), &[Capability::Action]),
            ("computer", Some("zoom"), &[Capability::Read]),
            ("computer", Some("scroll_to"), &[Capability::Read]),
            ("computer", Some("hover"), &[Capability::Read]),
            ("find", None, &[Capability::Read]),
            ("form_input", None, &[Capability::Write]),
            ("get_page_text", None, &[Capability::Read]),
            ("javascript_tool", None, &[Capability::Execute]),
            ("read_console_messages", None, &[Capability::Read]),
            ("read_network_requests", None, &[Capability::Read]),
            ("read_page", None, &[Capability::Read]),
            ("resize_window", None, &[]),
            ("update_plan", None, &[]),
        ];

        assert_eq!(DIRECTORY.len(), EXPECTED.len());
        for (row, expected) in DIRECTORY.iter().zip(EXPECTED.iter()) {
            assert_eq!(
                (row.tool, row.action, row.requires),
                *expected,
                "row order/content mismatch"
            );
        }
    }

    #[test]
    fn absent_is_none_and_empty_is_some() {
        assert_eq!(requires("no_such_tool", None), None);
        assert_eq!(requires("computer", None), None);
        assert_eq!(requires("computer", Some("no_such_action")), None);
        assert_eq!(requires("tabs_create_mcp", None), Some(&[][..]));
        assert_eq!(requires("update_plan", None), Some(&[][..]));
        assert_eq!(requires("computer", Some("wait")), Some(&[][..]));
        assert_eq!(requires("navigate", None), Some(&[Capability::Read][..]));
        assert_eq!(
            requires("javascript_tool", None),
            Some(&[Capability::Execute][..])
        );
        assert_eq!(requires("form_input", None), Some(&[Capability::Write][..]));
        assert_eq!(
            requires("computer", Some("left_click")),
            Some(&[Capability::Action][..])
        );
        assert_eq!(
            requires("read_page", Some("left_click")),
            Some(&[Capability::Read][..]),
            "action is ignored for non-computer tools"
        );
    }

    #[test]
    fn every_description_is_nonempty_ascii_and_short() {
        for row in DIRECTORY {
            assert!(!row.description.is_empty(), "empty description: {row:?}");
            assert!(row.description.is_ascii(), "non-ascii description: {row:?}");
            assert!(
                row.description.len() <= 90,
                "description too long ({} chars): {row:?}",
                row.description.len()
            );
            assert_eq!(
                row.description,
                row.description.trim(),
                "description has leading/trailing whitespace: {row:?}"
            );
        }
    }
}
