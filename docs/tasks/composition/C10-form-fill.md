# C10: the form_fill tool

Goal: semantic form filling -- one Write-class decision, hardened matcher, pre-authorized
internals, correlated audit. Normative: ADR-0036 (all decisions, as re-amended post-C8), PINS
SS13 + SS4 (and SS8's supersession note: there is NO idempotency cache; do not build one).

Protocol reminder: any impossibility or disagreement with these pins is BLOCKED + halt per
BOOTSTRAP -- never a redesign. (The C8 ledger entry is why this line exists.)

## Tree facts (as of authoring; re-read before editing)

- C1, C2 (post-grant Local arm + attribute_grant), C9 (form_structure_internal) committed.
  C8 landed dry-run only; there is NO idempotency cache and form_fill ships WITHOUT an
  idempotency_key (ADR-0035 D9 re-amended; ADR-0040 owns the future rebuild). C5's digest text
  exists unless C5 was SKIPPED (LEDGER).
- pipeline.rs action extraction: `descriptor.action_key.and_then(|key| args.get(key)).and_then(Value::as_str)`
  (:99-102 as of authoring).
- `directory.rs` requires-lookup inline tests around :967-1027.

## STOP preconditions

- STOP if the post-grant Local arm (C2) or `attribute_grant` (C1) is missing.
- STOP if `Gate::Proceed` does not expose the resolved grant id to the dispatch site (inspect
  `governance.authorize` -- if the grant id is only inside the audit, internals'
  `attribute_grant` uses None and you record deviation D-grant instead of stopping).

## Required behavior

1. Pipeline action extraction: booleans map per PINS SS13 (true -> the action_key NAME, false/
   absent -> None). Strings unchanged.
2. `src/browser/form_match.rs` (pure): types + `match_fields` per SS13 (normalization, tiers,
   longest-key-first, single consumption, substring-tie -> unmatched with candidates, form
   scoring 2*exact + 1*other, tie -> lower formIndex).
3. `src/transport/mcp/form_fill.rs`: the Local handler: mint batch_id + `_batch_id` side
   channel; call `browser.call("form_structure_internal",
   {tabId})`, parse; match; fill matched controls in form order via the internal executor
   (held_for check before EACH dispatch -> abort with remaining `skipped` reason "held";
   `browser.call("form_input", {tabId, ref, value})`; audit per internal: begin +
   `orchestrated("form_fill", batch, step)` + `attribute_grant(...)` + complete; the
   formStructure read audits as tool "form_structure", step 1, requires Read). submit: true ->
   click the chosen form's first submit candidate via `browser.call("computer",
   {action:"left_click", tabId, ref})`, audited likewise; the click result's `observation:`
   line (C5), when present, becomes the result's `observation` field; fields spanning forms ->
   `submitted: false` with reason (SS13/ADR-0036 D8). Result object per ADR-0036 D3 (password
   masking "********"; `skipped` for file/disabled/readonly; `unmatched` with candidates;
   `submit_ref`; `duration_ms`), rendered as a compact text summary
   (pinned: first line `Filled {n}/{m} fields.` then one line per filled `{label} -> {type}`,
   then `unmatched: {keys}` if any, then `submitted: {true|false}`) plus identical
   structuredContent.
4. Directory row per SS13 (action_key "submit", two variants, TabScoped, Local -- post-grant
   arm), advertised description + inputSchema EXACTLY per SS13, example call
   `{"tabId":0,"fields":{"Email":"user@example.com","Remember me":true},"submit":true}`,
   output_schema Some. Inserted before explain.

## Tests (by name; assertions verbatim)

- `form_match.rs` inline: SS13's three oracles, named `specificity_and_single_consumption`
  (Confirm Password/Password/Email fixture -> ref_4/ref_3/ref_1), `substring_tie_goes_unmatched`
  (First name/Last name), `exact_on_name_attr_beats_prefix_on_label`. Plus
  `form_scoring_picks_majority_form` (two forms, keys matching 2-exact in form 0 and 1-substring
  in form 1 -> form 0, other key unmatched).
- `directory.rs` inline: `requires("form_fill", None)` == Some(&[Read, Write]);
  `requires("form_fill", Some("submit"))` == Some(&[Read, Write, Action]).
- pipeline inline: `boolean_action_key_extraction`: args `{"submit":true}` with action_key
  "submit" -> Some("submit"); `{"submit":false}` -> None; absent -> None.
- `tests/tool_schema_fidelity.rs` + `tests/all_open_golden.rs` + directory name test:
  cumulative arrays per PINS SS4 after C10 (final order; explain last).
- `tests/tool_enforcement.rs`: add `form_fill_denied_upfront_under_write_deny`: a manifest
  denying Write on the tab's domain denies the PARENT call (one denial, no partial fill) --
  follow the file's existing manifest-fixture pattern.
- `tests/tool_enforcement.rs`: add `form_fill_without_extension_fails_with_parent_audit`:
  all-open, no extension; call form_fill `{tabId:0, fields:{"Email":"a@b.c"}}`. Assert: the
  result is an isError text containing `extension`; captured audit contains a parent record
  tool `"form_fill"` (batch_id non-null, action null, capability from the None variant) AND a
  record tool `"form_structure"` with orchestrator `"form_fill"`, the same batch_id, step 1
  (the failed internal read still completes its record with a real duration_ms).

## Verification

Gates.

## Out of scope

Levenshtein matching, field selectors, file uploads (skip-report only), multi-page wizards,
`read_table`.

Commit: `feat(tools): form_fill -- semantic form interaction by label (ADR-0036)`
