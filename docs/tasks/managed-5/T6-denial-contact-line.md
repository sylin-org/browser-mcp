# T6: Denials-as-doors -- org contact line (ADR-0055 D9)

## Goal
When managed governance denies an action AND the org published a contact, the denial message gains
ONE trailing line routing the human to their org. The `Denied (D-xxxxxxxx):` prefix, id scheme, and
existing message body are byte-identical; the line is APPENDED (the sole sanctioned denial change,
per BOOTSTRAP NEVER list).

## Preconditions (verify, else STOP)
- T2 DONE (sidecar readable).
- VERIFIED (2026-07-10 re-read): `Denied (D-` renders in MULTIPLE production sites (at least
  `browser/sacred.rs` for sacred denials and the governance enforcement path) -- there is NO single
  render function, so do NOT look for one. The correct append point is the PIPELINE's
  denial-emission chokepoint: `crates/core/src/mcp/pipeline.rs` is where a governance
  `Decision::Deny` message becomes the tool-result text sent to the client (its tests assert
  `text.starts_with("Denied (D-")`). Find where the deny message string is placed into the
  response (grep `Denied` and follow the non-test flow); STOP only if the pipeline has no single
  point where every denial message passes through.

## Required behavior
1. In denial.rs add a PURE function (doc cites ADR-0055 D9):
   `pub fn org_contact_line(org_name: Option<&str>, contact_value: &str) -> String` returning
   EXACTLY: `Questions about this policy? Contact {org_name or "your organization"}:
   {contact_value}` (one line, no trailing newline).
2. At the pipeline denial-emission chokepoint (found above; it is OUTSIDE src/governance/, so the
   a7 arch rules do not constrain the sidecar read there), AFTER the existing message is fully
   built: read the sidecar (`GovernancePaths::production()` -> `sidecar_path(managed_cache)` ->
   `read_sidecar`); if Some(status) and status.presentation has a non-empty contacts vec, append
   `"\n"` + `org_contact_line(org_name.as_deref(), &contacts[0].value)`. The pure function stays
   in governance/denial.rs. Record the exact chosen line site as a LEDGER note.
3. Absent bootstrap/sidecar/contacts: denial text byte-identical to before this task.

## Tests (denial.rs `mod tests`; pinned)
- `contact_line_with_org_name`: `org_contact_line(Some("Acme Security"), "security@acme.example")`
  == `Questions about this policy? Contact Acme Security: security@acme.example`.
- `contact_line_without_org_name`: `org_contact_line(None, "security@acme.example")` ==
  `Questions about this policy? Contact your organization: security@acme.example`.
- Existing denial tests byte-identical (no edits to them permitted).

## Verification (literal)
- `cargo test -p ghostlight-core --lib -- denial` -> all pass, existing included.
- `cargo test --workspace` -> the tool_enforcement suite MUST stay green (denials in those tests
  run without a managed bootstrap, so their strings are unchanged; if any fails, you appended
  unconditionally: STOP and fix).
- Global verification per BOOTSTRAP.

## Out of scope
- No change to denial IDs, audit denial records, or `explain` (T5 owns the Passport).

## Commit message (pinned)
`feat(managed): T6 denial org-contact door line (ADR-0055 D9)`
