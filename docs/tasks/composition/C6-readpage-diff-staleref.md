# C6: read_page diff mode + stale-ref corrective errors

Goal: `read_page(diff: true)` returns only changes; deref misses name the re-render.
Normative: ADR-0037 D3/D4, PINS SS11, SS15. SKIP allowed.

## Tree facts (as of authoring; re-read before editing)

- content.js renders the accessibility tree as lines (the `unit = line + "\n"` builder around
  :186); refs minted via refFor with a module-level refSeq (:25); deref failures surface
  through setFormValue/refCoordinates/scrollToRef error strings.
- C4's 500ms window counter exists in content.js.
- `src/browser/directory.rs` read_page row: its inputSchema properties map is the ONLY
  sanctioned trained-schema edit point in this batch (BOOTSTRAP NEVER list).

## STOP preconditions

- STOP if C4 is not committed. STOP if read_page's inputSchema in directory.rs does not have a
  plain `"properties"` object to extend.

## Required behavior

1. `extension/lib/treediff.js`: pure `diffLines(oldLines, newLines)` per PINS SS11 (ref-token
   keying, changed/removed/added, render order and prefixes).
2. content.js: keep, per content-script instance, the last rendered tree's lines and a render
   serial (increment per 500ms window with >= 3 mutations); refs remember the serial they were
   minted at. read_page handling: when the message carries `diff: true` and a baseline exists,
   respond with the rendered diff; no baseline -> full tree with first line
   `(no baseline; full tree)`.
3. Deref-miss errors in setFormValue/refCoordinates/scrollToRef become PINS SS11's exact
   corrective string (only when the miss is a stale ref, i.e. the serial moved; a never-minted
   ref keeps today's message).
4. directory.rs: add ONLY the `diff` property per SS11's exact JSON to read_page's properties.
   No other schema byte changes.
5. SW read_page handler forwards `diff` through to the content message.
6. manifest.json + ci.yml per PINS SS15 after-C6 values.

## Tests (by name; assertions verbatim)

- `tests/extension/treediff.test.js`: PINS SS11's oracle: old
  `["ref_1 button \"A\"","ref_2 link \"B\""]`, new `["ref_1 button \"A2\"","ref_3 link \"C\""]`
  -> changed `["ref_1 button \"A2\""]`, removed `["ref_2 link \"B\""]`, added
  `["ref_3 link \"C\""]`; plus: identical inputs -> all three empty; keyless lines compare by
  whole-line identity.
- `tests/tool_schema_fidelity.rs`: extend read_page's expected property-name set with `diff`
  ONLY if a test pins that set (log as deviation either way).

## Verification

Gates; node line per SS15.

## Out of scope

get_page_text, structured read_page vocab, content-addressed refs, cross-session baselines.

Commit: `feat(tools): read_page diff mode + stale-ref render-serial errors (ADR-0037)`
