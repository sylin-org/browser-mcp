# LEDGER: additional installer targets (ADR-0071)

Durable progress. One task = one commit. Update RESUME HERE and add a log entry after each task.

## RESUME HERE

- Next task: **T1 (Windsurf)** -- ready to execute. See `T1-windsurf.md`.
- T2-T4 (Zed, OpenCode, Crush) are BLOCKED: their ADR-0071 `PIN AT IMPLEMENTATION` items and the
  comment-safe JSONC merge are unresolved. A research task must resolve the pins and a `merge.rs`
  dialect+JSONC task must land before T2-T4 are authored. Do NOT start them.

## Task log

| Task | Commit | Status | Notes |
|------|--------|--------|-------|
| T1 Windsurf | (pending) | NOT STARTED | clients.rs only; reuses `Dialect::McpServers` |
| T2 Zed | -- | BLOCKED | pins: `source:"custom"`?, `Zed`/`zed` dir casing; needs JSONC merge |
| T3 OpenCode | -- | BLOCKED | pins: Windows config path; `mcp` command-array dialect; JSONC |
| T4 Crush | -- | BLOCKED | pins: plain-JSON vs JSONC; `mcp` type-stdio dialect |

## Deviations

(record any numbered deviation from a task file here, with the reason, as it happens)

## Open pins to resolve before T2-T4 (research task input)

1. **Zed** -- does the current Zed settings schema require `"source": "custom"` on a custom
   `context_servers` entry? Confirm against a live Zed. Confirm the settings dir casing per OS
   (`Zed` on macOS/Windows, `zed` on Linux).
2. **OpenCode** -- exact global config path on Windows (is it `~/.config/opencode/opencode.json`
   there too, or `%APPDATA%`?). Confirm the `mcp` entry requires `type:"local"` + `enabled:true`
   and combines command+args into one array with env under `environment`.
3. **Crush** -- is `crush.json` parsed as strict JSON or JSONC? Confirm the `mcp` entry shape
   (`type:"stdio"`, separate `command`/`args`, `env`).
4. **merge.rs** -- design the comment-safe JSONC path (ADR-0071 D2): tolerant detection read; write
   only when the file has no comments, else print exact manual steps. Plus the three new dialect
   arms in `ServerEntry::to_value` (command-string vs command-array; `type`/`enabled`/`source`).
