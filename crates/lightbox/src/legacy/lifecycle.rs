// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Adapter selection, reconnection, anti-squat, and browser-relay lifecycle parity scenarios.

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::path::Path;
use std::process::{ChildStdin, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{anyhow, ensure};
use serde_json::{json, Value};

use crate::scenarios::Scenario;
use crate::support::{self, ChildGuard, TempRoot};

pub(super) fn registry() -> Vec<Scenario> {
    vec![
        ("legacy-adapter-reconnect", adapter_reconnect),
        ("legacy-adapter-five-second-gap", adapter_five_second_gap),
        (
            "legacy-adapter-candidate-failover",
            adapter_candidate_failover,
        ),
        (
            "legacy-adapter-candidate-fallback",
            adapter_candidate_fallback,
        ),
        ("legacy-service-survives-adapter", service_survives_adapter),
        ("legacy-adapter-anti-squat", adapter_anti_squat),
        ("legacy-browser-relay-restart", browser_relay_restart),
    ]
}

fn start_service(
    endpoint: &str,
    instance: Option<&str>,
    log_dir: &Path,
    keep_warm: bool,
) -> anyhow::Result<ChildGuard> {
    std::fs::create_dir_all(log_dir)?;
    let mut command = support::service_command()?;
    command.arg("service");
    if keep_warm {
        command.arg("--keep-warm");
    }
    command
        .env("GHOSTLIGHT_ENDPOINT", endpoint)
        .env("GHOSTLIGHT_DEBUG", "1")
        .env("GHOSTLIGHT_LOG_DIR", log_dir)
        .env("GHOSTLIGHT_AUDIT_DIR", log_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(instance) = instance {
        command.env("GHOSTLIGHT_INSTANCE", instance);
    }
    let child = support::spawn_guard(&mut command)?;
    support::wait_for_debug_states(log_dir, 1, Duration::from_secs(15))?;
    Ok(child)
}

struct AgentRelay {
    child: ChildGuard,
    stdin: Option<ChildStdin>,
    replies: Receiver<String>,
}

impl AgentRelay {
    fn start(endpoints: &[String], instance: &str, log_dir: &Path) -> anyhow::Result<Self> {
        let mut command = support::relay_command()?;
        command
            .arg("--role")
            .arg("agent")
            .env_remove("GHOSTLIGHT_ENDPOINT")
            .env("GHOSTLIGHT_ENDPOINTS", endpoints.join(","))
            .env("GHOSTLIGHT_INSTANCE", instance)
            .env("GHOSTLIGHT_LOG_DIR", log_dir)
            .env("GHOSTLIGHT_DEBUG", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = support::spawn_guard(&mut command)?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("relay stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("relay stdout"))?;
        let (sender, replies) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            stdin: Some(stdin),
            replies,
        })
    }

    fn send(&mut self, value: &Value) -> anyhow::Result<()> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("relay stdin closed"))?;
        serde_json::to_writer(&mut *stdin, value)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        Ok(())
    }

    fn receive(&self, within: Duration) -> anyhow::Result<Value> {
        let line = match self.replies.recv_timeout(within) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => anyhow::bail!("no relay reply within {within:?}"),
            Err(RecvTimeoutError::Disconnected) => anyhow::bail!("relay stdout closed"),
        };
        Ok(serde_json::from_str(&line)?)
    }

    fn close(mut self) {
        self.stdin.take();
        let _ = self.child.wait();
    }
}

fn prime_relay(relay: &mut AgentRelay) -> anyhow::Result<()> {
    relay.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    relay.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
    relay.send(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))?;
    ensure!(relay.receive(Duration::from_secs(10))?["id"] == 1);
    let tools = relay.receive(Duration::from_secs(10))?;
    ensure!(tools["id"] == 2);
    ensure!(
        tools["result"]["tools"].as_array().map(Vec::len)
            == Some(ghostlight_core::browser::directory::advertised_tool_count())
    );
    Ok(())
}

fn reconnect_case(gap: Duration) -> anyhow::Result<()> {
    let tmp = TempRoot::new("adapter-reconnect")?;
    let endpoint = support::unique_endpoint("adapter-reconnect");
    let log_dir = tmp.path().join("logs");
    let instance = "lightboxreconnect";
    let mut first = start_service(&endpoint, Some(instance), &log_dir, false)?;
    let mut relay = AgentRelay::start(std::slice::from_ref(&endpoint), instance, &log_dir)?;
    prime_relay(&mut relay)?;

    first.kill()?;
    first.wait()?;
    std::thread::sleep(gap);
    let _second = start_service(&endpoint, Some(instance), &log_dir, false)?;
    relay.send(&json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}))?;
    let reply = relay.receive(Duration::from_secs(30))?;
    ensure!(
        reply["id"] == 3,
        "replayed initialize leaked to the client: {reply}"
    );
    ensure!(
        reply["result"]["tools"].as_array().map(Vec::len)
            == Some(ghostlight_core::browser::directory::advertised_tool_count())
    );
    let state =
        support::wait_state_for_role(&log_dir, "adapter", Duration::from_secs(10), |value| {
            value["counters"]["reconnects"].as_u64().unwrap_or(0) >= 1
        })?;
    ensure!(state["counters"]["identity_mints"] == 1);
    relay.close();
    Ok(())
}

fn adapter_reconnect() -> anyhow::Result<()> {
    reconnect_case(Duration::ZERO)
}

fn adapter_five_second_gap() -> anyhow::Result<()> {
    reconnect_case(Duration::from_secs(5))
}

fn adapter_candidate_failover() -> anyhow::Result<()> {
    let tmp = TempRoot::new("candidate-failover")?;
    let endpoint_a = support::unique_endpoint("candidate-a");
    let endpoint_b = support::unique_endpoint("candidate-b");
    let log_dir = tmp.path().join("logs");
    let instance_a = "lightboxa";
    let instance_b = "lightboxb";
    let mut first = start_service(&endpoint_a, Some(instance_a), &log_dir, true)?;
    let _second = start_service(&endpoint_b, Some(instance_b), &log_dir, true)?;
    support::wait_for_debug_states(&log_dir, 2, Duration::from_secs(15))?;
    let mut relay = AgentRelay::start(
        &[endpoint_a.clone(), endpoint_b.clone()],
        "lightboxoverride",
        &log_dir,
    )?;
    relay.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let init = relay.receive(Duration::from_secs(20))?;
    ensure!(init["result"]["serverInfo"]["name"] == format!("ghostlight-{instance_a}"));
    support::wait_state_for_role(&log_dir, "adapter", Duration::from_secs(10), |value| {
        value["counters"]["resolved_candidate"] == 1
    })?;
    relay.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
    first.kill()?;
    first.wait()?;
    relay.send(&json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}))?;
    ensure!(relay.receive(Duration::from_secs(30))?["id"] == 3);
    let state =
        support::wait_state_for_role(&log_dir, "adapter", Duration::from_secs(15), |value| {
            value["counters"]["resolved_candidate"] == 2
        })?;
    ensure!(state["counters"]["candidate_total"] == 2);
    relay.close();
    Ok(())
}

fn adapter_candidate_fallback() -> anyhow::Result<()> {
    let tmp = TempRoot::new("candidate-fallback")?;
    let absent = support::unique_endpoint("candidate-absent");
    let live = support::unique_endpoint("candidate-live");
    let log_dir = tmp.path().join("logs");
    let instance = "lightboxfallback";
    let _service = start_service(&live, Some(instance), &log_dir, true)?;
    let mut relay = AgentRelay::start(&[absent, live], instance, &log_dir)?;
    relay.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let init = relay.receive(Duration::from_secs(20))?;
    ensure!(init["result"]["serverInfo"]["name"] == format!("ghostlight-{instance}"));
    relay.close();
    Ok(())
}

fn service_survives_adapter() -> anyhow::Result<()> {
    let tmp = TempRoot::new("service-survives-adapter")?;
    let endpoint = support::unique_endpoint("service-survives-adapter");
    let log_dir = tmp.path().join("logs");
    let service = start_service(&endpoint, None, &log_dir, false)?;
    let service_pid = service.id();
    let mut relay =
        AgentRelay::start(std::slice::from_ref(&endpoint), "lightboxsurvive", &log_dir)?;
    relay.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    ensure!(relay.receive(Duration::from_secs(10))?["id"] == 1);
    drop(relay);
    std::thread::sleep(Duration::from_secs(2));
    ensure!(ghostlight_transport::proc::pid_exists(service_pid));
    Ok(())
}

fn adapter_anti_squat() -> anyhow::Result<()> {
    let tmp = TempRoot::new("adapter-anti-squat")?;
    let endpoint = support::unique_endpoint("adapter-anti-squat");
    let service_logs = tmp.path().join("service-logs");
    let adapter_logs = tmp.path().join("adapter-logs");
    std::fs::create_dir_all(&adapter_logs)?;
    let _service = start_service(&endpoint, None, &service_logs, false)?;
    let mut command = support::relay_command()?;
    command
        .arg("--role")
        .arg("agent")
        .env("GHOSTLIGHT_ENDPOINT", &endpoint)
        .env("GHOSTLIGHT_LOG_DIR", &adapter_logs)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut relay = support::spawn_guard(&mut command)?;
    let mut stderr = relay.stderr.take().ok_or_else(|| anyhow!("relay stderr"))?;
    let reader = std::thread::spawn(move || {
        let mut captured = String::new();
        let _ = stderr.read_to_string(&mut captured);
        captured
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let exited = loop {
        if relay.try_wait()?.is_some() {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    if !exited {
        relay.kill()?;
    }
    relay.wait()?;
    let captured = reader
        .join()
        .map_err(|_| anyhow!("stderr reader panicked"))?;
    ensure!(exited, "anti-squat mismatch did not terminate the relay");
    ensure!(captured.contains(
        "refusing to connect: the Ghostlight service on this endpoint is not the one this user installed"
    ));
    Ok(())
}

fn write_chrome_frame(stdin: &mut ChildStdin, payload: &[u8]) -> anyhow::Result<()> {
    stdin.write_all(&(payload.len() as u32).to_le_bytes())?;
    stdin.write_all(payload)?;
    stdin.flush()?;
    Ok(())
}

fn clear_debug_states(log_dir: &Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("debug-state-") && name.ends_with(".json") {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn browser_relay_restart() -> anyhow::Result<()> {
    let tmp = TempRoot::new("browser-relay-restart")?;
    let endpoint = support::unique_endpoint("browser-relay-restart");
    let log_dir = tmp.path().join("logs");
    let mut first = start_service(&endpoint, None, &log_dir, false)?;
    let mut command = support::relay_command()?;
    command
        .arg(format!("chrome-extension://{}/", "a".repeat(32)))
        .env("GHOSTLIGHT_ENDPOINT", &endpoint)
        .env("GHOSTLIGHT_LOG_DIR", &log_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut relay = support::spawn_guard(&mut command)?;
    let mut stdin = relay
        .stdin
        .take()
        .ok_or_else(|| anyhow!("browser relay stdin"))?;
    let identity = serde_json::to_vec(&json!({
        "type": ghostlight_transport::handshake::EXTENSION_IDENTITY_TYPE,
        ghostlight_transport::handshake::BROWSER_ID_FIELD: "lightbox-browser-relay",
    }))?;
    write_chrome_frame(&mut stdin, &identity)?;
    support::wait_extension_connected(&log_dir, Duration::from_secs(15))?;
    first.kill()?;
    first.wait()?;
    std::thread::sleep(Duration::from_secs(2));
    ensure!(
        relay.try_wait()?.is_none(),
        "browser relay exited with the service"
    );
    clear_debug_states(&log_dir)?;
    let _second = start_service(&endpoint, None, &log_dir, false)?;
    support::wait_extension_connected(&log_dir, Duration::from_secs(20))?;
    drop(stdin);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if relay.try_wait()?.is_some() {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "browser relay survived browser EOF"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
