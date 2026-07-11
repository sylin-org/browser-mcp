# T4: doctor managed section -- reads the sidecar (ADR-0055 Impl.8)

## Goal
`ghostlight doctor` answers the admin's "did my policy propagate?" from the T2 sidecar: seq,
fetched-at, freshness, source state, and the guardian doors -- WITHOUT needing a live service
session. Professional register (ADR-0055 D9): plain, precise lines; no mascot voice.

## Preconditions (verify, else STOP)
- T2 is DONE (LEDGER). `governance::managed::status::{read_sidecar, sidecar_path, ManagedStatus}`
  exist.
- `crates/core/src/hub/manage/doctor.rs` has `fn governance_section_lines() -> Vec<String>` whose
  lines use the two-space-indent `format!("  {:<9}{}", ...)` style (grep it; mirror the EXACT
  existing indent/width convention you find -- if the license section uses a different width,
  match the governance section's own style).

## Required behavior
1. In doctor.rs add `fn managed_section_lines() -> Vec<String>` (private, doc comment cites
   ADR-0055 Impl.8):
   - `let paths = crate::governance::paths::GovernancePaths::production();`
   - If `!paths.managed_bootstrap.exists()`: return exactly one line: `  managed  not configured`.
   - Else read the sidecar via `read_sidecar(&sidecar_path(cache_path))` where cache_path is
     `paths.managed_cache` (if None: line `  managed  configured; no data directory`).
   - Sidecar None -> `  managed  configured; no status yet (service has not resolved it)`.
   - Sidecar Some(s) -> lines (exact formats):
     - `  managed  seq {seq} ({freshness}{reason}), fetched {fetched_at}` where `{seq}` prints the
       number or `-` when None; `{freshness}` is the sidecar string; `{reason}` is empty when
       stale_reason is None else `: {stale_reason}`.
     - `  source   {source}`
     - when presentation.org_name is Some: `  org      {org_name}`
     - when last_error is Some: `  note     {last_error}`
2. VERIFIED (2026-07-10 re-read): the caller is doctor.rs ~77-81 and PRINTS in a for loop:
   `println!("Governance:"); for line in governance_section_lines() { println!("{line}"); }`.
   Integration: immediately AFTER that loop add
   `for line in managed_section_lines() { println!("{line}"); }` (no `lines` vec exists; do not
   restructure the caller). The line format is `format!("  {:<9}{}", "managed", rest)` -- the
   {:<9}-padded label convention produces exactly the pinned literals in this file (the pure
   renderer pattern mirrors the existing `render_governance_status` at doctor.rs ~313).

## Tests
- doctor output is process-environment dependent (fixed paths), so pin PURE tests instead: extract
  `fn render_managed_status(s: &ManagedStatus) -> Vec<String>` (the Some(s) arm above, pure) and
  unit-test it in doctor.rs:
  - `managed_line_renders_fresh`: ManagedStatus{v:1, freshness:"fresh", stale_reason:None,
    seq:Some(6), fetched_at:"2026-07-10T14:02:00+00:00", source:"https://policy.example/x",
    presentation:None, last_error:None} -> first line EXACTLY
    `  managed  seq 6 (fresh), fetched 2026-07-10T14:02:00+00:00`.
  - `managed_line_renders_guardian_door`: freshness "last_known_good",
    stale_reason Some("rollback_refused"), seq Some(9) -> first line EXACTLY
    `  managed  seq 9 (last_known_good: rollback_refused), fetched 2026-07-10T14:02:00+00:00`.
- `managed_section_lines` itself: no test (touches production paths); the pure renderer carries
  the oracle.

## Verification (literal)
- `cargo test -p ghostlight-core --lib -- managed_line` -> both pass.
- Global verification per BOOTSTRAP.

## Out of scope
- No Console/web changes; no explain changes (T5); doctor's OTHER sections byte-identical.

## Commit message (pinned)
`feat(managed): T4 doctor managed section from the status sidecar (ADR-0055 Impl.8)`
