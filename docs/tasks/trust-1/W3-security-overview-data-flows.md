# W3: security-overview.md + data-flows.md (ADR-0057 D2/D11/D12)

## Goal
The architecture whitepaper and the data-flow statement -- the two documents the security-persona
reviewer reads after the FAQ.

## Preconditions (verify, else STOP)
- W1 DONE. `docs/SPEC.md` exists; `rg -n "never-touch|sacred" docs/SPEC.md` hits;
  `rg -n "Ed25519|ML-DSA" docs/adr/0055-managed-scheme-central-policy-distribution.md` hits.

## Required content: `docs/trust/security-overview.md`
H2s pinned in order; prose yours per BOOTSTRAP:
1. `## Architecture and trust boundaries` -- three processes, two protocol boundaries (MCP client
   <-> binary <-> extension <-> browser); everything local; the agent/model belongs to the
   customer.
2. `## The governance layer` -- capability classification (read/action/write/execute), grants,
   sacred never-touch domains (even against org policy), observe/shadow/enforce, take-the-wheel,
   panic kill, audit.
3. `## Cryptography` -- composite Ed25519 + ML-DSA-65 signatures for licenses and managed policy
   bundles; signature-anchored trust (transport is not the trust anchor); anti-rollback sequence;
   the signed, verified-on-load policy cache. MUST NOT claim at-rest encryption (banned list).
4. `## Vendor-side security` -- the D11a crown-jewels section: source repo, signing keys
   (air-gapped signing, ADR-0028 D10), release pipeline; MFA/least-privilege statement; change
   management = ADRs + CI gates + signed releases.
5. `## Incident response` -- the inverted commitment: advisories for vendor-side compromise
   (build/signing/update channel) within 3 business days of confirmation; SECURITY.md channel;
   MUST include the publish-all-audits pledge as a verbatim-style commitment sentence: "Any
   third-party security audit of Ghostlight will be published in full, including findings."
6. Footer.

## Required content: `docs/trust/data-flows.md`
1. `## What runs where` -- binary on the endpoint, extension in Chrome, no vendor service.
2. `## Flows that exist` -- a plain table: MCP client <-> binary (stdio, local); binary <->
   extension (native messaging, local); extension <-> pages (the user's own session); audit ->
   customer-configured destinations (file, syslog UDP; "none"/stderr options); managed policy
   fetch <- customer's own endpoint (only when the org configures it).
3. `## Flows that do not exist` -- vendor telemetry, licensing callbacks, update phone-home,
   model-provider calls: each named and denied with the ADR citation (ADR-0028 D9).
4. `## Local artifacts` -- audit files, policy cache + status sidecar (signed, verified-on-load),
   config files; owned/retained by the customer.
5. Footer.

## Verification (literal)
- `rg -n "publish(ed)? in full, including findings" docs/trust/security-overview.md` -> 1 hit.
- `rg -ni "encrypt(ed|ion) at rest" docs/trust/*.md` -> 0 hits.
- `rg -n "## Flows that do not exist" docs/trust/data-flows.md` -> 1 hit.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W3 security overview and data flows (ADR-0057 D11/D12)`
