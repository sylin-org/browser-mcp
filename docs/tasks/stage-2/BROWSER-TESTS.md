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

## g10-1: popup renders hold state and the toggle button works
Changed: g10 added the first extension UI (`popup.html`/`popup.js`), the `action` and
`commands` manifest keys, and the hold request/reply plumbing in `service-worker.js`. This
is the extension's first popup ever; it can only be verified by loading the unpacked
extension in Chrome.
Steps:
1. Reload the unpacked extension at `chrome://extensions` (pick up the new `action`/
   `commands` manifest keys and the new JS).
2. With no MCP session running (browser-mcp binary not started), click the toolbar icon.
3. Start an MCP session (Claude Code connected, extension attached), click the toolbar
   icon again.
4. Click the `Pause agent browsing (take the wheel)` button.
5. Click the resulting `Resume agent browsing` button.
Expect: step 2 shows `No active browsing session.` with the button disabled, within about
1.5 seconds (the `holdRequest` timeout). Step 3 shows `Agent browsing is allowed.` with an
enabled `Pause agent browsing (take the wheel)` button. Step 4 flips the status to
`Agent browsing is PAUSED.`, the button label to `Resume agent browsing`, and the toolbar
badge shows `II`. Step 5 flips back to `Agent browsing is allowed.` and clears the badge.

## g10-2: a paused agent gets the pause text, never reaches the extension
Changed: g10 wired the hold check into `handle_tools_call`, before `governance.decide`,
the sacred check, and any extension traffic. Needs a live Claude Code + extension to
observe the agent's own behavior and confirm no `tool_request` frame reaches the
extension.
Steps: with the extension paused (see g10-1 step 4), ask the agent to take a screenshot.
Expect: the agent receives text starting with `Paused: the user has taken control of the
browser`, naming the `'computer (screenshot)'` call as NOT executed; the tool result is a
normal successful response (not an error) and the agent reports it is waiting for the
user, not retrying. No CDP/tab activity occurs in the browser.

## g10-3: the 2-minute resume hint appears
Changed: `hold_message` appends a second sentence once `held_for >= HOLD_HINT_AFTER`
(2 minutes). Needs a live timing check (or a temporarily lowered `HOLD_HINT_AFTER` in a
local build, restored before committing) since the automated suite only proves the pure
function's threshold logic, not a real elapsed-wall-clock pause.
Steps: pause the extension, wait past 2 minutes (or rebuild locally with a lowered
`HOLD_HINT_AFTER`, verify, then restore and rebuild the real constant), then ask the agent
for any browser tool call.
Expect: the reply carries the base `Paused:` text plus, appended, `This session has been
paused for more than 2 minutes. Only the user can resume it, from the Browser MCP
extension: the popup Pause/Resume button or the toggle keyboard shortcut.`

## g10-4: the hold survives a service-worker restart
Changed: the hold flag lives in `Browser` (the mcp-server process), not the extension;
`route_reply`'s hold-request handling and the flag itself are unaffected by the extension
process dying and Chrome relaunching its service worker. This is exactly the property a
disconnect-driven test cannot fully simulate without a real Chrome service-worker
lifecycle event.
Steps: pause the extension, then in `chrome://extensions` click the service worker's
"service worker" link and terminate it (or use the "Reload" action on the extension while
paused), let it restart, then ask the agent for a browser tool call.
Expect: the tool call is STILL answered with the `Paused:` text -- the binary-side flag
was never touched by the extension restart. Re-opening the popup after the restart shows
`Agent browsing is PAUSED.` (matches the binary's state once the new service worker
reconnects and queries it).

## g10-5: the keyboard shortcut toggles the hold with the popup closed
Changed: `chrome.commands.onCommand` (the `toggle-hold` command, default `Alt+Shift+P`) is
new; a keyboard shortcut can only be exercised via a live Chrome window.
Steps: with the popup closed and an MCP session active, press `Alt+Shift+P` (or whatever
`chrome://extensions/shortcuts` shows if reassigned), then open the popup.
Expect: the toolbar badge toggles (`II` appears or clears) immediately on the keypress;
the popup's rendered state (`Agent browsing is PAUSED.` / `Agent browsing is allowed.`)
matches the badge when opened afterward.
