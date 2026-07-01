//! Browser MCP binary -- a thin shell over the `browser_mcp` library crate.
//!
//! Governed browser automation over the user's **own authenticated Chromium session**. In v1.0
//! this is the unconstrained engine (all-open); the governance overlay is a v1.5 addition.

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
    let cli = Cli::parse();
    browser_mcp::init_tracing();
    tracing::info!(
        manifest = ?cli.manifest,
        "browser-mcp starting (v1.0 engine -- all-open, no governance overlay)"
    );
    // Role detection (mcp-server vs native-host) and the startup sequence are wired in Phase 1.
    Ok(())
}
