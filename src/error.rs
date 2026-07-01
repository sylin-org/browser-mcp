//! Typed error type for the engine.
//!
//! Per the project style: **typed errors in library code** (this crate), **`anyhow` in the
//! binary and integration tests**.

use thiserror::Error;

/// Errors surfaced by the Browser MCP engine.
#[derive(Debug, Error)]
pub enum Error {
    /// The MCP JSON-RPC layer received or produced something malformed.
    #[error("MCP protocol error: {0}")]
    Protocol(String),

    /// A failure in the Chrome native-messaging framing (4-byte LE length prefix + JSON).
    #[error("native messaging error: {0}")]
    NativeMessaging(String),

    /// A failure on the inter-instance IPC (named pipe / Unix domain socket).
    #[error("ipc error: {0}")]
    Ipc(String),

    /// Another Browser MCP session already owns the browser (single-session policy, v1.0).
    #[error("another Browser MCP session already owns the browser")]
    SessionBusy,

    /// JSON (de)serialization failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Underlying I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Convenience alias for fallible engine operations.
pub type Result<T> = std::result::Result<T, Error>;
