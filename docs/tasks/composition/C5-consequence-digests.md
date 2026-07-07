# C5: consequence digests on mutating actions

Goal: every mutating, non-screenshot action's confirmation gains the `observation:` block.
Normative: ADR-0037 D2 (as amended: block always present), PINS SS10, SS15. SKIP allowed.

## Tree facts (as of authoring; re-read before editing)

- SW `computer` action handlers return text confirmations for the SS10 action list;
  `form_input` returns a text confirmation (service-worker.js :1160 area).
- C4 landed the content.js MutationObserver counter (reuse it; do not add a second observer).
- manifest/ci baselines are the after-C4 values (PINS SS15).

## STOP preconditions

- STOP if C4 is not committed. STOP if any SS10-listed action currently returns a screenshot
  (only screenshot/scroll/zoom may).

## Required behavior

1. `extension/lib/observation.js`: pure `formatObservation(sig)` per PINS SS10 (segment order,
   strings, `observation: no observable change`, 400-char cap) with node-compatible exports.
2. content.js: an `observe` sampling message pair: snapshot (url, title, focused accessible
   name, counter value) before the action, sample again 300ms after; detect newly appeared
   role=alert/status text (first 200 chars) and role=dialog. SW calls it around each SS10
   action and appends `"\n" + digest` to the existing confirmation text (existing text
   untouched).
3. Structured twin: set/merge `structuredContent` per PINS SS10's shape on those results.
4. form_fill interplay: none yet (C10 consumes the digest from the submit click's text).
5. manifest.json + ci.yml per PINS SS15 after-C5 values.

## Tests (by name; assertions verbatim)

- `tests/extension/observation.test.js`:
  - `formatObservation({})` === `"observation: no observable change"`.
  - `formatObservation({url:"/dashboard",mutations:47,focus:"Search",alert:"Changes saved"})`
    === `"observation: url changed to /dashboard; 47 DOM mutations; focus moved to \"Search\"; alert appeared: \"Changes saved\""`.
  - A 500-char alert input yields a string of length <= 400 ending `"..."`.
  - Segment order: url before title before mutations before focus before alert before status
    before dialog (single case exercising all seven).

## Verification

Gates; node line per SS15.

## Out of scope

Screenshot-returning actions, navigate, opt-out config keys, tuning the 300ms window.

Commit: `feat(engine): consequence digests on mutating actions (ADR-0037 D2)`
