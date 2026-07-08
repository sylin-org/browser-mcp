// SPDX-License-Identifier: Apache-2.0 OR MIT
//! ghostlight-adapter-agent: the MCP-side pass-through executable (ADR-0046 Decision 1).
//!
//! An MCP client (Claude Code, Cursor, ...) launches this over stdio. It resolves the active
//! instance, connects to the already-running `ghostlight` SERVICE over the local IPC, and relays
//! its stdio as a resilient byte pipe (ADR-0045), dying with its editor via the ADR-0029
//! parent-death watchdog. It holds NO governance and depends ONLY on ghostlight-transport, so a
//! service rebuild never relinks (locks) this binary (ADR-0046 Decision 2).

use ghostlight_transport::instance::Instance;
use ghostlight_transport::observability::{build_debug_sink, DebugSink};
use ghostlight_transport::proc::{self, ProcId};
use ghostlight_transport::role::{self, Role};
use ghostlight_transport::{ipc, watchdog};

fn main() {
    // Resolve the instance from the same precedence root `ghostlight` uses (ADR-0044) and fold the
    // winner back into GHOSTLIGHT_INSTANCE so every point-of-use `Instance::resolve()` agrees.
    resolve_instance();

    let args: Vec<String> = std::env::args().collect();
    let debug =
        std::env::var_os("GHOSTLIGHT_DEBUG").is_some() || args.iter().any(|a| a == "--debug");
    ghostlight_transport::init_tracing(debug);
    role::set_role(Role::Adapter);

    // A `--manifest` on a client invocation is a no-op: only the running SERVICE loads policy
    // (PINS.md SS5.1). Warn exactly as the former run_mcp_server did.
    if args.iter().any(|a| a == "--manifest") || std::env::var_os("GHOSTLIGHT_MANIFEST").is_some() {
        tracing::warn!(
            "a --manifest on a client invocation is ignored; the running Ghostlight service's \
             policy governs all sessions"
        );
    }

    let sink = build_debug_sink(debug, "adapter");
    // The MCP client that spawned us, captured before the runtime starts (ADR-0029). None (no
    // resolvable parent) skips the watchdog and leaves stdin EOF as the sole exit trigger. NO
    // orphan sweep here: the standalone `ghostlight doctor --fix` and the watchdog cover it, and
    // core (which owns the reaper) is deliberately not a dependency of this binary (ADR-0046).
    let parent = proc::parent();

    let rt = tokio::runtime::Runtime::new().expect("build the adapter tokio runtime");
    let block_sink = sink.clone();
    let endpoint = ipc::default_endpoint();
    let code = rt.block_on(relay_with_watchdog(&endpoint, block_sink, parent));

    // The single ordered teardown. process::exit rather than unwinding: the stdin read may still be
    // parked in a blocking ReadFile, and dropping the runtime would hang joining that thread. Flush
    // the final observability snapshot first.
    sink.flush();
    std::process::exit(code)
}

/// Resolve `--instance <name>` / `--instance=<name>` / `GHOSTLIGHT_INSTANCE`, validate it, and fold
/// the winner back into `GHOSTLIGHT_INSTANCE` (mirrors the root `ghostlight` binary's resolver,
/// minus the argv[0] step: this bin is always launched WITH args by the MCP client, so a named
/// instance rides the flag/env; the argv[0] copy signal is the browser adapter's job, ADR-0046).
/// An invalid name is fatal: print the validation error and exit 2.
fn resolve_instance() {
    if let Some(flag) = instance_flag_value() {
        let name = flag.trim();
        if name.is_empty() {
            return; // default instance
        }
        if let Err(e) = Instance::validate(name) {
            eprintln!("ghostlight-adapter-agent: {e}");
            std::process::exit(2);
        }
        std::env::set_var(Instance::ENV_VAR, name);
        return;
    }
    if std::env::var_os(Instance::ENV_VAR).is_some() {
        if let Err(e) = Instance::validate_env() {
            eprintln!("ghostlight-adapter-agent: {e}");
            std::process::exit(2);
        }
    }
}

/// Scan argv for `--instance <value>` or `--instance=<value>` (no clap: this bin tolerates unknown
/// args, e.g. a stray `--manifest`).
fn instance_flag_value() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if let Some(v) = a.strip_prefix("--instance=") {
            return Some(v.to_string());
        }
        if a == "--instance" {
            return args.next();
        }
    }
    None
}

/// Relay the client's stdio to the service, ending when the client closes OR the parent-death
/// watchdog fires (ADR-0029/0045). Transcribed from the former core `run_as_adapter`; returns the
/// process exit code (0 on a clean end or watchdog trigger, 1 on a relay error).
async fn relay_with_watchdog(endpoint: &str, debug_sink: DebugSink, parent: Option<ProcId>) -> i32 {
    let shutdown = std::sync::Arc::new(tokio::sync::Notify::new());
    if let Some(parent) = parent {
        let shutdown = shutdown.clone();
        tokio::spawn(async move {
            watchdog::wait_until_orphaned(parent).await;
            tracing::warn!(
                parent_pid = parent.pid,
                "MCP client exited; ordering shutdown"
            );
            shutdown.notify_one();
        });
    }

    tokio::select! {
        result = ipc::relay_adapter(endpoint, &debug_sink) => {
            match result {
                Ok(()) => 0,
                Err(e) => {
                    tracing::error!(error = %e, "adapter relay ended with an error");
                    1
                }
            }
        }
        _ = shutdown.notified() => 0,
    }
}
