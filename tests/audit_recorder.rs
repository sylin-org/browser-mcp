//! Integration test for the audit flight recorder (G06) at its public-API boundary: the
//! `Governance` facade wired to a file-backed `Recorder`, exactly as `transport::mcp::server`
//! wires it in production. Adapts the g06 spec's test 13
//! (`a_recorded_call_lands_as_one_wellformed_jsonl_line`) to the post-A3/A5 architecture, where
//! `set_client`/`record_call` live on `Governance`, not on `Recorder` directly (Recorder only
//! implements the bare `AuditSink::record`).

use browser_mcp::browser::classify;
use browser_mcp::governance::dispatch::Governance;
use browser_mcp::governance::ports::AuditSink;
use serde_json::Value;
use std::sync::Arc;

fn temp_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "browser-mcp-audit-recorder-test-{}-{tag}.jsonl",
        std::process::id()
    ))
}

#[test]
fn a_recorded_call_lands_as_one_wellformed_jsonl_line() {
    let path = temp_path("one-line");
    let _ = std::fs::remove_file(&path);

    let recorder = browser_mcp::governance::audit::Recorder::to_file(path.clone());
    let governance =
        Governance::all_open(Arc::new(recorder) as Arc<dyn AuditSink>, classify::classify);

    governance.set_client("claude-code", "2.1.0");
    governance.record_call("computer", Some("left_click"), 42);

    let content = std::fs::read_to_string(&path).expect("audit file exists");
    assert!(content.ends_with('\n'), "file ends with a single LF");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one line after one recorded call");

    let rec: Value = serde_json::from_str(lines[0]).expect("line is a JSON object");
    let keys: Vec<&String> = rec
        .as_object()
        .expect("record is an object")
        .keys()
        .collect();
    assert_eq!(
        keys,
        vec![
            "event_id",
            "ts",
            "identity",
            "client",
            "tool",
            "action",
            "rw",
            "domain",
            "decision",
            "grant_id",
            "denial_id",
            "duration_ms",
            "manifest"
        ],
        "field order matches the shared format"
    );

    assert_eq!(rec["tool"], "computer");
    assert_eq!(rec["action"], "left_click");
    assert_eq!(rec["rw"], "mutate");
    assert_eq!(rec["decision"], "allow");
    assert_eq!(rec["duration_ms"], 42);
    assert_eq!(rec["client"]["name"], "claude-code");
    assert_eq!(rec["client"]["version"], "2.1.0");
    for field in ["identity", "domain", "grant_id", "denial_id", "manifest"] {
        assert!(rec[field].is_null(), "{field} must be null");
    }

    let event_id = rec["event_id"].as_str().expect("event_id is a string");
    assert_eq!(event_id.len(), 36, "event_id: {event_id}");
    for offset in [8, 13, 18, 23] {
        assert_eq!(event_id.as_bytes()[offset], b'-', "event_id: {event_id}");
    }
    let ts = rec["ts"].as_str().expect("ts is a string");
    assert_eq!(ts.len(), 24, "ts: {ts}");
    assert!(ts.ends_with('Z'), "ts: {ts}");
    chrono::DateTime::parse_from_rfc3339(ts).expect("ts parses as rfc3339");

    // Append, not truncate: a second call must add a second line.
    governance.record_call("navigate", None, 5);
    let content = std::fs::read_to_string(&path).expect("audit file exists");
    assert_eq!(content.lines().count(), 2, "second call appends a line");

    std::fs::remove_file(&path).ok();
}
