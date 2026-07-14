# lightbox-legacy batch: BOOTSTRAP

Execution package for ADR-0056 Decision 3. The batch migrates every tagged spawn test into a named
Lightbox scenario without weakening the production composition root.

## Authority order

1. `docs/adr/0056-lightbox-injectable-composition-and-e2e-harness.md`.
2. This BOOTSTRAP and `LEDGER.md`.
3. The live tree. If a recorded test name no longer exists, reconcile the ledger before editing.

## Rules

- One legacy test maps to one named scenario or one written retirement reason.
- Keep the old ignored test, `scripts/test-e2e.ps1`, `scripts/test-e2e.sh`, and both CI paths until
  all 27 ledger rows are DONE.
- Process scenarios build `ghostlight` and `ghostlight-relay` in `target/lightbox-under-test` by
  default. `--reuse-cache` may use the active Cargo target only on a clean CI worker.
- Use injected core composition for fixed governance paths. Never add a production runtime override.
- Every child process must be killed and reaped on normal return and failure.
- The fast in-process tier stays under `cargo test --workspace`.
- ASCII only. Do not touch trained tool schemas.

## Verification per migration commit

1. Run each new named scenario directly.
2. Run its old ignored test directly and compare the asserted invariant.
3. Run `cargo fmt --check`, strict workspace clippy, workspace fast tests, and Lightbox `run --all`.
4. Update `LEDGER.md` with the commit and any deviations.

## Completion gate

Only after every row is DONE: remove the 27 old ignored tests and obsolete spawn-only helpers,
replace the old E2E CI job with Lightbox, retire both shell wrappers, and run the full replacement
job on Windows and Linux.
