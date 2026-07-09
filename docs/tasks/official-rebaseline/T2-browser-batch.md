# T2 -- browser_batch (trained front door overloading the shared script engine)

Goal: add the additive trained tool `browser_batch` per ADR-0050 Decision 3. It takes
`actions: [{name, input}]`, runs them through the SAME sequential engine `script` uses, and returns
the result in browser_batch's trained shape (per-item content, images interleaved). `script` is KEPT
unchanged. Read ADR-0050 Decision 3 now; it is normative.

Runs AFTER T1. At T2 start the tree already has `file_upload` (tool count 18). Read BOOTSTRAP.md.

## STOP preconditions (re-read; if any is false, STOP)

- Tool count is 18 and `tests/tool_schema_fidelity.rs` asserts `names[16] == "file_upload"` and
  `names[17] == "explain"` (i.e. T1 landed). If count is 17, run T1 first.
- `crates/core/src/mcp/script.rs` still defines `fn interpret<R: StepRunner>(...)`, `trait StepRunner`,
  `struct PipelineRunner`, `fn script_handler(ctx)`, and the `#[cfg(test)] mod tests` with tests
  `all_ok_summary_is_n_of_n`, `nested_script_step_errors`, `tabid_is_inherited_by_steps_that_omit_it`.
- `crates/core/src/mcp/server.rs` still has the batch-reject teaching message ending
  "use the `script` tool." (as-of-authoring near line 629).
- `tests/mcp_protocol.rs` still asserts `msg.contains("`script`")` for the batch-reject teaching test.

## Part A -- Refactor script.rs: separate EXECUTION from FORMATTING (script output unchanged)

`interpret` currently runs the step loop AND formats the compact result (`build_compact`). Extract the
loop so both front doors share it, WITHOUT changing script's output.

A1. Add a `pub(crate)` execution function that runs the steps and returns the raw per-step outcomes
    plus run metadata -- everything `interpret` computes before `build_compact`. Suggested shape (adapt
    names to the live code; re-read):

        pub(crate) struct StepOutcome {
            pub step: u32,
            pub tool: String,
            pub status: &'static str, // "ok" | "error" | "denied" | "held" | "not_run"
            pub result: serde_json::Value, // the step's FULL MCP result (content array + optional
                                           // structuredContent); Null for a not_run step. Preserving
                                           // the full content is what lets browser_batch keep images.
        }
        pub(crate) struct BatchRun {
            pub steps: Vec<StepOutcome>,
            pub summary: String,
            pub duration_ms: u64,
            pub batch_id: String,
        }
        pub(crate) fn run_batch<R: StepRunner>(args: &Value, runner: &mut R, config_budget_ms: u64, dry_run: bool) -> BatchRun

    Move the existing loop body of `interpret` (budget gate, nesting check, tabId inheritance,
    resolve_refs, runner.run, status_of, hold-stops-unconditionally, onError, not_run backfill,
    summarize, batch_id) into `run_batch`. Populate `StepOutcome.result` with the step's full
    `CallOutcome` result (for Success: the `result` Value; for Failure/Denied/Held: a JSON
    `{"content":[{"type":"text","text": <message>}]}` synthesized from the existing `step_text`).

A2. Reimplement `interpret` as: `let run = run_batch(...); build_compact(run)`. `build_compact` keeps
    producing the EXACT compact object it does today (the same `results`/`summary`/`duration_ms`/
    `_batch_id` shape, the 2000-char step-text truncation, the 25000-char cap). It derives each
    compact entry's `result` text and `structured` twin from `StepOutcome.result` (first content-array
    text block; `structuredContent` for the twin) -- the SAME values as before. ALL existing
    `script.rs` unit tests must pass UNCHANGED (they are the regression guard proving the refactor is
    behavior-preserving for `script`).

A3. NESTING (symmetric, per ADR-0050 D3): the nesting rejection in `run_batch` rejects a step whose
    tool is `"script"` OR `"browser_batch"` (either batcher inside either batcher). Keep the existing
    error text for a `script` step ("script steps may not include script itself"); for a
    `browser_batch` step use "browser_batch steps may not include a batch tool". The existing test
    `nested_script_step_errors` must still pass.

## Part B -- browser_batch.rs (new module) + mod.rs

B1. Add `pub mod browser_batch;` to `crates/core/src/mcp/mod.rs` (next to `pub mod script;`).

B2. New `crates/core/src/mcp/browser_batch.rs` with:

    pub(crate) fn browser_batch_handler(ctx: LocalCtx<'_>) -> LocalFuture<'_>

    mirroring `script_handler` (wire a `PipelineRunner` from `ctx`). It:
    1. Translates `ctx.args["actions"]` (array of `{name, input}`) into a script-shaped args object:
       `{ "steps": [ {"tool": a["name"], "args": a["input"]} ... ], "tabId": ctx.args["tabId"]? }`.
       If `actions` is absent/empty, return a Success whose content is a single text block
       "browser_batch requires a non-empty `actions` array" (do not panic).
    2. Calls `run_batch(&translated, &mut runner, ctx.config.script_budget_ms(), false)`.
    3. Formats the browser_batch result via `build_batch_result(run)` (below) and returns
       `CallOutcome::Success { result }`. browser_batch has NO structuredContent and NO `_batch_id`
       side channel (it is not referenced; that machinery is script's).

B3. `fn build_batch_result(run: BatchRun) -> Value` (in browser_batch.rs): build ONE MCP result whose
    `content` array is, IN ORDER:
    - for each `StepOutcome` with status != "not_run": if status == "ok", EXTEND content with that
      step's `result["content"]` array blocks verbatim (this preserves text AND image blocks); else
      PUSH one `{"type":"text","text": "step {step} ({tool}) {status}: {first text block of result}"}`.
    - finally PUSH one `{"type":"text","text": run.summary}`.
    Return `{ "content": <that array> }` via `crate::mcp::types` helpers where they fit; a plain
    `json!({"content": [...]})` is acceptable. "not_run" steps contribute no content (the summary
    reports them).

## Part C -- The REGISTRY row (`crates/core/src/browser/directory.rs`)

Insert immediately before the `explain` row (after `file_upload`, which T1 placed there):

    ToolDescriptor {
        tool: "browser_batch",
        advertised_description: "Execute a sequence of browser tool calls in ONE round trip. Each item is {name, input} where input is exactly what you'd pass to that tool standalone. Actions execute SEQUENTIALLY (not in parallel) and stop on the first error. Use this tool extensively to quickly execute work whenever you can predict two or more steps ahead -- e.g. navigate, click a field, type, press Return, screenshot. Each tool's own permission check runs per item -- if an action navigates to a domain without permission, the next item's check fails and the batch stops. Screenshots and other images are returned interleaved with outputs; coordinates you write in THIS batch refer to the screenshot taken BEFORE this call. browser_batch cannot be nested.",
        input_schema: || json!({
            "type": "object",
            "properties": {
                "actions": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Tool name (e.g. computer, navigate, find, tabs_create). browser_batch cannot be nested." },
                            "input": { "type": "object", "description": "That tool's input -- same shape you'd pass when calling it directly." }
                        },
                        "required": ["name", "input"]
                    },
                    "description": "List of tool calls to execute sequentially. Example: [{\"name\":\"computer\",\"input\":{\"action\":\"left_click\",\"coordinate\":[100,200],\"tabId\":123}}, {\"name\":\"computer\",\"input\":{\"action\":\"type\",\"text\":\"hello\",\"tabId\":123}}, {\"name\":\"navigate\",\"input\":{\"url\":\"https://example.com\",\"tabId\":123}}]"
                }
            },
            "required": ["actions"],
            "additionalProperties": false
        }),
        example: Some(ToolExample {
            call: r#"{"actions":[{"name":"navigate","input":{"url":"https://example.com","tabId":0}},{"name":"computer","input":{"action":"screenshot","tabId":0}}]}"#,
            returns: Some("Each action's output, with screenshots interleaved, in order; stops on the first error."),
        }),
        action_key: None,
        variants: &[ActionVariant {
            action: None,
            requires: &[],
            directory_description:
                "Run a sequence of tool calls in one round trip; each item is name+input, authorized per item.",
        }],
        resource: ResourceShape::DomainLess,
        handler: Handler::Local(crate::mcp::browser_batch::browser_batch_handler),
        postprocess: None,
        post_dispatch: PostDispatch::None,
        output_schema: None,
    },

## Part D -- Fidelity / golden pins (sacred surface; deltas are relative to the POST-T1 tree)

1. `directory.rs` `EXPECTED` requires table: insert `("browser_batch", None, &[]),` immediately before
   the `("explain", None, &[])` line (after the `("file_upload", None, &[Capability::Write])` line
   T1 added).
2. `directory.rs` `total_variants` assert: `31` -> `32`.
3. `directory.rs` `EXPECTED_TOOLS` table: insert before the `explain` row:
   `("browser_batch", None, ResourceShape::DomainLess, true, false, PostDispatch::None),`
   (`true` = Local handler.)
4. `tests/tool_schema_fidelity.rs` position test: count `18` -> `19` (both the `names.len()` and the
   `all.len()` asserts and their messages, adding `browser_batch`). Change the tail asserts to:

        assert_eq!(names[16], "file_upload", ...);
        assert_eq!(names[17], "browser_batch", "the 18th tool is browser_batch, immediately before explain");
        assert_eq!(names[18], "explain", "explain stays positioned last");

5. `tests/all_open_golden.rs`: `[&str; 18]` -> `[&str; 19]`; insert `"browser_batch",` before
   `"explain",`; bump the count message and the doc comment to 19.
6. `tests/mcp_protocol.rs`: the tool-count assert `18` -> `19`, add `browser_batch` to the message.
7. `directory.rs` doc-comment counts (`18 descriptors`, `Linear scan over 18 rows`) -> 19.
8. `crates/core/src/hub/outbound/mod.rs`: bump BOTH `cap.directory().len()` and
   `reg.aggregated_directory().len()` asserts `18` -> `19`, and the "18-declaration REGISTRY" prose.
9. `tests/tool_enforcement.rs` (`all_open_invariant_no_manifest_means_no_denials`): bump
   `tools.len()` assert `18` -> `19`, add `browser_batch` to its message and the "18 tools" comment.
10. LEAVE `output_schemas_present_exactly_where_declared` unchanged (browser_batch has
   `output_schema: None`).

## Part E -- Repoint the ADR-0049 batch-reject teaching message (amends ADR-0049 D4)

1. `crates/core/src/mcp/server.rs`: in the batch-reject teaching message, change the final sentence
   from "use the `script` tool." to "use the `browser_batch` tool." (leave the rest verbatim).
2. `tests/mcp_protocol.rs`: change `assert!(msg.contains("`script`"), "teaches the script-tool
   alternative: {msg}");` to `assert!(msg.contains("`browser_batch`"), "teaches the browser_batch
   alternative: {msg}");`. Update the nearby doc comment ("use `script` for multi-step") to
   `browser_batch`.

## Part F -- Tests (add by name; assertions pinned)

Add to `crates/core/src/mcp/browser_batch.rs` a `#[cfg(test)] mod tests` (mirror script.rs's
`StubRunner` seam -- since `run_batch` is generic over `StepRunner`, reuse it: make `StubRunner` and
`run_batch` reachable from the test, e.g. via `use crate::mcp::script::...` if you make the stub
`pub(crate)`, or replicate a minimal stub). Pinned tests:

- `browser_batch_translates_actions_to_steps`: feed `run_batch` a translated args object built from
  `actions:[{name:"find",input:{query:"x"}},{name:"navigate",input:{url:"u"}}]`; assert the stub
  runner received `tool == "find"` then `tool == "navigate"`, with args `{query:"x"}` / `{url:"u"}`.
- `browser_batch_result_flattens_content_in_order`: two ok steps returning text "a" and "b"; assert
  `build_batch_result` content is `[{text:"a"},{text:"b"},{text:"2/2 steps completed"}]` (three text
  blocks, in that order).
- `browser_batch_preserves_image_blocks`: a step whose result content is `[{"type":"image","source":
  {...}}]`; assert that image block appears verbatim in the flattened content.
- `browser_batch_stops_on_first_error_and_notes_it`: steps ok then denied then (a third); assert the
  third is not_run (no content for it) and the summary is "1/3 steps completed; step 2 denied".
- `a_batch_tool_step_is_rejected`: a step named `"browser_batch"` (and one named `"script"`) yields
  status "error" and is never dispatched.
- Existing `script.rs` tests: ALL pass unchanged (regression proof).
- The updated `tests/mcp_protocol.rs` batch-reject test asserts `browser_batch`.

## Out of scope

- Do NOT rename, remove, or alter `script` (kept as the richer batcher). Do NOT add `$prev`/`onError`/
  `dry_run`/`budget_ms` to `browser_batch` (those stay script's; browser_batch is byte-faithful to the
  trained schema).
- No upload_image/gif_creator work.

## Commit

One commit: `feat(tools): browser_batch -- trained batch front door over the shared script engine (ADR-0050 D3)`.
Update the LEDGER T2 entry.
