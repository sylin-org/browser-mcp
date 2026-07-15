# ADR-0078: Closed-loop browser core

Date: 2026-07-14
Status: Accepted
Builds on: ADR-0034 (additive capability registry), ADR-0036 (`form_fill` semantic
interaction), ADR-0037 (page-state awareness), ADR-0038 (structured results), ADR-0042
(origin-flow provenance), and ADR-0077 (local-only browser control). Amends ADR-0037's
consequence digest, ADR-0038's result vocabulary, and ADR-0066 only to permit an explicit
request to control a session-owned tab. Preserves ADR-0005's policy-free extension and
ADR-0007's byte-stable 13 trained schemas.

## Context

Ghostlight has broad low-level browser coverage, but a model still spends too much context and too
many turns closing an ordinary interaction loop:

1. read a page;
2. infer a target;
3. issue a low-level action;
4. read again to learn whether the page changed;
5. diagnose ambiguity, a dialog, or another blocker;
6. recover and continue.

Each primitive is useful, but the seams push browser mechanics into the model. `find` returns a
small locator record but not enough target state to choose confidently. Mutating calls report a
fixed consequence digest but not a reusable interaction receipt. Page text and browser-authored
metadata do not carry one uniform trust boundary. JavaScript dialogs and the owned-tab lifecycle
lack explicit control surfaces. This costs tokens and roundtrips, makes failures harder for a user
to understand, and leaves governance with less outcome evidence than the engine already observes.

The answer is not a large autonomous task runner. Ghostlight acts in the local user's visible,
authenticated browser and must keep each meaningful action inspectable and governable. The useful
unit is a closed interaction loop: observe once, act by meaning, receive bounded proof, and
continue. Exact refs, coordinates, and low-level tools remain first-class escape hatches.

## Decision

### D1. The shared primitive is an interaction receipt

Every browser interaction that can change page or browser state returns a bounded receipt with one
stable conceptual vocabulary:

```text
target       what Ghostlight resolved or attempted
action       what mechanism was dispatched
observed_after
             bounded facts observed after dispatch
blockers     ambiguity, dialog, coverage, stale target, or timeout facts
page         tab, URL/origin, title, and render serial
provenance   where page-sourced facts came from and that they are untrusted
more         whether detail was omitted and the next narrow read to request
```

The receipt says only what Ghostlight observed after an action. It never claims that the action
caused the change, that a transaction committed, or that a remote server accepted an operation.
The existing consequence digest becomes the compact text rendering of this receipt. Structured
clients receive the same facts through `structuredContent` and the tool `outputSchema`.

Low-level mutating tools keep their current first observation at roughly 300 ms. They do not gain
an unconditional multi-second wait. The new semantic action may adaptively settle because it is a
deliberate higher-level roundtrip-saving primitive.

### D2. `find` and targeted `read_page` become actionable observations

Do not add a standalone `element_info` tool. Enrich the observations the model already requests.

An actionable element summary may contain:

- ref, role, accessible name, visibility, enabled state, selected or checked state;
- a bounded value using the existing secret-marker and redaction rules;
- href where applicable, viewport box, render serial, and optional frame origin;
- `mechanical_actions`, meaning what the browser mechanism can attempt, never what policy allows.

`find` ranks meaningful candidates using deterministic matching: exact accessible name, prefix,
token containment, then substring. It may list ranked ties because listing is read-only.
Targeted `read_page(ref_id)` returns the same compact summary with its bounded text view. Full-page
reads do not emit a large structured DOM mirror.

The 13 trained schemas do not change. Any optional field added to an existing additive tool follows
ADR-0034's compatibility rules. Existing text fields remain useful to text-only clients.

### D3. Add `act_on` for one governed semantic interaction

Add an advertised `act_on` tool through the capability registry. It performs one semantic
interaction and closes the immediate observation loop in one MCP roundtrip.

V1 target forms are mutually exclusive:

```json
{"ref":"r12"}
{"query":"Save changes"}
{"name":"Save", "role":"button"}
```

V1 actions are `left_click`, `right_click`, `double_click`, `hover`, `scroll_to`, and
`set_value`. `set_value` alone requires a `value`. An optional `expect` uses the `wait_for`
vocabulary for one postcondition: text or selector, state, and bounded timeout.

The whole semantic intent receives one parent governance decision before any browser mutation.
Capability requirements are derived from the complete request:

- click actions require Action;
- hover and scroll-to require Read;
- set-value requires Write;
- semantic resolution and `expect` add Read.

Internal browser calls remain correlated implementation steps, as in `form_fill`; they do not turn
one model intent into unrelated policy prompts. Page text, accessible names, candidate scores, and
resolved values never become policy inputs.

Semantic mutation is fail-closed on ambiguity. If the highest rank is not unique, `act_on` makes no
change and returns a bounded candidate capsule. Ref targets keep their current stale-ref behavior.
V1 operates in the top document and same-origin shadow trees only. A framed target returns a
corrective limitation rather than silently acting under the top origin's authority.

Before dispatch, the extension gives the resolved target a short deterministic glow and mechanical
caption. This is user visibility, not a confirmation prompt and not governance authority.

After dispatch, `act_on` takes the ordinary 300 ms observation. If it sees meaningful activity, it
may reuse the existing settle detector for up to five seconds. If `expect` is present, its bounded
condition is authoritative for completion. A quiet page is reported as no observed change, not as
settled or successful beyond the dispatched mechanism.

### D4. Failures carry progressive recovery capsules

Do not add a routine `page_diagnostics` call to every journey. On ambiguity, stale ref, a covered
target, an open dialog, or an unmet expectation, the failed interaction returns a bounded recovery
capsule made from facts Ghostlight already has. It names the blocker, the top candidate facts or
page state, and one narrow next step.

Console and network logs are never attached automatically. CDP domains are not enabled merely to
decorate a failure. A future diagnostics tool must justify its privacy, token, and lifecycle cost
separately.

### D5. Page-sourced output has uniform provenance and text boundaries

All new or enriched page-sourced structured results include:

- the top-page origin;
- optional frame origin;
- render serial where available;
- an explicit page-sourced, untrusted marker;
- a per-session nonce chosen outside page control.

For text-only results, the service places bounded page-sourced text between deterministic boundary
markers that include the nonce and origin. The boundary is added after the browser result reaches
the service so the page cannot predict it in advance. It is a defense-in-depth signal to the model,
not a sanitizer, content filter, DLP system, or policy decision.

This provenance ships in the permissively licensed engine. It never phones home, persists page
content, or changes the local-only boundary.

#### D5 amendment: machine consumers validate before unwrapping

The text boundary is part of the model-facing MCP result. An in-repository machine consumer that
must parse the enclosed value, such as the scripted demo's geometry helper, may remove only the
outer control markers after validating that `structuredContent.provenance` marks the result as
page-sourced and untrusted and that its origin and session nonce exactly match both text markers.
It must reject malformed, missing, or mismatched provenance instead of deleting marker-shaped page
text. Raw results remain accepted where compatibility with a pre-ADR-0078 service is intentional.
This consumer rule does not change the trained tool schema or the service's model-facing output.

### D6. Record content-free target assurance in result and audit

Each relevant interaction reports a target-assurance class: `semantic`, `ref`, `coordinate`, or
`none`. The class describes how the mechanism selected its target. It does not say that policy
approved the target, that the target was correct, or that the outcome succeeded.

The audit record may store this enum and bounded outcome categories. It must not store the semantic
query, accessible name, field value, page text, target box, screenshot, or a content-derived hash.
The free engine produces this neutral evidence. Commercial organization governance may later
report on or constrain assurance classes only through a separate accepted decision.

### D7. Add explicit dialog and owned-tab controls

Add a `dialog` tool with `status`, `accept`, `dismiss`, and `respond` actions. Status requires Read;
the other actions require Action. It is tab-scoped, reports whether a JavaScript dialog is blocking
the tab, and includes that blocker in relevant interaction receipts. Browser mechanism remains in
the extension; policy and classification remain in the service.

#### D7 amendment: the blocker guard precedes page-dependent preparation

A relevant interaction checks the extension's current dialog state before resolving a ref, reading
page geometry, moving the cursor, or performing any other page-dependent preparation. The guard is
not limited to the final input dispatch. This clarification was added after visible-Chrome
verification showed that coordinate resolution could wait on a modal page and never reach the
existing dispatch guard.

Add a `tab_control` tool with `focus`, `reload`, and `close`. Reload and close require Action. Focus
changes browser presentation but not page content and requires no RAWX capability. Every action
enforces the existing session ownership boundary. Close is an explicit request for one
session-owned tab; it never closes a user's own tab, automatically deletes a group, or performs
cleanup based on a guess. This narrow ability amends ADR-0066 without weakening its ownership model.

### D8. Transparent cross-frame refs require a separate ADR

Cross-origin frames are not just a locator implementation. A semantic read or action may touch a
different governed origin than the top tab, which breaks the present one-call, one-resource
assumption. Transparent frame refs therefore require a separate decision covering multi-resource
preflight, origin-aware authorization, filtered observations, ref routing, and audit attribution.

V1 reserves optional `frame_origin` fields so the receipt vocabulary does not need another redesign,
but it does not expose or act through cross-origin frame refs.

### D9. Delight is measured at three layers

Model delight means fewer turns and less repeated page text without concealing uncertainty. User
delight means visible target choice, quiet bounded feedback, and an exact explanation when the loop
cannot continue. Governance delight means one decision per semantic intent plus content-free target
and outcome evidence that can be reviewed without collecting page payloads.

The implementation batch must pin budgets and add journey tests for all three. Raw tool count is
not a success measure. The target is fewer calls and smaller useful payloads for the same ordinary
browser task.

## Non-goals

- No headless, isolated-profile, cloud, shared, or remote browser execution.
- No autonomous multi-step task planner, rollback, undo, transaction, or causal-success claim.
- No content inspection, prompt-injection detector, DLP, or page-text policy rule.
- No automatic managed confirmation. ADR-0075 remains a separate proposed policy design.
- No stable ref healing across navigation.
- No generic always-on console or network diagnostics.
- No cross-origin frame implementation in this batch.
- No policy logic in the extension and no modification of the trained 13 schemas.

## Acceptance criteria

1. The 13 trained schemas remain byte-stable; new tools enter only through the additive registry.
2. `find` and targeted `read_page` share one bounded actionable-element vocabulary and matcher.
3. `act_on` proves unique-match mutation, ambiguity refusal, dynamic RAWX requirements, optional
   expectation, adaptive settle, user-visible targeting, and one parent audit decision.
4. Existing mutating tools and `act_on` emit bounded receipts without adding unconditional latency
   to low-level calls.
5. Page-sourced structured and text results carry consistent, service-authored provenance; tests
   prove that page content cannot choose the session boundary nonce, and machine consumers validate
   matching structured provenance before unwrapping an enclosed value.
6. Audit tests prove target assurance is present while target and page payloads are absent.
7. Dialog and tab controls prove tab ownership, sacred-tab defense, classification, cleanup, and
   corrective failure behavior.
8. Real-browser verification completes a semantic find-act-observe journey, an ambiguity refusal,
   a dialog recovery, and an explicit owned-tab close in the visible local browser.
9. A pinned comparison records calls, input tokens, output bytes, and recovery turns for equivalent
   low-level and closed-loop journeys. The closed-loop path must reduce calls without dropping the
   outcome facts needed for the next decision.

## Consequences

- Ordinary browser work can collapse several mechanical MCP turns into one governed semantic
  interaction while keeping exact primitives available.
- Results get more structured but remain deliberately bounded; clients that use only text still get
  a compact trustworthy boundary and recovery advice.
- Governance gains useful target-method and outcome evidence without receiving page payloads.
- The extension gains matching, observation, visual-target, dialog, and tab mechanisms but no policy.
- The service gains additive tools, result post-processing, and audit vocabulary.
- Cross-origin frames stay visibly incomplete until their governance model is designed.

## Rejected alternatives

- **Clone agent-browser's command breadth.** Breadth alone does not close the observation loop and
  would import headless and isolated-browser assumptions that do not fit Ghostlight.
- **Add `element_info` as another call.** The information belongs in `find` and targeted reads the
  model already performs.
- **Make semantic targeting silently choose the first match.** A wrong mutation costs more than an
  ambiguity recovery and gives governance false confidence.
- **Put page meaning into policy.** Ghostlight governs capability and origin, not content. Page text
  is untrusted and must not become authorization input.
- **Return full DOM, accessibility, console, and network snapshots after each action.** This would
  spend tokens, expand sensitive-data handling, and obscure the few facts needed for the next move.
- **Auto-close tabs and groups.** Browser ownership does not imply permission to discard state based
  on a heuristic. Close remains explicit and one owned tab at a time.
- **Treat frames as a locator-only phase.** Cross-origin actions need resource-aware governance first.
