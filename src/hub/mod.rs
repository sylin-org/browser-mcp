// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The Hub composition root -- the free-licensed seam that H1/H2 later attach `ServiceContext`
//! and multiplex to (ADR-0030 Decision 2: "Extract the composition root into a free-licensed
//! `src/hub` module hosting `HubCore`"). Today (H0) this module hosts exactly the mcp-server
//! role's startup sequence, moved verbatim out of `main::run_server`: Browser handle creation,
//! the `ipc::serve` spawn, the parent-death watchdog wiring, `sweep_orphans`, and the tokio
//! runtime block. No role change, no behavior change, single stdio session only (ADR-0030
//! Decision 1: the mcp-server role is one of the four roles of the one binary).

use crate::browser::pattern;
use crate::debug::DebugSink;
use crate::governance::manifest::source;
use crate::native::ipc;
use crate::transport::executor::Browser;
use anyhow::{Context, Result};

/// mcp-server role: own the browser IPC endpoint + serve the native-host in the background, run the
/// stdio MCP JSON-RPC loop in the foreground. Both share the [`Browser`] handle.
pub fn run_mcp_server(manifest: Option<String>, debug_on: bool) -> Result<()> {
    // Resolve the user-supplied manifest source (G12, shared format doc section 1.3): the
    // --manifest flag wins when both it and GHOSTLIGHT_MANIFEST are set. Plain synchronous
    // I/O, before the async runtime starts: a source that is SELECTED but cannot be read,
    // parsed, or validated is a fatal startup error (an org policy that fails open is worse
    // than a crash), so this must happen before a single JSON-RPC line is served.
    let user_source = manifest.or_else(|| std::env::var("GHOSTLIGHT_MANIFEST").ok());
    let loaded_policy = source::load_policy(user_source.as_deref(), pattern::is_valid_pattern)
        .with_context(|| "loading the governance manifest")?;

    match (&loaded_policy.manifest, &loaded_policy.origin) {
        (Some(m), Some(origin)) => tracing::info!(
            name = %m.name,
            version = %m.version,
            hash = %m.hash,
            mode = ?m.mode,
            origin = ?origin,
            debug_mode = debug_on,
            "ghostlight starting (mcp-server role; governance overlay active)"
        ),
        _ => tracing::info!(
            debug_mode = debug_on,
            "ghostlight starting (mcp-server role; no manifest: all-open)"
        ),
    }

    // The MCP client that spawned us, captured before the runtime starts (ADR-0029). The
    // parent-death watchdog below watches it; None (no resolvable parent) simply skips the
    // watchdog and leaves stdin EOF as the sole exit trigger, as before.
    let parent = crate::proc::parent();

    // Startup self-heal (ADR-0029 part 4): reap any orphaned predecessor -- a server whose client
    // exited but that did not terminate (e.g. one built before the watchdog, or killed uncleanly)
    // -- before we serve. Best-effort and safe (only parent-dead orphans; see doctor::reap): a
    // no-op in a release build (no session registry) and when nothing is orphaned. Runs before the
    // sink is enabled, so our own not-yet-written state file is never a self-reap candidate.
    crate::doctor::sweep_orphans();

    let sink = build_debug_sink(debug_on, "mcp-server");
    let rt = tokio::runtime::Runtime::new()?;

    // Single shutdown coordinator (ADR-0029). Every shutdown trigger is a pure detector that reports
    // to `shutdown`; the one ordered teardown runs exactly once, below. No detector tears down or
    // exits on its own -- that is the single point of concern the design centers on.
    let block_sink = sink.clone();
    let code = rt.block_on(async move {
        let browser = Browser::with_debug(block_sink);
        let endpoint = ipc::default_endpoint();
        let shutdown = std::sync::Arc::new(tokio::sync::Notify::new());

        // Detector: parent-death watchdog. stdin EOF is the intended shutdown signal, but on Windows
        // a killed (not cleanly closed) client can leave our stdin read parked forever, so the read
        // loop alone would never notice the client is gone. The watchdog signals shutdown when the
        // parent process exits; it only signals -- the coordinator does the teardown.
        if let Some(parent) = parent {
            let shutdown = shutdown.clone();
            tokio::spawn(async move {
                crate::transport::watchdog::wait_until_orphaned(parent).await;
                tracing::warn!(
                    parent_pid = parent.pid,
                    "MCP client exited; ordering shutdown"
                );
                shutdown.notify_one();
            });
        }

        // The browser IPC endpoint: native-host connections attach here for the session's life.
        tokio::spawn({
            let browser = browser.clone();
            async move {
                match ipc::serve(browser, &endpoint).await {
                    Ok(()) => {}
                    Err(crate::Error::SessionBusy) => tracing::warn!(
                        "another ghostlight session already owns the browser; tool calls in this \
                         session will report the extension as unavailable"
                    ),
                    Err(e) => tracing::error!(error = %e, "browser IPC endpoint failed"),
                }
            }
        });

        // The coordinator: whichever shutdown trigger fires first lands here. stdin EOF makes
        // `server::run` return (after its own internal task cleanup); a detector signal arrives on
        // `shutdown`. Both paths fall through to the single teardown below.
        tokio::select! {
            result = crate::mcp::server::run(browser, loaded_policy, user_source) => {
                match result {
                    Ok(()) => 0,
                    Err(e) => {
                        tracing::error!(error = %e, "mcp-server loop ended with an error");
                        1
                    }
                }
            }
            _ = shutdown.notified() => 0,
        }
    });

    // The single ordered teardown. process::exit rather than unwinding: on a detector-triggered
    // shutdown the stdin read may still be parked in a blocking ReadFile, and dropping the runtime
    // would hang forever trying to join that thread (the same reason the native-host role exits
    // directly). Flush the final observability snapshot first; exiting then releases the IPC
    // endpoint for the next session.
    sink.flush();
    std::process::exit(code)
}

/// Build the observability sink for `role` ("mcp-server" or "native-host"). Debug-off yields a
/// no-op sink; if the log directory cannot be prepared we warn and continue without observability
/// rather than failing the process.
pub fn build_debug_sink(debug: bool, role: &'static str) -> DebugSink {
    if !debug {
        return DebugSink::disabled();
    }
    let Some(dir) = crate::debug::log_dir() else {
        tracing::warn!("no log directory available; running without debug observability");
        return DebugSink::disabled();
    };
    match DebugSink::enabled(&dir, role) {
        Ok(sink) => {
            tracing::info!(dir = %dir.display(), role, "debug mode on: state + event log under this dir");
            sink
        }
        Err(e) => {
            tracing::warn!(error = %e, "could not enable debug sink; continuing without it");
            DebugSink::disabled()
        }
    }
}
