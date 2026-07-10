# T5: explain-tool Policy Passport section (ADR-0055 D9 / Impl.8)

## Goal
The `explain` MCP tool's output gains the Policy Passport when managed governance is active: who
governs this session, policy seq + freshness, the org rationale, sacred-domains reassurance, and
how to reach a human. Read from the T2 sidecar (the ManagedStatus store). Additive text ONLY --
`explain`'s existing output stays byte-identical when no managed bootstrap is configured.

## Preconditions (verify, else STOP)
- T2 DONE. `status::{read_sidecar, sidecar_path, ManagedStatus}` exist.
- Locate the explain TOOL handler: `rg -n "explain" crates/core/src --type rust -l` then find where
  the MCP tool named `explain` builds its response text (the deterministic renderer lives in
  `crates/core/src/governance/explain.rs`; the tool handler composes it). STOP if you cannot
  identify a single composition point that returns the explain tool's text.
- `rg -n "sacred_domains" crates/core/src/governance` hits (the config key exists) -- the Passport
  references sacredness only in PROSE; it does not read the key.

## Required behavior
1. In `crates/core/src/governance/explain.rs` add a PURE function (doc comment cites ADR-0055 D9):
   `pub fn managed_passport(status: &ManagedStatus) -> String` producing EXACTLY these lines
   (joined with `\n`, trailing newline at the end of the block):
   - `Managed governance: active.`
   - when presentation.org_name Some: `Governed by: {org_name}.`
   - `Policy version {seq}, {freshness_phrase}.` where freshness_phrase is:
     fresh -> `fetched {fetched_at} (current)`;
     last_known_good + source_unreachable -> `enforcing your last verified policy from
     {fetched_at} (the policy source is unreachable; you remain protected)`;
     last_known_good + update_rejected -> `enforcing your last verified policy from {fetched_at}
     (a newer update failed verification and was refused)`;
     last_known_good + rollback_refused -> `enforcing your last verified policy from {fetched_at}
     (an older policy was offered and refused)`;
     seq None prints `-` for {seq}.
   - when presentation.rationale Some: `Why: {rationale}`
   - `Sacred domains remain off-limits to automation under any policy, including this one.`
   - when contacts non-empty: `Questions? Contact {org_name or "your organization"}: {value of the
     first contact}` (label ignored in v1).
2. In the explain TOOL handler (the composition point found above): when
   `GovernancePaths::production().managed_bootstrap.exists()` AND the sidecar reads Some, append
   `"\n" + managed_passport(&status)` to the tool's existing text. Absent bootstrap or sidecar:
   append NOTHING (byte-identity).

## Tests (explain.rs `mod tests`; pinned)
- `passport_renders_fresh`: status (fresh, seq 6, fetched_at "2026-07-10T14:02:00+00:00", org_name
  "Acme Security", rationale "Baseline policy.", one contact value "security@acme.example") ->
  assert the EXACT full string:
  `Managed governance: active.\nGoverned by: Acme Security.\nPolicy version 6, fetched
  2026-07-10T14:02:00+00:00 (current).\nWhy: Baseline policy.\nSacred domains remain off-limits to
  automation under any policy, including this one.\nQuestions? Contact Acme Security:
  security@acme.example\n` (single string; the wrapping here is documentation -- the assertion is
  one literal).
- `passport_renders_guardian`: last_known_good/rollback_refused, no presentation -> contains
  `an older policy was offered and refused` and the sacred line; does NOT contain `Governed by`.

## Verification (literal)
- `cargo test -p ghostlight-core --lib -- passport` -> both pass.
- Global verification per BOOTSTRAP (all_open_golden and the advertised-set goldens MUST stay
  green -- if any golden diff appears, you appended in the wrong place: STOP).

## Out of scope
- No schema/description change to the `explain` tool (trained-adjacent surface: the tool's NAME,
  parameters, and description are untouched -- only its returned TEXT gains a section).
- No Console changes.

## Commit message (pinned)
`feat(managed): T5 explain-tool Policy Passport from the status sidecar (ADR-0055 D9)`
