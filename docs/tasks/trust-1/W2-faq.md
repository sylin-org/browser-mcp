# W2: faq.md -- the 22-question front door (ADR-0057 D3/D10/D11)

## Goal
`docs/trust/faq.md`: six sections, 22 questions, each answered in the copy-paste-ready +
`Evidence:` format. This is the center's front door and its hardest-working page.

## Preconditions (verify, else STOP)
- W1 DONE. The following evidence targets exist (verify EACH; if one is missing, STOP and record
  which): `docs/legal/PRIVACY.md`, `docs/legal/PERMISSION_JUSTIFICATIONS.md`, `SECURITY.md`,
  `docs/guides/siem-integration.md`, `docs/guides/governance-configuration.md`,
  `docs/guides/licensing.md`, `docs/adr/0028-tripwire-licensing-and-continuity-promise.md`,
  `docs/adr/0055-managed-scheme-central-policy-distribution.md`, `extension/manifest.json`.
- Lightbox scenarios exist: `rg -n "continuity-source-unreachable|rollback-guardian|fail-closed-cold-boot|managed-activation-local" crates/lightbox/src/scenarios.rs` -> 4+ hits.

## Required content
Six H2 sections with these EXACT questions as H3s, in this order. Answers: yours, within BOOTSTRAP
conventions and BANNED CLAIMS; each ends with an `Evidence:` line citing AT LEAST the targets
named in brackets (add more if true; never fewer). Where marked RUNNABLE, include the lightbox
command line.

`## Data and privacy`
1. `### Does any of our data ever reach the vendor?` -- No, structurally; zero vendor-bound
   traffic; audit destinations are customer-configured. [ADR-0028 D9; docs/legal/PRIVACY.md]
2. `### Is our data used to train AI models? Which model providers sit behind the product?` --
   Ghostlight calls no LLM; the model belongs to the customer via their MCP client. MUST include
   the sentence: "There is no model-provider client in Ghostlight's dependency tree." [Cargo.toml
   dependency tree; docs/SPEC.md architecture]
3. `### What can the browser extension access, and where does that data go?` -- content flows only
   to the local binary over native messaging; no cloud backend. [extension/manifest.json;
   docs/legal/PERMISSION_JUSTIFICATIONS.md; data-flows.md]
4. `### Who are your subprocessors?` -- None. [sub-processors.md]
5. `### Where is our data stored and processed, and how is it retained or deleted?` -- on customer
   infrastructure exclusively; customer policies govern; name the local artifacts (audit files,
   policy cache) and where they live. [data-flows.md]
6. `### Do you offer a DPA? How do you comply with GDPR/CCPA?` -- yes; the DPA attests no vendor
   processing occurs. [dpa.md; docs/legal/PRIVACY.md]

`## AI and agents`
7. `### How do you mitigate prompt injection, including indirect injection from web content?` --
   the honest framing per ADR-0057 D11d (unsolved industry-wide; governance bounds the blast
   radius: sacred domains, capability grants, modes, kill switch). [docs/SPEC.md; ADR-0022;
   governance-configuration guide]
8. `### What can the agent do autonomously? Can we pause or stop it mid-run?` -- capability
   classification, observe/shadow/enforce, take-the-wheel pause, panic kill switch. [docs/SPEC.md;
   governance-configuration guide]
9. `### What is logged per agent action? Does the audit record capture the policy state at decision time?` --
   identity-bound tool-call records; the org-signed policy sequence (`policy_seq`) stamps
   tool-call records under managed governance; syslog/file today, HTTP deferred (banned-claims
   wording). [siem-integration guide; ADR-0055 Impl.9c]
10. `### Can we enforce policy centrally across a fleet?` -- managed:// signed central policy,
    MDM-provisioned, last-known-good continuity. RUNNABLE: managed-activation-local.
    [ADR-0055; governance-configuration guide]
11. `### What is your posture under the EU AI Act, ISO/IEC 42001, and NIST AI RMF?` -- tool
    vendor / customer is deployer / supports Article 12 and 26 duties; no 42001 certificate;
    orientation in controls.md; the D11e no-legal-advice rule applies. [controls.md]
12. `### Analysts have advised blocking AI browsers. How is Ghostlight different?` -- the D11c
    Gartner flip: drives the user's own hardened Chrome under policy; not a replacement browser;
    agent actions are attributed in the audit trail. [docs/SPEC.md architecture; ADR-0001]

`## Security posture`
13. `### What certifications do you hold?` -- none yet; the D2/D12 honesty answer + roadmap;
    architecture-as-evidence. [README.md what-we-do-not-have; controls.md]
14. `### How do you secure your own infrastructure?` -- the D11a crown-jewels framing: source,
    signing keys (air-gapped, ADR-0028 D10), release pipeline; MFA and least privilege on those.
    [supply-chain.md; ADR-0028]
15. `### Has Ghostlight been penetration tested? How do you handle vulnerabilities?` -- not yet
    commissioned (when-funded); publish-all-audits pledge; source access = standing audit right
    (D11f); SECURITY.md disclosure channel. [SECURITY.md; security-overview.md]
16. `### What is your incident response and breach notification commitment?` -- the inverted
    framing (D11a): no customer data to breach; vendor-side compromise = build/signing/update
    channel; advisory commitment with a defined window (pin: security advisories within 3 business
    days of confirming a vendor-side compromise). [security-overview.md]

`## Continuity and viability`
17. `### What happens if the vendor disappears, or we stop paying?` -- Continuity Promise verbatim
    quote from ADR-0028; source-available beats escrow; expiry changes exactly one thing (the
    audit stamp). RUNNABLE: continuity-source-unreachable. [ADR-0028; licensing guide;
    continuity.md]
18. `### What are your BC/DR commitments?` -- inverted: nothing of the vendor's runs in the
    customer's path; last-known-good cache; fail-closed cold boot. RUNNABLE:
    fail-closed-cold-boot. [continuity.md; ADR-0055 D5]

`## Supply chain`
19. `### Do you provide an SBOM, signed releases, and build provenance?` -- per-release CycloneDX
    SBOM (W8), checksums + provenance attestations on releases, dependency posture. [supply-chain.md;
    the release workflow]
20. `### How do we review and force-install the extension?` -- per-permission justifications,
    Manifest V3 / no remote code, CWS listing, ExtensionInstallForcelist guidance.
    [docs/legal/PERMISSION_JUSTIFICATIONS.md; extension/manifest.json]

`## Legal and support`
21. `### What support do you commit to?` -- acknowledgment 3 business days (Team) / 2 (Enterprise),
    support@sylin.org; security reports via SECURITY.md. [support-policy.md]
22. `### What are the license terms, and what happens at expiry?` -- open-core split; the
    governance module is source-available (never "open source"); expiry -> stamp only.
    [docs/guides/licensing.md; ADR-0027; ADR-0028]

## Verification (literal)
- `rg -c "^### " docs/trust/faq.md` -> exactly 22.
- `rg -c "^## " docs/trust/faq.md` -> exactly 6.
- `rg -c "Evidence:" docs/trust/faq.md` -> exactly 22.
- `rg -n "ghostlight-lightbox -- run" docs/trust/faq.md` -> >= 3.
- `rg -n "There is no model-provider client" docs/trust/faq.md` -> 1.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W2 the 22-question FAQ front door (ADR-0057 D10)`
