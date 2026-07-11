# Installing Ghostlight (for AI agents)

Ghostlight is an MCP server that gives you governed access to the user's own, already-logged-in
Chromium browser. You run the commands, then test; pull the user in only for what a CLI can't do
-- restart the client to load the server, and load the browser extension if the test shows it
missing. Everything here is idempotent and safe to re-run.

## 1. Add the MCP server

Add this stdio server entry to the MCP settings file you manage. The `npx` launcher fetches the
portable binary on first run, so there is nothing to pre-install:

```json
{
  "mcpServers": {
    "ghostlight": {
      "command": "npx",
      "args": ["-y", "ghostlight"]
    }
  }
}
```

Requirements on the machine: Node.js (for `npx`) and a Chromium browser (Chrome, Edge, Brave, or
Chromium, version 116+). The binary itself has zero runtime dependencies.

## 2. Connect the browser side (one time)

Run, and show the user the output:

```sh
npx -y ghostlight install
```

This registers the browser's native-messaging host and the MCP client entries (an idempotent,
value-level merge -- it never clobbers existing config and never duplicates entries).

## 3. Test the whole chain

No ghostlight tools yet? Have the user restart the client (skip if it hot-reloaded them). Then get a
`tabId` (`tabs_context_mcp`, `createIfEmpty: true`) and `navigate` to https://sylin.org/ghostlight/.

- Loads and readable -> the whole chain works. Go to step 4; the extension is already there, so do
  not ask the user to install it.
- Errors or not connected -> the browser extension is the likely cause (the one piece the CLI can't
  install):
  1. Run `npx -y ghostlight doctor` to confirm which link is broken.
  2. Have the user download `ghostlight-extension-v*.zip` from
     https://github.com/sylin-org/ghostlight/releases/latest, unzip it, and load it unpacked at
     `chrome://extensions` (Developer mode -> Load unpacked). A Web Store listing ("Ghostlight in
     Browser") is coming; once live, they can use that instead.
  3. Retest (reload the extension at `chrome://extensions` first if the browser was already open).

## 4. First use

You already have a `tabId` from the test above (reuse it, or call `tabs_context_mcp` again). You
work inside a dedicated, clearly labeled tab group, visually separate from the user's own tabs.
Call `explain` at any time to see every available action and the capability it requires under the
session's policy (with no policy configured, everything is allowed).
