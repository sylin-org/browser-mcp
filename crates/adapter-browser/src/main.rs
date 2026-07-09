// SPDX-License-Identifier: Apache-2.0 OR MIT
//! ghostlight-adapter-browser: the browser-side pass-through executable (ADR-0046 Decision 1).
//!
//! Chrome launches this via `chrome.runtime.connectNative` and speaks the native-messaging framing
//! (4-byte LE length prefix + JSON) on stdin/stdout. It resolves the active instance, connects to
//! the running `ghostlight` SERVICE over the local IPC, and relays extension frames to it -- a
//! stateless byte pipe. It holds NO governance and depends ONLY on ghostlight-transport (ADR-0046
//! Decision 2), so a service rebuild never relinks (locks) this binary.

use ghostlight_transport::instance::{Instance, Selection};
use ghostlight_transport::ipc;

fn main() {
    // Chrome launches this with a BARE path and no argument room, plus the extension origin
    // (`chrome-extension://<id>/`) and `--parent-window=<hwnd>` as positional/flag args this bin
    // simply ignores. Resolve the instance, then relay.
    let selection = resolve_selection();

    // Chrome never passes `--debug`; the only debug signal is an inherited GHOSTLIGHT_DEBUG.
    let debug = std::env::var_os("GHOSTLIGHT_DEBUG").is_some();
    ghostlight_transport::init_tracing(debug);

    tracing::info!("ghostlight starting (native-host role, launched by the browser)");
    let sink = ghostlight_transport::observability::build_debug_sink(debug, "native-host");
    let rt = tokio::runtime::Runtime::new().expect("build the native-host tokio runtime");
    let endpoints = ipc::endpoint_candidates(&selection);
    let result = rt.block_on(async { ipc::relay_native_host(&endpoints, &sink).await });
    if let Err(e) = result {
        tracing::warn!(error = %e, "native-host relay ended with error");
    }
    sink.flush();
    // The relay has ended (the mcp-server or the extension went away). Exit the process directly
    // instead of returning: tokio's stdin reader parks a blocking thread in a ReadFile on Chrome's
    // still-open stdin, and dropping the runtime would hang forever trying to join it. This role is
    // a stateless relay with nothing else to flush, so an immediate exit is correct -- and it lets
    // Chrome observe the disconnect and reconnect to the next mcp-server session (no zombie).
    tracing::info!("native-host relay ended; exiting");
    std::process::exit(0);
}

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
