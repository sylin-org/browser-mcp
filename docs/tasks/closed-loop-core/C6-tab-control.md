# C6: Tab control

## Goal

Add explicit controls for one Ghostlight-owned tab without weakening the managed-tab boundary or
inventing automatic cleanup.

## Read before editing

- ADR-0034, ADR-0047, ADR-0061, ADR-0066, ADR-0078
- tab ownership and grouping code in service and extension
- registry, pipeline, sacred-tab, and cleanup tests

## Implementation

1. Add the additive `tab_control` tool with PINS P6 actions and const schemas.
2. Enforce session ownership for every variant. Focus only the specified owned tab. Reload or close
   only that tab after Action authorization.
3. Closing a tab removes its ownership and transient state deterministically. It does not close
   another tab, delete a group, or infer that other tabs are disposable.
4. Add receipt and audit categories without storing page content.

## Tests

- Focus, reload, and close happy paths and validation.
- RAWX none for focus; Action for reload/close.
- Foreign and user tabs refused; sacred/hold/panic behavior preserved.
- Close cleanup is exact and idempotent at the ownership seam; group remains intact.
- Script/browser_batch use and additive surface count; trained schemas unchanged.

## Commit

`feat(browser): add explicit owned-tab control (ADR-0078)`
