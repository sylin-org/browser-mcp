//! End-to-end MCP protocol check: spawn the binary as an mcp-server and drive `initialize` +
//! `tools/list` + a stub `tools/call` over stdio, asserting the advertised surface equals the
//! sacred fixture and notifications get no response.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn initialize_tools_list_and_stub_call_over_stdio() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_browser-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn browser-mcp");

    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        // a notification -- must produce no response
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"navigate","arguments":{}}}),
    ];

    // Write every request, then close stdin so the server loop ends on EOF.
    let mut stdin = child.stdin.take().expect("stdin");
    for req in &requests {
        let line = serde_json::to_string(req).unwrap();
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
    }
    drop(stdin);

    let stdout = child.stdout.take().expect("stdout");
    let responses: Vec<Value> = BufReader::new(stdout)
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).expect("each stdout line is JSON"))
        .collect();
    child.wait().expect("wait for child");

    // Three requests carry an id; the notification does not -> exactly three responses, in order.
    assert_eq!(responses.len(), 3, "expected 3 responses, got {responses:?}");

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
    assert_eq!(list["result"], fixture, "tools/list must equal the sacred fixture");

    let call = &responses[2];
    assert_eq!(call["id"], 3);
    let text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("stub call returns a text block");
    assert!(text.contains("navigate"), "stub confirms the tool name: {text}");
}
