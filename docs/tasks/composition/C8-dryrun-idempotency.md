# C8: script dry_run + the idempotency cache

> POST-HOC RECORD (2026-07-06). This task was NOT executed as written. What landed (c69f432):
> dry-run as a pipeline-level `run_tool_call` parameter on the real decision path, and NO
> idempotency cache. The departure was ratified after review: ADR-0035 D8 re-amended to the
> landed design (plus the navigate landing-caveat suffix, implemented separately), D9 marked
> not-taken and superseded by ADR-0040 (Proposed). The authoritative account is the C8 LEDGER
> entry; the body below is the original instruction, kept for history. Do not execute it.

Goal: pre-flight verdicts without execution; retry safety for orchestrated calls.
Normative: ADR-0035 D8/D9 (as amended: service-scoped cache), PINS SS8.

## Tree facts (as of authoring; re-read before editing)

- C7 committed: script.rs answers dry_run/idempotency_key with the placeholder corrective text.
- `src/hub/outbound/browser.rs`: the Browser handle (service-scoped; owns hold state,
  TOOL_TIMEOUT at :76). C1's `mark_dry_run` exists on CallAudit.

## STOP preconditions

- STOP if C7 is not committed, or if Browser cannot own a new field without breaking its
  Clone/sharing model (inspect how Browser is shared; wrap the cache in Arc inside Browser if
  Browser is Clone).

## Required behavior

1. `src/transport/mcp/idempotent.rs` per PINS SS8: the cache type, `run_idempotent`, in-flight
   join, TTL 600s, cap 64, lazy eviction. Cache instance owned by Browser (Arc-wrapped as
   needed).
2. script.rs: `idempotency_key` wraps the whole interpretation in `run_idempotent(browser,
   "script", key, ...)`; a replayed result gains top-level `"replayed": true` in BOTH the text
   rendering and structuredContent; replays write no audit records (the wrap sits OUTSIDE the
   audit-producing path).
3. dry_run per PINS SS8: no dispatch, no tool frames; per step evaluate registry lookup,
   schema validation, ref GRAMMAR validation, sacred + authorize verdicts (tab-URL probes
   allowed); statuses "would_allow" | "would_deny" | "indeterminate" (navigate -> indeterminate
   when the verdict depends on the landing; a reference whose value determines the touched
   resource -> indeterminate). Parent audit gets `mark_dry_run()`; NO step records. dry_run
   with idempotency_key: the key is IGNORED on dry runs (nothing to protect; document in the
   result text `(idempotency_key ignored on dry_run)`).

## Tests (by name; assertions verbatim)

- `idempotent.rs` inline (tokio::test):
  - `second_call_replays_without_second_run`: closure increments an AtomicU32; two sequential
    calls, same key -> counter == 1, second returns replayed == true.
  - `concurrent_duplicate_joins_in_flight`: two tasks race the same key on a slow future ->
    counter == 1, both results equal, exactly one replayed == false.
  - `different_keys_run_independently`: counter == 2, both replayed == false.
  - `eviction_beyond_64`: insert 65 distinct keys; the first no longer replays.
- `script.rs` inline:
  - `dry_run_produces_no_step_dispatches`: stub runner panics if called; dry_run over 2 steps
    returns 2 would_* statuses.
  - `replayed_flag_injected`: same key twice -> second compact result contains
    `"replayed": true`.
- Extend `tests/script_tool.rs` (integration, no extension needed -- dry runs never dispatch):
  - `dry_run_verdicts_without_step_records`: all-open; script dry_run true, steps
    `[find {tabId:0,query:"x"}, navigate {tabId:0,url:"https://example.com"}]` -> statuses
    exactly `["would_allow","indeterminate"]`; captured audit contains exactly ONE record
    (tool `"script"`, dry_run true, batch_id non-null) and no step records.
  - `idempotent_replay_writes_no_new_audit`: two identical script calls sharing an
    idempotency_key -> second result contains `"replayed": true`, and the audit record count
    does not grow between the two calls.

## Verification

Gates.

## Out of scope

form_fill's use of the cache (C10), per-session scoping, persistence across restarts.

Commit: `feat(tools): script dry_run verdicts + service-scoped idempotency cache (ADR-0035)`
