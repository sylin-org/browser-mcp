# C3: structured results + outputSchema

Goal: the extension preserves structure (`structuredContent`) for the v1 vocabulary tools; the
binary declares `outputSchema`. Text stays byte-identical. Normative: ADR-0038 D1-D4, PINS SS5.

## Tree facts (as of authoring; re-read before editing)

- `extension/service-worker.js:1149` `find` flattens `{results, more}` to prose via `text(out)`;
  `:632` `tabContext` returns `text(JSON.stringify({...}))`; `:1108` `tabs_create_mcp` prefixes
  `Created tab N.\n`; `:1117` `navigate`. Helper `text(...)` builds `{content:[{type,text}]}`.
- `src/browser/directory.rs:100` `ToolDescriptor` (no output_schema field yet).
- `src/transport/mcp/tools.rs` renders `advertised_tools_json()` from REGISTRY.
- Binary dispatch passes the extension result Value through opaque (pipeline Ok(mut result)),
  so `structuredContent` set by the SW flows to the client with NO binary dispatch change.
- `tests/tool_schema_fidelity.rs` asserts names/order/descriptions; `tests/all_open_golden.rs`
  pins the advertised name array.

## STOP preconditions

- STOP if `tool_schema_fidelity.rs` or `all_open_golden.rs` byte-compares entire per-tool JSON
  objects such that ADDING an `outputSchema` key cannot be accommodated by extending an
  expected-keys list; report what they pin instead.
- STOP if the SW's `find`/`tabContext`/`navigate` handlers do not match the line references
  above after re-reading.

## Required behavior

1. SW: for find, tabs_context_mcp, tabs_create_mcp, navigate -- build one source object, render
   the EXISTING text byte-identically, and set `result.structuredContent` to PINS SS5 / ADR-0038
   D2's shapes. navigate samples `chrome.tabs.get(tabId)` after navigation for
   `{tabId, url, title}`. tabs_create_mcp's structured is `{tabId, tabs:[...]}` with the
   created id.
2. Binary: `ToolDescriptor` gains `pub output_schema: Option<fn() -> Value>`; rows for the four
   tools carry schemas matching the shapes (write them as inline `json!` JSON-Schema; keep them
   minimal: type/properties/required). All other rows None. `advertised_tools_json` emits
   `"outputSchema"` when Some.
3. Do NOT touch inputSchemas, descriptions, name order, or any text rendering.

## Tests (by name; assertions verbatim)

- `tests/tool_schema_fidelity.rs::output_schemas_present_exactly_where_declared`: the tools
  carrying `outputSchema` are exactly `["tabs_context_mcp","tabs_create_mcp","navigate","find"]`
  (advertised order) and each is a JSON object with `"type":"object"`.
- Existing fidelity + golden tests pass with at most expected-keys-list extensions (log each
  edit as a deviation).
- Extension: no new node tests (chrome.* untestable there); verification is the byte-identical
  text rule -- diff the SW changes by eye and state in the LEDGER entry that no text literal
  changed.

## Verification

Gates per BOOTSTRAP (four commands).

## Out of scope

wait_for/script/form_fill vocab entries (their own tasks), read_page/get_page_text (no vocab,
ADR-0038 D2), guidance text (C11), any binary dispatch change.

Commit: `feat(results): structuredContent for find/tabs/navigate + declared outputSchema`
