// SPDX-License-Identifier: Apache-2.0 OR MIT
//! `ghostlight demo`: a scripted foil-card QA story on the public Sylin Card Foundry
//! (sylin.org/ghostlight/demo/foundry), driven as an ordinary MCP client so it exercises the REAL
//! tool surface -- the same path Claude takes. Cross-platform, superseding the pre-ADR-0046/0051
//! PowerShell harnesses that directly spawned the old single-process server.
//!
//! It connects by spawning `ghostlight-relay --role agent` and speaking newline-delimited
//! JSON-RPC over its stdio -- the relay handles all the connect/handshake/reconnect resilience, so
//! this stays a thin scripted client. At `initialize` it declares a tighten-only session policy
//! overlay (ADR-0060, `examples/demo-policy.json`): grants the public stage plus explicit loopback
//! hosts for local preview. Every Foundry step then works, and the finale -- a navigation to
//! example.com -- is refused by the overlay in ANY service mode, with zero operator setup, so the
//! governance ribbon appears on screen.
//!
//! Prerequisites (checked/reported, never worked around): a running Ghostlight service with the
//! extension attached (`ghostlight doctor`), and a real, visible browser window -- the effects are
//! deliberately hidden from screenshots, so this is a watch-your-browser demo.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// The tighten-only session overlay declared at `initialize` (ADR-0060): grants the public stage
/// and its explicit loopback preview, while the finale's off-domain navigation remains refused.
const DEMO_POLICY: &str = include_str!("../examples/demo-policy.json");

/// How long each chapter caption remains visible and how long the demo waits before acting. The
/// matching values make narration a deliberate chapter card instead of an overlay the next action
/// races underneath.
const NARRATION_DURATION: Duration = Duration::from_secs(6);

/// Let a deliberate page-read scan finish before the next action begins. This is tied to the
/// visual phrase itself rather than the operator's general step pacing.
const INSPECTION_SCAN_DURATION: Duration = Duration::from_millis(1600);

/// Route under the configured demo base for the cohesive simulated application.
const FOUNDRY_ROUTE: &str = "foundry/";

/// Human-readable rejection recorded in the harmless simulated QA form.
const REJECTION_REASON: &str =
    "Rainbow foil crosses the lower-right safe area by 6 px and enters the trim reserve.";

/// Stable local filename used when the recorded journey is placed into the Foundry evidence rail.
const REPLAY_FILENAME: &str = "aurora-qa-replay.gif";

/// The extension's screenshot geometry tunables are included rather than duplicated so computed
/// drag and zoom coordinates stay aligned with ADR-0010 if the source constants change.
const EXTENSION_CONSTANTS: &str = include_str!("../extension/lib/constants.js");

/// The demo's three watchability rhythms, all operator-tunable: a short beat after each visible
/// step, a long hold right after the tab opens (time to resize/position the window before the
/// tour starts), and a breather between sections so each "test" reads as its own scene.
#[derive(Debug, Clone, Copy)]
pub struct Pacing {
    /// Seconds after each visible step (`--pause`, default 3).
    pub step_secs: f64,
    /// Seconds after the demo tab opens, before the tour starts (`--setup-pause`, default 10).
    pub setup_secs: f64,
    /// Seconds between the tour's sections (`--section-pause`, default 5).
    pub section_secs: f64,
}

/// Entry point for the `demo` subcommand. `base_url` defaults to the live site; `pacing` carries
/// the three watchability rhythms (step beat, window-setup hold, section breather).
pub fn run(base_url: &str, pacing: Pacing) -> Result<()> {
    let base = base_url.trim_end_matches('/').to_string();
    let rt = tokio::runtime::Runtime::new().context("build the demo tokio runtime")?;
    rt.block_on(drive(base, pacing))
}

/// A minimal MCP client speaking JSON-RPC over a spawned `ghostlight-relay --role agent`.
struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    next_id: i64,
    pause: Duration,
}

impl Client {
    /// Spawn the relay (the sibling binary of this executable) as an agent-role MCP pass-through
    /// and take its stdio. The relay resolves the same instance this process did (it inherits
    /// `GHOSTLIGHT_INSTANCE`), so `ghostlight --instance dev demo` drives the dev service.
    async fn spawn(pause: Duration) -> Result<Self> {
        let relay = relay_path().context("locate the ghostlight-relay binary")?;
        let mut child = tokio::process::Command::new(&relay)
            .arg("--role")
            .arg("agent")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn {}", relay.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("relay stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("relay stdout unavailable"))?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
            next_id: 0,
            pause,
        })
    }

    /// Send a request and await the response with the matching id, skipping notifications and
    /// unrelated ids. Fails if the relay closes its output before answering.
    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let frame = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.write(&frame).await?;
        loop {
            let line = self
                .stdout
                .next_line()
                .await
                .context("read from relay")?
                .ok_or_else(|| anyhow!("relay closed its output while awaiting '{method}'"))?;
            if line.trim().is_empty() {
                continue;
            }
            let msg: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if msg.get("id").and_then(Value::as_i64) == Some(id) {
                if let Some(err) = msg.get("error") {
                    bail!("'{method}' returned a JSON-RPC error: {err}");
                }
                return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }

    /// Send a notification (no id, no response awaited).
    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.write(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
            .await
    }

    async fn write(&mut self, frame: &Value) -> Result<()> {
        let mut line = serde_json::to_string(frame)?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .context("write to relay")?;
        self.stdin.flush().await.context("flush relay stdin")
    }

    /// Call a tool and return the first text block of its result. A denial is ordinary text
    /// beginning `Denied (` (rendered as a successful result), so callers that want to detect the
    /// guardrail inspect the returned string; a genuine `isError` result is surfaced as an error.
    async fn call_tool_result(&mut self, name: &str, arguments: Value) -> Result<Value> {
        let result = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            bail!("tool '{name}' reported an error: {}", first_text(&result));
        }
        Ok(result)
    }

    /// Call a tool and return the first text block of its successful result.
    async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<String> {
        let result = self.call_tool_result(name, arguments).await?;
        Ok(first_text(&result))
    }

    async fn pause(&self) {
        tokio::time::sleep(self.pause).await;
    }
}

/// The first text block of an MCP tool result (`{ content: [ { type, text } ] }`), or "".
fn first_text(result: &Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        })
        .and_then(|b| b.get("text").and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}

/// Join every text content block in result order. Screenshot results carry their ordinary capture
/// confirmation first and the minted `imageId` instruction in a later block.
fn all_text(result: &Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

/// The `ghostlight-relay` binary sitting next to this executable.
fn relay_path() -> Result<std::path::PathBuf> {
    let exe = std::env::current_exe().context("resolve the current executable")?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow!("executable has no parent directory"))?;
    let name = if cfg!(windows) {
        "ghostlight-relay.exe"
    } else {
        "ghostlight-relay"
    };
    let path = dir.join(name);
    if !path.exists() {
        bail!(
            "ghostlight-relay not found next to {} (expected {})",
            exe.display(),
            path.display()
        );
    }
    Ok(path)
}

fn step(msg: &str) {
    println!("\n>> {msg}");
}

/// The between-sections breather: a visible countdown-free hold so each section of the tour
/// reads as its own scene rather than one continuous blur.
async fn section_break(pacing: &Pacing) {
    tokio::time::sleep(Duration::from_secs_f64(pacing.section_secs.max(0.0))).await;
}

/// Put the demo's own semantic caption track on screen, then leave it undisturbed for its full
/// lifetime so the sentence reads as a chapter card before the section begins. The visual layer
/// controls replacement and expiry; this helper is only pacing and copy.
async fn narrate(c: &mut Client, tab_id: i64, message: &str) -> Result<()> {
    c.call_tool(
        "narrate",
        json!({
            "tabId": tab_id,
            "text": message,
            "position": "auto",
            "duration_ms": NARRATION_DURATION.as_millis()
        }),
    )
    .await?;
    tokio::time::sleep(NARRATION_DURATION).await;
    Ok(())
}

/// Run the whole scripted tour. Returns an error (non-zero exit) if any step fails, so this
/// doubles as an end-to-end smoke test.
async fn drive(base: String, pacing: Pacing) -> Result<()> {
    println!("Ghostlight demo");
    let foundry = format!("{base}/{FOUNDRY_ROUTE}");
    println!("  stage : {foundry}");
    println!(
        "  story : inspect, reject, revise, prove, and govern one simulated foil-card release"
    );
    println!("  note  : keep the browser visible; a 1280 x 720 page viewport is the intended composition.");

    let mut c = Client::spawn(Duration::from_secs_f64(pacing.step_secs.max(0.0))).await?;

    step("Handshake, declaring a tighten-only session policy for the public or loopback stage");
    c.request(
        "initialize",
        json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "ghostlight-demo", "version": env!("CARGO_PKG_VERSION") },
            "_meta": { "ghostlightSessionPolicy": DEMO_POLICY }
        }),
    )
    .await
    .context("initialize (is a Ghostlight service running with the extension attached? run `ghostlight doctor`)")?;
    c.notify("notifications/initialized", json!({})).await?;

    step("Open a fresh tab in the Ghostlight group");
    let created = c.call_tool("tabs_create_mcp", json!({})).await?;
    let tab_id = parse_tab_id(&created)
        .ok_or_else(|| anyhow!("could not read the new tab id from: {created}"))?;
    println!("   tab {tab_id}");
    let setup = pacing.setup_secs.max(0.0);
    if setup > 0.0 {
        println!("   (holding {setup:.0}s -- resize and position the browser window now)");
        tokio::time::sleep(Duration::from_secs_f64(setup)).await;
    }

    step("Open the simulated Sylin Card Foundry");
    c.call_tool("navigate", json!({ "tabId": tab_id, "url": foundry }))
        .await?;
    c.pause().await;

    step("Start the memory-only recording lease");
    c.call_tool(
        "gif_creator",
        json!({ "action": "start_recording", "tabId": tab_id }),
    )
    .await?;
    narrate(
        &mut c,
        tab_id,
        "A foil proof failed QA. Ghostlight will inspect it and leave a release record behind.",
    )
    .await?;

    step("Read the stable Foundry controls once for the inspection and evidence phases");
    let stable_refs = RefInventory::read(
        &mut c,
        tab_id,
        &[
            "Lantern Warden foil card preview",
            "Rotate foil proof",
            "Foil registration drift",
            "Border safe-area collision",
            "Rejection reason",
            "Revision B screenshot evidence",
            "Animated Ghostlight replay",
            "Promote Aurora set to production",
        ],
    )
    .await?;
    tokio::time::sleep(INSPECTION_SCAN_DURATION).await;

    step("Inspect the complete surface, hover the foil, rotate the proof, and zoom the defect");
    let _ = take_screenshot(&mut c, tab_id).await?;
    hover_ref(
        &mut c,
        tab_id,
        stable_refs.require("Lantern Warden foil card preview")?,
    )
    .await?;
    tokio::time::sleep(INSPECTION_SCAN_DURATION).await;
    click_ref(&mut c, tab_id, stable_refs.require("Rotate foil proof")?).await?;
    tokio::time::sleep(Duration::from_millis(3200)).await;
    let defect_region = model_rect(&mut c, tab_id, "#foundry-defect", 42.0).await?;
    c.call_tool(
        "computer",
        json!({ "action": "zoom", "tabId": tab_id, "region": defect_region }),
    )
    .await?;
    c.pause().await;

    section_break(&pacing).await;
    narrate(
        &mut c,
        tab_id,
        "The foil crosses the safe area. The agent will document the defect and request a revision.",
    )
    .await?;

    step("Record the two failed checks and type the rejection reason");
    click_ref(
        &mut c,
        tab_id,
        stable_refs.require("Foil registration drift")?,
    )
    .await?;
    c.pause().await;
    click_ref(
        &mut c,
        tab_id,
        stable_refs.require("Border safe-area collision")?,
    )
    .await?;
    c.pause().await;
    click_ref(&mut c, tab_id, stable_refs.require("Rejection reason")?).await?;
    c.call_tool(
        "computer",
        json!({ "action": "type", "tabId": tab_id, "text": REJECTION_REASON }),
    )
    .await?;
    c.pause().await;

    step("Drag the defect ticket to Request revision, then inspect its local signals");
    // A full screenshot re-establishes ADR-0010's whole-viewport coordinate context after zoom.
    let _ = take_screenshot(&mut c, tab_id).await?;
    let drag_points = model_centers(
        &mut c,
        tab_id,
        &["#foundry-ticket", "#foundry-revision-drop"],
    )
    .await?;
    c.call_tool(
        "computer",
        json!({
            "action": "left_click_drag",
            "tabId": tab_id,
            "start_coordinate": [drag_points[0], drag_points[1]],
            "coordinate": [drag_points[2], drag_points[3]]
        }),
    )
    .await?;
    c.pause().await;
    let _ = c
        .call_tool("read_console_messages", json!({ "tabId": tab_id }))
        .await;
    let _ = c
        .call_tool("read_network_requests", json!({ "tabId": tab_id }))
        .await;
    c.call_tool(
        "wait_for",
        json!({ "tabId": tab_id, "text": "Revision B ready", "timeout_ms": 8000 }),
    )
    .await?;

    section_break(&pacing).await;
    narrate(
        &mut c,
        tab_id,
        "Revision B is clean. Ghostlight will attach visual proof and complete the release packet.",
    )
    .await?;

    step("Read the newly rendered Revision B controls once");
    let revision_refs = RefInventory::read(
        &mut c,
        tab_id,
        &[
            "Foil registration verified",
            "Sylin back stamp verified",
            "Visual evidence attached",
        ],
    )
    .await?;
    c.pause().await;

    step("Capture Revision B and place the screenshot into the local evidence rail");
    let image_id = take_screenshot(&mut c, tab_id).await?;
    c.call_tool(
        "upload_image",
        json!({
            "imageId": image_id,
            "ref": stable_refs.require("Revision B screenshot evidence")?,
            "tabId": tab_id,
            "filename": "aurora-revision-b.jpg"
        }),
    )
    .await?;
    c.pause().await;

    step("Click each final QA check, then complete the structured release packet");
    for query in [
        "Foil registration verified",
        "Sylin back stamp verified",
        "Visual evidence attached",
    ] {
        click_ref(&mut c, tab_id, revision_refs.require(query)?).await?;
        c.pause().await;
    }
    c.call_tool(
        "form_fill",
        json!({
            "tabId": tab_id,
            "fields": {
                "Release name": "Aurora",
                "Set code": "AUR-01",
                "Release owner": "Mira Chen",
                "QA note": "Revision B keeps the foil highlight inside the artwork mask."
            },
            "submit": true
        }),
    )
    .await?;
    c.call_tool(
        "wait_for",
        json!({ "tabId": tab_id, "text": "Aurora is ready for its boundary check" }),
    )
    .await?;
    c.pause().await;

    section_break(&pacing).await;
    hover_ref(
        &mut c,
        tab_id,
        stable_refs.require("Promote Aurora set to production")?,
    )
    .await?;
    narrate(
        &mut c,
        tab_id,
        "The release record is complete. Production remains a separately governed boundary.",
    )
    .await?;
    step("Ask Ghostlight to step off the granted domain -- the real policy should refuse");
    let outcome = c
        .call_tool(
            "navigate",
            json!({ "tabId": tab_id, "url": "https://example.com/" }),
        )
        .await?;
    if outcome.starts_with("Denied") {
        println!("   refused, on screen and in plain language:");
        println!("   {outcome}");
        c.pause().await;
    } else {
        bail!(
            "the off-domain navigation was NOT refused (got: {outcome}). The session policy overlay \
             did not take effect -- is this build's service current with ADR-0060?"
        );
    }

    step("Export the memory-only recording into the Foundry evidence rail");
    let export = c
        .call_tool(
            "gif_creator",
            json!({
                "action": "export",
                "tabId": tab_id,
                "ref": stable_refs.require("Animated Ghostlight replay")?,
                "filename": REPLAY_FILENAME
            }),
        )
        .await?;
    println!("   {export}");
    c.call_tool(
        "wait_for",
        json!({ "tabId": tab_id, "text": "Replay ready", "timeout_ms": 15000 }),
    )
    .await?;
    c.call_tool("gif_creator", json!({ "action": "clear", "tabId": tab_id }))
        .await?;

    println!("\nDemo complete -- the card was revised, the evidence replayed, captured bytes were cleared, and the guardrail held.");
    Ok(())
}

/// Capture the whole viewport, leave ADR-0010's full-page coordinate context current, and return
/// the minted screenshot id used by `upload_image` later in the story.
async fn take_screenshot(c: &mut Client, tab_id: i64) -> Result<String> {
    let result = c
        .call_tool_result(
            "computer",
            json!({ "action": "screenshot", "tabId": tab_id }),
        )
        .await?;
    let text = all_text(&result);
    parse_image_id(&text).ok_or_else(|| anyhow!("screenshot did not report an imageId: {text}"))
}

/// References for one stable Foundry phase, collected by one meaningful interactive page read.
/// Reusing them keeps the read-scan effect tied to inspection rather than making it appear before
/// every click, keystroke, and capture.
#[derive(Debug)]
struct RefInventory {
    refs: BTreeMap<String, String>,
}

impl RefInventory {
    /// Read the interactive surface once and require every named control in that snapshot.
    async fn read(c: &mut Client, tab_id: i64, names: &[&str]) -> Result<Self> {
        let page = c
            .call_tool(
                "read_page",
                json!({ "tabId": tab_id, "filter": "interactive" }),
            )
            .await?;
        let mut refs = BTreeMap::new();
        for name in names {
            let element_ref = ref_for_name(&page, name)
                .ok_or_else(|| anyhow!("Card Foundry control not found: {name}"))?;
            refs.insert((*name).to_string(), element_ref);
        }
        Ok(Self { refs })
    }

    /// Require a reference that was declared when this phase inventory was built.
    fn require(&self, name: &str) -> Result<&str> {
        self.refs
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| anyhow!("Card Foundry reference was not inventoried: {name}"))
    }
}

/// Click one already-inspected control through the ordinary `computer` path.
async fn click_ref(c: &mut Client, tab_id: i64, element_ref: &str) -> Result<()> {
    c.call_tool(
        "computer",
        json!({ "action": "left_click", "tabId": tab_id, "ref": element_ref }),
    )
    .await?;
    Ok(())
}

/// Hover one already-inspected element so the phantom cursor and page-owned foil treatment agree.
async fn hover_ref(c: &mut Client, tab_id: i64, element_ref: &str) -> Result<()> {
    c.call_tool(
        "computer",
        json!({ "action": "hover", "tabId": tab_id, "ref": element_ref }),
    )
    .await?;
    Ok(())
}

/// Return one screenshot-geometry constant from the extension's canonical constants source.
fn extension_numeric_constant(name: &str) -> Result<f64> {
    let marker = format!("{name}:");
    let after = EXTENSION_CONSTANTS
        .split_once(&marker)
        .map(|(_, value)| value.trim_start())
        .ok_or_else(|| anyhow!("extension constant {name} is missing"))?;
    let numeric: String = after
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect();
    numeric
        .parse::<f64>()
        .with_context(|| format!("parse extension constant {name} from {numeric:?}"))
}

/// JavaScript statements that reproduce `extension/lib/geometry.js::targetDims` from the included
/// canonical tunables. This projects CSS page geometry into the model screenshot coordinate space
/// expected by `computer` after a full screenshot.
fn screenshot_projection_js() -> Result<String> {
    let px_per_token = extension_numeric_constant("PX_PER_TOKEN")?;
    let max_tokens = extension_numeric_constant("MAX_TOKENS")?;
    let max_side = extension_numeric_constant("MAX_SIDE")?;
    Ok(format!(
        "const vpW=window.innerWidth,vpH=window.innerHeight;let shotW=vpW,shotH=vpH;\
         const tokens=Math.ceil(shotW/{px_per_token})*Math.ceil(shotH/{px_per_token});\
         if(tokens>{max_tokens}){{const s=Math.sqrt({max_tokens}/tokens);shotW=Math.round(shotW*s);shotH=Math.round(shotH*s);}}\
         const longest=Math.max(shotW,shotH);if(longest>{max_side}){{const s={max_side}/longest;shotW=Math.round(shotW*s);shotH=Math.round(shotH*s);}}\
         const project=(x,y)=>[Math.round(x*shotW/vpW),Math.round(y*shotH/vpH)];"
    ))
}

/// Find an element rectangle in the page and project it into the current full-screenshot model
/// coordinates, expanded by `padding` CSS pixels for a useful zoom composition.
async fn model_rect(c: &mut Client, tab_id: i64, selector: &str, padding: f64) -> Result<Vec<f64>> {
    let selector = serde_json::to_string(selector)?;
    let projection = screenshot_projection_js()?;
    let script = format!(
        "(()=>{{{projection}const el=document.querySelector({selector});if(!el)return null;\
         const r=el.getBoundingClientRect(),a=project(Math.max(0,r.left-{padding}),Math.max(0,r.top-{padding})),\
         b=project(Math.min(vpW,r.right+{padding}),Math.min(vpH,r.bottom+{padding}));return[a[0],a[1],b[0],b[1]];}})()"
    );
    let result = c
        .call_tool_result(
            "javascript_tool",
            json!({ "action": "javascript_exec", "tabId": tab_id, "text": script }),
        )
        .await?;
    let text = page_content_payload(&result)?;
    parse_number_array(&text, 4).with_context(|| format!("project zoom rectangle for {selector}"))
}

/// Project the centers of exactly two page elements into the current full-screenshot coordinate
/// space, producing `[start_x, start_y, end_x, end_y]` for `left_click_drag`.
async fn model_centers(c: &mut Client, tab_id: i64, selectors: &[&str; 2]) -> Result<Vec<f64>> {
    let selectors = serde_json::to_string(selectors)?;
    let projection = screenshot_projection_js()?;
    let script = format!(
        "(()=>{{{projection}const els={selectors}.map(s=>document.querySelector(s));if(els.some(e=>!e))return null;\
         const points=els.map(e=>{{const r=e.getBoundingClientRect();return project(r.left+r.width/2,r.top+r.height/2);}});\
         return[points[0][0],points[0][1],points[1][0],points[1][1]];}})()"
    );
    let result = c
        .call_tool_result(
            "javascript_tool",
            json!({ "action": "javascript_exec", "tabId": tab_id, "text": script }),
        )
        .await?;
    let text = page_content_payload(&result)?;
    parse_number_array(&text, 4).context("project Foundry drag centers")
}

/// Return the payload inside a service-authored page-content boundary for a machine consumer.
///
/// Current services provide the same nonce in structured provenance and both text markers. The
/// demo validates all three before removing the control text. A raw result remains accepted for
/// compatibility with services from before ADR-0078. Marker-shaped text without matching
/// structured provenance is never stripped.
fn page_content_payload(result: &Value) -> Result<String> {
    const PREFIX: &str = "--- GHOSTLIGHT PAGE CONTENT ";
    let text = first_text(result);
    let Some(provenance) = result.pointer("/structuredContent/provenance") else {
        if text.starts_with(PREFIX) {
            bail!("page-content boundary is missing structured provenance");
        }
        return Ok(text);
    };
    if provenance.get("pageSourced").and_then(Value::as_bool) != Some(true)
        || provenance.get("untrusted").and_then(Value::as_bool) != Some(true)
    {
        bail!("page-content provenance is missing its untrusted page marker");
    }
    let nonce = provenance
        .get("sessionNonce")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("page-content provenance has no session nonce"))?;
    if nonce.len() != 32
        || !nonce
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("page-content provenance has an invalid session nonce");
    }
    let origin = provenance
        .get("topOrigin")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("page-content provenance has no top origin"))?;
    let opening = format!("{PREFIX}{nonce} origin={origin} UNTRUSTED ---\n");
    let closing = format!("\n--- END GHOSTLIGHT PAGE CONTENT {nonce} ---");
    let payload = text
        .strip_prefix(&opening)
        .and_then(|body| body.strip_suffix(&closing))
        .ok_or_else(|| anyhow!("page-content boundary does not match structured provenance"))?;
    Ok(payload.to_string())
}

/// Parse a JSON number array returned by `javascript_tool` and pin its expected length.
fn parse_number_array(text: &str, expected: usize) -> Result<Vec<f64>> {
    let values: Vec<f64> = serde_json::from_str(text)
        .with_context(|| format!("parse numeric JavaScript result: {text}"))?;
    if values.len() != expected {
        bail!(
            "numeric JavaScript result had {} values; expected {expected}",
            values.len()
        );
    }
    Ok(values)
}

/// Pull the minted `img_...` id from the text block appended to screenshot results.
fn parse_image_id(text: &str) -> Option<String> {
    let start = text.find("[imageId: ")? + "[imageId: ".len();
    let rest = &text[start..];
    let end = rest.find(']')?;
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// Find the ref on the interactive-tree line whose accessible name contains `name`.
fn ref_for_name(page: &str, name: &str) -> Option<String> {
    let needle = name.to_ascii_lowercase();
    page.lines()
        .find(|line| line.to_ascii_lowercase().contains(&needle))
        .and_then(parse_first_ref)
}

/// Pull the first `ref_N` token out of a find/read_page result.
fn parse_first_ref(text: &str) -> Option<String> {
    let idx = text.find("ref_")?;
    let rest = &text[idx..];
    let end = rest
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || *ch == '_'))
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// Pull the composite `tabId` out of a tabs_create_mcp result (either a `"tabId": N` field in the
/// text, or a bare "Created tab N").
fn parse_tab_id(text: &str) -> Option<i64> {
    // tabs_create's human confirmation names the newly created composite id before its diagnostic
    // tab list, which can contain native `"tabId"` values. Prefer that explicit confirmation.
    if let Some(idx) = text.find("Created tab ") {
        return extract_i64(&text[idx + 12..]);
    }
    if let Some(idx) = text.find("\"tabId\"") {
        let rest = &text[idx + 7..];
        return extract_i64(rest);
    }
    extract_i64(text)
}

/// The first run of ASCII digits (optionally sign-prefixed) parsed as i64.
fn extract_i64(s: &str) -> Option<i64> {
    let start = s.find(|c: char| c.is_ascii_digit() || c == '-')?;
    let rest = &s[start..];
    let end = rest
        .char_indices()
        .skip(1)
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

impl Drop for Client {
    fn drop(&mut self) {
        // Best-effort: close stdin and reap the relay so it does not linger.
        let _ = self.child.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_composite_tab_id_from_structured_text() {
        assert_eq!(
            parse_tab_id(r#"{"tabId": 93825471234567, "url": "x"}"#),
            Some(93825471234567)
        );
        assert_eq!(parse_tab_id("Created tab 42 in the group"), Some(42));
        assert_eq!(parse_tab_id("no id here"), None);
        assert_eq!(
            parse_tab_id("Created tab 5541167942.\n{\"tabs\":[{\"tabId\":1246200642}]}"),
            Some(5541167942)
        );
    }

    #[test]
    fn pulls_the_first_ref_token() {
        assert_eq!(
            parse_first_ref("button [ref_7] primary"),
            Some("ref_7".to_string())
        );
        assert_eq!(parse_first_ref("nothing"), None);
    }

    #[test]
    fn finds_named_refs_in_one_interactive_tree() {
        let page = "button \"Rotate foil proof\" [ref_4]\n  checkbox \"Foil registration drift Color layer leaves the artwork mask\" [ref_7]\n\nViewport: 1280x720";
        assert_eq!(
            ref_for_name(page, "Rotate foil proof"),
            Some("ref_4".to_string())
        );
        assert_eq!(
            ref_for_name(page, "foil REGISTRATION drift"),
            Some("ref_7".to_string())
        );
        assert_eq!(ref_for_name(page, "Missing control"), None);
    }

    #[test]
    fn first_text_reads_the_text_block() {
        let r = json!({ "content": [ { "type": "text", "text": "hello" } ] });
        assert_eq!(first_text(&r), "hello");
    }

    #[test]
    fn all_text_preserves_later_screenshot_metadata() {
        let r = json!({ "content": [
            { "type": "text", "text": "Screenshot captured (jpeg)." },
            { "type": "image", "data": "abc" },
            { "type": "text", "text": "[imageId: img_42] Use it." }
        ] });
        assert_eq!(
            all_text(&r),
            "Screenshot captured (jpeg).\n[imageId: img_42] Use it."
        );
    }

    #[test]
    fn parses_the_screenshot_cache_id() {
        assert_eq!(
            parse_image_id(
                "Screenshot captured.\n[imageId: img_0123abcd] Reference this id with upload_image."
            ),
            Some("img_0123abcd".to_string())
        );
        assert_eq!(parse_image_id("Screenshot captured."), None);
    }

    #[test]
    fn reads_screenshot_geometry_from_the_extension_source() {
        assert_eq!(extension_numeric_constant("PX_PER_TOKEN").unwrap(), 28.0);
        assert_eq!(extension_numeric_constant("MAX_TOKENS").unwrap(), 1568.0);
        assert_eq!(extension_numeric_constant("MAX_SIDE").unwrap(), 1568.0);
    }

    #[test]
    fn pins_numeric_javascript_result_length() {
        assert_eq!(
            parse_number_array("[10,20.5,30,40]", 4).unwrap(),
            vec![10.0, 20.5, 30.0, 40.0]
        );
        assert!(parse_number_array("[1,2]", 4).is_err());
    }

    #[test]
    fn unwraps_a_provenance_bound_machine_result() {
        let nonce = "00112233445566778899aabbccddeeff";
        let result = json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "--- GHOSTLIGHT PAGE CONTENT {nonce} origin=https://example.com UNTRUSTED ---\n\
                     [10,20.5,30,40]\n\
                     --- END GHOSTLIGHT PAGE CONTENT {nonce} ---"
                )
            }],
            "structuredContent": {
                "provenance": {
                    "pageSourced": true,
                    "untrusted": true,
                    "topOrigin": "https://example.com",
                    "sessionNonce": nonce
                }
            }
        });
        let payload = page_content_payload(&result).unwrap();
        assert_eq!(payload, "[10,20.5,30,40]");
        assert_eq!(
            parse_number_array(&payload, 4).unwrap(),
            vec![10.0, 20.5, 30.0, 40.0]
        );
    }

    #[test]
    fn preserves_raw_machine_results_from_older_services() {
        let result = json!({ "content": [{ "type": "text", "text": "[1,2,3,4]" }] });
        assert_eq!(page_content_payload(&result).unwrap(), "[1,2,3,4]");
    }

    #[test]
    fn refuses_to_strip_unverified_or_mismatched_boundaries() {
        let nonce = "00112233445566778899aabbccddeeff";
        let wrapped = format!(
            "--- GHOSTLIGHT PAGE CONTENT {nonce} origin=https://example.com UNTRUSTED ---\n\
             [1,2,3,4]\n\
             --- END GHOSTLIGHT PAGE CONTENT {nonce} ---"
        );
        let unverified = json!({ "content": [{ "type": "text", "text": wrapped }] });
        assert!(page_content_payload(&unverified).is_err());

        let mismatched = json!({
            "content": [{ "type": "text", "text": wrapped }],
            "structuredContent": {
                "provenance": {
                    "pageSourced": true,
                    "untrusted": true,
                    "topOrigin": "https://example.com",
                    "sessionNonce": "ffeeddccbbaa99887766554433221100"
                }
            }
        });
        assert!(page_content_payload(&mismatched).is_err());
    }
}
