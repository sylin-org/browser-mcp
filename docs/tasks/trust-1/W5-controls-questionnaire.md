# W5: controls.md + questionnaire.md (ADR-0057 D5/D11f/D11g)

## Goal
The auditor's two documents: the framework orientation, and the CAIQ-shaped self-assessment that
can be filed directly as due-diligence evidence (and later submitted to CSA STAR Level 1 as a copy
job).

## Preconditions (verify, else STOP)
- W1-W4 DONE (this task cites their files).

## Required content: `docs/trust/controls.md`
1. Opening paragraph: this page maps Ghostlight's properties to the frameworks reviewers assess
   against; Ghostlight holds NO certification (state plainly, cite README what-we-do-not-have).
2. `## ISO/IEC 27001 Annex A orientation` -- a table over the vendor-relevant themes: supplier
   relationships (A.5.19-5.23), incl. the D11f verbatim-style line "source access is a standing
   audit right"; for each theme: what the customer's control needs, what Ghostlight provides,
   where the evidence lives (link).
3. `## SOC 2 orientation` -- one short section: which trust-services criteria a reviewer would map
   to which Ghostlight properties, with the honest note that no SOC 2 report exists.
4. `## AI frameworks` -- EU AI Act (tool vendor; customer = deployer; Article 12/26 support via
   audit + the Policy Passport; the D11e no-legal-advice sentence), ISO/IEC 42001 (no cert;
   orientation only), NIST AI RMF (a short govern/map/measure/manage orientation).
5. Footer.

## Required content: `docs/trust/questionnaire.md`
CAIQ-SHAPED per D11g: group by CAIQ v4 domain names (A&A, AIS, BCR, CCC, CEK, DCS, DSP, GRC, HRS,
IAM, IPY, IVS, LOG, SEF, STA, TVM, UEM -- use the full names with the acronyms). For each domain:
either (a) the honest short answer for a no-SaaS vendor -- many are `N/A -- structurally
impossible: <reason>` per D11b -- or (b) the real answer with an Evidence link (supply-chain, IR,
SDLC, access-to-crown-jewels domains). Opening paragraph MUST state: this is a self-assessment in
CAIQ v4 shape, suitable for filing as vendor due diligence; a CSA STAR Level 1 registry submission
is planned (roadmap language, no date).

## Verification (literal)
- `rg -n "standing audit right" docs/trust/controls.md` -> 1 hit.
- `rg -c "N/A" docs/trust/questionnaire.md` -> >= 6.
- `rg -n "STAR Level 1" docs/trust/questionnaire.md` -> >= 1.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W5 controls orientation + CAIQ-shaped questionnaire (ADR-0057 D5)`
