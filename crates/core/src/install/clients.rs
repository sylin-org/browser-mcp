// SPDX-License-Identifier: Apache-2.0 OR MIT
//! MCP-client detection and config targets: which clients are installed, where their config lives,
//! how we add our server entry (CLI vs safe JSON merge), and the dialect each uses (doc 11 B.*).

use super::merge::{Dialect, ServerEntry};
use super::{on_path, PlanCtx};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The v1 client set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientId {
    ClaudeCode,
    ClaudeDesktop,
    Cursor,
    VsCode,
}

/// How we register with a client. `FileMerge` is the idempotent value-level merge used for every
/// plain-JSON config; `VsCodeCli` drives VS Code's `code --add-mcp` (its config is JSONC, which a
/// value-level merge would strip of comments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddVia {
    VsCodeCli,
    FileMerge,
}

pub struct ClientSpec {
    pub id: ClientId,
    pub cli_id: &'static str,
    pub display: &'static str,
    pub dialect: Dialect,
    pub add_via: AddVia,
    /// True if the config permits comments (JSONC) -- such clients are CLI-only (never hand-merged).
    pub is_jsonc: bool,
}

pub const CLIENTS: &[ClientSpec] = &[
    ClientSpec {
        id: ClientId::ClaudeCode,
        cli_id: "claude-code",
        display: "Claude Code",
        dialect: Dialect::McpServers,
        // ~/.claude.json is plain JSON; a value-level merge is idempotent and safe even while
        // Claude Code is running (the merge re-reads at apply time -- see install::apply_merge).
        add_via: AddVia::FileMerge,
        is_jsonc: false,
    },
    ClientSpec {
        id: ClientId::ClaudeDesktop,
        cli_id: "claude-desktop",
        display: "Claude Desktop",
        dialect: Dialect::McpServers,
        add_via: AddVia::FileMerge,
        is_jsonc: false,
    },
    ClientSpec {
        id: ClientId::Cursor,
        cli_id: "cursor",
        display: "Cursor",
        dialect: Dialect::McpServers,
        add_via: AddVia::FileMerge,
        is_jsonc: false,
    },
    ClientSpec {
        id: ClientId::VsCode,
        cli_id: "vscode",
        display: "VS Code",
        dialect: Dialect::Servers,
        add_via: AddVia::VsCodeCli,
        is_jsonc: true,
    },
];

pub fn client_by_id(id: &str) -> Option<&'static ClientSpec> {
    CLIENTS.iter().find(|c| c.cli_id == id)
}

/// The user-scope config file for a client. Uniform across OSes because [`PlanCtx::config`] is the
/// per-OS base (`%APPDATA%` / `~/Library/Application Support` / `~/.config`).
pub fn config_path(spec: &ClientSpec, ctx: &PlanCtx) -> PathBuf {
    match spec.id {
        ClientId::ClaudeCode => ctx.home.join(".claude.json"),
        ClientId::ClaudeDesktop => ctx.config.join("Claude").join("claude_desktop_config.json"),
        ClientId::Cursor => ctx.home.join(".cursor").join("mcp.json"),
        ClientId::VsCode => ctx.config.join("Code").join("User").join("mcp.json"),
    }
}

/// Multi-signal detection (doc 11 C.2).
pub fn detect(spec: &ClientSpec, ctx: &PlanCtx) -> bool {
    match spec.id {
        ClientId::ClaudeCode => on_path("claude") || ctx.home.join(".claude.json").is_file(),
        ClientId::ClaudeDesktop => config_path(spec, ctx).is_file(),
        ClientId::Cursor => ctx.home.join(".cursor").is_dir(),
        ClientId::VsCode => {
            on_path("code")
                || config_path(spec, ctx)
                    .parent()
                    .is_some_and(std::path::Path::is_dir)
        }
    }
}

/// The server entry we register: absolute binary path, never npx (doc 11 B.7/C.4).
pub fn server_entry(exe: &Path) -> ServerEntry {
    let instance = ghostlight_transport::instance::Instance::resolve();
    // A non-default instance carries `--instance <n>` so the client launches the right stack. The
    // command stays the bare (stable) binary path, so a dev rebuild is picked up with no reinstall
    // (the adapter is a dumb pipe; ADR-0044 Decision 4 / ADR-0045).
    let args = match instance.name() {
        Some(name) => vec!["--instance".to_string(), name.to_string()],
        None => vec![],
    };
    ServerEntry {
        name: instance.mcp_server_name(),
        command: super::native_host::sibling_bin(exe, "ghostlight-adapter-agent")
            .to_string_lossy()
            .into_owned(),
        args,
        env: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The client entry launches the AGENT ADAPTER sibling (ADR-0046), never the `ghostlight`
    /// binary itself: MCP clients speak to `ghostlight-adapter-agent`, which relays to the service.
    #[test]
    fn server_entry_points_at_the_agent_adapter_sibling() {
        let exe = Path::new("/opt/gl/ghostlight");
        let cmd = server_entry(exe).command;
        assert!(
            cmd.contains("ghostlight-adapter-agent"),
            "command names the agent adapter: {cmd}"
        );
        let suffix = if cfg!(windows) {
            "ghostlight-adapter-agent.exe"
        } else {
            "ghostlight-adapter-agent"
        };
        assert!(cmd.ends_with(suffix), "command ends with {suffix}: {cmd}");
        assert!(
            cmd.contains("gl"),
            "command retains the parent dir /opt/gl: {cmd}"
        );
    }
}
