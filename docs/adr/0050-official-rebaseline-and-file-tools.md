# ADR-0050: The 1.0.80 re-baseline -- official as sole reference, file_upload, browser_batch (script reshaped), upload_image, gif_creator

Status: Proposed (2026-07-09). Amends ADR-0035 (adds `browser_batch` as a second front door that
overloads the shared script engine; `script` is UNCHANGED and kept) and ADR-0049 Decision 4 (the
batch-reject teaching pointer moves from `script` to `browser_batch`); exercises ADR-0034 Decision 7
(additive tool growth via the registry); removes the `upload_image` exclusion in SPEC section 10 and
the CLAUDE.md "No upload_image tool" line; retires the community reference named in the CLAUDE.md
Origin section.

This ADR is the single normative source for the execution batch in `docs/tasks/official-rebaseline/`.
The task prompts CITE this document instead of restating semantics. Exact test oracles (pinned
expected strings, counts, positions) live in the task prompts, computed from the schemas fixed here.

## Context

The motivating discovery: an agent driving Ghostlight could not upload a file (attach a PDF to
ServiceNow, publish a docx to SharePoint), because Ghostlight has no file tool and cannot drive the
OS-native file-picker dialog. Investigation of the INSTALLED official Claude-in-Chrome extension
(id `fcoeoabgfenejglbffodgkkbkcdhcgfn`, now at v1.0.80 -- our last harvest, docs/research/12, was
v1.0.78) found that the official surface has GROWN and now ships four capabilities we lack:

- `file_upload` -- upload one or more files (base64 bytes) to a located file input. FUNCTIONAL.
- `upload_image` -- upload a prior screenshot / user image to a file input OR drag-drop target.
- `browser_batch` -- execute a sequence of tool calls in one round trip (a batching wrapper).
- `gif_creator` -- record browser actions and export an animated GIF.

The full harvested schemas and mechanisms are in the session scratchpad
(`scratchpad/harvest/HARVEST-1.0.80.md`); the normative extracts are inlined per-decision below.

Two owner directives frame the work: (1) implement all four new capabilities; (2) drop the community
reimplementation entirely and treat the official extension as the sole reference going forward.

Key architectural facts established by a read-only map of the current tree (three study agents):

- Tools are `ToolDescriptor` rows in `const REGISTRY` in `crates/core/src/browser/directory.rs`
  (a `&'static str` `advertised_description` + an `input_schema: fn() -> Value` inline `json!`
  closure). Registration = adding a row; advertisement, grant-filtering, dispatch, and `explain`
  all derive from `REGISTRY`. The whole file is strict ASCII; every em-dash is written `--`.
- Capability class (read/action/write/execute, `crates/core/src/governance/ports.rs`) is carried
  per `ActionVariant.requires: &'static [Capability]` on the row; `directory::requires(tool, action)`
  is the classifier. `None` (a classification miss) is fail-closed deny under a manifest.
- Fidelity is a set of HAND-MAINTAINED Rust asserts (no golden fixture file, no regeneration flag).
  The advertised-tool COUNT is hard-coded in SEVEN sites that all derive from `REGISTRY` and must be
  bumped in lockstep: `tests/tool_schema_fidelity.rs` (2), `tests/all_open_golden.rs`
  (`GOLDEN_TOOL_NAMES` length + 1 assert), `tests/mcp_protocol.rs` (1), `tests/tool_enforcement.rs`
  (1, `all_open_invariant_no_manifest_means_no_denials`), `crates/core/src/hub/outbound/mod.rs` (2,
  `browser_capability_exposes_the_full_directory` + `registry_aggregates_the_browser_directory`),
  plus the `#[cfg(test)]` pin tables inside `directory.rs` (`total_variants`, `with_action_key`, the
  `EXPECTED` + `EXPECTED_TOOLS` tables). A new tool appends AFTER the 13 trained tools and the existing
  additive tools, BEFORE `explain` (which stays last); every count / ordered position moves in lockstep.
- The pipeline (`crates/core/src/mcp/pipeline.rs`) is the single enforcement point: validity ->
  schema validation -> capability classification -> audit-begin -> hold -> sacred-domain check ->
  governing-resource resolution -> grant authorization (host polarity + capability subset) ->
  dispatch -> audit-complete. Arguments stay raw `serde_json::Value` (no per-tool struct). Audit
  records carry NO tool arguments (only the sub-action), so there is nothing to redact in audit;
  sensitive result values are masked via a `postprocess` hook.
- The extension keeps a usable element-ref map: `refToEl` / `deref(ref)` in `extension/content.js`,
  refs formatted `ref_N`, populated by read_page / find / form_input / wait_for. DOM-executing tools
  flow native message -> `handlers[tool]` (service-worker.js) -> `content(tabId, {type,...})` ->
  a `case` in the `content.js` `onMessage` switch -> `{result}`. No `<input type=file>` support
  exists today.

## Decision 1 -- Official v1.0.80 is the sole reference; retire the community reimplementation

The community `reference/open-claude-in-chrome/` (a Node.js reimplementation) is retired: its
`upload_image` was a non-functional stub and it has drifted from the official surface it proxied. The
official installed extension is the sole ground truth for tool interface (names, params, enums,
description strings) and technique (CDP sequences, algorithms). We continue to harvest INTERFACE and
TECHNIQUE and reimplement leanly; we do NOT copy official code into the repo (Anthropic proprietary;
our engine is open, the governance module source-available).

Concretely: remove `reference/open-claude-in-chrome/` and `reference/ANALYSIS.md` from the tree (git
history preserves them); rewrite the CLAUDE.md Origin section to name the official extension as the
reference and keep a one-line clean-room-provenance acknowledgement; the fidelity baseline is
whatever `REGISTRY` renders, re-verified against v1.0.80.

## Decision 2 -- `file_upload` (new additive tool)

Model-facing schema (ASCII; the em-dash rendered `--`, matching every existing description in
`directory.rs`; the description is otherwise the official string verbatim):

    name: "file_upload"
    description: "Upload one or multiple files to a file input element on the page. Do not click on
      file upload buttons or file inputs -- clicking opens a native file picker dialog that you
      cannot see or interact with. Instead, use read_page or find to locate the file input element,
      then use this tool with its ref to upload files directly."
    input_schema (type object, additionalProperties false):
      files: array, items { type object, properties { data: string, name: string, mimeType: string },
             required ["data","name"] }, description "Files to upload, as base64-encoded bytes."
      paths: array of string, description "DEPRECATED. Use `files` instead."
      ref:   string, description 'Element reference ID of the file input from read_page or find
             tools (e.g., "ref_1", "ref_2").'
      tabId: number, description "Tab ID where the file input is located. Use tabs_context first if
             you don't have a valid tab ID."
      required: ["ref","tabId"]

Governance: ResourceShape `TabScoped`; single `ActionVariant` with `requires: &[Capability::Write]`
(bytes leave the user's control into a web destination; the `ref` was located by a separately-governed
read_page/find, so this call does not itself Read). Handler `ExtensionForward` -- the tool name and
args ride the existing `tool_request` envelope; NO new Rust arg struct and NO new wire type.

Mechanism (extension, page world): a new `setFiles(ref, files)` in `content.js` resolves the element
via `deref(ref)` (with `innerInput` shadow unwrap), requires `INPUT`/`type=file`, builds a
`DataTransfer` from `atob(data)` -> `Uint8Array` -> `new File([bytes], name, {type: mimeType ||
"application/octet-stream"})`, assigns `el.files`, and dispatches `input`+`change` (bubbling,
composed) -- exactly the official technique and the same event tail `setFormValue` already uses. The
`File`/`DataTransfer` is built in the content script, never passed over messaging. A new
`case "setFiles"` in the `onMessage` switch and a new `async file_upload(a)` handler in
`service-worker.js` (mirroring `form_input`, wrapped in `withObservation`) complete the path.

`paths` is advertised (trained-shape fidelity) but rejected binary- or extension-side with the
official message ("file_upload no longer accepts host filesystem paths. The MCP controller must read
the file and pass its contents via the `files` parameter."). Ghostlight NEVER reads the host
filesystem: the caller supplies bytes, so file_upload introduces NO local-filesystem trust boundary.

## Decision 3 -- `browser_batch` as a trained front door OVERLOADING the shared batch engine (script kept)

`browser_batch` is the trained batching tool; `script` (ADR-0035) is Ghostlight's richer batcher
(`$prev`/`$N` data-flow between steps, `onError`, `dry_run`, `budget_ms`). We KEEP `script` unchanged
and ADD `browser_batch` as a new additive tool that OVERLOADS the same execution engine. One engine,
two front doors: `browser_batch` = the trained, familiar, minimal batcher; `script` = the power-user
superset. `browser_batch` is purely additive -- it does NOT supersede or rename `script`.

Model-facing schema (ASCII; official description verbatim with `--` for em-dashes; NO Ghostlight
extras -- `browser_batch` stays byte-faithful to the trained shape, the extras live on `script`):

    name: "browser_batch"
    description: "Execute a sequence of browser tool calls in ONE round trip. Each item is
      {name, input} where input is exactly what you'd pass to that tool standalone. Actions execute
      SEQUENTIALLY (not in parallel) and stop on the first error. Use this tool extensively to
      quickly execute work whenever you can predict two or more steps ahead -- e.g. navigate, click a
      field, type, press Return, screenshot. Each tool's own permission check runs per item -- if an
      action navigates to a domain without permission, the next item's check fails and the batch
      stops. Screenshots and other images are returned interleaved with outputs; coordinates you
      write in THIS batch refer to the screenshot taken BEFORE this call. browser_batch cannot be
      nested."
    input_schema:
      actions: array, minItems 1, items { type object, properties { name: string, input: object },
               required ["name","input"] }, description "List of tool calls to execute sequentially.
               Example: [ ... ]" (the official example string, verbatim)
      required: ["actions"]

The overload is BOTH input and output translation over a shared core:
- INPUT: the handler maps each `actions[i]` -> an engine step (`name` -> the step tool,
  `input` -> the step args), then runs the SAME sequential executor `script` uses (per-step validate/
  authorize/audit, stop-on-first-error == `script`'s `onError:stop`).
- OUTPUT: the handler formats results in `browser_batch`'s TRAINED result shape -- per-item outputs
  with screenshots/images interleaved -- which is DISTINCT from `script`'s structured,
  `$prev`-referenceable result. `browser_batch` items therefore do NOT expose `$prev`/`$N`
  (that is `script`'s contract).

Implementation: extract `script`'s core executor into a shared function called by BOTH the `script`
handler and the new `browser_batch` handler (both `Handler::Local`, `requires: &[]` -- the batch tool
itself is free; every sub-step is independently classified and authorized). Nesting is rejected
symmetrically: neither `browser_batch` nor `script` may appear as a step inside either batcher.

ADR-0049 Decision 4 (a JSON-RPC batch array is rejected with a teaching message pointing at `script`)
is amended: the pointer becomes `browser_batch` (the trained batching tool the model already reaches
for). That teaching string and its `mcp_protocol.rs` oracle move in lockstep.

## Decision 4 -- `upload_image` (screenshot-cache imageId; drag-drop; trimmed)

The official resolves `imageId` against the app's CONVERSATION message history, which a pure MCP
server does not have. Ghostlight instead resolves `imageId` against a binary-side, per-session,
bounded screenshot cache that the `computer` screenshot action populates: a screenshot mints a stable
id, caches its bytes, and surfaces the id ADDITIVELY in the screenshot result (the `computer` INPUT
schema -- a trained surface -- is untouched; only an additive output field is added). `upload_image`
then resolves `imageId` -> cached bytes and forwards them like `file_upload`, choosing the file-input
DataTransfer path (for `ref`) or the DragEvent dragenter/dragover/drop path (for `coordinate`).

TRIM: the "user-uploaded image" source (an image dragged into the Claude side panel) has no Ghostlight
equivalent and is dropped; the schema `imageId` description is adjusted to name only "a previously
captured screenshot". Params otherwise match the official (`imageId`, `ref` XOR `coordinate`, `tabId`,
`filename` default "image.png"). Governance: `TabScoped`, `requires: &[Capability::Write]`.

This is the highest-risk task (it touches the `computer` result and adds a cache subsystem); it is
sequenced after file_upload and browser_batch and MAY be trimmed further or split during execution.

## Decision 5 -- `gif_creator` (phased)

`gif_creator` records browser actions (frames) and exports an animated GIF with overlays. It is a
self-contained subsystem: extension-side frame capture + GIF encoding + overlay rendering + export.
It is phased and sequenced LAST:

- Phase 1: `action` enum start_recording | stop_recording | clear | export-with-`download:true`
  (produce the GIF and return/download it). Capability per-action (like `computer`): recording
  controls classify Read/none; a download-export classifies Read (it yields a file to the client, no
  web write).
- Phase 2: export via `coordinate` drag-drop upload to a page element (reuses upload_image's
  DragEvent mechanism); that export classifies Write.

Full schema and encoder choice are pinned in the T4 task prompt. If execution time is constrained,
Phase 1 alone is a coherent, landable increment.

## Decision 6 -- Re-baseline the existing 13 trained schemas against v1.0.80

docs/research/12 harvested v1.0.78 and logged schema corrections (navigate `force`, get_page_text
`max_chars`, computer.duration max 10, javascript_tool wording, enum orders, prose bare-name usage,
etc.), some marked DONE. Re-verify each against v1.0.80, apply any still-unapplied or newly-drifted
delta as ADDITIVE / description-only changes (never renaming a trained param or enum), and refresh the
fidelity asserts. This is bounded, per-tool, and independently landable; it does not gate T1-T4.

## Decision 7 -- Sequencing, stop-anywhere value, and the NEVER list

Task order (each prefix leaves a green, coherent tree; the first task is smallest and standalone):
T1 file_upload -> T2 browser_batch (overload of the shared script engine; `script` kept) ->
T3 upload_image -> T4 gif_creator -> T5 re-baseline + reference retirement. T5's reference-retirement
half (Decision 1) may land first as a pure docs/removal change if convenient.

NEVER (for the executor; each NEVER names its sanctioned exception):
- NEVER change any of the 13 trained tools' names, parameter names, enum values, or description
  strings. Exception: Decision 6 permits ADDITIVE optional params and description-only edits on the
  existing tools, and an ADDITIVE output field on `computer` for Decision 4 -- never a rename/removal.
- NEVER edit the `EXPECTED_TRAINED` block in `tests/tool_schema_fidelity.rs`. Exception: none.
- NEVER remove the ADR-0049 no-initialize-before-use guard behavior (reconnect replay depends on it).
- NEVER write non-ASCII into code (`.rs`/`.js`/`.json`): em-dash -> `--`, no arrows/curly quotes.
- `directory.rs`, `tests/tool_schema_fidelity.rs`, `tests/all_open_golden.rs` are the sacred surface:
  edited ONLY to APPEND the new additive rows/names and bump the hand-maintained counts and pin
  tables; `explain` stays last in every ordered list.

## Consequences

- ADR-0035's `script` tool is UNCHANGED and kept; `browser_batch` is added as a second additive front
  door over the SAME engine (script's core executor is extracted into a shared function called by
  both handlers).
- ADR-0049 D4's teaching pointer and its `mcp_protocol.rs` oracle move to `browser_batch`.
- SPEC section 10 loses the `upload_image` exclusion; CLAUDE.md loses "No upload_image tool" and its
  Origin section names the official extension.
- The trained surface grows by four tools via the ADR-0034 D7 additive path; the fidelity snapshot
  (a regression guard, not a byte-freeze) is re-pinned to the new declared surface.
- Ghostlight gains file egress into web destinations as a governed Write capability; it still never
  reads the host filesystem (callers supply bytes).

## Provenance (decided; do not re-litigate)

- Bytes-in-call, not host paths: the official deprecated host paths; the caller/controller reads the
  file. Ghostlight relays bytes only -> no local-FS trust boundary. `paths` is advertised-but-rejected
  purely for trained-shape fidelity and self-correction.
- Keep `script` AND add `browser_batch` as an overload of the shared engine (owner steer): serves the
  trained `browser_batch` name/shape without losing `script`'s data-flow power. One engine, two
  front doors, with BOTH input and output translated for `browser_batch`. `browser_batch` stays
  byte-faithful to the trained schema (no extras); `script` carries `$prev`/`onError`/`dry_run`/
  `budget_ms`. The two batchers are differentiated in their descriptions (fast-familiar vs
  data-flow), so model choice is clear.
- Official-only reference: the community clone was a lossy proxy with a dead upload stub; single
  source of truth removes drift.
- em-dash -> `--`: prose punctuation is not a trained-behavior load-bearing token (names/params/enums
  are, and are all ASCII); the whole `directory.rs` precedent is `--`.
- upload_image trim: no conversation/side-panel access in a pure MCP server; the screenshot-cache
  path is the faithful, self-contained analogue.
- gif_creator phasing: it is a whole subsystem; Phase 1 (record + download-export) is independently
  valuable and lands without the drag-drop export.
