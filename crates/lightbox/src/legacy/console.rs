// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Console and web-ingestion parity scenarios migrated from the legacy spawn tier.

use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use anyhow::{anyhow, ensure};

use crate::scenarios::Scenario;
use crate::support::{self, ChildGuard, TempRoot};

const ENABLE_REMOTE_ROUTE: &str = "/api/v1/config/inbound-web-enable-remote";

pub(super) fn registry() -> Vec<Scenario> {
    vec![
        ("legacy-console-index", console_index),
        ("legacy-console-assets", console_assets),
        ("legacy-console-not-found", console_not_found),
        (
            "legacy-console-method-not-allowed",
            console_method_not_allowed,
        ),
        (
            "legacy-console-websocket-default-off",
            websocket_default_off,
        ),
        ("legacy-console-websocket-opt-in", websocket_opt_in),
        ("legacy-console-config-registry", config_registry),
        ("legacy-console-config-source-denied", config_source_denied),
        (
            "legacy-console-enable-remote-disabled",
            enable_remote_disabled,
        ),
        ("legacy-console-enable-remote-csrf", enable_remote_csrf),
        ("legacy-console-live-sessions", live_sessions),
    ]
}

struct Console {
    _root: TempRoot,
    _service: ChildGuard,
    endpoint: String,
    log_dir: std::path::PathBuf,
    user_config_dir: std::path::PathBuf,
    port: u16,
}

impl Console {
    fn start(tag: &str, user_config: Option<&str>) -> anyhow::Result<Self> {
        let root = TempRoot::new(tag)?;
        let endpoint = support::unique_endpoint(tag);
        let log_dir = root.path().join("logs");
        let user_config_dir = root.path().join("config");
        let configured = if let Some(json) = user_config {
            let directory = user_config_dir.join("ghostlight");
            std::fs::create_dir_all(&directory)?;
            std::fs::write(directory.join("config.json"), json)?;
            Some(user_config_dir.as_path())
        } else {
            None
        };
        let (service, port) = support::spawn_service_with_webapi(&endpoint, &log_dir, configured)?;
        Ok(Self {
            _root: root,
            _service: service,
            endpoint,
            log_dir,
            user_config_dir,
            port,
        })
    }

    fn adapter(&self) -> anyhow::Result<ChildGuard> {
        support::spawn_adapter(&self.endpoint, &self.log_dir)
    }
}

fn request(
    port: u16,
    method: &str,
    path: &str,
    headers: &str,
    body: &str,
) -> anyhow::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn get(port: u16, path: &str, headers: &str) -> anyhow::Result<String> {
    request(port, "GET", path, headers, "")
}

fn status(response: &str) -> &str {
    response.lines().next().unwrap_or_default()
}

fn body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default()
}

fn header<'a>(response: &'a str, wanted: &str) -> Option<&'a str> {
    response
        .split("\r\n")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
        })
        .map(|(_, value)| value.trim())
}

fn websocket_response(port: u16) -> anyhow::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let request = format!(
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    let mut buffer = [0u8; 512];
    let count = stream.read(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer[..count]).into_owned())
}

fn console_index() -> anyhow::Result<()> {
    let console = Console::start("console-index", None)?;
    let response = get(console.port, "/", "")?;
    ensure!(status(&response) == "HTTP/1.1 200 OK");
    ensure!(header(&response, "Content-Type") == Some("text/html; charset=utf-8"));
    ensure!(body(&response).contains("/manage.css"));
    ensure!(body(&response).contains("/manage.js"));
    Ok(())
}

fn console_assets() -> anyhow::Result<()> {
    let console = Console::start("console-assets", None)?;
    let css = get(console.port, "/manage.css", "")?;
    let js = get(console.port, "/manage.js", "")?;
    ensure!(status(&css) == "HTTP/1.1 200 OK");
    ensure!(header(&css, "Content-Type") == Some("text/css; charset=utf-8"));
    ensure!(status(&js) == "HTTP/1.1 200 OK");
    ensure!(header(&js, "Content-Type") == Some("application/javascript; charset=utf-8"));
    Ok(())
}

fn console_not_found() -> anyhow::Result<()> {
    let console = Console::start("console-not-found", None)?;
    let response = get(console.port, "/api/v1/nope", "")?;
    ensure!(status(&response) == "HTTP/1.1 404 Not Found");
    ensure!(body(&response) == "not found");
    let outside = get(console.port, "/nope", "")?;
    ensure!(status(&outside) == "HTTP/1.1 400 Bad Request");
    Ok(())
}

fn console_method_not_allowed() -> anyhow::Result<()> {
    let console = Console::start("console-method", None)?;
    let response = request(console.port, "POST", "/", "", "")?;
    ensure!(status(&response) == "HTTP/1.1 405 Method Not Allowed");
    ensure!(body(&response) == "method not allowed");
    Ok(())
}

fn websocket_default_off() -> anyhow::Result<()> {
    let console = Console::start("console-ws-off", None)?;
    ensure!(websocket_response(console.port)?.starts_with("HTTP/1.1 403 Forbidden"));
    Ok(())
}

fn websocket_opt_in() -> anyhow::Result<()> {
    let console = Console::start(
        "console-ws-on",
        Some(r#"{"config":{"inbound.web.enabled":true}}"#),
    )?;
    ensure!(websocket_response(console.port)?.starts_with("HTTP/1.1 101 Switching Protocols"));
    Ok(())
}

fn config_registry() -> anyhow::Result<()> {
    let console = Console::start("console-config", None)?;
    let response = get(console.port, "/api/v1/config", "")?;
    ensure!(status(&response) == "HTTP/1.1 200 OK");
    let parsed: serde_json::Value = serde_json::from_str(body(&response))?;
    let keys = parsed["keys"]
        .as_array()
        .ok_or_else(|| anyhow!("config response has no keys array"))?;
    ensure!(keys.len() == ghostlight_core::governance::config::KEYS.len());
    for (entry, definition) in keys.iter().zip(ghostlight_core::governance::config::KEYS) {
        ensure!(entry["key"] == definition.key);
        ensure!(entry.get("value").is_some());
        ensure!(entry["locked"].is_boolean());
        ensure!(!entry["description"].as_str().unwrap_or_default().is_empty());
        ensure!(matches!(
            entry["source"].as_str(),
            Some("org_mandatory" | "user" | "org_recommended" | "preset" | "builtin")
        ));
    }
    Ok(())
}

fn config_source_denied() -> anyhow::Result<()> {
    let console = Console::start("console-config-denied", None)?;
    let response = get(
        console.port,
        "/api/v1/config",
        "Origin: http://evil.example.com\r\n",
    )?;
    ensure!(status(&response) == "HTTP/1.1 403 Forbidden");
    Ok(())
}

fn enable_remote_disabled() -> anyhow::Result<()> {
    let console = Console::start("console-enable-disabled", None)?;
    let response = request(
        console.port,
        "POST",
        ENABLE_REMOTE_ROUTE,
        "X-Ghostlight-Intent: enable-remote\r\n",
        "",
    )?;
    ensure!(status(&response) == "HTTP/1.1 403 Forbidden");
    let parsed: serde_json::Value = serde_json::from_str(body(&response))?;
    ensure!(parsed["disabled"] == true);
    ensure!(parsed["error"]
        .as_str()
        .unwrap_or_default()
        .contains("temporarily disabled"));
    ensure!(!console
        .user_config_dir
        .join("ghostlight")
        .join("config.json")
        .exists());
    Ok(())
}

fn enable_remote_csrf() -> anyhow::Result<()> {
    let console = Console::start("console-enable-csrf", None)?;
    let response = request(console.port, "POST", ENABLE_REMOTE_ROUTE, "", "")?;
    ensure!(status(&response) == "HTTP/1.1 403 Forbidden");
    ensure!(!console
        .user_config_dir
        .join("ghostlight")
        .join("config.json")
        .exists());
    Ok(())
}

fn live_sessions() -> anyhow::Result<()> {
    let console = Console::start("console-sessions", None)?;
    let mut adapter = console.adapter()?;
    let mut stdin = adapter
        .stdin
        .take()
        .ok_or_else(|| anyhow!("adapter has no stdin"))?;
    stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")?;
    stdin.write_all(
        b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\
          \"params\":{\"name\":\"navigate\",\"arguments\":{\"tabId\":424242}}}\n",
    )?;

    let deadline = Instant::now() + Duration::from_secs(5);
    let parsed = loop {
        let response = get(console.port, "/api/v1/sessions", "")?;
        ensure!(status(&response) == "HTTP/1.1 200 OK");
        let parsed: serde_json::Value = serde_json::from_str(body(&response))?;
        let binding = parsed["adapter_bindings"].as_array().and_then(|bindings| {
            bindings.iter().find(|binding| {
                binding["owned_tab_ids"]
                    .as_array()
                    .map(|ids| ids.iter().any(|id| id == 424242))
                    .unwrap_or(false)
            })
        });
        if binding.is_some() {
            break parsed;
        }
        ensure!(
            Instant::now() < deadline,
            "no adapter binding appeared: {parsed}"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    ensure!(parsed["live_session_count"].as_u64().unwrap_or(0) >= 1);
    let binding = parsed["adapter_bindings"]
        .as_array()
        .and_then(|bindings| {
            bindings.iter().find(|binding| {
                binding["owned_tab_ids"]
                    .as_array()
                    .map(|ids| ids.iter().any(|id| id == 424242))
                    .unwrap_or(false)
            })
        })
        .ok_or_else(|| anyhow!("no binding owns the synthetic tab"))?;
    ensure!(binding["guid"].as_str().map(str::len) == Some(8));
    if cfg!(not(target_os = "macos")) {
        ensure!(binding["pid"].as_u64().unwrap_or(0) > 0);
    }
    Ok(())
}
