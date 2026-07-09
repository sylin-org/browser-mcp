# T1 -- file_upload

Goal: add the additive MCP tool `file_upload` per ADR-0050 Decision 2. It uploads base64 file bytes
(supplied in the call) to a file `<input>` located by a `ref` from read_page/find, via an in-page
DataTransfer. It never reads the host filesystem. Read ADR-0050 Decision 2 now; it is normative.

This is the FIRST task and the template for T2-T5. Read BOOTSTRAP.md first (authority order, oracle
rule, V-ALL, NEVER list).

## STOP preconditions (verify by re-reading; if any is false, STOP per the Failure protocol)

- `tests/tool_schema_fidelity.rs` still asserts `names.len() == 17` and `names[16] == "explain"`.
- `tests/all_open_golden.rs` still declares `GOLDEN_TOOL_NAMES: [&str; 17]` ending in `"explain"`.
- `crates/core/src/browser/directory.rs` `REGISTRY` still ends with the `form_fill` row then the
  `explain` row; the `EXPECTED` table (test `registry_requires_match_the_adr_table`) still ends with
  `("explain", None, &[])`; `total_variants` is asserted `== 30`.
- `extension/content.js` still defines `function deref(ref)` and `function innerInput(el)`, and its
  `chrome.runtime.onMessage` switch still has a `case "setFormValue":`.
- `extension/service-worker.js` still has a `handlers` object with an `async form_input(a)` entry and
  helpers `effectiveTabId`, `content`, `withObservation`, `hopError`, `text`.

If the current advertised tool count is not 17, a prior task already ran or the tree drifted -- STOP.

## Standing orders

- RE-READ every file before editing; the line numbers here are as-of-authoring and may have moved.
  Anchor edits on the quoted code, not the line numbers.
- Insert `file_upload` in REGISTRY and in every ordered pin list IMMEDIATELY BEFORE `explain`
  (so it becomes index 16; `explain` moves to 17). `explain` stays LAST everywhere.
- ASCII only; the description's em-dashes are already rendered `--` below -- copy verbatim.

## Part A -- Binary: the REGISTRY row (`crates/core/src/browser/directory.rs`)

Insert this `ToolDescriptor`, VERBATIM, immediately before the `explain` row in `const REGISTRY`
(i.e., after the `form_fill` row's closing `},`):

    ToolDescriptor {
        tool: "file_upload",
        advertised_description: "Upload one or multiple files to a file input element on the page. Do not click on file upload buttons or file inputs -- clicking opens a native file picker dialog that you cannot see or interact with. Instead, use read_page or find to locate the file input element, then use this tool with its ref to upload files directly.",
        input_schema: || json!({
            "type": "object",
            "properties": {
                "files": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "data": { "type": "string" },
                            "name": { "type": "string" },
                            "mimeType": { "type": "string" }
                        },
                        "required": ["data", "name"]
                    },
                    "description": "Files to upload, as base64-encoded bytes."
                },
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "DEPRECATED. Use `files` instead."
                },
                "ref": {
                    "type": "string",
                    "description": "Element reference ID of the file input from read_page or find tools (e.g., \"ref_1\", \"ref_2\")."
                },
                "tabId": {
                    "type": "number",
                    "description": "Tab ID where the file input is located. Use tabs_context first if you don't have a valid tab ID."
                }
            },
            "required": ["ref", "tabId"],
            "additionalProperties": false
        }),
        example: Some(ToolExample {
            call: r#"{"ref":"ref_1","tabId":0,"files":[{"data":"aGVsbG8=","name":"hello.txt"}]}"#,
            returns: Some("Uploads the base64-decoded file(s) to the file input at ref; returns a text confirmation with the file names and total size."),
        }),
        action_key: None,
        variants: &[ActionVariant {
            action: None,
            requires: &[Capability::Write],
            directory_description:
                "Upload files (base64 bytes) to a file input located by read_page or find, via its ref.",
        }],
        resource: ResourceShape::TabScoped,
        handler: Handler::ExtensionForward,
        postprocess: None,
        post_dispatch: PostDispatch::None,
        output_schema: None,
    },

Rationale (do not deviate): `[Capability::Write]` -- the ref was located by a separately-governed
read_page/find, so this call only writes bytes into a destination; `TabScoped` + `ExtensionForward`
so the tool name and args ride the existing `tool_request` envelope (no new Rust arg struct, no new
wire type); `output_schema: None` -- it returns a text confirmation like `form_input`, so it is NOT
added to the output-schema list in Part B.

Also update the two non-load-bearing count comments in this file for accuracy: the `REGISTRY` doc
comment ("17 descriptors ...") and the `descriptor()` doc comment ("Linear scan over 17 rows") become
18. Do not change any other prose.

## Part B -- Fidelity and golden pins (sacred surface: APPEND/BUMP only)

RE-READ each assertion, then make exactly these changes (before -> after):

1. `crates/core/src/browser/directory.rs`, test `registry_requires_match_the_adr_table`, `EXPECTED`
   table: insert this row immediately before the `("explain", None, &[])` line:

        ("file_upload", None, &[Capability::Write]),

2. Same file, test `every_variant_is_unique_and_the_total_is_pinned` (the `total_variants` assert):
   `assert_eq!(total_variants, 30);` -> `assert_eq!(total_variants, 31);`

3. Same file, test `per_tool_fields_match_the_adr_table`, `EXPECTED_TOOLS` table: insert this row
   immediately before the `("explain", ...)` row:

        (
            "file_upload",
            None,
            ResourceShape::TabScoped,
            false,
            false,
            PostDispatch::None,
        ),

   (`false, false` = not a Local handler, no postprocess. `REGISTRY.len() == EXPECTED_TOOLS.len()` is
   asserted automatically.)

4. `tests/tool_schema_fidelity.rs`, test
   `advertises_exactly_the_thirteen_trained_tools_plus_explain_positioned_last`:
   - `names.len()` assert `17` -> `18`; its message
     `"13 trained tools plus wait_for, script, form_fill, and explain"` ->
     `"13 trained tools plus wait_for, script, form_fill, file_upload, and explain"`.
   - Change `assert_eq!(names[16], "explain", "explain stays positioned last");` to TWO asserts:

         assert_eq!(names[16], "file_upload", "the 17th tool is file_upload, immediately before explain");
         assert_eq!(names[17], "explain", "explain stays positioned last");

5. `tests/tool_schema_fidelity.rs`, test
   `explain_tool_object_matches_the_pinned_adr_0022_decision_7_shape`: the `all.len()` assert `17` ->
   `18`; its message add `file_upload` the same way as step 4's message.

6. `tests/all_open_golden.rs`: `GOLDEN_TOOL_NAMES: [&str; 17]` -> `[&str; 18]`; insert `"file_upload",`
   immediately before `"explain",`. Update the `tools.len()` assert message `"all 17 tools ..."` ->
   `"all 18 tools ..."` adding `file_upload`. The doc comment "The 17 tool names ..." -> "18".

7. `tests/mcp_protocol.rs`: find the tool-count assertion (as-of-authoring near line 191, value 17,
   message mentioning "13 trained tools plus wait_for, script, form_fill, and the explain addition").
   Bump `17` -> `18` and add `file_upload` to the message.

8. `crates/core/src/hub/outbound/mod.rs` (these two asserts also derive from `REGISTRY`): bump
   `assert_eq!(cap.directory().len(), 17);` -> `18` (test `browser_capability_exposes_the_full_directory`)
   AND `assert_eq!(reg.aggregated_directory().len(), 17);` -> `18` (test
   `registry_aggregates_the_browser_directory`); update the prose comment "17-declaration REGISTRY" ->
   "18-declaration REGISTRY".
9. `tests/tool_enforcement.rs`, test `all_open_invariant_no_manifest_means_no_denials`: bump
   `assert_eq!(tools.len(), 17, "...")` -> `18` and add `file_upload` to its message; update the
   "17 tools" doc comment above the test to 18.
10. LEAVE UNCHANGED: `tests/tool_schema_fidelity.rs` test
   `output_schemas_present_exactly_where_declared` -- `file_upload` has `output_schema: None`, so it
   does NOT join that list. Confirm by re-reading that the list does not gain a `file_upload` entry.
   Also LEAVE `EXPECTED_TRAINED` untouched (NEVER list).

## Part C -- Extension (`extension/`)

C1. New pure helper `extension/lib/fileset.js`. Mirror the dual-export idiom of an existing lib module
(re-read `extension/lib/observation.js` for the exact `module.exports` / global-assignment tail this
repo uses). Export one pure function:

    // decodeFiles(files): validate and base64-decode an array of {data, name, mimeType?}.
    // Returns { ok: true, decoded: [{ name, type, bytes: Uint8Array }], totalBytes }
    //      or { ok: false, error: "<message>" }.
    // Rules: each item's `data` and `name` must be non-empty strings; `type` defaults to
    // "application/octet-stream" when mimeType is absent/empty; bytes = Uint8Array from atob(data).
    // On a bad item return { ok:false, error:"each file must have `data` and `name`" }.

C2. `extension/content.js`: add `function setFiles(ref, files)` next to `setFormValue`. It must:
    - `const el = deref(ref)`; on miss return `{ error: staleRefMessage(ref) || "Element " + ref +
      " not found or was garbage-collected." }` (mirror `setFormValue`'s miss branch).
    - `const target = innerInput(el) || el;` require `target.tagName === "INPUT"` and
      `target.type === "file"`, else return
      `{ error: "Element is not a file input. Found: <" + target.tagName.toLowerCase() + ...>" }`
      (mirror the official message shape).
    - `const r = decodeFiles(files);` if `!r.ok` return `{ error: r.error }`.
    - Build a `DataTransfer`; for each `r.decoded` item add `new File([item.bytes], item.name,
      { type: item.type, lastModified: Date.now() })`; set `target.files = dt.files`; `target.focus()`;
      dispatch `new Event("input", { bubbles: true, composed: true })` then the same `"change"`.
    - Return `{ success: true, output: "Uploaded " + r.decoded.length + " file(s) to file input: " +
      r.decoded.map(f => f.name).join(", ") + " (" + Math.round(r.totalBytes/1024) + " KB total)" }`.
    Add a switch case in the `onMessage` listener next to `case "setFormValue":`:

        case "setFiles": sendResponse({ result: setFiles(msg.ref, msg.files) }); return true;

    `decodeFiles` must be in scope: add `lib/fileset.js` to the content-script `js` array in
    `extension/manifest.json` (next to `lib/settle.js`/`lib/observation.js`/`lib/treediff.js`) AND to
    the injection file list in `extension/service-worker.js`'s `content()` fallback
    (`chrome.scripting.executeScript({ ..., files: [...] })`) -- KEEP THE TWO LISTS IN SYNC.

C3. `extension/service-worker.js`: add to the `handlers` object, mirroring `async form_input(a)`:

        async file_upload(a) {
          const tabId = await effectiveTabId(a.tabId);
          if (!a.files || a.files.length === 0) {
            if (a.paths && a.paths.length > 0) {
              throw hopError("binary", "file_upload no longer accepts host filesystem paths. The MCP controller must read the file and pass its contents via the `files` parameter.");
            }
            throw hopError("binary", "files parameter is required and must be a non-empty array");
          }
          return withObservation(tabId, async () => {
            const r = await content(tabId, { type: "setFiles", ref: a.ref, files: a.files });
            if (r && r.result && r.result.error) {
              const msg = r.result.error.endsWith(".") ? r.result.error.slice(0, -1) : r.result.error;
              throw hopError("page", msg);
            }
            return text(r.result.output);
          });
        },

    (Re-read `form_input` to match the exact `hopError`/`text`/`withObservation` shapes; adapt if they
    differ from the snippet above -- the snippet is the intent, the live helpers are authoritative.)

## Part D -- Tests (add by name; assertions pinned)

D1. Rust: the Part B fidelity/pin edits ARE the registry tests; they must pass unchanged otherwise.
    Add ONE governance test to `tests/all_open_golden.rs` (or the nearest existing governance test
    file if that fits better -- re-read to place it): a test named
    `file_upload_is_all_open_allowed_and_classifies_write` that asserts
    `Governance::all_open(...).decide("file_upload", None, &[], GoverningResource::None, ...)` is
    `Decision::Allow` (mirror `facade_decide_is_all_open_after_the_move`'s call shape verbatim) and
    that `directory::requires("file_upload", None) == Some(&[Capability::Write][..])`. (Add whatever
    `use` imports the new test needs -- e.g. `requires`, `Capability`, `Decision` -- via the crate
    paths the file already uses; `all_open_golden.rs` currently imports only `descriptor`.)

D2. JS: new `tests/extension/fileset.test.js` (mirror `tests/extension/observation.test.js`'s harness
    and import style). Pinned assertions:
    - `decodeFiles([{ data: "aGVsbG8=", name: "hello.txt" }])` -> `ok === true`, `decoded.length === 1`,
      `decoded[0].name === "hello.txt"`, `decoded[0].type === "application/octet-stream"`,
      `totalBytes === 5`, and `String.fromCharCode(...decoded[0].bytes) === "hello"`.
    - `decodeFiles([{ data: "aGVsbG8=", name: "a.txt" }, { data: "d29ybGQ=", name: "b.txt" }])` ->
      `decoded.length === 2`, `totalBytes === 10`.
    - `decodeFiles([{ data: "eA==" }])` (no name) -> `ok === false`,
      `error === "each file must have \`data\` and \`name\`"`.
    - `decodeFiles([{ data: "aGVsbG8=", name: "c.png", mimeType: "image/png" }])` ->
      `decoded[0].type === "image/png"`.
    ("aGVsbG8=" is base64 "hello" (5 bytes); "d29ybGQ=" is "world" (5 bytes); "eA==" is "x".)
    Add this file to the `node --test ...` line in `.github/workflows/ci.yml` (the `extension-unit`
    job) and in BOOTSTRAP.md's V-ALL line.

## Part E -- Verify (V-ALL, all must pass)

Run the BOOTSTRAP V-ALL block, now including `tests/extension/fileset.test.js` on the `node --test`
line. Additionally sanity-check the new tool advertises: `cargo test --locked -p ghostlight --test
tool_schema_fidelity` and `--test all_open_golden` are green.

## Out of scope (do NOT do in T1)

- No `browser_batch`/`upload_image`/`gif_creator` work (later tasks).
- No changes to any trained tool, to `script`/`form_fill`/`wait_for`/`explain`, or to `EXPECTED_TRAINED`.
- No binary-side FileUpload arg struct, no new native-messaging message `type` (ExtensionForward
  reuses `tool_request`).
- Do NOT implement host-path reading; `paths` stays advertised-but-rejected per the handler above.

## Commit

One commit: `feat(tools): file_upload -- base64 bytes to a located file input (ADR-0050 D2)`.
Then update the LEDGER T1 entry (status done, commit hash, V-ALL pass, deviations) -- either in the
same commit or a `docs(rebaseline): ledger T1` commit.
