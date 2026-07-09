# T4 -- gif_creator (phased: record + encode + export)

Goal: add the additive trained tool `gif_creator` per ADR-0050 Decision 5. It records browser actions
as frames and exports an animated GIF. Read ADR-0050 Decision 5 now; it is normative.

Runs AFTER T3 (tool count 20 at start). This is the LARGEST task and is PHASED. Phase 1 is the
landable floor; Phase 2 is additive on top. Each phase leaves a green tree and its own LEDGER entry.
Read BOOTSTRAP.md.

## Phasing

- Phase 1 (REQUIRED floor): `action` in {start_recording, stop_recording, clear} + `export` with
  `download: true`. Frame capture + GIF encode + a returned/downloaded GIF. Advertise the full tool
  (all four actions) but `export` without `download:true` and without `coordinate` returns a
  "not yet supported: provide download:true, or a coordinate (Phase 2)" text -- so the schema is the
  trained shape from day one and Phase 2 fills the drag-drop path.
- Phase 2: `export` with `coordinate` -- drag-drop the encoded GIF onto a page element, reusing T3's
  `setImage` DragEvent mechanism with the GIF File.

## STOP preconditions (re-read; if false, STOP)

- Tool count is 20 with `names[19] == "explain"` (T1-T3 landed). Else STOP.
- `directory.rs` `#[cfg(test)]` still asserts `with_action_key.len() == 2` (only `computer`, `form_fill`
  carry an `action_key` today) and `total_variants == 33` (post-T3). If these differ, re-derive the
  post-T3 numbers from the live tree before applying the deltas below.
- The extension can capture a tab screenshot on demand (re-read `service-worker.js` for the existing
  screenshot capture used by `computer`; gif_creator reuses it for frames).

## Part A -- Re-harvest the exact gif_creator schema (the `options` object was truncated)

The captured description + params are in `scratchpad/harvest/HARVEST-1.0.80.md` section 4 EXCEPT the
`options` object (truncated). Before writing the schema, RE-EXTRACT `gif_creator`'s full parameters
from the installed official extension (recipe in `docs/research/12-official-extension-parity.md`
"Re-extracting"; the tool is in `assets/mcpPermissions-*.js`, search `name:"gif_creator"`). Pin the
`options` sub-fields (watermark/labels/etc.) into the schema VERBATIM (ASCII, `--` for em-dashes). If
re-extraction is not possible, declare `options` as an optional `{"type":"object"}` and note the gap
in the LEDGER (fidelity is a regression snapshot, so a leaner `options` is acceptable but flag it).

Description + the four `action` values, `tabId`, `coordinate`, `download`, `filename` are pinned in the
harvest note; transcribe them.

## Part B -- The REGISTRY row (per-action, like `computer`)

`gif_creator` carries `action_key: Some("action")` and FOUR `ActionVariant`s (this is the SECOND tool
with an action_key, after `computer`). Insert the row before `explain`. Capability per action (PINNED):

    variants: &[
        ActionVariant { action: Some("start_recording"), requires: &[Capability::Read],
            directory_description: "Start recording browser actions in the tab's group as GIF frames." },
        ActionVariant { action: Some("stop_recording"), requires: &[],
            directory_description: "Stop recording; keep the captured frames for export." },
        ActionVariant { action: Some("clear"), requires: &[],
            directory_description: "Discard the captured recording frames." },
        ActionVariant { action: Some("export"), requires: &[Capability::Write],
            directory_description: "Encode the frames to a GIF and export it (download, or drag-drop at a coordinate)." },
    ],

    resource: ResourceShape::TabScoped,
    handler: Handler::ExtensionForward,   // recording state + frames + encoder live in the extension
    postprocess: None,
    post_dispatch: PostDispatch::None,
    output_schema: None,

`export` is classified `[Write]` (fail-closed: its drag-drop mode writes to the page; a download-only
export over-classifies as Write, which is the safe direction -- the variant system keys on `action`,
not the `download` flag). Provide an `example` whose `call` validates against the schema, e.g.
`{"action":"start_recording","tabId":0}`.

## Part C -- Extension: recording + encoding + export (`extension/`)

C1. `service-worker.js`: a per-tab-group recording buffer (frames = captured screenshots, bounded --
    PIN a max frame count, e.g. 100, and drop-oldest beyond it). A `gif_creator` handler in `handlers`
    dispatching on `a.action`:
    - start_recording: init/clear the buffer for the group; capture the first frame now.
    - stop_recording: stop capturing (keep frames).
    - clear: drop the buffer.
    - export: encode frames -> GIF; if `download:true` trigger a download (chrome.downloads or a
      data-URL) and return a text confirmation; if `coordinate` present (Phase 2) drag-drop the GIF
      File at the coordinate via a content-script `setImage`-style path; else the Phase-1 "provide
      download:true or a coordinate" text.
    Frame capture during actions: capture a frame after each mutating tool while a recording is active
    (hook the existing post-action point; re-read how `computer` actions complete). Keep it simple in
    Phase 1: capture on start + on each `computer`/`navigate` while recording.

C2. GIF encoder: a SELF-CONTAINED, ASCII, pure-JS encoder vendored at `extension/lib/gifenc.js`
    (MV3 forbids remote code, so it MUST ship in the package). Port a permissively-licensed minimal
    encoder (e.g. an omggif/gif.js-style LZW GIF89a encoder) into ASCII JS with a header credit +
    SPDX; do NOT fetch it at runtime. Expose `encodeGif(frames, {width,height,delayMs}) -> Uint8Array`.
    Add `lib/gifenc.js` to the manifest content-script list AND the `content()` injection list if the
    encoder runs in the content script; if it runs in the service worker, import it there. Overlays
    (click indicators, labels, watermark, progress bar) are Phase-1-optional -- a plain frame GIF is an
    acceptable floor; note deferred overlays in the LEDGER.

## Part D -- Fidelity / golden pins (post-T3; count 20 -> 21; +4 variants; +1 action_key)

- `tool_schema_fidelity.rs`: count `20` -> `21`; add `names[19] == "gif_creator"`, move explain to
  `names[20]`; update messages.
- `all_open_golden.rs`: `[&str; 20]` -> `[&str; 21]`, insert `"gif_creator"` before `"explain"`; bump
  count message + doc comment.
- `mcp_protocol.rs`: count `20` -> `21`, add `gif_creator` to the message.
- `crates/core/src/hub/outbound/mod.rs`: bump BOTH `cap.directory().len()` and
  `reg.aggregated_directory().len()` asserts `20` -> `21`, and the REGISTRY-count prose comment.
- `tests/tool_enforcement.rs` (`all_open_invariant_no_manifest_means_no_denials`): `tools.len()`
  assert `20` -> `21`, add `gif_creator` to its message and the count doc comment.
- `directory.rs` `EXPECTED` requires table: insert the FOUR gif_creator rows before `explain`:
    ("gif_creator", Some("start_recording"), &[Capability::Read]),
    ("gif_creator", Some("stop_recording"), &[]),
    ("gif_creator", Some("clear"), &[]),
    ("gif_creator", Some("export"), &[Capability::Write]),
- `directory.rs` `total_variants`: `33` -> `37`.
- `directory.rs` `with_action_key` count: `2` -> `3` (find the assert; as-of-authoring near the
  `with_action_key.len() == 2` check).
- `directory.rs` `EXPECTED_TOOLS`: insert before explain:
    ("gif_creator", Some("action"), ResourceShape::TabScoped, false, false, PostDispatch::None),
- `directory.rs` doc-comment counts -> 21.

## Part E -- Tests (pinned assertions)

- `tests/extension/gifenc.test.js` (node --test; add to the ci.yml + BOOTSTRAP node --test lines):
  `encodeGif([<one 2x2 frame of known RGBA>], {width:2,height:2,delayMs:100})` returns a Uint8Array
  that STARTS WITH the bytes of `GIF89a` (0x47,0x49,0x46,0x38,0x39,0x61) and is non-trivially long.
  (This pins the encoder produces a valid GIF89a header -- a real, checkable oracle.)
- A recording-buffer unit test (JS or via the extension harness): start -> buffer has 1 frame; two
  captures -> 3; clear -> 0; bound at the max drops oldest.
- Rust: the fidelity/golden/mcp_protocol asserts from Part D (the registry regression is the pin).
- `directory.rs` `registry_requires_match_the_adr_table` passes with the 4 new rows.

## Out of scope / deferrals

- Phase-2 drag-drop export and rich overlays may defer (note in LEDGER). No change to any trained
  tool. Do not fetch any encoder at runtime.

## Commit

Phase 1: `feat(tools): gif_creator phase 1 -- record + encode + download-export (ADR-0050 D5)`.
Phase 2 (if done): `feat(tools): gif_creator phase 2 -- drag-drop GIF export`.
Update the LEDGER T4 entry per phase (note deferrals/deviations).
