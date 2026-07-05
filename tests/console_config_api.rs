// SPDX-License-Identifier: Apache-2.0 OR MIT
//! K3 (`docs/tasks/console/K3-config-provenance-api.md`; PINS.md CS1, CS2): `GET /api/v1/config`,
//! the provenance-aware config view (per key: value, source layer, lock, description) -- a READ
//! of the ADR-0019 five-layer key registry, never a manifest document.

mod support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

static SEQ: AtomicU32 = AtomicU32::new(0);

fn test_webapi_port(seq: u32) -> u16 {
    20000 + ((std::process::id()).wrapping_add(seq) % 10000) as u16
}

/// One raw HTTP/1.1 GET over a plain TCP connection, with an optional `Origin` header (used to
/// exercise the `channels.webapi.from` decision without needing a genuinely remote peer).
fn http_get(port: u16, path: &str, origin: Option<&str>) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the web API");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let origin_header = origin
        .map(|o| format!("Origin: {o}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{origin_header}Connection: close\r\n\r\n"
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
    response.split("\r\n\r\n").nth(1).unwrap_or_default()
}

/// PINS.md CS2: structural shape only (never a specific key's value/source), since a real
/// spawned service reads this machine's own, un-isolated user config path -- asserting an exact
/// default for an arbitrary pre-existing key would be fragile on a machine with its own real
/// Ghostlight configuration. Registry key COUNT and ORDER come straight from the live registry
/// itself (`ghostlight::governance::config::KEYS`), never a hardcoded guess.
#[test]
fn config_api_returns_every_registered_key_in_registry_order() {
    let endpoint = format!(
        "ghostlight-console-config-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let port = test_webapi_port(10);
    let mut service = support::spawn_service_with_webapi_port(&endpoint, port);

    let response = http_get(port, "/api/v1/config", None);
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK");
    let parsed: serde_json::Value = serde_json::from_str(body(&response)).expect("valid JSON");
    let keys = parsed["keys"].as_array().expect("keys array");

    let expected_keys: Vec<&'static str> = ghostlight::governance::config::KEYS
        .iter()
        .map(|def| def.key)
        .collect();
    assert_eq!(keys.len(), expected_keys.len());

    for (entry, expected_key) in keys.iter().zip(expected_keys.iter()) {
        assert_eq!(entry["key"], *expected_key);
        assert!(
            entry.get("value").is_some(),
            "{expected_key}: missing value"
        );
        let source = entry["source"].as_str().expect("source is a string");
        assert!(
            matches!(
                source,
                "org_mandatory" | "user" | "org_recommended" | "preset" | "builtin"
            ),
            "{expected_key}: unexpected source {source}"
        );
        assert!(entry["locked"].is_boolean(), "{expected_key}: locked");
        assert!(
            !entry["description"].as_str().unwrap_or_default().is_empty(),
            "{expected_key}: empty description"
        );
    }

    let _ = service.kill();
    let _ = service.wait();
}

/// PINS.md CS2: an org-mandatory override is reflected as `source: "org_mandatory"`,
/// `locked: true`. Uses a real org-policy-file override (the same shape
/// `tests/manifest_validation.rs::org_policy_file_with_config_boots_the_server` uses), which is
/// SAFE regardless of this machine's own real user config state: org-mandatory always outranks
/// the user layer in the resolution precedence, so this assertion cannot be defeated by an
/// unrelated pre-existing user config file.
#[test]
fn config_api_reflects_a_locked_org_mandatory_key() {
    let pid = std::process::id();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let program_data_dir =
        std::env::temp_dir().join(format!("ghostlight-console-config-org-{pid}-{seq}"));
    let policy_dir = program_data_dir.join("ghostlight");
    std::fs::create_dir_all(&policy_dir).expect("create fake ProgramData\\ghostlight");
    let policy_path = policy_dir.join("policy.json");

    let manifest = serde_json::json!({
        "schema": 3,
        "name": "console-k3-org-mandatory",
        "version": "1",
        "grants": [],
        "config": [
            { "key": "audit.enabled", "value": true, "level": "mandatory" },
        ],
    });
    std::fs::write(&policy_path, serde_json::to_vec(&manifest).unwrap())
        .expect("write the org policy file");

    let endpoint = format!("ghostlight-console-config-org-{pid}-{seq}");
    let port = test_webapi_port(11);
    let mut service = support::spawn_service_with_program_data_and_webapi_port(
        &endpoint,
        &program_data_dir,
        port,
    );

    let response = http_get(port, "/api/v1/config", None);
    assert_eq!(status_line(&response), "HTTP/1.1 200 OK");
    let parsed: serde_json::Value = serde_json::from_str(body(&response)).expect("valid JSON");
    let keys = parsed["keys"].as_array().expect("keys array");
    let entry = keys
        .iter()
        .find(|k| k["key"] == "audit.enabled")
        .expect("audit.enabled is registered");
    assert_eq!(entry["source"], "org_mandatory");
    assert_eq!(entry["locked"], true);
    assert_eq!(entry["value"], true);

    let _ = service.kill();
    let _ = service.wait();
    std::fs::remove_dir_all(&program_data_dir).ok();
}

/// PINS.md CS1.3: `/api/v1/config` is gated by the SAME `channels.webapi.from` decision every
/// other Console route uses -- a source outside the default `["localhost"]` allowlist (forced via
/// an `Origin` header naming a non-loopback host) is refused with the SAME 403 shape.
#[test]
fn config_api_is_refused_when_channels_webapi_from_denies_the_source() {
    let endpoint = format!(
        "ghostlight-console-config-403-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let port = test_webapi_port(12);
    let mut service = support::spawn_service_with_webapi_port(&endpoint, port);

    let response = http_get(port, "/api/v1/config", Some("http://evil.example.com"));
    assert_eq!(status_line(&response), "HTTP/1.1 403 Forbidden");

    let _ = service.kill();
    let _ = service.wait();
}
