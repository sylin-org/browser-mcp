# Composition batch (ADR-0035..0038): LEDGER

Durable progress. One task = one commit. Update at the end of every task per BOOTSTRAP step 5.
A fresh executor resumes from RESUME HERE with no other context.

## RESUME HERE

**C4 is NEXT.** Baseline: dev @ 6c5d351 (ADRs amended + this batch authored). C1, C2, C3
committed.

## Log

Template per task:

```
### C<N>: <title> -- DONE (<commit>) | BLOCKED | SKIPPED
- Baseline test count -> new test count.
- What landed (2-4 sentences, concrete file names).
- Deviations: D1..Dn (or "none"). A deviation is ANY divergence from the task file or PINS,
  including renames, moved code, extra tests, or clarified wording.
```

### C1: audit orchestration keys -- DONE (2c7a65c)
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

### C2: CallOutcome split + async Handler::Local -- DONE (193d78f)
- Baseline 589 -> 591.
- New `src/transport/mcp/outcome.rs` (SPDX Apache-2.0 OR MIT) holds `CallOutcome`,
  `DenialSource`, `LocalCtx`, `LocalFuture` (PINS SS2's sanctioned fallback placement, keeping
  `browser::directory` free of Browser/Governance/ConfigStore/Config imports); registered in
  `src/transport/mcp/mod.rs`. `directory.rs`'s `Handler::Local` grew from `fn() -> String` to
  `for<'a> fn(LocalCtx<'a>) -> LocalFuture<'a>`; `explain`'s row migrated to a capture-free
  closure coercing to that fn-pointer type. `pipeline.rs`'s `handle_tools_call` split into
  `run_tool_call(..., orchestration) -> CallOutcome` (the full stage-1..12 chokepoint) plus a
  thin `handle_tools_call` wrapper and `render_outcome` (the SS1 edge-render table); added
  `take_batch_id` (SS7's `_batch_id` side channel) and `is_free_local_action` (SS2's free-action
  guard: Local AND the `action:None` variant's requires is empty). Both Local dispatch
  positions now exist (free-action arm; post-grant arm for a future non-empty-requires Local
  tool, e.g. C10's `form_fill`) though nothing populates the second one yet. Added
  `calloutcome_render_table` and `local_batch_id_side_channel` to `pipeline.rs`'s test module.
- Deviations:
  - D1: `CallOutcome`/`DenialSource` are `pub`, not PINS SS1's literal `pub(crate)`. Forced by
    rustc's `private_interfaces` lint (promoted to a hard error by `-D warnings`):
    `directory::Handler` (and `ToolDescriptor`/`REGISTRY`) are already fully `pub` and reachable
    from `tests/*.rs` (separate crates), and `Handler::Local`'s fn-pointer variant names
    `LocalCtx`/`LocalFuture`/`CallOutcome` directly, so a `pub(crate)` `CallOutcome` behind a
    `pub enum Handler` cannot compile clean under this batch's gates. Confirmed no external
    test references `Handler` at all before widening (`grep -rn "Handler::" tests/` = 0 hits),
    so this is a safe, mechanically-forced widening, not a real API-surface expansion.
  - D2: `CallOutcome::Failure { error: ToolError }` (PINS SS1's literal shape) has no slot for
    the wait-note text that today's code appends to an ERROR result when the extension
    connected within the handshake grace window but the dispatched call still failed. No test
    pins this combination (`grep -rn "append_wait_note" tests/` = 0 hits); documented in a code
    comment at the `Err(e) => CallOutcome::Failure { error: e }` arm in `pipeline.rs` rather
    than silently dropped. The wait-note on a SUCCESS result is unaffected (still appended,
    still byte-identical).
  - D3: the `LocalFuture` import needed to live inside `pipeline.rs`'s `#[cfg(test)] mod tests`
    block, not the file's top-level `use` list: the type is named only by the new tests'
    explicit fn-pointer annotation, so a top-level import triggered `unused_imports` (also
    promoted to a hard error) in the non-test compilation pass.
  - D4: the `directory.rs` inline test at (pre-edit) line 1192 needed NO textual change --
    `matches!(row.handler, Handler::Local(_))` doesn't depend on the variant's inner type, so it
    compiles unchanged against the new fn-pointer shape.

### C3: structured results + outputSchema -- DONE (pending commit)
- Baseline 591 -> 592.
- `ToolDescriptor` gained `output_schema: Option<fn() -> Value>` (`src/browser/directory.rs`);
  all 14 rows updated (4 with a real minimal JSON-Schema: `tabs_context_mcp`, `tabs_create_mcp`,
  `navigate`, `find`; 10 with `None`); `advertised_tools_json` emits `"outputSchema"` when Some.
  Extension (`extension/service-worker.js`): `tabContext` now also sets
  `structuredContent = {mcpGroupId, tabs}`; `tabs_create_mcp` overrides it to
  `{tabId: <created tab>, tabs}` reusing the same `tabs` array; `navigate` sets
  `structuredContent = {tabId, url, title}` off the `chrome.tabs.get` call the handler already
  made; `find` builds `{results, more}` and attaches it on BOTH the empty and non-empty text
  branches. No text-rendering line changed (confirmed by re-reading each diff: only new
  `structuredContent`/`r.structuredContent` assignments added, no existing string literal
  touched). Added `tests/tool_schema_fidelity.rs::output_schemas_present_exactly_where_declared`.
- Verified the extension node gate (`constants`/`geometry`/`keys`.test.js, unaffected by this
  task's files) still passes: 17/17.
- Deviations: none. Neither `tool_schema_fidelity.rs` nor `all_open_golden.rs` byte-compares a
  whole per-tool JSON object (both index into specific keys), so the STOP precondition never
  applied and adding `outputSchema` required no test restructuring beyond the one new test.
