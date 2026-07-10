# T6: Denials-as-doors -- org contact line (ADR-0055 D9)

## Goal
When managed governance denies an action AND the org published a contact, the denial message gains
ONE trailing line routing the human to their org. The `Denied (D-xxxxxxxx):` prefix, id scheme, and
existing message body are byte-identical; the line is APPENDED (the sole sanctioned denial change,
per BOOTSTRAP NEVER list).

## Preconditions (verify, else STOP)
- T2 DONE (sidecar readable).
- `crates/core/src/governance/denial.rs` exists and owns denial message formatting. Locate the
  single function that renders the final user-facing denial string (grep `Denied (D-` -- STOP if
  the literal renders in more than one place).

## Required behavior
1. In denial.rs add a PURE function (doc cites ADR-0055 D9):
   `pub fn org_contact_line(org_name: Option<&str>, contact_value: &str) -> String` returning
   EXACTLY: `Questions about this policy? Contact {org_name or "your organization"}:
   {contact_value}` (one line, no trailing newline).
2. At the denial-rendering composition point (found above), AFTER the existing message is fully
   built: read the sidecar (same pattern as T4/T5: `GovernancePaths::production()` ->
   `sidecar_path` -> `read_sidecar`); if Some(status) and status.presentation has a non-empty
   contacts vec, append `"\n" + org_contact_line(org_name.as_deref(), &contacts[0].value)`.
   IMPORTANT: if the denial renderer is inside `src/governance/` and the a7 arch rules make the
   production-paths read awkward at that layer, do the append at the TRANSPORT-side call site that
   emits the denial to the client instead (one place; grep where denial text reaches the MCP
   response) -- the pure function stays in denial.rs either way. Record which site you chose as a
   LEDGER deviation note (both are sanctioned).
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
