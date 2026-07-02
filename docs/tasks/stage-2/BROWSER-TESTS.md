# Stage 2 browser tests

Deferred live-browser verification for stage-2 governance. The unattended executor CANNOT drive a real
browser, so every check that needs one is written here instead of run. A human runs these against a
live browser after the code lands (as in release-1). Accumulate entries as tasks land; do not delete
them.

## Format

One entry per check:

```
## <task-id>-<n>: <one-line purpose>
Changed: <what code changed and why a browser is needed to verify it>
Steps: <exact, ordered steps a human runs (tools, URLs, inputs)>
Expect: <the precise observable result that means PASS>
```

Keep steps concrete and self-contained (name the tool, the URL, the manifest/config used). Prefer
checks that are unambiguous to eyeball. Note when a check depends on a specific manifest or config
posture (all-open vs a restrictive manifest vs observe/shadow mode).

## Checks

## g08-1: sacred domains deny the agent live, and the audit log records it
Changed: g08 wired the first real enforcement path (ADR-0018 step 2) at the dispatch
chokepoint: a `content.security.sacred_domains` entry now denies any tool call whose
current tab or `navigate` target matches it, before the tool runs. This needs a live
browser and a live MCP client (Claude Code) restart to observe end to end; the automated
suite (`transport::mcp::server::tests::sacred_tab_denies_every_tool_and_never_runs_it`,
`navigate_target_denied_even_when_tab_is_clean`, `empty_list_is_byte_identical`,
`denied_call_writes_one_deny_record`) proves the same code path against a fake extension,
but not real on-screen browser behavior or the real default audit file location.
Steps:
1. Edit the user config file (Windows: `%APPDATA%\browser-mcp\config.json`) to
   `{ "config": { "content.security.sacred_domains": ["example.com", "*.example.com"] } }`.
2. Restart the MCP client (Claude Code) so the new binary/config is picked up.
3. Ask the agent to navigate a tab to `https://example.com/`.
4. Manually navigate a Browser MCP group tab to `https://example.com/` (or reuse the tab
   from step 3), then ask the agent to read or screenshot that tab, and separately ask it
   to navigate that same tab to `https://example.org/`.
5. Ask the agent to navigate to `https://example.org/` (a clean domain).
6. If `audit.enabled` resolves true (the Minimal default), inspect the audit JSONL file
   (default `%LOCALAPPDATA%\browser-mcp\audit.jsonl`) after the above.
Expect: step 3's tool result starts with `Denied (D-` and names `example.com`; the browser
does not actually navigate. Step 4's read/screenshot is denied with the same message
shape (naming `example.com`), and navigating that tab elsewhere is ALSO denied (the
never-touch rule blocks moving the tab away, not just reading it). Step 5 works normally
(the browser navigates, the agent gets real page content). Step 6 shows one
`"decision":"deny"` record per denial above, each with a stable `denial_id` (identical
across repeats of the same denial), `"grant_id":null`, and `"domain"` naming the matched
host; no denial record for the step-5 call.
