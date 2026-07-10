# W6: support-policy.md + tiers.md + PLAN.md pricing sync (ADR-0057 D7)

## Goal
The support commitment, the tier-claims-to-features map, and consistency with the public pricing
text.

## Preconditions (verify, else STOP)
- W1 DONE. `docs/business/PLAN.md` exists and contains tier/pricing text mentioning support
  response times (`rg -ni "business.day|support" docs/business/PLAN.md` -- read the hits first).

## Required content: `docs/trust/support-policy.md`
1. `## Channel` -- support@sylin.org; who reads it (the maintainer); what belongs there vs
   SECURITY.md (vulnerabilities NEVER go to the support lane).
2. `## Response commitment` -- the D7 pins verbatim in substance: ACKNOWLEDGMENT within 3 business
   days (Team) / 2 business days (Enterprise); the clock measures first human acknowledgment, not
   resolution; define business days Monday-Friday and STATE THE TIMEZONE as UTC; "typically much
   faster" may appear exactly once, as color.
3. `## Severity and scope` -- what support covers (install, configuration, policy authoring,
   managed:// deployment) and does not (custom development, the customer's own MCP client/model).
4. `## Enterprise extras` -- deployment help and roadmap input, matching the pricing page's words.
5. Footer.

## Required content: `docs/trust/tiers.md`
A table: each pricing-page claim -> the shipped feature -> the guide/evidence link. Rows at
minimum: Central policy -> managed:// (ADR-0055; governance-configuration guide); SIEM audit ->
syslog/file audit + policy_seq (siem-integration guide; the banned-claims wording); email support
-> support-policy.md; security questionnaires -> questionnaire.md + faq.md; MSA -> msa.md; DPA ->
dpa.md; deployment help + roadmap input -> support-policy.md Enterprise extras. Plus one sentence:
seats and licensee are legal terms, never enforced at runtime (ADR-0028). Footer.

## Required change: `docs/business/PLAN.md`
Update ONLY the support-response wording to 3/2 business days (acknowledgment) so PLAN.md and
support-policy.md agree. Touch nothing else in the file. If PLAN.md states 2-day/1-day, replace
those phrases; if it states no times, add none (record a deviation instead).

## Verification (literal)
- `rg -n "UTC" docs/trust/support-policy.md` -> >=1; `rg -c "typically" docs/trust/support-policy.md` -> <=1.
- `rg -ni "1.business.day" docs/business/PLAN.md docs/trust/` -> 0 hits.
- `rg -n "never enforced at runtime" docs/trust/tiers.md` -> 1 hit.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W6 support policy, tier map, pricing sync to 3/2 (ADR-0057 D7)`
