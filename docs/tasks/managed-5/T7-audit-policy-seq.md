# T7: Audit provenance -- policy_seq on tool-call records (ADR-0055 Impl.9c)

## Goal
Every TOOL-CALL audit record made under managed governance carries the org-signed policy sequence:
"decision X was made under policy version N". Same additive channel as the ADR-0028 license stamp.
Session-event shapes FROZEN (untouched). All-open and non-managed streams byte-identical.

## Preconditions (verify, else STOP)
- T2 DONE. `rg -n "set_license_stamp" crates/core/src` shows the Recorder setter AND its hub call
  site inside the `governance_operational` block (hub/mod.rs). STOP if either is missing.
- Confirm how the license stamp reaches tool-call records: read the Recorder + record-building code
  the setter feeds (follow `license` field usage from `set_license_stamp`). Your change mirrors it
  field-for-field.

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
