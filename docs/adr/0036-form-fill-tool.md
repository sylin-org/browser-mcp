# 0036. The `form_fill` tool: semantic form interaction by label

- Status: Accepted
- Date: 2026-07-06

## Relationship to other decisions

- BUILDS ON ADR-0035 (`script`): `form_fill` is the first domain-specific semantic helper built
  on top of the primitives. It is internally a composition (read the form → match labels → fill
  fields → optionally submit), but the model sees one tool call, not a `script` of `read_page` +
  N × `form_input` + `computer`. It establishes the pattern for future semantic helpers
  (`read_table`, `follow_pagination`, etc.).
- BUILDS ON ADR-0034 (the capability registry): `form_fill` is a browser-capability tool,
  declared in the browser directory and advertised in the capability manifest.
- BUILDS ON ADR-0024 (the generic pipeline): `form_fill` enters the same pipeline as every other
  tool; its RAWX capability is `Write` (it commits values to form fields). Its dispatch is
  `Handler::Local` (the service orchestrates the form interaction internally).

## Context

Filling a form is one of the most common browser-automation tasks, and it's painfully
round-trip-heavy today:

1. `read_page` (interactive) — learn the form's fields and their `ref_N` identifiers.
2. `form_input(ref_1, "user@example.com")` — fill email.
3. `form_input(ref_2, "password123")` — fill password.
4. `form_input(ref_3, true)` — check "remember me."
5. `computer(left_click, ref_4)` — click submit.

That's **5 inference passes** for one form. Even with `script` (ADR-0035), it's 2 passes:
one to `read_page` (so the model can see the refs and build the script), then one `script` call.
The `read_page` step is pure overhead — the model doesn't need to *see* the form structure to
*fill* it; it just needs to know "put this value in the field labeled 'Email'."

The model's mental model of form filling is semantic: "set Email to X, Password to Y, check
Remember Me, submit." The current tool surface forces it to translate that into DOM-level
operations (read refs, fill by ref, click by ref). `form_fill` closes that gap: the model
expresses the intent; the service resolves the DOM.

## Decision

### Decision 1: `form_fill` matches field labels to values

The model provides a map of `{ "label or placeholder or name": value }`. The service:

1. Reads the page's form structure internally (via the existing `read_page` mechanism — no
   client-visible round-trip).
2. Matches each provided key to a form field by (in priority order):
   - The field's `<label>` text.
   - The field's `placeholder` attribute.
   - The field's `name` or `id` attribute.
   - The field's `aria-label`.
   Matching is case-insensitive, whitespace-trimmed, and substring-tolerant (e.g., "email"
   matches "Email Address"). Multiple matches resolve to the first (closest to the top of the
   form); an ambiguous match (two equally-good candidates for different fields) is flagged in
   the result.
3. Fills each matched field using the existing `form_input` mechanism (same CDP path, same
   value-type handling — boolean for checkboxes, string for text, number for number inputs,
   matching option text/value for `<select>`).
4. Optionally clicks the submit button (the first `<button type="submit">` or `<input
   type="submit">` in the form, or the element with `role="submit"` / a submit-like label
   such as "Sign in", "Log in", "Submit", "Save").

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

- `tabId` — the tab to operate on (required, same as every tab-scoped tool).
- `fields` — the semantic map. Keys are human-readable field identifiers (labels, placeholders,
  names); values are the data to set (string, boolean, or number — the same types `form_input`
  accepts). `minProperties: 1`.
- `submit` — `true` (click submit after filling) or `false`/omitted (fill only; the model
  verifies before submitting).

### Decision 3: the result shape

```json
{
  "filled": [
    { "label": "Email", "ref": "ref_1", "value": "user@example.com", "type": "email" },
    { "label": "Password", "ref": "ref_2", "value": "********", "type": "password" },
    { "label": "Remember me", "ref": "ref_3", "value": true, "type": "checkbox" }
  ],
  "unmatched": ["First name"],
  "submitted": true,
  "submit_ref": "ref_5",
  "duration_ms": 1200
}
```

- **`filled`** — each matched field: the label the service matched on, the ref it resolved to
  (so the model can reference it in a follow-up if needed), the value set, and the field type.
  Password values are masked in the result (`********`) — the model set them; it doesn't need
  them echoed back.
- **`unmatched`** — keys from the `fields` map that could not be matched to any form field. The
  model can adjust its label and retry, or fall back to `form_input` with a specific ref.
- **`submitted`** — whether the submit button was found and clicked. If `submit: true` but no
  submit button was found, this is `false` and the model can click manually.
- **`submit_ref`** — the ref of the submit button (if found), for follow-up interaction.

### Decision 4: RAWX capability is Write

`form_fill` commits values to form fields — it is a declared `Write`. Under a manifest that
denies Write on a given domain, `form_fill` is denied at the governance chokepoint, exactly as
`form_input` is today. The governance decision is per-call (one `form_fill` = one Write decision
covering all fields in the map), not per-field.

### Decision 5: `Handler::Local`, internally a composition

`form_fill` is `Handler::Local` — the service orchestrates the entire interaction without a
dedicated extension frame. Internally it:

1. Calls `read_page` (internally, via the pipeline — the result stays in service memory, never
   sent to the client).
2. Matches labels to refs.
3. Calls `form_input` for each matched field (via the pipeline — each fill goes to the extension
   as a normal `form_input` frame).
4. Optionally calls `computer(left_click, submit_ref)` (via the pipeline).

Each internal call enters the same pipeline as if the model called it directly. The difference
is the model sees only the final `form_fill` result, not the intermediate `read_page` + N ×
`form_input` outputs.

### Decision 6: field-type-aware value handling

The service inspects each matched field's type and handles the value appropriately:

- `checkbox` / `radio`: boolean → check/uncheck (or select for radio).
- `select` / `select-one`: string → match the option's text or value.
- `text` / `email` / `password` / `number` / `tel` / `url` / `search` / `date` / etc.: the
  value is set as-is (string or number).
- `textarea`: string → set the text content.

This matches `form_input`'s existing type handling but applies it automatically based on the
field's discovered type, so the model doesn't need to know whether "Remember me" is a checkbox
vs a toggle — it just passes `true` and the service does the right thing.

## Consequences

### Fixed

- Form filling goes from 5+ inference passes to **1**, with zero refs in the model's context.
  The model says "Email = X, Password = Y, submit" and gets back a compact result.
- The label-matching eliminates the `read_page` → ref-mapping step entirely for forms. The
  model doesn't need to see the form structure to fill it.
- Password values are masked in the result (defense-in-depth: the model set them; they're not
  echoed back into the context).
- The pattern establishes the "semantic helper" approach: domain-specific tools that compose
  the primitives internally and expose a high-level intent interface to the model.

### Cost

- One new tool declaration in the browser capability's directory.
- The label-matching logic (~100 lines: parse `read_page` output, match keys to labels/
  placeholders/names, resolve to refs, handle ambiguity).
- The field-type-aware fill orchestration (~50 lines: call `form_input` per matched field with
  the right value-type handling).
- The submit-button finder (~30 lines: scan for submit-type elements, match by role/label/type).

### Limitations (accepted)

- **Label matching is heuristic.** Complex forms (dynamic labels, iframes, shadow DOM,
  dynamically-added fields) may not match cleanly. The `unmatched` result field surfaces these;
  the model can fall back to `form_input` with an explicit ref. This is an accepted limitation —
  `form_fill` is the fast path, not the only path.
- **Multi-step forms** (wizard-style forms with "Next" buttons between pages) are not handled
  by a single `form_fill` call. Each page is a separate `form_fill` (or a `script` of
  `form_fill`s). This is correct — the model needs to see each page to fill it.
- **Captcha / human-verification fields** are not filled. If a form has a captcha, `form_fill`
  fills the other fields and returns; the model or the user handles the captcha. This is the
  correct behavior — automating captcha defeats its purpose.

## Open questions (deferred)

- **Fuzzy matching threshold:** how aggressively should "email" match "Email Address"?
  Substring matching (one contains the other) is the v1; Levenshtein-distance fuzzy matching
  is a possible future enhancement if real-world labels prove ambiguous.
- **Field selectors** (`{ "Email": { "value": "x", "selector": "css:#my-field" } }`): for
  when label matching fails and the model knows the CSS selector. Deferred; the `unmatched`
  result + `form_input` fallback covers this case today.
- **File-upload fields:** `form_fill` does not handle `<input type="file">` (that's the
  `upload_image` tool's domain). `form_fill` skips them and notes them in the result.
