# BOOTSTRAP: closed-loop browser core (ADR-0078)

Implements ADR-0078 as six ordered, independently green commits. The batch makes ordinary browser
interaction smaller for models, clearer for users, and more useful to governance without changing
the 13 trained schemas or the local, visible-browser product boundary.

## Authority order

1. The live tree. Re-read every file named by the active task before editing.
2. `PINS.md` in this directory. It fixes code-level vocabulary, budgets, seams, and test oracles.
3. `docs/adr/0078-closed-loop-browser-core.md`. It fixes product and architectural semantics.
4. The active task file.

If the live tree invalidates a pin, stop and record the exact mismatch in `LEDGER.md`. Do not move
policy into the extension, page content into audit, or a cross-origin frame shortcut into v1.

## Environment facts

- Windows 11; branch `dev`; Rust 2021 workspace plus a Manifest V3 extension.
- Use `CARGO_TARGET_DIR=target-check-closed-loop`. Live clients and the service can lock the normal
  target executables.
- The registry and shared ingest chokepoint are in `crates/core/src/browser/directory.rs` and
  `crates/core/src/mcp/pipeline.rs`.
- The closest semantic composition pattern is `crates/core/src/mcp/form_fill.rs`.
- Extension refs, reads, find, wait, and form structure live in `extension/content.js`.
- Browser dispatch and post-action observation live in `extension/service-worker.js` and
  `extension/lib/observation.js`.
- `CallOutcome` is in `crates/core/src/mcp/outcome.rs`; typed public failures are in the browser
  error path. Audit types live under `crates/core/src/governance/`.

## Task sequence

| # | File | Goal | Depends on |
|---|---|---|---|
| C1 | `C1-actionable-observations.md` | Shared element summaries and ranked matching | -- |
| C2 | `C2-interaction-receipts.md` | Bounded receipt and recovery vocabulary | C1 |
| C3 | `C3-act-on.md` | One governed semantic interaction | C1, C2 |
| C4 | `C4-output-provenance.md` | Structured provenance and text boundaries | C2 |
| C5 | `C5-dialog-control.md` | Explicit JavaScript dialog lifecycle | C2, C3 |
| C6 | `C6-tab-control.md` | Explicit session-owned focus/reload/close | C2 |

Strict implementation order is C1 through C6. Every task is one logical commit and leaves the full
fast tier green. Do not combine tasks merely because they touch the same registry file.

## Common gates

Run before every task commit:

```powershell
$env:CARGO_TARGET_DIR = "target-check-closed-loop"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node --test tests/extension/*.test.js
```

Also run `node --check` for every changed JavaScript file. At batch completion run:

```powershell
$env:CARGO_TARGET_DIR = "target-check-closed-loop"
cargo run -p ghostlight-lightbox -- run --all
```

## Per-task procedure

1. Re-read the active task's tree facts and every affected ADR.
2. Update `LEDGER.md` to mark only that task IN PROGRESS.
3. Implement only the task scope and its pinned tests.
4. Run the task checks and all common gates.
5. Update the ledger with results, deviations, and the commit id.
6. Commit with a conventional message ending in `(ADR-0078)`.

## Never

- Never alter a name, description, property order, enum order, or other trained field in the 13
  sacred schemas.
- Never use page text, accessible names, values, candidate scores, or target geometry as policy
  inputs or audit payloads.
- Never claim an action caused an observed change or that a remote operation committed.
- Never add cross-origin frame refs in this batch.
- Never add headless, isolated-profile, cloud, remote, or phone-home behavior.
- Never put policy, RAWX classification, audit, or grant logic in the extension.
- Never make diagnostics enable console or network capture by default.

## Completion criteria

The batch is complete when C1-C6 are committed, all common gates and Lightbox pass, `LIVE-VERIFY.md`
is completed against the visible local browser, tool inventory documentation is synchronized, and
`docs/STATUS.md` records the shipped state. Cross-origin frame work remains an explicitly separate
ADR candidate.
