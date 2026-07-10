# Ghostlight Tiers: Claims to Evidence

This page maps each claim on the pricing page to the feature that delivers it and the evidence
that documents it. The intent is that a reviewer can trace every purchasable claim to something
concrete rather than a marketing line.

| Pricing-page claim | Shipped feature | Evidence |
| --- | --- | --- |
| Central policy | `managed://` signed central policy, provisioned by your MDM | [ADR-0055](../adr/0055-managed-scheme-central-policy-distribution.md), [governance configuration guide](../guides/governance-configuration.md) |
| SIEM audit | Identity-bound audit to syslog (RFC 5424 over UDP) or JSON Lines files, with `policy_seq` provenance; HTTP delivery is deferred | [SIEM integration guide](../guides/siem-integration.md) |
| Email support | Acknowledgment commitments and scope | [support-policy.md](support-policy.md) |
| Security questionnaires | A CAIQ v4-shaped self-assessment and the evidence-linked FAQ | [questionnaire.md](questionnaire.md), [faq.md](faq.md) |
| MSA | Master software agreement template (draft, pending counsel) | [msa.md](msa.md) |
| DPA | No-processing data processing addendum template (draft, pending counsel) | [dpa.md](dpa.md) |
| Deployment help and roadmap input | Enterprise extras | [support-policy.md](support-policy.md) |

Seats and licensee are legal terms in the agreement, never enforced at runtime: Ghostlight
never phones home, never counts seats, and license state never changes behavior (ADR-0028).

Last reviewed: 2026-07-10 against v0.5.4 | Contact: support@sylin.org
