// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Embedded static assets for the management plane's web UI (ADR-0030 Decision 9; ADR-0033
//! Decision 2). Plain `include_str!` const literals -- no new crate dependency.

pub const INDEX_HTML: &str = include_str!("assets/index.html");
pub const MANAGE_CSS: &str = include_str!("assets/manage.css");
pub const MANAGE_JS: &str = include_str!("assets/manage.js");
