# BOOTSTRAP: additional installer targets (ADR-0071)

Adds MCP clients to the `ghostlight install` auto-registration set. ADR-0071 is normative.
Only **T1 (Windsurf)** is ready: it reuses the existing `mcpServers` JSON dialect and has no open
questions. T2-T4 (Zed, OpenCode, Crush) are BLOCKED on the ADR-0071 `PIN AT IMPLEMENTATION`
items and on the comment-safe JSONC merge; their task files are authored only after a research
task resolves those pins. Do not attempt them from this batch yet.

## Authority order (on conflict, higher wins; an unanticipated conflict = STOP)

1. The live tree (facts). Task files state tree facts AS OF AUTHORING (2026-07-13, dev @ the commit
   that added `docs/adr/0071-additional-installer-targets.md`). ALWAYS re-read the named files
   before editing -- the `install/` module was recently edited by a concurrent workstream.
2. `docs/adr/0071-additional-installer-targets.md` (semantics: paths, dialects, shapes, sequencing).
3. The task file being executed.

Do not re-litigate decided questions (ADR-0071 Decision + Provenance). Do not resolve ambiguity by
judgment: STOP per the failure protocol.

## Environment facts

- Windows 11; repo root `f:\Replica\NAS\Files\repo\github\sylin-org\browser-mcp`; branch `dev`.
- Rust workspace; installer lives in `crates/core/src/install/`. Client registry:
  `install/clients.rs` (`ClientId` enum, `CLIENTS` array, `config_path`, `detect`, tests). JSON
  merge: `install/merge.rs` (`Dialect`, `ServerEntry`). T1 touches ONLY `clients.rs`.
- **Build/test in an isolated target dir** (live clients + the service lock `target/*.exe`; a plain
  build can relink-fail with os error 5 and leave a stale binary): prefix cargo with
  `CARGO_TARGET_DIR=target-check`.
- Gates (ALL must pass before the commit):
  1. `CARGO_TARGET_DIR=target-check cargo fmt --check`
  2. `CARGO_TARGET_DIR=target-check cargo clippy -p ghostlight-core --all-targets -- -D warnings`
  3. `CARGO_TARGET_DIR=target-check cargo test -p ghostlight-core --lib install::`
- ASCII only in code and docs: no em-dashes, arrows, or curly quotes. Code reads greenfield: cite an
  ADR only where the surrounding file already does so.
- SPDX header on any new file: `Apache-2.0 OR MIT`. T1 creates no new file.

## Task sequence (strict order; every prefix leaves a coherent, green tree)

| # | File | One-line goal | Status | On block |
|---|---|---|---|---|
| T1 | T1-windsurf.md | Add Windsurf as an installer target (reuses `mcpServers`) | READY | HALT |
| T2 | (not authored) | Zed -- `context_servers`, JSONC | BLOCKED on ADR-0071 pins (`source:"custom"`, dir casing) + JSONC merge | -- |
| T3 | (not authored) | OpenCode -- `mcp` (type local, command array), JSONC | BLOCKED on pins (Windows path) + JSONC merge | -- |
| T4 | (not authored) | Crush -- `mcp` (type stdio), format PIN | BLOCKED on pins (JSONC vs plain) + JSONC merge | -- |

T2-T4 also depend on a not-yet-written dialect+JSONC change to `merge.rs`. That change is authored
as its own task once the pins are resolved; until then, T2-T4 do not exist.

## Per-task procedure

1. Re-read every file the task's "Tree facts" names. If any named shape is gone or different, STOP
   (the module was refactored under you).
2. Make the edits exactly as pinned. Add the named test(s) with the pinned assertions verbatim --
   transcribe oracles, never derive them.
3. Run all three gates. All green.
4. One task = one commit: `feat(install): <summary> (ADR-0071)`. Update the LEDGER RESUME HERE +
   log entry (numbered deviations, if any).

## Completion criteria

- T1: `client_by_id("windsurf")` resolves; `ghostlight install --client windsurf --dry-run` plans a
  `mcpServers.ghostlight` entry at `~/.codeium/windsurf/mcp_config.json`; the pinned test passes;
  all gates green.

## Failure protocol

If a task cannot complete as written: revert its edits (leave the tree green at the prior commit),
mark it BLOCKED in the LEDGER with the specific reason and the exact tree fact that did not hold,
and HALT. Do not improvise around a broken assumption. Do not skip ahead.

## NEVER touch (each NEVER names its one sanctioned exception, if any)

- The sacred MCP tool schemas / any tool surface. (No exception.)
- `install/merge.rs`. (No exception in T1 -- Windsurf reuses `Dialect::McpServers` unchanged. The
  future JSONC/dialect task is the only sanctioned editor, and it is not in this batch yet.)
- Any client arm other than the one you are adding. (No exception.)
- `extension/`, `crates/core/src/governance/**`, docs/README/`llms-install.md` prose. (Doc/prose
  sync for the new client is a SEPARATE follow-up task, deliberately out of scope so the code lands
  green first. `doctor` lists the new client automatically because it iterates `CLIENTS`.)
