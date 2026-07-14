// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Embedded static assets for the read-only loopback management UI. Plain `include_str!` const
//! literals -- no new crate dependency.

pub const INDEX_HTML: &str = include_str!("assets/index.html");
pub const MANAGE_CSS: &str = include_str!("assets/manage.css");
pub const MANAGE_JS: &str = include_str!("assets/manage.js");
