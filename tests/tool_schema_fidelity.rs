//! Fidelity guard for the sacred `tools/list` surface (`src/mcp/schemas/tools.json`).
//!
//! Ensures the embedded schema fixture stays intact: exactly the 13 preserved tools, in order,
//! each with a non-empty description and an object inputSchema. Once `tools/list` is implemented
//! (Phase 1), this is extended to byte-compare the emitted output against the fixture.

use browser_mcp::mcp::tools::TOOLS_JSON;
use serde_json::Value;

/// The exact advertised surface, in order. Changing this array is changing the sacred contract.
const EXPECTED: [&str; 13] = [
    "tabs_context_mcp",
    "tabs_create_mcp",
    "navigate",
    "computer",
    "find",
    "form_input",
    "get_page_text",
    "javascript_tool",
    "read_console_messages",
    "read_network_requests",
    "read_page",
    "resize_window",
    "update_plan",
];

fn tools() -> Vec<Value> {
    let v: Value = serde_json::from_str(TOOLS_JSON).expect("tools.json must be valid JSON");
    v["tools"]
        .as_array()
        .expect("`tools` must be an array")
        .clone()
}

#[test]
fn advertises_exactly_the_thirteen_preserved_tools_in_order() {
    let names: Vec<String> = tools()
        .iter()
        .map(|t| {
            t["name"]
                .as_str()
                .expect("name must be a string")
                .to_string()
        })
        .collect();
    assert_eq!(
        names, EXPECTED,
        "the advertised tool set/order must match the sacred surface"
    );
}

#[test]
fn every_tool_is_well_formed() {
    for t in tools() {
        let name = t["name"].as_str().expect("name");
        assert!(!name.is_empty(), "tool name must be non-empty");
        assert!(
            t["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{name}: description must be a non-empty string"
        );
        assert_eq!(
            t["inputSchema"]["type"].as_str(),
            Some("object"),
            "{name}: inputSchema.type must be \"object\""
        );
    }
}

#[test]
fn computer_advertises_all_thirteen_actions() {
    let computer = tools()
        .into_iter()
        .find(|t| t["name"] == "computer")
        .expect("computer tool must exist");
    let actions = computer["inputSchema"]["properties"]["action"]["enum"]
        .as_array()
        .expect("computer.action must have an enum");
    assert_eq!(
        actions.len(),
        13,
        "computer must advertise all 13 actions (was {})",
        actions.len()
    );
}
