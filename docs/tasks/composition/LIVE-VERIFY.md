# LIVE-VERIFY: composition batch (operator checklist, NOT an executor task)

The unattended executor cannot drive a real browser, and the automated gates cannot reach
content.js/service-worker behavior (e2e-smoke is quarantined). This checklist is run BY THE
OPERATOR after C11, with Chrome + the debug extension + a live MCP client, following the house
precedent of the stage-4 live pass. Each item pins the expected observation; a miss is filed as
a post-batch bug, not fixed by the executor.

1. **wait_for, bare settle** on a JS-heavy page (a news site): `wait_for {tabId}` returns
   `Page settled after Nms (peak P mutations/window).` with N typically 2000-8000 and P > 20;
   structuredContent carries found/settled/elapsed_ms/peak_mutations/final_rate.
2. **wait_for, condition**: `wait_for {tabId, text:"<known heading>"}` returns the matched ref;
   `computer left_click` on that ref works.
3. **wait_for, timeout**: `wait_for {tabId, text:"zzz-not-present", timeout_ms:3000}` returns
   an isError naming what WAS on the page (title present in the message).
4. **Digest, real change**: click a link -> confirmation gains
   `observation: url changed to ...`. Click an inert area ->
   `observation: no observable change`.
5. **Digest, alert**: submit a form that shows a toast/alert -> `alert appeared: "..."` segment.
6. **read_page diff**: read_page; type into one field; `read_page {diff:true}` -> only `~`
   lines (a handful), NOT a full tree. First read on a fresh tab with diff:true ->
   `(no baseline; full tree)`.
7. **Stale ref**: read_page on an SPA; navigate in-app; form_input with an old ref -> the
   corrective error naming render serials and suggesting a re-read.
8. **script happy path**: steps [navigate example.com, wait_for {min_ms:1000}, find "More
   information", computer left_click $prev.results.0.ref] -> `4/4 steps completed`; the audit
   file shows 1 parent + 4 step records sharing one batch_id, steps 1..4.
9. **script dry_run** under the restricted preset with a Write-denying manifest: a form_input
   step reports `would_deny` with the REAL denial text; nothing dispatches; parent record has
   dry_run true and no step records exist. A navigate step's `would_allow` carries the suffix
   `(pre-dispatch verdict; the post-redirect landing is checked live)`.
10. **Retry behavior (idempotency NOT SHIPPED -- ADR-0040 is the follow-up)**: confirm the
    schema advertises no idempotency_key on script or form_fill, and note in the ledger any
    observed client-timeout-and-retry during items 8-11 (real-world data for ADR-0040's
    constants).
11. **form_fill**: on a simple login-style form (e.g. a local fixture page or
    https://httpbin.org/forms/post): fields matched per label, unmatched keys listed with
    candidates, password value masked as `********` in the result, `submitted` accurate, and
    with C5 present an `observation` field after submit. Audit: parent form_fill + form_structure
    + one form_input per field + optional computer click, all sharing one batch_id.
12. **structuredContent on the wire**: a raw MCP `tools/call` of `find` shows
    `structuredContent` alongside byte-identical text; `tools/list` shows `outputSchema` on
    exactly find/tabs_context_mcp/tabs_create_mcp/navigate/wait_for/script/form_fill.
13. **Sacred + hold regressions**: with a sacred domain configured, a script step touching it
    reports status `denied`; engaging take-the-wheel mid-script stops the script with `held`
    at that step and `not_run` after.

Record outcomes in LEDGER.md under a `### LIVE-VERIFY` heading (pass/fail per item).
