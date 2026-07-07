# C2: CallOutcome split + async Handler::Local

Goal: the pipeline core returns a structured outcome; Handler::Local becomes async and
ctx-bearing; both Local dispatch positions exist. Client-visible output stays byte-identical.
Normative: ADR-0035 D6, PINS SS1 + SS2.

## Tree facts (as of authoring; re-read before editing)

- `src/transport/mcp/pipeline.rs:50` `handle_tools_call(browser, store, governance, id,
  params) -> JsonRpcResponse`; return sites: -32602 missing name (:62), unknown tool (:77-81),
  validation (:88-92), held (:118-121), sacred deny (:146-149), free-action Local arm
  (:172-176, runs `audit.complete()` then wraps `f()` text), Gate::Deny (:204), dispatch
  Ok/Err (:266-290), landing deny (:255). Module doc pins stage order (:21).
- `src/browser/directory.rs:64` `pub enum Handler { ExtensionForward, Local(fn() -> String) }`;
  `:745` explain's row uses `Handler::Local(explain_text)`; `:1192` an inline test matches
  `Handler::Local(_)`.
- `tests/all_open_golden.rs`, `tests/mcp_protocol.rs`, `tests/tool_enforcement.rs` exercise the
  envelopes end-to-end.

## STOP preconditions

- STOP if the free-action Local arm is not at pipeline.rs's position described above.
- STOP if introducing LocalCtx in directory.rs creates an import cycle AND the SS2 fallback
  module (`src/transport/mcp/outcome.rs`) ALSO cycles. (The fallback is expected to work.)

## Required behavior

1. Introduce `CallOutcome`/`DenialSource` per PINS SS1 (placement per SS2's import-direction
   note). Split `handle_tools_call` into `run_tool_call(...) -> CallOutcome` + the edge
   renderer, mapping outcomes to envelopes EXACTLY per SS1's table. `run_tool_call` gains the
   trailing parameter `orchestration: Option<(&'static str, &str, u32)>` (PINS SS7), applied
   via `audit.orchestrated(...)` immediately after `governance.begin`; pass `None` from
   `handle_tools_call`.
2. Handler::Local becomes SS2's fn-pointer-returning-boxed-future shape with `LocalCtx`.
   Migrate explain; its rendered text stays byte-identical.
3. Free-action arm: its guard CHANGES from "handler is Local" to "handler is Local AND the
   None-variant requires is EMPTY" (today only explain matches; C10's form_fill must NOT
   dispatch here). For a matching handler, await the future; if the returned Success result
   carries a top-level `_batch_id` string key, remove it and call `audit.set_batch_id(&value)`
   before `audit.complete()` (PINS SS7's parent-stamping side channel; explain sets none).
4. Post-grant Local arm: after `Gate::Proceed`, when `descriptor.handler` is Local (non-empty
   requires), dispatch the handler at the `browser.call` site position instead (same
   audit/postprocess flow; `_batch_id` handling identical). No registry row uses it yet.

## Tests (by name; assertions verbatim)

- All existing tests pass UNCHANGED (the identity proof; do not edit expectations).
- `pipeline.rs` inline `#[cfg(test)]`:
  - `calloutcome_render_table`: construct each variant, render, assert Success -> the result
    value verbatim; Failure -> an `isError: true` result; Denied/Held -> a text content result
    with the exact message and NO isError key (match today's `text_content` shape).
  - `local_batch_id_side_channel`: a synthetic Local handler returning Success with
    `_batch_id: "b-1"`; assert the rendered client result does NOT contain `_batch_id`.
- `directory.rs` inline test at :1192 updated mechanically to the new Local shape.

## Verification

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` -- with special
attention: `cargo test --test all_open_golden --test mcp_protocol --test tool_enforcement`.

## Out of scope

No new tools, no audit-field producers besides the orchestration parameter plumbing, no
behavior change visible to any client, no extension changes.

Commit: `refactor(pipeline): CallOutcome core + async ctx-bearing Handler::Local`
