# T7: Audit provenance -- policy_seq on tool-call records (ADR-0055 Impl.9c)

## Goal
Every TOOL-CALL audit record made under managed governance carries the org-signed policy sequence:
"decision X was made under policy version N". Same additive channel as the ADR-0028 license stamp.
Session-event shapes FROZEN (untouched). All-open and non-managed streams byte-identical.

## Preconditions (verify, else STOP)
- T2 DONE. `rg -n "set_license_stamp" crates/core/src` shows the Recorder setter AND its hub call
  site inside the `governance_operational` block (hub/mod.rs). STOP if either is missing.
- VERIFIED (2026-07-10 re-read), your anchors in `crates/core/src/governance/audit/mod.rs`:
  the field `license_stamp: Mutex<Option<&'static str>>` (~line 40, initialized `Mutex::new(None)`
  in all four constructors), `pub fn set_license_stamp` (~127), and the tool-call-only gate (~168):
  `let stamp = if kind == "tool_call" { *self.license_stamp.lock()... } else { None };` followed by
  the serialization that appends the `"license"` key. Mirror EXACTLY: `policy_seq:
  Mutex<Option<u64>>` field (init None in the same four constructors), `pub fn set_policy_seq`, a
  parallel `let seq = if kind == "tool_call" { ... } else { None };`, and the same
  serialization-append mechanism for `"policy_seq"`. Re-read the region first; line numbers drift.
- Scope hint for the LIVE-update wiring: `hub::ServiceContext::from_startup` already clones
  `Arc<Recorder>` into a spawned config-subscription task -- the policy-subscription task (mcp/
  server.rs, near `store.policy()`) may have or may be given the same clone if its spawn site
  already receives the recorder or ServiceContext; a one-line clone there is sanctioned.

## Required behavior
1. Recorder gains `set_policy_seq(seq: Option<u64>)` mirroring `set_license_stamp`'s storage
   exactly (same synchronization primitive, same doc-comment style, cites ADR-0055 Impl.9c).
2. Tool-call record emission: where the license stamp is conditionally written into a TOOL-CALL
   record, also write `"policy_seq": <n>` when the stored seq is Some. Key name EXACTLY
   `policy_seq`. It must appear ONLY on tool-call records (grep the existing stamp's scoping --
   ADR-0028 scoped it to tool-call records after an e2e break; follow the identical scoping).
3. Hub wiring (hub/mod.rs, inside the existing `governance_operational` block): when
   `loaded_policy.origin == Some(ManifestOrigin::Managed)`, read the T2 sidecar
   (`GovernancePaths::production()` -> sidecar) and `recorder.set_policy_seq(status.seq)`; for any
   other origin call `set_policy_seq(None)` is NOT needed (default None).
4. LIVE update: locate the policy-subscription task (`rg -n "policy_changes|store.policy()"
   crates/core/src/mcp/server.rs`); where it reacts to a published policy, if the new policy's
   origin is Managed, re-read the sidecar and `recorder.set_policy_seq(status.seq)`; if origin is
   not Managed, `recorder.set_policy_seq(None)`. STOP if the subscription task has no recorder
   handle in scope (record BLOCKED; do not thread new parameters through public signatures without
   it being a one-line addition to an existing struct the task already receives).

## Tests (pinned)
- Recorder unit test `policy_seq_stamps_tool_call_records_only` beside the existing license-stamp
  tests (find them: `rg -n "license" crates/core/src/governance/audit -l`): build a recorder, set
  `set_policy_seq(Some(6))`, emit one tool-call record and one session event via the same
  test-harness calls the license-stamp tests use; assert the tool-call record JSON contains
  `"policy_seq":6` and the session event does NOT contain `policy_seq`.
- `no_seq_no_field`: without the setter, the tool-call record does NOT contain `policy_seq`
  (byte-identity for non-managed).

## Verification (literal)
- `cargo test -p ghostlight-core --lib -- policy_seq` -> both pass.
- `cargo test --workspace` -> all_open_golden and every audit-shape test green (any golden diff =
  you stamped outside tool-call records: STOP and fix).
- Global verification per BOOTSTRAP.

## Out of scope
- No session-event changes. No audit-destination changes. No seq in denial text.

## Commit message (pinned)
`feat(managed): T7 policy_seq provenance on tool-call audit records (ADR-0055 Impl.9c)`
