# C1: audit orchestration keys

Goal: the four additive AuditRecord keys and the CallAudit setters every later task stamps.
Normative: ADR-0035 D7/D8 (as amended: keys append after `held`), ADR-0036 D7, PINS SS3.

## Tree facts (as of authoring; re-read before editing)

- `src/governance/ports.rs:193` `pub struct AuditRecord` ends at `pub held: bool,` (14 fields:
  event_id, ts, identity, client, tool, action, capability, domain, decision, grant_id,
  denial_id, duration_ms, manifest, held). Serialization preserves field order (serde
  `preserve_order` noted at ports.rs ~241).
- `src/governance/dispatch.rs:278` `pub fn begin(...)`; `:483` `pub struct CallAudit` with
  methods set_domain/held/sacred_deny/dispatch_finished/landing_allow/landing_shadow_deny (and
  a completion path -- find `complete`).
- `tests/audit_recorder.rs` exists; other tests may pin full record lines (grep
  `"held"` across tests/ to find every pinned line).
- `docs/SPEC.md` contains the audit record format (the "shared format doc" sections 6.x).

## STOP preconditions

- STOP if AuditRecord's last field is not `held`, or if any field named orchestrator/batch_id/
  step/dry_run already exists.
- STOP if record serialization does NOT emit `held` on every record (the always-present style
  PINS SS3 relies on).

## Required behavior

1. Append to AuditRecord, after `held`, exactly PINS SS3's four fields with its doc comments
   and order: `orchestrator: Option<&'static str>`, `batch_id: Option<String>`,
   `step: Option<u32>`, `dry_run: bool`. Always serialized (null/false when unset), matching
   `held`'s always-present style.
2. CallAudit setters (PINS SS3): `orchestrated(&mut self, orchestrator: &'static str,
   batch_id: &str, step: Option<u32>)`, `mark_dry_run(&mut self)`,
   `attribute_grant(&mut self, grant_id: Option<String>)`, and
   `set_batch_id(&mut self, batch_id: &str)` (parent-record stamping; SS7). Each simply stores
   into the record under construction; no behavior change to any existing path.
3. Every existing construction site of AuditRecord (grep `AuditRecord {`) gains the four
   fields as None/None/None/false.
4. `docs/SPEC.md` audit-format section: append a short subsection "Orchestration fields
   (additive)" documenting the four keys, values, and that they are always present.

## Tests (by name; assertions verbatim)

- `tests/audit_recorder.rs::orchestration_keys_serialize_last_in_order`: build a record via the
  normal begin/complete path with no orchestration; assert the serialized line ends with
  `"held":false,"orchestrator":null,"batch_id":null,"step":null,"dry_run":false}`.
- `tests/audit_recorder.rs::orchestrated_setters_stamp_fields`: begin, call
  `orchestrated("script", "00000000-0000-4000-8000-000000000001", Some(3))`, `mark_dry_run()`,
  `attribute_grant(Some("g-1".into()))`, complete; assert the line contains
  `"orchestrator":"script"`, `"batch_id":"00000000-0000-4000-8000-000000000001"`, `"step":3`,
  `"dry_run":true`, `"grant_id":"g-1"`.
- Update any existing test pinning a full record line by APPENDING the four keys to its
  expected string (deviation-log each such test by name).

## Verification

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`

## Out of scope

No pipeline changes, no producers of these fields beyond tests, no SPEC edits beyond the one
subsection, nothing in src/ outside ports.rs + dispatch.rs.

Commit: `feat(audit): additive orchestration keys (orchestrator, batch_id, step, dry_run)`
