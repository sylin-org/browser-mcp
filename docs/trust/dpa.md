# Ghostlight Data Processing Addendum (Template)

> **DRAFT -- template pending counsel review. Not an offer.** This document is published for
> transparency and early review. It becomes binding only when executed in writing by both
> parties after legal review.

This addendum is short by design, because the structural fact it records is short: the Vendor
processes no customer personal data. Its brevity is the point.

## 1. Recitals

The Ghostlight software runs entirely on Customer infrastructure. The Vendor receives, stores,
and processes no customer personal data through the software: there is no vendor-side service in
the path of Customer's use, and no data flows to the Vendor, as established in
[data-flows.md](data-flows.md) and foreclosed by ADR-0028 Decision 9 (never phone home). This
addendum records that fact rather than papering over a data flow that does not exist.

## 2. Controller and processor roles

Because the Vendor processes no customer personal data, the controller-to-processor clauses of a
conventional DPA are NOT ENGAGED. There is no processing by the Vendor to instruct, restrict, or
audit. Customer remains the controller of any personal data Customer processes on its own
systems using the software; that processing is Customer's, not the Vendor's.

## 3. If vendor processing were ever introduced

If a future Ghostlight service ever introduced Vendor-side processing of customer personal data,
the parties would execute a conventional DPA covering that processing before it began. This
addendum does not authorize any such processing.

## 4. Sub-processors

None. The Vendor engages no sub-processors, because it processes no customer personal data. See
[sub-processors.md](sub-processors.md).

## 5. International transfers

None. No customer personal data is transferred to the Vendor, so no cross-border transfer of
such data to the Vendor occurs.

## 6. Breach notification

The Vendor holds no customer personal data to breach. For the vendor-side compromise scenario
that does apply (the build, signing keys, or update channel), the advisory commitment is stated
in [security-overview.md](security-overview.md).

Last reviewed: 2026-07-10 against v0.5.4 | Contact: support@sylin.org
