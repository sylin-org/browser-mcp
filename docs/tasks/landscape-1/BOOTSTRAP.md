# BOOTSTRAP: landscape-1 batch (post-evaluation response, ADR-0041/0042)

Implements the executable slice of the accepted post-evaluation proposals: the origin-flow
provenance `sources` audit key (ADR-0042 phase 1), MCP protocol-version negotiation
(ADR-0041 Decision 5), and the org-rollout guide (proposal P8). Saved scripts (ADR-0039) are
NOT implemented by this batch; neither is any flow ENFORCEMENT (ADR-0042 Decision 5 is a
future ADR).

## Authority order (on conflict, higher wins; a conflict a task file does not anticipate = STOP)

1. The live tree (facts). Task files state tree facts AS OF AUTHORING (2026-07-07, dev @
   656259c plus the same-day docs commit carrying ADR-0041/0042/0043 and this batch); ALWAYS
   re-read the named files before editing.
2. `PINS.md` in this directory (exact code-level shapes and oracles).
3. The ADRs: `docs/adr/0041-post-evaluation-response.md`, `docs/adr/0042-origin-flow-provenance.md`
   (semantics), and `docs/design/mcp-spec-currency-2026-07.md` (L2's rationale).
4. The task file being executed.

Do not re-litigate decided questions (ADR-0041/0042 Provenance sections list them). Do not
resolve ambiguity by judgment: STOP per the failure protocol.

## STOP preconditions for the whole batch

- `docs/adr/0042-origin-flow-provenance.md` must exist with Status: Accepted. Absent: STOP.
- `docs/design/mcp-spec-currency-2026-07.md` must exist. Absent: STOP (L2 has no authority).
- `git status` must be clean at batch start. Dirty: STOP.

## Environment facts

- Windows 11; repo root `f:\Replica\NAS\Files\repo\github\sylin-org\browser-mcp`; branch `dev`.
- Rust workspace, crate `ghostlight`. This batch touches NO extension JS and NO installer.
- Gates (ALL must pass before every commit):
  1. `cargo fmt --check`
  2. `cargo clippy --all-targets -- -D warnings`
  3. `cargo test`
  4. `node --test tests/extension/constants.test.js tests/extension/geometry.test.js tests/extension/grouping.test.js tests/extension/keys.test.js tests/extension/observation.test.js tests/extension/settle.test.js tests/extension/treediff.test.js` -- run as regression only; this batch must not change extension behavior or these files. If the tree's `tests/extension/` listing differs, run every `*.test.js` present; do not edit any.
- ASCII only in code and docs: no emdashes, no arrows, no curly quotes.
- SPDX headers on new files: `Apache-2.0 OR MIT` everywhere EXCEPT files under
  `src/governance/**` (LicenseRef-Ghostlight-Commercial). This batch creates ONE new file
  (`docs/guides/org-rollout.md`, a doc: no SPDX header needed, matching the existing guides).
  It edits existing governance files (L1) but creates no new governance files.
- The `tests/architecture.rs` a7 boundary: `src/governance/**` must not name
  browser/transport/mcp/native/tabId/token/socket. L1's `sources` key and its doc comments are
  neutral names; keep it so (say "orchestrated step", never "script tool" with a crate path,
  in governance-file doc comments -- match the existing `orchestrator` field's comment style).

## Task sequence (strict order; every prefix leaves a coherent, green tree)

| # | File | One-line goal | On block |
|---|---|---|---|
| L1 | L1-sources-audit-key.md | The `sources` audit key: resolver reporting + stamp + record | HALT |
| L2 | L2-protocol-version-negotiation.md | Negotiate `protocolVersion` over a supported set | HALT |
| L3 | L3-org-rollout-guide.md | docs/guides/org-rollout.md + cross-links | SKIP allowed |

L1 and L2 are independent in code but pinned in this order; do not reorder. If L3 is skipped
nothing depends on it.

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

- L1..L3 committed (or L3 explicitly SKIPPED) and LEDGER.md complete.
- Full gates green. `tools/list` still advertises 17 tools; NO tool schema changed.
- The only behavior changes visible to a client: the negotiated `protocolVersion` value in the
  `initialize` result (L2), and the additive `"sources"` key in audit records (L1).

## NEVER touch (each NEVER names its only sanctioned exception, if any)

- The 13 trained tools' schemas and `explain`/`script`/`form_fill`/`wait_for` declarations in
  `src/browser/directory.rs`. No exception (this batch adds no tool and no parameter).
- `tests/all_open_golden.rs` / `tests/tool_schema_fidelity.rs`. No exception. If either fails,
  you broke a NEVER; revert.
- `extension/**` and `tests/extension/**`. No exception.
- `src/governance/**` beyond L1's pinned edits to `dispatch.rs`, `ports.rs`, and
  `audit/mod.rs` (tests only in the latter). No other governance edits.
- `src/transport/mcp/form_fill.rs`. No exception (its records carry `sources: null` by
  construction; PINS SS3 explains why no edit is needed -- do not "improve" it).
- `docs/adr/**` and `docs/SPEC.md`. Exception: L1 appends exactly the one SPEC.md bullet PINS
  SS6 pins, after the existing `dry_run` bullet. Nothing else.
- The native-messaging wire framing, `src/transport/native/**`. No exception.
- ADR-0039 features (saved scripts, `$param`, script storage) and ADR-0042 Decision 5
  (flow ENFORCEMENT of any kind). No exception.
- `Cargo.toml` version, `reference/`, `docs/tasks/` other than this directory, LICENSE files,
  installer (`src/install/**`), hub machinery (`src/hub/**`). No exception.
