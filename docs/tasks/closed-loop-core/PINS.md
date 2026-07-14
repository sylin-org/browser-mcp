# PINS: closed-loop browser core

These are resolved implementation choices for ADR-0078. They prevent six tasks from inventing
slightly different meanings for the same target, receipt, provenance, or audit fact.

## P1. Shared actionable element

Use one internal element-summary builder and one serialized vocabulary:

```text
ref, role, name, visible, enabled, checked, selected, value, href,
box {x, y, width, height}, renderSerial, frameOrigin, mechanicalActions
```

- Omit inapplicable optional fields. Do not serialize null-heavy objects.
- Bound `name`, `value`, and `href` to 120, 120, and 240 characters after existing secret handling.
- `mechanicalActions` is derived from element mechanism and state. It never means policy-allowed.
- Box coordinates are CSS viewport coordinates and are descriptive, not a substitute for the
  existing coordinate normalization rules.
- V1 `frameOrigin` is absent for the top document. Do not mint cross-origin frame refs.

Ranking is deterministic: exact normalized accessible name, prefix, token containment, substring.
Role is an exact normalized filter when supplied. Preserve document order inside one score tier.
`find` may return ties. `act_on` must refuse mutation unless the best tier has exactly one result.

## P2. Interaction receipt

The internal structured shape is:

```text
target: optional actionable element summary
targetAssurance: semantic | ref | coordinate | none
action: stable mechanical action name
observedAfter:
  urlChanged, titleChanged, renderAdvanced, changedElements[0..3], alertOrStatus
blockers[0..3]: kind, summary, nextStep
page: tabId, url, origin, title, renderSerial
provenance: pageSourced, untrusted, topOrigin, frameOrigin, sessionNonce
more: boolean
```

- Text receipt maximum: 800 characters on success, 1200 on failure.
- Changed elements: at most 3; names at most 120 characters.
- Alert/status text: at most 200 characters.
- Blocker summaries and next steps: at most 200 characters each.
- `more: true` means facts were omitted and the text rendering names one narrow next call.
- Use `observed after`, never `caused`, `committed`, `completed`, or `verified`, unless an explicit
  `expect` condition was in fact observed.
- Existing low-level actions keep one approximately 300 ms sample. Do not add adaptive waiting to
  them. `act_on` alone may settle for up to 5 seconds after meaningful activity.

Stable blocker kinds for this batch: `ambiguous_target`, `stale_ref`, `covered_target`,
`dialog_open`, `expect_timeout`, `frame_unsupported`, `target_missing`.

## P3. `act_on` surface

Tool name: `act_on`. Additive only. Required top-level properties: `tabId`, `target`, `action`.

Model-facing description:

```text
Resolve one visible element by ref or accessible meaning, perform one action, and return a bounded
observation receipt. Use this when the target should be unique and you want to avoid a separate
find, action, and wait loop. Ambiguous semantic matches are reported without acting.
```

```text
target:
  exactly one of ref:string, query:string, or name:string with optional role:string
action:
  left_click | right_click | double_click | hover | scroll_to | set_value
value:
  string, required only for set_value and forbidden otherwise
expect:
  the existing wait_for condition vocabulary: exactly one of text or selector,
  state visible | present | gone, optional timeout_ms with the existing 30000 ms hard cap
```

Property order is `tabId`, `target`, `action`, `value`, `expect`. Target property order is `ref`,
`query`, `name`, `role`. Expect property order is `selector`, `text`, `state`, `timeout_ms`.
All objects set `additionalProperties:false`. Reuse existing field descriptions and limits where a
matching `computer`, `find`, `form_fill`, or `wait_for` concept exists; do not create a second
meaning for ref, accessible name, selector state, or wait timeout.

The shared shallow schema validator does not prove nested exclusivity. `mcp/act_on.rs` must perform
typed local validation and return corrective examples before browser dispatch.

Requirements: click variants add Action; hover/scroll-to add Read; set-value adds Write; query,
name, and expect add Read. One parent authorization covers resolution, dispatch, and observation.
Use the `form_fill` parent/internal correlation pattern. Do not perform a mutation after an
ambiguous semantic match.

The direct result carries `targetAssurance: semantic` for query/name, `ref` for ref targets. The
existing low-level `computer` path uses `coordinate` when coordinates select the target and `ref`
when ref input selects it.

Use the registry's existing generic `action_key` path, which already supports `computer` and
`form_fill`, for all three new tools. Update stale computer-only comments and tests, not the working
lookup model. `act_on`, `dialog`, and `tab_control` use registry variants as the primary
classification authority. `act_on` then applies one pure argument refinement that adds Read for a
semantic target or expectation. Do not add three unrelated tool-name conditionals to the pipeline.

## P4. Provenance boundary

Create one random, memory-only session nonce of at least 96 bits. Render it in lowercase hex. The
service, not page or extension content, adds the nonce to model-facing boundaries.

Text-only page output uses:

```text
--- GHOSTLIGHT PAGE CONTENT <nonce> origin=<origin> UNTRUSTED ---
<bounded page-authored text>
--- END GHOSTLIGHT PAGE CONTENT <nonce> ---
```

Do not wrap service-authored confirmations, validation errors, policy messages, or audit output.
Structured provenance uses the P2 fields. The nonce is not persisted to audit and rotates with the
MCP session. Tests inject a nonce source; do not make randomness-dependent snapshots.

## P5. Audit minimization

Add only content-free enum/category fields required to answer how a target was selected and what
kind of observation followed. Permitted values include target assurance and bounded outcome kinds
such as `changed`, `unchanged`, `blocked`, `expect_met`, and `expect_timeout`.

Audit must not contain: target query, accessible name, element value, href, page text, dialog text,
box, screenshot, boundary nonce, candidate score, or a content-derived hash. Keep browser/domain
types outside `governance`; the governance record stores neutral serialized vocabulary through its
existing setters/builders.

## P6. Dialog and tab tools

`dialog` actions: `status`, `accept`, `dismiss`, `respond`. `respond` requires `text`; other actions
forbid it. Status requires Read. Accept, dismiss, and respond require Action. Do not include dialog
text in audit.

`dialog` model-facing description:

```text
Inspect or explicitly resolve the JavaScript dialog blocking one owned tab. Use status when the
dialog state is unknown. Never accept, dismiss, or respond without intent from the current task.
```

Property order is `tabId`, `action`, `text`; action enum order is `status`, `accept`, `dismiss`,
`respond`. All objects set `additionalProperties:false`.

`tab_control` actions: `focus`, `reload`, `close`. Reload and close require Action. Focus is RAWX
none because it changes browser presentation, not page content. Every variant requires a
session-owned tab and passes the normal hold, sacred-tab, dispatch, and audit chokepoint. Close one
tab only; never auto-close and never delete a tab group.

`tab_control` model-facing description:

```text
Focus, reload, or close one tab owned by this Ghostlight session. Close is always explicit and
never affects a user-owned tab or automatically deletes the containing tab group.
```

Property order is `tabId`, `action`; action enum order is `focus`, `reload`, `close`. All objects
set `additionalProperties:false`.

## P7. Evaluation oracle

Add one deterministic in-process journey expressed both ways:

- low level: find/read, action, wait/read, recovery if needed;
- closed loop: `act_on` with optional expect.

Record call count, serialized input bytes, serialized output bytes, and recovery turns. Token counts
may use the repository's existing deterministic estimator if one exists; otherwise report bytes and
call count rather than inventing tokenizer fidelity. The closed-loop path must use fewer calls and
must preserve target assurance, observed outcome, blocker, page, and provenance facts needed for
the next decision.
