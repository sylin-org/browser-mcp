//! The `Browser` handle -- the mcp-server's view of the connected browser extension.
//!
//! A tool call becomes a framed request sent to the extension (through the native-host instance
//! over the local IPC) and a correlated response, awaited by id. This module is transport-agnostic:
//! [`Browser::attach`] takes any async duplex stream -- a real IPC connection in production, an
//! in-memory pipe in tests -- so the correlation logic is verifiable without a browser.
//!
//! Wire protocol (see also `native/messages.rs`): the mcp-server sends
//! `{ "id", "type": "tool_request", "tool", "args" }`; the extension replies with
//! `{ "id", "type": "tool_response", "result" }` or `{ "id", "type": "tool_error", "error" }`.
//! Messages without an `id` (events, heartbeats) are ignored here (Phase 3 buffers events).

use crate::native::host;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};

/// How long to wait for the extension to answer a single tool call before giving up.
const TOOL_TIMEOUT: Duration = Duration::from_secs(60);

/// Delivered to a waiting caller: `Ok(result)` or `Err(tool error message)`.
type CallResult = std::result::Result<Value, String>;
type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<CallResult>>>>;

/// A cloneable handle the mcp-server uses to call tools on the extension.
#[derive(Clone)]
pub struct Browser {
    next_id: Arc<AtomicU64>,
    pending: Pending,
    /// `Some` when a native-host (and thus the extension) is connected; `None` otherwise.
    outgoing: Arc<Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>>,
}

impl Browser {
    /// Create a handle with no extension connected yet.
    pub fn new() -> Self {
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            outgoing: Arc::new(Mutex::new(None)),
        }
    }

    /// True while a native-host / extension is connected.
    pub fn is_connected(&self) -> bool {
        self.outgoing.lock().unwrap().is_some()
    }

    /// Invoke `tool` with `args` on the extension and await its result.
    ///
    /// Returns [`Error::NativeMessaging`] if no extension is connected, if the extension reports a
    /// tool error, or if the call times out.
    pub async fn call(&self, tool: &str, args: &Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), tx);

        let request = json!({ "id": id, "type": "tool_request", "tool": tool, "args": args });
        let framed = host::encode(&serde_json::to_vec(&request)?)?;

        // Enqueue only if a native-host is connected; otherwise fail fast. The lock is scoped so it
        // is never held across the await below.
        let sent = {
            let outgoing = self.outgoing.lock().unwrap();
            match outgoing.as_ref() {
                Some(tx) => tx.send(framed).is_ok(),
                None => false,
            }
        };
        if !sent {
            self.pending.lock().unwrap().remove(&id);
            return Err(Error::NativeMessaging(
                "browser extension is not connected".into(),
            ));
        }

        match tokio::time::timeout(TOOL_TIMEOUT, rx).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(msg))) => Err(Error::NativeMessaging(msg)),
            Ok(Err(_closed)) => Err(Error::NativeMessaging(
                "extension disconnected before responding".into(),
            )),
            Err(_elapsed) => {
                self.pending.lock().unwrap().remove(&id);
                Err(Error::NativeMessaging("tool request timed out".into()))
            }
        }
    }

    /// Attach a connected native-host stream: spawn a writer draining outgoing frames to it and run
    /// a reader routing replies back to waiting callers. Returns when the stream closes, at which
    /// point the browser is marked disconnected and every pending call is failed.
    pub async fn attach<S>(&self, stream: S)
    where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let (mut read_half, mut write_half) = tokio::io::split(stream);
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        *self.outgoing.lock().unwrap() = Some(tx);

        let writer = tokio::spawn(async move {
            while let Some(frame) = rx.recv().await {
                if write_half.write_all(&frame).await.is_err() || write_half.flush().await.is_err()
                {
                    break;
                }
            }
        });

        // Route replies until the stream closes (Ok(None)) or errors.
        while let Ok(Some(payload)) = host::read_message(&mut read_half).await {
            self.route_reply(&payload);
        }

        *self.outgoing.lock().unwrap() = None;
        writer.abort();
        for (_, tx) in self.pending.lock().unwrap().drain() {
            let _ = tx.send(Err("extension disconnected".to_string()));
        }
    }

    /// Route one framed reply to its waiting caller (by id). Replies without an id are events.
    fn route_reply(&self, payload: &[u8]) {
        let Ok(reply) = serde_json::from_slice::<Value>(payload) else {
            tracing::warn!("dropping unparseable extension reply");
            return;
        };
        let Some(id) = reply.get("id").and_then(Value::as_str) else {
            return; // an event/heartbeat, not a tool reply
        };
        let Some(tx) = self.pending.lock().unwrap().remove(id) else {
            return; // late or duplicate reply
        };
        let result = match reply.get("type").and_then(Value::as_str) {
            Some("tool_error") => Err(reply
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("tool execution failed")
                .to_string()),
            _ => Ok(reply.get("result").cloned().unwrap_or(Value::Null)),
        };
        let _ = tx.send(result);
    }
}

impl Default for Browser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    async fn wait_connected(browser: &Browser) {
        for _ in 0..200 {
            if browser.is_connected() {
                return;
            }
            sleep(Duration::from_millis(5)).await;
        }
        panic!("browser never reported connected");
    }

    #[tokio::test]
    async fn call_round_trips_a_tool_response() {
        let (browser_side, mut ext_side) = tokio::io::duplex(64 * 1024);
        let browser = Browser::new();

        let attached = browser.clone();
        tokio::spawn(async move { attached.attach(browser_side).await });

        // Fake extension: read one framed request, reply with a result echoing the tool name.
        let fake_ext = tokio::spawn(async move {
            let req = host::read_message(&mut ext_side).await.unwrap().unwrap();
            let v: Value = serde_json::from_slice(&req).unwrap();
            let id = v["id"].as_str().unwrap();
            let reply =
                json!({ "id": id, "type": "tool_response", "result": { "echoed": v["tool"] } });
            host::write_message(&mut ext_side, &serde_json::to_vec(&reply).unwrap())
                .await
                .unwrap();
        });

        wait_connected(&browser).await;
        let result = browser
            .call("navigate", &json!({ "url": "https://example.com" }))
            .await
            .unwrap();
        assert_eq!(result, json!({ "echoed": "navigate" }));
        fake_ext.await.unwrap();
    }

    #[tokio::test]
    async fn call_surfaces_a_tool_error() {
        let (browser_side, mut ext_side) = tokio::io::duplex(64 * 1024);
        let browser = Browser::new();
        let attached = browser.clone();
        tokio::spawn(async move { attached.attach(browser_side).await });

        tokio::spawn(async move {
            let req = host::read_message(&mut ext_side).await.unwrap().unwrap();
            let v: Value = serde_json::from_slice(&req).unwrap();
            let reply = json!({ "id": v["id"], "type": "tool_error", "error": "boom" });
            host::write_message(&mut ext_side, &serde_json::to_vec(&reply).unwrap())
                .await
                .unwrap();
        });

        wait_connected(&browser).await;
        let err = browser
            .call("javascript_tool", &json!({}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("boom"), "{err}");
    }

    #[tokio::test]
    async fn call_without_a_connection_fails_fast() {
        let browser = Browser::new();
        let err = browser.call("navigate", &json!({})).await.unwrap_err();
        assert!(err.to_string().contains("not connected"), "{err}");
    }
}
