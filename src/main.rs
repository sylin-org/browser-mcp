//! Browser MCP binary -- a thin shell over the `browser_mcp` library crate.
//!
//! Governed browser automation over the user's **own authenticated Chromium session**. In v1.0
//! this is the unconstrained engine (all-open); the governance overlay is a v1.5 addition.
//!
//! The same executable runs in two roles, selected at startup by launch context:
//! - **mcp-server** (default) -- launched by the MCP client over stdio. Owns the browser IPC
//!   endpoint, serves the native-host, and runs the JSON-RPC loop, forwarding tool calls to the
//!   extension via a shared [`Browser`](browser_mcp::browser::Browser) handle.
//! - **native-host** -- launched by Chrome via `connectNative` (Chrome passes the calling
//!   extension's origin, `chrome-extension://<id>/`, as an argument). Connects to the mcp-server
//!   endpoint and relays native-messaging frames to/from the extension.

use anyhow::Result;
use browser_mcp::browser::Browser;
use browser_mcp::native::ipc;
use clap::Parser;

/// Browser MCP -- the user's own authenticated browser, for AI agents.
#[derive(Debug, Parser)]
#[command(name = "browser-mcp", version, about, long_about = None)]
struct Cli {
    /// Capability-manifest source for the governance overlay (v1.5).
    /// Absent = all-open (the v1.0 default).
    #[arg(long, value_name = "SOURCE")]
    manifest: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    browser_mcp::init_tracing();

    // Role detection must precede clap: Chrome launches the native-messaging host with extra
    // positional args (the calling extension origin) that clap would reject.
    if std::env::args().any(|a| a.starts_with("chrome-extension://")) {
        tracing::info!("browser-mcp starting (native-host role, launched by the browser)");
        ipc::relay_native_host(&ipc::default_endpoint()).await?;
        return Ok(());
    }

    let cli = Cli::parse();
    tracing::info!(
        manifest = ?cli.manifest,
        "browser-mcp starting (mcp-server role; v1.0 engine -- all-open, no governance overlay)"
    );

    // Own the browser IPC endpoint and serve the native-host in the background; run the stdio MCP
    // loop in the foreground. Both share the Browser handle.
    let browser = Browser::new();
    let endpoint = ipc::default_endpoint();
    tokio::spawn({
        let browser = browser.clone();
        async move {
            match ipc::serve(browser, &endpoint).await {
                Ok(()) => {}
                Err(browser_mcp::Error::SessionBusy) => tracing::warn!(
                    "another browser-mcp session already owns the browser; tool calls in this \
                     session will report the extension as unavailable"
                ),
                Err(e) => tracing::error!(error = %e, "browser IPC endpoint failed"),
            }
        }
    });

    browser_mcp::mcp::server::run(browser).await?;
    Ok(())
}
