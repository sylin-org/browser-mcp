// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Ghostlight transport: the small, stable substrate the role executables share (ADR-0046).
//! Wire framing, dialing, the resilient relay, identity, and process-lifecycle primitives.
//! The adapters depend on THIS crate only; a dependency on ghostlight-core here or in an
//! adapter is a design error (it would reintroduce the exe-lock ADR-0046 removes).
