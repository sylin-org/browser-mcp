# managed-5 batch: BOOTSTRAP

Execution package for ADR-0055 Phase 5 + the act-now strategic riders (ADR-0055 Implementation
Decisions 8 and 9). Executor: a lesser model with ZERO conversational context. Follow literally;
resolve nothing by judgment. Semantics live in the ADR; these files pin the HOW.

## Authority order (conflicts resolve upward)
1. `docs/adr/0055-managed-scheme-central-policy-distribution.md` (Impl. Decisions 7-9) and
   `docs/adr/0056-lightbox-injectable-composition-and-e2e-harness.md`.
2. This BOOTSTRAP + the task files `T1`..`T8`.
3. The live tree (re-read before every task; as-of-authoring facts may have drifted -- if a task's
   PRECONDITIONS fail, STOP per the failure protocol; do not improvise).

## Environment facts
- Windows 11; repo `f:\Replica\NAS\Files\repo\github\sylin-org\browser-mcp`; branch `dev`.
- Live MCP clients LOCK `target/`. ALWAYS build/test with an isolated target dir:
  bash: `export CARGO_TARGET_DIR="$TEMP/gl-managed5-ct"`.
- Workspace members: root facade, crates/transport, crates/core, crates/relay, crates/lightbox.
- The managed:// engine is COMPLETE through ADR-0055 Phase 4b (commit c0741ec region). You are
  adding Phase 5 surfaces + riders only.

## Per-task procedure
1. Read the task file fully. Verify EVERY item in its PRECONDITIONS section against the live tree
   (exact grep/read commands are given). Any mismatch -> STOP (failure protocol).
2. Implement exactly the REQUIRED BEHAVIOR. Add the named tests with the pinned assertions.
3. Run VERIFICATION (literal commands). All must pass.
4. Commit: `git add <only files the task names> && git commit -m "<task's pinned commit message>"`.
5. Update `LEDGER.md`: status, commit hash, numbered deviations (any judgment call = a deviation).

## Global verification (every task, after its own verification)
- `cargo test --workspace` (exit 0)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0)
- `cargo run -q -p ghostlight-lightbox -- run --all` (all scenarios `ok`)

## Failure protocol
On any STOP: `git checkout -- .` (revert unstaged), mark the task BLOCKED in LEDGER.md with the
exact failing precondition/output, and HALT the batch (do not skip ahead).

## Style rules (repo-wide, enforced)
- ASCII only in code and docs; `--` never an em-dash. rustfmt; clippy clean.
- Every new public fn/module gets a doc comment referencing its ADR decision.
- SPDX headers: `LicenseRef-Ghostlight-Commercial` under `crates/core/src/governance/`;
  `Apache-2.0 OR MIT` elsewhere (match each file's existing header when editing).
- The a7 arch test forbids `src/governance/**` from naming crate::browser/transport/mcp/native,
  the url crate, or the BARE identifiers `tabId`/`token`/`socket` in CODE lines (use `bearer`,
  `seq`, etc.).

## NEVER touch (each names its sanctioned exception, if any)
- The 13 trained tool schemas / tools.json trained fields, names, descriptions, enums. NO exception.
- Session-event audit record shapes. NO exception. (Tool-call records: T7 MAY add the one field it
  pins -- that is the ONLY sanctioned audit change in this batch.)
- The all-open path's byte-identity: with no manifest and no managed bootstrap, audit lines, tool
  lists, and outputs are byte-identical to before the batch. Every task's tests must keep the
  existing goldens green. NO exception.
- The `Denied (D-xxxxxxxx):` denial prefix and id scheme. Exception: T6 APPENDS a trailing
  contact line after the existing message body, exactly as pinned there.
- `GovernancePaths::production()` fixed locations; no new env overrides anywhere. NO exception.
- `governance/license/**` semantics; the extension (`extension/`); SPEC section 10 text;
  `scripts/test-e2e.*` and the 27 `#[ignore=e2e]` tests (a SEPARATE later batch owns them);
  release/CI workflows. NO exception.
- Never run `git push`, never touch versions/Cargo package metadata, never `cargo publish`.

## Task sequence (one task = one commit; every prefix leaves a coherent tree)
T1 bundle `kind` -> T2 ManagedStatus sidecar -> T3 presentation validation -> T4 doctor line ->
T5 explain-tool Passport -> T6 denial contact line -> T7 audit policy_seq -> T8 lightbox scenarios.
