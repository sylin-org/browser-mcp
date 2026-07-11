# W7: msa.md + dpa.md -- public drafts behind the counsel gate (ADR-0057 D8)

## Goal
The two legal templates, published as drafts. These are TEMPLATES for review, not offers; the
banner makes that unmistakable.

## Preconditions (verify, else STOP)
- W1 DONE. `LICENSE-GOVERNANCE` exists at the repo root.

## Required banner (VERBATIM, first lines of BOTH files, right after the H1)
```
> **DRAFT -- template pending counsel review. Not an offer.** This document is published for
> transparency and early review. It becomes binding only when executed in writing by both
> parties after legal review.
```

## Required content: `docs/trust/msa.md`
A standard-shape master software agreement TEMPLATE at modest length (~12-18 sections), plain
language, including at minimum: definitions; license grant referencing the Ghostlight Commercial
License (LICENSE-GOVERNANCE) for the governance module and Apache-2.0 OR MIT for the engine;
support terms BY REFERENCE to support-policy.md (do not restate numbers); the Continuity Promise
BY REFERENCE to continuity.md; warranties/disclaimers; limitation of liability (placeholder caps
marked `[TO BE COMPLETED IN REVIEW]`); term/termination (termination never disables the software
-- cite ADR-0028); governing law `[TO BE COMPLETED IN REVIEW]`; notices (support@sylin.org).
Placeholders are square-bracketed and UPPERCASE so nothing reads as final. Footer.

## Required content: `docs/trust/dpa.md`
The no-processing DPA per ADR-0057 D8: recitals establishing the structural fact (the vendor
receives, stores, and processes NO customer personal data; cite data-flows.md and ADR-0028 D9);
therefore controller/processor clauses are stated as NOT ENGAGED, with a short conditional section
("if a future service ever introduced vendor processing, a conventional DPA would be executed
first"); sub-processors: none (cite sub-processors.md); international transfers: none; breach
notification: the vendor-side-compromise advisory commitment BY REFERENCE to
security-overview.md. Keep it SHORT -- its brevity is the point. Footer.

## Verification (literal)
- `rg -c "DRAFT -- template pending counsel review" docs/trust/msa.md docs/trust/dpa.md` -> 1 each.
- `rg -c "TO BE COMPLETED IN REVIEW" docs/trust/msa.md` -> >= 2.
- `rg -n "no customer personal data|NO customer personal data" -i docs/trust/dpa.md` -> >=1.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W7 MSA and no-processing DPA drafts behind the counsel gate (ADR-0057 D8)`
