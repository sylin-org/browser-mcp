# T04: Extension-channel warmup at initialize + bounded first-call wait

## Goal

The first tool call of a fresh session races the extension channel establishment and fails
with a generic not-connected error even though everything is healthy one second later. Make
the binary (a) start observing channel readiness at MCP `initialize`, and (b) wait a bounded
5 seconds for the channel when a `tools/call` arrives before it is ready, instead of failing
immediately. When a wait happened, say so truthfully in the tool result; when the wait times
out, return an actionable error.

## Project context

Browser MCP is governed browser automation. A single Rust binary is BOTH the MCP server
(JSON-RPC 2.0 over stdio, hand-rolled on tokio, no MCP SDK crate) AND the Chrome
native-messaging host. A thin Manifest V3 extension executes CDP commands. Architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles are separate OS processes bridged by tokio-native local IPC (a Windows
named pipe or a Unix domain socket):

- mcp-server role (default, launched by the MCP client): runs the stdio JSON-RPC loop in
  `src/mcp/server.rs` and, from process startup, serves the IPC endpoint in a spawned task
  (see `run_server` in `src/main.rs`, lines 230-259: `tokio::spawn(ipc::serve(...))` happens
  before `mcp::server::run(browser)` is awaited).
- native-host role (launched by Chrome via `connectNative` when the extension connects):
  dials the mcp-server's IPC endpoint with retry for about 30 seconds
  (`connect` in `src/native/ipc.rs`) and relays 4-byte-LE-framed JSON both ways.

Connection direction matters for this task: the binary CANNOT dial the extension. The
extension side initiates (Chrome spawns the native-host process, which dials the endpoint the
mcp-server already serves). So "warmup" on the binary side means: verify and record readiness
without blocking, and make the first tool call tolerate the handshake latency.

How a tool call flows today: the MCP client writes a JSON-RPC line to stdin; `run` in
`src/mcp/server.rs` routes it through `handle_line` to `handle_tools_call`, which calls
`Browser::call` (`src/browser.rs`). `Browser::call` frames a `tool_request`, sends it through
the attached IPC stream, and awaits the correlated `tool_response` or `tool_error` (60s
timeout, `TOOL_TIMEOUT` at `src/browser.rs` line 25). `Browser::attach` (lines 120-150) is
invoked by `ipc::serve` when a native-host connects; it sets the `outgoing` sender and clears
it when the stream closes. `Browser` is `Clone`; all its state is behind `Arc`.

Files involved in this task:

- `src/mcp/server.rs` (the JSON-RPC loop; most changes land here)
- `src/browser.rs` (add a readiness-wait primitive)
- `tests/mcp_protocol.rs` (update one assertion, add one integration test)

Files you will read but MUST NOT modify: `src/mcp/schemas/tools.json` (sacred, byte-frozen
tool schemas), `tests/tool_schema_fidelity.rs` (guard test), `src/native/ipc.rs`,
`src/main.rs`, `src/dispatch.rs`, `src/policy/`, everything under `extension/`.

Build and test: `cargo test` from the repo root (this is a Windows dev machine; the same
commands work on Unix). Also run `cargo fmt` and
`cargo clippy --all-targets -- -D warnings`. If `target/debug/browser-mcp.exe` is locked by a
running session, rename it aside first (for example
`mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and rebuild.

## Current behavior

Verified against the working tree:

- `src/mcp/server.rs` lines 30-46: `run` is a strictly sequential loop. Each line from stdin
  is handled by `handle_line(...).await` inline, and the response is written to stdout before
  the next line is read. Any slow `tools/call` blocks all later protocol handling (a
  subsequent `initialize`, `ping`, or notification would sit unread).
- `src/mcp/server.rs` line 86: the `"initialize"` arm returns `initialize_result()`
  immediately. Nothing observes or warms the extension channel at initialize time.
- `src/mcp/server.rs` lines 116-155: `handle_tools_call` extracts `name` and `args`, runs the
  no-op v1.0 seams `dispatch::policy_check(name)` and `dispatch::audit(name)` (lines
  132-133), then calls `browser.call(name, &args).await` (line 135). On `Ok`, `read_page`
  results pass through `policy::redact::apply_to_result` (lines 140-144). On `Err`, it builds
  `text_content(format!("Error: {e}"))` and sets `"isError": true` (lines 147-153).
- `src/browser.rs` lines 83-96: `Browser::call` fails fast when no native-host is attached:
  it returns `Error::NativeMessaging("browser extension is not connected")`. Through the
  `thiserror` display prefix (`src/error.rs` lines 15-17) and the server's error wrapper,
  the MCP client sees the text
  `Error: native messaging error: browser extension is not connected` with `isError: true`.
  This is the failure the first call of every session can hit while the handshake is still
  in flight.
- `src/browser.rs` lines 64-66: `is_connected()` reports whether `outgoing` is `Some`. There
  is no way to WAIT for connectedness; callers can only poll (the unit tests do exactly that
  in `wait_connected` helper loops, lines 187-195).
- `src/browser.rs` lines 120-150: `attach` sets `outgoing` to `Some(tx)` and calls
  `self.debug.set_connected(true)` on connect; on stream close it clears `outgoing`, calls
  `set_connected(false)`, and fails all pending calls.
- `tests/mcp_protocol.rs` lines 16-46: the `drive` helper spawns the real binary with an
  isolated `BROWSER_MCP_ENDPOINT`, writes request lines, closes stdin, and collects response
  lines. Lines 80-90: with no extension connected, the `tools/call` response is asserted to
  be `isError: true` with text containing `"not connected"`.
- `docs/adr/` contains ADRs 0001 through 0016. ADR-0019 does not exist; it is referenced
  below strictly as "(proposed)" in a doc comment. Do not create it.
- Cargo.lock pins tokio 1.52.3 with the `sync` and `time` features already enabled, so
  `tokio::sync::watch`, `tokio::sync::mpsc`, and `tokio::time::timeout` are available with no
  manifest change.

## Required behavior

Four pieces. Implement all of them exactly as specified.

### 1. `Browser::wait_connected` (src/browser.rs)

Add a `tokio::sync::watch` channel to `Browser` so callers can await connectedness instead of
polling.

- Extend the import `use tokio::sync::{mpsc, oneshot};` to include `watch`.
- Add a field to the `Browser` struct: `connected: Arc<watch::Sender<bool>>` with a short
  doc comment (readiness signal; `true` while a native-host is attached).
- Initialize it in `with_debug`: `connected: Arc::new(watch::channel(false).0)`. Dropping the
  initial receiver is fine because updates use `send_replace` (see next point). Do NOT use
  `watch::Sender::send`, which fails and skips the update when no receiver exists.
- In `attach`, immediately after `*self.outgoing.lock().unwrap() = Some(tx);` and
  `self.debug.set_connected(true);`, add: `self.connected.send_replace(true);`
- In `attach`, on the disconnect path (after `*self.outgoing.lock().unwrap() = None;` and
  `self.debug.set_connected(false);`), add: `self.connected.send_replace(false);`
- Add this public method (doc comment required):

```rust
/// Wait until a native-host / extension is attached, up to `timeout`. Returns `true`
/// immediately when already connected, `true` when a connection arrives within the window,
/// and `false` when the window elapses without one.
pub async fn wait_connected(&self, timeout: Duration) -> bool {
    let mut rx = self.connected.subscribe();
    if *rx.borrow() {
        return true;
    }
    tokio::time::timeout(timeout, async {
        while rx.changed().await.is_ok() {
            if *rx.borrow() {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false)
}
```

- Leave `is_connected()` unchanged.
- Add two async unit tests to the existing `#[cfg(test)] mod tests` in `src/browser.rs`:
  - `wait_connected_times_out_without_a_connection`: a fresh `Browser::new()` returns `false`
    from `wait_connected(Duration::from_millis(50))`.
  - `wait_connected_wakes_when_the_extension_attaches`: create a `tokio::io::duplex` pair,
    spawn a task that sleeps 50ms then calls `attach` on a clone, and assert
    `wait_connected(Duration::from_secs(2))` returns `true`.

### 2. The constant (src/mcp/server.rs)

Add near `PROTOCOL_VERSION`:

```rust
/// How long a `tools/call` waits for the extension channel to come up before failing. The
/// first call of a session races the native-messaging handshake; waiting briefly turns the
/// single most common spurious failure into a success. Slated to become governance config
/// key `engine.connection.first_call_wait_ms` per ADR-0019 (proposed); a hardcoded constant
/// until the config plumbing lands.
const FIRST_CALL_WAIT_MS: u64 = 5000;
```

### 3. Concurrent tools/call handling + single stdout writer (src/mcp/server.rs)

The bounded wait must not block other protocol handling (initialize responses, ping,
notifications such as cancellations). The current sequential loop would block, so restructure
`run` as follows:

- Create `let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<JsonRpcResponse>();`
  before the read loop.
- Spawn a single writer task that owns stdout. Move the existing debug-response logging
  (currently `run` lines 36-41) into it, using a clone of the sink taken before spawning
  (`let debug = browser.debug().clone();` -- `DebugSink` derives `Clone`). Writer loop shape:

```rust
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
```

- The read loop keeps calling `handle_line(...).await` for each nonempty line, but any
  `Some(resp)` it returns is now sent through `tx` instead of written directly:
  `let _ = tx.send(resp);`
- Change `handle_line` to take one extra parameter,
  `tx: &tokio::sync::mpsc::UnboundedSender<JsonRpcResponse>`, and change its `"tools/call"`
  arm so the call runs on its own task and its response goes out through `tx`:

```rust
"tools/call" => {
    let browser = browser.clone();
    let tx = tx.clone();
    let params = raw.get("params").cloned();
    tokio::spawn(async move {
        let resp = handle_tools_call(&browser, config, id, params.as_ref()).await;
        let _ = tx.send(resp);
    });
    None
}
```

  All other arms stay inline and unchanged in behavior, so `initialize`, `tools/list`,
  `ping`, error responses, and notification handling keep their current semantics and their
  arrival-order response ordering relative to each other. Responses to in-flight tool calls
  may now interleave with later responses; JSON-RPC correlates by id, so this is correct.
- After the read loop ends (stdin EOF): `drop(tx);` then `let _ = writer.await;` then return
  `Ok(())`. Because each spawned call task holds a `tx` clone, the writer drains all
  in-flight tool responses before the server exits.
- Update the module doc comment at the top of `src/mcp/server.rs` (lines 1-6) to mention that
  `tools/call` is handled concurrently and all responses funnel through a single stdout
  writer task.

### 4. Warmup at initialize + bounded wait + truthful note (src/mcp/server.rs)

Warmup: in the `"initialize"` arm of `handle_line`, before returning the response, spawn a
non-blocking watcher (the initialize response itself must not be delayed):

```rust
"initialize" => {
    // Warm the extension channel while the client finishes its handshake. The extension
    // side initiates the connection (Chrome spawns the native-host, which dials the
    // endpoint this process has served since startup), so there is nothing to dial from
    // here; this watcher verifies readiness and records the outcome.
    tokio::spawn({
        let browser = browser.clone();
        async move {
            let started = Instant::now();
            if browser
                .wait_connected(Duration::from_millis(FIRST_CALL_WAIT_MS))
                .await
            {
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
```

Bounded wait: in `handle_tools_call`, between the `dispatch::audit(name);` seam and the
`browser.call(...)` invocation, insert:

```rust
// Bounded first-call wait: the first call of a session races the extension handshake.
// Wait briefly for the channel instead of failing a healthy session (also covers calls
// arriving during a mid-session reconnect).
let mut waited: Option<Duration> = None;
if !browser.is_connected() {
    let started = Instant::now();
    if browser
        .wait_connected(Duration::from_millis(FIRST_CALL_WAIT_MS))
        .await
    {
        waited = Some(started.elapsed());
    } else {
        tracing::warn!(tool = name, "tools/call failed: extension channel never came up");
        let mut result = text_content(format!(
            "Browser extension not connected after {}s. Check that Chrome is running \
             with the extension enabled; run with --debug and inspect the status files.",
            FIRST_CALL_WAIT_MS / 1000
        ));
        if let Some(obj) = result.as_object_mut() {
            obj.insert("isError".into(), json!(true));
        }
        return JsonRpcResponse::success(id, result);
    }
}
```

The timeout message rendered to the client must be exactly (one line, no `Error:` prefix,
error-ness carried by `isError: true` as with existing tool failures):

    Browser extension not connected after 5s. Check that Chrome is running with the extension enabled; run with --debug and inspect the status files.

Truthful note: when (and only when) `waited` is `Some`, append one note as an additional text
block at the END of the result's `content` array, in BOTH result arms:

- Success arm: after the existing `read_page` redaction call, so the note is the last block.
- Error arm: after building the `Error: {e}` text content and before/after setting
  `isError` (order between note and `isError` does not matter; the note must be the last
  content block).

Use a private helper with a doc comment:

```rust
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
```

The note text format is exactly `(waited N.Ns for browser extension handshake)` with one
decimal place, for example `(waited 1.2s for browser extension handshake)`. If the result has
no `content` array (not expected from the extension), leave the result untouched; do not
fabricate structure.

Behavioral notes you must preserve:

- If the channel drops between the wait and the send, `Browser::call` still returns its
  truthful `browser extension is not connected` error; do NOT retry or loop. One wait, one
  attempt.
- If another browser-mcp session owns the IPC endpoint (`SessionBusy` warned at startup in
  `src/main.rs`), the channel never comes up, so every `tools/call` now takes the bounded 5s
  before returning the timeout message instead of failing instantly. This is accepted; the
  message stays the same.
- Imports needed in `src/mcp/server.rs`: `std::time::{Duration, Instant}` and
  `tokio::sync::mpsc`.

### Tests (tests/mcp_protocol.rs)

- Update `initialize_tools_list_and_tool_call_over_stdio`: the no-extension `tools/call` now
  returns after the bounded wait (about 5 seconds; accept the slower test). Replace the
  `text.contains("not connected")` assertion (currently line 88) with an exact-equality
  assertion against the full timeout message given above, and update the nearby comment
  (currently line 80) to say the call waits the bounded window before returning the error.
- Update the file-level doc comment (lines 1-5), which currently claims no native-host is
  ever connected in this file; the new test below connects a fake one.
- Add a new integration test `tools_call_waits_for_a_late_extension_and_notes_the_wait`:
  1. Spawn the binary exactly as `drive` does (unique endpoint via the existing `SEQ`
     counter, piped stdin/stdout, null stderr), but keep stdin open for the whole test.
  2. Write two request lines: `initialize` (id 1) and
     `tools/call` (id 2, `{"name":"navigate","arguments":{"url":"https://example.com"}}`).
  3. From a `std::thread::spawn`, build a `tokio::runtime::Runtime`, sleep 1000ms, then act
     as a fake extension over the real IPC: `browser_mcp::native::ipc::connect(&endpoint)`,
     split the stream, read one framed request with
     `browser_mcp::native::host::read_message`, parse it, and reply with
     `browser_mcp::native::host::write_message` sending
     `{"id": <same id>, "type": "tool_response", "result": {"content": [{"type": "text", "text": "navigated"}]}}`.
     (This mirrors the fake-extension pattern already used by the unit tests in
     `src/browser.rs` and `src/native/ipc.rs`.)
  4. Read stdout lines with a `BufReader`: the first response is id 1 (initialize); the
     second is id 2. Assert the id-2 result is not an error (`result["isError"]` absent or
     not `true`), its first content block text is `"navigated"`, and its LAST content block
     text starts with `"(waited "` and ends with `"s for browser extension handshake)"`.
     Do not assert the exact digits (timing varies).
  5. Join the fake-extension thread, drop stdin, `child.wait()`.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in extension
   JS. (This task touches no extension file at all.)
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly quotes, anywhere,
   including comments.
4. The engine is truthful: never fake success, never silently substitute behavior; when
   something failed or was recovered, say so in the tool result text. The wait note and the
   timeout message above are the truthful surface of this feature; emit them exactly.
5. No new runtime dependencies. No Cargo.toml changes; tokio 1.52 already provides `watch`,
   `mpsc`, and `time` under the enabled features.
6. Rust 2021 edition, `thiserror` for typed errors in library code, doc comments on public
   items, `rustfmt` clean, `clippy --all-targets -- -D warnings` clean.
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style (this codebase comments the WHY on non-obvious blocks).
8. Do NOT copy code from the official Anthropic extension or any other project; implement the
   described behavior from scratch.

Task-specific:

9. Do not change `TOOL_TIMEOUT` in `src/browser.rs`, and do not add retries anywhere.
10. Do not add `engine.connection.first_call_wait_ms` to the `KEYS` registry or `Config` in
    `src/policy/mod.rs`. Constant only; the doc comment names the future key.
11. Do not create `docs/adr/0019-*.md` or edit any ADR.
12. Do not modify `src/native/ipc.rs`, `src/native/host.rs`, `src/main.rs`,
    `src/dispatch.rs`, or anything under `src/policy/` or `extension/`.
13. All stdout writes in the server must funnel through the single writer task; never write
    to stdout from a spawned call task directly.
14. Preserve the JSON-RPC behaviors covered by `malformed_method_and_null_id_follow_jsonrpc_rules`:
    null-id requests get an echoed `"id": null`, id-bearing malformed requests get `-32600`,
    notifications get no response.

## Verification

1. `cargo fmt` then `cargo clippy --all-targets -- -D warnings` from the repo root: clean.
2. `cargo test` from the repo root: all tests pass, including the updated
   `initialize_tools_list_and_tool_call_over_stdio` (now about 5 seconds slower by design),
   the new `tools_call_waits_for_a_late_extension_and_notes_the_wait`, the two new
   `wait_connected_*` unit tests in `src/browser.rs`, `tests/tool_schema_fidelity.rs`
   unchanged, and `tests/peer_death.rs`.
3. If `target/debug/browser-mcp.exe` is locked by a running session, rename it aside (for
   example `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and rebuild.
4. Manual check (binary-only change: the user must restart the MCP client to pick up the new
   binary; no extension reload is needed):
   - With Chrome running and the extension enabled, start a fresh MCP client session and
     issue a tool call immediately. It should succeed; if the handshake was still in flight,
     the result ends with the `(waited N.Ns for browser extension handshake)` note.
   - With Chrome fully closed, a tool call should fail after about 5 seconds with exactly:
     `Browser extension not connected after 5s. Check that Chrome is running with the extension enabled; run with --debug and inspect the status files.`
   - While a call is waiting, the server must still answer `ping` immediately (observable
     with `--debug` via the event log, or by pipelining requests over stdio).

## Out of scope

- Any extension-side change: reconnect logic, keepalive tuning, service-worker death
  recovery, error surfaces in `extension/service-worker.js` or `extension/content.js`. Those
  belong to other tasks (T05 covers extension error truthfulness; SW recovery is its own
  task).
- Config-key plumbing for the wait window: no `KeyDef` entry, no `Config` field, no manifest
  reading, no env override. The constant plus its doc comment is the entire footprint.
- Writing ADR-0019 (it is referenced as proposed, nothing more).
- Retrying tool calls, reconnect orchestration from the binary side, or changing the IPC
  transport, endpoint ownership, or the single-session (`SessionBusy`) policy.
- Changing `TOOL_TIMEOUT`, the native-messaging framing, or `Browser::call`'s
  request/response correlation.
- Any change to tool result shapes beyond the single appended note block, and any change to
  the sacred tool schemas or their guard test.
- Progress notifications, MCP `$/cancelRequest` handling, or any new protocol method.
