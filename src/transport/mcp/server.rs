//! JSON-RPC 2.0 server over stdio (the mcp-server role).
//!
//! Reads newline-delimited JSON-RPC from stdin, handles `initialize` / `tools/list` / `tools/call`,
//! and writes responses to stdout (one compact JSON object per line). `tools/call` routes through
//! the [`Governance`] facade (the dispatch chokepoint) and then forwards to the extension via the
//! [`Browser`] handle. stdout is reserved for the protocol stream; operational logs go to stderr.
//!
//! `tools/call` runs concurrently: each call is spawned on its own task (so a slow or waiting call
//! never blocks `initialize`, `ping`, or later requests) and every response -- inline or from a
//! spawned call -- funnels through a single writer task that owns stdout, so lines are never
//! interleaved mid-write.

use crate::browser::pattern::HostOutcome;
use crate::browser::{advertise, classify, pattern, redact, resource, sacred};
use crate::governance::audit::Recorder;
use crate::governance::config::reload::ConfigStore;
use crate::governance::dispatch::{hold_message, Governance};
use crate::governance::enforcement::LocalPdp;
use crate::governance::manifest::source::{self, LoadedPolicy};
use crate::governance::ports::{AuditSink, Decision, Denial, EffectiveMode, GoverningResource};
use crate::transport::executor::Browser;
use crate::transport::mcp::tools::{is_known_tool, TOOLS_JSON};
use crate::transport::mcp::types::{text_content, JsonRpcResponse};
use crate::{Result, ToolError};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// MCP protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio MCP server loop until stdin closes. `browser` is the (shared) handle to the
/// extension; tool calls are forwarded through it. `loaded_policy` is the manifest resolved at
/// startup (G12, shared format doc sections 1.2-1.3): `None` manifest means all-open. G12
/// itself only feeds a user-supplied manifest's `config` entries into the layer resolver
/// (below) and holds the rest at this scope for later stage-2 tasks (G13 grant enforcement,
/// G14 tool-advertisement filtering) to read grants from; loading it does not change which
/// calls execute.
pub async fn run(browser: Browser, loaded_policy: LoadedPolicy) -> Result<()> {
    if let Some(manifest) = &loaded_policy.manifest {
        tracing::debug!(
            name = %manifest.name,
            version = %manifest.version,
            hash = %manifest.hash,
            "active manifest held for later governance tasks"
        );
    }

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    // Hot-reload substrate (ADR-0019): the resolved Config is held behind an atomic swap; the
    // watcher re-resolves on a config/org/manifest change with no restart. With no files
    // present this resolves to the built-in defaults, so all-open behavior is byte-identical
    // to stage 1. A user-supplied manifest's `config` entries feed the user layer here too
    // (G12); an org-sourced manifest's entries already reach the org layers through G02's own
    // independent parse of the same file, so `manifest_config_as_user_layer` yields an empty
    // map in that case (see its own doc comment).
    let store = ConfigStore::load_initial_with_manifest_config(
        pattern::is_valid_pattern,
        source::manifest_config_as_user_layer(&loaded_policy),
    )?;
    store.clone().spawn_watcher();

    // The audit flight recorder (ADR-0018 step 1) is orthogonal to the governance mode: it
    // records under all-open too, gated only by audit.enabled (shared format doc section 4.5).
    // Its destination is live (RECONCILIATION.md section 3): a config-change watcher re-opens
    // the sink whenever audit.enabled / audit.destination / audit.file.path changes.
    let recorder = Arc::new(Recorder::from_config(&store.current()));
    tokio::spawn({
        let recorder = Arc::clone(&recorder);
        let mut changes = store.subscribe();
        async move {
            while changes.changed().await.is_ok() {
                let config = changes.borrow().clone();
                recorder.reload(&config);
            }
        }
    });

    // Grant enforcement (g13): governed once a manifest is active (org or user-sourced;
    // `loaded_policy` already resolved which one wins), all-open otherwise. `LocalPdp` is the
    // in-process decision point, wired with the browser plugin's real G07 matcher so the
    // domain-agnostic core never names `browser::` directly (the a7 arch-test).
    let governance = Arc::new(match &loaded_policy.manifest {
        Some(manifest) => Governance::governed(
            Box::new(LocalPdp::new(pattern::pattern_matches_normalized_host)),
            recorder.clone() as Arc<dyn AuditSink>,
            classify::classify,
            manifest.grants.clone(),
            manifest.hash.clone(),
            manifest.mode,
        ),
        None => Governance::all_open(recorder as Arc<dyn AuditSink>, classify::classify),
    });

    // Panic kill switch (g11, ADR-0018 step 2): the extension signals `session_killed` once it
    // has severed its own debugger attachments; the binary writes exactly one audit
    // session-event record per kill (`tracing::info!` fires regardless of `audit.enabled`, so
    // the operational log always has the event).
    browser.on_session_killed({
        let governance = Arc::clone(&governance);
        move || {
            governance.record_session_killed();
            tracing::info!("session killed by the user");
        }
    });

    let (tx, mut rx) = mpsc::unbounded_channel::<JsonRpcResponse>();

    // A single writer owns stdout so responses -- including those from spawned `tools/call`
    // tasks -- never interleave mid-write. `debug` is cloned before the spawn so both the
    // writer and the read loop below can record the MCP boundary.
    let debug = browser.debug().clone();
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(resp) = rx.recv().await {
            let mut buf = match serde_json::to_string(&resp) {
                Ok(buf) => buf,
                Err(e) => {
                    tracing::warn!(error = %e, "dropping unserializable response");
                    continue;
                }
            };
            if debug.is_enabled() {
                // Use the already-typed id (do not re-parse the whole -- possibly large -- body).
                let id = resp.id.as_ref().map(Value::to_string).unwrap_or_default();
                debug.mcp_response(&id, &buf);
            }
            buf.push('\n');
            if stdout.write_all(buf.as_bytes()).await.is_err() || stdout.flush().await.is_err() {
                break;
            }
        }
    });

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(resp) = handle_line(&browser, &store, &governance, line, &tx).await {
            let _ = tx.send(resp);
        }
    }
    drop(tx);
    let _ = writer.await;
    Ok(())
}

/// Parse and route one JSON-RPC line.
///
/// Returns `Some(response)` for requests (an `id` member is present, even if `null`) and `None` for
/// notifications (no `id` member) and for lines we cannot parse at all. Fields are read from a raw
/// [`Value`] so a structurally invalid but id-bearing request still gets an addressable `-32600`.
async fn handle_line(
    browser: &Browser,
    store: &Arc<ConfigStore>,
    governance: &Arc<Governance>,
    line: &str,
    tx: &mpsc::UnboundedSender<JsonRpcResponse>,
) -> Option<JsonRpcResponse> {
    let raw: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "dropping unparseable JSON-RPC line");
            return None;
        }
    };

    let is_notification = raw.get("id").is_none();
    let id = raw.get("id").cloned();

    let Some(method) = raw.get("method").and_then(Value::as_str) else {
        return if is_notification {
            tracing::debug!("dropping malformed notification (no method)");
            None
        } else {
            Some(JsonRpcResponse::error(
                id,
                -32600,
                "Invalid Request: missing or non-string 'method'",
            ))
        };
    };

    if browser.debug().is_enabled() {
        let id_str = id.as_ref().map(Value::to_string).unwrap_or_default();
        browser.debug().mcp_request(method, &id_str, line);
    }

    match method {
        "initialize" => {
            // Record the MCP client's self-reported identity (clientInfo.name [+ version]), if it
            // sent one, for `browser-mcp doctor`/`status` to display. Missing params/clientInfo, or
            // non-string fields, are silently fine: this is best-effort observability, not part of
            // the protocol contract, and the response below never depends on it.
            if let Some(client_info) = raw.get("params").and_then(|p| p.get("clientInfo")) {
                if let Some(name) = client_info.get("name").and_then(Value::as_str) {
                    let ident = match client_info.get("version").and_then(Value::as_str) {
                        Some(version) => format!("{name} {version}"),
                        None => name.to_string(),
                    };
                    browser.debug().set_client(&ident);
                }
            }
            // Capture the same clientInfo into the audit recorder's client field (shared
            // format doc section 6.1), first-wins for the whole session.
            capture_client_info(governance, raw.get("params"));
            // Warm the extension channel while the client finishes its handshake. The extension
            // side initiates the connection (Chrome spawns the native-host, which dials the
            // endpoint this process has served since startup), so there is nothing to dial from
            // here; this watcher verifies readiness and records the outcome.
            let wait_ms = store.current().first_call_wait_ms();
            tokio::spawn({
                let browser = browser.clone();
                async move {
                    let started = Instant::now();
                    if browser.wait_connected(Duration::from_millis(wait_ms)).await {
                        tracing::info!(
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            "extension channel ready"
                        );
                    } else {
                        tracing::info!(
                            "extension channel not ready within the warmup window; \
                             the first tools/call will wait for it"
                        );
                    }
                }
            });
            Some(JsonRpcResponse::success(id, initialize_result()))
        }
        "tools/list" => Some(JsonRpcResponse::success(id, tools_list_result(governance))),
        "tools/call" => {
            let browser = browser.clone();
            let store = Arc::clone(store);
            let governance = Arc::clone(governance);
            let tx = tx.clone();
            let params = raw.get("params").cloned();
            tokio::spawn(async move {
                let resp =
                    handle_tools_call(&browser, &store, &governance, id, params.as_ref()).await;
                let _ = tx.send(resp);
            });
            None
        }
        "ping" => Some(JsonRpcResponse::success(id, json!({}))),
        _ if is_notification => {
            tracing::debug!(method, "ignoring unknown notification");
            None
        }
        other => Some(JsonRpcResponse::error(
            id,
            -32601,
            format!("Method not found: {other}"),
        )),
    }
}

/// Capture `clientInfo` from the MCP `initialize` params into the audit recorder (shared
/// format doc section 6.1 `client` field). Both `name` and `version` must be strings;
/// otherwise the session's records carry `client: null`.
fn capture_client_info(governance: &Governance, params: Option<&Value>) {
    let info = params.and_then(|p| p.get("clientInfo"));
    let name = info.and_then(|i| i.get("name")).and_then(Value::as_str);
    let version = info.and_then(|i| i.get("version")).and_then(Value::as_str);
    if let (Some(name), Some(version)) = (name, version) {
        governance.set_client(name, version);
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "browser-mcp", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The advertised surface (g14): the embedded sacred fixture verbatim under all-open, or
/// filtered to the union over the active manifest's grants (`browser::advertise::advertised_tools`)
/// once one is active. Schema text is never altered; only which tools appear in the array
/// changes.
fn tools_list_result(governance: &Governance) -> Value {
    let fixture: Value = serde_json::from_str(TOOLS_JSON).expect("embedded tools.json is valid");
    advertise::advertised_tools(&fixture, governance.grants())
}

async fn handle_tools_call(
    browser: &Browser,
    store: &Arc<ConfigStore>,
    governance: &Governance,
    id: Option<Value>,
    params: Option<&Value>,
) -> JsonRpcResponse {
    // One snapshot for the whole call, taken once at entry: a reload mid-call must not tear
    // the snapshot the call already started with.
    let config = store.current();

    let Some(name) = params.and_then(|p| p.get("name")).and_then(Value::as_str) else {
        return JsonRpcResponse::error(id, -32602, "tools/call requires a string 'name'");
    };
    let args = params
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or(Value::Null);

    // Unknown tool names are rejected before dispatch (and before waiting on the extension
    // channel at all): this is a client-request problem, not a browser/extension problem, and the
    // client should learn that instantly regardless of whether an extension is even connected.
    // The extension keeps its own `Unknown tool: ...` guard as a safety net (defense in depth);
    // this pre-check just means well-formed clients never round-trip to hit it.
    if !is_known_tool(name) {
        let err = ToolError::invalid_request(format!("Unknown tool: {name}"))
            .next_step("call tools/list and use one of the advertised tool names");
        return JsonRpcResponse::success(id, error_result(err));
    }

    // The only tool-call argument ever read for audit purposes: the computer sub-action
    // (shared format doc section 6.2 sensitive-parameter omission; no other argument is read,
    // logged, or stored).
    let action = if name == "computer" {
        args.get("action").and_then(Value::as_str)
    } else {
        None
    };

    // Take-the-wheel hold (g10, ADR-0018 step 2): a user gesture, not a policy decision, so it
    // is checked before ANY dispatch machinery -- before governance.decide, before the sacred
    // check, before any extension traffic. A held call is answered immediately with a
    // successful (never isError) text result and is never queued, deferred, or replayed;
    // resuming affects only future calls. Held calls still produce one audit record
    // (`decision: "allow"`, `held: true`, `duration_ms: 0`).
    if let Some(held_for) = browser.held_for() {
        governance.record_held(name, action);
        return JsonRpcResponse::success(id, text_content(hold_message(name, action, held_for)));
    }

    let dispatch_started = Instant::now();

    // The sacred-domains never-touch check (ADR-0018 step 2, g08): always enforced,
    // independent of governance.mode or manifest presence -- RECONCILIATION.md section 1's
    // "always-on carve-out", and ahead of grant evaluation below (g13: "if the sacred-domains
    // check has already landed, leave it in place and ahead of grant evaluation"). STEP A: an
    // empty list (every preset's default) is the byte-identical fast path -- no extension
    // traffic, no parsing, no allocation.
    let sacred_domains = config.sacred_domains();
    let SacredCheck { tab_domain, denial } = if sacred_domains.is_empty() {
        SacredCheck {
            tab_domain: None,
            denial: None,
        }
    } else {
        sacred_check(browser, sacred_domains, name, &args).await
    };
    if let Some(denial) = denial {
        governance.record_deny(name, action, &denial, tab_domain.as_deref());
        return JsonRpcResponse::success(id, text_content(denial.message));
    }

    // Grant enforcement (g13, ADR-0018 step 3): resolve the governing resource for this call and
    // consult the dispatch chokepoint. All-open (no manifest) skips resolution entirely -- STEP 0
    // must add zero new frames and zero new latency (constraint 3); `Governance::decide` itself
    // would short-circuit to Allow under all-open regardless, but there is nothing to gain from
    // resolving a resource value it will never look at. `audit_domain` starts at the
    // sacred-domains check's own tab resolution (the pre-g13 default for an ungoverned call) and
    // is overwritten with the grant machinery's own resolved host once governed, per shared
    // format doc section 6.1 ("domain: the parser-normalized host, or null"); the two mechanisms
    // resolve the tab independently and deliberately (g08's sacred check and g13's grant check
    // are separate, out-of-scope-for-each-other concerns; see RECONCILIATION.md section 1).
    //
    // `config_mode` (g15) feeds the mode precedence (per-grant > manifest > `governance.mode`);
    // `shadow_denial` carries the would-deny `Denial` forward when the effective mode resolves to
    // observe, so the call still dispatches (exactly like an allow) but the eventual audit record
    // is `shadow_deny`, not `allow`.
    let config_mode = EffectiveMode::from_config_str(config.governance_mode());
    let mut audit_domain = tab_domain.clone();
    let mut audit_grant_id: Option<String> = None;
    let mut shadow_denial: Option<Denial> = None;
    let mut navigate_post_check = false;
    if governance.is_governed() {
        if let Some((resource, domain)) = resolve_governing_resource(browser, name, &args).await {
            audit_domain = domain;
            if name == "navigate" {
                navigate_post_check = true;
            }
            match governance.decide(name, action, resource, config_mode) {
                Decision::Deny(d) => {
                    governance.record_deny(name, action, &d, audit_domain.as_deref());
                    return JsonRpcResponse::success(id, text_content(d.message));
                }
                Decision::Allow { grant_id } => audit_grant_id = grant_id,
                Decision::ShadowDeny(d) => {
                    audit_grant_id = d.grant_id.clone();
                    shadow_denial = Some(d);
                }
            }
        }
        // `None`: an unparseable `navigate` target. The extension refuses an invalid URL without
        // navigating (an ordinary, non-isError "Invalid URL" text result), so there is nothing to
        // govern here or at point 5; fall through to dispatch unconditionally.
    }

    // Bounded first-call wait: the first call of a session races the extension handshake.
    // Wait briefly for the channel instead of failing a healthy session (also covers calls
    // arriving during a mid-session reconnect). If the wait times out, `waited` stays `None` and
    // control falls through to `Browser::call` below, which fails fast with the canonical
    // "extension not connected" `ToolError` -- one hop-attributed message, not two to keep in sync.
    let mut waited: Option<Duration> = None;
    if !browser.is_connected() {
        let started = Instant::now();
        if browser
            .wait_connected(Duration::from_millis(config.first_call_wait_ms()))
            .await
        {
            waited = Some(started.elapsed());
        } else {
            tracing::warn!(
                tool = name,
                "tools/call failed: extension channel never came up"
            );
        }
    }

    let outcome = browser.call(name, &args).await;
    let duration_ms = u64::try_from(dispatch_started.elapsed().as_millis()).unwrap_or(u64::MAX);

    // Point 5 (g13/g15): after a dispatched `navigate` succeeds, re-check the FINAL
    // (post-redirect) landing -- authoritative over the pre-dispatch verdict above for the
    // audit record, since a redirect can land somewhere the target itself never named. Only
    // reachable when governed and the pre-check above actually ran (skipped for an unparseable
    // target, per the fall-through comment above); a failed dispatch gets no post-check
    // (nothing landed).
    if navigate_post_check && outcome.is_ok() {
        if let Some(tab_id) = args.get("tabId").and_then(Value::as_i64) {
            let (landing, landing_domain) =
                post_navigate_landing_check(browser, governance, tab_id, config_mode).await;
            match landing {
                Decision::Allow { grant_id } => {
                    audit_grant_id = grant_id;
                    audit_domain = landing_domain;
                    shadow_denial = None;
                }
                Decision::Deny(d) => {
                    governance.record_navigate_landing_deny(
                        action,
                        &d,
                        landing_domain.as_deref(),
                        duration_ms,
                    );
                    return JsonRpcResponse::success(id, text_content(d.message));
                }
                Decision::ShadowDeny(d) => {
                    audit_grant_id = d.grant_id.clone();
                    audit_domain = landing_domain;
                    shadow_denial = Some(d);
                }
            }
        }
    }

    if let Some(denial) = &shadow_denial {
        governance.record_shadow_deny(name, action, denial, audit_domain.as_deref(), duration_ms);
    } else {
        governance.record_call(
            name,
            action,
            duration_ms,
            audit_domain.as_deref(),
            audit_grant_id.as_deref(),
        );
    }

    match outcome {
        // The extension returns an MCP result object (`{ content: [...] }`). The engine is truthful:
        // read_page carries secret field values under a `secret_value=` marker; the governance
        // overlay rewrites that marker here (redacting per `content.security.secrets.redact`) before
        // the result leaves the binary. Other tools pass through untouched.
        Ok(mut result) => {
            if name == "read_page" {
                redact::apply_to_result(&mut result, config.secrets_redact());
            }
            if let Some(waited) = waited {
                append_wait_note(&mut result, waited);
            }
            JsonRpcResponse::success(id, result)
        }
        // A tool execution failure is an MCP tool error result (isError), not a JSON-RPC error.
        // The rendered text is exactly the hop-attributed ToolError Display: no "Error: " prefix.
        Err(e) => {
            let mut result = error_result(e);
            if let Some(waited) = waited {
                append_wait_note(&mut result, waited);
            }
            JsonRpcResponse::success(id, result)
        }
    }
}

/// Outcome of the sacred-domains check (shared format doc section 3.4, g08).
struct SacredCheck {
    /// The current tab's host at decision time (shared format doc section 6.1 `domain` field),
    /// resolved independently of whether a denial fired -- an allowed call on a clean tab still
    /// carries its `domain` through to the audit record.
    tab_domain: Option<String>,
    /// The denial, if the current tab (STEP B) or, for `navigate`, the target (STEP C) matched
    /// a sacred pattern.
    denial: Option<Denial>,
}

/// STEPs B and C of the sacred-domains check. Only called when the list is non-empty (STEP A,
/// the caller's job). Always enforced, independent of `governance.mode` or manifest presence --
/// RECONCILIATION.md section 1's "always-on carve-out": this runs at the dispatch chokepoint
/// directly, bypassing the grant-based `PolicyDecisionPoint` machinery g12/g13 wire in later
/// (this rule predates and is exempt from that machinery by design, g08 constraint 9).
///
/// STEP B (current-tab check, any tool carrying a numeric `tabId`) runs first, so a sacred
/// current tab denies with the tab's host in the message even for `navigate` (never-touch means
/// the user, not the agent, moves that tab). STEP C (the `navigate` target) runs even when
/// STEP B could not resolve the tab, since it is local and needs no extension.
async fn sacred_check(
    browser: &Browser,
    sacred_domains: &[String],
    tool: &str,
    args: &Value,
) -> SacredCheck {
    let tab_host = match args.get("tabId").and_then(Value::as_i64) {
        Some(tab_id) => resolve_tab_host(browser, tab_id).await,
        None => None,
    };
    let tab_domain = tab_host.as_ref().map(|h| h.as_str().to_string());

    if let Some(host) = &tab_host {
        if let Some(pattern) = sacred::first_match(host, sacred_domains) {
            return SacredCheck {
                tab_domain,
                denial: Some(sacred::sacred(host.as_str(), pattern)),
            };
        }
    }

    if tool == "navigate" {
        if let Some(target_host) = args
            .get("url")
            .and_then(Value::as_str)
            .and_then(sacred::navigate_target_host)
        {
            if let Some(pattern) = sacred::first_match(&target_host, sacred_domains) {
                return SacredCheck {
                    tab_domain,
                    denial: Some(sacred::sacred(target_host.as_str(), pattern)),
                };
            }
        }
    }

    SacredCheck {
        tab_domain,
        denial: None,
    }
}

/// Resolve the g13 governing resource for one call (section 5's summary table). Only called
/// once [`Governance::is_governed`] is true. Returns `None` only for an unparseable `navigate`
/// target: nothing to govern (section 4: "dispatch without pre- or post-check"). Otherwise
/// `Some((resource, domain))`, where `domain` is the resolved host for the audit record's
/// `domain` field when `resource` is [`GoverningResource::Resource`], `None` otherwise (shared
/// format doc section 6.1: never the denial message's `(unknown)` placeholder).
async fn resolve_governing_resource(
    browser: &Browser,
    tool: &str,
    args: &Value,
) -> Option<(GoverningResource, Option<String>)> {
    match tool {
        "navigate" => match args.get("url").and_then(Value::as_str) {
            // "back"/"forward" and a missing/non-string url argument have no target to check
            // pre-dispatch (point 5 covers the landing for "back"/"forward"; the extension's own
            // handling covers a missing url). The union rule (no host, tool/access still apply)
            // is the closest faithful fit: it is never more permissive than a resolved host would
            // be, and it does not require inventing a bypass-everything resource variant.
            Some("back") | Some("forward") | None => Some((GoverningResource::None, None)),
            Some(url_arg) => match resource::navigate_target_resource(url_arg) {
                Some(GoverningResource::Resource(host)) => {
                    Some((GoverningResource::Resource(host.clone()), Some(host)))
                }
                Some(other) => Some((other, None)),
                None => None,
            },
        },
        "tabs_context_mcp" | "tabs_create_mcp" | "update_plan" => {
            Some((GoverningResource::None, None))
        }
        _ => {
            let Some(tab_id) = args.get("tabId").and_then(Value::as_i64) else {
                // Missing/non-integer tabId on a tab-scoped tool: fail closed (constraint 11).
                return Some((GoverningResource::Indeterminate, None));
            };
            let resolved = match browser.tab_url(tab_id).await {
                Ok(Some(url)) => resource::resolved_url_resource(&url),
                Ok(None) | Err(_) => GoverningResource::Indeterminate,
            };
            let domain = match &resolved {
                GoverningResource::Resource(h) => Some(h.clone()),
                _ => None,
            };
            Some((resolved, domain))
        }
    }
}

/// Point 5 (g13, SPEC 5.2 step 5; g15 shadow enforcement): after a dispatched `navigate`
/// succeeds, re-query tab `tab_id`'s FINAL (post-redirect) URL and re-run the SAME governed
/// decision `navigate` itself would get pre-dispatch (reusing [`Governance::decide`] rather
/// than duplicating grant logic), returning the full [`Decision`] plus the resolved landing
/// host (`None` for a non-host landing -- never the denial message's `(unknown)` placeholder).
/// The caller decides what each variant means for the response and the audit record; this
/// function's own side effect is limited to the best-effort `about:blank` park, and ONLY for
/// an actual [`Decision::Deny`] -- a [`Decision::ShadowDeny`] landing must leave the browser
/// untouched (shadow mode is a fully transparent pass-through; parking would be a visible,
/// detectable side effect that gives away a shadowed call, breaking g15's own truthfulness
/// requirement that "the agent must not be able to tell a shadowed call from a permitted one").
async fn post_navigate_landing_check(
    browser: &Browser,
    governance: &Governance,
    tab_id: i64,
    config_mode: EffectiveMode,
) -> (Decision, Option<String>) {
    let resolved = match browser.tab_url(tab_id).await {
        Ok(Some(url)) => resource::resolved_url_resource(&url),
        Ok(None) | Err(_) => GoverningResource::Indeterminate,
    };
    let domain = match &resolved {
        GoverningResource::Resource(h) => Some(h.clone()),
        _ => None,
    };
    let decision = governance.decide("navigate", None, resolved, config_mode);
    if let Decision::Deny(_) = &decision {
        let _ = browser
            .call(
                "navigate",
                &json!({ "url": "about:blank", "tabId": tab_id }),
            )
            .await;
    }
    (decision, domain)
}

/// Resolve the current host of tab `tab_id` via the internal `tabs_context_mcp` lookup. This is
/// machinery, not an MCP tool call: it produces no audit record of its own (shared format doc
/// section 6). Any failure along the way -- the call errors (extension not connected), the reply
/// is not the expected JSON shape (for example the `No Browser MCP tab group` plain-text reply),
/// the tab id is absent from the list, or the url is empty/unparseable -- yields `None`: a deny
/// requires a positive match on a resolved host, so an unresolved lookup never denies (g08
/// constraint 12). Tabs outside the group are refused by the extension itself, and a genuinely
/// failing extension fails the real call identically; this function does not fabricate
/// protection from that failure.
async fn resolve_tab_host(browser: &Browser, tab_id: i64) -> Option<pattern::MatchHost> {
    let result = browser
        .call("tabs_context_mcp", &json!({ "createIfEmpty": false }))
        .await
        .ok()?;
    let text = result.get("content")?.get(0)?.get("text")?.as_str()?;
    let parsed: Value = serde_json::from_str(text).ok()?;
    let tabs = parsed.get("tabs")?.as_array()?;
    let url = tabs
        .iter()
        .find(|t| t.get("tabId").and_then(Value::as_i64) == Some(tab_id))?
        .get("url")?
        .as_str()?;
    match pattern::host_for_matching(url) {
        HostOutcome::Host(h) => Some(h),
        HostOutcome::NonHttpScheme(_) | HostOutcome::Unparseable => None,
    }
}

/// Build an MCP tool error result (`{ content: [...], isError: true }`) from a hop-attributed
/// [`ToolError`]. The result text is exactly the error's `Display`:
/// `[hop: <hop>] <message>. Next step: <next step>.`
fn error_result(err: ToolError) -> Value {
    let mut result = text_content(err.to_string());
    if let Some(obj) = result.as_object_mut() {
        obj.insert("isError".into(), json!(true));
    }
    result
}

/// Append the truthful handshake-wait note as a final text block on an MCP tool result.
fn append_wait_note(result: &mut Value, waited: Duration) {
    let note = format!(
        "(waited {:.1}s for browser extension handshake)",
        waited.as_secs_f64()
    );
    if let Some(content) = result.get_mut("content").and_then(Value::as_array_mut) {
        content.push(json!({ "type": "text", "text": note }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::config::layers::{self, LayerInputs};
    use crate::governance::config::{Config, CONTENT_SECURITY_SACRED_DOMAINS};
    use crate::transport::native::host;
    use std::sync::Mutex;
    use std::time::Duration as StdDuration;

    fn temp_audit_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "browser-mcp-server-audit-test-{}-{tag}.jsonl",
            std::process::id()
        ))
    }

    fn read_lines(path: &std::path::Path) -> Vec<Value> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        content
            .lines()
            .map(|l| serde_json::from_str(l).expect("each line is a JSON object"))
            .collect()
    }

    fn assert_wellformed_event_id_and_ts(rec: &Value) {
        let event_id = rec["event_id"].as_str().expect("event_id is a string");
        assert_eq!(event_id.len(), 36, "event_id: {event_id}");
        for offset in [8, 13, 18, 23] {
            assert_eq!(event_id.as_bytes()[offset], b'-', "event_id: {event_id}");
        }
        let ts = rec["ts"].as_str().expect("ts is a string");
        assert_eq!(ts.len(), 24, "ts: {ts}");
        assert!(ts.ends_with('Z'), "ts: {ts}");
        chrono::DateTime::parse_from_rfc3339(ts).expect("ts parses as rfc3339");
    }

    /// A `Config` whose `content.security.sacred_domains` resolves to exactly `patterns`,
    /// everything else at its Minimal default. Built through the real layered resolver (not a
    /// hand-built `Config`) so validation runs exactly as it would in production.
    fn config_with_sacred_domains(patterns: &[&str]) -> Config {
        let inputs = LayerInputs {
            user: serde_json::Map::from_iter([(
                CONTENT_SECURITY_SACRED_DOMAINS.to_string(),
                json!(patterns),
            )]),
            ..Default::default()
        };
        Config::from_resolution(&layers::resolve(&inputs))
    }

    async fn wait_connected(browser: &Browser) {
        for _ in 0..200 {
            if browser.is_connected() {
                return;
            }
            tokio::time::sleep(StdDuration::from_millis(5)).await;
        }
        panic!("browser never reported connected");
    }

    /// Attach a fake extension over an in-memory duplex pipe (the same pattern
    /// `transport::executor`'s own tests use). Answers a `tool_request` for any tool name found
    /// in `responses` with that canned result and records the tool names seen, in arrival order,
    /// into the returned `Arc<Mutex<Vec<String>>>`. Panics if a `tool_request` arrives for a
    /// tool not in `responses` -- tests use this to prove a denied call never reaches the real
    /// tool. No `tab_url_request` support: g13's point-5 tests use
    /// [`attach_fake_extension_with_tab_urls`] instead.
    fn attach_fake_extension(
        browser: &Browser,
        responses: Vec<(&'static str, Value)>,
    ) -> (tokio::task::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
        attach_fake_extension_with_tab_urls(browser, responses, Vec::new())
    }

    /// Like [`attach_fake_extension`], plus a `tab_url_request` answer table (g13): `tab_urls`
    /// maps a `tabId` to the URL the fake extension reports for it (`None` for `url: null`, an
    /// unknown/closed tab). A `tab_url_request` for a `tabId` absent from the table panics, same
    /// posture as an unregistered `tool_request`. `seen` records a `"tab_url_request:<tabId>"`
    /// entry for each query, distinguishable from the tool names `tool_request` entries record.
    fn attach_fake_extension_with_tab_urls(
        browser: &Browser,
        responses: Vec<(&'static str, Value)>,
        tab_urls: Vec<(i64, Option<&'static str>)>,
    ) -> (tokio::task::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
        let (browser_side, mut ext_side) = tokio::io::duplex(64 * 1024);
        let attached = browser.clone();
        tokio::spawn(async move {
            let _ = attached.attach(browser_side).await;
        });

        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_for_task = Arc::clone(&seen);
        let responses: std::collections::HashMap<&'static str, Value> =
            responses.into_iter().collect();
        let tab_urls: std::collections::HashMap<i64, Option<&'static str>> =
            tab_urls.into_iter().collect();
        let handle = tokio::spawn(async move {
            loop {
                let Some(req) = host::read_message(&mut ext_side).await.unwrap() else {
                    break;
                };
                let v: Value = serde_json::from_slice(&req).unwrap();
                if v["type"] == "tab_url_request" {
                    let tab_id = v["tabId"]
                        .as_i64()
                        .expect("tab_url_request carries a tabId");
                    seen_for_task
                        .lock()
                        .unwrap()
                        .push(format!("tab_url_request:{tab_id}"));
                    let url = *tab_urls
                        .get(&tab_id)
                        .unwrap_or_else(|| panic!("unexpected tab_url_request for tabId {tab_id}"));
                    let reply = json!({ "id": v["id"], "type": "tab_url_response", "result": { "url": url } });
                    host::write_message(&mut ext_side, &serde_json::to_vec(&reply).unwrap())
                        .await
                        .unwrap();
                    continue;
                }
                let tool = v["tool"].as_str().unwrap().to_string();
                seen_for_task.lock().unwrap().push(tool.clone());
                let result = responses
                    .get(tool.as_str())
                    .cloned()
                    .unwrap_or_else(|| panic!("unexpected tool_request for '{tool}'"));
                let reply = json!({ "id": v["id"], "type": "tool_response", "result": result });
                host::write_message(&mut ext_side, &serde_json::to_vec(&reply).unwrap())
                    .await
                    .unwrap();
            }
        });
        (handle, seen)
    }

    /// A `tabs_context_mcp` reply reporting one tab at `url`, in the exact shape
    /// `resolve_tab_host` expects: a text content item whose text is the pretty/compact JSON of
    /// `{ "mcpGroupId": 1, "tabs": [...] }`.
    fn tabs_context_reply(tab_id: i64, url: &str) -> Value {
        let text = json!({
            "mcpGroupId": 1,
            "tabs": [{ "tabId": tab_id, "title": "", "url": url }],
        })
        .to_string();
        json!({ "content": [{ "type": "text", "text": text }] })
    }

    /// Test 6 (g08 spec section 6): a tab showing a sacred host denies every tool that carries
    /// its `tabId`, including `navigate` (navigating AWAY is denied too), and the extension
    /// never receives anything but the `tabs_context_mcp` pre-flight.
    #[tokio::test]
    async fn sacred_tab_denies_every_tool_and_never_runs_it() {
        let path = temp_audit_path("sacred-tab");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store = crate::governance::config::reload::ConfigStore::for_test_with_config(
            config_with_sacred_domains(&["*.mybank.com"]),
        );
        let browser = Browser::new();
        let (_ext, seen) = attach_fake_extension(
            &browser,
            vec![(
                "tabs_context_mcp",
                tabs_context_reply(5, "https://www.mybank.com/account"),
            )],
        );
        wait_connected(&browser).await;

        let cases = [
            ("read_page", json!({ "tabId": 5 })),
            ("computer", json!({ "action": "screenshot", "tabId": 5 })),
            (
                "javascript_tool",
                json!({ "action": "javascript_exec", "text": "1", "tabId": 5 }),
            ),
            (
                "navigate",
                json!({ "url": "https://example.com", "tabId": 5 }),
            ),
        ];
        for (tool, args) in cases {
            let params = json!({ "name": tool, "arguments": args });
            let resp =
                handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params))
                    .await;
            let text = resp.result.as_ref().expect("tool result present")["content"][0]["text"]
                .as_str()
                .expect("text content block");
            assert!(
                text.starts_with("Denied (D-af6633ec)"),
                "{tool}: unexpected text: {text}"
            );
            assert!(text.contains("www.mybank.com"), "{tool}: {text}");
        }

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 4, "exactly one deny record per denied call");
        for rec in &lines {
            assert_eq!(rec["decision"], "deny");
            assert_eq!(rec["denial_id"], "D-af6633ec");
            assert_eq!(rec["domain"], "www.mybank.com");
        }
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["tabs_context_mcp"; 4],
            "the extension must never see anything but the tabs_context_mcp pre-flight"
        );

        std::fs::remove_file(&path).ok();
    }

    /// Test 7 (g08 spec section 6): a `navigate` target matching a sacred pattern is denied
    /// even when the current tab is clean; a target that does not match is allowed.
    #[tokio::test]
    async fn navigate_target_denied_even_when_tab_is_clean() {
        let recorder = Arc::new(Recorder::disabled());
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store = crate::governance::config::reload::ConfigStore::for_test_with_config(
            config_with_sacred_domains(&["mybank.com"]),
        );
        let browser = Browser::new();
        let (_ext, _seen) = attach_fake_extension(
            &browser,
            vec![
                (
                    "tabs_context_mcp",
                    tabs_context_reply(5, "https://example.com/"),
                ),
                (
                    "navigate",
                    json!({ "content": [{ "type": "text", "text": "navigated" }] }),
                ),
            ],
        );
        wait_connected(&browser).await;

        let denied_params = json!({
            "name": "navigate",
            "arguments": { "url": "mybank.com", "tabId": 5 },
        });
        let denied = handle_tools_call(
            &browser,
            &store,
            &governance,
            Some(json!(1)),
            Some(&denied_params),
        )
        .await;
        let denied_text = denied.result.as_ref().expect("tool result present")["content"][0]
            ["text"]
            .as_str()
            .expect("text content block");
        assert!(
            denied_text.starts_with("Denied (D-171052e3)"),
            "{denied_text}"
        );
        assert!(denied_text.contains("mybank.com"));

        let allowed_params = json!({
            "name": "navigate",
            "arguments": { "url": "https://example.org", "tabId": 5 },
        });
        let allowed = handle_tools_call(
            &browser,
            &store,
            &governance,
            Some(json!(2)),
            Some(&allowed_params),
        )
        .await;
        let allowed_text = allowed.result.as_ref().expect("tool result present")["content"][0]
            ["text"]
            .as_str()
            .expect("text content block");
        assert_eq!(allowed_text, "navigated");
    }

    /// Test 8 (g08 spec section 6): with the default (empty) sacred list, a call reaches the
    /// fake extension directly -- no `tabs_context_mcp` pre-flight ever -- and an unconnected
    /// browser still resolves the sacred check without any browser access.
    #[tokio::test]
    async fn empty_list_is_byte_identical() {
        let recorder = Arc::new(Recorder::disabled());
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        assert!(store.current().sacred_domains().is_empty());

        let browser = Browser::new();
        let (_ext, seen) = attach_fake_extension(
            &browser,
            vec![(
                "read_page",
                json!({ "content": [{ "type": "text", "text": "page text" }] }),
            )],
        );
        wait_connected(&browser).await;

        let params = json!({ "name": "read_page", "arguments": { "tabId": 5 } });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;
        let text = resp.result.as_ref().expect("tool result present")["content"][0]["text"]
            .as_str()
            .expect("text content block");
        assert_eq!(text, "page text");
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["read_page"],
            "no tabs_context_mcp pre-flight ever, with an empty list"
        );

        // Allow resolves without touching the browser at all: an unconnected Browser still
        // reaches the ordinary not-connected error, never a sacred pre-flight attempt.
        let unconnected = Browser::new();
        let params2 = json!({ "name": "navigate", "arguments": {} });
        let resp2 = handle_tools_call(
            &unconnected,
            &store,
            &governance,
            Some(json!(2)),
            Some(&params2),
        )
        .await;
        let text2 = resp2.result.as_ref().expect("tool result present")["content"][0]["text"]
            .as_str()
            .expect("text content block");
        assert!(text2.contains("not connected"), "{text2}");
    }

    /// Test 9 (g08 spec section 6): a denied call writes exactly one audit record, and the
    /// internal `tabs_context_mcp` lookup writes none.
    #[tokio::test]
    async fn denied_call_writes_one_deny_record() {
        let path = temp_audit_path("deny-record");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store = crate::governance::config::reload::ConfigStore::for_test_with_config(
            config_with_sacred_domains(&["*.mybank.com"]),
        );
        let browser = Browser::new();
        let (_ext, _seen) = attach_fake_extension(
            &browser,
            vec![(
                "tabs_context_mcp",
                tabs_context_reply(5, "https://www.mybank.com/account"),
            )],
        );
        wait_connected(&browser).await;

        let params = json!({ "name": "read_page", "arguments": { "tabId": 5 } });
        let _ =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;

        let lines = read_lines(&path);
        assert_eq!(
            lines.len(),
            1,
            "exactly one record: the tabs_context_mcp lookup writes none"
        );
        let rec = &lines[0];
        assert_eq!(rec["decision"], "deny");
        let denial_id = rec["denial_id"].as_str().expect("denial_id is a string");
        assert!(
            denial_id.starts_with("D-") && denial_id.len() == 10,
            "{denial_id}"
        );
        assert_eq!(rec["grant_id"], Value::Null);
        assert_eq!(rec["duration_ms"], 0);
        assert_eq!(rec["domain"], "www.mybank.com");

        std::fs::remove_file(&path).ok();
    }

    /// Test 10 (g06 spec section 6, adapted to the post-A3/A5 architecture): drives the real
    /// `handle_line` dispatch for `initialize` (proving `capture_client_info` is wired at the
    /// real chokepoint, not just callable in isolation) and `handle_tools_call` for a
    /// `navigate` call, then asserts the resulting audit line end to end.
    #[tokio::test]
    async fn tools_call_produces_one_audit_record_with_client_identity() {
        let path = temp_audit_path("basic");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();
        let (tx, _rx) = mpsc::unbounded_channel::<JsonRpcResponse>();

        let init_line = json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "clientInfo": { "name": "test-client", "version": "9.9.9" } },
        })
        .to_string();
        handle_line(&browser, &store, &governance, &init_line, &tx).await;

        let params = json!({ "name": "navigate", "arguments": {} });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(2)), Some(&params)).await;
        let text = resp.result.as_ref().expect("tool result present")["content"][0]["text"]
            .as_str()
            .expect("text content block")
            .to_string();
        assert!(text.contains("not connected"), "unexpected text: {text}");

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 1, "exactly one audit record");
        let rec = &lines[0];
        assert_eq!(rec["tool"], "navigate");
        assert!(rec["action"].is_null());
        assert_eq!(rec["rw"], "observe");
        assert_eq!(rec["decision"], "allow");
        assert_eq!(rec["client"]["name"], "test-client");
        assert_eq!(rec["client"]["version"], "9.9.9");
        for field in ["identity", "domain", "grant_id", "denial_id", "manifest"] {
            assert!(rec[field].is_null(), "{field} must be null");
        }
        assert_wellformed_event_id_and_ts(rec);

        std::fs::remove_file(&path).ok();
    }

    /// Test 11: a `computer` call with `action: "screenshot"` records that action and the
    /// observe class.
    #[tokio::test]
    async fn computer_call_records_action_and_observe_class() {
        let path = temp_audit_path("computer");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();

        let params = json!({ "name": "computer", "arguments": { "action": "screenshot" } });
        let _ =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 1, "exactly one audit record");
        assert_eq!(lines[0]["action"], "screenshot");
        assert_eq!(lines[0]["rw"], "observe");

        std::fs::remove_file(&path).ok();
    }

    /// Test 12: a `tools/call` whose params lack `name` returns the `-32602` error and never
    /// reaches the dispatch chokepoint, so no audit file is created.
    #[tokio::test]
    async fn invalid_tools_call_without_name_records_nothing() {
        let path = temp_audit_path("no-name");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();

        let params = json!({ "arguments": {} });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;
        assert_eq!(resp.error.as_ref().expect("error present")["code"], -32602);
        assert!(!path.exists(), "no audit file must be created");
    }

    /// Test 4 (g10 spec section 6): a held `Browser` with NO extension connected returns the
    /// `Paused:` text as a successful result (never `isError`), proving the hold check
    /// precedes the "extension not connected" failure path; with the hold released, the
    /// existing `isError` result is unchanged.
    #[tokio::test]
    async fn held_call_returns_the_pause_text_before_the_not_connected_error() {
        let recorder = Arc::new(Recorder::disabled());
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();
        browser.set_held(true);

        let params = json!({ "name": "computer", "arguments": { "action": "screenshot" } });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;
        assert!(resp.error.is_none(), "a held reply is a JSON-RPC success");
        let result = resp.result.as_ref().expect("tool result present");
        assert_ne!(
            result["isError"], true,
            "a held reply must never be isError"
        );
        let text = result["content"][0]["text"].as_str().expect("text block");
        assert!(text.starts_with("Paused:"), "{text}");
        assert!(text.contains("'computer (screenshot)' call"), "{text}");

        browser.set_held(false);
        let resp2 =
            handle_tools_call(&browser, &store, &governance, Some(json!(2)), Some(&params)).await;
        let result2 = resp2.result.as_ref().expect("tool result present");
        assert_eq!(
            result2["isError"], true,
            "with hold released, the not-connected path returns"
        );
        let text2 = result2["content"][0]["text"].as_str().expect("text block");
        assert!(text2.contains("not connected"), "{text2}");
    }

    /// Test 6 (g10 spec section 6): a held call writes one audit record with
    /// `decision: "allow"`, `held: true`, `duration_ms: 0`; a normal allowed call writes
    /// `held: false`.
    #[tokio::test]
    async fn held_call_marks_the_audit_record_and_normal_calls_do_not() {
        let path = temp_audit_path("held");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(Governance::all_open(
            recorder as Arc<dyn AuditSink>,
            classify::classify,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();

        browser.set_held(true);
        let held_params = json!({ "name": "navigate", "arguments": {} });
        let _ = handle_tools_call(
            &browser,
            &store,
            &governance,
            Some(json!(1)),
            Some(&held_params),
        )
        .await;

        browser.set_held(false);
        let allowed_params = json!({ "name": "navigate", "arguments": {} });
        let _ = handle_tools_call(
            &browser,
            &store,
            &governance,
            Some(json!(2)),
            Some(&allowed_params),
        )
        .await;

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["decision"], "allow");
        assert_eq!(lines[0]["held"], true);
        assert_eq!(lines[0]["duration_ms"], 0);
        assert_eq!(lines[1]["held"], false);

        std::fs::remove_file(&path).ok();
    }

    // --- g13: grant enforcement, point 5 (navigate final-landing check) ---
    //
    // Every other g13 scenario (pre-dispatch domain/access/scheme/union-rule denials, the
    // all-open invariant, denial-id determinism) is covered end to end by the black-box
    // subprocess tests in `tests/tool_enforcement.rs`, which deliberately run with no extension
    // connected at all. Point 5 needs a dispatched `navigate` to actually succeed and then be
    // re-queried, which requires a connected (fake) extension; that is only practical here,
    // inline, using the same fake-extension pattern g08's sacred-domain tests above already
    // established.

    use crate::governance::enforcement::LocalPdp;
    use crate::governance::manifest::document::{Access, Grant};

    fn full_grant(id: &str, domains: &[&str]) -> Grant {
        Grant {
            id: id.to_string(),
            domains: domains.iter().map(|d| d.to_string()).collect(),
            access: Access::All,
            tools: None,
            exclude_tools: None,
            description: None,
            mode: None,
        }
    }

    fn governed_with_grants(grants: Vec<Grant>, sink: Arc<dyn AuditSink>) -> Governance {
        governed_with_grants_and_mode(grants, sink, None)
    }

    fn governed_with_grants_and_mode(
        grants: Vec<Grant>,
        sink: Arc<dyn AuditSink>,
        manifest_mode: Option<crate::governance::ports::EffectiveMode>,
    ) -> Governance {
        Governance::governed(
            Box::new(LocalPdp::new(pattern::pattern_matches_normalized_host)),
            sink,
            classify::classify,
            grants,
            "test-hash".to_string(),
            manifest_mode,
        )
    }

    /// A landing that stays on-grant: the navigate result passes through unchanged, no denial.
    #[tokio::test]
    async fn point5_navigate_landing_on_grant_passes_through() {
        let recorder = Arc::new(Recorder::disabled());
        let governance = Arc::new(governed_with_grants(
            vec![full_grant("g1", &["example.com"])],
            recorder as Arc<dyn AuditSink>,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();
        let (_ext, seen) = attach_fake_extension_with_tab_urls(
            &browser,
            vec![(
                "navigate",
                json!({ "content": [{ "type": "text", "text": "navigated" }] }),
            )],
            vec![(5, Some("https://example.com/"))],
        );
        wait_connected(&browser).await;

        let params = json!({
            "name": "navigate",
            "arguments": { "url": "https://example.com/", "tabId": 5 },
        });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;
        let text = resp.result.as_ref().expect("tool result present")["content"][0]["text"]
            .as_str()
            .expect("text content block");
        assert_eq!(text, "navigated");
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["navigate", "tab_url_request:5"],
            "one dispatch, one point-5 re-query, no park"
        );
    }

    /// A landing that drifts off-grant (e.g. a redirect): the tab is best-effort parked on
    /// `about:blank`, the navigate result is replaced with a denial naming the FINAL host, and
    /// the audit record is a deny with the real elapsed duration (not the pre-dispatch `0`).
    #[tokio::test]
    async fn point5_navigate_landing_off_grant_parks_and_denies() {
        let path = temp_audit_path("point5-deny");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(governed_with_grants(
            vec![full_grant("g1", &["example.com"])],
            recorder as Arc<dyn AuditSink>,
        ));
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());
        let browser = Browser::new();
        let (_ext, seen) = attach_fake_extension_with_tab_urls(
            &browser,
            vec![(
                "navigate",
                json!({ "content": [{ "type": "text", "text": "navigated" }] }),
            )],
            vec![(5, Some("https://evil.com/"))],
        );
        wait_connected(&browser).await;

        let params = json!({
            "name": "navigate",
            "arguments": { "url": "https://example.com/", "tabId": 5 },
        });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;
        let text = resp.result.as_ref().expect("tool result present")["content"][0]["text"]
            .as_str()
            .expect("text content block");
        assert!(text.starts_with("Denied (D-"), "{text}");
        assert!(text.contains("evil.com"), "{text}");
        assert_eq!(
            *seen.lock().unwrap(),
            vec!["navigate", "tab_url_request:5", "navigate"],
            "the original dispatch, the point-5 re-query, then the best-effort park"
        );

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 1, "exactly one record for this call");
        assert_eq!(lines[0]["decision"], "deny");
        assert_eq!(lines[0]["domain"], "evil.com");
        assert_eq!(lines[0]["grant_id"], Value::Null);
        assert!(
            lines[0]["duration_ms"].as_u64().is_some(),
            "duration_ms present: {:?}",
            lines[0]["duration_ms"]
        );

        std::fs::remove_file(&path).ok();
    }

    /// g15 constraint 9 (the sacred carve-out): a sacred-domain denial is ALWAYS a real
    /// `Deny`, never `ShadowDeny`, even when the active manifest's own mode is `observe`.
    /// Sacred denials never pass through `Decision`/`check_call` at all (a separate, always-on
    /// code path at the dispatch chokepoint, ahead of grant evaluation); this test pins the
    /// observable end-to-end behavior rather than relying on that structural fact alone.
    #[tokio::test]
    async fn sacred_domain_denies_even_under_an_observe_mode_manifest() {
        let path = temp_audit_path("sacred-under-observe");
        let _ = std::fs::remove_file(&path);
        let recorder = Arc::new(Recorder::to_file(path.clone()));
        let governance = Arc::new(governed_with_grants_and_mode(
            vec![full_grant("g1", &["www.mybank.com"])],
            recorder as Arc<dyn AuditSink>,
            Some(crate::governance::ports::EffectiveMode::Observe),
        ));
        let store = crate::governance::config::reload::ConfigStore::for_test_with_config(
            config_with_sacred_domains(&["*.mybank.com"]),
        );
        let browser = Browser::new();
        let (_ext, _seen) = attach_fake_extension(
            &browser,
            vec![(
                "tabs_context_mcp",
                tabs_context_reply(5, "https://www.mybank.com/account"),
            )],
        );
        wait_connected(&browser).await;

        let params = json!({ "name": "read_page", "arguments": { "tabId": 5 } });
        let resp =
            handle_tools_call(&browser, &store, &governance, Some(json!(1)), Some(&params)).await;
        let text = resp.result.as_ref().expect("tool result present")["content"][0]["text"]
            .as_str()
            .expect("text content block");
        assert!(text.starts_with("Denied (D-"), "{text}");

        let lines = read_lines(&path);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0]["decision"], "deny",
            "a sacred denial is never shadow_deny, even under an observe-mode manifest"
        );

        std::fs::remove_file(&path).ok();
    }

    /// g15 required test 3/5 (non-sacred mode switch, inline variant): the SAME grant-based
    /// would-deny call, under an enforcing vs an observing manifest, yields `deny` (tool did
    /// not run) vs `shadow_deny` (tool ran, ordinary result, no `Denied (` text) with the
    /// IDENTICAL `grant_id`/`denial_id`. The subprocess-level equivalent
    /// (`tests/shadow_mode.rs`) additionally proves `duration_ms` truthfully differs (`0` vs
    /// real elapsed) using the real dispatch path with no extension connected; this inline
    /// version uses a fake extension so the observe-mode call can actually "execute". The
    /// would-deny call is `tabs_create_mcp` (domain-less, denied via the union rule) since s01
    /// reclassified `navigate` observe (ADR-0022): `navigate` is no longer deniable by a
    /// read-only grant on its own covered domain.
    #[tokio::test]
    async fn grant_shadow_deny_runs_the_tool_and_matches_the_enforce_denial_id() {
        let enforce_path = temp_audit_path("shadow-enforce");
        let observe_path = temp_audit_path("shadow-observe");
        let _ = std::fs::remove_file(&enforce_path);
        let _ = std::fs::remove_file(&observe_path);

        fn read_only_grant() -> Grant {
            let mut g = full_grant("r", &["example.com"]);
            g.access = crate::governance::manifest::document::Access::Read;
            g
        }
        let store =
            crate::governance::config::reload::ConfigStore::for_test_with_config(Config::minimal());

        // Enforce: the mutate-class call on a read-only grant is blocked outright.
        let enforce_recorder = Arc::new(Recorder::to_file(enforce_path.clone()));
        let enforce_governance = Arc::new(governed_with_grants_and_mode(
            vec![read_only_grant()],
            enforce_recorder as Arc<dyn AuditSink>,
            Some(crate::governance::ports::EffectiveMode::Enforce),
        ));
        let browser = Browser::new();
        let params = json!({ "name": "tabs_create_mcp", "arguments": {} });
        let enforce_resp = handle_tools_call(
            &browser,
            &store,
            &enforce_governance,
            Some(json!(1)),
            Some(&params),
        )
        .await;
        let enforce_text = enforce_resp.result.as_ref().expect("result")["content"][0]["text"]
            .as_str()
            .expect("text");
        assert!(enforce_text.starts_with("Denied (D-"), "{enforce_text}");
        let enforce_lines = read_lines(&enforce_path);
        assert_eq!(enforce_lines.len(), 1);
        assert_eq!(enforce_lines[0]["decision"], "deny");
        assert_eq!(enforce_lines[0]["duration_ms"], 0);

        // Observe: the identical call now dispatches (a fake extension answers it) and the
        // response carries no denial text at all.
        let observe_recorder = Arc::new(Recorder::to_file(observe_path.clone()));
        let observe_governance = Arc::new(governed_with_grants_and_mode(
            vec![read_only_grant()],
            observe_recorder as Arc<dyn AuditSink>,
            Some(crate::governance::ports::EffectiveMode::Observe),
        ));
        let observe_browser = Browser::new();
        let (_ext, _seen) = attach_fake_extension(
            &observe_browser,
            vec![(
                "tabs_create_mcp",
                json!({ "content": [{ "type": "text", "text": "created" }] }),
            )],
        );
        wait_connected(&observe_browser).await;
        let observe_resp = handle_tools_call(
            &observe_browser,
            &store,
            &observe_governance,
            Some(json!(1)),
            Some(&params),
        )
        .await;
        let observe_text = observe_resp.result.as_ref().expect("result")["content"][0]["text"]
            .as_str()
            .expect("text");
        assert_eq!(
            observe_text, "created",
            "shadow mode returns the ordinary tool result, no denial text: {observe_text}"
        );
        let observe_lines = read_lines(&observe_path);
        assert_eq!(observe_lines.len(), 1);
        assert_eq!(observe_lines[0]["decision"], "shadow_deny");
        assert!(
            observe_lines[0]["duration_ms"].as_u64().is_some(),
            "duration_ms present (a shadow-denied call ran, unlike an enforce deny's fixed 0): {:?}",
            observe_lines[0]["duration_ms"]
        );

        assert_eq!(
            enforce_lines[0]["grant_id"], observe_lines[0]["grant_id"],
            "enforce and observe must attribute the same grant"
        );
        assert_eq!(
            enforce_lines[0]["denial_id"], observe_lines[0]["denial_id"],
            "enforce and observe must derive the identical denial id"
        );

        std::fs::remove_file(&enforce_path).ok();
        std::fs::remove_file(&observe_path).ok();
    }
}
