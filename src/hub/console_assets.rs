// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Embedded static assets for the Console (ADR-0030 Decision 9; PINS.md CS10,
//! `docs/tasks/console`). Plain `include_str!` const literals, matching the sole embedding
//! convention already used for `TOOLS_JSON` (`src/transport/mcp/tools.rs`) -- no new crate
//! dependency.

pub const INDEX_HTML: &str = include_str!("console/index.html");
pub const CONSOLE_CSS: &str = include_str!("console/console.css");
pub const CONSOLE_JS: &str = include_str!("console/console.js");
