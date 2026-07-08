# C9: the formStructure content-script read

Goal: the dedicated, value-free form identity read form_fill matches against.
Normative: ADR-0036 D5 step 1, PINS SS12.

## Tree facts (as of authoring; re-read before editing)

- content.js: message dispatch switch (:460 area), helpers `visible`, `refFor`, `collectAll`
  (shadow-DOM-aware traversal), label helpers inside `accessibleName` (:56-83) -- do NOT reuse
  accessibleName for the label field (it collapses sources; SS12 wants label[for]/wrapping
  ONLY).
- SW: handler map + `content(tabId, msg)` helper.

## STOP preconditions

- STOP if C4 is not committed (batch order; no technical dependency beyond it).

## Required behavior

1. content.js: `case "formStructure"` responding PINS SS12's shape exactly: forms (by
   containing `<form>`, document order, formIndex from 0), formless controls, per-control
   `{ref, type, label, placeholder, name, id, ariaLabel, disabled, readonly}` (null for absent
   strings; NO field values read), submit candidates `{ref, label, kind}` with the pinned
   labeled-button list ["submit","sign in","log in","save"] (normalized exact match).
   Controls = input (except type hidden), select, textarea; visibility-filtered.
2. SW: `form_structure_internal(a)` handler: `content(tabId, {type:"formStructure"})`, returns
   the raw object as `{content:[{type:"text",text:JSON.stringify(result)}]}` -- it is an
   internal read; no prose rendering. It gets NO REGISTRY row (models cannot call it; the
   binary's unknown-tool pre-check guarantees that; only C10's handler dials it via
   browser.call directly).

## Tests (by name; assertions verbatim)

- No node test (DOM-bound). Verification is C10's matcher fixtures (which mirror SS12's shape)
  plus a LEDGER note confirming: no `.value` access anywhere in the new content.js code (grep
  the added block for `.value` -- the only allowed hit is `el.type` handling, i.e. none).

## Verification

Gates (unchanged test set).

## Out of scope

Matching (C10), read_page changes, iframes (accepted limitation, ADR-0036).

Commit: `feat(extension): formStructure identity read for form_fill (ADR-0036 D5)`
