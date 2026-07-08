# 0043. WebMCP stance: future governed consumer, no implementation yet

- Status: Accepted (a recorded stance; sanctions NO implementation)

## Relationship to other decisions

- BUILDS ON ADR-0034 (capability & transport registry): the registry is the designated
  integration point if and when WebMCP is adopted.
- BUILDS ON ADR-0013 / ADR-0022: site-declared tools would be governed like everything else;
  consuming them safely is precisely a governance problem.
- CLOSES research 13's WebMCP watch item into a position (accepted proposal P9, ADR-0041 D6).

## Context

WebMCP (the W3C draft from Google and Microsoft: web pages declare callable tools to the
browser's agent) moved from a flag to a public origin trial (Chrome 149 through 156, 2026).
The API surface is not stable: it shipped as `navigator.modelContext` and was renamed to
`document.modelContext` in Chrome 150, mid-trial. Today its only consumer is Gemini in
Chrome; the mainstream agents still navigate, read, and click.

If WebMCP lands broadly, it partially inverts Ghostlight's mechanism (sites offer semantic
tools; screenshots and CDP clicks matter less on cooperating sites) while leaving the
governance question fully intact: which site-declared tools may this agent call, as whom, with
what audit? An ungoverned WebMCP consumer trusts arbitrary page-supplied tool declarations,
which is a worse injection surface than reading page text.

## Decision

1. **Stance: future governed consumer.** If adopted, site-declared WebMCP tools enter through
   the capability registry (ADR-0034) as a capability whose directory is dynamic per page,
   with each declared tool classified into RAWX, subject to grants, host polarity, sacred
   domains, and audit exactly like the built-in surface. "The governed way to consume WebMCP"
   is the coherent future offer; an ungoverned passthrough is not.
2. **No implementation while the API is in origin-trial flux.** The rename mid-trial is the
   evidence; building now means building twice.
3. **Re-evaluation triggers** (any one suffices): WebMCP reaches a stable Chrome release
   channel; a second major agent ships as a consumer; a Ghostlight user asks for it; or the
   annual landscape refresh (research-14 cadence) finds the trial extended into a de facto
   standard.
4. **Until then:** watch item only, tracked in the research series. No extension permissions,
   no schema work, no speculative plumbing.

## Consequences

- The question "what about WebMCP?" has a citable answer that is neither dismissal nor
  premature adoption.
- The capability registry's design (dynamic directories, per-capability guidance) is already
  shaped to absorb it, so waiting costs nothing structurally.
- Risk accepted: if WebMCP explodes faster than the triggers fire, Ghostlight arrives late
  with governance as the differentiator, which is the same position that is working for the
  CDP surface.
