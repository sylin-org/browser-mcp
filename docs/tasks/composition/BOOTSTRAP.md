# BOOTSTRAP: ADR-0035..0038 composition batch

Implements the `script`, `form_fill`, and `wait_for` tools, structured results, page-state
awareness, and their governance/audit substrate. ADR-0039 (saved scripts) is Proposed and is
NOT implemented by this batch.

## Authority order (on conflict, higher wins; a conflict a task file does not anticipate = STOP)

1. The live tree (facts). Task files state tree facts AS OF AUTHORING (2026-07-06, dev @
   6c5d351 plus this batch's own prior commits); ALWAYS re-read the named files before editing.
2. `PINS.md` in this directory (exact code-level shapes and oracles).
3. The ADRs, as amended 2026-07-06: `docs/adr/0035..0038-*.md` (semantics).
4. The task file being executed.

Do not re-litigate decided questions (PINS provenance section lists them). Do not resolve
ambiguity by judgment: STOP per the failure protocol.

## Environment facts

- Windows 11; repo root `f:\Replica\NAS\Files\repo\github\sylin-org\browser-mcp`; branch `dev`.
- Rust workspace, crate `ghostlight`. Extension JS under `extension/` (no bundler; SW uses
  `importScripts("lib/...")`; content.js is injected via manifest `content_scripts`).
- Gates (ALL must pass before every commit):
  1. `cargo fmt --check`
  2. `cargo clippy --all-targets -- -D warnings`
  3. `cargo test`
  4. `node --test tests/extension/constants.test.js tests/extension/geometry.test.js tests/extension/keys.test.js` plus any files this batch has added per PINS SS15.
- ASCII only in code and docs: no emdashes, no arrows, no curly quotes. Code reads greenfield:
  no "renamed from"/ADR-number markers in src/ or tests/ comments beyond the existing style of
  citing ADR decisions where the file already does so.
- SPDX headers on new files: `Apache-2.0 OR MIT` everywhere EXCEPT files under
  `src/governance/**` (LicenseRef-Ghostlight-Commercial). This batch creates no new governance
  files; it edits existing ones (C1).
- The `tests/architecture.rs` a7 boundary: `src/governance/**` must not name
  browser/transport/mcp/native/tabId/token/socket. C1's audit keys are neutral names; keep it so.

## Task sequence (strict order; every prefix leaves a coherent, green tree)

| # | File | One-line goal | On block |
|---|---|---|---|
| C1 | C1-audit-orchestration-keys.md | Additive audit keys + CallAudit setters | HALT |
| C2 | C2-calloutcome-local-handler.md | CallOutcome split + async Handler::Local | HALT |
| C3 | C3-structured-results.md | structuredContent + outputSchema, v1 vocab | HALT |
| C4 | C4-wait-for.md | wait_for tool + settle detector | HALT |
| C5 | C5-consequence-digests.md | Observation digests on mutating actions | SKIP allowed |
| C6 | C6-readpage-diff-staleref.md | read_page diff + stale-ref errors | SKIP allowed |
| C7 | C7-script-tool.md | script tool: resolver + interpreter + budget | HALT |
| C8 | C8-dryrun-idempotency.md | script dry_run (landed pipeline-level; idempotency not taken, see ADR-0040) | HALT |
| C9 | C9-form-structure-read.md | formStructure content-script read | HALT |
| C10 | C10-form-fill.md | form_fill: matcher + orchestration | HALT |
| C11 | C11-cost-guidance.md | Cost notes in the capability guide | SKIP allowed |

If C5 or C6 is SKIPPED: C10 still lands, but its `observation` field is omitted when the digest
text is absent (the task says how). If C11 is skipped nothing depends on it.

## Per-task procedure

1. Read the task file fully, then its cited ADR sections and PINS entries, then re-read every
   file the task's Tree Facts section names.
2. Check every STOP precondition. Any failure: do not improvise; go to the failure protocol.
3. Implement exactly the Required Behavior. Add the named tests with the pinned assertions
   VERBATIM (the oracles are computed; transcribe, never re-derive).
4. Run the four gates. All green.
5. Update `LEDGER.md`: move RESUME HERE, add the task's log entry (commit hash, deviations
   numbered D1..Dn -- ANY divergence from the task file, however small, is a deviation).
6. One commit per task, message exactly as the task file pins. Do not push.

## Failure protocol

Revert the task's working-tree changes (`git checkout -- .` plus deleting new files; never
revert prior committed tasks). In LEDGER.md mark the task `BLOCKED` with the precondition or
gate that failed and your reasoning. Then: HALT the batch if the task's On-block column says
HALT; if SKIP is allowed, record the skip and continue with the next task.

## Completion criteria

After C11, the batch is code-complete but NOT verified against a live browser: the operator
runs `LIVE-VERIFY.md` (13 pinned observations). The executor does not attempt it.

- C1..C11 committed (or explicitly SKIPPED where allowed) and LEDGER.md complete.
- `tools/list` advertises 17 tools in PINS SS4's final order (16 if C10 skipped-by-halt never
  happens -- C10 is HALT, so 17 or the batch stopped).
- Full gates green; `tests/all_open_golden.rs` and `tests/tool_schema_fidelity.rs` updated only
  in the ways PINS SS4 sanctions.

## NEVER touch (each NEVER names its only sanctioned exception, if any)

- The 13 trained tools' names, parameter names, types, descriptions, enum values, field order
  in `src/browser/directory.rs`. Exception: C6 adds the optional `diff` property to
  `read_page`'s inputSchema properties map ONLY.
- `explain`'s row, description, and last position. No exception.
- `tests/all_open_golden.rs` / `tests/tool_schema_fidelity.rs` semantics. Exception: C4/C7/C10
  extend name arrays and counts exactly per PINS SS4; C3 adds outputSchema presence assertions.
- `.github/workflows/*`. Exception: C4/C5/C6 extend the extension test line per PINS SS15.
- `extension/manifest.json`. Exception: C4/C5/C6 extend `content_scripts[0].js` per PINS SS15.
  The manifest `key`, permissions, and everything else: no exception.
- `src/governance/**` beyond C1's pinned edits to ports.rs/dispatch.rs and C7's config-key
  registration in `src/governance/config/mod.rs` (SS14). No other governance edits.
- The native-messaging wire framing, `src/transport/native/**` protocol behavior. No exception.
- `docs/adr/**` (this batch implements; it does not amend). Exception: none. Record friction in
  LEDGER deviations instead.
- ADR-0039 features (saved scripts, $param, script storage). No exception.
- `reference/`, `docs/tasks/` other than this directory, LICENSE files, Cargo.toml version,
  installer (`src/install/**`), hub session machinery (`src/hub/session.rs`,
  `src/hub/inbound/**`, `src/hub/manage/**`). No exception.
- Existing extension text outputs (byte-identical rule, PINS SS5). Exception: the digest
  APPENDS a new line (C5) and wait-note style appends already exist; never reformat existing
  strings.
