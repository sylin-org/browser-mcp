// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The inbound.pipe transport: the named-pipe (Windows) / Unix-domain-socket (Unix) listener
//! that thin MCP adapters dial into. Accepts connections, captures the peer's OS credential,
//! validates the session-hello (GUID + anti-squat proof), and hands the accepted stream to
//! `serve_session` -- the SAME governance chokepoint every transport enters.
//!
//! This module owns the TRANSPORT (the listener lifecycle, the policy gate); the platform-
//! specific wire primitives (bind, accept, peer-cred capture, the anti-squat proof exchange)
//! live in [`crate::transport::native::ipc`] -- the platform-abstraction layer, exactly as
//! [`super::web`] delegates TCP/WS primitives to `tokio::net`.
//!
//! The pipe transport carries a richer handshake than the web transport: a session-hello with
//! the adapter's GUID, a per-peer mint-quota check, a session-registry admission, and an
//! anti-squat proof. All of that lives in `ipc::handle_adapter_connection` (called by
//! `ipc::serve_adapters`); this module's job is to run the accept loop over a claimed listener.

use crate::hub::inbound::ITransport;
use crate::hub::ServiceContext;
use crate::transport::native::ipc;

/// The inbound.pipe transport instance. Constructed at the composition root with the
/// ALREADY-CLAIMED adapter listener (the composition root claims it as a process-level
/// single-instance guard before any transport runs). The transport owns the accept loop.
pub struct PipeTransport {
    /// `Some` until `run` is called (which moves the listener into the spawned task).
    listener: Option<ipc::AdapterListener>,
}

impl PipeTransport {
    pub fn new(listener: ipc::AdapterListener) -> Self {
        Self {
            listener: Some(listener),
        }
    }

    /// Run the accept loop for the life of the service. Takes the claimed listener and the
    /// shared context; per connection it clones the context and hands it to
    /// `ipc::serve_adapters`, which demuxes the session-hello and enters `serve_session`.
    /// A policy-disabled transport logs and returns without serving.
    pub async fn run(self, ctx: ServiceContext) {
        let enabled = {
            let resolution = ctx.store.current_resolution();
            let resolved = resolution
                .get(crate::governance::config::INBOUND_PIPE_ENABLED)
                .expect("registered key resolves");
            resolved.value.as_bool().unwrap_or(true)
        };
        if !enabled {
            tracing::info!(
                "inbound.pipe transport disabled by policy (inbound.pipe.enabled = false); \
                 not serving"
            );
            return;
        }

        let Some(listener) = self.listener else {
            tracing::error!("inbound.pipe transport has no listener");
            return;
        };
        tracing::info!("inbound.pipe listening");
        if let Err(e) = ipc::serve_adapters(ctx, listener).await {
            tracing::error!(error = %e, "inbound.pipe endpoint failed");
        }
    }
}

impl ITransport for PipeTransport {
    fn code(&self) -> &'static str {
        "pipe"
    }
}
