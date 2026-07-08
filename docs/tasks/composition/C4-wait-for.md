# C4: the wait_for tool + settle detector

Goal: condition + settlement waiting per amended ADR-0037 D1/D5/D6. Normative: those decisions,
PINS SS9 (schema, strings), SS4 (order), SS15 (manifest + CI).

## Tree facts (as of authoring; re-read before editing)

- `extension/content.js`: IIFE; message dispatch at :460 area (`case "find"` etc.); helpers
  `visible`, `refFor`, `deref`. No MutationObserver counter exists yet.
- `extension/manifest.json:48` content_scripts js is `["content.js"]`.
- `extension/service-worker.js`: tool handler map (methods like `async find(a)`); helper
  `content(tabId, msg)`; error style `hopError("page", msg)`.
- `src/browser/directory.rs`: REGISTRY rows end with explain; inline tests pin name arrays.
- `.github/workflows/ci.yml:56` the node --test line (PINS SS15 baseline).
- `tests/tool_schema_fidelity.rs` EXPECTED_TRAINED (13) + explain-last assertion;
  `tests/all_open_golden.rs` pinned name array.

## STOP preconditions

- STOP if C2/C3 are not committed (Handler shape + output_schema field must exist).
- STOP if content.js's async sendResponse pattern (`return true`) is not available for a
  long-running handler (it is used today; confirm).

## Required behavior

1. `extension/lib/settle.js`: pure module per PINS SS9 (`settleThreshold`,
   `createSettleDetector`), loadable both as a content-script global and under node --test
   (follow lib/constants.js's export pattern).
2. content.js: `waitFor` message: 250ms condition polls; MutationObserver counter binned into
   500ms windows feeding the detector; return per SS9's condition (condition AND settle-gate
   AND min_ms), response fields per SS9. Timeout responds `{timeout: true, rate, title,
   excerpt}` for the SW to render.
3. SW `wait_for(a)`: defaults + corrective validations per SS9 (selector+text; state "settled"
   with a condition; min_ms > timeout_ms; timeout_ms > 30000). Success text and timeout
   `hopError` strings EXACTLY per SS9. `structuredContent` per amended ADR-0038 vocab.
4. Directory row per SS9 (requires [Read], TabScoped, ExtensionForward, before explain) with
   SS9's advertised description and this example:
   call `{"tabId":0,"text":"Results"}`, returns "Waits for the text AND page settlement;
   returns elapsed_ms, settle diagnostics, and the matched element's ref."
   output_schema Some (shape per amended ADR-0038 table).
5. manifest.json + ci.yml per PINS SS15 (after-C4 values exactly).

## Tests (by name; assertions verbatim)

- `tests/extension/settle.test.js`: `settleThreshold`: 400->20, 100->5, 80->4, 61->3, 60->3,
  59->3, 30->3, 0->3. Detector feeds (PINS SS9): [400,200,80,15,10,2] settles at window 6 with
  peak 400, lastRate 2; [5,1,0,0] at 4; [10,4,4,4,4,4,4,4] never (8 pushes, still false);
  [300,2,2,100,50,10,5,2,1] at 9; [0,0,0,0] at 4.
- `tests/tool_schema_fidelity.rs`: cumulative array per PINS SS4 after C4 (14 trained+wait_for
  count = 15 total with explain); explain still last.
- `tests/all_open_golden.rs`: name array extended identically.
- directory.rs inline name-order test extended identically.

## Verification

Gates; the node --test line now includes settle.test.js (SS15).

## Out of scope

Digests (C5), diff (C6), any change to `computer` `wait`, network-idle conditions.

Commit: `feat(tools): wait_for -- condition + adaptive settle detector (ADR-0037)`
