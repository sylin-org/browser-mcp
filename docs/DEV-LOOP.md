# The Ghostlight dev loop

Ghostlight ships as two executables (ADR-0046, ADR-0051 Phase 3): `ghostlight` (the CLI + the
persistent service) and `ghostlight-relay` (the single thin pass-through, role-selected at launch:
`--role agent` for your MCP client, the browser role auto-detected when Chrome launches it). Only the
service carries the churny code; the relay is a thin, resilient pipe. That split is what makes the dev
loop frictionless: you rebuild and restart the service while the relay keeps your editor and browser
connected.

Use a named instance (here `dev`) so your work never touches the default install. Since ADR-0064,
`dev` is a FULLY ISOLATED stack: its own native host (`org.sylin.ghostlight.dev`), its own relay copy
(`ghostlight-relay-dev`), its own service and dirs. The unpacked dev extension self-selects the dev
host by its identity, so it targets the dev service explicitly -- there is no auto-shadow that makes
an unpinned client "prefer a live dev" anymore. Your real browser + store extension keep talking to
the default install, untouched.

## 1. Build

```
cargo build -p ghostlight
```

Build ONLY the `ghostlight` package. It does not relink the `ghostlight-relay` binary, so a running
relay (launched by your editor as `ghostlight-relay --role agent`) is never locked, and the rebuild
always succeeds even while an editor session is live.

## 2. Install the dev instance (once)

```
ghostlight --instance dev install --debug --no-supervisor
```

ADR-0064: a `dev` install registers the FULL isolated dev stack -- the `org.sylin.ghostlight.dev`
native host (allowing the unpacked-dev extension id), a `ghostlight-relay-dev.exe` copy the browser
launches by name (it pins `instance=dev` from its own argv[0]), and a PINNED `ghostlight-dev`
MCP-client entry. `--no-supervisor` skips the OS auto-start (a developer runs the dev service from a
terminal, next step; an auto-started service would hold the exe lock during rebuilds). Then load the
unpacked extension at chrome://extensions -- it self-selects the dev host by its pinned identity.

Rebuilds and the relay copy: the dev host points at the `ghostlight-relay-dev.exe` copy the install
placed under the dev data dir, so a plain `cargo build` does NOT reach the browser relay. Use
`.\scripts\dev-loop.ps1`, which refreshes that copy from the fresh build on every run; if you rebuild
by hand, re-copy `target/release/ghostlight-relay.exe` over the dev copy yourself.

Your real browser + the Web Store extension keep talking to the default `org.sylin.ghostlight` host
and default service the whole time -- the dev stack never shadows them.

## 3. Run the service in a terminal

```
ghostlight --debug --instance dev service --keep-warm
```

`--keep-warm` disables the idle-grace shutdown, so the terminal service stays up between actions
instead of exiting after a quiet window. Note the flag placement: `--debug` is a root-level flag
and must come BEFORE the `service` subcommand (`--instance` and `--keep-warm` are accepted in
either position).

## 4. The edit loop

Edit code, then in the service terminal:

```
Ctrl-C            # stop the running service (releases the exe lock)
cargo build -p ghostlight
ghostlight --instance dev service --keep-warm --debug   # rerun
```

You do NOT restart your editor or the browser. The agent adapter reconnects to the fresh service
within its patient reconnect window (up to 120s; ADR-0045), replays the MCP handshake, and your
next tool call is served by the new code. A rebuild that takes a minute or two is invisible to the
MCP client.

## 5. Faster iteration and diagnostics (ADR-0059)

For wire-protocol changes (routing, tabId encoding, focus, notifications) you do not need a real
Chrome session at all:

```
.\scripts\dev-loop.ps1                                              # kill/rebuild/restart/health-check in one shot
.\target\release\lightbox.exe fake-browser --instance dev --auto-reply   # attach as a fake browser, no Chrome needed
```

`fake-browser` dials the real service exactly as the real relay does, prints every frame it
receives, and (with `--auto-reply`) answers `tabs_context_mcp`/`tabs_create_mcp` with a
DELIBERATELY billion-scale tab id -- the same magnitude a real Chrome session actually produces --
so a tabId-encoding regression is caught on the first offline round trip. Commands at its prompt:
`focus`, `kill`, `reply <id> <json-result>`, `quit`.

When you do need a real browser, `.\scripts\dev-browser.ps1` launches an isolated, disposable
Chrome profile (never your real one) pointed at the unpacked dev extension, with
`GHOSTLIGHT_DEBUG=1` set so the browser-role relay writes debug state too.

Every attach/detach/focus/reject decision (both sides: the service's own and, when the
extension's "Developer diagnostics" option is on, the extension's `connect_attempt`/
`connect_disconnect` notes) lands in the SAME structured event ring `debug-state-<pid>.json`
already carries -- `ghostlight --instance dev doctor` and that file are the first places to look,
before reasoning about timing from raw process logs.

## 6. Live-testing a browser-visible feature end-to-end

For anything you actually need to SEE (FX, notifications, layout) rather than just wire-protocol
correctness, `fake-browser` is not enough -- it never renders a page. This is the concrete
recipe, and the gotchas that cost real time the first few passes.

### 6.1 Before you touch anything: check who is attached

Since ADR-0064 each client targets exactly ONE instance explicitly: a `dev`-pinned MCP client (the
`ghostlight-dev` entry the dev install wrote, or one launched with `--instance dev`) drives the dev
service; a default client drives the default service -- the user's real, authenticated browser
session. There is no auto-shadow, so a client never silently jumps between them. Still check who is
attached before driving, so you know your `navigate`/`computer` calls land where you expect:

```
ghostlight doctor                       # default instance: what's attached right now?
ghostlight --instance dev doctor        # dev instance: same question
```

If the default instance's most recent session shows `extension not connected` / `[exited]`,
nothing routes there and it is safe to proceed. If it shows `extension connected (live)`, treat
the browser as live and do not send it tool calls as part of a test.

### 6.2 Bring up dev and a disposable browser

```
.\scripts\dev-loop.ps1        # kill this repo's own dev processes, rebuild, restart, health-check
.\scripts\dev-browser.ps1     # isolated, disposable Chrome profile + unpacked dev extension
```

Then confirm attachment before doing anything else:

```
.\target\release\ghostlight.exe --instance dev doctor
```

Look for `extension connected (live)` and a `Browsers:` line naming a pid, and verdict `OK`.

**Known flake:** the first `dev-browser.ps1` launch sometimes produces a relay process that stays
alive but never registers ANY attach attempt server-side (zero entries in
`debug-state-<pid>.json`'s `recent` ring, even after minutes). Not root-caused as of 2026-07 --
suspected a Chrome/native-messaging cold-start race, not a code defect, since the exact same
binaries attach cleanly on the next attempt. If doctor does not show `extension connected` within
~15 seconds, kill the disposable Chrome + its relay (both are safe to kill -- never anything
matching your real profile) and re-run `dev-browser.ps1`. Do not add `--remote-debugging-port` to
chase this: doing so caused the extension to fail to load AT ALL (absent from `chrome://extensions`
entirely) in the one session that tried it, whatever the cause -- keep the CDP-debugging surface
and the "does this reproduce normally" surface separate.

### 6.3 Drive the browser with your own tool calls

Once attached, a `dev`-pinned MCP client's `mcp__ghostlight__*` tools drive the dev service (ADR-0064)
and land in the disposable browser you just opened -- not the user's real one. (If your MCP client
is NOT dev-pinned, it drives the default service; pin it with `--instance dev` or use the
`ghostlight-dev` client entry.)

```
tabs_context_mcp(createIfEmpty: true)   # note the huge composite tabId -- (slot << 32) | native_tab_id, expected
navigate(tabId, url)
computer(action: "screenshot", tabId)
```

Three gotchas:

- **`chrome://newtab/` and other `chrome://` pages cannot host a content script.** Anything that
  renders via `agent-visual-indicator.js` or `content.js` (FX, denial notifications) needs a real
  `http(s)` page loaded in the tab first. Navigate to an in-grant page (the committed
  `examples/dev-live-test.json` fixture grants `example.org`) before triggering the thing you
  actually want to see.
- **A screenshot NEVER shows FX or the notification bar in the captured pixels, by design** --
  every effect (cursor, ripples, the notification layer) is hidden for the duration of the
  capture so the agent's own screenshot stays clean, then restored after. Do not read a clean
  screenshot as "it didn't render" or "it got dismissed" -- it means neither on its own. Only a
  read-only action (screenshot, zoom, get_page_text, wait) hides-and-restores; a genuine
  mutating action (click, type, scroll, navigate) on the SAME tab actually dismisses a
  notification, by its own design (persistent until the next real action or an explicit close).
  To see whether something is still there, either ask the user to look at their own screen (the
  fastest path in practice), or capture it out-of-band over the browser's own devtools websocket
  (`Page.captureScreenshot` via `--remote-debugging-port`, launched fresh and separately from the
  attach you are trying to observe -- see the caution in 6.2 about combining the two).
- **After editing extension JS, a fresh disposable profile is not enough on its own.** One
  session's testing showed a content-script edit NOT taking effect even in a brand-new
  `--load-extension` profile (the stale behavior persisted identically to before the edit) until
  the extension was explicitly reloaded via `chrome://extensions`'s Reload button -- suggesting
  Chrome caches something (plausibly V8 bytecode) keyed by the extension's pinned id
  (`manifest.json`'s `key` field) across profiles, not just within one. Not root-caused as of
  2026-07. After any content-script/service-worker edit, reload the extension explicitly before
  trusting a "still broken" observation, even on a fresh profile.

### 6.4 The `notify` tool: iterating on notifications without a denial

`notify` is an UNLISTED tool: a direct entry point onto `Browser::notify()` -- the same primitive
governance denials call to draw the on-screen ribbon. It takes `tabId`, `class`
(`error`/`warn`/`info`/`debug`), optional `icon` (`lock` or anything else -> shield), `title`, and
optional `description`, and renders the ribbon immediately, bypassing governance (it IS the channel
governance speaks through). It is deliberately absent from `tools/list` and NOT registered in
`browser/directory.rs` -- the ribbon is a governance-authority signal, not something the trained
model should emit -- so it exists only as the first branch of `run_tool_call` in
`crates/core/src/mcp/pipeline.rs`. Look there, not in the directory, when auditing what tools exist.

For notification-design work this is the fast path: rebuild the service ONCE (to pick up the tool),
reload the extension ONCE (to pick up any renderer CSS), then fire every severity/icon combination
as plain `notify` calls -- no rebuild per variant.

Two caveats when driving it:
- Because it is unlisted, an MCP client's own tool list will not contain it. Send a raw JSON-RPC
  `tools/call` (name `notify`) over the agent relay (`ghostlight-relay --role agent --instance dev`)
  rather than through a client's advertised-tool surface.
- `server.rs`'s cross-session tab-ownership guard runs BEFORE `run_tool_call` and refuses a
  `tools/call` naming a `tabId` a DIFFERENT live session owns (returns "unknown tab"). So the notify
  call must come from a session that OWNS the tab: have the same relay session create its own tab
  (`tabs_create_mcp`) and navigate it before calling `notify`. The internal denial path is
  unaffected -- it calls `Browser::notify()` directly, never through an incoming `tools/call`.

### 6.5 Clean up

Kill only processes whose executable path is under this repo's own `target\` directory, or whose
command line names the disposable `ghostlight-dev-browser` profile directory -- the same rule
`dev-loop.ps1` itself follows. Never a bare `taskkill /IM ghostlight.exe` or `/IM chrome.exe`: the
user's real, installed instance and real browser windows share those names.
