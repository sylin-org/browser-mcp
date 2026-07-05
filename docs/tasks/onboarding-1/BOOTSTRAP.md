# onboarding-1 batch: BOOTSTRAP

Ground rules for the executor implementing the onboarding-1 batch (ADR-0031: agent onboarding
contract). Assume ZERO conversational context survives to you. Follow instructions literally;
resolve nothing by judgment. Read this file fully before touching any code.

## What you are building

This batch makes ghostlight model-agnostic: an untrained model (any MCP client, not just Claude)
gets the workflow contract at handshake, valid example shapes per tool in `tools/list`, and a
corrective `ToolError` naming the missing field + example when it sends a malformed call. It is
five tasks, each one commit, in execution order:

- o01 -- Reconcile ADR-0031 itself (withdraw Decision 3, sharpen Decision 4 to hard-fail, record
  the ToolError discovery). Docs only.
- o02 -- Add the additive content to tools.json: the top-level `agentGuide` section + an
  `example` field per trained tool.
- o03 -- Emit MCP `initialize.instructions` from `agentGuide` (Decision 1).
- o04 -- Hard-fail inputSchema validation with corrective `ToolError`s at the tools/call entry
  point (Decision 4, the flagship).
- o05 -- Extend the fidelity test to pin the whole contract (Decision 5).

The authoritative design is `docs/adr/0031-agent-onboarding-contract.md` (as reconciled by o01).
The plan in the conversation that approved this batch is normative for the fix shape; this LEDGER
records what was actually done.

## Two reconciliation decisions (load-bearing; from the planning phase)

1. **Decision 3 is WITHDRAWN.** The directory's per-variant `description` is NOT parallel
   documentation to tools.json's description. It is the production source for `explain_text()`
   (`src/browser/directory.rs:405-433`), the `explain` tool's response body, and that output is
   golden-pinned. The two description strings serve different consumers (tools/list vs explain)
   and are BOTH load-bearing. o01 rewrites Decision 3 to reflect this; the implementation never
   touches the directory's description field.

2. **Decision 4 is HARD-FAIL.** inputSchema violations are REJECTED before dispatch with a
   corrective `ToolError`, not advisory. A missing `tabId` (today: silent `None` -> extension
   error) becomes an explicit corrective error. This is strictly better for an untrained model
   and matches the existing `ToolError::invalid_request(...).next_step(...)` convention already
   used for "Unknown tool" at `pipeline.rs:78-80`. The whole error+suggestion mechanism is the
   codebase's EXISTING convention (`ToolError` carries a `next_step` field on every variant,
   `error.rs:188`); Decision 4 USES `ToolError` rather than inventing a parallel mechanism.

## Preserved invariants (NEVER touch)

- The 13 trained tools' `name`/`description`/`inputSchema` in `tools.json` are byte-identical
  (ADR-0007, scoped by ADR-0031 to the trained fields). The additive `example` and `agentGuide`
  content is ghostlight's own, never part of any model's training surface.
- `tests/tool_schema_fidelity.rs`'s existing assertions stay byte-stable; o05 only ADDS new
  assertions.
- The directory's per-variant `description` is load-bearing (feeds `explain_text()`); never
  deleted (Decision 3 withdrawn).
- `tests/architecture.rs` a7 boundary stays green.
- SPDX-License-Identifier headers on every file touched: `tools.json` and `tools.rs` are
  `Apache-2.0 OR MIT`; the validator module (if new) is `Apache-2.0 OR MIT`.

## Environment facts

- Rust stable, one Cargo workspace, single portable binary `ghostlight`, zero runtime deps.
- Work on the `onboarding-1` branch (off `dev`, with the ADR-0031 commit cherry-picked). One
  task = one commit.
- ASCII only in every line you ADD. No new dependencies of any kind.
- Verification commands (a task is not done until all four pass):
  - `cargo build --all-targets`
  - `cargo test` (the whole suite; this branch starts green at 584 tests)
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
- On Windows a running ghostlight.exe locks `target/debug/ghostlight.exe`; route the build at a
  scratch target dir if needed: `CARGO_TARGET_DIR=target/onboarding cargo test` (build-artifact
  routing only, never a source change).

## Per-task procedure

1. RE-READ the named files; confirm line numbers and signatures.
2. Make the change, one logical unit.
3. Add or update the tests the task names.
4. Run all four verification commands.
5. `git add` exactly the files the task touched; commit with the task's prefix.
6. Update LEDGER.md: record the commit hash, any deviations, move RESUME HERE to the next task.
