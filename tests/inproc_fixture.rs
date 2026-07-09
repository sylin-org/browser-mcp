// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Self-test + worked example for the in-process session fixture (ADR-0051 Phase 4, P4.1;
//! `support::inproc`). Proves the fixture drives the REAL `serve_session` chokepoint -- governance
//! decide, tool advertisement, dispatch, and a fake-extension round trip -- over an in-memory
//! duplex with NO spawned process, so the P4.2 migrations that follow have a verified seam to build
//! on. Each assertion mirrors one a spawn-based test already makes (named inline), demonstrating
//! that migrating onto the fixture changes HOW a test reaches the code, never WHAT it proves.

mod support;

use serde_json::json;
use support::inproc::{by_id, init_and_call, manifest_from_value, text_of, Harness};

/// The all-open `tools/list` surface reaches the wire byte-identically to the code-declared
/// fixture, and advertises exactly `advertised_tool_count()` tools -- the same invariant
/// `tests/tool_enforcement.rs::all_open_invariant_no_manifest_means_no_denials` proves over a
/// spawned service.
#[tokio::test]
async fn all_open_tools_list_is_byte_identical_to_the_fixture() {
    let harness = Harness::all_open();
    let responses = harness
        .drive(&[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        ])
        .await;

    let list = by_id(&responses, 2);
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(
        tools.len(),
        ghostlight::browser::directory::advertised_tool_count(),
        "the wire advertises the full REGISTRY surface"
    );
    assert_eq!(
        list["result"],
        ghostlight::mcp::tools::advertised_tools_json(),
        "byte-identical tools/list"
    );
}

/// Under all-open, a `tools/call` with no extension connected passes policy, reaches dispatch, and
/// returns the familiar `not connected` execution error -- never a `Denied (` text. The "reaches
/// dispatch" contrast that `tests/tool_enforcement.rs` is built around.
#[tokio::test]
async fn all_open_call_reaches_dispatch_without_an_extension() {
    let harness = Harness::all_open();
    let responses = harness
        .drive(&init_and_call(
            "navigate",
            json!({"url":"https://example.com/","tabId":1}),
        ))
        .await;

    let call = by_id(&responses, 2);
    assert_eq!(call["result"]["isError"], true, "no extension -> isError");
    let text = text_of(call);
    assert!(text.contains("not connected"), "reached dispatch: {text}");
    assert!(!text.starts_with("Denied ("), "no denial under all-open: {text}");
}

/// A governed manifest whose grants do not cover the target domain denies the call before dispatch,
/// naming the uncovered host -- the same signal as
/// `tool_enforcement::permitted_call_passes_and_denied_domain_is_denied_with_matching_audit`, now
/// entirely in-process.
#[tokio::test]
async fn governed_denies_an_uncovered_domain_before_dispatch() {
    let manifest = manifest_from_value(&json!({
        "schema": 3,
        "name": "inproc-denial",
        "version": "1",
        "grants": [
            { "id": "example-full", "hosts": {"allow": ["example.com"]}, "allowed": ["read", "action", "write"] }
        ],
    }));
    let harness = Harness::governed(manifest);
    let responses = harness
        .drive(&init_and_call(
            "navigate",
            json!({"url":"https://evil.com/","tabId":1}),
        ))
        .await;

    let denied = by_id(&responses, 2);
    assert_ne!(denied["result"]["isError"], true, "a denial is not isError");
    let text = text_of(denied);
    assert!(text.starts_with("Denied (D-"), "{text}");
    assert!(text.contains("no grant covers evil.com"), "{text}");
}

/// With a fake extension attached, a dispatched call reaches it and comes back with the extension's
/// reply instead of `not connected` -- proving the `Browser`-over-duplex leg of the fixture is
/// wired (the `tests/hub_multiplex.rs` seam, now reusable).
#[tokio::test]
async fn attached_extension_answers_a_dispatched_call() {
    let harness = Harness::all_open();
    harness
        .attach_fake_extension(|_req| {
            json!({ "content": [ { "type": "text", "text": "extension answered" } ] })
        })
        .await;

    let responses = harness
        .drive(&init_and_call(
            "navigate",
            json!({"url":"https://example.com/","tabId":1}),
        ))
        .await;

    let call = by_id(&responses, 2);
    let text = call["result"]["content"]
        .as_array()
        .and_then(|c| c.first())
        .and_then(|c| c["text"].as_str())
        .unwrap_or("");
    assert!(
        !text.contains("not connected"),
        "the call reached the attached extension, not the no-extension path: {call:?}"
    );
}
