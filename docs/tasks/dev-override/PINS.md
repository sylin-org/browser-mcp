# dev-override batch -- PINS (the oracles)

Computed by the author against dev @ 3928a74. Executors TRANSCRIBE these; never re-derive an
expectation (an executor-derived expectation validates its own bugs). Anchors quote current tree
text; re-locate by anchor, not line number. Semantics live in ADR-0048; this file pins shapes,
names, strings, signatures, and test assertions only.

## P1 -- Selection + candidates + agent resolution (T1; ADR-0048 D1/D2/D3)

### crates/transport/src/instance.rs (append after the `impl Instance` block, before `mod tests`)

```rust
/// The reserved development-override instance name (ADR-0048 D1): when an ADAPTER is unpinned,
/// a live `dev` instance shadows the default.
pub const DEV_INSTANCE: &str = "dev";

/// An adapter's instance selection (ADR-0048 D2). Only ADAPTERS have an [`Selection::Unpinned`]
/// state (connect-time resolution, dev first); the service and the installer always operate on
/// exactly one instance via [`Instance::resolve`], where an absent name IS the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Explicitly bound to one instance (a named one, or the reserved word `default`): no
    /// override, connect to exactly this instance.
    Pinned(Instance),
    /// No instance named anywhere: resolve at connect time, preferring a live dev instance.
    Unpinned,
}

impl Selection {
    /// Classify one raw instance source (a `--instance` value or the env var's content) into a
    /// selection (ADR-0048 D2). Pure (no environment access), so it is unit-testable without
    /// racing parallel tests over process-global env state: `None`/blank is Unpinned; the
    /// reserved word `default` (any case) pins the DEFAULT instance; a valid name pins that
    /// named instance; anything else returns the validation error verbatim.
    fn classify(source: Option<&str>) -> std::result::Result<Self, String> {
        match source.map(str::trim) {
            None | Some("") => Ok(Selection::Unpinned),
            Some(s) if s.eq_ignore_ascii_case("default") => {
                Ok(Selection::Pinned(Instance::default()))
            }
            Some(s) => Instance::from_name(s).map(Selection::Pinned),
        }
    }

    /// Resolve an adapter's selection from an optional `--instance` flag value, falling back to
    /// [`Instance::ENV_VAR`] (ADR-0048 D2; a blank flag value is treated as absent), and
    /// NORMALIZE the environment so every downstream point-of-use `Instance::resolve()` agrees:
    /// a pinned NAMED instance writes its name back; pinned-default and unpinned REMOVE the
    /// variable (both leave downstream derivations on the default identity -- an unpinned
    /// adapter's own logs live under the default dirs, ADR-0048 D8).
    pub fn resolve_from(flag: Option<&str>) -> std::result::Result<Self, String> {
        let env = std::env::var(Instance::ENV_VAR).ok();
        let source = match flag.map(str::trim) {
            Some(f) if !f.is_empty() => Some(f.to_string()),
            _ => env,
        };
        let selection = Self::classify(source.as_deref())?;
        match &selection {
            Selection::Pinned(i) if !i.is_default() => {
                std::env::set_var(Instance::ENV_VAR, i.name().expect("a named instance"));
            }
            _ => std::env::remove_var(Instance::ENV_VAR),
        }
        Ok(selection)
    }

    /// The connect-order instance candidates (ADR-0048 D1/D3): pinned names exactly its
    /// instance; unpinned tries the dev override first, then the default.
    pub fn candidates(&self) -> Vec<Instance> {
        match self {
            Selection::Pinned(i) => vec![i.clone()],
            Selection::Unpinned => vec![
                Instance::from_name(DEV_INSTANCE).expect("'dev' is a valid instance name"),
                Instance::default(),
            ],
        }
    }
}
```

New tests appended to instance.rs's existing `mod tests` (classify/candidates are pure -- no env
access, no race):

```rust
    /// ADR-0048 D2: the three selection states, from one raw source string.
    #[test]
    fn selection_classify_maps_the_three_states() {
        assert_eq!(Selection::classify(None).unwrap(), Selection::Unpinned);
        assert_eq!(Selection::classify(Some("")).unwrap(), Selection::Unpinned);
        assert_eq!(Selection::classify(Some("  ")).unwrap(), Selection::Unpinned);
        assert_eq!(
            Selection::classify(Some("default")).unwrap(),
            Selection::Pinned(Instance::default())
        );
        assert_eq!(
            Selection::classify(Some("DEFAULT")).unwrap(),
            Selection::Pinned(Instance::default())
        );
        assert_eq!(
            Selection::classify(Some("dev")).unwrap(),
            Selection::Pinned(Instance::from_name("dev").unwrap())
        );
        assert!(Selection::classify(Some("Not Valid")).is_err());
    }

    /// ADR-0048 D1: unpinned candidate order is dev, then default; pinned is exactly one.
    #[test]
    fn unpinned_candidates_are_dev_then_default() {
        let c = Selection::Unpinned.candidates();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].name(), Some("dev"));
        assert!(c[1].is_default());
        let p = Selection::Pinned(Instance::from_name("qa").unwrap()).candidates();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name(), Some("qa"));
    }
```

### crates/transport/src/ipc.rs -- candidate endpoints

Add directly below the existing `default_endpoint` fn (anchor:
`pub fn default_endpoint() -> String {`), which itself stays UNCHANGED (the service and doctor
keep using it):

```rust
/// The ordered MAIN-endpoint candidates an adapter dials (ADR-0048 D2/D3), pure core: the
/// single-endpoint override wins, then the list override, then the selection's instances. Split
/// from [`endpoint_candidates`] so it is unit-testable without racing parallel tests over
/// process-global env state.
fn candidates_from(
    single: Option<&str>,
    list: Option<&str>,
    selection: &crate::instance::Selection,
) -> Vec<String> {
    if let Some(ep) = single.map(str::trim).filter(|s| !s.is_empty()) {
        return vec![ep.to_string()];
    }
    if let Some(raw) = list {
        let eps: Vec<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !eps.is_empty() {
            return eps;
        }
    }
    selection
        .candidates()
        .iter()
        .map(crate::instance::Instance::endpoint)
        .collect()
}

/// The ordered endpoint candidates for `selection` (ADR-0048 D2/D3): `GHOSTLIGHT_ENDPOINT` (one
/// pinned endpoint; tests and advanced deployments) wins, then `GHOSTLIGHT_ENDPOINTS` (a
/// comma-separated pinned candidate LIST -- the override integration tests' seam), then the
/// selection's instances' endpoints (`[dev, default]` when unpinned, exactly one when pinned).
pub fn endpoint_candidates(selection: &crate::instance::Selection) -> Vec<String> {
    candidates_from(
        std::env::var("GHOSTLIGHT_ENDPOINT").ok().as_deref(),
        std::env::var("GHOSTLIGHT_ENDPOINTS").ok().as_deref(),
        selection,
    )
}
```

New unit test in ipc.rs's tests module:

```rust
    /// ADR-0048 D2: candidate precedence -- the single override, the list override, then the
    /// selection's instances (dev first when unpinned). Pure: no env access.
    #[test]
    fn candidates_from_honors_the_precedence_order() {
        use crate::instance::{Instance, Selection};
        let unpinned = Selection::Unpinned;
        assert_eq!(
            candidates_from(Some("ep-one"), Some("a,b"), &unpinned),
            vec!["ep-one".to_string()]
        );
        assert_eq!(
            candidates_from(None, Some(" a , b ,,"), &unpinned),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(
            candidates_from(None, None, &unpinned),
            vec![
                "org.sylin.ghostlight.dev.v1".to_string(),
                "org.sylin.ghostlight.v1".to_string()
            ]
        );
        let pinned = Selection::Pinned(Instance::from_name("qa").unwrap());
        assert_eq!(
            candidates_from(None, None, &pinned),
            vec!["org.sylin.ghostlight.qa.v1".to_string()]
        );
        // Blank overrides fall through rather than pinning an empty endpoint.
        assert_eq!(
            candidates_from(Some("  "), None, &pinned),
            vec!["org.sylin.ghostlight.qa.v1".to_string()]
        );
    }
```

### crates/transport/src/ipc.rs -- relay_adapter goes multi-candidate

- Signature (anchor: `pub async fn relay_adapter(endpoint: &str, debug:`):

```rust
pub async fn relay_adapter(
    endpoints: &[String],
    debug: &crate::observability::DebugSink,
) -> Result<()> {
```

  APPEND to its doc comment (never edit the existing ADR-0047 D2 sentences) a final paragraph:

```
/// ADR-0048 D3: `endpoints` is the ORDERED main-endpoint candidate list (exactly one when
/// pinned; `[dev, default]` when unpinned). Every connect episode -- the first connect and each
/// reconnect tick -- walks the list in order, so a live dev instance shadows the default and a
/// dead one fails over to it at reconnect speed.
```

- Body: replace `let adapter_endpoint = adapter_endpoint_name(endpoint);` with

```rust
    let adapter_endpoints: Vec<String> =
        endpoints.iter().map(|e| adapter_endpoint_name(e)).collect();
```

- Replace the connect line
  `let stream = connect_and_handshake(&adapter_endpoint, !first, &session_guid).await?;` with

```rust
        let (stream, which) = connect_and_handshake(&adapter_endpoints, !first, &session_guid).await?;
```

- Immediately AFTER the existing first/reconnect `if first { ... } else { ... }` debug-note
  block, insert:

```rust
        if adapter_endpoints.len() > 1 {
            debug.ipc_note(&format!(
                "override resolution: connected to candidate {}/{}",
                which + 1,
                adapter_endpoints.len()
            ));
        }
```

### crates/transport/src/ipc.rs -- connect_and_handshake walks candidates

Replace the whole fn (anchor: `async fn connect_and_handshake(`) body and signature; the doc
comment KEEPS its existing paragraphs and gains one sentence at the end: `Walks the ordered
candidate list on every attempt (ADR-0048 D3) and returns the winning candidate's index alongside
the stream.`

```rust
async fn connect_and_handshake(
    adapter_endpoints: &[String],
    reconnect: bool,
    guid: &crate::session_guid::SessionGuid,
) -> Result<(
    impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    usize,
)> {
    debug_assert!(!adapter_endpoints.is_empty());
    let mut last_err: Option<Error> = None;
    for (which, ep) in adapter_endpoints.iter().enumerate() {
        match try_connect_once(ep, guid).await {
            Ok(stream) => return Ok((stream, which)),
            Err(e) => last_err = Some(e),
        }
    }
    crate::supervisor::start_service();
    // Reconnect patience (ADR-0045 amendment): the FIRST connect stays fail-fast (3s) so a
    // misconfigured install errors quickly; a RECONNECT episode is patient (120s) so a
    // rebuild-length service gap or a prod crash/upgrade never forces a client reload.
    let (interval, window) = if reconnect {
        (RECONNECT_RETRY_INTERVAL, RECONNECT_RETRY_WINDOW)
    } else {
        (
            crate::supervisor::SELF_HEAL_RETRY_INTERVAL,
            crate::supervisor::SELF_HEAL_RETRY_WINDOW,
        )
    };
    let deadline = tokio::time::Instant::now() + window;
    loop {
        sleep(interval).await;
        for (which, ep) in adapter_endpoints.iter().enumerate() {
            match try_connect_once(ep, guid).await {
                Ok(stream) => return Ok((stream, which)),
                Err(e) => last_err = Some(e),
            }
        }
        if tokio::time::Instant::now() >= deadline {
            tracing::error!("{}", crate::supervisor::SELF_HEAL_FAILURE_MESSAGE);
            return Err(last_err.expect("at least one candidate was tried"));
        }
    }
}
```

(`try_connect_once`, the hello shape, `verify_service_proof`, `dial_once`, and every retry
constant are UNCHANGED.)

### crates/adapter-agent/src/main.rs

- Import line `use ghostlight_transport::instance::Instance;` becomes
  `use ghostlight_transport::instance::Selection;`.
- In `main`, replace `resolve_instance();` with `let selection = resolve_selection();` and
  replace `let endpoint = ipc::default_endpoint();` with
  `let endpoints = ipc::endpoint_candidates(&selection);`, and the block_on line with
  `let code = rt.block_on(relay_with_watchdog(&endpoints, block_sink, parent));`.
- DELETE `fn resolve_instance()` entirely; ADD in its place:

```rust
/// Resolve the adapter's instance SELECTION (ADR-0048 D2): `--instance <name>` /
/// `--instance=<name>` wins over `GHOSTLIGHT_INSTANCE`; the reserved word `default` pins the
/// default instance (no override); NOTHING pins nothing -- the adapter resolves at connect time,
/// preferring a live dev instance (ADR-0048 D1). An invalid name is fatal: print the validation
/// error and exit 2.
fn resolve_selection() -> Selection {
    match Selection::resolve_from(instance_flag_value().as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ghostlight-adapter-agent: {e}");
            std::process::exit(2);
        }
    }
}
```

- `instance_flag_value()` stays byte-identical.
- `relay_with_watchdog` signature becomes
  `async fn relay_with_watchdog(endpoints: &[String], debug_sink: DebugSink, parent: Option<ProcId>) -> i32`
  and its relay arm becomes `result = ipc::relay_adapter(endpoints, &debug_sink) => {`.

### tests/hub_identity.rs (one call site)

Anchor: `let _ = ghostlight::native::ipc::relay_adapter(&relay_endpoint, &debug).await;`
inside the `tokio::spawn(async move { ... })`. Becomes:

```rust
    tokio::spawn(async move {
        let eps = [relay_endpoint];
        let _ = ghostlight::native::ipc::relay_adapter(&eps, &debug).await;
    });
```

### tests/adapter_override.rs (NEW integration test)

Helper block: TRANSCRIBE from `tests/adapter_reconnect.rs` (the tree is the oracle for these),
FIRST its entire `use` block VERBATIM (the seven lines from `use serde_json::{json, Value};`
through `use std::time::{Duration, Instant};` -- every import is used by the transcribed
helpers), THEN the items `static SEQ`, `fn bin`, `fn adapter_bin`, `fn service_cmd`,
`fn wait_for_state`, `fn send`, `fn recv`, and the adapter-stdout reader-thread pattern used by
`adapter_reconnects_across_a_service_restart_without_a_client_reload` (the
`std::sync::mpsc::channel` + `std::thread::spawn` + `BufReader::new(stdout).lines()` forwarding
loop -- it is INLINE there; extract it here as the pinned `spawn_reader` below), with EXACTLY
these deltas and additions:

0. `fn service_cmd` delta: its `.arg("service")` becomes `.args(["service", "--keep-warm"])`.
   (Idle grace is 30s; without `--keep-warm` a sessionless service B can exit before the
   failover reaches it. DEV-LOOP.md documents the flag.)

1. `fn unique()` returns a five-tuple:

```rust
/// A fresh (endpoint_a, endpoint_b, instance_a, instance_b, log_dir) set for one test run.
fn unique() -> (String, String, String, String, PathBuf) {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let tag = format!("{}-{n}", std::process::id());
    (
        format!("ghostlight-override-a-{tag}"),
        format!("ghostlight-override-b-{tag}"),
        format!("ovra{n}"),
        format!("ovrb{n}"),
        std::env::temp_dir().join(format!("ghostlight-override-log-{tag}")),
    )
}
```

2. `fn spawn_adapter` takes the candidate LIST plus a per-run instance name whose ONLY job is to
   point the self-heal supervisor kick at a guaranteed-nonexistent unit (the same isolation
   trick adapter_reconnect.rs documents). `GHOSTLIGHT_ENDPOINTS` outranks the pinned selection
   in `candidates_from`, so the multi-candidate WALK is still exercised end-to-end; the
   `Selection::Unpinned` state itself is covered by the pure unit tests.

```rust
/// Spawn the agent adapter on the candidate-list seam (ADR-0048 D2's GHOSTLIGHT_ENDPOINTS),
/// which outranks the instance selection -- so the ordered candidate walk is exercised while
/// GHOSTLIGHT_INSTANCE pins the self-heal supervisor target to a unit that never exists on this
/// machine (a harmless failed no-op, never the real "Ghostlight Service" task).
fn spawn_adapter(endpoints: &[String], instance: &str, log_dir: &Path) -> Child {
    Command::new(adapter_bin())
        .env("GHOSTLIGHT_ENDPOINTS", endpoints.join(","))
        .env_remove("GHOSTLIGHT_ENDPOINT")
        .env("GHOSTLIGHT_INSTANCE", instance)
        .env("GHOSTLIGHT_LOG_DIR", log_dir)
        .env("GHOSTLIGHT_DEBUG", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ghostlight adapter")
}
```

3. A count-aware state wait (beside the transcribed `wait_for_state`):

```rust
/// Wait until `log_dir` holds at least `count` parseable `debug-state-*.json` files (one per
/// live service), so a second service is provably up before the adapter is spawned.
fn wait_for_states(log_dir: &Path, count: usize, within: Duration) {
    let deadline = std::time::Instant::now() + within;
    loop {
        let n = std::fs::read_dir(log_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        name.starts_with("debug-state-") && name.ends_with(".json")
                    })
                    .filter(|e| {
                        std::fs::read_to_string(e.path())
                            .ok()
                            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                            .is_some()
                    })
                    .count()
            })
            .unwrap_or(0);
        if n >= count {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "expected {count} debug-state files under {} within {within:?}",
            log_dir.display()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
```

Module doc comment (top of file):

```rust
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! ADR-0048: the development override. An UNPINNED adapter resolves its service at connect time,
//! walking an ordered candidate list (dev then default in production; test-unique endpoints here
//! via the GHOSTLIGHT_ENDPOINTS seam) -- preferring the first live candidate and falling back to
//! the next, both at first connect and on every reconnect episode.
//!
//! Isolation notes: everything runs on per-run unique endpoints (never the machine's real
//! `org.sylin.ghostlight*` names); one shared GHOSTLIGHT_LOG_DIR gives every process the same
//! anti-squat hub-key; unique GHOSTLIGHT_INSTANCE names per service make `serverInfo.name` a
//! which-service-answered oracle (`ghostlight-<instance>`); the adapter also carries a per-run
//! instance so its self-heal supervisor kick targets a guaranteed-nonexistent unit (a harmless
//! failed no-op) instead of this machine's real "Ghostlight Service" -- GHOSTLIGHT_ENDPOINTS
//! outranks the selection, so the candidate walk is still fully exercised.
```

The two pinned tests (bodies transcribed exactly; the send/recv/reader plumbing mirrors
adapter_reconnect.rs):

```rust
/// ADR-0048 D3: with BOTH candidates live, an unpinned adapter connects to the FIRST (the dev
/// slot); when that service dies, the reconnect episode fails over to the SECOND (the default
/// slot) without a client reload -- and the debug events record both resolutions.
#[test]
fn unpinned_adapter_prefers_the_first_candidate_and_fails_over() {
    let (ep_a, ep_b, inst_a, inst_b, log_dir) = unique();
    let _ = std::fs::remove_dir_all(&log_dir);

    let mut service_a = service_cmd(&ep_a, &inst_a, &log_dir)
        .spawn()
        .expect("spawn service A");
    wait_for_state(&log_dir, Duration::from_secs(15));
    let mut service_b = service_cmd(&ep_b, &inst_b, &log_dir)
        .spawn()
        .expect("spawn service B");
    wait_for_states(&log_dir, 2, Duration::from_secs(15));

    let mut adapter = spawn_adapter(&[ep_a.clone(), ep_b.clone()], &inst_a, &log_dir);
    let mut stdin = adapter.stdin.take().expect("adapter stdin");
    let rx = spawn_reader(adapter.stdout.take().expect("adapter stdout"));

    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );
    let init = recv(&rx, Duration::from_secs(20));
    assert_eq!(
        init["result"]["serverInfo"]["name"],
        format!("ghostlight-{inst_a}"),
        "with both candidates live, the FIRST wins: {init:?}"
    );
    send(&mut stdin, &json!({"jsonrpc":"2.0","method":"notifications/initialized"}));

    // Kill the preferred service: the reconnect episode must fail over to the second candidate.
    let _ = service_a.kill();
    let _ = service_a.wait();
    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}),
    );
    let list = recv(&rx, Duration::from_secs(30));
    assert_eq!(
        list["id"], 3,
        "the post-failover reply answers the new request: {list:?}"
    );
    assert_eq!(
        list["result"]["tools"].as_array().map(|t| t.len()),
        Some(17),
        "the fallback service answered a real request: {list:?}"
    );

    // The adapter's debug events recorded both resolutions.
    let mut events = String::new();
    for entry in std::fs::read_dir(&log_dir).expect("read log_dir") {
        let path = entry.expect("dir entry").path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("debug-events-") && name.ends_with(".jsonl") {
                events.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    assert!(
        events
            .matches("override resolution: connected to candidate 1/2")
            .count()
            >= 1,
        "the first connect resolved to candidate 1"
    );
    assert!(
        events
            .matches("override resolution: connected to candidate 2/2")
            .count()
            >= 1,
        "the failover resolved to candidate 2"
    );

    drop(stdin);
    let _ = adapter.wait();
    let _ = service_b.kill();
    let _ = service_b.wait();
    let _ = std::fs::remove_dir_all(&log_dir);
}

/// ADR-0048 D3: when the FIRST candidate is absent, an unpinned adapter falls back to the
/// SECOND on the fast path (an absent pipe fails the dial instantly; no retry window burned).
#[test]
fn unpinned_adapter_falls_back_when_the_first_candidate_is_absent() {
    let (ep_a, ep_b, _inst_a, inst_b, log_dir) = unique();
    let _ = std::fs::remove_dir_all(&log_dir);

    // Only B runs; A's endpoint is never served.
    let mut service_b = service_cmd(&ep_b, &inst_b, &log_dir)
        .spawn()
        .expect("spawn service B");
    wait_for_state(&log_dir, Duration::from_secs(15));

    let mut adapter = spawn_adapter(&[ep_a.clone(), ep_b.clone()], &inst_b, &log_dir);
    let mut stdin = adapter.stdin.take().expect("adapter stdin");
    let rx = spawn_reader(adapter.stdout.take().expect("adapter stdout"));

    send(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );
    let init = recv(&rx, Duration::from_secs(20));
    assert_eq!(
        init["result"]["serverInfo"]["name"],
        format!("ghostlight-{inst_b}"),
        "with the first candidate absent, the second wins: {init:?}"
    );

    drop(stdin);
    let _ = adapter.wait();
    let _ = service_b.kill();
    let _ = service_b.wait();
    let _ = std::fs::remove_dir_all(&log_dir);
}
```

`spawn_reader` is the name given to the transcribed stdout reader-thread helper; if
adapter_reconnect.rs inlines that pattern instead of naming it, extract it here as:

```rust
/// Forward the adapter's stdout lines over a channel so `recv` can timeout (transcribed from
/// tests/adapter_reconnect.rs's inline reader).
fn spawn_reader(stdout: std::process::ChildStdout) -> Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(|l| l.ok()) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}
```

- Pinned commit message (T1):
  `feat(transport): the development override -- unpinned adapters resolve dev-first (ADR-0048 D1/D2/D3)`

## P2 -- browser-adapter resolution (T2; ADR-0048 D4)

### crates/transport/src/ipc.rs

- New private fn directly ABOVE `relay_native_host`:

```rust
/// Pick the native-host connect target from ordered candidates (ADR-0048 D4): the first whose
/// endpoint EXISTS right now (probe != Absent -- a busy pipe is still a live service) wins; when
/// every candidate is absent, the LAST one (the default instance in the unpinned order), whose
/// `connect()` retry patience then covers a service that is still starting up. `probe` is
/// injected so this stays a pure, unit-testable decision.
fn pick_native_host_endpoint(
    endpoints: &[String],
    probe: impl Fn(&str) -> EndpointProbe,
) -> String {
    for ep in endpoints {
        if probe(ep) != EndpointProbe::Absent {
            return ep.clone();
        }
    }
    endpoints.last().cloned().unwrap_or_default()
}
```

- `relay_native_host` signature becomes:

```rust
pub async fn relay_native_host(
    endpoints: &[String],
    debug: &crate::observability::DebugSink,
) -> Result<()> {
```

  and its first body line `let stream = connect(endpoint).await?;` becomes:

```rust
    let endpoint = pick_native_host_endpoint(endpoints, probe_endpoint);
    let stream = connect(&endpoint).await?;
```

  APPEND this paragraph to its doc comment (as `///` lines, verbatim):

```rust
/// ADR-0048 D4: `endpoints` is the ordered candidate list; the first candidate whose endpoint
/// exists is dialed (a fresh pick happens naturally per connect episode, because Chrome respawns
/// this process on every native-messaging reconnect).
```

- Two new unit tests in ipc.rs's tests module:

```rust
    /// ADR-0048 D4: the first PRESENT candidate wins; busy still counts as present.
    #[test]
    fn pick_native_host_endpoint_prefers_the_first_present_candidate() {
        let eps = vec!["dev-ep".to_string(), "default-ep".to_string()];
        let picked = pick_native_host_endpoint(&eps, |ep| {
            if ep == "dev-ep" {
                EndpointProbe::Accepts
            } else {
                EndpointProbe::Absent
            }
        });
        assert_eq!(picked, "dev-ep");
        let picked = pick_native_host_endpoint(&eps, |ep| {
            if ep == "dev-ep" {
                EndpointProbe::Rejects("busy".into())
            } else {
                EndpointProbe::Accepts
            }
        });
        assert_eq!(picked, "dev-ep");
    }

    /// ADR-0048 D4: all-absent falls to the LAST candidate (the default), preserving connect()'s
    /// startup patience toward the canonical target.
    #[test]
    fn pick_native_host_endpoint_falls_to_the_last_when_all_are_absent() {
        let eps = vec!["dev-ep".to_string(), "default-ep".to_string()];
        assert_eq!(
            pick_native_host_endpoint(&eps, |_| EndpointProbe::Absent),
            "default-ep"
        );
        let one = vec!["only-ep".to_string()];
        assert_eq!(
            pick_native_host_endpoint(&one, |_| EndpointProbe::Absent),
            "only-ep"
        );
    }
```

### crates/adapter-browser/src/main.rs

- Import line `use ghostlight_transport::instance::Instance;` becomes
  `use ghostlight_transport::instance::{Instance, Selection};`.
- In `main`, replace `resolve_instance();` with `let selection = resolve_selection();` and the
  block_on line with:

```rust
    let endpoints = ipc::endpoint_candidates(&selection);
    let result = rt.block_on(async { ipc::relay_native_host(&endpoints, &sink).await });
```

- DELETE `fn resolve_instance()` entirely; ADD in its place:

```rust
/// Resolve this native host's instance SELECTION (ADR-0048 D2/D4): an inherited, explicit
/// `GHOSTLIGHT_INSTANCE` wins (the reserved word `default` pins the default; an invalid value is
/// non-fatal -- Chrome launched us with no console, so warn and fall through); else a
/// `ghostlight-adapter-browser-<n>` per-instance copy pins `<n>` via its own argv[0] (the legacy
/// ADR-0044 Decision 4 launcher); else UNPINNED -- the plain sibling binary resolves at connect
/// time, preferring a live dev instance.
fn resolve_selection() -> Selection {
    if let Ok(raw) = std::env::var(Instance::ENV_VAR) {
        let name = raw.trim();
        if !name.is_empty() {
            if name.eq_ignore_ascii_case("default") {
                std::env::remove_var(Instance::ENV_VAR);
                return Selection::Pinned(Instance::default());
            }
            match Instance::from_name(name) {
                Ok(i) => {
                    std::env::set_var(Instance::ENV_VAR, name);
                    return Selection::Pinned(i);
                }
                Err(e) => {
                    tracing::warn!(value = %name, error = %e, "ignoring an invalid GHOSTLIGHT_INSTANCE; resolving at connect time");
                    std::env::remove_var(Instance::ENV_VAR);
                }
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(inst) = Instance::from_exe_stem_with_base(&exe, "ghostlight-adapter-browser") {
            if let Some(name) = inst.name() {
                std::env::set_var(Instance::ENV_VAR, name);
                return Selection::Pinned(inst);
            }
        }
    }
    std::env::remove_var(Instance::ENV_VAR);
    Selection::Unpinned
}
```

  (Note the deliberate semantic change carried by this fn: the PLAIN `ghostlight-adapter-browser`
  binary was the pinned default before; it is UNPINNED now -- that is ADR-0048 D4's point. A
  `ghostlight-adapter-browser-<n>` copy stays pinned.)

- Pinned commit message (T2):
  `feat(transport): the browser adapter probes candidates and picks the first live service (ADR-0048 D4)`

## P3 -- one extension host (T3; ADR-0048 D5)

All anchors are current service-worker.js/popup.js/options.js text.

### extension/service-worker.js

- Replace the ENTIRE block from the comment line
  `// Native-messaging host name. An unpacked/dev extension (installType "development") targets the`
  through the closing brace of `function boundInstance() { ... }` (inclusive) with:

```js
// Native-messaging host name. ONE host for every install (ADR-0048 D5): the browser-side
// adapter resolves WHICH service (a live dev instance, else the default) at connect time, so
// the extension no longer guesses from installType -- a static label here would lie about where
// traffic actually goes.
const NATIVE_HOST = "org.sylin.ghostlight";
```

- In `connect()`: delete the line `const host = await nativeHost();` (the following
  `if (nativePort) return; // re-check...` line STAYS -- there is still an await above it), and
  `nativePort = chrome.runtime.connectNative(host);` becomes
  `nativePort = chrome.runtime.connectNative(NATIVE_HOST);`.
- In the `GET_SESSION_STATE` handler: delete the line
  `await nativeHost(); // resolve the instance label before answering` and delete the line
  `instance: boundInstance(),` from the sendResponse object. The async wrapper stays.

### extension/popup.js

- In `renderLinkDot`: the connected branch

```js
    linkDot.title = state.instance
      ? `Connected to Ghostlight (${state.instance})`
      : "Connected to Ghostlight";
```

  becomes `linkDot.title = "Connected to Ghostlight";`

- In `renderSession`: delete this line (quoted verbatim):

```js
  const inst = state.instance ? ` (${state.instance})` : "";
```

  and in the `connectedLine` ternary the template branch

```js
    ? `Connected to Ghostlight${inst}.`
```

  becomes the plain string

```js
    ? "Connected to Ghostlight."
```

### extension/options.js

- In `renderLink`: delete this line (quoted verbatim):

```js
  const inst = state.instance ? ` (${state.instance})` : "";
```

  and the connected-branch assignment

```js
    linkText.textContent = `Connected${inst}`;
```

  becomes

```js
    linkText.textContent = "Connected";
```
- In `refreshLink`: the fallback object
  `{ killed: false, connected: false, attachedTabs: 0, instance: null }` becomes
  `{ killed: false, connected: false, attachedTabs: 0 }`.

### Post-condition grep (pinned)

`grep -n "nativeHost\|boundInstance\|NATIVE_HOST_DEV\|NATIVE_HOST_DEFAULT\|state.instance" extension/service-worker.js extension/popup.js extension/options.js`
returns ZERO matches (the bare `NATIVE_HOST` const does not match any of these patterns).

- Pinned commit message (T3):
  `feat(extension): one native host -- adapter-side resolution replaces installType selection (ADR-0048 D5)`

## P4 -- unified install surface (T4; ADR-0048 D5/D6)

### crates/core/src/install/native_host.rs

- Add directly above `pub fn origin_for`:

```rust
/// The Chrome Web Store extension id (the published "Ghostlight in Browser" listing).
pub const STORE_EXTENSION_ID: &str = "lejccfmoeogmhemakeknjjdhkfkgncdl";

/// The unpacked-dev extension id, pinned by the committed manifest `key` (ADR-0016).
pub const DEV_EXTENSION_ID: &str = "cjcmhepmagomefjggkcohdbfemacojoa";
```

- Replace `HostManifest::resolve` (fn + doc comment) with:

```rust
    /// Build from the binary path plus an OPTIONAL extra extension id (ADR-0048 D5): the two
    /// shipped identities ([`STORE_EXTENSION_ID`], [`DEV_EXTENSION_ID`]) are always allowed, so
    /// a default install needs no flag; `--extension-id` appends one more origin (validated,
    /// deduplicated) for a fork or an enterprise-packaged extension.
    pub fn resolve(current_exe: &Path, extension_id: Option<&str>) -> Result<Self> {
        let mut allowed_origins =
            vec![origin_for(STORE_EXTENSION_ID), origin_for(DEV_EXTENSION_ID)];
        if let Some(id) = extension_id {
            validate_extension_id(id)?;
            let origin = origin_for(id);
            if !allowed_origins.contains(&origin) {
                allowed_origins.push(origin);
            }
        }
        Ok(Self {
            path: normalize_exe_path(current_exe),
            allowed_origins,
        })
    }
```

- Tests: `host_manifest_json_has_type_stdio_and_exact_origin` keeps its name; its origin
  assertions become:

```rust
        let origins = v["allowed_origins"].as_array().unwrap();
        assert_eq!(origins.len(), 3);
        assert_eq!(
            origins[0],
            format!("chrome-extension://{STORE_EXTENSION_ID}/")
        );
        assert_eq!(origins[1], format!("chrome-extension://{DEV_EXTENSION_ID}/"));
        assert_eq!(origins[2], format!("chrome-extension://{}/", "a".repeat(32)));
```

  DELETE `missing_extension_id_is_an_error` and ADD in its place:

```rust
    /// ADR-0048 D5: no --extension-id needed -- both shipped identities are always allowed, and
    /// re-passing one of them never duplicates the origin.
    #[test]
    fn resolve_without_an_id_allows_the_two_shipped_extensions() {
        let m = HostManifest::resolve(Path::new("/x"), None).unwrap();
        assert_eq!(
            m.allowed_origins,
            vec![
                format!("chrome-extension://{STORE_EXTENSION_ID}/"),
                format!("chrome-extension://{DEV_EXTENSION_ID}/"),
            ]
        );
        let dup = HostManifest::resolve(Path::new("/x"), Some(DEV_EXTENSION_ID)).unwrap();
        assert_eq!(dup.allowed_origins.len(), 2);
    }
```

### crates/transport/src/error.rs

DELETE the `MissingExtensionId` variant and its doc/attr lines (anchor: the three lines beginning
`/// The installer needs the unpacked extension ID` through `MissingExtensionId,`). No other
variant moves.

### crates/core/src/install/mod.rs

- `plan_install` becomes a thin resolver wrapper plus `plan_install_for` (so the dev-thin branch
  is unit-testable without env races):

```rust
fn plan_install(opts: &InstallOptions, ctx: &PlanCtx) -> Result<Vec<Action>> {
    plan_install_for(
        opts,
        ctx,
        &ghostlight_transport::instance::Instance::resolve(),
    )
}

fn plan_install_for(
    opts: &InstallOptions,
    ctx: &PlanCtx,
    instance: &ghostlight_transport::instance::Instance,
) -> Result<Vec<Action>> {
```

  The restructure, stated exactly (the current fn opens with five `let` statements --
  `launcher`/`needs_copy`, `manifest`, `manifest_json`, `scope`, `actions` -- followed by the
  `if needs_copy { ... }` copy block and the `if cfg!(windows) { ... } else { ... }` browser
  block, then the MCP-clients section):

  1. DELETE the existing five opening `let` statements.
  2. The new opening of `plan_install_for` is EXACTLY:

```rust
    let scope = scope_of(opts.system);
    let mut actions = Vec::new();

    // ADR-0048 D6: the reserved dev instance is reached through the UNIFIED browser surface the
    // default install registers, so its install is THIN -- pinned MCP-client entries only. (Dev
    // UNINSTALL still cleans up any legacy per-instance artifacts from pre-0048 installs.)
    let dev_thin = instance.name() == Some(ghostlight_transport::instance::DEV_INSTANCE);
    if !dev_thin {
        // ADR-0044 Decision 4: the DEFAULT instance's manifest points at the bare binary; a
        // non-default instance's points at a per-instance copy Chrome launches by name (argv[0]).
        let (launcher, needs_copy) = native_host::instance_launcher(ctx);
        let manifest = HostManifest::resolve(&launcher, opts.extension_id.as_deref())?;
        let manifest_json = manifest.to_json();
```

  3. The existing `if needs_copy { ... }` block and the whole `if cfg!(windows) { ... } else
     { ... }` block move INSIDE the `if !dev_thin { ... }` braces, re-indented one level,
     otherwise byte-identical, and the `if !dev_thin` block closes after them.
  4. The MCP-clients section and the final `Ok(actions)` are unchanged and stay OUTSIDE the
     `if !dev_thin` block (they run for every instance).
  5. THERE IS EXACTLY ONE `let mut actions` in the final fn (the one in step 2). If the old one
     survives anywhere, the fn is wrong.

- In `run_install`, the supervisor arm (anchor: the line
  `println!("  (skipped: --no-supervisor)");` -- the `if opts.no_supervisor` branch it sits in)
  gains a middle arm:

```rust
    } else if ghostlight_transport::instance::Instance::resolve().name()
        == Some(ghostlight_transport::instance::DEV_INSTANCE)
    {
        // ADR-0048 D6: a dev service runs in a terminal (docs/DEV-LOOP.md); never auto-started.
        println!("  (skipped: the dev instance runs its service in a terminal; ADR-0048)");
    } else {
```

- `plan_uninstall` is UNCHANGED (dev uninstall keeps full legacy cleanup by design).
- New test in mod.rs's tests module:

```rust
    #[test]
    fn plan_install_for_the_dev_instance_is_client_entries_only() {
        let dir = std::env::temp_dir().join(format!("ghostlight-devthin-{}", std::process::id()));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(".claude.json"), "{}").unwrap();
        let ctx = PlanCtx {
            current_exe: PathBuf::from("/abs/ghostlight"),
            home,
            config: dir.join("config"),
            local: dir.join("local"),
        };
        let dev = ghostlight_transport::instance::Instance::from_name("dev").unwrap();
        let opts = InstallOptions {
            extension_id: None,
            dry_run: true,
            system: false,
            browsers: Selection::ForceAll,
            clients: Selection::Only(vec!["claude-code".into()]),
            debug: false,
            no_supervisor: true,
        };
        let actions = plan_install_for(&opts, &ctx, &dev).unwrap();
        assert!(
            actions.iter().all(|a| !a.label.contains("native host")),
            "a dev install plans no native-host action"
        );
        assert!(
            actions.iter().any(|a| a.label.contains("(client)")),
            "a dev install still plans MCP-client entries"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
```

  (NOTE: `Selection` inside install::mod.rs is `install::Selection` -- the enum already imported
  there; the instance type is referenced fully qualified. Do not import
  `ghostlight_transport::instance::Selection` into this file.)

### tests/install_instance.rs

This dry-run subprocess test file asserts the PRE-0048 dev plan and must move with the design.

- Module doc: append one sentence to the `//!` block's first paragraph, after "while the default
  install stays byte-identical":
  ` ADR-0048 D6: the reserved dev instance is the exception -- its install is THIN (client
  entries only); every OTHER named instance keeps the full stack.`
- DELETE the test `dev_install_plan_copies_a_named_binary_and_suffixes_the_whole_stack`
  entirely. ADD in its place these two tests:

```rust
#[test]
fn dev_install_plan_is_thin_client_entries_only() {
    // ADR-0048 D6: the reserved dev instance rides the UNIFIED default browser surface, so its
    // install plans NO host artifacts and NO supervisor -- pinned MCP-client entries only.
    let plan = install_plan(Some("dev"));
    assert!(
        !plan.contains("instance binary"),
        "the dev plan places no per-instance binary copy: {plan}"
    );
    assert!(
        !plan.contains("org.sylin.ghostlight.dev"),
        "the dev plan registers no per-instance native host: {plan}"
    );
    assert!(
        plan.contains("(client)"),
        "the dev plan still registers MCP-client entries: {plan}"
    );
    assert!(
        plan.contains("(skipped: the dev instance runs its service in a terminal; ADR-0048)"),
        "the dev supervisor section prints the pinned skip line: {plan}"
    );
}

#[test]
fn a_named_non_dev_instance_still_plans_the_full_stack() {
    // ADR-0048 D6: only `dev` thins; every other named instance keeps ADR-0044's full
    // per-instance stack (copy launched by name, isolated host, suffixed supervisor).
    let plan = install_plan(Some("qa"));
    assert!(
        plan.contains("instance binary") && plan.contains("ghostlight-adapter-browser-qa"),
        "a qa plan copies a per-instance browser-adapter binary: {plan}"
    );
    assert!(
        plan.contains("org.sylin.ghostlight.qa"),
        "a qa plan uses a suffixed native-host name: {plan}"
    );
    assert!(
        plan.contains("Ghostlight Service (qa)"),
        "a qa plan registers a suffixed supervisor: {plan}"
    );
}
```

- The other two tests in the file (`default_install_plan_is_byte_identical_and_places_no_copy`,
  `no_supervisor_flag_plans_no_supervisor_steps`) stay byte-identical (the default plan is
  unchanged and `--extension-id` remains a valid optional flag).

### src/main.rs (ONE line -- the batch's sanctioned exception)

The help comment `/// Unpacked-dev extension id (32 chars, a-p). Required until a build-time key ships.`
becomes:

```rust
    /// Extra extension id to allow (the Web Store and unpacked-dev ids are always allowed).
```

- Pinned commit message (T4):
  `feat(install): one browser surface -- both shipped extension ids allowed, dev install thinned (ADR-0048 D5/D6)`

## P5 -- doctor + docs (T5; ADR-0048 D7)

### crates/core/src/hub/manage/doctor.rs

Insert AFTER the IPC endpoint section (anchor: the line
`println!("  {:<9}{}", "state", state_line(&probe));`) and BEFORE
`let (log_dir, rows) = gather_sessions();`:

```rust
    // ADR-0048 D7: when this report is for the DEFAULT instance, say where UNPINNED clients
    // (agent adapters and the browser native host with no --instance) currently route: a live
    // dev instance shadows the default (the development override).
    if instance.is_default() {
        let dev = ghostlight_transport::instance::Instance::from_name(
            ghostlight_transport::instance::DEV_INSTANCE,
        )
        .expect("'dev' is a valid instance name");
        let dev_probe = ipc::probe_endpoint(&ipc::adapter_endpoint_name(&dev.endpoint()));
        println!();
        println!("Development override:");
        if matches!(dev_probe, ipc::EndpointProbe::Absent) {
            println!(
                "  no dev instance is running; unpinned clients route to this default instance"
            );
        } else {
            println!("  a dev instance is LIVE; unpinned clients currently route to it (ADR-0048)");
        }
    }
```

(Reference `ipc` and `EndpointProbe` through the SAME import path the file already uses for
`ipc::probe_endpoint` -- re-read its `use` lines and match them; if `EndpointProbe` is not
reachable through that path, add it to the existing import, never a new one.)

### docs/DEV-LOOP.md

Replace the WHOLE `## 2. Install the dev instance (once)` section (heading + body, up to but not
including `## 3.`) with the content between the four-backtick fences (the inner three-backtick
fences are part of the replacement):

````markdown
## 2. Install (once)

```
ghostlight install --debug --no-supervisor
```

Since ADR-0048 the plain DEFAULT install is all the dev loop needs: it registers ONE browser
native host (whose manifest already allows the unpacked-dev extension id -- no --extension-id)
and ONE unpinned MCP-client entry (`ghostlight`). An unpinned client resolves at connect time and
PREFERS a live dev instance, so the moment your terminal service (next step) is up, unpinned
clients and the browser route to it; when it is down, they fall back to a default service if one
exists. `--no-supervisor` matters when installing FROM target/debug: an auto-started default
service would hold the exe lock during rebuilds. Then load the unpacked extension at
chrome://extensions and restart your editor once so it picks up the registration.

Optional pin: `ghostlight --instance dev install --debug` additionally registers a PINNED
`ghostlight-dev` client entry (client entries only since ADR-0048 D6 -- no second native host, no
supervisor). Pin a client when you want it bound to dev even while a default service is running
(dev-or-nothing, e.g. mid-rebuild).
````

The rest of the file is unchanged EXCEPT: if any later line still says the dev install takes
`--extension-id <your-unpacked-id>`, delete that flag from the line.

### README.md

- The step-4 lines

```
4. Note the extension ID that Chrome assigns. The committed manifest key pins it to a stable value:
   `cjcmhepmagomefjggkcohdbfemacojoa`. Confirm the ID shown matches; you will pass it to the
   installer.
```

  become:

```
4. The committed manifest key pins the extension ID to `cjcmhepmagomefjggkcohdbfemacojoa`; the
   installer already allows it (and the Web Store ID), so there is nothing to copy.
```

- The install command block

```sh
./target/release/ghostlight install --extension-id cjcmhepmagomefjggkcohdbfemacojoa
```

  becomes:

```sh
./target/release/ghostlight install
```

- If the "Useful flags" list below mentions `--extension-id`, reword that bullet to: an EXTRA
  extension id to allow (the Web Store and unpacked-dev ids are always allowed). If it does not
  mention it (verified at authoring: it does not), add nothing.
- In the Troubleshooting section, the bullet ending
  `and confirm the extension ID matches what you passed to ``install``.` becomes
  `and check ``ghostlight doctor`` (the host manifest already allows both shipped extension
  ids).` (keep the bullet's bold lead-in and the rest of its sentence intact).

### CHANGELOG.md

Under `## [Unreleased]`, insert a new `### Added` section ABOVE the existing `### Fixed`:

```markdown
### Added
- The development override (ADR-0048): an MCP client or browser registered WITHOUT an explicit
  instance now resolves at connect time, preferring a live `dev` instance and falling back to
  the default -- run a dev service and every unpinned client routes to it; stop it and they
  return to the release install on their next connect.
- One browser surface: the native-host manifest always allows both the Web Store and the pinned
  unpacked-dev extension ids, so `ghostlight install` needs no --extension-id and one
  registration serves a store install and a dev checkout at once.
- `ghostlight doctor` reports whether a live dev instance is currently shadowing the default for
  unpinned clients.
```

And APPEND to the `### Changed` list under `## [Unreleased]` -- the ADR-0047 entries, currently
ending `...pruned on service-worker restart (ADR-0047 D5).` (NOT the `### Changed` headings under
[0.3.0] or [0.2.0]):

```markdown
- `--instance dev install` is now thin (ADR-0048 D6): it registers only the pinned
  `ghostlight-dev` MCP-client entries; browser traffic rides the unified default host.
- The extension always connects to the `org.sylin.ghostlight` host; the installType-based
  dev-host selection is superseded by adapter-side resolution (ADR-0048 D5).
```

- Pinned commit message (T5):
  `feat(doctor): report the live development-override routing + ADR-0048 docs (ADR-0048 D7)`

## Cross-cutting pins

- Ledger commit message per task: `docs(dev-override): ledger T<n>` (T1..T5).
- `tests/adapter_reconnect.rs` is NEVER edited and must stay green after every task (its
  GHOSTLIGHT_ENDPOINT + GHOSTLIGHT_INSTANCE env pins exercise the single-candidate path).
- After T1, `relay_adapter` takes `&[String]` everywhere; after T2, `relay_native_host` does. A
  leftover `&str` call is a compile error fixed by the pinned per-site changes only.
- No task edits `crates/core/src/browser/directory.rs`, the fidelity/golden tests,
  `extension/manifest.json`, or anything else on the BOOTSTRAP NEVER list.
- The words `org.sylin.ghostlight.dev` must NOT appear in extension/*.js after T3.
