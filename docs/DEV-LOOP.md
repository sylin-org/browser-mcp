# The Ghostlight dev loop

Ghostlight runs ONE stack (ADR-0065): one native host (`org.sylin.ghostlight`, allowing both the
Web Store extension and the unpacked dev extension), one service endpoint, one `ghostlight` MCP
entry in your editor. The "engine" is whichever `ghostlight` service currently holds the endpoint
-- the installed release, or the build you made thirty seconds ago. Nothing selects an engine;
ownership of the endpoint IS the selection.

That works because the relays are dumb, resilient pipes: the agent relay reconnects to a restarted
service and replays the MCP handshake (ADR-0045), and the browser relay reconnects and replays the
extension's identity frame (ADR-0062), so an engine swap is invisible to your editor and your
browser. `deploy.lock` (ADR-0063) keeps relay self-heal from respawning the OLD engine mid-swap.

There is no dev install, no `-dev` host, no second MCP entry, and no separate dev browser. Your
real, authenticated browser and your real editors ride the engine under test -- which is the point:
dev exercises the real scenario. The symmetric cost: while a broken build holds the endpoint, real
use is broken until you swap back (`-Restore`) or land a fix.

## 1. The loop

```
.\scripts\dev-loop.ps1              # quiesce, stop the engine, rebuild, start the fresh build
# ... edit code, repeat ...
.\scripts\dev-loop.ps1 -Restore     # hand the endpoint back to the newest installed release
```

What the script does, in order: writes `deploy.lock` into every candidate engine directory (the
repo `target\` dir and each versioned dir under `~\.ghostlight\bin`) so no relay self-heals the old
image mid-swap; stops SERVICE processes only (identified by executable path, never a bare taskkill
-- and never relays, which stay connected and ride through); renames any running relay exe aside
(Windows allows renaming a running image) so the build can write; builds `ghostlight` +
`ghostlight-relay` + `lightbox`; starts the fresh build as THE engine (`--debug service
--keep-warm`); waits for `ghostlight doctor` to report the endpoint healthy; removes the locks; and
runs one offline `fake-browser` smoke check.

`--keep-warm` disables the idle-grace shutdown so the engine stays up between actions. Add
`-Manifest examples\dev-live-test.json` when you want the engine started under a restrictive test
policy (default is none: the engine serves real use with the real config).

When NO dev build is running and a relay finds the endpoint down, self-heal launches the engine
sibling to that relay's own directory -- the system reverts to an available engine on its own.
`-Restore` does it deterministically: stops the repo-built engine and starts the newest installed
release.

## 2. Who is serving right now?

```
ghostlight doctor
```

Doctor names the endpoint state, the attached browsers, and the live sessions. Because there is
one endpoint, there is no "which instance?" question -- only "who holds it?", and doctor answers
that. Every attach/detach/focus/reject decision (both sides: the service's own and, when the
extension's "Developer diagnostics" option is on, the extension's `connect_attempt`/
`connect_disconnect` notes) lands in the structured event ring `debug-state-<pid>.json` carries --
look there before reasoning about timing from raw process logs.

## 3. Extensions

Both extension builds talk to the same host, and the host manifest allows both ids:

- The **unpacked dev extension** (chrome://extensions, Load unpacked, `extension/`; its id is
  pinned by the committed manifest `key`, ADR-0016). Load it in whatever browser you actually use.
- The **Web Store extension**, once released.

Do not run both builds in the SAME browser profile -- they would each open a native port and appear
as two browsers (harmless to the service, ADR-0061 gives each a slot, but confusing to you). One
browser, one build. After editing extension JS, reload the extension from chrome://extensions --
Chrome caches aggressively (plausibly V8 bytecode keyed by the pinned extension id), and a stale
worker has survived even a fresh profile. Never trust a "still broken" observation until after an
explicit reload.

Version skew is a normal condition here: right after an engine swap, the loaded extension is one
build older than the engine until you reload it (and a released extension may be older still).
Wire-protocol changes must stay additive and tolerant -- unknown fields ignored, absent fields
defaulted (ADR-0065 Decision 6).

## 4. Offline iteration (no browser at all)

For wire-protocol changes (routing, tabId encoding, focus, notifications) you do not need a real
browser:

```
.\target\release\lightbox.exe fake-browser --auto-reply
```

`fake-browser` dials the engine exactly as the real relay does, prints every frame it receives,
and (with `--auto-reply`) answers `tabs_context_mcp`/`tabs_create_mcp` with a DELIBERATELY
billion-scale tab id -- the same magnitude a real browser produces -- so a tabId-encoding
regression is caught on the first offline round trip. Commands at its prompt: `focus`, `kill`,
`reply <id> <json-result>`, `quit`.

Tests and the e2e harness never touch the real endpoint: they run ephemeral NAMED instances
(`--instance <name>` / `GHOSTLIGHT_INSTANCE`, ADR-0044) as a pure isolation seam. That is the only
remaining use of named instances -- no user- or dev-facing workflow installs or pins one
(ADR-0065 Decision 5).

## 5. Live-testing a browser-visible feature end-to-end

For anything you actually need to SEE (FX, notifications, layout) rather than wire-protocol
correctness, `fake-browser` is not enough -- it never renders a page.

### 5.1 Check who is attached first

```
ghostlight doctor
```

Look for `extension connected (live)` and a `Browsers:` line naming your browser. Because your
tool calls land in the user's real browser, know what is attached before driving it.

### 5.2 Drive the browser with your own tool calls

```
tabs_context_mcp(createIfEmpty: true)   # note the huge composite tabId -- (slot << 32) | native_tab_id, expected
navigate(tabId, url)
computer(action: "screenshot", tabId)
```

Three gotchas:

- **`chrome://newtab/` and other `chrome://` pages cannot host a content script.** Anything that
  renders via `agent-visual-indicator.js` or `content.js` (FX, denial notifications) needs a real
  `http(s)` page loaded in the tab first. Navigate somewhere real (with `-Manifest
  examples\dev-live-test.json`, the committed fixture grants `example.org`) before triggering the
  thing you want to see.
- **A screenshot NEVER shows FX or the notification bar in the captured pixels, by design** --
  every effect (cursor, ripples, the notification layer) is hidden for the duration of the
  capture so the agent's own screenshot stays clean, then restored after. Do not read a clean
  screenshot as "it didn't render" or "it got dismissed" -- it means neither on its own. Only a
  read-only action (screenshot, zoom, get_page_text, wait) hides-and-restores; a genuine
  mutating action (click, type, scroll, navigate) on the SAME tab actually dismisses a
  notification, by its own design (persistent until the next real action or an explicit close).
  To see whether something is still there, ask the user to look at their own screen (the fastest
  path in practice), or capture out-of-band over the browser's own devtools websocket
  (`Page.captureScreenshot` via `--remote-debugging-port`, launched fresh and separately from the
  attach you are trying to observe).
- **After editing extension JS, reload the extension explicitly** (section 3) before trusting any
  observation.

### 5.3 The `notify` tool: iterating on notifications without a denial

`notify` is an UNLISTED tool: a direct entry point onto `Browser::notify()` -- the same primitive
governance denials call to draw the on-screen ribbon. It takes `tabId`, `class`
(`error`/`warn`/`info`/`debug`), optional `icon` (`lock` or anything else -> shield), `title`, and
optional `description`, and renders the ribbon immediately, bypassing governance (it IS the channel
governance speaks through). It is deliberately absent from `tools/list` and NOT registered in
`browser/directory.rs` -- the ribbon is a governance-authority signal, not something the trained
model should emit -- so it exists only as the first branch of `run_tool_call` in
`crates/core/src/mcp/pipeline.rs`. Look there, not in the directory, when auditing what tools exist.

For notification-design work this is the fast path: swap the engine ONCE (to pick up the tool),
reload the extension ONCE (to pick up any renderer CSS), then fire every severity/icon combination
as plain `notify` calls -- no rebuild per variant.

Two caveats when driving it:
- Because it is unlisted, an MCP client's own tool list will not contain it. Send a raw JSON-RPC
  `tools/call` (name `notify`) over the agent relay (`ghostlight-relay --role agent`) rather than
  through a client's advertised-tool surface.
- `server.rs`'s cross-session tab-ownership guard runs BEFORE `run_tool_call` and refuses a
  `tools/call` naming a `tabId` a DIFFERENT live session owns (returns "unknown tab"). So the notify
  call must come from a session that OWNS the tab: have the same relay session create its own tab
  (`tabs_create_mcp`) and navigate it before calling `notify`. The internal denial path is
  unaffected -- it calls `Browser::notify()` directly, never through an incoming `tools/call`.

## 6. Clean up

Kill only processes whose executable path is under this repo's own `target\` directory or under
`~\.ghostlight\bin` -- the same rule `dev-loop.ps1` itself follows. Never a bare
`taskkill /IM ghostlight.exe` or `/IM chrome.exe`. Prefer `.\scripts\dev-loop.ps1 -Restore` over
manual killing: it hands the endpoint back to the installed release cleanly.
