# 0036. The `form_fill` tool: semantic form interaction by label

- Status: Accepted
- Date: 2026-07-06 (amended same day: pre-implementation correction pass -- matcher hardened
  for a Write-class tool, the authorization contradiction resolved, a dedicated form-structure
  read replaces the read_page premise, audit shape pinned, multi-form scoping pinned.
  Re-amended same day, post-C8: `idempotency_key` removed -- ADR-0035 D9 not taken, retry
  safety re-homed in ADR-0040, Proposed)

## Relationship to other decisions

- SIBLING OF ADR-0035 (`script`): `form_fill` shares the orchestration substrate that ADR
  pins (the `CallOutcome` split, the async `Handler::Local`, the orchestrator audit keys, the
  `idempotency_key` semantics) but does NOT execute via `script` -- it composes the primitives
  directly. It is the first domain-specific semantic helper and establishes the pattern for
  future ones (`read_table`, `follow_pagination`).
- BUILDS ON ADR-0034 (the capability registry): a browser-capability tool, declared in the
  browser directory and advertised in the capability manifest.
- BUILDS ON ADR-0024 (the generic pipeline): `form_fill` enters the same pipeline as every
  other tool. Its dispatch is `Handler::Local` with non-empty requires, in the post-grant
  position ADR-0035 Decision 6 pins.
- DEPENDS ON ADR-0038 (structured results): the result object is delivered as
  `structuredContent` alongside its text rendering.
- COMPANION TO ADR-0037 (page-state awareness): when `submit: true` clicks the submit control,
  the consequence digest on that click (URL change, alert text, dialog appearance) rides into
  `form_fill`'s result -- the model learns what submission caused without a verify round-trip.

## Context

Filling a form is one of the most common browser-automation tasks, and it is painfully
round-trip-heavy today:

1. `read_page` (interactive) -- learn the form's fields and their `ref_N` identifiers.
2. `form_input(ref_1, "user@example.com")` -- fill email.
3. `form_input(ref_2, "password123")` -- fill password.
4. `form_input(ref_3, true)` -- check "remember me."
5. `computer(left_click, ref_4)` -- click submit.

Five inference passes for one form. Even with `script` (ADR-0035), it is 2 passes: one
`read_page` so the model can see the refs, then one script. The model's mental model is
semantic ("set Email to X, submit"); the tool surface forces DOM-level translation. `form_fill`
closes that gap: the model expresses intent; the service resolves the DOM.

The amendment pass corrected two things against the live tree. First, the original claimed
matching runs "via the existing `read_page` mechanism," but the content script collapses
aria-label, placeholder, title, and label text into ONE accessible-name string
(extension/content.js, `accessibleName`), so the declared matching priority ladder cannot be
evaluated from `read_page` output, and the `name`/`id` HTML attributes are not in that output
at all. `form_fill` gets its own form-structure read (Decision 5). Second, the original
declared both "one Write decision covering all fields" and "each internal call enters the same
pipeline" -- which cannot both be true, since the internal `read_page` requires Read and the
submit click requires Action; a Write-only manifest would have denied the tool's own mechanism
mid-fill. Decision 4 resolves this.

## Decision

### Decision 1: `form_fill` matches field labels to values -- with Write-class rigor

The model provides a map of `{ "label or placeholder or name": value }`. The service reads the
form structure (Decision 5), matches keys to controls, fills, and optionally submits.

Matching, pinned precisely because the failure mode of a fuzzy matcher on a Write tool is
writing into the WRONG field:

- **Sources, in priority order:** associated `<label>` text (for/wrapping), `placeholder`,
  `name` / `id` attribute, `aria-label`.
- **Scoring tiers:** (1) exact match after normalization (casefold, trim, collapse internal
  whitespace); (2) prefix match; (3) substring match (either direction). A higher tier always
  beats a lower tier regardless of source priority; source priority breaks ties within a tier.
- **Resolution order:** keys are resolved most-specific-first (longest normalized key first),
  and each control is consumable AT MOST once. This is what keeps `"Password"` and
  `"Confirm Password"` from landing on the same control: the longer key claims its exact match
  first, then the shorter key resolves among the remaining controls.
- **Ambiguity is surfaced, never guessed:** if a key's best tier is substring-only and two or
  more distinct controls tie, the key goes to `unmatched` with the tied candidates listed
  (label, ref, type each). Exact-tier and prefix-tier ties resolve to the control closest to
  the top of the form (document order) -- those ties are rare and overwhelmingly duplicates of
  the same visual field.

### Decision 2: the input shape

```json
{
  "tool": "form_fill",
  "args": {
    "tabId": 0,
    "fields": {
      "Email": "user@example.com",
      "Password": "hunter2",
      "Remember me": true
    },
    "submit": true
  }
}
```

- `tabId` -- the tab to operate on (required).
- `fields` -- the semantic map. Keys are human-readable field identifiers; values are string,
  boolean, or number (the same types `form_input` accepts). `minProperties: 1`.
- `submit` -- `true` (click submit after filling) or `false`/omitted (fill only).
- Retry safety: v1 ships WITHOUT an `idempotency_key` (ADR-0035 Decision 9 as re-amended --
  not taken; the pipeline-level rebuild is ADR-0040, Proposed). A `form_fill` fires once; a
  re-fire is an explicit choice.

### Decision 3: the result shape

```json
{
  "filled": [
    { "label": "Email", "ref": "ref_1", "value": "user@example.com", "type": "email" },
    { "label": "Password", "ref": "ref_2", "value": "********", "type": "password" },
    { "label": "Remember me", "ref": "ref_3", "value": true, "type": "checkbox" }
  ],
  "unmatched": [
    { "key": "First name", "candidates": [] }
  ],
  "skipped": [
    { "label": "Avatar", "ref": "ref_9", "reason": "file input (out of scope)" }
  ],
  "submitted": true,
  "submit_ref": "ref_5",
  "observation": "url changed to /dashboard; focus moved to \"Search\"",
  "duration_ms": 1200
}
```

- **`filled`** -- each matched field: matched label, resolved ref, value set, discovered type.
  Password values are masked (`********`) -- the model set them; they are not echoed back into
  context. (Values never reach audit either: the pipeline reads no tool-call argument for audit
  except the `computer` sub-action; that invariant is untouched.)
- **`unmatched`** -- keys that could not be matched, each with its `candidates` array (empty
  when nothing came close; populated on substring-tier ambiguity per Decision 1). The model
  adjusts the label and retries, or falls back to `form_input` with an explicit ref.
- **`skipped`** -- controls deliberately not filled: file inputs, disabled/readonly controls,
  with reasons.
- **`submitted`** / **`submit_ref`** -- whether submit was found and clicked, and its ref.
- **`observation`** -- the consequence digest from the submit click (ADR-0037), present only
  when `submit: true` and a digest was captured.
- Delivered as `structuredContent` with a text rendering (ADR-0038).

### Decision 4: authorization -- one semantic decision at the parent

`form_fill`'s declared requirement is `Read + Write`, and `Write + Action + Read` when
`submit: true` (the per-argument requirement mechanism that already serves `computer`'s
`action` key extends to a boolean flag rendered as `"true"`/`"false"` for the lookup). The
governance decision happens ONCE, at the parent call, against that full requirement set --
before anything dispatches.

This replaces the original's contradictory pair. The pinned principle, shared with ADR-0035
Decision 5: **a composition of arbitrary model-authored calls gets per-step decisions
(`script`); the internal mechanism of a single semantic intent gets one decision at the parent
(`form_fill`).** A manifest that wants to stop form filling on a domain denies Write there and
the whole call is denied up front -- never mid-fill with three fields committed.

### Decision 5: `Handler::Local`, a dedicated form-structure read, pre-authorized internals

`form_fill` is an async `Handler::Local` (ADR-0035 Decision 6 shape) dispatched in the
post-grant position. Internally it:

1. Sends a dedicated `formStructure` content-script read (a new extension message, not
   `read_page`): per control -- ref, control type, label text, `aria-label`, `placeholder`,
   `name` attr, `id` attr, form membership index, disabled/readonly flags; per form -- its
   submit candidates (`button[type=submit]`, `input[type=submit]`, submit-like labeled
   buttons). No field VALUES are read: the matcher needs identity, not content, so secrets
   never enter service memory. The result never reaches the client.
2. Matches per Decision 1.
3. Fills each matched control via the existing `setFormValue` mechanism, one frame per fill,
   respecting the per-frame `TOOL_TIMEOUT`.
4. If `submit: true`, clicks the submit control via the existing input path.

These internal executions are **pre-authorized**: they do not re-enter governance `authorize`
(the parent's decision covered them), but they DO each check the take-the-wheel hold before
dispatching (a hold mid-fill aborts: `filled` reports what committed, remaining fields report
skipped-with-held, `submitted: false`), and they are each audited (Decision 7).

The sacred-domains check runs once, at the parent, against the call's single `tabId`:
`form_fill` initiates no navigation, and every internal frame targets that same tab. A
submit-caused navigation is the page's own behavior -- exactly as it is today when a model
clicks submit via `computer`, which has no landing re-check either. Parity, pinned.

### Decision 6: field-type-aware value handling

The service inspects each matched control's discovered type:

- `checkbox`: boolean -- check/uncheck.
- `radio`: string -- select the group member whose label/value matches (Decision 1 tiers).
- `select` / `select-one`: string -- match option text or value.
- `text` / `email` / `password` / `number` / `tel` / `url` / `search` / `date` / etc.: set
  as-is (string or number).
- `textarea`: string -- set text content.
- `file`: never filled; reported in `skipped`.

The model passes `true` for "Remember me" without knowing whether it is a checkbox or a
toggle; the service does the right thing.

### Decision 7: audit shape

Symmetric with ADR-0035 Decision 7:

- **The parent `form_fill` record** carries the governance decision (the one Write/Action
  authorization, with grant attribution) and a fresh `batch_id`.
- **Each internal execution** (the form-structure read, each fill, the submit click) writes a
  record with its own mechanism name, `orchestrator: "form_fill"`, the parent's `batch_id`,
  and `step`. Decision fields on internal records read `allow` with the parent's grant
  attribution -- honest, because the parent's decision is WHY they were allowed.

The semantic intent ("filled Email + Password on example.com and submitted") lives on the
parent record; the mechanism trail lives on the correlated internals. Both survive replay.

### Decision 8: multi-form scoping

Pages routinely carry several forms (login form + header search form). Pinned:

- Matching considers all forms, then commits to the SINGLE form containing the majority of
  matched keys (exact-tier matches weighted first). Keys whose only matches live in other
  forms go to `unmatched` with a note naming the other form's candidates.
- `submit: true` clicks only a submit control belonging to THAT form. If the filled fields do
  not all share one form, nothing is submitted: `submitted: false` with the reason -- filling
  across forms is almost certainly a matching accident, and submitting one of them would
  compound it.

## Consequences

### Fixed

- Form filling goes from 5+ inference passes to 1, with zero refs in the model's context.
- The matcher is safe for a Write-class tool: specificity-ordered, single-consumption,
  ambiguity surfaced with candidates instead of guessed.
- The authorization story is coherent: one semantic decision, up front, covering the whole
  interaction -- no mid-fill denials of the tool's own mechanism.
- Passwords are masked in the result and absent from audit and from the form-structure read.
- The pattern for semantic helpers is established: intent in, mechanism inside, one decision,
  correlated audit.

### Cost

- One new tool declaration in the browser capability's directory (requires Read+Write, the
  submit-conditional Action, `Handler::Local` post-grant dispatch).
- The `formStructure` content-script read (a new extension message + its renderer) -- an
  extension change the original draft did not price.
- The matcher (~150 lines: normalization, tiers, specificity ordering, single-consumption,
  candidate reporting).
- The fill orchestration over `CallOutcome` (per-fill frames, hold checks, abort reporting).
- The submit finder + single-form scoping.
- The orchestrator audit keys on internal records (shared with ADR-0035's mechanism).

### Limitations (accepted)

- **Label matching is heuristic.** Dynamic labels, iframes, exotic shadow-DOM composition may
  not match cleanly. `unmatched` + `candidates` surface it; `form_input` with an explicit ref
  is the fallback. `form_fill` is the fast path, not the only path.
- **Multi-step wizard forms** are one `form_fill` per page (or a `script` of `form_fill`s).
  Correct: the model needs to see each page to fill it.
- **Captcha / human-verification fields** are never filled. The other fields are filled and
  the captcha is reported; the model or the user handles it. Automating captcha defeats its
  purpose.

## Open questions (deferred)

- **Fuzzy matching threshold:** substring is the v1 floor; Levenshtein-tier matching only if
  real-world labels prove ambiguous in ways the candidate reporting cannot resolve.
- **Field selectors** (`{ "Email": { "value": "x", "selector": "css:#my-field" } }`): deferred;
  `unmatched` + `form_input` fallback covers it.
- **File-upload fields:** out of scope permanently for this tool (reported in `skipped`).
- **The semantic-helper roadmap:** `read_table` is the committed NEXT helper (DOM tables to
  structured JSON server-side -- page-to-structured-data is the most common agent task there
  is, and today it costs a giant `get_page_text` plus error-prone in-context parsing); then
  `follow_pagination`. Each gets its own ADR when scheduled, on this ADR's pattern.
