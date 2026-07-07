# L3: the org rollout guide (enterprise proof pack)

Goal: one guide an org operator can follow end to end -- push policy to a fleet, roll out
observe -> shadow -> enforce, and hand their compliance team a one-pager -- assembled from
what already exists, not invented. Authority: ADR-0041 Decision 6 (proposal P8); outline and
fences PINS SS8.

## STOP preconditions

- `docs/guides/compliance-team.md`, `docs/guides/siem-integration.md`, and
  `docs/guides/solo-developer.md` exist.
- `open-spec/rawx-owasp-agentic-mapping.md` exists (linked from section 5).
- ADR-0018, ADR-0019, ADR-0020 exist under docs/adr/.

## Required behavior

Create `docs/guides/org-rollout.md` following PINS SS8's pinned outline (five H2 sections, in
order), sourcing every factual claim from the named existing documents. The two cross-links
(README documentation-table row with the pinned text; one pointer sentence atop
compliance-team.md) are part of this task.

Style: match the existing guides -- plain, human, ASCII, no em-dashes, no marketing voice.
Every mechanism named must link to its ADR or guide; every path or command must already
appear in a cited source (re-read them; do not trust this file's memory of their contents).

## The binding claims fence (PINS SS8)

No pricing, no license terms, no availability promises, no security claims beyond the cited
ADRs and the mapping doc. The ADR-0042 Decision 1 honesty fence applies verbatim in spirit to
section 5: in-band flows only; no content-inspection or DLP language. A sentence that cannot
be sourced is a sentence to delete.

## Verification (literal)

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

(Unchanged code; the gates are regression insurance.) Manually verify: every relative link in
the new file resolves; the README row renders as a table row (pipe count matches the table).
Then commit exactly:

```
docs(guides): org rollout guide -- policy push, staged enforcement, compliance one-pager
```

## Out of scope (fences)

- No edits to the other guides beyond compliance-team.md's one pointer sentence.
- No new claims about the EU AI Act beyond its August 2026 high-risk timing (link out for the
  rest); no legal advice language.
- No code, config keys, or example-manifest changes.
