# 0042. Origin-flow provenance: audit-first cross-host data flow

- Status: Accepted (phase 1: the `sources` audit key). Phase 2 (flow enforcement) is
  direction-pinned only; its details require a future ADR.

## Relationship to other decisions

- BUILDS ON ADR-0035 (`script`): reference substitution (`$prev`/`$N`) is today the ONLY
  in-band data flow in the entire tool surface, which makes it the one place provenance can be
  attested truthfully.
- BUILDS ON ADR-0038 (structured results): references resolve against `structuredContent`, so
  the substrate already knows exactly which step fed which argument.
- BUILDS ON the composition batch's orchestration audit keys (`orchestrator`, `batch_id`,
  `step`): provenance is the next additive key in that family, and joins against them.
- APPLIES ADR-0018's observe-then-enforce pattern to flows: record reality first, gate later.
- HONORS ADR-0011 (truthful engine): the engine never claims to see flows it cannot see.
- FEEDS ADR-0039 Decision 6: the retrospective saved-script path reconstructs workflows from
  the audit stream; with provenance in the stream, a reconstructed script carries its data-flow
  shape for review.
- MOTIVATED BY research 14: the University of Washington study (2026-07) established
  cross-origin data movement by an injected agent as the named attack class in agentic
  browsing; no shipping product governs it. Owner ruling 2026-07-07: this is the focus.

## Context

When a multi-step workflow reads data on one host and writes it on another, today's audit
stream shows both calls but not the connection between them. The connection exists in the
engine: the `script` interpreter resolves `$prev`/`$N` references against prior steps'
structured results before dispatching each step, so at dispatch time it knows precisely which
earlier steps fed the current step's arguments. That knowledge is currently discarded.

Recording it makes the audit stream provenance-complete for in-band flows: an auditor (or the
Console, or a future policy gate) can reconstruct "step 4 on host B was built from step 1's
result on host A" by joining records on `batch_id` and `step`. That is the observable core of
the cross-origin exfiltration pattern the UW study demonstrated.

What the engine cannot see, it must not claim to see. If the model reads a page in one call
and retypes the content into another host in a later call, the data flowed through the model's
context, out of band; no local mechanism can attest that flow. Pretending otherwise would be
data-loss-prevention theater.

## Decision

### Decision 1: the honesty fence

Ghostlight attests IN-BAND flows only: data that moved through the engine's own reference
substitution. Out-of-band flows (through the model's context window) are declared permanently
out of scope for provenance, and every document describing this feature must say so. The
mitigation for out-of-band movement remains what it already is: capability floors on writes,
host polarity, grants, and sacred domains. Marketing or docs language implying content
inspection, DLP, or model-context tracking is an ADR violation.

### Decision 2: the `sources` audit key (phase 1, this ADR's implementable core)

`AuditRecord` gains one additive key, `sources: Option<Vec<u32>>`, appended after `dry_run`
(field order is part of the format; additive keys append at the end, matching the
orchestration-keys precedent):

- On an orchestrated `script` step record: the sorted, deduplicated, 1-indexed step numbers
  whose structured results fed this step's resolved arguments (`$prev` normalizes to the
  actual step index it referenced). A step whose arguments contain no references carries
  `sources: null`, not an empty array.
- `null` everywhere else: parent records, `form_fill` internals (fill values come from the
  model's arguments, an out-of-band source by Decision 1), non-orchestrated calls, and
  dry-run records (a dry run resolves nothing; no flow occurred).
- Serialization matches the existing key family: always present, `null` when absent.

### Decision 3: the resolver reports what it resolved

`resolve_refs` (ADR-0035 Decision 2's resolver) returns the resolved arguments AND the set of
source step indexes it substituted from. The interpreter threads that set through the existing
orchestration stamp into the audit record. No second resolution pass, no argument re-parsing:
the provenance is a byproduct of the substitution that already happens, which is why it is
truthful by construction.

### Decision 4: host-level flow derivation is the consumer's join (phase 1)

Phase 1 deliberately does NOT compute host-to-host flow edges inside the engine. Each step
record already carries `domain` (decision-time tab host); `sources` names the feeding steps;
the join (`batch_id` + `step` -> `domain`) is trivial for any audit consumer. Keeping the
record per-call-truthful avoids new cross-call state in the dispatch path and keeps the key
cheap enough to always emit. The Console and the activity ledger are the natural first
consumers (a "flows" rendering is future work, not part of this ADR).

### Decision 5: phase 2 direction -- flow enforcement (pinned, not designed)

The destination is manifest-expressible flow rules (deny or require-grant on source-host ->
destination-host edges), evaluated pre-dispatch in the interpreter, where the resolved sources
are known before the step runs. This requires the step outcome to carry its decision-time
domain back to the interpreter, a grant vocabulary for flow edges, and denial semantics; none
of that is designed here. Per ADR-0018's pattern, enforcement follows observation: phase 2
gets its own ADR once real `sources` data exists to learn from. Nothing in phase 1 may
foreclose it (the key's shape -- step indexes, not pre-joined hosts -- is what enforcement
will consume, and it does not).

## Consequences

### If taken

- The audit stream becomes provenance-complete for in-band flows, which no shipping
  real-session automation product offers; the UW attack class becomes visible in Ghostlight's
  audit trail today and gateable tomorrow.
- Retrospective saved scripts (ADR-0039 D6) will inherit data-flow shapes for free.
- Cost: a resolver signature change (one production call site), the orchestration stamp grows
  one field, one additive audit key, doc updates. Bounded and reversible.

### Risks, named now

- **Misreading as DLP.** Decision 1's language mandate is the mitigation; COMPARISON.md and
  the mapping doc must carry the fence explicitly.
- **Key bloat.** The audit record is growing a key per batch lately; the family rule
  (additive, append-only, null-when-absent) keeps old consumers safe, but the SPEC.md key list
  must stay the single documented registry.

## Provenance

Owner ruling 2026-07-07 (research 14, delta 3): "excellent, let's focus on this." Phase 1
scope (audit-only, sources-as-step-indexes) and the honesty fence were set in the same
post-evaluation pass (ADR-0041 Decision 3). Decided; do not re-litigate.
