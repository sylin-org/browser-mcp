# Ghostlight for the solo developer

Ten minutes from clone to an agent driving your real browser, plus the optional personal
safety rails. Everything on this page is free, forever, with no key and no registration.

## What you get

Your AI agent (Claude Code, Claude Desktop, Cursor, VS Code, or any MCP client) gets a
real browser: your cookies, your logins, your tabs. Seventeen tools -- navigate, click,
type, screenshot, read the page, find elements, fill forms (by ref or by label), run
JavaScript, inspect console and network traffic, wait for dynamic pages to settle,
compose multi-step scripts, and manage tabs -- at byte-parity with the schemas the model
was trained on, plus the composition tools. The agent works inside its own tab group
(labeled with a ghost) so its activity is visually separate from yours.

By default Ghostlight is all-open: no policy, no restrictions, no audit. Governance is
an overlay you can opt into later, one setting at a time.

## Setup

Prerequisites: a Chromium browser (Chrome, Edge, Brave, or Chromium 116+), an MCP
client, and a stable Rust toolchain (https://rustup.rs) to build the binary.

1. Build:

       git clone https://github.com/sylin-org/ghostlight
       cd ghostlight
       cargo build --release

2. Load the extension: open `chrome://extensions`, enable Developer mode, click
   "Load unpacked", select the `extension/` directory. The committed manifest key pins
   the extension id to `cjcmhepmagomefjggkcohdbfemacojoa`; confirm that is what Chrome
   shows.

3. Register everything (native host + your MCP clients), idempotently:

       ./target/release/ghostlight install --extension-id cjcmhepmagomefjggkcohdbfemacojoa

   Add `--dry-run` first if you want to see the plan before it writes.

4. Restart your MCP client and reload the extension, then verify the whole chain:

       ./target/release/ghostlight doctor

   A healthy report says the browser and client are registered, the IPC endpoint
   accepts, and the extension is connected. Anything wrong prints as a specific finding.

5. First prompt to your agent:

   > Open a new browser tab, go to example.com, and tell me what the page says.

## Optional personal safety rails

These are for you, not for an employer, and they are always free.

**Sacred domains** -- sites the agent must never touch, enforced on every tool call
regardless of anything else:

    ./target/release/ghostlight config set content.security.sacred_domains '["*.mybank.com","brokerage.example"]'

**The pause and the kill switch.** The extension popup gives you take-the-wheel: pause
the agent mid-run, take over the browser, resume when ready. The panic kill switch
severs the session outright.

**Secret redaction.** Password, OTP, and payment field values are replaced with
`[value redacted]` in page reads when `content.security.secrets.redact` is on (it is on
under the default preset).

**The audit trail, for yourself.** One JSON line per tool call:

    ./target/release/ghostlight config set audit.enabled true

Records land in your local data directory (`audit.jsonl`); `config get audit.file.path`
shows where.

**A personal policy**, if you want the agent limited to certain sites. Start from an
example and preview what it means in plain sentences:

    ./target/release/ghostlight policy init --template developer-unrestricted --out my-policy.json
    ./target/release/ghostlight policy explain my-policy.json

Then point the server at it by setting `GHOSTLIGHT_MANIFEST=file:///path/to/my-policy.json`
in the MCP server's environment (or see `examples/research-read-only.json` for a
read-only starting point). No manifest means all-open; removing the variable removes all
policy.

## Where the free line is

Everything above, all of it, free forever, including for your side business. The paid
line is organizations of more than five people running centrally-managed governance in
production: see [PRICING.md](../../PRICING.md). If that is not you, you never need to
think about it.

## When something breaks

Run `ghostlight doctor` first; it pinpoints the common failures. On Windows, remember a
running server locks `ghostlight.exe`: stop the MCP client before `cargo build`. The
Troubleshooting section of the [README](../../README.md) has the rest.
