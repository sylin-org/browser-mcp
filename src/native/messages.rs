//! Command/response envelope exchanged with the extension.
//!
//! Mirrors the reference's message shapes: `tool_request {id, tool, args}` ->
//! `tool_response {id, result}` / `tool_error {id, error}`. Minimal stub; expanded in Phase 1.

use serde::{Deserialize, Serialize};

/// A tool invocation dispatched to the extension (binary -> extension).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub tool: String,
    pub args: serde_json::Value,
}
