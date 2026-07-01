//! Inter-instance IPC between the mcp-server-role and native-host-role instances.
//!
//! Transport: a **named pipe (Windows) / Unix domain socket (macOS, Linux)** -- no localhost TCP,
//! no network dependency (this is the simplification over the reference's TCP relay). The
//! native-host instance (launched by Chrome) owns the browser and serves; the mcp-server
//! instance connects. Single-session arbitration: a second mcp-server instance that finds the
//! endpoint already owned is rejected with [`crate::Error::SessionBusy`]. Implemented in Phase 1.
