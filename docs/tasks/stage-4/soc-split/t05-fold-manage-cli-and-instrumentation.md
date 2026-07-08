# t05: Fold `doctor`/`status`/`debug` under `manage/`; coherence pass

Cites: ADR-0033 Decision 2 (the manage plane is the whole operator surface). Needs t04 DONE.

## What this task is

The coherence completion: `manage` becomes the bounded context for the WHOLE operator surface,
not just the web UI. `doctor`, `status`, and the debug/instrumentation sinks move under
`src/hub/manage/` (or a `src/manage/` peer if the cross-cutting nature fits better there — see
Open question). This is the lowest-urgency phase; the urgent SoC break (inbound/manage-web) is
already done in t01-t04. This phase is pure coherence: making the bounded context match its
declared scope.

## Why last

It depends on nothing structural from t01-t04, but it reorganizes modules that those phases
didn't touch. Doing it last keeps the urgent fix unblocked and lets this phase proceed even if
t01-t04 ship first.

## Current-tree facts (re-verify)

- `src/doctor.rs` — the `doctor` subcommand's logic (read-only chain diagnosis).
- `src/debug.rs` — `DebugSink`, `log_dir`, `raw_state`, `status_report` (the snapshot/event log
  mechanism the service writes and `status`/`doctor` read).
- `main.rs` wires `Command::Doctor` / `Command::Status` to these.
- The support module `tests/support/mod.rs` polls `debug-state-*.json` snapshots as a readiness
  signal — this depends on the SNAPSHOT-FILE mechanism, not on any `debug` *subcommand*, so it is
  unaffected by a relocation of `src/debug.rs`.
- No integration test under `tests/` currently invokes a `doctor`, `status`, or `debug`
  subcommand (verified by the test survey). The `policy_*.rs` files are the template for any new
  CLI test.

## What changes

1. **Decide the home** (Open question below): either `src/hub/manage/{cli,instrumentation}.rs`
   (keeps manage under hub, consistent with t03/t04) or a top-level `src/manage/` peer (if the
   cross-cutting, runs-outside-the-service nature of the CLI subcommands argues for separation).
   Recommendation: `src/hub/manage/cli.rs` for the subcommand logic (it reads service state, so
   it belongs with the service's operator surface) and `src/hub/manage/instrumentation.rs` for the
   sink/log mechanism. Keep them under `hub/` for zone coherence; the CLI invocations still
   happen via `main.rs`'s existing subcommand wiring.
2. **Move `src/doctor.rs` → `src/hub/manage/cli.rs`** (or `manage/doctor.rs` if the file is
   large). Update `main.rs`'s `use` and the `Command::Doctor` dispatch. Update `lib.rs`'s
   `pub mod doctor;` declaration.
3. **Move `src/debug.rs` → `src/hub/manage/instrumentation.rs`**. Update consumers: `main.rs`
   (`build_debug_sink`, `init_tracing`'s `log_dir`), `src/hub/mod.rs` (`build_debug_sink`),
   `tests/support/mod.rs`'s `log_dir_for` / `newest_state` (these reference the snapshot file
   PATTERN, which is unchanged — only the `use` path of `ghostlight::debug::...` updates), and
   `tests/hub_identity.rs:120` (`ghostlight::debug::DebugSink::disabled()`).
4. **The `status` subcommand** (`run_status` in `main.rs`) reads `debug::raw_state` /
   `debug::status_report`; it flows through the new `manage::instrumentation` path. No behavioral
   change.
5. **Coherence pass**: re-read the whole `src/hub/` tree. Confirm the zone boundaries are clean:
   `inbound/` depends only on the pipeline; `outbound/` is consumed by the pipeline; `manage/`
   reads state directly and never enters the pipeline. If any stray cross-dependency leaked in
   during t01-t04, fix it here. Consider whether a formal `trait InboundChannel` /
   `trait OutboundCapability` is now worth introducing (the file structure is in place; the trait
   would lock the contract). Optional — defer if it adds abstraction without value.

## Tests

- No test moves (the survey found no doctor/status/debug subcommand tests).
- Add at least one CLI test per folded subcommand, modeled on `tests/policy_explain.rs`:
  `ghostlight doctor` (assert exit code / a known stdout line), `ghostlight status` (assert it
  reports "no debug state found" when no service is running). This is new coverage, not a migrate.
- `tests/support/mod.rs` continues to compile against the relocated `debug` module — its `use`
  path updates, the snapshot-file polling logic is unchanged.
- The four gates green.

## Verification

- All four gates green.
- `find src -maxdepth 1 -name "doctor.rs" -o -name "debug.rs"` returns nothing.
- `grep -rn "ghostlight::doctor\|ghostlight::debug" .` returns only updated paths.
- A manual `ghostlight doctor` and `ghostlight status` run produce the expected output.

## Out of scope

- The recursive grant grammar for `inbound`/`manage` — still deferred to its own ADR.
- A formal `InboundChannel` / `OutboundCapability` trait — optional coherence move; defer unless
  the file structure demands it for compile-time enforcement.
- Renaming `GHOSTLIGHT_DEBUG` / `GHOSTLIGHT_WEBAPI_PORT` env vars — separate decision, deferred
  (these are stable operator-facing names; renaming them is user-visible churn without a clear
  SoC payoff).

## Open question (resolve before starting)

Home for `manage`: under `src/hub/manage/` (recommendation — keeps the operator surface with the
service it operates on, and matches t03/t04's placement) or a top-level `src/manage/` peer (if
the cross-cutting, runs-outside-the-service nature of CLI subcommands argues for it). Lean: under
`hub/`. The CLI subcommands ARE the service's operator surface, even when invoked as a separate
process; they read service state and diagnose the service chain.
