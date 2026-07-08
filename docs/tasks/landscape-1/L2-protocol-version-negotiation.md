# L2: negotiate `protocolVersion` over a supported set

Goal: the `initialize` response echoes the client's requested MCP protocol revision when it is
one the server supports, and offers the latest supported revision otherwise, replacing the
unconditional `"2024-11-05"`. Rationale: docs/design/mcp-spec-currency-2026-07.md ("THE
finding"); authority ADR-0041 Decision 5; shapes and oracles PINS SS7.

## STOP preconditions

- `src/transport/mcp/server.rs` line-160 area contains
  `pub const PROTOCOL_VERSION: &str = "2024-11-05";`.
- `initialize_result` has exactly ONE production (non-test) call site (verify by grep before
  editing). More than one: STOP.
- `tests/mcp_protocol.rs` contains
  `assert_eq!(init["result"]["protocolVersion"], "2024-11-05");` and its initialize request
  sends `"params":{}`.

## Tree facts (as of authoring, 2026-07-07; re-read before editing)

- server.rs: `PROTOCOL_VERSION` line 160 (its only two references in src/ are the const and
  `initialize_result`'s json at line 641); `initialize_result` line 614;
  `capture_client_info` line 605 shows how initialize params are already read.
- tests/mcp_protocol.rs: the initialize request at line 102, the assertion at line 117.
- No other file in src/ or tests/ contains the string `2024-11-05` (verified at authoring).

## Required behavior

Implement PINS SS7 exactly: the two new consts (replacing `PROTOCOL_VERSION`; fix every
compiler-flagged reference), the pure `negotiate_protocol_version`, the `initialize_result`
parameter, and the call-site extraction of `params.protocolVersion`. The negotiation function
is the unit-tested seam; `initialize_result` stays a thin renderer.

## Tests (names and oracles pinned in PINS SS7; transcribe verbatim)

- `protocol_version_negotiation_echoes_supported`
- `protocol_version_negotiation_offers_latest_for_unknown`
- `protocol_version_negotiation_offers_latest_when_absent`
- tests/mcp_protocol.rs line 117's expected value becomes `"2025-06-18"` (the ONLY test
  expected-value change; any other failing version assertion = STOP).

## Verification (literal)

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All green, plus the extension regression line from BOOTSTRAP. Then commit exactly:

```
feat(mcp): negotiate protocolVersion over a supported set (ADR-0041 D5)
```

## Out of scope (fences)

- No capability declarations beyond the existing `{"tools": {}}`; no elicitation, sampling,
  Tasks, or extensions plumbing (the currency note records those as non-goals or future).
- No changes to `instructions`, the capability manifest, `serverInfo`, or tools/list.
- No transport changes (stdio and the hub web tunnel are untouched).
