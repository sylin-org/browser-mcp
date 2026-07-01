//! Inter-instance IPC between the mcp-server-role and native-host-role instances.
//!
//! Transport: a local socket -- a **named pipe on Windows, a Unix domain socket elsewhere** -- via
//! the `interprocess` crate. No localhost TCP, no network dependency (the simplification over the
//! reference's TCP relay).
//!
//! Ownership (mirrors the reference's proven ordering): the **mcp-server** instance (launched first
//! by the MCP client, long-lived) owns the endpoint and [`serve`]s it; the **native-host** instance
//! (launched by Chrome, short-lived, may relaunch on service-worker wake) [`connect`]s with retry
//! and relays frames between the extension and the mcp-server ([`relay_native_host`]). Single active
//! session: if an mcp-server is already listening, a second one refuses with [`Error::SessionBusy`].

use crate::browser::Browser;
use crate::native::host;
use crate::{Error, Result};
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Name, ToNsName};
use tokio::time::{sleep, Duration};

/// Default local-socket name; override with `BROWSER_MCP_ENDPOINT` (used by tests and advanced
/// deployments that run more than one isolated instance on a host).
const DEFAULT_ENDPOINT: &str = "org.sylin.browser_mcp.v1.sock";

/// The endpoint name both roles use: the `BROWSER_MCP_ENDPOINT` env override, else the default.
pub fn default_endpoint() -> String {
    std::env::var("BROWSER_MCP_ENDPOINT").unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string())
}

fn make_name(endpoint: &str) -> Result<Name<'_>> {
    endpoint
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| Error::Ipc(format!("invalid endpoint name: {e}")))
}

/// mcp-server role: own the IPC endpoint (single active session) and serve native-host connections.
///
/// For each connected native-host, [`Browser::attach`] runs until that connection closes, then the
/// loop accepts the next (e.g. after a service-worker restart relaunches the native-host).
pub async fn serve(browser: Browser, endpoint: &str) -> Result<()> {
    // Single active session: if someone is already listening, refuse rather than double-bind.
    if Stream::connect(make_name(endpoint)?).await.is_ok() {
        return Err(Error::SessionBusy);
    }
    let listener = ListenerOptions::new()
        .name(make_name(endpoint)?)
        .create_tokio()
        .map_err(|e| Error::Ipc(format!("cannot own the IPC endpoint: {e}")))?;
    tracing::info!(endpoint, "mcp-server owns the browser IPC endpoint");

    loop {
        match listener.accept().await {
            Ok(stream) => {
                tracing::info!("native-host connected");
                browser.attach(stream).await;
                tracing::info!("native-host disconnected");
            }
            Err(e) => tracing::warn!(error = %e, "IPC accept failed"),
        }
    }
}

/// Connect to the mcp-server endpoint, retrying for ~30s so startup ordering does not matter.
pub async fn connect(endpoint: &str) -> Result<Stream> {
    for _ in 0..60u32 {
        if let Ok(stream) = Stream::connect(make_name(endpoint)?).await {
            return Ok(stream);
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(Error::Ipc(
        "timed out connecting to the mcp-server endpoint".into(),
    ))
}

/// native-host role: connect to the mcp-server endpoint and relay frames between Chrome native
/// messaging (this process's stdin/stdout) and the mcp-server, until either side closes.
pub async fn relay_native_host(endpoint: &str) -> Result<()> {
    let stream = connect(endpoint).await?;
    let (mut ipc_read, mut ipc_write) = tokio::io::split(stream);
    let mut chrome_in = tokio::io::stdin();
    let mut chrome_out = tokio::io::stdout();

    // extension -> mcp-server
    let upstream = async {
        while let Ok(Some(frame)) = host::read_message(&mut chrome_in).await {
            if host::write_message(&mut ipc_write, &frame).await.is_err() {
                break;
            }
        }
    };
    // mcp-server -> extension
    let downstream = async {
        while let Ok(Some(frame)) = host::read_message(&mut ipc_read).await {
            if host::write_message(&mut chrome_out, &frame).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = upstream => {}
        _ = downstream => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[tokio::test]
    async fn serve_bridges_a_tool_call_over_the_real_ipc() {
        let endpoint = "browser-mcp-test-serve-bridge";
        let browser = Browser::new();
        let serving = browser.clone();
        tokio::spawn(async move {
            let _ = serve(serving, endpoint).await;
        });

        // Fake native-host: connect (retrying until serve is listening) and answer one request.
        let mut stream = connect(endpoint).await.expect("connect to serve");
        let fake = tokio::spawn(async move {
            let req = host::read_message(&mut stream).await.unwrap().unwrap();
            let v: Value = serde_json::from_slice(&req).unwrap();
            let reply = json!({ "id": v["id"], "type": "tool_response", "result": { "echoed": v["tool"] } });
            host::write_message(&mut stream, &serde_json::to_vec(&reply).unwrap())
                .await
                .unwrap();
        });

        for _ in 0..200 {
            if browser.is_connected() {
                break;
            }
            sleep(Duration::from_millis(5)).await;
        }
        let result = browser
            .call("navigate", &json!({}))
            .await
            .expect("tool call round-trips over the real IPC");
        assert_eq!(result["echoed"], "navigate");
        fake.await.unwrap();
    }
}
