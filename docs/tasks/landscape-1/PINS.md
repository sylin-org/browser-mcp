# PINS: landscape-1 batch

Exact code-level shapes and computed oracles. Task files cite these sections (SS1..SS8).
Tree facts are AS OF AUTHORING (2026-07-07, dev @ 656259c); re-read the named files before
editing. If a pinned line number has drifted, locate the construct by its quoted text; if the
CONSTRUCT is absent, STOP.

## Provenance (decided questions; do not re-litigate)

- `sources` carries STEP INDEXES, not hosts; the host join is the consumer's (ADR-0042 D4).
- `sources` is `null` (not `[]`) whenever no in-band flow occurred (ADR-0042 D2).
- form_fill internals carry `sources: null` because fill values are model-supplied, an
  out-of-band source (ADR-0042 D1/D2).
- The supported protocol set is exactly `2024-11-05`, `2025-03-26`, `2025-06-18`; unknown or
  absent requests are answered with `2025-06-18` (ADR-0041 D5; docs/design/
  mcp-spec-currency-2026-07.md "THE finding").
- No flow enforcement of any kind in this batch (ADR-0042 D5 is a future ADR).

## SS1: resolver signature (L1)

`src/transport/mcp/refs.rs` line 23 today:

```rust
pub(crate) fn resolve_refs(args: &Value, structured: &[Option<Value>]) -> Result<Value, String> {
```

becomes:

```rust
pub(crate) fn resolve_refs(
    args: &Value,
    structured: &[Option<Value>],
) -> Result<(Value, Vec<u32>), String> {
```

The `Vec<u32>` is the set of 1-indexed step numbers whose structured results were substituted
from, SORTED ascending, DEDUPLICATED, EMPTY when no reference resolved. `$prev` contributes
the actual index it normalized to (`structured.len()` as u32). Internal plumbing: thread a
`&mut std::collections::BTreeSet<u32>` collector through `resolve_value`/`resolve_string` and
convert to `Vec<u32>` at the end (BTreeSet gives sorted+deduped for free). Record an insert at
the SAME point the substitution succeeds (in `resolve_string`, after the target's structured
value is confirmed present -- i.e. once the function can no longer return Err for THIS
reference... a path-miss later in the same string still returns Err, and an Err from any
reference discards the whole resolution, so collector state on the Err path is irrelevant).
The error cases, grammar, escape, and pass-through behavior are byte-identical: only the Ok
type changes.

Existing tests in refs.rs (all nine) update mechanically: `resolve_refs(...).unwrap()` becomes
`resolve_refs(...).unwrap().0` where only args are asserted. Err-path tests are untouched.

## SS2: interpreter and runner threading (L1)

`src/transport/mcp/script.rs`:

- The `StepRunner` trait's `run` (line 39) and `PipelineRunner::run` (line 56) gain one
  parameter, inserted between `orchestration` and `dry_run`:
  `sources: Option<Vec<u32>>`.
- The production resolution site (lines 239-258) destructures the new tuple:

```rust
let resolved = resolve_refs(&step_args, &structured);
let (step_args, step_sources) = match resolved {
    Ok(pair) => pair,
    Err(msg) => { /* existing error arm, unchanged */ }
};
```

- The dispatch site (lines 260-265) becomes:

```rust
let sources_opt = if step_sources.is_empty() {
    None
} else {
    Some(step_sources)
};
let outcome = runner.run(
    &tool,
    &step_args,
    Some(("script", &batch_id, step_no)),
    sources_opt,
    dry_run,
);
```

The empty-to-None mapping lives HERE and only here; `run_tool_call` stamps any `Some` it
receives as-is (SS4). Dry-run needs no special case: on a dry run no step has a structured
result, so any reference errors before dispatch (existing behavior, line 251) and a
reference-free step yields an empty set -> `None`. Do not add a `dry_run` conditional around
sources.

- `PipelineRunner::run` forwards `sources` into `run_tool_call` via the private
  `futures_await_block` helper (line 530; the actual `run_tool_call` call is at line 541), which
  gains the same parameter in the same position.
- Test scaffolding: `RecordedCall` (line 561) gains `sources: Option<Vec<u32>>`; `StubRunner`'s
  `run` (line 584) records it. Update every existing `runner.run(...)`-shaped call in tests by
  compiler guidance; existing assertions are untouched.

## SS3: pipeline and audit stamp (L1)

`src/transport/mcp/pipeline.rs`:

- `run_tool_call` (line 170) gains `sources: Option<Vec<u32>>` between `orchestration` and
  `dry_run`. Its two non-script callers pass `None`: the top-level call at line 70
  (`run_tool_call(browser, store, governance, name, &args, None, false)` becomes
  `..., None, None, false)`) and any other caller the compiler finds (as of authoring there are
  exactly two callers total: pipeline.rs:70 and script.rs:541's `futures_await_block` path; if
  the compiler finds a third, record it as a deviation and pass `None` there).
- Immediately after the existing orchestration stamp (lines 223-227), add:

```rust
if let Some(sources) = sources {
    audit.flow_sources(sources);
}
```

`src/transport/mcp/form_fill.rs` is NOT edited: its internals never enter `run_tool_call`
(module doc, form_fill.rs line 7) and its `CallAudit` instances default to `sources: None`.

`src/governance/dispatch.rs`:

- `CallAudit` (fields at lines 488-502) gains `sources: Option<Vec<u32>>` after `dry_run`;
  the constructor in `Governance::begin` (the initializer that sets `orchestrator: None` at
  line 295) gains `sources: None`.
- New setter, placed directly after `orchestrated` (line 550), comment style matching its
  neighbors:

```rust
/// Stamp the 1-indexed positions of the orchestrated steps whose structured results fed
/// this call's resolved arguments (the in-band flow record; sorted, deduplicated).
pub fn flow_sources(&mut self, sources: Vec<u32>) {
    self.sources = Some(sources);
}
```

- `build_record` (the constructor call at line 710 area) threads
  `sources: self.sources.clone()` -- `build_record` takes `&self` (the terminal methods call
  it before consuming themselves), so a move will not compile.

## SS4: the record and its serialization (L1)

`src/governance/ports.rs`: `AuditRecord` (line 193) gains, AFTER `dry_run` (line 243, the
current last field -- field order is part of the format):

```rust
/// On an orchestrated step whose resolved arguments drew on earlier steps' structured
/// results: the sorted, deduplicated, 1-indexed positions of those source steps. `None`
/// (serialized `null`) on every other record: parents, internals whose values are
/// caller-supplied, dry-run records, and steps that referenced nothing.
pub sources: Option<Vec<u32>>,
```

No serde attribute (the struct's existing fields use none; `None` serializes as `null`,
matching `orchestrator`/`batch_id`/`step`).

Every `AuditRecord { ... }` construction the compiler flags gains `sources: None` EXCEPT the
one in `build_record` (SS3). Known construction sites as of authoring: dispatch.rs
`build_record`, audit/mod.rs `sample_record` (line 190). Others found by the compiler: add
`sources: None` and record a deviation naming the file.

## SS5: pinned tests and oracles (L1)

Test names are exact; assertion VALUES are oracles (transcribe, never re-derive).

1. `refs.rs`, new test `sources_report_referenced_steps_sorted_deduped`:

```rust
let args = json!({"a": "$1.x", "b": "$prev.y", "c": "$1.x"});
let structured = vec![
    Some(json!({"x": 1, "y": 2})),
    Some(json!({"x": 10, "y": 20})),
];
let (resolved, sources) = resolve_refs(&args, &structured).unwrap();
assert_eq!(resolved, json!({"a": 1, "b": 20, "c": 1}));
assert_eq!(sources, vec![1, 2]);
```

2. `refs.rs`, new test `sources_empty_when_no_references`:

```rust
let (resolved, sources) = resolve_refs(&json!({"n": 5, "s": "plain"}), &[]).unwrap();
assert_eq!(resolved, json!({"n": 5, "s": "plain"}));
assert!(sources.is_empty());
```

3. `script.rs`, extend the EXISTING test `references_resolve_through_the_interpreter`
   (line 773) with two assertions on the recorded calls (adapt field access to `RecordedCall`'s
   actual shape; the VALUES are pinned):

```rust
assert_eq!(calls[0].sources, None, "step 1 referenced nothing");
assert_eq!(calls[1].sources, Some(vec![1]), "step 2 drew on step 1");
```

4. `src/governance/audit/mod.rs`, new test
   `sources_serializes_null_by_default_and_array_when_set`: serialize `sample_record(...)`
   through the existing file-recorder path (mirror `file_destination_appends_one_line_per_record`'s
   harness) and assert the line contains `"sources":null`; then a record with
   `sources: Some(vec![1, 3])` asserts the line contains `"sources":[1,3]`.

If any existing test asserts a FULL serialized audit record, its expected string gains
`,"sources":null` before the closing brace; record each such edit as a deviation naming the
test.

## SS6: the SPEC.md bullet (L1)

`docs/SPEC.md` line 481 today ends the audit-key bullets with the `dry_run` bullet. Append
directly after it, as one bullet:

```
- `sources`: on an orchestrated `script` step record, the sorted, deduplicated 1-indexed step
  numbers whose structured results fed this step's resolved arguments via `$prev`/`$N`
  references (ADR-0042); `null` everywhere else -- parent records, `form_fill` internals,
  dry-run records, and steps that referenced nothing. In-band flows only: data the model
  carries between calls in its own context is out of scope by design.
```

## SS7: protocol version negotiation (L2)

`src/transport/mcp/server.rs` line 160 today:

```rust
pub const PROTOCOL_VERSION: &str = "2024-11-05";
```

becomes:

```rust
/// MCP revisions this server implements, oldest first. The advertised surface uses only
/// features present in ALL of them beyond capability-gated additions (structuredContent /
/// outputSchema entered 2025-06-18); optional features are declared via `capabilities`, so
/// claiming a revision never claims its optional features.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18"];
/// The newest supported revision: offered when the client requests nothing or something
/// unknown (per the spec's version-negotiation rule).
pub const LATEST_PROTOCOL_VERSION: &str = "2025-06-18";
```

New pure function in server.rs (near `initialize_result`):

```rust
fn negotiate_protocol_version(requested: Option<&str>) -> &'static str {
    SUPPORTED_PROTOCOL_VERSIONS
        .iter()
        .find(|v| Some(**v) == requested)
        .copied()
        .unwrap_or(LATEST_PROTOCOL_VERSION)
}
```

`initialize_result` (line 614) gains a first parameter `requested: Option<&str>` and its
`"protocolVersion"` value becomes `negotiate_protocol_version(requested)`. Its ONE production
call site (find it by grep for `initialize_result(`; if more than one production call site
exists, STOP) extracts the request's `params.protocolVersion` as
`params.and_then(|p| p.get("protocolVersion")).and_then(Value::as_str)` and passes it.

Pinned unit tests (server.rs tests module), exact names and oracles:

```rust
#[test]
fn protocol_version_negotiation_echoes_supported() {
    assert_eq!(negotiate_protocol_version(Some("2024-11-05")), "2024-11-05");
    assert_eq!(negotiate_protocol_version(Some("2025-03-26")), "2025-03-26");
    assert_eq!(negotiate_protocol_version(Some("2025-06-18")), "2025-06-18");
}

#[test]
fn protocol_version_negotiation_offers_latest_for_unknown() {
    assert_eq!(negotiate_protocol_version(Some("9999-01-01")), "2025-06-18");
}

#[test]
fn protocol_version_negotiation_offers_latest_when_absent() {
    assert_eq!(negotiate_protocol_version(None), "2025-06-18");
}
```

Pinned integration-test edit: `tests/mcp_protocol.rs` line 117 today is
`assert_eq!(init["result"]["protocolVersion"], "2024-11-05");` -- its initialize request
(line 102) sends `"params":{}` (no version), so the expected value becomes `"2025-06-18"`.
That line is the ONLY expected-value change in tests/; if any OTHER test fails on a version
string, STOP (an unpinned consumer exists).

## SS8: the org-rollout guide (L3)

New file `docs/guides/org-rollout.md` (no SPDX header; match the existing guides' plain style,
ASCII, no em-dashes). Pinned outline (H2 sections, in order):

1. `## What you are rolling out` -- the binary + extension pair, the policy file, org locks;
   one paragraph each, citing docs/SPEC.md and ADR-0019/0020 by relative link.
2. `## Pushing policy to a fleet` -- the file-path table per platform (draw from
   docs/guides/compliance-team.md and ADR-0020; do not invent new paths: every path named must
   appear in one of those two sources).
3. `## Rollout: observe, then shadow, then enforce` -- the ADR-0018 sequence with `explain`
   and `policy simulate` as the operator's tools at each stage.
4. `## Audit and SIEM` -- two sentences and a link to docs/guides/siem-integration.md.
5. `## The compliance one-pager` -- what the audit record proves (identity, tool, capability,
   domain, decision, grant/denial id, orchestration, sources), the EU AI Act high-risk
   obligations timing (August 2026), and the honesty fence VERBATIM in spirit: in-band flows
   only, no content inspection, no DLP claims (ADR-0042 Decision 1 language is binding).

Cross-links to add (both, exactly):

- `README.md` documentation table (the two-column table whose rows look like
  `| [docs/guides/solo-developer.md](docs/guides/solo-developer.md) | ... |`): add, adjacent
  to the other guide rows, the row:
  `| [docs/guides/org-rollout.md](docs/guides/org-rollout.md)           | Rolling Ghostlight out to a fleet: policy push, org locks, observe-shadow-enforce, and the compliance one-pager. |`
  (If the README table does not contain guide rows, add the row to the same table that lists
  open-spec/ and docs/research/NORTH-STAR.md.)
- `docs/guides/compliance-team.md`: one sentence at the top pointing to org-rollout.md for
  fleet deployment mechanics.

Claims fence for L3: no pricing, no license terms, no availability promises, no security
claims beyond what ADRs 0011/0018/0042 and the mapping doc
(open-spec/rawx-owasp-agentic-mapping.md) already state. When in doubt, link instead of
restating.

## Commit messages (exact)

- L1: `feat(audit): origin-flow provenance -- the sources audit key (ADR-0042)`
- L2: `feat(mcp): negotiate protocolVersion over a supported set (ADR-0041 D5)`
- L3: `docs(guides): org rollout guide -- policy push, staged enforcement, compliance one-pager`
