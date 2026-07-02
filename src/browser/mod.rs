//! Browser domain plugin -- tool implementations and page-content redaction.
//!
//! This bounded context (see docs/design/ghostlight-service-architecture.md section 3)
//! is the browser-specific plugin over the domain-agnostic [`crate::governance`] core: it
//! owns the tool wrappers ([`tools`]) that translate an MCP `tools/call` into an extension
//! command, the secret-value redaction overlay ([`redact`]) applied to `read_page` output,
//! and the domain-pattern module ([`pattern`], syntax today, matching semantics added by a
//! later task). It may depend on the governance core and on std/serde; the governance core
//! must never depend back on this module.

pub mod pattern;
pub mod redact;
pub mod tools;
