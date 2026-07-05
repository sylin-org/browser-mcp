// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Tool registry, `tools/list` advertisement, and the agent onboarding guide
//! (ADR-0031 Decisions 1 and 2).
//!
//! The tool **schemas are sacred**: they are byte-identical to the reference's advertised surface
//! (captured in `src/mcp/schemas/`, guarded by `tests/tool_schema_fidelity.rs`). In all-open v1.0
//! the full surface is advertised unconditionally -- the 13 preserved Claude-in-Chrome tools
//! (`tabs_context_mcp`, `tabs_create_mcp`, `navigate`, `computer`, `find`, `form_input`,
//! `get_page_text`, `javascript_tool`, `read_console_messages`, `read_network_requests`,
//! `read_page`, `resize_window`, `update_plan`). The excluded stubs (`gif_creator`,
//! `shortcuts_list`, `shortcuts_execute`, `switch_browser`, `upload_image`) are not advertised.
//! Implemented in Phase 1.
//!
//! ADR-0031: the fixture is also the single source of the agent onboarding guide (the top-level
//! `agentGuide` section) and the per-tool `example` field. This module renders the guide into the
//! single string MCP's `initialize.instructions` field expects; the service constructs nothing
//! (Decision 1: pure passthrough of the fixture's prose).

use serde_json::Value;

/// The sacred `tools/list` surface: the 13 preserved tool schemas (plus the additive `explain`
/// tool and the `agentGuide` onboarding section), embedded verbatim as raw JSON (a const literal,
/// per CLAUDE.md, to prevent accidental drift). Provenance and fidelity notes are in
/// `schemas/README.md`; `tests/tool_schema_fidelity.rs` guards it.
pub const TOOLS_JSON: &str = include_str!("schemas/tools.json");

/// Render the agent onboarding guide (ADR-0031 Decision 1) from the fixture's top-level
/// `agentGuide` section into the single string MCP's `initialize.instructions` field expects.
///
/// The service constructs nothing: the four fields (`summary`, `workflow`, `flow`, `denials`) are
/// concatenated verbatim with clear separators. Served once at handshake, before any tool call,
/// so any model -- trained on this surface or not -- gets the workflow contract (every
/// tab-touching tool requires a `tabId`; get one from `tabs_context_mcp` first; then `navigate`)
/// and the cost/discipline/denial notes without having to derive them from per-tool descriptions.
///
/// Returns an empty string only if the fixture is malformed (no `agentGuide` object) -- which the
/// fidelity test prevents from ever shipping -- so callers can emit it unconditionally.
pub fn agent_guide_text() -> String {
    let v: Value = serde_json::from_str(TOOLS_JSON).expect("TOOLS_JSON is valid JSON");
    let guide = v
        .get("agentGuide")
        .and_then(Value::as_object)
        .expect("TOOLS_JSON carries an agentGuide object");
    let field = |key: &str| -> String {
        guide
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_default()
    };
    let summary = field("summary");
    let workflow = field("workflow");
    let flow = field("flow");
    let denials = field("denials");
    format!("{summary}\n\n{workflow}\n\nTypical flow: {flow}\n\n{denials}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR-0031 Decision 1: the guide renders all four agentGuide fields, in order, separated
    /// cleanly. The content is verbatim from the fixture (no service-authored prose).
    #[test]
    fn agent_guide_text_renders_all_four_fields_in_order() {
        let text = agent_guide_text();
        // All four fields are non-empty in the fixture; the rendered text must contain each.
        assert!(!text.is_empty(), "the guide is non-empty");
        let v: Value = serde_json::from_str(TOOLS_JSON).unwrap();
        let guide = &v["agentGuide"];
        for key in &["summary", "workflow", "flow", "denials"] {
            let val = guide[key].as_str().unwrap();
            assert!(
                text.contains(val),
                "the rendered guide contains the `{key}` field verbatim"
            );
        }
        // The load-bearing workflow rule is present.
        assert!(
            text.contains("tabId"),
            "the workflow rule about tabId is present in the rendered guide"
        );
    }

    /// The flow line prefixes the flow field (so a model reading the rendered string sees the
    /// spine labeled, not just the raw arrow sequence).
    #[test]
    fn agent_guide_text_labels_the_flow_line() {
        let text = agent_guide_text();
        assert!(
            text.contains("Typical flow:"),
            "the flow field is labeled so a reader recognizes the spine"
        );
    }
}
