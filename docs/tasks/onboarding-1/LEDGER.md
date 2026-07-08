# onboarding-1 batch: LEDGER

Durable progress for the onboarding-1 batch. One task = one commit. Update this file at the end
of every task. This is the single source of truth for "where are we"; a fresh executor resumes
from RESUME HERE with no other context.

## Baseline

- Branched from `dev` at 4d8f2de, with the ADR-0031 commit (207a7f3 on security-1) cherry-picked
  as the starting point (commit 43ec639 on this branch).
- 584 tests passing, clippy clean, fmt clean, working tree clean.

## RESUME HERE

**The batch is COMPLETE.** o01-o05 all landed. See PLAIN STATEMENT below.

## o01 -- Reconcile ADR-0031

Status: DONE (0157fa1).

## o02 -- Add agentGuide + per-tool example to tools.json

Status: DONE (8965581).

## o03 -- Emit initialize.instructions from agentGuide

Status: DONE (928c02a).

## o04 -- Hard-fail schema validation with corrective errors (flagship)

Status: DONE (this commit).

Files: new `src/transport/mcp/validation.rs` (`ToolSchema`, `validate_arguments`, the corrective
hint generators, 7 inline unit tests), `src/transport/mcp/mod.rs` (declare the new module),
`src/transport/mcp/pipeline.rs` (wire the validator in after the registry lookup), 6 test files
adapted to well-formed args (see deviation D2), `docs/adr/0031-agent-onboarding-contract.md`
(Decision 4 refined: three structural checks, NOT enum; see deviation D3).

What landed: the flagship. inputSchema violations are now REJECTED before dispatch with a
corrective `ToolError::invalid_request(...).next_step(...)`, in the same shape the "Unknown tool"
path already uses. The three structural checks (unknown property, missing required, wrong type)
each produce a corrective message naming the field and a derived suggestion (the example shape
from the fixture; for `tabId` specifically, "get one from tabs_context_mcp first"). The
behavioral tightening: a missing `tabId` (today: silent None -> extension error) is now an
explicit corrective error -- exactly the untrained-model delight the ADR targets.

Verification: 576 tests pass (was 569), clippy `-D warnings` clean, fmt clean.

Deviation D2: the validator's tightening rippled through 8 existing tests that sent minimal/malformed
args (no tabId, etc.) and relied on the validator's absence. Each was updated to send well-formed
args; their oracles (the actual behavior under test -- redaction, audit records, hold text,
chunking, multiplex routing) are unchanged. The one substantive change: `resource_shape_drives_
resolution` previously asserted a `read_page` call with no tabId reaches governance's fail-closed
"(unknown)" denial; under o04 the validator now catches that earlier with a STRICTLY BETTER
corrective error ("missing required field 'tabId'; get one from tabs_context_mcp first"). The
test now asserts the new (better) behavior. `all_open_golden.rs::read_page_redaction` (a
NEVER-touch fence per the BOOTSTRAP) was adapted minimally -- its input args gained a tabId so
the call reaches the redaction chokepoint; the redaction oracle (the byte-stable text) is
unchanged. Impact on later tasks: none.

Deviation D3: the plan and the ADR named FOUR checks (missing required, wrong type, unknown enum,
unknown property). Implementation removed the enum check: governance already handles an unknown
`computer.action` fail-closed with a stable denial id, which is MORE informative than a generic
validation error. Enforcing enums in the validator would shadow that well-designed path. The ADR
is updated to record three structural checks + the explicit "enums NOT checked" rationale.
Impact on later tasks: o05 must NOT assert enum-example validation; it asserts the three
structural checks only.

Files: `src/transport/mcp/tools.rs` (new `agent_guide_text()` helper + 2 inline unit tests; the
module grew from a single const to a real helper module), `src/transport/mcp/server.rs`
(`initialize_result` gains an `instructions` field; the import widens to bring in
`agent_guide_text`).

What landed: MCP `initialize` now carries the agent onboarding guide (ADR-0031 Decision 1). The
helper parses the fixture's top-level `agentGuide` and concatenates the four fields (summary,
workflow, flow, denials) into the single string MCP's `instructions` field expects. The service
constructs nothing -- pure passthrough of the fixture's prose. The `instructions` field is
additive, so the existing initialize-touching tests (mcp_protocol, all_open_golden) pass
unchanged.

Verification: 569 tests pass (was 567; +2 new inline unit tests on `agent_guide_text`), clippy
`-D warnings` clean, fmt clean. The two new tests pin: all four agentGuide fields render verbatim
in order, and the flow line is labeled ("Typical flow:") so a reader recognizes the spine.

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

Status: DONE (this commit).

Files: `tests/tool_schema_fidelity.rs` (+2 assertions: agentGuide well-formed; every trained
tool's example.call validates against its own inputSchema via the o04 validator).

What landed: the contract is now pinned end to end. The agentGuide assertion guards all four
fields non-empty + the tabId-first rule present. The example-validation assertion runs each
trained tool's `example.call` through the SAME validator o04 wires into the pipeline -- so a
future trimmed-or-stale example is mechanically uncommittable (CI fails), exactly the drift
class ADR-0031 Decision 5 targets. The validator's per-case behavior is pinned by o04's 7 inline
unit tests; this test pins the fixture side.

Verification: 580 tests pass (was 576), clippy `-D warnings` clean, fmt clean. The 7 pre-existing
fidelity assertions stay byte-stable; o05 only ADDS the two new ones.

## PLAIN STATEMENT (per BOOTSTRAP)

The onboarding-1 batch is complete on the `onboarding-1` branch (5 commits beyond the cherry-
picked ADR baseline, one per task). Baseline was `dev` at 4d8f2de (567 tests); the batch closes
at 580 tests, clippy `-D warnings` clean, fmt clean.

What landed (ADR-0031 in full): an untrained model connecting to ghostlight now receives, at
handshake, the workflow contract (every tab-touching tool requires a tabId; get one from
tabs_context_mcp first; then navigate) plus cost discipline and denial-flow notes via MCP's
native `initialize.instructions` field, sourced verbatim from the fixture; in `tools/list`, a
valid `example.call` shape per trained tool; and on a malformed call, a corrective `ToolError`
naming the missing/wrong field and the example shape, generated from the fixture. The first-call
success rate for untrained models stops being a function of guessing.

The fixture (`tools.json`) is the single source of agent-facing truth. Three consumers
(initialize.instructions, tools/list, the validation-error path) all derive from it; the service
constructs no agent-facing prose. Drift is a CI failure: the fidelity test now validates every
example against its own schema.

Three deviations from the original plan, all recorded inline (D1: update_plan example carries a
short returns string; D2: 8 existing tests adapted to well-formed args after the validator's
tightening, including a minimal NEVER-touch-fence edit; D3: enum check removed -- governance
already handles unknown actions fail-closed with a better denial). The ADR is reconciled to
match (Decision 3 withdrawn; Decision 4 records three structural checks + the enums-NOT-checked
rationale).

What is NOT done here: the live engine verification against a real untrained-model session (the
whole point of the batch, but it comes AFTER, as the proof). The branch is left for a human to
merge into `dev` when ready, then rebuild, then re-test the live MCP connection so the model
receives the new initialize.instructions and the corrective errors.

## Deviation format

```
D<n>: <what the plan said> -> <what you actually did> because <the tree fact that forced it>.
     Impact on later tasks: <none | names the task + what it must now assume>.
```
