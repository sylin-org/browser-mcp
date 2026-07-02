# 0018. Governance ships observe-then-enforce

- Status: Accepted
- Date: 2026-07

## Context

The project is staged: stage 1 is engine correctness, stage 2 is the
governance layer (ADR-0013). The sequencing is deliberate. Debugging
governance and engine functionality at the same time is the swiss-cheese
double-layer scenario: a misbehavior could live in either layer, and every
diagnosis would have to cross both.

The same risk repeats inside stage 2 itself. If enforcement, its manifest
machinery, and its audit trail all land at once, a wrong denial cannot be
distinguished from a wrong grant resolution or a wrong URL match without
tooling that does not exist yet.

## Decision

Stage 2 lands in three steps, each observable before anything enforces:

1. **Audit flight recorder first.** Structured per-call records (identity,
   domain, tool, read/write class, decision, timing) written by the binary for
   every call, permitted and denied alike. Pure observation, zero behavior
   change; it exercises the dispatch choke point and produces the forensic
   layer everything later is debugged against.
2. **Sacred domains and the kill switch.** A user-authored never-touch domain
   list enforced in the binary, plus a take-the-wheel pause and panic
   kill-switch honored mid-action. Small, self-contained enforcement, landed
   with the audit layer already watching it.
3. **Full manifest engine.** Identity-bound domain grants, observe-vs-mutate
   classification down to `computer` sub-actions, tool advertisement
   filtering, and an observe-only mode. The URL matcher ships with the
   CVE-2025-47241 userinfo-bypass class and redirect handling as published
   test cases.

Until each step ships, public copy states plainly that governance is staged
and not yet shipped. The truthful-engine principle applies to marketing too.

## Consequences

- Positive: each layer is debuggable in isolation, and audit data exists
  before grants do, so grant resolution is verified against recorded reality
  rather than intuition.
- Positive: steps 1 and 2 deliver user-facing value (the flight recorder and
  the kill switch) without waiting for the manifest engine.
- Negative: the differentiating moat (governance fused with real-session
  automation) is claimable but unshipped until step 3; positioning leads with
  engine wins until then.
- Follow-up: the audit record shape should be designed once and reused by the
  local activity ledger and session recap features.
