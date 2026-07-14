// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Browser handshake, output redaction, and parent-audit parity scenarios.

use std::io::{BufRead as _, BufReader, Write as _};
use std::time::Duration;

use anyhow::{anyhow, ensure};
use serde_json::{json, Value};

use crate::scenarios::Scenario;
use crate::support::{self, TempRoot};

pub(super) fn registry() -> Vec<Scenario> {
    vec![
        ("legacy-read-page-redaction", read_page_redaction),
        ("legacy-late-extension-wait", late_extension_wait),
        ("legacy-form-fill-parent-audit", form_fill_parent_audit),
    ]
}

fn write_line(stdin: &mut std::process::ChildStdin, value: &Value) -> anyhow::Result<()> {
    serde_json::to_writer(&mut *stdin, value)?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;
    Ok(())
}

fn read_line(reader: &mut BufReader<std::process::ChildStdout>) -> anyhow::Result<Value> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    ensure!(!line.is_empty(), "adapter stdout closed");
    Ok(serde_json::from_str(line.trim_end())?)
}

fn start_pair(
    tag: &str,
) -> anyhow::Result<(TempRoot, String, support::ChildGuard, support::ChildGuard)> {
    let tmp = TempRoot::new(tag)?;
    let endpoint = support::unique_endpoint(tag);
    let service = support::spawn_service(&endpoint, tmp.path())?;
    let adapter = support::spawn_adapter(&endpoint, tmp.path())?;
    Ok((tmp, endpoint, service, adapter))
}

fn read_page_redaction() -> anyhow::Result<()> {
    let (_tmp, endpoint, _service, mut adapter) = start_pair("read-page-redaction")?;
    let mut stdin = adapter
        .stdin
        .take()
        .ok_or_else(|| anyhow!("adapter stdin"))?;
    let mut reader = BufReader::new(
        adapter
            .stdout
            .take()
            .ok_or_else(|| anyhow!("adapter stdout"))?,
    );
    write_line(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    )?;
    write_line(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_page","arguments":{"tabId":1}}}),
    )?;

    let extension = std::thread::spawn(move || -> anyhow::Result<()> {
        tokio::runtime::Runtime::new()?.block_on(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let stream = ghostlight_transport::ipc::connect(&endpoint).await?;
            let (mut ext_reader, mut ext_writer) = tokio::io::split(stream);
            support::send_extension_attach_frames(&mut ext_writer).await?;
            let request = support::read_frame_answering_tab_urls(
                &mut ext_reader,
                &mut ext_writer,
                "tool_request",
            )
            .await?;
            let reply = json!({
                "id": request["id"],
                "type": "tool_response",
                "result": {"content":[{
                    "type":"text",
                    "text":"textbox \"Password\" [ref_3] secret_value=\"hunter2\" type=\"password\""
                }]},
            });
            ghostlight_transport::host::write_message(
                &mut ext_writer,
                &serde_json::to_vec(&reply)?,
            )
            .await?;
            Ok(())
        })
    });

    ensure!(read_line(&mut reader)?["id"] == 1);
    let response = read_line(&mut reader)?;
    ensure!(response["id"] == 2 && response["result"]["isError"] != true);
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("read_page returned no text"))?;
    ensure!(text.contains("value=\"[value redacted]\""), "{text}");
    ensure!(!text.contains("secret_value="), "{text}");
    ensure!(!text.contains("hunter2"), "{text}");
    extension
        .join()
        .map_err(|_| anyhow!("fake extension panicked"))??;
    drop(stdin);
    Ok(())
}

fn late_extension_wait() -> anyhow::Result<()> {
    let (_tmp, endpoint, _service, mut adapter) = start_pair("late-extension")?;
    let mut stdin = adapter
        .stdin
        .take()
        .ok_or_else(|| anyhow!("adapter stdin"))?;
    let mut reader = BufReader::new(
        adapter
            .stdout
            .take()
            .ok_or_else(|| anyhow!("adapter stdout"))?,
    );
    write_line(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    )?;
    write_line(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"navigate","arguments":{"url":"https://example.com","tabId":1}}}),
    )?;

    let extension = std::thread::spawn(move || -> anyhow::Result<()> {
        tokio::runtime::Runtime::new()?.block_on(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let stream = ghostlight_transport::ipc::connect(&endpoint).await?;
            let (mut ext_reader, mut ext_writer) = tokio::io::split(stream);
            support::send_extension_attach_frames(&mut ext_writer).await?;
            let request = support::read_frame_answering_tab_urls(
                &mut ext_reader,
                &mut ext_writer,
                "tool_request",
            )
            .await?;
            let reply = json!({
                "id": request["id"],
                "type":"tool_response",
                "result":{"content":[{"type":"text","text":"navigated"}]},
            });
            ghostlight_transport::host::write_message(
                &mut ext_writer,
                &serde_json::to_vec(&reply)?,
            )
            .await?;
            Ok(())
        })
    });

    ensure!(read_line(&mut reader)?["id"] == 1);
    let response = read_line(&mut reader)?;
    ensure!(response["id"] == 2 && response["result"]["isError"] != true);
    let content = response["result"]["content"]
        .as_array()
        .ok_or_else(|| anyhow!("navigate returned no content"))?;
    ensure!(content[0]["text"] == "navigated");
    let note = content
        .last()
        .and_then(|block| block["text"].as_str())
        .ok_or_else(|| anyhow!("navigate returned no wait note"))?;
    ensure!(
        note.starts_with("(waited ") && note.ends_with("s for browser extension handshake)"),
        "{note}"
    );
    extension
        .join()
        .map_err(|_| anyhow!("fake extension panicked"))??;
    drop(stdin);
    Ok(())
}

fn form_fill_parent_audit() -> anyhow::Result<()> {
    let tmp = TempRoot::new("form-fill-parent-audit")?;
    let endpoint = support::unique_endpoint("form-fill-parent-audit");
    let config_root = tmp.path().join("config");
    let config_dir = config_root.join("ghostlight");
    let audit_path = tmp.path().join("audit.jsonl");
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(
        config_dir.join("config.json"),
        serde_json::to_vec(&json!({"config":{
            "audit.enabled":true,
            "audit.destination":"file",
            "audit.file.path":audit_path.to_string_lossy(),
        }}))?,
    )?;
    let (_service, _port) =
        support::spawn_service_with_webapi(&endpoint, tmp.path(), Some(&config_root))?;
    let mut adapter = support::spawn_adapter(&endpoint, tmp.path())?;
    let mut stdin = adapter
        .stdin
        .take()
        .ok_or_else(|| anyhow!("adapter stdin"))?;
    let mut reader = BufReader::new(
        adapter
            .stdout
            .take()
            .ok_or_else(|| anyhow!("adapter stdout"))?,
    );
    write_line(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    )?;
    write_line(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"form_fill","arguments":{"tabId":0,"fields":{"Email":"a@b.c"}}}}),
    )?;
    ensure!(read_line(&mut reader)?["id"] == 1);
    let response = read_line(&mut reader)?;
    ensure!(response["id"] == 2 && response["result"]["isError"] == true);
    ensure!(response["result"]["content"][0]["text"]
        .as_str()
        .is_some_and(|text| text.contains("extension")));
    drop(stdin);
    let _ = adapter.wait();

    let audit: Vec<Value> = std::fs::read_to_string(&audit_path)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let parent = audit
        .iter()
        .find(|record| record["tool"] == "form_fill")
        .ok_or_else(|| anyhow!("no form_fill parent record: {audit:?}"))?;
    ensure!(parent["batch_id"].is_string());
    ensure!(parent["action"].is_null());
    ensure!(parent["capability"] == "read");
    let structure = audit
        .iter()
        .find(|record| record["tool"] == "form_structure")
        .ok_or_else(|| anyhow!("no form_structure record: {audit:?}"))?;
    ensure!(structure["orchestrator"] == "form_fill");
    ensure!(structure["batch_id"] == parent["batch_id"]);
    ensure!(structure["step"] == 1);
    ensure!(structure["duration_ms"].is_u64());
    Ok(())
}
