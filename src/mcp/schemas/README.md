# Tool Schemas -- the sacred surface

`tools.json` is the advertised MCP `tools/list` surface: the **13 tools** Browser MCP preserves
from the reference (open-claude-in-chrome). It is byte-faithful to the reference's tool **names,
descriptions, parameter names/types/enums/constraints, and required sets** -- the content the
model's trained behavior depends on. This is the **one** thing we preserve verbatim; everything
else is a clean, lean re-design (Browser MCP is not a port).

## Provenance and fidelity
- **Authored from the reference's verbatim zod definitions** (`host/mcp-server.js`), captured in
  `reference/ANALYSIS.md` Section 1. No external code was executed to produce this.
- The **semantic content is exact**. The JSON-Schema wrapper here
  (`type`/`properties`/`required`/`additionalProperties`) is our canonical form; the exact
  serialization the reference's MCP SDK emits was **not** byte-verified (that would require running
  the reference). If byte-identical-to-SDK output is ever needed, capture the reference's live
  `tools/list` and diff against this file.
- `tests/tool_schema_fidelity.rs` guards this file (and, once `tools/list` is implemented, our
  emitted output) against drift.

## Tools (13)
`tabs_context_mcp`, `tabs_create_mcp`, `navigate`, `computer` (13 actions), `find`, `form_input`,
`get_page_text`, `javascript_tool`, `read_console_messages`, `read_network_requests`, `read_page`,
`resize_window`, `update_plan`.

Excluded reference stubs (not advertised): `gif_creator`, `shortcuts_list`, `shortcuts_execute`,
`switch_browser`, `upload_image`.
