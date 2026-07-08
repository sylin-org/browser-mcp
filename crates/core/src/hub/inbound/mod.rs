// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The inbound zone -- per-channel INGESTORS that translate a wire/transport into a native
//! tool-call and converge on the governance pipeline ([`serve_session`]).
//!
//! Each channel (the named-pipe/UDS listener thin MCP adapters dial into today, plus the local
//! HTTP/WS adapter a web MCP client drives) gets its own module here, symmetric with the
//! per-capability executors in [`crate::hub::outbound`]. The pair forms the matrix: inbound
//! ingestors converge on the pipeline, which dispatches a native tool-call to the matching
//! outbound executor. The pipeline knows neither end; the ingestors know no policy.
//!
//! Every transport implements [`ITransport`] and is spawned at the composition root. A transport
//! is a blackbox: it binds a listener, accepts connections, translates wire bytes into a session
//! the pipeline speaks, and stamps the call with its transport identity. It knows nothing of
//! capabilities; the pipeline knows nothing of wire formats.
//!
//! [`serve_session`]: crate::mcp::server::serve_session

pub mod pipe;
pub mod web;

/// A transport channel: owns a listener, accepts connections, and feeds sessions into the
/// governance pipeline.
///
/// A transport is a blackbox that waits for clients to connect and makes the communication with
/// the hub pipeline agnostic. The common denominator: produce a [`ServiceContext`] (cheaply
/// cloneable, all fields `Arc`-backed) and a stream the pipeline can read/write, then hand them
/// to `serve_session`. The pre-`serve_session` handshake differs per transport (the pipe carries
/// a session-hello + peer-cred + anti-squat proof; the web mints its own GUID and runs the WS
/// upgrade) and stays in each transport's module.
///
/// The trait exposes only `code()` (the stable identifier used in audit); the actual `run`
/// function is per-transport (each has different constructor args and a different listener type).
/// The composition root spawns each transport's `run()` directly.
pub trait ITransport: Send + Sync {
    /// The stable identifier (`"pipe"`, `"web"`). Used as the audit `transport` field.
    fn code(&self) -> &'static str;
}
