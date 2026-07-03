//! Browser domain plugin -- tool implementations and page-content redaction.
//!
//! This bounded context (see docs/design/ghostlight-service-architecture.md section 3)
//! is the browser-specific plugin over the domain-agnostic [`crate::governance`] core: it
//! owns the tool wrappers ([`tools`]) that translate an MCP `tools/call` into an extension
//! command, the secret-value redaction overlay ([`redact`]) applied to `read_page` output,
//! the domain-pattern module ([`pattern`], authored-pattern syntax plus the WHATWG-parser-backed
//! matcher), the sacred never-touch list ([`sacred`], ADR-0018 step 2, always enforced), the
//! URL-to-governing-resource classification ([`resource`], g13: what a URL IS, for the grant
//! enforcement pre/post-dispatch checks), the tool-advertisement filter ([`advertise`], g14: a
//! visibility optimization over `tools/list`, never a security boundary), and the read/write
//! classification table ([`classify`], the plugin half of
//! [`crate::governance::ports::DomainPolicy::classify`]; the observe/mutate axis type itself
//! is core). It may depend on the governance core and on std/serde; the governance core must
//! never depend back on this module.

pub mod advertise;
pub mod classify;
pub mod pattern;
pub mod redact;
pub mod resource;
pub mod sacred;
pub mod tools;
