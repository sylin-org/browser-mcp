# Installing Ghostlight (for AI agents)

Ghostlight is an MCP server that gives you governed access to the user's own, already-logged-in
Chromium browser. Follow these steps exactly; each is idempotent and safe to re-run.

## 1. Add the MCP server

Add this stdio server entry to the MCP settings file you manage (no install step needed; the npm
launcher fetches a single portable binary on first run):

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

## 3. Ask the user to add the browser extension

The extension cannot be installed from the command line. Ask the user to:

- download `ghostlight-extension-v*.zip` from
  https://github.com/sylin-org/ghostlight/releases/latest, unzip it, and load it unpacked at
  `chrome://extensions` (Developer mode -> Load unpacked).

A Chrome Web Store listing ("Ghostlight in Browser") is in preparation; once it is live, the
user can install from the store instead.

Then ask the user to restart the MCP client (so it picks up the new server) and, if the browser
was already open, reload the extension.

## 4. Verify

```sh
npx -y ghostlight doctor
```

`doctor` is read-only and prints a specific, actionable finding for anything unhealthy (browser
not registered, extension not connected, no server running). Exit code 0 means the whole chain is
healthy. If the extension shows disconnected, reloading it at `chrome://extensions` is the usual
fix.

## 5. First use

Call `tabs_context_mcp` with `createIfEmpty: true` to get a `tabId`, then `navigate`. The agent
works inside a dedicated, clearly labeled tab group, visually separate from the user's own tabs.
Call `explain` at any time to see every available action and the capability it requires under the
session's policy (with no policy configured, everything is allowed).
