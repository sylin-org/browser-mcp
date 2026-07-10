# W1: Trust-center skeleton -- docs/trust/README.md (ADR-0057 D1/D3)

## Goal
The trust center's landing page: what it is, the never-gated promise, the evidence-linked method,
and the index of documents (some listed as "coming in this batch" until later tasks land).

## Preconditions (verify, else STOP)
- `docs/trust/` does not exist yet (`ls docs/trust` fails) or is empty.
- ADR-0057 exists and contains the section heading `## Research ratification`
  (`rg -n "Research ratification" docs/adr/0057-open-trust-center.md`).

## Required content: `docs/trust/README.md`
Headings pinned (H1 then H2s in this order); prose yours within BOOTSTRAP conventions:
1. `# Ghostlight Trust Center` -- opening paragraph MUST include these two verbatim sentences:
   "Every document here is public. Nothing in this trust center is gated behind an NDA, a form, or
   a sales call." and "Where a claim can be verified, the answer links the evidence -- an
   architecture decision record, a source file, a test, or a runnable scenario."
2. `## How to read this` -- the answer shape (quotable paragraph first, links after, `Evidence:`
   lines), the review footer convention, and one sentence noting the git history of this folder is
   the change record.
3. `## Documents` -- a table listing every inventory file from ADR-0057 D9 with a one-line
   description each (faq, security-overview, data-flows, sub-processors, supply-chain, continuity,
   controls, questionnaire, support-policy, msa, dpa, tiers). Link them all; files not yet written
   in this batch still get their row (they land in W2-W7).
4. `## What we do not have` -- the honesty section per ADR-0057 D2 and D12: no SOC 2 / ISO 27001 /
   ISO 42001 certification, no completed third-party penetration test, a solo-founder company;
   each stated as a fact with its reason/mitigation (architecture-as-evidence, source-available,
   the publish-all-audits pledge, pen-test-when-funded and certification roadmap).
5. Footer per BOOTSTRAP.

## Verification (literal)
- `rg -n "Nothing in this trust center is gated" docs/trust/README.md` -> 1 hit.
- `rg -c "\\]\\(" docs/trust/README.md` -> >= 12 (the index links).
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W1 trust-center index and never-gated charter (ADR-0057)`
