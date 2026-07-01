//! JSON-RPC 2.0 server over stdio.
//!
//! Reads framed JSON-RPC from stdin, dispatches `initialize` / `tools/list` / `tools/call`, and
//! writes responses to stdout. `tools/call` routes through [`crate::dispatch`] (the policy/audit
//! seams) and then to the native-host instance over IPC. Implemented in Phase 1.
