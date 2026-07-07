# L1: the `sources` audit key (origin-flow provenance, phase 1)

Goal: every orchestrated `script` step's audit record names the earlier steps whose structured
results fed its resolved arguments, as the additive `sources` key; every other record carries
`sources: null`. ADR-0042 Decisions 2, 3, 4 are the semantics; PINS SS1-SS6 are the shapes and
oracles. No enforcement of any kind (ADR-0042 D5 is out of scope).

## STOP preconditions

- `docs/adr/0042-origin-flow-provenance.md` exists with Status: Accepted.
- `src/transport/mcp/refs.rs` contains
  `pub(crate) fn resolve_refs(args: &Value, structured: &[Option<Value>]) -> Result<Value, String>`.
- `src/transport/mcp/script.rs` contains the dispatch call
  `Some(("script", &batch_id, step_no)),` (the orchestration tuple).
- `src/governance/ports.rs`'s `AuditRecord` ends with the field `pub dry_run: bool,`.
- `src/governance/dispatch.rs` contains `pub fn orchestrated(&mut self,`.
- `docs/SPEC.md` contains a bullet beginning ``- `dry_run`:``.

Any of these absent: STOP (the tree has drifted past this batch's authoring; do not adapt).

## Tree facts (as of authoring, 2026-07-07; re-read before editing)

- refs.rs: resolver at line 23; nine unit tests, all calling `resolve_refs(...).unwrap()` or
  `.unwrap_err()`.
- script.rs: `StepRunner::run` line 39; `PipelineRunner::run` line 56; resolution site lines
  237-258; dispatch site lines 260-265; `RecordedCall` line 561; `StubRunner::run` line 584;
  test `references_resolve_through_the_interpreter` line 773.
- pipeline.rs: `run_tool_call` line 170; top-level caller line 70; orchestration stamp lines
  223-227.
- dispatch.rs: `CallAudit` fields lines 488-502; `begin`'s initializer sets `orchestrator: None`
  near line 295; `orchestrated` line 550; `build_record` threading near line 710.
- ports.rs: `AuditRecord` lines 193-244.
- audit/mod.rs: `sample_record` line 190; file-recorder serialization tests from line 231.
- form_fill.rs: does NOT call `run_tool_call` (module doc line 7); it is NOT edited.

## Required behavior

Implement PINS SS1 (resolver returns `(Value, Vec<u32>)`), SS2 (interpreter threads
`sources: Option<Vec<u32>>` through `StepRunner`/`PipelineRunner`, empty-to-None mapping at
the dispatch site, no dry-run conditional), SS3 (`run_tool_call` parameter + `flow_sources`
setter stamped right after the orchestration stamp; `begin` initializes `sources: None`), and
SS4 (the `AuditRecord` field, appended after `dry_run`, no serde attribute). Every
compiler-flagged `AuditRecord` construction gains `sources: None` (deviation-log any site not
named in SS4).

Doc-comment style in `src/governance/**` files: neutral vocabulary per BOOTSTRAP (the a7
architecture scan reads doc comments; never name transport/browser crate paths there).

## Tests to add (names and oracles pinned in PINS SS5; transcribe verbatim)

1. `refs.rs::sources_report_referenced_steps_sorted_deduped`
2. `refs.rs::sources_empty_when_no_references`
3. extend `script.rs::references_resolve_through_the_interpreter` (two pinned assertions)
4. `audit/mod.rs::sources_serializes_null_by_default_and_array_when_set`

Mechanical updates: refs.rs existing tests destructure `.0`; StubRunner/RecordedCall grow the
field; full-record expected strings gain `,"sources":null` (each one is a logged deviation).

## Documentation edit

Append the PINS SS6 bullet to docs/SPEC.md after the `dry_run` bullet. Nothing else in
SPEC.md changes.

## Verification (literal)

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture
```

All green, plus the extension regression line from BOOTSTRAP. Then commit exactly:

```
feat(audit): origin-flow provenance -- the sources audit key (ADR-0042)
```

## Out of scope (fences)

- No host names, domain joins, or flow edges anywhere in code (D4: the join is the
  consumer's).
- No changes to form_fill.rs, no `sources` on form_fill internals.
- No config keys, no manifest schema changes, no denial paths, no Console/ledger rendering.
- No changes to any tool schema, tools.json-derived surface, or the fidelity/golden tests.
