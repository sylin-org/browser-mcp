//! JSON-RPC 2.0 server over stdio (the mcp-server role).
//!
//! Reads newline-delimited JSON-RPC from stdin, handles `initialize` / `tools/list` / `tools/call`,
//! and writes responses to stdout (one compact JSON object per line). `tools/call` routes through
//! the [`Governance`] facade (the dispatch chokepoint) and then forwards to the extension via the
//! [`Browser`] handle. stdout is reserved for the protocol stream; operational logs go to stderr.
//!
//! `tools/call` runs concurrently: each call is spawned on its own task (so a slow or waiting call
//! never blocks `initialize`, `ping`, or later requests) and every response -- inline or from a
//! spawned call -- funnels through a single writer task that owns stdout, so lines are never
//! interleaved mid-write.

use crate::browser::{classify, pattern, redact};
use crate::governance::audit::Recorder;
use crate::governance::config::reload::ConfigStore;
use crate::governance::dispatch::Governance;
use crate::governance::ports::AuditSink;
use crate::transport::executor::Browser;
use crate::transport::mcp::tools::{is_known_tool, TOOLS_JSON};
use crate::transport::mcp::types::{text_content, JsonRpcResponse};
use crate::{Result, ToolError};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// MCP protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio MCP server loop until stdin closes. `browser` is the (shared) handle to the
/// extension; tool calls are forwarded through it.
pub async fn run(browser: Browser) -> Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    // Hot-reload substrate (ADR-0019): the resolved Config is held behind an atomic swap; the
    // watcher re-resolves on a config/org/manifest change with no restart. With no files
    // present this resolves to the built-in defaults, so all-open behavior is byte-identical
    // to stage 1.
    let store = ConfigStore::load_initial(pattern::is_valid_pattern)?;
    store.clone().spawn_watcher();

    // The audit flight recorder (ADR-0018 step 1) is orthogonal to the governance mode: it
    // records under all-open too, gated only by audit.enabled (shared format doc section 4.5).
    // Its destination is live (RECONCILIATION.md section 3): a config-change watcher re-opens
    // the sink whenever audit.enabled / audit.destination / audit.file.path changes.
    let recorder = Arc::new(Recorder::from_config(&store.current()));
    tokio::spawn({
        let recorder = Arc::clone(&recorder);
        let mut changes = store.subscribe();
        async move {
            while changes.changed().await.is_ok() {
                let config = changes.borrow().clone();
                recorder.reload(&config);
            }
        }
    });

    let governance = Arc::new(Governance::all_open(
        recorder as Arc<dyn AuditSink>,
        classify::classify,
    ));

    let (tx, mut rx) = mpsc::unbounded_channel::<JsonRpcResponse>();

    // A single writer owns stdout so responses -- including those from spawned `tools/call`
    // tasks -- never interleave mid-write. `debug` is cloned before the spawn so both the
    // writer and the read loop below can record the MCP boundary.
    let debug = browser.debug().clone();
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(resp) = rx.recv().await {
            let mut buf = match serde_json::to_string(&resp) {
                Ok(buf) => buf,
                Err(e) => {
                    tracing::warn!(error = %e, "dropping unserializable response");
                    continue;
                }
            };
            if debug.is_enabled() {
                // Use the already-typed id (do not re-parse the whole -- possibly large -- body).
                let id = resp.id.as_ref().map(Value::to_string).unwrap_or_default();
                debug.mcp_response(&id, &buf);
            }
            buf.push('\n');
            if stdout.write_all(buf.as_bytes()).await.is_err() || stdout.flush().await.is_err() {
                break;
            }
        }
    });

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(resp) = handle_line(&browser, &store, &governance, line, &tx).await {
            let _ = tx.send(resp);
        }
    }
    drop(tx);
    let _ = writer.await;
    Ok(())
}

/// Parse and route one JSON-RPC line.
///
/// Returns `Some(response)` for requests (an `id` member is present, even if `null`) and `None` for
/// notifications (no `id` member) and for lines we cannot parse at all. Fields are read from a raw
/// [`Value`] so a structurally invalid but id-bearing request still gets an addressable `-32600`.
async fn handle_line(
    browser: &Browser,
    store: &Arc<ConfigStore>,
    governance: &Arc<Governance>,
    line: &str,
    tx: &mpsc::UnboundedSender<JsonRpcResponse>,
) -> Option<JsonRpcResponse> {
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
        "initialize" => {
            // Record the MCP client's self-reported identity (clientInfo.name [+ version]), if it
            // sent one, for `browser-mcp doctor`/`status` to display. Missing params/clientInfo, or
            // non-string fields, are silently fine: this is best-effort observability, not part of
            // the protocol contract, and the response below never depends on it.
            if let Some(client_info) = raw.get("params").and_then(|p| p.get("clientInfo")) {
                if let Some(name) = client_info.get("name").and_then(Value::as_str) {
                    let ident = match client_info.get("version").and_then(Value::as_str) {
                        Some(version) => format!("{name} {version}"),
                        None => name.to_string(),
                    };
                    browser.debug().set_client(&ident);
                }
            }
            // Capture the same clientInfo into the audit recorder's client field (shared
            // format doc section 6.1), first-wins for the whole session.
            capture_client_info(governance, raw.get("params"));
            // Warm the extension channel while the client finishes its handshake. The extension
            // side initiates the connection (Chrome spawns the native-host, which dials the
            // endpoint this process has served since startup), so there is nothing to dial from
            // here; this watcher verifies readiness and records the outcome.
            let wait_ms = store.current().first_call_wait_ms();
            tokio::spawn({
                let browser = browser.clone();
                async move {
                    let started = Instant::now();
                    if browser.wait_connected(Duration::from_millis(wait_ms)).await {
                        tracing::info!(
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            "extension channel ready"
                        );
                    } else {
                        tracing::info!(
                            "extension channel not ready within the warmup window; \
                             the first tools/call will wait for it"
                        );
                    }
                }
            });
            Some(JsonRpcResponse::success(id, initialize_result()))
        }
        "tools/list" => Some(JsonRpcResponse::success(id, tools_list_result())),
        "tools/call" => {
            let browser = browser.clone();
            let store = Arc::clone(store);
            let governance = Arc::clone(governance);
            let tx = tx.clone();
            let params = raw.get("params").cloned();
            tokio::spawn(async move {
                let resp =
                    handle_tools_call(&browser, &store, &governance, id, params.as_ref()).await;
                let _ = tx.send(resp);
            });
            None
        }
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

/// Capture `clientInfo` from the MCP `initialize` params into the audit recorder (shared
/// format doc section 6.1 `client` field). Both `name` and `version` must be strings;
/// otherwise the session's records carry `client: null`.
fn capture_client_info(governance: &Governance, params: Option<&Value>) {
    let info = params.and_then(|p| p.get("clientInfo"));
    let name = info.and_then(|i| i.get("name")).and_then(Value::as_str);
    let version = info.and_then(|i| i.get("version")).and_then(Value::as_str);
    if let (Some(name), Some(version)) = (name, version) {
        governance.set_client(name, version);
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
    store: &Arc<ConfigStore>,
    governance: &Governance,
    id: Option<Value>,
    params: Option<&Value>,
) -> JsonRpcResponse {
    // One snapshot for the whole call, taken once at entry: a reload mid-call must not tear
    // the snapshot the call already started with.
    let config = store.current();

    let Some(name) = params.and_then(|p| p.get("name")).and_then(Value::as_str) else {
        return JsonRpcResponse::error(id, -32602, "tools/call requires a string 'name'");
    };
    let args = params
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or(Value::Null);

    // Unknown tool names are rejected before dispatch (and before waiting on the extension
    // channel at all): this is a client-request problem, not a browser/extension problem, and the
    // client should learn that instantly regardless of whether an extension is even connected.
    // The extension keeps its own `Unknown tool: ...` guard as a safety net (defense in depth);
    // this pre-check just means well-formed clients never round-trip to hit it.
    if !is_known_tool(name) {
        let err = ToolError::invalid_request(format!("Unknown tool: {name}"))
            .next_step("call tools/list and use one of the advertised tool names");
        return JsonRpcResponse::success(id, error_result(err));
    }

    // Dispatch chokepoint. The decision seam is a literal STEP-0 short-circuit to Allow under
    // all-open (no manifest, default config) that queries no port and resolves no resource, so
    // behavior is byte-identical to the ungoverned engine; acting on a Deny (enforcement)
    // attaches here in later stage-2 tasks. The audit seam records every call (ADR-0018 step 1)
    // after it resolves, so the record carries the real duration and completion timestamp.
    let dispatch_started = Instant::now();
    let _decision = governance.decide(name);
    // The only tool-call argument ever read for audit purposes: the computer sub-action
    // (shared format doc section 6.2 sensitive-parameter omission; no other argument is read,
    // logged, or stored).
    let action = if name == "computer" {
        args.get("action").and_then(Value::as_str)
    } else {
        None
    };

    // Bounded first-call wait: the first call of a session races the extension handshake.
    // Wait briefly for the channel instead of failing a healthy session (also covers calls
    // arriving during a mid-session reconnect). If the wait times out, `waited` stays `None` and
    // control falls through to `Browser::call` below, which fails fast with the canonical
    // "extension not connected" `ToolError` -- one hop-attributed message, not two to keep in sync.
    let mut waited: Option<Duration> = None;
    if !browser.is_connected() {
        let started = Instant::now();
        if browser
            .wait_connected(Duration::from_millis(config.first_call_wait_ms()))
            .await
        {
            waited = Some(started.elapsed());
        } else {
            tracing::warn!(
                tool = name,
                "tools/call failed: extension channel never came up"
            );
        }
    }

    let outcome = browser.call(name, &args).await;
    let duration_ms = u64::try_from(dispatch_started.elapsed().as_millis()).unwrap_or(u64::MAX);
    governance.record_call(name, action, duration_ms);

    match outcome {
        // The extension returns an MCP result object (`{ content: [...] }`). The engine is truthful:
        // read_page carries secret field values under a `secret_value=` marker; the governance
        // overlay rewrites that marker here (redacting per `content.security.secrets.redact`) before
        // the result leaves the binary. Other tools pass through untouched.
        Ok(mut result) => {
            if name == "read_page" {
                redact::apply_to_result(&mut result, config.secrets_redact());
            }
            if let Some(waited) = waited {
                append_wait_note(&mut result, waited);
            }
            JsonRpcResponse::success(id, result)
        }
        // A tool execution failure is an MCP tool error result (isError), not a JSON-RPC error.
        // The rendered text is exactly the hop-attributed ToolError Display: no "Error: " prefix.
        Err(e) => {
            let mut result = error_result(e);
            if let Some(waited) = waited {
                append_wait_note(&mut result, waited);
            }
            JsonRpcResponse::success(id, result)
        }
    }
}

/// Build an MCP tool error result (`{ content: [...], isError: true }`) from a hop-attributed
/// [`ToolError`]. The result text is exactly the error's `Display`:
/// `[hop: <hop>] <message>. Next step: <next step>.`
fn error_result(err: ToolError) -> Value {
    let mut result = text_content(err.to_string());
    if let Some(obj) = result.as_object_mut() {
        obj.insert("isError".into(), json!(true));
    }
    result
}

/// Append the truthful handshake-wait note as a final text block on an MCP tool result.
fn append_wait_note(result: &mut Value, waited: Duration) {
    let note = format!(
        "(waited {:.1}s for browser extension handshake)",
        waited.as_secs_f64()
    );
    if let Some(content) = result.get_mut("content").and_then(Value::as_array_mut) {
        content.push(json!({ "type": "text", "text": note }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::config::Config;

    fn temp_audit_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "browser-mcp-server-audit-test-{}-{tag}.jsonl",
            std::process::id()
        ))
    }

    fn read_lines(path: &std::path::Path) -> Vec<Value> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        content
            .lines()
            .map(|l| serde_json::from_str(l).expect("each line is a JSON object"))
            .collect()
    }

    fn assert_wellformed_event_id_and_ts(rec: &Value) {
        let event_id = rec["event_id"].as_str().expect("event_id is a string");
        assert_eq!(event_id.len(), 36, "event_id: {event_id}");
        for offset in [8, 13, 18, 23] {
            assert_eq!(event_id.as_bytes()[offset], b'-', "event_id: {event_id}");
        }
        let ts = rec["ts"].as_str().expect("ts is a string");
        assert_eq!(ts.len(), 24, "ts: {ts}");
        assert!(ts.ends_with('Z'), "ts: {ts}");
        chrono::DateTime::parse_from_rfc3339(ts).expect("ts parses as rfc3339");
    }

    /// Test 10 (g06 spec section 6, adapted to the post-A3/A5 architecture): drives the real
    /// `handle_line` dispatch for `initialize` (proving `capture_client_info` is wired at the
    /// real chokepoint, not just callable in isolation) and `handle_tools_call` for a
    /// `navigate` call, then asserts the resulting audit line end to end.
    #[tokio::test]
    async fn tools_call_produces_one_audit_record_with_client_identity() {
        let path = temp_audit_path("basic");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();
        let (tx, _rx) = mpsc::unbounded_channel::<JsonRpcResponse>();

        let init_line = json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "clientInfo": { "name": "test-client", "version": "9.9.9" } },
        })
        .to_string();
        handle_line(&browser, &store, &governance, &init_line, &tx).await;

        let params = json!({ "name": "navigate", "arguments": {} });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(2)), Some(&params)).await;
        let text = resp.result.as_ref().expect("tool result present")["content"][0]["text"]
            .as_str()
            .expect("text content block")
            .to_string();
        assert!(text.contains("not connected"), "unexpected text: {text}");

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 1, "exactly one audit record");
        let rec = &lines[0];
        assert_eq!(rec["tool"], "navigate");
        assert!(rec["action"].is_null());
        assert_eq!(rec["rw"], "mutate");
        assert_eq!(rec["decision"], "allow");
        assert_eq!(rec["client"]["name"], "test-client");
        assert_eq!(rec["client"]["version"], "9.9.9");
        for field in ["identity", "domain", "grant_id", "denial_id", "manifest"] {
            assert!(rec[field].is_null(), "{field} must be null");
        }
        assert_wellformed_event_id_and_ts(rec);

        std::fs::remove_file(&path).ok();
    }

    /// Test 11: a `computer` call with `action: "screenshot"` records that action and the
    /// observe class.
    #[tokio::test]
    async fn computer_call_records_action_and_observe_class() {
        let path = temp_audit_path("computer");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();

        let params = json!({ "name": "computer", "arguments": { "action": "screenshot" } });
        let _ =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 1, "exactly one audit record");
        assert_eq!(lines[0]["action"], "screenshot");
        assert_eq!(lines[0]["rw"], "observe");

        std::fs::remove_file(&path).ok();
    }

    /// Test 12: a `tools/call` whose params lack `name` returns the `-32602` error and never
    /// reaches the dispatch chokepoint, so no audit file is created.
    #[tokio::test]
    async fn invalid_tools_call_without_name_records_nothing() {
        let path = temp_audit_path("no-name");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();

        let params = json!({ "arguments": {} });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;
        assert_eq!(resp.error.as_ref().expect("error present")["code"], -32602);
        assert!(!path.exists(), "no audit file must be created");
    }
}
