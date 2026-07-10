# ADR-0057: The Open Trust Center -- procurement documentation as a public, evidence-linked surface

Status: Accepted (2026-07-10; owner: "I want the documentation set to be immediately and openly
available", FAQ front door proposed by the owner, north star: "ISO professionals should think 'man,
I wish all software procurement were like this'"). Realizes the enterprise document pack ADR-0028
Decision 8's provenance note already directed ("security questionnaires, MSA, DPA... backed by a
ready-to-go document pack that leads with the offline/no-phone-home/post-quantum posture"). Legal
templates carry a counsel-review gate before first execution.

## Context

The pricing tiers (ADR-0028 Decision 5) promise Team buyers central policy, SIEM audit, and email
support, and Enterprise buyers security questionnaires, MSA, DPA, faster support, deployment help,
and roadmap input. As of the managed-5 batch, the product claims are shipped and tested (managed://
central policy, syslog/file audit with `policy_seq` provenance, the Policy Passport). What is
missing is the procurement-facing layer: the documents a security reviewer, GRC analyst, or ISO
27001 auditor needs before a purchase can clear.

The industry default is to gate these behind an NDA and a sales call. Ghostlight's architecture
permits the opposite: the vendor never receives customer data (ADR-0028 D9, permanent), the
governance module is source-available, and reliability claims are backed by runnable lightbox
scenarios (ADR-0056 D5). Openness is therefore not a concession but the differentiator: every trust
answer can be a public URL with a citation.

## Decision

1. **`docs/trust/` is the Open Trust Center**: public, in the repo, versioned, shipping with
releases. The repo is the canonical home; the website renders the same files later. NO document in
it is ever gated behind an NDA, a form, or a sales contact (normative, permanent). The register is
PROFESSIONAL (ADR-0055 D9's split): clarity and precision, no mascot voice.

2. **Honesty over theater (normative).** No claim of certifications we do not hold, no enterprise
cosplay. The architecture is the compliance story: the runtime runs entirely on the customer's
infrastructure under the customer's existing certifications, and the vendor side holds almost
nothing to audit because nothing reaches the vendor. A dedicated "what we do not have" section
states plainly: no SOC 2 / ISO certification (yet), no commissioned penetration test (yet), seats
are legal terms never enforced at runtime, support is email-only, a solo-founder company -- each
with its mitigation (source-available, never-phone-home, the Continuity Promise).

3. **The FAQ is the front door** (owner-directed). The top procurement questions, answered
directly, ordered by how often they are asked. Each answer is (a) COPY-PASTE-READY: one
self-contained, quotable paragraph first, links after -- designed for the reviewer's actual
workflow of pasting into an assessment portal; (b) EVIDENCE-LINKED: it ends with an `Evidence:`
line naming the artifact (ADR, source file, test, release attestation, guide) that makes the
answer true.

4. **Executable evidence.** Wherever a lightbox scenario proves a claim, the answer links the
runnable command (e.g. Continuity -> `cargo run -p ghostlight-lightbox -- run
continuity-source-unreachable`; anti-rollback -> `rollback-guardian`; fail-closed ->
`fail-closed-cold-boot`). A reliability claim the auditor can execute is the center's signature
move; no bespoke demo environment is required beyond the repo itself.

5. **A controls orientation, not a certification claim.** `controls.md` maps Ghostlight's
properties to the frameworks reviewers actually assess against (ISO 27001 Annex A themes; SOC 2
CC-series orientation), phrased as "which of YOUR controls this touches and how" -- doing the
auditor's vendor-assessment mapping homework for them while stating explicitly that Ghostlight
itself is not certified.

6. **Document hygiene mirrors the product's own legibility.** Every trust document carries a
footer: `Last reviewed: <date> against v<version>`, plus the owner (support@sylin.org). The git
history is the documented change record -- the trust center has a changelog by construction, the
same freshness-and-provenance discipline managed:// gives policy.

7. **Support policy (owner-decided):** channel `support@sylin.org`; ACKNOWLEDGMENT within 3
business days (Team) / 2 business days (Enterprise) -- the clock measures first human
acknowledgment, not resolution; business days defined in the policy with the timezone stated;
"typically much faster" stated as color, never as the promise. Security vulnerability reports are
OUT of these lanes and follow SECURITY.md. The pricing references in `docs/business/PLAN.md` are
updated to 3/2 so the tier table and the trust center never disagree. Rationale: a promise a solo
founder can keep for years beats an impressive one that breaks the first sick day --
honesty-over-theater applied to support.

8. **Legal templates ship public as drafts.** `msa.md` and `dpa.md` are published in the trust
center marked `DRAFT -- template pending counsel review; not an offer` (the "immediately available"
promise applied to legal), and the ADR-0028 gate stands: counsel review before any first execution.
The DPA leads with the structural fact that the vendor processes no customer data; the
sub-processor list is empty and published as such.

9. **Inventory** (all under `docs/trust/`): `README.md` (index) + `faq.md` (front door) +
`security-overview.md`, `data-flows.md`, `sub-processors.md`, `supply-chain.md`, `continuity.md`,
`controls.md`, `questionnaire.md`, `support-policy.md`, `msa.md`, `dpa.md`, `tiers.md` (pricing
claims -> shipped features -> guides). Existing guides (`compliance-team`, `siem-integration`,
`governance-configuration`, `licensing`) are cited, not duplicated: one source of truth per fact.

## Consequences

- An enterprise reviewer can self-serve the entire assessment in one sitting, with citations; the
  first sales conversation starts after trust is established instead of being gated on it.
- Every claim is red-teamed against the tree before publication (the over-claim risk is the
  center's biggest liability: SIEM means syslog UDP + file today with HTTP deferred; enforcement
  and support scope stated exactly).
- Owner-side gates remain: counsel skim (MSA, DPA, LICENSE-GOVERNANCE) before first execution;
  pushing/publishing the docs (outward-facing) stays an owner action.
- The maintenance cost is a review footer per release touch -- absorbed into the release checklist.

## Provenance

Owner (2026-07-10): tiers quoted from the pricing page; "immediately and openly available";
support@sylin.org; response times widened to 3/2 business days (owner proposal, assistant endorsed
as honesty-over-theater); the FAQ front door is the owner's addition; the delight north star quoted
in Status. Assistant contributions ratified in session: copy-paste-ready + evidence-linked answer
format, executable evidence via lightbox, the controls orientation page, the "what we do not have"
section, and review footers. Extends ADR-0027 (open-core), ADR-0028 (tiers, Continuity Promise,
never-phone-home, document-pack direction), ADR-0055 D9 (professional register), ADR-0056 D5
(scenarios as executable spec).
