# PINS: ADR-0035..0038 composition batch

Shared, pinned implementation vocabulary. Task files cite these SS numbers instead of restating.
Semantics live in the ADRs (as amended 2026-07-06); these pins are the exact code-level shapes.
On any conflict between a pin and the live tree that the task file does not anticipate: STOP.

Provenance: authored 2026-07-06 against dev @ 6c5d351 by the frontier session that amended the
ADRs. Decided questions (do not re-litigate): parent calls ARE audited; additive audit keys go
at END of record; references resolve against structuredContent only; form_fill authorizes once
at the parent; script steps authorize per-step; idempotency is service-scoped; new tools insert
BEFORE `explain` in REGISTRY; ADR-0039 is NOT implemented in this batch.

## SS1: CallOutcome and the edge renderer (ADR-0035 D6)

New in `src/transport/mcp/pipeline.rs`:

```rust
pub(crate) enum DenialSource { Policy, Sacred }
pub(crate) enum CallOutcome {
    /// The MCP result object (the extension's `{content:[...]}` or a locally built one),
    /// post-processed, wait-note appended. May carry `structuredContent`.
    Success { result: Value },
    /// A tool execution failure (rendered as an isError result at the edge).
    Failure { error: ToolError },
    Denied { message: String, source: DenialSource },
    Held { message: String },
}
```

`handle_tools_call` splits into:
- `pub(crate) async fn run_tool_call(browser: &Browser, store: &Arc<ConfigStore>, governance:
  &Governance, name: &str, args: &Value) -> CallOutcome` -- everything from the registry lookup
  through post-dispatch, including per-call config snapshot, schema validation, hold, sacred,
  authorize, dispatch, landing re-check, postprocess, wait-note.
- `handle_tools_call` (unchanged signature) parses params, calls `run_tool_call`, renders.

Edge render table (byte-identical to today's envelopes -- the oracle):
| CallOutcome | Renders as |
|---|---|
| Success { result } | `JsonRpcResponse::success(id, result)` |
| Failure { error } | `JsonRpcResponse::success(id, error_result(error))` |
| Denied { message, .. } | `JsonRpcResponse::success(id, text_content(message))` |
| Held { message } | `JsonRpcResponse::success(id, text_content(message))` |

The missing-`name` -32602 protocol error stays in `handle_tools_call` (it is not a tool
outcome). Unknown tool and schema-validation failures become `Failure` (they render into the
same `error_result` success envelope as today). `tests/all_open_golden.rs` and the full suite
green is the identity proof.

## SS2: the new Handler::Local and its two dispatch positions (ADR-0035 D6)

In `src/browser/directory.rs`:

```rust
pub struct LocalCtx<'a> {
    pub browser: &'a Browser,          // crate::hub::outbound::browser::Browser
    pub store: &'a Arc<ConfigStore>,
    pub governance: &'a Governance,
    pub config: &'a Config,            // this call's snapshot
    pub args: &'a Value,
}
pub type LocalFuture<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = CallOutcome> + Send + 'a>>;
pub enum Handler {
    ExtensionForward,
    Local(for<'a> fn(LocalCtx<'a>) -> LocalFuture<'a>),
}
```

Import-direction note: `directory.rs` currently imports only governance ports + serde_json. If
naming Browser/Governance/ConfigStore in `directory.rs` creates a cycle, put `LocalCtx`,
`LocalFuture`, `CallOutcome`, and `DenialSource` in a small new engine module
`src/transport/mcp/outcome.rs` (SPDX `Apache-2.0 OR MIT`) and have both directory.rs and
pipeline.rs import from it. Either placement is sanctioned; a cycle is not.

`explain` migrates: `Handler::Local(|ctx| Box::pin(async move { let _ = ctx;
CallOutcome::Success { result: text_content_value(explain_text()) } }))` -- wrap however the
existing `text_content` result-object builder is reachable; the rendered text stays
byte-identical.

Dispatch positions (both pinned):
- A Local tool whose None-variant `requires` is EMPTY dispatches in today's free-action arm
  (where `explain` answers), position unchanged.
- A Local tool with NON-EMPTY requires falls through sacred + grant enforcement exactly like
  ExtensionForward, and dispatches at the `browser.call` site position via its handler instead.
  (`form_fill` is the first user, in C10; C2 lands the arm.)

## SS3: audit additive keys (ADR-0035 D7/D8, ADR-0036 D7)

`AuditRecord` (src/governance/ports.rs) currently ends at `held` (14 keys: event_id, ts,
identity, client, tool, action, capability, domain, decision, grant_id, denial_id, duration_ms,
manifest, held). Append, in this exact order, serialized after `held`:

```rust
    /// "script" | "form_fill" | None. Present only on orchestrated internal executions.
    pub orchestrator: Option<&'static str>,
    /// Correlates one parent call with its steps. Set on the parent AND each step/internal.
    pub batch_id: Option<String>,      // UUID v4 lowercase hyphenated
    /// 1-indexed position within the parent. None on the parent record itself.
    pub step: Option<u32>,
    /// true only on a script dry-run parent record. Never serialized as false: use
    /// #[serde(skip_serializing_if = "...")] ONLY if existing keys already do; otherwise emit
    /// literal false/null to match the record's existing always-present style. MATCH THE FILE:
    /// `held` is always present, so these four are always present too (null when None).
    pub dry_run: bool,
```

`CallAudit` (src/governance/dispatch.rs) gains:
- `pub fn orchestrated(&mut self, orchestrator: &'static str, batch_id: &str, step: Option<u32>)`
- `pub fn mark_dry_run(&mut self)`
- `pub fn attribute_grant(&mut self, grant_id: Option<String>)` (used by C10 internals)

Oracle (C1 test): a record with none of these set serializes with `"held":false,
"orchestrator":null,"batch_id":null,"step":null,"dry_run":false` as the final five keys, in
that order. Existing golden audit tests updated by APPENDING those keys to expected lines only.

## SS4: REGISTRY order, fidelity + golden expected arrays

Insertion point: new tools go immediately BEFORE the `explain` row. Cumulative advertised order:
- After C4: [...13 trained..., "wait_for", "explain"]
- After C7: [...13 trained..., "wait_for", "script", "explain"]
- After C10: [...13 trained..., "wait_for", "script", "form_fill", "explain"]

`tests/tool_schema_fidelity.rs`: EXPECTED_TRAINED stays EXACTLY 13; the explain-positioned-last
assertion stays; per tool task, extend the total-count and any full-name-list assertions to the
cumulative array above. `tests/all_open_golden.rs`: update its pinned advertised-name array the
same way. If either test byte-compares whole tool JSON beyond names/order/description presence:
STOP (task files say what to do).

## SS5: structured results mechanism (ADR-0038)

Extension service-worker: for tools with a vocabulary, build ONE source object, then render
text exactly as today AND set `result.structuredContent = <object>`. Binary: no dispatch change
(the result Value passes through opaque). Existing text strings byte-identical -- do not
reformat.

v1 vocabulary (== ADR-0038 D2 table, wait_for row as amended): find, tabs_context_mcp,
tabs_create_mcp, navigate, wait_for, form_fill, script, digest-twin on mutating actions.

`ToolDescriptor` gains `pub output_schema: Option<fn() -> Value>` (None on all rows except the
vocabulary tools); `advertised_tools_json` emits `"outputSchema"` when Some. navigate's
structured `{ "tabId": <i64>, "url": <final url>, "title": <title> }` is sampled via
`chrome.tabs.get` after navigation completes.

## SS6: reference grammar + resolver (ADR-0035 D2) -- src/transport/mcp/refs.rs

`pub(crate) fn resolve_refs(args: &Value, structured: &[Option<Value>]) -> Result<Value, String>`
-- returns a NEW args Value (immutably built) or the corrective error string. `structured[i]` is
step i+1's structuredContent (None if the step failed, was skipped, or its tool has none).

Rules: only STRING leaves are inspected. A leading `$$` unescapes to one literal `$`, done. A
string matching `^\$(prev|[1-9][0-9]*)(\.[^.]+)*$` is a reference (head + optional dot path);
any other `$`-string passes through unchanged. Path segments: numeric = array index, else object
key. Bare `$prev`/`$N` substitutes the whole structured value.

Pinned oracles (C7 unit tests, exact):
- `{"ref":"$prev.results.0.ref"}` over prev `{"results":[{"ref":"ref_12","x":5}],"more":false}`
  -> `{"ref":"ref_12"}`
- `"$$1.50"` -> `"$1.50"` ; `"$hello"` -> `"$hello"` (no grammar match)
- `"$1.50"` with step 1 structured `{"tabId":3}` -> Err containing exactly:
  `unresolved reference "$1.50": step 1 has no field "50". If you meant a literal string starting with "$", write "$$1.50".`
- `"$2.tabId"` when only 1 step has run -> Err containing `references step 2, but only 1 step has run`
- `"$prev.tabId"` when the previous step has NO structured result -> Err containing
  `has no structured result; only tools with a declared result vocabulary can be referenced`
- `"$0.x"` -> not grammar (index must be >= 1) -> passes through unchanged.

## SS7: script tool (ADR-0035 D1/D3/D4/D5) -- src/transport/mcp/script.rs

Directory row (before explain): tool `"script"`, action_key None, one variant
`{action: None, requires: &[], directory_description: "Run up to 20 tool calls sequentially in one request; each step is authorized and audited individually."}`,
resource `DomainLess`, handler Local, postprocess None, post_dispatch None.

Advertised description (exact):
"Run a sequence of tool calls in one request. Steps execute in order; each step is validated, authorized, and audited exactly as if called individually. Step arguments may reference a prior step's structured result: $prev.field for the previous step, $N.field for step N (1-indexed), with .0-style numeric segments indexing arrays (example: $prev.results.0.ref after find). Write $$ for a literal leading $. Only tools with structured results (find, tabs_context_mcp, tabs_create_mcp, navigate, wait_for) can be referenced. Steps may not include script itself. Use wait_for between navigate and reads on dynamic pages."

inputSchema (exact): object; properties: tabId (integer, optional; steps inherit it when their
args omit tabId), steps (array, minItems 1, maxItems 20; items: object with required
tool:string, args:object), onError (string enum ["stop","continue"], default "stop"), dry_run
(boolean), idempotency_key (string), budget_ms (integer); required ["steps"];
additionalProperties false. If `validation.rs` cannot express nested item validation, validate
steps' inner shape in the handler with the same corrective-error style instead: STOP is NOT
needed for that case.

Interpreter: for each step -- inherit tabId if absent; reject `tool == "script"` (status
"error", corrective text `script steps may not include script itself`); resolve refs (SS6; a
resolution error = status "error", no dispatch); check budget; call `run_tool_call`; map
CallOutcome -> status: Success->"ok", Failure->"error", Denied->"denied", Held->"held".
Held STOPS unconditionally; remaining steps status "not_run". onError "stop": any non-ok stops.
Budget: `config.script_budget_ms()` clamped by arg `budget_ms` (arg may only lower); on
exhaustion remaining steps "not_run". Per-step text: the result's first text content, truncated
at 2000 chars with `(truncated)`; whole compact result capped 25000 chars. Include
`"structured"` per step when present. Compact result (a Success whose result is a text
rendering of this JSON, plus the SAME object as structuredContent):

```json
{"results":[{"step":1,"tool":"navigate","status":"ok","result":"...","structured":{}}],
 "summary":"3/4 steps completed; step 4 denied","duration_ms":3400}
```

Summary strings (exact): all ok -> `N/N steps completed`; stopped by failure at K ->
`{K-1}/N steps completed; step K failed`; denied at K -> `...; step K denied`; held at K ->
`...; held at step K`; budget -> `...; budget exhausted after step {K}`.

Audit: parent record via the normal pipeline path (requires [], free-action allow) with
batch_id set (a fresh UUID minted by the handler and handed to the parent's CallAudit --
plumbing pin: the handler receives its own CallAudit? NO. The parent's audit is completed by
the free-action arm before the handler returns; C2 must move the free-action-arm Local
`audit.complete()` so the handler can stamp batch_id first: the arm calls
`audit.orchestrated("script", &batch, None)` is WRONG (parent has no orchestrator). Pin:
CallAudit gains nothing new for this; the arm passes `&mut audit` INTO the Local handler via
LocalCtx? No -- keep it simple and honest: LocalCtx gains `pub audit: &'a mut CallAudit` is a
borrow tangle. FINAL PIN: the free-action arm, for Local handlers, sets
`audit.set_batch_id(value)` AFTER the handler returns, reading the batch id from a
`CallOutcome::Success { result }` side-channel key `_batch_id` that the script handler embeds
at the result's top level and the arm REMOVES before rendering. `set_batch_id` is a fourth
CallAudit method added in C1.) Step records: each `run_tool_call` re-entry begins its own
CallAudit; the interpreter cannot reach it, so orchestrator/batch/step stamping for STEPS moves
into `run_tool_call` itself: `run_tool_call` gains a final parameter
`orchestration: Option<(&'static str, &str, u32)>` (None from handle_tools_call; Some from
interpreters), applied via `audit.orchestrated(...)` right after `governance.begin`.

## SS8: dry-run + idempotency (ADR-0035 D8/D9) -- C8

> SUPERSEDED (2026-07-06, post-C8). What landed instead: dry-run as a `run_tool_call`
> parameter running the real decision path (would_allow/would_deny, no indeterminate; navigate
> verdicts carry a pre-dispatch landing caveat), and NO idempotency cache at all (ADR-0035 D9
> re-amended to not-taken; the pipeline-level rebuild is ADR-0040, Proposed). See the C8
> LEDGER entry. The body below is the pre-implementation pin, kept for history; C10 and later
> tasks must NOT implement anything from it.

Dry-run: interpreter loop without dispatch: per step evaluate registry lookup, schema
validation, ref-grammar validation, sacred + authorize verdicts (tab-URL probe allowed, no tool
frames). Statuses: "would_allow" | "would_deny" | "indeterminate". Parent audit: mark_dry_run();
no step records. navigate steps: "indeterminate" when the verdict depends on landing.

Idempotency: `src/transport/mcp/idempotent.rs`, cache OWNED BY `Browser` (add field; Browser is
the one service-scoped engine handle the pipeline already has). API:
`pub async fn run_idempotent(browser:&Browser, tool:&'static str, key:&str, fut: impl Future<Output=Value>) -> (Value, bool)`
-- (result, replayed). In-flight duplicate AWAITS the original (tokio::sync::watch or Notify)
and returns (original, true); completed repeat within TTL returns (stored, true); else runs,
stores, returns (fresh, false). Constants: `MAX_ENTRIES: usize = 64`, `TTL: Duration =
Duration::from_secs(600)`. Eviction: oldest-inserted beyond 64, lazily on insert. `replayed:
true` is injected as a top-level key of the compact/form_fill result object. Replays write no
audit records.

## SS9: wait_for (ADR-0037 D1/D5/D6 as amended) -- C4

Schema (exact): object; properties tabId (integer), selector (string), text (string), state
(string enum ["visible","present","gone","settled"]), timeout_ms (integer), min_ms (integer),
settle (boolean); required ["tabId"]; additionalProperties false. Handler-level validation
(corrective): selector+text together invalid; state "settled" with selector/text invalid;
min_ms > timeout_ms invalid; timeout_ms capped 30000 (larger value -> corrective error, not
clamp). Defaults: state "visible", timeout_ms 10000, min_ms 0, settle true.

Directory row (before explain): requires `&[Capability::Read]`, resource TabScoped,
ExtensionForward, action_key None, postprocess None, post_dispatch None. Advertised description
(exact): "Wait until the page is ready. By default waits for BOTH your condition and page
settlement (DOM mutation rate decayed). Provide selector (CSS) or text (visible substring) with
state visible|present|gone, or call with neither to wait for settlement alone. min_ms sets a
minimum elapsed time; settle:false gates on the condition only. Returns elapsed_ms, settle
diagnostics, and the matched element's ref for follow-up clicks. Times out with an error naming
what WAS on the page."

extension/lib/settle.js (pure; loaded by BOTH worlds: added to manifest content_scripts js
array BEFORE content.js, and to the SW importScripts list is NOT needed):
- `settleThreshold(peak)` = `Math.max(Math.floor(peak * 0.05), 3)`. Oracles: 400->20, 100->5,
  80->4, 61->3, 60->3, 59->3, 30->3, 0->3.
- `createSettleDetector()` -> `{ push(count) -> bool /* settled now */, peak, lastRate,
  windows }`; first pushed window NEVER counts as a candidate; settled when count < threshold
  for 3 consecutive pushes (after window 1). Oracles (feeds -> settled-at-window, peak, final):
  [400,200,80,15,10,2] -> settled at window 6, peak 400, final_rate 2;
  [5,1,0,0] -> window 4, peak 5, final_rate 0;
  [10,4,4,4,4,4,4,4] -> never;
  [300,2,2,100,50,10,5,2,1] -> window 9, peak 300, final_rate 1;
  [0,0,0,0] -> window 4, peak 0, final_rate 0.
  NOTE [300,2,2,100,...]: windows 2,3 are candidates 1,2; window 4 (100 >= 15) RESETS.
content.js: `waitFor` message (async sendResponse): 250ms polls for the condition; a
MutationObserver counter binned into 500ms windows feeds the detector; return when
(condition-met AND settled-per-settle-flag AND elapsed >= min_ms) or timeout. Response
`{ found, settled, elapsedMs, ref, peakMutations, finalRate }` (ref via existing refFor when a
condition matched an element; settled fields omitted when settle:false). SW `wait_for(a)`
handler: validates, forwards, renders text
`Condition met after {elapsed}ms (settled; peak {peak} mutations/window).` / bare form
`Page settled after {elapsed}ms (peak {peak} mutations/window).`, +`structuredContent` per SS5;
timeout -> hopError("page", `did not settle within {timeout}ms (still changing at ~{rate} mutations/500ms)` or
`"{text}" not visible within {timeout}ms. Page title: "{title}".`) matching ADR wording.

## SS10: consequence digests (ADR-0037 D2) -- C5

extension/lib/observation.js (pure): `formatObservation(sig)` where sig may carry
`{url, title, mutations, focus, alert, status, dialog}`. Segment order (exact): url ->
`url changed to {url}`; title -> `title changed to "{title}"`; mutations (>0) ->
`{n} DOM mutations`; focus -> `focus moved to "{focus}"`; alert -> `alert appeared: "{text}"`;
status -> `status appeared: "{text}"`; dialog -> `dialog appeared`. Join `"; "`, prefix
`observation: `, empty -> `observation: no observable change`, cap 400 chars (truncate + `...`).
Oracles: `{}` -> `observation: no observable change`;
`{url:"/dashboard",mutations:47,focus:"Search",alert:"Changes saved"}` ->
`observation: url changed to /dashboard; 47 DOM mutations; focus moved to "Search"; alert appeared: "Changes saved"`.
Actions covered: computer left_click, right_click, double_click, triple_click, type, key,
left_click_drag, hover, scroll_to, plus form_input. 300ms settle sample in content.js
(observe message pair around the action from the SW side; alert/status/dialog detection =
elements with those roles appearing during the window; first 200 chars of textContent).
Placement pin: `formatObservation` runs IN content.js (observation.js is a content-script
global via the manifest, NOT importScripts); content.js returns the FINISHED digest string and
the structured twin; the SW appends the string verbatim, separated by `\n`. Structured twin
`{url_changed?, title_changed?, focus?, mutations, alert?, dialog_appeared?}` via SS5.

## SS11: read_page diff + stale refs (ADR-0037 D3/D4) -- C6

read_page inputSchema: ADD optional `"diff": {"type":"boolean", "description":"Return only
changes since your previous read_page on this tab (+ added, - removed, ~ changed)."}` --
trained fields/enums untouched. extension/lib/treediff.js (pure):
`diffLines(oldLines, newLines)` -> `{added:[...], removed:[...], changed:[...]}`; a line's key =
its first `ref_\d+` token, else the whole line; changed = same key, different text. Render
order: changed (`~ `), removed (`- `), added (`+ `), each group in new-tree order (removed in
old-tree order). Oracle: old `["ref_1 button \"A\"","ref_2 link \"B\""]`, new
`["ref_1 button \"A2\"","ref_3 link \"C\""]` -> changed `["ref_1 button \"A2\""]`, removed
`["ref_2 link \"B\""]`, added `["ref_3 link \"C\""]`. No baseline -> full tree prefixed line 1
`(no baseline; full tree)`. Render serial: content.js increments per 500ms window with >= 3
mutations; refs minted during a read remember the serial; deref-miss errors become:
`{ref} no longer resolves: the page re-rendered since your last read (render serial {a} -> {b}). Call read_page (or read_page with diff: true) and use a fresh ref.`

## SS12: formStructure read (ADR-0036 D5) -- C9

content.js message `{type:"formStructure"}` -> `{result: {forms:[{formIndex, controls:[...],
submits:[...]}], formless:[...controls]}}`. Control: `{ref, type, label, placeholder, name, id,
ariaLabel, disabled, readonly}` -- label = label[for] / wrapping-label text ONLY (null if none);
type = tagName-based: "textarea", "select", else input.type or "text"; NO VALUES READ. Submit
candidate: `{ref, label, kind}`, kind in "button-submit" | "input-submit" | "labeled-button";
labeled-button = a button whose normalized accessible name is exactly one of
["submit","sign in","log in","save"]. Visibility-filtered via existing `visible()`; refs via
existing `refFor`.

## SS13: form_fill (ADR-0036, all decisions) -- C10

Matcher `src/browser/form_match.rs` (pure; SPDX Apache-2.0 OR MIT): serde types mirroring SS12;
`match_fields(keys, structure) -> MatchOutcome {matched: Vec<(key, ControlRef)>, unmatched:
Vec<(key, Vec<Candidate>)>, form_index: Option<usize>}`. Normalization: casefold + trim +
collapse whitespace. Tiers: exact > prefix (source startsWith key) > substring (either
contains); tier beats source; source priority within tier: label > placeholder > name/id >
ariaLabel. Keys resolved longest-normalized-first; each control consumed once; substring-only
tie across distinct controls -> unmatched with candidates. Form selection: score = 2*exact +
1*other per form; highest wins; tie -> lower formIndex; keys matching only other forms ->
unmatched. Oracles (fixture: form 0 = [ref_1 label "Email Address" name "email", ref_2 label
"Confirm Email Address", ref_3 label "Password", ref_4 label "Confirm Password"]):
keys {"Confirm Password","Password","Email"} -> ref_4, ref_3, ref_1 (prefix tier), no
unmatched. Keys {"name"} over [ref_7 label "First name", ref_8 label "Last name"] -> unmatched
with candidates [ref_7, ref_8]. Key "email" over [A label "Email Address"(prefix), B
name="email"(exact)] -> B.

Directory row (before explain): tool "form_fill", action_key Some("submit"), variants:
`[{action: None, requires: &[Read, Write]}, {action: Some("submit"), requires: &[Read, Write, Action]}]`,
resource TabScoped, handler Local (post-grant arm, SS2), postprocess None, post_dispatch None.
Pipeline action extraction (the ONE sanctioned tweak): where `args.get(key).and_then(Value::as_str)`
runs, booleans map: `true` -> the action_key NAME (`"submit"`), `false`/absent -> None. (So the
audit `action` field reads "submit" on submitting fills -- pinned.)

inputSchema (exact): object; properties tabId (integer), fields (object, minProperties 1,
additionalProperties allowing string|boolean|number), submit (boolean); required
["tabId","fields"]; additionalProperties false. (No idempotency_key: SS8 supersession note.) Advertised description
(exact): "Fill a form by field labels in one call. Provide fields as a map from a label,
placeholder, or name attribute to the value (string, number, or boolean for checkboxes).
Matching is case-insensitive and specificity-ordered; ambiguous keys are returned unmatched
with candidates instead of guessed. submit:true clicks the form's own submit control after
filling. Passwords are masked in the result. Falls back cleanly: anything unmatched can be
filled with form_input using the refs in the result."

Handler flow: formStructure (direct `browser.call`? NO -- there is no formStructure TOOL; the
SW must expose it: C9 adds SW handler `form_structure_internal` reachable via
`browser.call("form_structure_internal", {tabId})`; it is NOT in REGISTRY, so models cannot
call it -- run_tool_call rejects unknown names before dispatch, and the handler bypasses the
registry by calling browser.call directly, which is sanctioned for this internal read ONLY);
no idempotency wrap (SS8 supersession note); match; fill each via internal executor: `browser.held_for()` check -> abort (remaining
skipped, reason "held"); `browser.call("form_input", {tabId, ref, value})`; audit each internal
(begin("form_input", None, requires("form_input", None)) + orchestrated("form_fill", batch,
step) + attribute_grant(parent grant) + complete()); submit via
`browser.call("computer", {action:"left_click", tabId, ref})` similarly audited (the digest
from C5 rides its result text -> `observation` field). formStructure internal read is audited
as tool "read_page"? NO -- audit truth: record tool "form_structure" orchestrated step 1
(begin("form_structure", None, Some(&[Capability::Read]))). Result object per ADR-0036 D3
(password masking: matched control type "password" -> value "********"), structuredContent +
text render (text = compact human summary, pinned in task).

## SS14: config key (C7)

`pub const ENGINE_SCRIPT_BUDGET_MS: &str = "engine.script.budget_ms";` KeyDef: description
"Total wall-clock budget for one script tool call, in milliseconds.", constraint UintRange
{min: 1000, max: 480000}, defaults Uint(120000) for fully_open, safe, restricted (no behavioral
gating). Accessor `Config::script_budget_ms() -> u64` next to `first_call_wait_ms`.

## SS15: CI + manifest cumulative oracles

ci.yml extension test line is cumulative; exact expected value after each task:
- After C4: `- run: node --test tests/extension/constants.test.js tests/extension/geometry.test.js tests/extension/keys.test.js tests/extension/settle.test.js`
- After C5: append ` tests/extension/observation.test.js`
- After C6: append ` tests/extension/treediff.test.js`
(grouping.test.js is a pre-existing omission; do NOT add it.)

extension/manifest.json content_scripts js array, cumulative:
- After C4: `["lib/settle.js", "content.js"]`
- After C5: `["lib/settle.js", "lib/observation.js", "content.js"]`
- After C6: `["lib/settle.js", "lib/observation.js", "lib/treediff.js", "content.js"]`

## SS16: cost-note guidance strings (C11)

Locate the browser capability's AgentGuide/instructions text (grep `AgentGuide`). Append a
paragraph titled `Cost notes:` with EXACTLY these four sentences: "get_page_text can return
tens of thousands of tokens on document-heavy pages; prefer find for targeted lookups and
read_page filter interactive for form work. A screenshot costs roughly 1,600 tokens; prefer
read_page or find when you need targets rather than appearance. read_page full is large on
complex pages; filter interactive is dramatically smaller, and diff true returns only changes
since your last read. script steps still cost a browser round-trip each; use wait_for between
navigation and reads."
