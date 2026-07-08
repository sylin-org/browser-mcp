# C7: the script tool

Goal: sequential multi-tool composition with reference resolution, budget, honest per-step
status, and correlated audit. Normative: ADR-0035 D1-D7 (as amended), PINS SS6 + SS7 + SS14 +
SS4.

## Tree facts (as of authoring; re-read before editing)

- C1 (audit keys + setters), C2 (run_tool_call + CallOutcome + Local shape + orchestration
  parameter + _batch_id side channel), C3 (structuredContent), C4 (wait_for) are committed.
- `src/governance/config/mod.rs`: key constants at :352+ (`engine.connection.first_call_wait_ms`
  pattern), KeyDef rows at :424+, accessors at :717+ (`first_call_wait_ms`).
- `src/transport/mcp/validation.rs`: `ToolSchema::for_tool` drives pre-dispatch validation from
  the registry's inputSchema; its capability for nested array-item validation is UNVERIFIED
  (SS7 pins the fallback: validate steps' inner shape in the handler; that is NOT a STOP).

## STOP preconditions

- STOP if C2's `run_tool_call` does not exist with the orchestration parameter.
- STOP if `structuredContent` is not present on `find` results (C3 landed it).

## Required behavior

1. Config key + accessor per PINS SS14 (`engine.script.budget_ms`, UintRange 1000..480000,
   default 120000 in all three presets; `Config::script_budget_ms()`).
2. `src/transport/mcp/refs.rs` (SPDX Apache-2.0 OR MIT): `resolve_refs` per PINS SS6 exactly
   (grammar, `$$` escape, error strings).
3. `src/transport/mcp/script.rs`: the interpreter per PINS SS7: tabId inheritance, no-nesting
   rejection, ref resolution, per-step `run_tool_call(...,
   Some(("script", &batch_id, step_no)))`, status mapping, hold-stops-unconditionally,
   onError, budget clamp (arg may only lower the config value), per-step 2000-char / whole
   25000-char truncation, compact result text + identical structuredContent, `_batch_id` side
   channel for the parent record. `dry_run` and `idempotency_key` are ACCEPTED by the schema
   but answered with the corrective text
   `dry_run and idempotency_key land in the next engine release` -- C8 replaces that (this
   keeps C7 independently landable with the full schema stable from day one).
4. Directory row + advertised description + example per PINS SS7, inserted before explain;
   example call `{"steps":[{"tool":"find","args":{"tabId":0,"query":"submit button"}},{"tool":"computer","args":{"action":"left_click","ref":"$prev.results.0.ref"}}]}`,
   returns note "Each step's status (ok, error, denied, held, not_run), its text, and its
   structured result; a summary line; total duration_ms." output_schema Some (compact-result
   shape).

## Tests (by name; assertions verbatim)

- `refs.rs` inline unit tests: every PINS SS6 oracle, named
  `resolves_prev_path`, `double_dollar_escapes`, `non_grammar_dollar_passes_through`,
  `money_value_errors_with_escape_hint`, `forward_reference_errors`,
  `unstructured_step_errors`, `zero_index_passes_through`.
- `script.rs` inline unit tests (drive the interpreter with a stubbed step runner if
  `run_tool_call` needs a live Browser; a thin `trait StepRunner` seam INSIDE script.rs is
  sanctioned for testability):
  - `hold_stops_unconditionally_even_on_continue`: steps [ok, held, ok] with onError
    "continue" -> statuses ["ok","held","not_run"], summary `1/3 steps completed; held at step 2`.
  - `denied_step_reports_denied_not_ok`: [ok, denied] -> ["ok","denied"], summary
    `1/2 steps completed; step 2 denied`.
  - `budget_exhaustion_marks_not_run`: 3 steps, budget forcing stop after 1 -> ["ok","not_run",
    "not_run"], summary `1/3 steps completed; budget exhausted after step 1`.
  - `nested_script_step_errors`: step tool "script" -> status "error", text contains
    `script steps may not include script itself`.
  - `truncation_applies_at_2000`: a 3000-char step text ends with `(truncated)` and total step
    text length <= 2011.
- NEW integration test `tests/script_tool.rs` (follow the harness patterns of
  `tests/audit_recorder.rs` / `tests/tool_enforcement.rs`; no extension is connected, which is
  the point):
  - `script_reports_step_error_and_not_run_with_correlated_audit`: all-open; call script with
    steps `[navigate {tabId:0,url:"https://example.com"}, find {tabId:0,query:"x"}]`. Assert:
    step 1 status `"error"` (its text contains `extension`), step 2 `"not_run"`, summary
    exactly `0/2 steps completed; step 1 failed`. Captured audit: exactly one record with
    tool `"script"` (batch_id non-null, step null, orchestrator null); exactly one record with
    tool `"navigate"` carrying orchestrator `"script"`, the SAME batch_id, step 1; NO record
    for `find`.
- `tests/tool_schema_fidelity.rs` + `tests/all_open_golden.rs` + directory inline name test:
  cumulative arrays per PINS SS4 after C7.
- `tests/config_schema_golden.rs` (exists): extend for the new key exactly as its pattern
  requires (log the added expected line as a deviation-free pinned edit).

## Verification

Gates. Then a smoke assertion via `cargo test --test tool_enforcement` (script's parent is a
free action; no enforcement regressions).

## Out of scope

dry_run/idempotency EXECUTION (C8), parallel mode, named steps, saved scripts (ADR-0039 --
NEVER), image store.

Commit: `feat(tools): script -- sequential composition with structured references (ADR-0035)`
