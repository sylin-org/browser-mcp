# C11: cost-aware guidance

Goal: the capability guide tells models what tools cost before they pay. Normative: ADR-0038
D5, PINS SS16. SKIP allowed.

## Tree facts (as of authoring; re-read before editing)

- The browser capability's AgentGuide (grep `AgentGuide` -- trait method `agent_guide()` in
  `src/hub/outbound/mod.rs`, browser impl in `src/hub/outbound/browser.rs` or where the guide
  text constant lives). The scoped agentGuide composes into `initialize.instructions`
  (registry batch t05).

## STOP preconditions

- STOP if no guide/instructions text surface exists to append to (then LEDGER-note where
  guidance actually lives and mark BLOCKED).

## Required behavior

1. Append PINS SS16's `Cost notes:` paragraph VERBATIM to the browser capability's guide text
   (the surface that reaches `initialize.instructions`).
2. If wait_for/script/form_fill landed with per-tool guidance surfaces, no further edits: the
   cost paragraph is capability-level by design.

## Tests (by name)

- Extend whichever existing test pins the instructions/guide content (grep `instructions` in
  tests/) by asserting it CONTAINS `Cost notes:` and the get_page_text sentence's first eight
  words. If no test pins it, add
  `tests/tool_advertisement.rs::instructions_carry_cost_notes` doing exactly that.

## Verification

Gates.

## Out of scope

Description strings of any tool, measured/per-domain hints, explain output.

Commit: `docs(guide): cost-aware coaching in the browser capability guide (ADR-0038 D5)`
