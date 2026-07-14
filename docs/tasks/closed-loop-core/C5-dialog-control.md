# C5: Dialog control

## Goal

Make JavaScript dialog blockers visible and explicitly resolvable without hidden automation.

## Read before editing

- ADR-0005, ADR-0022, ADR-0034, ADR-0066, ADR-0078
- registry, pipeline, and dynamic-requirement code
- CDP session/event handling in the extension
- C2 receipt and C3 semantic-interaction code

## Implementation

1. Add the additive `dialog` tool and pinned actions from PINS P6 with const schemas.
2. Track only the minimum current per-tab CDP dialog state needed for status and dispatch. Clear it
   on resolution, navigation, tab close, session cleanup, and panic.
3. Classify status as Read and accept/dismiss/respond as Action before dispatch.
4. Surface `dialog_open` in relevant receipts. Never auto-accept or auto-dismiss.
5. Keep dialog text out of audit. Return it to the model only when needed for the active status or
   recovery result, bounded and marked as page-sourced.

## Tests

- Status/no-dialog, each resolution action, respond text validation, and stale event cleanup.
- Ownership, sacred-tab, hold, RAWX, audit minimization, and script/batch composition.
- Dialog blocker appears in `act_on`; no mutation continues through an unresolved dialog.
- Tool count grows additively; trained schemas stay byte-stable.

## Commit

`feat(browser): add explicit dialog control (ADR-0078)`
