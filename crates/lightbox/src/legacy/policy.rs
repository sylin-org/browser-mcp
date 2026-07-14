// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Injected local-policy boot and live-reload parity scenarios.

use std::ffi::OsString;
use std::sync::Once;
use std::time::Duration;

use anyhow::{anyhow, ensure};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

use ghostlight_core::browser::pattern;
use ghostlight_core::governance::config::reload::PolicySource;
use ghostlight_core::governance::manifest::source;
use ghostlight_core::governance::paths::GovernancePaths;
use ghostlight_core::hub::outbound::browser::Browser;
use ghostlight_core::hub::session::SessionGuid;
use ghostlight_core::hub::ServiceContext;
use ghostlight_core::mcp::server::serve_session;
use ghostlight_transport::observability::DebugSink;
use ghostlight_transport::role::{set_role, Role};

use crate::scenarios::Scenario;
use crate::support::TempRoot;

static ROLE_ONCE: Once = Once::new();

pub(super) fn registry() -> Vec<Scenario> {
    vec![
        ("legacy-org-policy-boot", org_policy_boot),
        ("legacy-org-policy-hot-reload", org_policy_hot_reload),
    ]
}

fn ensure_service_role() {
    ROLE_ONCE.call_once(|| set_role(Role::Service));
}

fn manifest(capabilities: &[&str]) -> Value {
    json!({
        "schema": 3,
        "name": "lightbox-local-policy",
        "version": "1",
        "grants": [{
            "id": "r",
            "hosts": {"allow": ["example.com"]},
            "allowed": capabilities,
        }],
    })
}

fn read_only_tools() -> Vec<String> {
    [
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
        "narrate",
        "wait_for",
        "script",
        "browser_batch",
        "gif_creator",
        "explain",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn expanded_tools() -> Vec<String> {
    [
        "tabs_context_mcp",
        "tabs_create_mcp",
        "navigate",
        "computer",
        "find",
        "form_input",
        "get_page_text",
        "read_console_messages",
        "read_network_requests",
        "read_page",
        "resize_window",
        "update_plan",
        "narrate",
        "wait_for",
        "script",
        "form_fill",
        "file_upload",
        "browser_batch",
        "upload_image",
        "gif_creator",
        "explain",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn tool_names(response: &Value) -> anyhow::Result<Vec<String>> {
    response["result"]["tools"]
        .as_array()
        .ok_or_else(|| anyhow!("tools/list returned no tools: {response}"))?
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("tool has no name: {tool}"))
        })
        .collect()
}

fn build_context(paths: &GovernancePaths) -> anyhow::Result<ServiceContext> {
    ensure_service_role();
    let loaded = source::load_policy_at(&paths.org_policy, None, pattern::is_valid_pattern)?;
    ensure!(
        loaded.manifest.is_some(),
        "injected org policy did not load"
    );
    Ok(ServiceContext::from_startup(
        Browser::new(),
        DebugSink::disabled(),
        loaded,
        PolicySource::Local {
            paths: paths.clone(),
            user_source: None,
        },
        None,
    )?)
}

struct SessionDriver {
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    replies: tokio::sync::mpsc::UnboundedReceiver<Value>,
    history: Vec<Value>,
    session: tokio::task::JoinHandle<()>,
    reader: tokio::task::JoinHandle<()>,
}

impl SessionDriver {
    async fn start(context: ServiceContext) -> Self {
        let (client, server) = tokio::io::duplex(256 * 1024);
        let session = tokio::spawn(async move {
            let _ = serve_session(server, context, SessionGuid::mint()).await;
        });
        let (read_half, writer) = tokio::io::split(client);
        let (sender, replies) = tokio::sync::mpsc::unbounded_channel();
        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(read_half).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(value) = serde_json::from_str(&line) else {
                    break;
                };
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        Self {
            writer,
            replies,
            history: Vec::new(),
            session,
            reader,
        }
    }

    async fn send(&mut self, value: &Value) -> anyhow::Result<()> {
        self.writer.write_all(&serde_json::to_vec(value)?).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn receive_id(&mut self, id: i64, within: Duration) -> anyhow::Result<Value> {
        let receive = async {
            loop {
                let value = self
                    .replies
                    .recv()
                    .await
                    .ok_or_else(|| anyhow!("session output closed"))?;
                self.history.push(value.clone());
                if value["id"] == id {
                    return Ok(value);
                }
            }
        };
        tokio::time::timeout(within, receive)
            .await
            .map_err(|_| anyhow!("no response for id {id} within {within:?}"))?
    }

    async fn request(&mut self, id: i64, method: &str, params: Value) -> anyhow::Result<Value> {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await?;
        self.receive_id(id, Duration::from_secs(15)).await
    }

    async fn poll_tools_until(
        &mut self,
        next_id: &mut i64,
        expected: &[String],
    ) -> anyhow::Result<()> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            let id = *next_id;
            *next_id += 1;
            let response = self.request(id, "tools/list", json!({})).await?;
            if tool_names(&response)? == expected {
                return Ok(());
            }
            ensure!(
                tokio::time::Instant::now() < deadline,
                "advertised tools never matched {expected:?}; last response: {response}"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    async fn finish(mut self) {
        let _ = self.writer.shutdown().await;
        drop(self.writer);
        let _ = self.session.await;
        let _ = self.reader.await;
    }
}

struct AuditDirGuard {
    previous: Option<OsString>,
}

impl AuditDirGuard {
    fn set(path: &std::path::Path) -> Self {
        let previous = std::env::var_os("GHOSTLIGHT_AUDIT_DIR");
        std::env::set_var("GHOSTLIGHT_AUDIT_DIR", path);
        Self { previous }
    }
}

impl Drop for AuditDirGuard {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            std::env::set_var("GHOSTLIGHT_AUDIT_DIR", previous);
        } else {
            std::env::remove_var("GHOSTLIGHT_AUDIT_DIR");
        }
    }
}

fn org_policy_boot() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let tmp = TempRoot::new("org-policy-boot")?;
        let _audit = AuditDirGuard::set(tmp.path());
        let paths = GovernancePaths::under(tmp.path());
        std::fs::write(&paths.org_policy, serde_json::to_vec(&manifest(&["read"]))?)?;
        let context = build_context(&paths)?;
        let mut driver = SessionDriver::start(context).await;
        let initialized = driver.request(1, "initialize", json!({})).await?;
        ensure!(initialized["result"].is_object(), "{initialized}");
        let tools = driver.request(2, "tools/list", json!({})).await?;
        ensure!(tool_names(&tools)? == read_only_tools(), "{tools}");
        driver.finish().await;
        Ok(())
    })
}

fn org_policy_hot_reload() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let tmp = TempRoot::new("org-policy-hot-reload")?;
        let _audit = AuditDirGuard::set(tmp.path());
        let paths = GovernancePaths::under(tmp.path());
        std::fs::write(&paths.org_policy, serde_json::to_vec(&manifest(&["read"]))?)?;
        let context = build_context(&paths)?;
        let mut driver = SessionDriver::start(context).await;
        let initialized = driver
            .request(
                1,
                "initialize",
                json!({"clientInfo":{"name":"lightbox-hot-reload","version":"1.2.3"}}),
            )
            .await?;
        ensure!(initialized["result"].is_object(), "{initialized}");
        let initial = driver.request(2, "tools/list", json!({})).await?;
        ensure!(tool_names(&initial)? == read_only_tools());

        std::fs::write(
            &paths.org_policy,
            serde_json::to_vec(&manifest(&["read", "action", "write"]))?,
        )?;
        let mut next_id = 3;
        driver
            .poll_tools_until(&mut next_id, &expanded_tools())
            .await?;
        let call_id = 9_000;
        let call = driver
            .request(
                call_id,
                "tools/call",
                json!({"name":"tabs_create_mcp","arguments":{}}),
            )
            .await?;
        ensure!(call["id"] == call_id);

        std::fs::remove_file(&paths.org_policy)?;
        let all_open: Vec<String> = ghostlight_core::browser::directory::advertised_tool_names()
            .into_iter()
            .map(str::to_string)
            .collect();
        driver.poll_tools_until(&mut next_id, &all_open).await?;
        let notifications = driver
            .history
            .iter()
            .filter(|value| value["method"] == "notifications/tools/list_changed")
            .count();
        ensure!(notifications == 2, "notifications: {:?}", driver.history);
        driver.finish().await;

        tokio::time::sleep(Duration::from_millis(100)).await;
        let audit_path = tmp.path().join("audit.jsonl");
        let audit: Vec<Value> = std::fs::read_to_string(&audit_path)?
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()?;
        let reloads: Vec<&Value> = audit
            .iter()
            .filter(|record| record["event"] == "manifest_reload")
            .collect();
        ensure!(reloads.len() == 2, "reload events: {audit:?}");
        ensure!(reloads[1]["manifest"].is_null(), "{reloads:?}");
        ensure!(audit.iter().any(|record| {
            record.get("event").is_none() && record["client"]["name"] == "lightbox-hot-reload"
        }));
        Ok(())
    })
}
