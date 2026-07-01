//! Committed-origin state (Fork 7a -- engine correctness).
//!
//! Tracks the true, **browser-process-committed** origin per frame, derived from CDP
//! `Page.frameNavigated` (`securityOrigin`) events forwarded by the extension -- never from
//! page-controlled signals like `Runtime.evaluate(location)`. See
//! `docs/research/08-cdp-origin-verification-extension-trust.md`.
//!
//! In v1.0 the committed origin is simply **reported truthfully** (no enforcement). The v1.5
//! governance overlay consumes this state for per-frame domain enforcement. The raw committed URL
//! is the authoritative reported/audited value; any canonicalized matching-key is overlay-only.
//! Implemented in Phase 1/2.
