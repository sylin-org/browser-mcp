# Competitive Landscape: user-session browser MCP + governance

Date: 2026-07. Method: a four-lens cited web sweep (verify-known-candidates, discovery, positioning,
governance). This is a discovery note. See also docs/research/01 (Stagehand/Browserbase), 03
(governance prior art), and 04 (MCP gateways) for earlier prior-art work.

## Bottom line

"A browser extension that drives your real, logged-in Chrome and works with any MCP client" is now a
crowded space, not a novel one. The differentiator that holds is GOVERNANCE fused into that model.
No project found combines all four of:

1. automating the user's own authenticated session via a thin extension,
2. MCP-client-agnostic (any client, not one vendor's app),
3. a built-in governance overlay: identity-bound domain grants, read/write tool classification, and
   structured audit, with all-open as a first-class mode,
4. open-source, local-first, single binary.

The market splits into two camps that do not overlap: real-session automation with no governance,
and governance with no user-session automation surface.

## Camp A: real-session automation, client-agnostic, no governance

These match our automation model and herald "any MCP client." None ship access control, read/write
tool classification, domain limits, or audit. Their privacy story is "it runs locally."

| Project | Model | Traction | Note |
|---|---|---|---|
| hangwin/mcp-chrome | extension on your Chrome | ~12k stars, MIT, active | Closest architectural twin: extension + native messaging + bridge, stdio and Streamable HTTP. Pitches "chatbot/model agnostic." |
| browsermcp.io (Browser MCP) | extension | ~6.8k stars, Apache-2.0 | Our namesake. Stale (last push ~Apr 2025). The extension itself is closed source; only the npm server is open. |
| ofershap/real-browser-mcp | extension | ~35 stars, MIT, early | Near-identical shape: server + MV3 extension over localhost WebSocket, 18 tools. |
| Agent360dk/browser-mcp, djyde/browser-mcp | extension | small | Also literally named "browser-mcp." Agent360dk adds human-in-the-loop for 2FA and tab-group session isolation. |
| Microsoft Playwright MCP (`--extension`) | opt-in real session | ~34.6k stars, Apache-2.0 | Reuses your logged-in tab via a Chrome-Web-Store extension. The best-funded competitor on the automation axis. |

## Adjacent

- Google chrome-devtools-mcp (~44.9k stars, Apache-2.0): can attach to a running Chrome
  (`--browser-url`/`--autoConnect`); default is a fresh dedicated profile. Debug and inspection
  altitude. Docs warn it exposes all browser data to the client.
- vercel-labs/agent-browser (~37.7k stars, Apache-2.0): the closest architecture analog. A single
  Rust binary over CDP with domain allowlists and action policies (governance-lite). It copies the
  Chrome profile to a temporary snapshot (a fresh browser, no extension, not the live session) and
  has no audit or identity layer.
- browser-use (~102k stars, MIT): the most popular OSS "make the browser do things" framework, and
  an MCP server. Drives its own Playwright browser by default; can opt into a real profile, CDP, or
  a recent extension bridge. Enterprise controls live in the paid cloud, not the local server.
- Nanobrowser (~13.4k stars, Apache-2.0): the biggest extension-on-real-session player, but it runs
  its own multi-agent loop with your LLM key and is not an MCP server. It cannot be driven by Claude
  Code or Cursor, which validates the "bring your own MCP client" angle.
- Anthropic Claude for Chrome (closed): the origin this project is a clean-room rewrite of. It does
  pair real-session automation with governance (per-site permissions, enterprise admin allow and
  blocklists, ask-vs-act modes, high-risk confirmations), but it is single-vendor, closed source,
  and exposes no MCP surface for other clients.

## Camp B: governance without a user-session automation surface

- Generic MCP gateways: Lasso MCP Gateway, Stacklok ToolHive, MintMCP, Portkey, Bifrost. Real policy
  plus audit for any MCP tool call, but no browser of their own. One could sit in front of a browser
  MCP as a bolt-on proxy.
- Enterprise agentic-browser security: LayerX, Island, Zenity, SquareX, Palo Alto Prisma Access
  Browser. They police and audit AI-agent activity inside the browser (Island governs "tool calls,
  MCP access, and agent-to-agent communication" with a full audit trail), but they are closed SaaS,
  several replace the whole browser, and they oversee agents rather than expose an automation API.
- PageBolt MCP (closed): the one shipping "browser automation + audit, rate-limit, SSRF" bundle, but
  it runs its own managed cloud browser, not your authenticated session.

## The uncontested intersection

Across all four lenses, no project combines Camp A's real-session client-agnostic automation with
Camp B's governance in an open, local-first, single-binary form. "Governed browser automation over
your own session, open and self-hostable" is genuinely uncontested. It is, in effect, Claude for
Chrome's governance model made open, vendor-neutral, and self-hostable.

## Recommendations

1. Rename. "Browser MCP" is an established ~6.8k-star product (browsermcp.io), and at least two more
   repositories are literally named `browser-mcp` (djyde, Agent360dk). The name is overloaded;
   discovery and identity will suffer. Choosing a distinct name is the clearest action from this
   research.
2. Lead with governance, not client-agnostic. Client-agnostic is now table stakes (mcp-chrome,
   Playwright MCP extension mode, and others all have it). The defensible position is the governance
   fusion plus open and self-hostable. The Rust single binary is a credible secondary
   differentiator: almost every competitor is Node/TS; only agent-browser is Rust.

## Watch items

- browser-use: largest OSS player and adding session/extension modes. If it adds governance it
  becomes the most credible competitor.
- Playwright MCP extension mode: well funded, squarely on the automation axis.
- WebMCP (`navigator.modelContext`): a W3C draft from Google and Microsoft, in Chrome origin trials
  in 2026. It inverts the model (sites declare callable tools to the browser's agent) and could
  complement or partly obviate CDP and screenshot-driven tools over time.

## Key sources

- hangwin/mcp-chrome: https://github.com/hangwin/mcp-chrome
- browsermcp.io: https://browsermcp.io/ , https://github.com/browsermcp/mcp
- ofershap/real-browser-mcp: https://github.com/ofershap/real-browser-mcp
- Microsoft Playwright MCP: https://github.com/microsoft/playwright-mcp , https://playwright.dev/mcp/configuration/browser-extension
- Google chrome-devtools-mcp: https://github.com/ChromeDevTools/chrome-devtools-mcp
- vercel-labs/agent-browser: https://github.com/vercel-labs/agent-browser
- browser-use: https://github.com/browser-use/browser-use
- Nanobrowser: https://github.com/nanobrowser/nanobrowser
- Claude for Chrome: https://claude.com/claude-for-chrome
- Stacklok ToolHive: https://github.com/stacklok/toolhive
- Island (enterprise browser): https://www.island.io/
- WebMCP (W3C): https://github.com/webmachinelearning/webmcp
