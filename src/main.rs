//! Browser MCP binary -- a thin shell over the `browser_mcp` library crate.
//!
//! Governed browser automation over the user's **own authenticated Chromium session**. In v1.0
//! this is the unconstrained engine (all-open); the governance overlay is a v1.5 addition.
//!
//! The same executable runs in two roles, selected at startup by launch context:
//! - **mcp-server** (default) -- launched by the MCP client over stdio; runs the JSON-RPC loop.
//! - **native-host** -- launched by Chrome via `connectNative`; Chrome passes the calling
//!   extension's origin (`chrome-extension://<id>/`) as an argument. Phase 2 bridges it to the
//!   mcp-server instance over the local IPC.

use anyhow::Result;
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
        // Phase 2 bridges Chrome native messaging <-> the mcp-server instance over the local IPC.
        // Until the extension exists there is nothing to serve; exit cleanly.
        return Ok(());
    }

    let cli = Cli::parse();
    tracing::info!(
        manifest = ?cli.manifest,
        "browser-mcp starting (mcp-server role; v1.0 engine -- all-open, no governance overlay)"
    );
    browser_mcp::mcp::server::run().await?;
    Ok(())
}
