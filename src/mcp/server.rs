//! JSON-RPC 2.0 server over stdio (the mcp-server role).
//!
//! Reads newline-delimited JSON-RPC from stdin, handles `initialize` / `tools/list` / `tools/call`,
//! and writes responses to stdout (one compact JSON object per line). `tools/call` routes through
//! [`crate::dispatch`] (the v1.0 no-op policy/audit seams). In v1.0 there is no extension wired yet,
//! so `tools/call` returns a stub confirmation -- real execution lands in Phase 2. stdout is
//! reserved for the protocol stream; operational logs go to stderr.

use crate::dispatch;
use crate::mcp::tools::TOOLS_JSON;
use crate::mcp::types::{text_content, JsonRpcRequest, JsonRpcResponse};
use crate::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// MCP protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio MCP server loop until stdin closes.
pub async fn run() -> Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<JsonRpcRequest>(line) {
            Ok(req) => handle(&req),
            Err(e) => {
                // The id is unknown, so per JSON-RPC we cannot address a response; drop and log.
                tracing::warn!(error = %e, "dropping unparseable JSON-RPC line");
                None
            }
        };
        if let Some(resp) = response {
            let mut buf = serde_json::to_string(&resp)?;
            buf.push('\n');
            stdout.write_all(buf.as_bytes()).await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

/// Dispatch one request. Returns `None` for notifications (no `id`), which get no response.
fn handle(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => Some(JsonRpcResponse::success(req.id.clone(), initialize_result())),
        "tools/list" => Some(JsonRpcResponse::success(req.id.clone(), tools_list_result())),
        "tools/call" => Some(handle_tools_call(req)),
        "ping" => Some(JsonRpcResponse::success(req.id.clone(), json!({}))),
        method if method.starts_with("notifications/") => None,
        _ if req.is_notification() => None,
        other => Some(JsonRpcResponse::error(
            req.id.clone(),
            -32601,
            format!("Method not found: {other}"),
        )),
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "browser-mcp", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The advertised surface: the embedded sacred fixture (`{ "tools": [...] }`) verbatim. In all-open
/// v1.0 the full surface is advertised unconditionally -- there is no overlay to filter it.
fn tools_list_result() -> Value {
    serde_json::from_str(TOOLS_JSON).expect("embedded tools.json is valid")
}

fn handle_tools_call(req: &JsonRpcRequest) -> JsonRpcResponse {
    let Some(name) = req.params.get("name").and_then(Value::as_str) else {
        return JsonRpcResponse::error(
            req.id.clone(),
            -32602,
            "tools/call requires a string 'name'",
        );
    };
    // v1.0 engine: the policy and audit seams are no-ops (all-open). The v1.5 overlay slots in here
    // without touching this code (see src/dispatch.rs).
    let _decision = dispatch::policy_check(name);
    dispatch::audit(name);
    // Phase 1: no extension is wired yet, so we acknowledge without executing.
    let result = text_content(format!(
        "[stub] tool '{name}' accepted by the v1.0 engine; extension execution lands in Phase 2."
    ));
    JsonRpcResponse::success(req.id.clone(), result)
}
