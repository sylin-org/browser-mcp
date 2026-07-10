# W9: Red-team pass + cross-links (ADR-0057 Consequences)

## Goal
The over-claim audit of everything W1-W8 wrote, then the two sanctioned cross-links. This task
exists because trust docs' biggest liability is a 95%-true claim.

## Preconditions (verify, else STOP)
- W1-W8 all DONE per LEDGER.

## Required pass 1: claim audit
For EVERY file in docs/trust/, re-verify every factual claim against the tree and the BOOTSTRAP
banned-claims list. Mechanical sweep first:
- `rg -ni "SOC 2|ISO 27001|ISO/IEC 42001|penetration test" docs/trust/` -- every hit must be a
  negation, orientation, or roadmap statement, never a possession claim.
- `rg -ni "encrypt" docs/trust/` -- no at-rest claims (transit/TLS statements and the
  SIGNED-cache wording are fine).
- `rg -ni "open source" docs/trust/` -- only about the Apache/MIT engine.
- `rg -n "ghostlight-lightbox -- run ([a-z-]+)" -o docs/trust/ -r '$1' | sort -u` -- every named
  scenario must appear in `crates/lightbox/src/scenarios.rs` (verify each with rg).
- Every relative link in docs/trust/ resolves: for each `](` target that is a repo path, verify
  the file exists (a short shell loop is fine; record the command used).
- Every file ends with the exact footer shape (BOOTSTRAP).
Fix violations in place; list EVERY fix in the LEDGER entry (file, original claim, corrected
claim). Zero fixes is a suspicious result -- state explicitly that the sweep ran clean if so.

## Required pass 2: cross-links (the ONLY files outside docs/trust/ this task touches)
- Root `README.md`: add ONE line/link to the trust center in whatever section lists documentation
  (mirror the existing style; do not restructure).
- `docs/guides/README.md`: add ONE line/link ("Trust Center (procurement and security review):
  ../trust/README.md" in the existing list style).

## Verification (literal)
- Re-run every W1-W8 verification command (they are all cheap rg checks); all still pass.
- `rg -n "docs/trust|trust/README" README.md docs/guides/README.md` -> >=1 each.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W9 over-claim red-team pass + cross-links (ADR-0057)`
