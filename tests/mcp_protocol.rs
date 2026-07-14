// SPDX-License-Identifier: Apache-2.0 OR MIT
//! MCP protocol checks: drive the real `serve_session` chokepoint (initialize, tools/list,
//! tools/call, and the ADR-0049 raw-frame parse/batch rules) and assert the wire behavior.
//!
//! Most tests here connect no extension/native-host, so `tools/call` waits out the bounded
//! handshake window and returns an MCP tool error result (the request/response bridge itself is
//! covered by the `browser` and `ipc` unit tests).
//!
//! ADR-0051 Phase 4 moved these protocol cases onto the in-process
//! `support::inproc::Harness`. Process-boundary late-attach coverage lives in the ADR-0056
//! Lightbox scenario library.

mod support;

use serde_json::{json, Value};
use support::inproc::{manifest_from_value, Harness};

/// Drive `requests` through an all-open in-process session, returning one reply per `id`-bearing
/// request (a notification, no `id` key at all, gets none).
async fn drive(requests: &[Value]) -> Vec<Value> {
    Harness::all_open().drive(requests).await
}

/// Like [`drive`], but optionally under a schema-3 manifest `Value`. `None` is the all-open posture.
async fn drive_with_manifest(manifest: Option<&Value>, requests: &[Value]) -> Vec<Value> {
    let harness = match manifest {
        Some(value) => Harness::governed(manifest_from_value(value)),
        None => Harness::all_open(),
    };
    harness.drive(requests).await
}

/// Send RAW lines verbatim (so a malformed frame or a JSON-RPC array batch can be exercised) and
/// read EXACTLY `expected` responses. The ADR-0049 parse-error / batch rejects reply with
/// `id: null`, so they cannot be counted by id-presence the way [`drive`] does.
async fn drive_raw(lines: &[&str], expected: usize) -> Vec<Value> {
    Harness::all_open().drive_raw(lines, expected).await
}

/// ADR-0049 (as amended by ADR-0050 D3): a JSON-RPC batch (a top-level array of requests) is
/// rejected with -32600 and a teaching message (send one per line; use `browser_batch` for
/// multi-step), not dropped silently.
#[tokio::test]
async fn batch_array_frame_is_rejected_with_a_teaching_message() {
    let batch =
        r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#;
    let responses = drive_raw(&[batch], 1).await;
    let err = &responses[0];
    assert_eq!(err["id"], Value::Null);
    assert_eq!(err["error"]["code"], -32600);
    let msg = err["error"]["message"].as_str().expect("error message");
    assert!(
        msg.contains("one JSON-RPC message per line"),
        "teaches the one-per-line rule: {msg}"
    );
    assert!(
        msg.contains("`browser_batch`"),
        "teaches the browser_batch alternative: {msg}"
    );
}

/// ADR-0049: an unparseable NON-empty line gets an addressable -32700 (id:null); a blank line is a
/// benign keepalive that draws NO response. Sending the blank first proves it is silent -- the sole
/// reply is the malformed line's -32700, not a response to the blank.
#[tokio::test]
async fn parse_error_answers_32700_and_blank_lines_stay_silent() {
    let responses = drive_raw(&["", "{ this is not valid json"], 1).await;
    let err = &responses[0];
    assert_eq!(err["id"], Value::Null);
    assert_eq!(err["error"]["code"], -32700);
}

#[tokio::test]
async fn initialize_tools_list_and_tool_call_over_stdio() {
    let responses = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}), // no response
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        // o04: inputSchema validation now runs before dispatch; navigate needs url + tabId.
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"navigate","arguments":{"url":"https://example.com","tabId":1}}}),
    ])
    .await;

    assert_eq!(
        responses.len(),
        3,
        "expected 3 responses, got {responses:?}"
    );

    let init = &responses[0];
    assert_eq!(init["id"], 1);
    // ADR-0049: with no protocolVersion requested (params:{}), the latest supported is offered.
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(init["result"]["capabilities"]["tools"]["listChanged"], true);
    assert_eq!(init["result"]["serverInfo"]["name"], "ghostlight");

    let list = &responses[1];
    assert_eq!(list["id"], 2);
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(
        tools.len(),
        ghostlight::browser::directory::advertised_tool_count(),
        "the wire advertises the full REGISTRY surface (see directory::advertised_tool_names)"
    );
    assert_eq!(tools[0]["name"], "tabs_context_mcp");
    // The advertised surface must equal the embedded sacred fixture, byte for byte.
    let fixture = ghostlight::mcp::tools::advertised_tools_json();
    assert_eq!(
        list["result"], fixture,
        "tools/list must equal the sacred fixture"
    );

    // No extension is connected, so the tool call waits the bounded window (about 5s), falls
    // through to Browser::call's fail-fast "not connected" path, and returns an MCP tool error
    // result (isError) with the exact hop-attributed message.
    let call = &responses[2];
    assert_eq!(call["id"], 3);
    assert_eq!(call["result"]["isError"], true, "no extension -> isError");
    let text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("error result carries a text block");
    assert!(
        text.starts_with("[hop: extension]"),
        "hop-attributed message: {text}"
    );
    assert_eq!(
        text,
        "[hop: extension] Browser extension not connected. \
         Next step: check chrome://extensions and that Chrome is running.",
        "exact message: {text}"
    );
}

/// ADR-0022 Decision 7: `explain` appears in `tools/list` last (the one sanctioned addition to
/// the sacred surface) and `tools/call explain` returns the directory text without ever needing
/// an extension attached -- proving the tool is handled entirely server-side, with zero
/// native-messaging traffic.
#[tokio::test]
async fn explain_is_advertised_last_and_answers_with_no_extension_attached() {
    let responses = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"explain","arguments":{}}}),
    ])
    .await;
    assert_eq!(responses.len(), 3, "got {responses:?}");

    let list = &responses[1];
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(
        tools.last().expect("at least one tool")["name"],
        "explain",
        "explain must be the last advertised tool"
    );

    let call = &responses[2];
    assert_eq!(call["id"], 3);
    assert_ne!(call["result"]["isError"], true, "explain must never error");
    let text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("text content block");
    assert!(
        text.starts_with("Capabilities: read = "),
        "explain's response opens with the capability vocabulary: {text}"
    );
    assert!(
        text.trim_end().ends_with(
            "explain: requires nothing. Show every action available here and the capability \
             each one requires."
        ),
        "explain's response lists its own row last: {text}"
    );
}

/// Run `explain` under a given manifest posture and return its response text, asserting along the
/// way that it is advertised last and never errors regardless of posture.
async fn explain_text_under_manifest(manifest: Option<&Value>) -> String {
    let responses = drive_with_manifest(
        manifest,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"explain","arguments":{}}}),
        ],
    )
    .await;
    assert_eq!(responses.len(), 3, "got {responses:?}");
    let list = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("tools/list reply");
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(
        tools.last().expect("at least one tool")["name"],
        "explain",
        "explain must be advertised (last) under every posture"
    );
    let call = responses
        .iter()
        .find(|r| r["id"] == 3)
        .expect("explain tools/call reply");
    assert_ne!(
        call["result"]["isError"], true,
        "explain must never error under any posture: {call:?}"
    );
    call["result"]["content"][0]["text"]
        .as_str()
        .expect("explain text content block")
        .to_string()
}

/// ADR-0022 Decision 7 (the map is always the same map): `explain` returns byte-identical output
/// regardless of manifest posture. It requires nothing and is answered server-side before any
/// grant machinery, so a locked-down session sees the identical directory an all-open one does.
/// Pins the actual invariant (same output everywhere), not merely that `explain` is present.
#[tokio::test]
async fn explain_output_is_byte_identical_across_manifest_postures() {
    let open = explain_text_under_manifest(None).await;
    let empty_grants = explain_text_under_manifest(Some(&json!({
        "schema": 3, "name": "empty", "version": "1", "grants": []
    })))
    .await;
    let read_only = explain_text_under_manifest(Some(&json!({
        "schema": 3, "name": "ro", "version": "1",
        "grants": [{"id":"read-only","hosts":{"allow":["example.com"]},"allowed":["read"]}]
    })))
    .await;

    assert!(
        open.starts_with("Capabilities: read = "),
        "sanity: explain opens with the vocabulary block: {open}"
    );
    assert_eq!(
        open, empty_grants,
        "explain output must not change under an empty-grants manifest"
    );
    assert_eq!(
        open, read_only,
        "explain output must not change under a restrictive read-only manifest"
    );
}

#[tokio::test]
async fn unknown_tool_name_is_rejected_before_dispatch() {
    // No extension is ever connected in this test. If the unknown-tool pre-check ran AFTER the
    // bounded extension-channel wait (or not at all), this would instead time out and surface
    // "[hop: extension] Browser extension not connected. ...". Getting the invalid-request hop
    // back proves the pre-check runs first.
    let started = std::time::Instant::now();
    let responses = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"bogus_tool","arguments":{}}}),
    ])
    .await;
    let elapsed = started.elapsed();

    assert_eq!(responses.len(), 2, "got {responses:?}");
    let call = &responses[1];
    assert_eq!(call["id"], 2);
    assert_eq!(call["result"]["isError"], true, "unknown tool -> isError");
    let text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("error result carries a text block");
    assert!(
        text.starts_with("[hop: invalid-request]"),
        "hop-attributed message: {text}"
    );
    assert!(
        text.contains("Unknown tool: bogus_tool"),
        "names the unknown tool: {text}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "the pre-check must return well before the 5s extension-channel wait: {elapsed:?}"
    );
}

#[tokio::test]
async fn malformed_method_and_null_id_follow_jsonrpc_rules() {
    let responses = drive(&[
        json!({"jsonrpc":"2.0","id":7,"params":{}}), // id present, method missing
        json!({"jsonrpc":"2.0","id":null,"method":"ping"}), // legal null-id request
        json!({"method":"notifications/initialized"}), // notification -> no response
    ])
    .await;

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
