// SPDX-License-Identifier: Apache-2.0 OR MIT
//! `POST /api/v1/config/inbound-web-enable-remote` -- the Console's former enable-remote action,
//! TEMPORARILY DISABLED (SEC-HIGH-02, 2026-07): opening `inbound.web` to the LAN over plaintext
//! HTTP is a foot-gun (mass-exposed Ollama/Selenium/Ray -> RCE/botnet in the prior art), so until a
//! secure remote-access design lands the action refuses and points at a tunnel. These tests pin
//! that disabled behavior (no config write, no audit event) plus the CSRF header gate that still
//! guards the endpoint. The old write-path tests (pinned-value write, config_changed audit,
//! org-lock 409, body-ignored) were removed with the write path they exercised.

mod support;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static SEQ: AtomicU32 = AtomicU32::new(0);

/// POST carrying the `X-Ghostlight-Intent: enable-remote` consent header (CSRF hard-stop) -- the
/// header the Console's own JS sends.
fn http_post(port: u16, path: &str, body: &str) -> String {
    http_post_with_intent(port, path, body, true)
}

/// POST with the consent header optionally omitted, for the CSRF negative test.
fn http_post_with_intent(port: u16, path: &str, body: &str, intent: bool) -> String {
    let mut stream = support::connect_webapi(port);
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let intent_header = if intent {
        "X-Ghostlight-Intent: enable-remote\r\n"
    } else {
        ""
    };
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{intent_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn status_line(response: &str) -> &str {
    response.lines().next().unwrap_or_default()
}

fn body(response: &str) -> &str {
    // split_once: everything after the FIRST header/body delimiter, even when the body itself
    // contains a blank line (a "\r\n\r\n" run). A plain split(..).nth(1) would return only the
    // segment up to the body's first blank line and silently truncate it.
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default()
}

const ROUTE: &str = "/api/v1/config/inbound-web-enable-remote";

/// SEC-HIGH-02: the action is disabled -- a request WITH the consent header is refused with 403
/// and a `disabled` body, and the isolated user config file is never written (so no LAN listener
/// is ever opened over plaintext).
#[ignore = "e2e: spawns a real ghostlight service/adapter; run via the e2e tier -- cargo test -- --ignored"]
#[test]
fn enable_remote_is_disabled_and_writes_nothing() {
    let pid = std::process::id();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let user_config_dir =
        std::env::temp_dir().join(format!("ghostlight-console-enable-remote-{pid}-{seq}"));
    std::fs::create_dir_all(&user_config_dir).unwrap();

    let endpoint = format!("ghostlight-console-enable-remote-{pid}-{seq}");
    let (mut service, port) =
        support::spawn_service_with_user_config_dir_and_webapi_port(&endpoint, &user_config_dir);

    let response = http_post(port, ROUTE, "");
    assert_eq!(status_line(&response), "HTTP/1.1 403 Forbidden");
    let parsed: serde_json::Value = serde_json::from_str(body(&response)).expect("valid JSON");
    assert_eq!(parsed["disabled"], true, "body: {}", body(&response));
    assert!(
        parsed["error"]
            .as_str()
            .unwrap_or_default()
            .contains("temporarily disabled"),
        "body: {}",
        body(&response)
    );

    assert!(
        !user_config_dir
            .join("ghostlight")
            .join("config.json")
            .exists(),
        "the disabled action must never write the user config file"
    );

    let _ = service.kill();
    let _ = service.wait();
    std::fs::remove_dir_all(&user_config_dir).ok();
}

/// The CSRF hard-stop still guards the endpoint: a request WITHOUT the
/// `X-Ghostlight-Intent: enable-remote` header is refused with 403 before the handler runs, and
/// nothing is written. Kept so the gate is verified for whatever secure action replaces this one.
#[ignore = "e2e: spawns a real ghostlight service/adapter; run via the e2e tier -- cargo test -- --ignored"]
#[test]
fn enable_remote_without_the_intent_header_is_refused_and_writes_nothing() {
    let pid = std::process::id();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let user_config_dir = std::env::temp_dir().join(format!(
        "ghostlight-console-enable-remote-no-intent-{pid}-{seq}"
    ));
    std::fs::create_dir_all(&user_config_dir).unwrap();

    let endpoint = format!("ghostlight-console-enable-remote-no-intent-{pid}-{seq}");
    let (mut service, port) =
        support::spawn_service_with_user_config_dir_and_webapi_port(&endpoint, &user_config_dir);

    let response = http_post_with_intent(port, ROUTE, "", false);
    assert_eq!(status_line(&response), "HTTP/1.1 403 Forbidden");

    assert!(
        !user_config_dir
            .join("ghostlight")
            .join("config.json")
            .exists(),
        "a refused write must not touch the user config file"
    );

    let _ = service.kill();
    let _ = service.wait();
    std::fs::remove_dir_all(&user_config_dir).ok();
}
