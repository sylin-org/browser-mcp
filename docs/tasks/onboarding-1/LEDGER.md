# onboarding-1 batch: LEDGER

Durable progress for the onboarding-1 batch. One task = one commit. Update this file at the end
of every task. This is the single source of truth for "where are we"; a fresh executor resumes
from RESUME HERE with no other context.

## Baseline

- Branched from `dev` at 4d8f2de, with the ADR-0031 commit (207a7f3 on security-1) cherry-picked
  as the starting point (commit 43ec639 on this branch).
- 584 tests passing, clippy clean, fmt clean, working tree clean.

## RESUME HERE

**o03 (Emit initialize.instructions from agentGuide) is NEXT.** o01 and o02 landed.

## o01 -- Reconcile ADR-0031

Status: DONE (0157fa1).

## o02 -- Add agentGuide + per-tool example to tools.json

Status: DONE (this commit).

Files: `src/transport/mcp/schemas/tools.json` (additive only).

What landed: the additive agent-facing content in the fixture. A new top-level `agentGuide`
section (summary + workflow + flow + denials, ~350 tokens), and an `example` block per tool.
The 13 trained tools each carry a complete, valid `example.call`; deterministic-shape tools
(navigate, tabs_context_mcp, tabs_create_mcp, update_plan) also carry `example.returns`;
page-dependent tools (find, form_input, get_page_text, javascript_tool, read_console_messages,
read_network_requests, resize_window) omit `returns`. `read_page` carries `example.returns`
pinning the `ref_N` invariant (the page-independent fact that refs flow to form_input.ref and
computer.ref). `computer`'s example uses `screenshot` as a representative action and its
`returns` documents the action-dependent return shape. `explain` (the 14th, unsanctioned tool)
omits `example` per the ADR (argument-less, self-describing).

The 14 existing tool objects' `name`/`description`/`inputSchema` are byte-stable: all 7 existing
fidelity tests pass unchanged.

Verification: 567 tests pass (the dev baseline; security-1's +17 are on a separate branch),
clippy `-D warnings` clean, fmt clean. JSON validates; agentGuide carries all four fields; 14
tools present. The example-against-schema validation lands in o05.

Deviation D1: `update_plan.example.call` carries full `domains` + `approach` arrays (required
fields) -- the plan said "deterministic-shape tools carry returns" but update_plan's return is
auto-approved echo, so it carries a short `returns` string noting that. No impact on later tasks.

## o02 -- Add agentGuide + per-tool example to tools.json

Status: TODO.

## o03 -- Emit initialize.instructions from agentGuide

Status: TODO.

## o04 -- Hard-fail schema validation with corrective errors (flagship)

Status: TODO.

## o05 -- Extend the fidelity test

Status: TODO.

## Deviation format

```
D<n>: <what the plan said> -> <what you actually did> because <the tree fact that forced it>.
     Impact on later tasks: <none | names the task + what it must now assume>.
```
