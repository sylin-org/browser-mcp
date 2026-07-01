//! End-to-end MCP protocol checks: spawn the binary as an mcp-server and drive it over stdio.
//!
//! No extension/native-host is connected here, so `tools/call` returns an MCP tool error result
//! (the request/response bridge itself is covered by the `browser` and `ipc` unit tests). Each
//! spawned binary gets a unique IPC endpoint so the tests never contend for one.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(0);

/// Spawn the binary (with an isolated IPC endpoint), send each request as a line, close stdin, and
/// collect the response lines.
fn drive(requests: &[Value]) -> Vec<Value> {
    let endpoint = format!(
        "browser-mcp-it-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_browser-mcp"))
        .env("BROWSER_MCP_ENDPOINT", &endpoint)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn browser-mcp");

    let mut stdin = child.stdin.take().expect("stdin");
    for req in requests {
        stdin
            .write_all(serde_json::to_string(req).unwrap().as_bytes())
            .unwrap();
        stdin.write_all(b"\n").unwrap();
    }
    drop(stdin); // EOF -> the server loop ends

    let stdout = child.stdout.take().expect("stdout");
    let responses: Vec<Value> = BufReader::new(stdout)
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).expect("each stdout line is JSON"))
        .collect();
    child.wait().expect("wait for child");
    responses
}

#[test]
fn initialize_tools_list_and_tool_call_over_stdio() {
    let responses = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}), // no response
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"navigate","arguments":{}}}),
    ]);

    assert_eq!(
        responses.len(),
        3,
        "expected 3 responses, got {responses:?}"
    );

    let init = &responses[0];
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init["result"]["serverInfo"]["name"], "browser-mcp");

    let list = &responses[1];
    assert_eq!(list["id"], 2);
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 13, "all 13 tools advertised");
    assert_eq!(tools[0]["name"], "tabs_context_mcp");
    // The advertised surface must equal the embedded sacred fixture, byte for byte.
    let fixture: Value = serde_json::from_str(browser_mcp::mcp::tools::TOOLS_JSON).unwrap();
    assert_eq!(
        list["result"], fixture,
        "tools/list must equal the sacred fixture"
    );

    // No extension is connected, so the tool call returns an MCP tool error result (isError).
    let call = &responses[2];
    assert_eq!(call["id"], 3);
    assert_eq!(call["result"]["isError"], true, "no extension -> isError");
    let text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("error result carries a text block");
    assert!(
        text.contains("not connected"),
        "error explains the extension is unavailable: {text}"
    );
}

#[test]
fn malformed_method_and_null_id_follow_jsonrpc_rules() {
    let responses = drive(&[
        json!({"jsonrpc":"2.0","id":7,"params":{}}), // id present, method missing
        json!({"jsonrpc":"2.0","id":null,"method":"ping"}), // legal null-id request
        json!({"method":"notifications/initialized"}), // notification -> no response
    ]);

    // The notification yields nothing; the other two are addressable.
    assert_eq!(responses.len(), 2, "got {responses:?}");

    // Missing method, but the id is recoverable -> -32600 addressed to id 7.
    assert_eq!(responses[0]["id"], 7);
    assert_eq!(responses[0]["error"]["code"], -32600);

    // id: null is a legal request; the response must echo the id as null (present, not omitted).
    assert!(
        responses[1].as_object().unwrap().contains_key("id"),
        "a null-id request must get an id back, not an omitted field"
    );
    assert_eq!(responses[1]["id"], Value::Null);
}
