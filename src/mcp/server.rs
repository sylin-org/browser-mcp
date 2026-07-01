//! JSON-RPC 2.0 server over stdio (the mcp-server role).
//!
//! Reads newline-delimited JSON-RPC from stdin, handles `initialize` / `tools/list` / `tools/call`,
//! and writes responses to stdout (one compact JSON object per line). `tools/call` routes through
//! [`crate::dispatch`] (the v1.0 no-op policy/audit seams) and then forwards to the extension via
//! the [`Browser`] handle. stdout is reserved for the protocol stream; operational logs go to stderr.

use crate::browser::Browser;
use crate::dispatch;
use crate::mcp::tools::TOOLS_JSON;
use crate::mcp::types::{text_content, JsonRpcResponse};
use crate::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// MCP protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio MCP server loop until stdin closes. `browser` is the (shared) handle to the
/// extension; tool calls are forwarded through it.
pub async fn run(browser: Browser) -> Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(resp) = handle_line(&browser, line).await {
            let mut buf = serde_json::to_string(&resp)?;
            if browser.debug().is_enabled() {
                // Use the already-typed id (do not re-parse the whole -- possibly large -- body).
                let id = resp.id.as_ref().map(Value::to_string).unwrap_or_default();
                browser.debug().mcp_response(&id, &buf);
            }
            buf.push('\n');
            stdout.write_all(buf.as_bytes()).await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

/// Parse and route one JSON-RPC line.
///
/// Returns `Some(response)` for requests (an `id` member is present, even if `null`) and `None` for
/// notifications (no `id` member) and for lines we cannot parse at all. Fields are read from a raw
/// [`Value`] so a structurally invalid but id-bearing request still gets an addressable `-32600`.
async fn handle_line(browser: &Browser, line: &str) -> Option<JsonRpcResponse> {
    let raw: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "dropping unparseable JSON-RPC line");
            return None;
        }
    };

    let is_notification = raw.get("id").is_none();
    let id = raw.get("id").cloned();

    let Some(method) = raw.get("method").and_then(Value::as_str) else {
        return if is_notification {
            tracing::debug!("dropping malformed notification (no method)");
            None
        } else {
            Some(JsonRpcResponse::error(
                id,
                -32600,
                "Invalid Request: missing or non-string 'method'",
            ))
        };
    };

    if browser.debug().is_enabled() {
        let id_str = id.as_ref().map(Value::to_string).unwrap_or_default();
        browser.debug().mcp_request(method, &id_str, line);
    }

    match method {
        "initialize" => Some(JsonRpcResponse::success(id, initialize_result())),
        "tools/list" => Some(JsonRpcResponse::success(id, tools_list_result())),
        "tools/call" => Some(handle_tools_call(browser, id, raw.get("params")).await),
        "ping" => Some(JsonRpcResponse::success(id, json!({}))),
        _ if is_notification => {
            tracing::debug!(method, "ignoring unknown notification");
            None
        }
        other => Some(JsonRpcResponse::error(
            id,
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

async fn handle_tools_call(
    browser: &Browser,
    id: Option<Value>,
    params: Option<&Value>,
) -> JsonRpcResponse {
    let Some(name) = params.and_then(|p| p.get("name")).and_then(Value::as_str) else {
        return JsonRpcResponse::error(id, -32602, "tools/call requires a string 'name'");
    };
    let args = params
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or(Value::Null);

    // v1.0 engine: the policy and audit seams are no-ops (all-open). The v1.5 overlay slots in here
    // without touching this code (see src/dispatch.rs).
    let _decision = dispatch::policy_check(name);
    dispatch::audit(name);

    match browser.call(name, &args).await {
        // The extension returns an MCP result object (`{ content: [...] }`); pass it through.
        Ok(result) => JsonRpcResponse::success(id, result),
        // A tool execution failure is an MCP tool error result (isError), not a JSON-RPC error.
        Err(e) => {
            let mut result = text_content(format!("Error: {e}"));
            if let Some(obj) = result.as_object_mut() {
                obj.insert("isError".into(), json!(true));
            }
            JsonRpcResponse::success(id, result)
        }
    }
}
