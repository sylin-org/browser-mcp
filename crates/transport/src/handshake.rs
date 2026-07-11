// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The ADAPTER/CONTROL endpoint's session-hello (ADR-0030 Decision 1, the 2026-07-04 two-endpoint
//! amendment; PINS.md SS1).
//!
//! Carried ON TOP OF the existing 4-byte-LE `transport::native::host` framing (never a change to
//! that framing) as one JSON object: `{ "hub": 1, "role": "<role>", "guid": "<uuid>"? }`. This
//! endpoint is the ONLY place a hello is ever sent: the EXTENSION endpoint keeps its exact
//! server-speaks-first contract and carries no hello frame at all, so there is NO `ROLE_EXT` --
//! the extension is identified by the endpoint it arrives at, not by a role string.

/// The session-hello protocol major version (PINS.md SS1).
pub const HUB_PROTO: u32 = 1;

/// An MCP stdio adapter session (PINS.md SS1): the role `hub::run_mcp_server` (ALWAYS the thin
/// ADAPTER as of ADR-0030 Decision 8's always-ready-service amendment; PINS.md SS5.1) sends via
/// `ipc::relay_adapter`, and the role dispatched to
/// [`crate::transport::mcp::server::serve_session`] on the service side.
pub const ROLE_ADAPTER: &str = "adapter";

/// The control-plane role (doctor/console): a non-session, read-only request/reply over the
/// ADAPTER/CONTROL endpoint. The hello is `{ hub, role: "control", request: "<name>" }`; the
/// service answers one framed reply and closes, admitting no session (no guid, no anti-squat
/// proof, no `serve_session`). Access is bounded by the endpoint's owner-only transport ACL
/// (same OS user only), and replies carry only non-sensitive liveness. The first request is
/// [`CONTROL_REQUEST_STATUS`], which `ghostlight doctor` uses to render a real extension
/// connected/disconnected verdict without requiring `--debug` instrumentation (CAP-MED-01).
pub const ROLE_CONTROL: &str = "control";

/// The `control` request that returns a liveness snapshot ([`crate::ipc::StatusReply`]): whether
/// the browser extension is currently attached, and how many tool sessions are live.
pub const CONTROL_REQUEST_STATUS: &str = "status";

/// The SERVICE's anti-squat proof, sent AFTER admitting the adapter's hello and BEFORE
/// `serve_session` (ADR-0030 Decision 8 amendment; PINS.md SS5.3): `{"hub":1,"role":"service-proof",
/// "mac":"<hex>"}`, the lowercase-hex HMAC-SHA256 of the adapter's exact hello bytes, keyed by this
/// install's per-user `hub-key` (`src/hub/antisquat.rs`).
pub const ROLE_SERVICE_PROOF: &str = "service-proof";
