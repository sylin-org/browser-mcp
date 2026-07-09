# T3 -- upload_image (screenshot-cache imageId + drag-drop)

Goal: add the additive trained tool `upload_image` per ADR-0050 Decision 4. It uploads a PREVIOUSLY
CAPTURED screenshot (resolved from a per-session cache by `imageId`) to a file input (`ref`) or a
drag-drop target (`coordinate`). Read ADR-0050 Decision 4 now; it is normative.

Runs AFTER T2 (tool count 19 at start). This task is SUBSYSTEM-SCALE and the highest-risk in the
batch. It MAY be split (3a = the cache + imageId injection; 3b = the upload_image tool) -- if you
split, each half must leave a green tree and its own LEDGER entry. Read BOOTSTRAP.md.

## What is PINNED (transcribe) vs DESIGNED (implement against this spec)

PINNED: the tool schema, the fidelity/golden deltas, the capability class, the imageId format, and the
named tests' assertions. DESIGNED-by-you: the cache data structure internals and the exact injection
site (with the STOP guard below).

## STOP preconditions (re-read; if false, STOP)

- Tool count is 19 at T3 start (T1+T2 landed: file_upload, browser_batch). If it is not 19 with
  `names[18] == "explain"`, a prior task is missing -- STOP.
- `crates/core/src/hub/outbound/browser.rs` still defines `pub struct Browser { ... }` with
  `Arc<Mutex<...>>` fields and an `pub async fn call(&self, guid: &str, tool: &str, args: &Value)`.
- The `computer` screenshot result still returns an MCP `image` content block (re-read how a
  screenshot result is shaped; if screenshots are NOT surfaced as an `image` content block on the
  `computer` result that `Browser::call` returns, STOP and record the actual shape -- the injection
  site below depends on it).

## Part A -- The per-session screenshot cache (DESIGNED; home is PINNED to Browser)

A1. Add a per-guid, BOUNDED screenshot cache to `Browser` (`hub/outbound/browser.rs`): a new field, e.g.
    `screenshot_cache: Arc<Mutex<HashMap<String /*guid*/, VecDeque<(String /*imageId*/, CachedImage)>>>>`
    where `CachedImage { base64: String, media_type: String }`. Bound each guid's deque to the last N
    entries (PIN: N = 8) -- pushing a 9th evicts the oldest. Initialize the field in `Browser::new`.

A2. imageId format (PINNED): `"img_" + <a uuid::Uuid::new_v4() simple hyphenless string>` -- e.g.
    `img_9f1c...`. (uuid is already a dependency; see script.rs.) Do NOT use Date/random-free-of-uuid.

A3. INJECTION SITE: in `Browser::call`, AFTER a successful response whose `tool == "computer"` and
    whose result contains an `image` content block, mint an imageId, store `{base64, media_type}` from
    that image block into the cache under `guid`, and INJECT the id additively into the returned result
    so the model can reference it: append a content block
    `{"type":"text","text":"[imageId: <img_...>] Reference this id with upload_image to place this
    screenshot into a file input or drag-drop target."}`.
    This is the ONE additive change to a trained tool's OUTPUT sanctioned by ADR-0050 (the `computer`
    INPUT schema and its descriptor row are UNTOUCHED). RE-READ for any test asserting the exact
    `computer` screenshot result content shape; if one exists, update it to expect the extra trailing
    text block (and note it in the LEDGER); if none exists, add a Browser-level test (Part D).

## Part B -- The upload_image tool (Local handler)

B1. New module `crates/core/src/mcp/upload_image.rs`, declared `pub mod upload_image;` in
    `crates/core/src/mcp/mod.rs`. Handler:

        pub(crate) fn upload_image_handler(ctx: LocalCtx<'_>) -> LocalFuture<'_>

    It:
    1. Reads `imageId` (required), exactly one of `ref` / `coordinate` (error if both or neither with
       the messages below), `tabId` (required), `filename` (default "image.png").
       - both ref+coordinate -> Failure "Provide either ref or coordinate, not both."
       - neither -> Failure "Either ref or coordinate parameter is required."
    2. Resolves `imageId` against `ctx.browser`'s cache for `ctx.guid`. Miss -> Failure
       "Image not found with ID: <imageId>. Capture it first with the computer screenshot action."
    3. Forwards to the extension via `ctx.browser.call(ctx.guid, "upload_image_exec", &args2)` where
       `args2 = { tabId, ref?, coordinate?, filename, data: <base64>, mimeType: <media_type> }`.
       (`upload_image_exec` is a new EXTENSION-side command name, not an advertised tool -- same
       pattern `form_fill` uses with its internal `form_structure_internal` call. Re-read form_fill.rs
       to match the internal-call idiom and error unwrapping.)
    4. Returns a text confirmation from the extension's `output`.

B2. Governance for `upload_image` (the advertised tool): ResourceShape `TabScoped`,
    `requires: &[Capability::Write]`, Handler `Local(crate::mcp::upload_image::upload_image_handler)`,
    output_schema `None`.

## Part C -- Extension: the upload_image_exec command (`extension/`)

C1. `extension/content.js`: add `function setImage(ref, coordinate, dataUrlOrB64, filename, mimeType)`
    that, given base64 bytes: builds a `File` from `atob` (reuse the byte-decode approach from
    `lib/fileset.js`'s `decodeFiles`, or a one-file variant) and then:
    - if `ref`: resolve `deref(ref)` -> `innerInput` -> require `<input type=file>` -> DataTransfer
      assign + dispatch input/change (SAME as `setFiles`).
    - if `coordinate` [x,y]: `document.elementFromPoint(x,y)` (handle IFRAME per the official technique
      in scratchpad/harvest/HARVEST-1.0.80.md section 2); dispatch DragEvent `dragenter`/`dragover`/
      `drop` carrying the DataTransfer at (x,y).
    Return `{ success, output }` or `{ error }`, mirroring `setFiles`.
    Add `case "setImage": sendResponse({ result: setImage(msg.ref, msg.coordinate, msg.data, msg.filename, msg.mimeType) }); return true;`
    to the onMessage switch.

C2. `extension/service-worker.js` `handlers`: add `async upload_image_exec(a)` mirroring `file_upload`
    (effectiveTabId, withObservation, `content(tabId, {type:"setImage", ...})`, error unwrap, text).
    (`upload_image_exec` is dispatched by the binary's upload_image_handler, not advertised.)

## Part D -- The advertised schema (`crates/core/src/browser/directory.rs`, insert before explain)

    ToolDescriptor {
        tool: "upload_image",
        advertised_description: "Upload a previously captured screenshot to a file input or drag & drop target. Supports two approaches: (1) ref -- for targeting specific elements, especially hidden file inputs, (2) coordinate -- for drag & drop to visible locations like Google Docs. Provide either ref or coordinate, not both.",
        input_schema: || json!({
            "type": "object",
            "properties": {
                "imageId": { "type": "string", "description": "ID of a previously captured screenshot (from the computer tool's screenshot action), e.g. \"img_...\" as reported in the screenshot result." },
                "ref": { "type": "string", "description": "Element reference ID from read_page or find tools (e.g., \"ref_1\", \"ref_2\"). Use this for file inputs (especially hidden ones). Provide either ref or coordinate, not both." },
                "coordinate": { "type": "array", "description": "Viewport coordinates [x, y] for drag & drop to a visible location like Google Docs. Provide either ref or coordinate, not both." },
                "tabId": { "type": "number", "description": "Tab ID where the target element is located. This is where the image will be uploaded to." },
                "filename": { "type": "string", "description": "Optional filename for the uploaded file (default: \"image.png\")." }
            },
            "required": ["imageId", "tabId"],
            "additionalProperties": false
        }),
        example: Some(ToolExample {
            call: r#"{"imageId":"img_example","ref":"ref_1","tabId":0}"#,
            returns: Some("Uploads the cached screenshot to the file input at ref (or drag-drops it at coordinate); returns a text confirmation."),
        }),
        action_key: None,
        variants: &[ActionVariant {
            action: None,
            requires: &[Capability::Write],
            directory_description:
                "Upload a previously captured screenshot to a file input (ref) or drag-drop target (coordinate).",
        }],
        resource: ResourceShape::TabScoped,
        handler: Handler::Local(crate::mcp::upload_image::upload_image_handler),
        postprocess: None,
        post_dispatch: PostDispatch::None,
        output_schema: None,
    },

NOTE the description drops the official's "or user-uploaded image" (ADR-0050 D4 trim: no side panel).

## Part E -- Fidelity / golden pins (post-T2; count 19 -> 20)

- `EXPECTED` requires table: insert `("upload_image", None, &[Capability::Write]),` before `explain`.
- `total_variants`: `32` -> `33`.
- `EXPECTED_TOOLS`: insert `("upload_image", None, ResourceShape::TabScoped, true, false, PostDispatch::None),`
  before `explain` (`true` = Local handler).
- `tool_schema_fidelity.rs`: count `19` -> `20`; add `names[18] == "upload_image"`, move explain to
  `names[19]`; update messages to add `upload_image`.
- `all_open_golden.rs`: `[&str; 19]` -> `[&str; 20]`, insert `"upload_image"` before `"explain"`;
  bump count message + doc comment.
- `mcp_protocol.rs`: count `19` -> `20`, add `upload_image` to the message.
- `crates/core/src/hub/outbound/mod.rs`: bump BOTH `cap.directory().len()` and
  `reg.aggregated_directory().len()` asserts `19` -> `20`, and the REGISTRY-count prose comment.
- `tests/tool_enforcement.rs` (`all_open_invariant_no_manifest_means_no_denials`): `tools.len()`
  assert `19` -> `20`, add `upload_image` to its message and the count doc comment.
- `directory.rs` doc-comment counts -> 20. `computer`'s EXPECTED_TOOLS row STAYS (`postprocess:None`
  unchanged -- the injection is in Browser::call, not the descriptor).

## Part F -- Tests (pinned assertions)

- `browser.rs` unit test `screenshot_cache_round_trips_and_injects_imageId`: drive `Browser::call`
  (or a direct helper if `call` needs a live stream -- re-read; if `call` is not unit-testable in
  isolation, add a `pub(crate)` helper `cache_screenshot(guid, base64, media_type) -> String` and test
  THAT plus a `resolve_cached_image(guid, id)` getter): assert caching an image returns an `img_`-
  prefixed id, `resolve` returns the same bytes, an unknown id returns None, and the 9th insert evicts
  the 1st (bound N=8).
- `upload_image.rs` unit test `upload_image_rejects_ref_and_coordinate_together` and
  `_requires_one_of_ref_or_coordinate` (validate the arg-guard messages) using a stub browser/ctx if
  one exists (re-read how form_fill.rs unit-tests its handler; if the handler is not unit-testable
  without a live Browser, assert the arg-guard via a small extracted pure `validate_target(args)`
  function instead and test that).
- `tests/extension/*`: a `decodeFiles`-style test already covers byte decode (T1). If `setImage`'s
  decode is a distinct pure helper, add one pinned base64 round-trip assertion for it.
- Fidelity/golden asserts from Part E.

## Out of scope

- No "user-uploaded image" source (no side panel). No gif_creator. No change to `computer`'s INPUT
  schema or its descriptor row.

## Commit

One commit (or two if split 3a/3b): `feat(tools): upload_image -- cached-screenshot upload to file input or drag-drop (ADR-0050 D4)`.
Update the LEDGER T3 entry (note any 3a/3b split and any deviation).
