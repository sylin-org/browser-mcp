# Composition batch (ADR-0035..0038): LEDGER

Durable progress. One task = one commit. Update at the end of every task per BOOTSTRAP step 5.
A fresh executor resumes from RESUME HERE with no other context.

## RESUME HERE

**C2 is NEXT.** Baseline: dev @ 6c5d351 (ADRs amended + this batch authored). C1 committed.

## Log

Template per task:

```
### C<N>: <title> -- DONE (<commit>) | BLOCKED | SKIPPED
- Baseline test count -> new test count.
- What landed (2-4 sentences, concrete file names).
- Deviations: D1..Dn (or "none"). A deviation is ANY divergence from the task file or PINS,
  including renames, moved code, extra tests, or clarified wording.
```

### C1: audit orchestration keys -- DONE (pending commit)
- Baseline 587 -> 589.
- Appended `orchestrator`/`batch_id`/`step`/`dry_run` to `AuditRecord`
  (`src/governance/ports.rs`) after `held`; added `CallAudit::orchestrated`/`mark_dry_run`/
  `attribute_grant`/`set_batch_id` and the matching fields to `CallAudit`
  (`src/governance/dispatch.rs`); updated the three existing `AuditRecord {}` construction
  sites (`ports.rs::sample_audit_record`, `src/governance/audit/mod.rs::sample_record`,
  `dispatch.rs::build_record`); added the two named tests to `tests/audit_recorder.rs`;
  appended an "Orchestration fields (additive)" subsection to `docs/SPEC.md` section 7.
- Deviations:
  - D1: folded PINS SS3's trailing `// UUID v4 lowercase hyphenated` annotation into
    `batch_id`'s `///` doc comment instead of a trailing `//` line comment, matching this
    struct's existing doc-comment-only style.
  - D2: the task's tree-facts pointed at `grep "held"` across `tests/` to find every pinned
    full-record assertion; that missed two MORE pinned key-order assertions living in `src/`'s
    own `#[cfg(test)]` modules (`dispatch.rs::begin_complete_produces_the_allow_record_bytes`,
    `ports.rs::record_serializes_all_fields_in_shared_format_order`), only surfaced by the
    `cargo test` gate failing. Appended the four keys to both, and updated their "14-key"/
    "the 14-key AuditRecord order is unchanged" prose (and the same phrase in
    `tests/inbound_web_auth.rs`'s comment) to "18-key" for accuracy.
  - D3: gate commands were run with `CARGO_TARGET_DIR` pointed at an isolated scratch
    directory instead of the default `target/`, because Chrome's live native-messaging host
    (a real, currently-connected `ghostlight.exe`, respawned by Chrome on kill) held
    `target/debug/ghostlight.exe` open for the whole session. No source or test content
    changed by this; noted here since it applies to every task's gate runs in this batch.
