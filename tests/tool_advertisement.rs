// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Integration test for G14 tool advertisement filtering: proves the wiring end to end (a
//! restrictive manifest's grants actually reach `tools/list` through the real server loop), not
//! just the pure filtering logic (`browser::advertise`'s own exhaustive inline unit tests cover
//! that). No extension is ever connected; `tools/list` never touches it.

mod support;

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(0);

/// Mirrors `tests/tool_enforcement.rs`'s helper of the same name: the `file://` source-string
/// form `governance::manifest::source::parse_source_string` expects, on either platform.
fn file_uri(path: &Path) -> String {
    let forward = path.to_string_lossy().replace('\\', "/");
    match forward.strip_prefix('/') {
        Some(rest) => format!("file:///{rest}"),
        None => format!("file:///{forward}"),
    }
}

fn write_manifest(tag: &str, value: &Value) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ghostlight-tool-advertisement-{}-{tag}-{}.json",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    path
}

/// D (H6, forced): only the standalone SERVICE loads policy now (ADR-0030 Decision 8 amendment,
/// PINS.md SS5.1); a bare `ghostlight` invocation is ALWAYS the thin ADAPTER and ignores
/// `--manifest`. Spawns `ghostlight service --manifest <uri>` (`support::spawn_service_with_manifest`)
/// plus a thin adapter dialing it (`support::spawn_adapter`), preserving every pinned assertion
/// verbatim (same category of forced fix as `tests/hub_multiplex.rs`'s own H6 deviation note).
/// Reads exactly the expected `id`-bearing replies BEFORE closing the adapter's stdin (mirrors
/// `tests/mcp_protocol.rs::drive`'s own H6 fix): `relay_adapter` races its two copy directions, so
/// an early close could tear the relay down before a still-in-flight reply is delivered.
fn drive(manifest_path: &Path, requests: &[Value]) -> Vec<Value> {
    let endpoint = format!(
        "ghostlight-ad-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let mut service =
        support::spawn_service_with_manifest(&endpoint, Some(&file_uri(manifest_path)));
    let mut adapter = support::spawn_adapter(&endpoint);

    let mut stdin = adapter.stdin.take().expect("adapter stdin");
    for req in requests {
        stdin
            .write_all(serde_json::to_string(req).unwrap().as_bytes())
            .unwrap();
        stdin.write_all(b"\n").unwrap();
    }

    let expected = requests.iter().filter(|r| r.get("id").is_some()).count();
    let stdout = adapter.stdout.take().expect("adapter stdout");
    let mut lines = BufReader::new(stdout).lines();
    let responses: Vec<Value> = (0..expected)
        .map(|_| {
            let line = lines
                .next()
                .expect("the adapter's stdout closed before every expected reply arrived")
                .unwrap();
            serde_json::from_str(&line).expect("each stdout line is JSON")
        })
        .collect();

    drop(stdin);
    let _ = adapter.wait();
    let _ = service.kill();
    let _ = service.wait();
    responses
}

/// A read-only manifest (`allowed: ["read"]`). Per ADR-0022 Decision 8, a read-only grant
/// advertises every tool with a directory variant that is `requires: []` or a subset of `read`
/// -- everything except `form_input` (requires `write`) and `javascript_tool` (requires
/// `execute`).
#[test]
fn read_only_manifest_advertises_everything_except_write_and_execute_tools() {
    let manifest = write_manifest(
        "read-only",
        &json!({
            "schema": 3,
            "name": "g14-read-only",
            "version": "1",
            "grants": [
                { "id": "r", "hosts": {"allow": ["example.com"]}, "allowed": ["read"] },
            ],
        }),
    );

    let responses = drive(
        &manifest,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        ],
    );
    let list = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("a tools/list response");
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        vec![
            "tabs_context_mcp",
            "tabs_create_mcp",
            "navigate",
            "computer",
            "find",
            "get_page_text",
            "read_console_messages",
            "read_network_requests",
            "read_page",
            "resize_window",
            "update_plan",
            "wait_for",
            "script",
            "explain",
        ]
    );

    std::fs::remove_file(&manifest).ok();
}

/// An empty `grants` array advertises exactly the requires-empty set (ADR-0022 Decision 5 step
/// 2: those actions need no grant at all), not the full surface and not nothing.
#[test]
fn empty_grants_manifest_advertises_exactly_the_requires_empty_set() {
    let manifest = write_manifest(
        "empty-grants",
        &json!({
            "schema": 3,
            "name": "g14-empty-grants",
            "version": "1",
            "grants": [],
        }),
    );

    let responses = drive(
        &manifest,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        ],
    );
    let list = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("a tools/list response");
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        vec![
            "tabs_create_mcp",
            "computer",
            "resize_window",
            "update_plan",
            "script",
            "explain",
        ]
    );

    std::fs::remove_file(&manifest).ok();
}

/// C11 (ADR-0038 Decision 5, PINS.md SS16): the composed guide text -- the exact surface that
/// reaches `initialize.instructions` -- carries the `Cost notes:` paragraph verbatim, and no test
/// under `tests/` pinned the instructions/guide content before this one (grep `instructions`
/// found nothing relevant), so this is the new test the task file names.
#[test]
fn instructions_carry_cost_notes() {
    let text = ghostlight::mcp::tools::agent_guide_text();
    assert!(text.contains("Cost notes:"), "{text}");
    assert!(
        text.contains("get_page_text can return tens of thousands of tokens"),
        "{text}"
    );
}
