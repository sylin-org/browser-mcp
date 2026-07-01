//! The binary <-> extension wire protocol (reference documentation).
//!
//! Both directions carry UTF-8 JSON, one object per native message (Chrome frames each with a
//! 4-byte little-endian length prefix; see [`super::host`]). The native-host relays these objects
//! verbatim; only the mcp-server (in [`crate::browser`]) constructs and parses them, so they are
//! documented here rather than modeled as types.
//!
//! ## binary -> extension
//! ```json
//! { "id": "<string>", "type": "tool_request", "tool": "<tool name>", "args": { ... } }
//! ```
//!
//! ## extension -> binary
//! ```json
//! { "id": "<string>", "type": "tool_response", "result": { "content": [ ... ] } }
//! { "id": "<string>", "type": "tool_error",    "error":  "<message>" }
//! ```
//!
//! `result` is an MCP tool result object. Replies without an `id` (events, heartbeats) are ignored
//! by the mcp-server in v1.0; Phase 3 will buffer console/network events pushed this way.
