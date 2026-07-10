# Ghostlight Master Software Agreement (Template)

> **DRAFT -- template pending counsel review. Not an offer.** This document is published for
> transparency and early review. It becomes binding only when executed in writing by both
> parties after legal review.

This template sets out the terms on which Ghostlight software is licensed to a customer. It is
written in plain language and is meant to be read before it is negotiated. Bracketed, uppercase
items are placeholders to be settled during review.

## 1. Definitions

"Vendor" means the provider of the Ghostlight software. "Customer" means the organization
identified in the executed order. "Engine" means the Ghostlight automation engine, licensed
Apache-2.0 OR MIT. "Governance Module" means the Ghostlight governance components licensed under
the Ghostlight Commercial License (LICENSE-GOVERNANCE). "Software" means the Engine and the
Governance Module together. "Documentation" means the materials published in this repository,
including the trust center. "Order" means the ordering document that references this Agreement.

## 2. License grant

The Engine is licensed under Apache-2.0 OR MIT, and nothing in this Agreement narrows the
rights those licenses grant. The Governance Module is licensed under the Ghostlight Commercial
License (LICENSE-GOVERNANCE) for the term and scope stated in the Order. The Governance Module
is source-available: Customer may read and audit its source. The license is granted for
Customer's internal use in accordance with the Order.

## 3. Deployment and operation

The Software runs entirely on Customer infrastructure. Vendor operates no service in the path
of Customer's use and receives no Customer data through the Software, as described in
[data-flows.md](data-flows.md).

## 4. Fees

Fees, billing period, and any applicable seat or licensee counts are stated in the Order.
Amounts and payment terms are `[TO BE COMPLETED IN REVIEW]`. Seat and licensee figures are
contractual terms only and are never enforced by the Software at runtime.

## 5. Support

Vendor provides support as described in [support-policy.md](support-policy.md), including the
acknowledgment commitments and scope stated there. The specific commitments applicable to
Customer follow Customer's tier as identified in the Order.

## 6. Continuity

The Continuity Promise applies to this Agreement as described in [continuity.md](continuity.md).
The Software continues to operate regardless of license state, Vendor availability, or Vendor's
continued existence.

## 7. Intellectual property

As between the parties, Vendor retains all rights in the Software except for the licenses
expressly granted. Customer retains all rights in Customer's own data and configurations.

## 8. Confidentiality

Each party protects the other's non-public information disclosed under this Agreement using at
least reasonable care, and uses it only to perform under this Agreement. This section does not
apply to information that is public or independently developed.

## 9. Warranties

Vendor warrants that it has the right to grant the licenses in this Agreement. Except for that
warranty, the Software is provided "as is" to the fullest extent permitted by law.

## 10. Disclaimer

Except as expressly stated in Section 9, Vendor disclaims all other warranties, whether express
or implied, including implied warranties of merchantability and fitness for a particular
purpose.

## 11. Limitation of liability

Each party's aggregate liability under this Agreement is capped at `[TO BE COMPLETED IN
REVIEW]`, and neither party is liable for indirect or consequential damages, except for
`[TO BE COMPLETED IN REVIEW]` (for example, breach of confidentiality or infringement).

## 12. Term and termination

This Agreement runs for the term stated in the Order. Either party may terminate for material
breach not cured within a reasonable notice period. Termination ends the commercial license to
the Governance Module for future periods; it does not disable, degrade, or interrupt the
Software already deployed, consistent with the Continuity Promise and ADR-0028.

## 13. Governing law and disputes

This Agreement is governed by the laws of `[TO BE COMPLETED IN REVIEW]`, and the parties submit
to the venue stated in `[TO BE COMPLETED IN REVIEW]`.

## 14. Notices

Notices to Vendor are sent to support@sylin.org. Notices to Customer are sent to the contact in
the Order.

## 15. General

This Agreement, together with the Order, is the entire agreement between the parties on its
subject matter. If any provision is unenforceable, the rest remains in effect. Neither party
may assign this Agreement without the other's consent, except in connection with a merger or
sale of substantially all assets.

Last reviewed: 2026-07-10 against v0.5.4 | Contact: support@sylin.org
