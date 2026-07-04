# How Ghostlight compares

As of 2026-07, from a four-lens cited sweep of the landscape
([docs/research/13](research/13-competitive-landscape.md) has the sources and star
counts). The honest summary: "an extension that drives your real, logged-in Chrome from
any MCP client" is a crowded space. The combination that stays uncontested is that model
PLUS a fused governance layer, open and local-first. If you do not need governance,
several of the projects below are excellent.

The four properties, together, are the product:

1. Automates YOUR authenticated session (real cookies, real SSO, real tabs) via a thin
   extension -- never a fresh profile, a profile copy, or a cloud browser.
2. Client-agnostic MCP server: Claude Code, Cursor, VS Code, anything.
3. Governance fused in: capability classification per action, identity-bound host
   grants, sacred never-touch domains, observe/enforce modes, structured audit -- with
   all-open as a first-class default.
4. Open, local-first, single Rust binary; the governance module's source is readable.

## Against the closest neighbors

**Anthropic Claude for Chrome** (closed) -- the origin Ghostlight is a clean-room
rewrite of, and the one competitor that DOES pair real-session automation with
governance (per-site permissions, enterprise allow/blocklists, ask-vs-act). It is
single-vendor (Claude only), closed source, and exposes no MCP surface for other
clients. Ghostlight preserves its trained tool schemas byte-for-byte, so a trained agent
behaves identically, and makes the governance model open, vendor-neutral, and
self-hostable, with capability grants, audit, and an inspectable policy engine.

**hangwin/mcp-chrome** (~12k stars, MIT) -- the closest architectural twin: extension +
native messaging, model-agnostic. No access control, no capability classification, no
domain limits, no audit. Its privacy story is "it runs locally"; Ghostlight's is "it
runs locally, and here is the policy engine, the denial ids, and the audit trail your
security team asked for."

**Microsoft Playwright MCP, extension mode** (~35k stars) -- the best-funded competitor
on the automation axis; its `--extension` mode reuses a real logged-in tab. Node-based,
no governance layer, and browser automation is a side feature of a testing tool.
Ghostlight is purpose-built for the governed-agent case: single portable binary, no
Node, policy and audit at the dispatch chokepoint.

**vercel-labs/agent-browser** (~38k stars) -- the closest ARCHITECTURE analog (a single
Rust binary over CDP) and the only other Rust player, with domain allowlists and action
policies (governance-lite). But it copies your Chrome profile to a temporary snapshot: a
fresh browser, not your live session, with no extension, no identity layer, and no
audit. Good for sandboxed tasks; not for "act as me in my real tabs, governed."

**browsermcp.io ("Browser MCP", ~7k stars)** -- the namesake. Extension-driven real
session, but stale (last push April 2025) and the extension itself is closed source;
only the npm server is open.

**Google chrome-devtools-mcp** (~45k stars) -- debugging and inspection altitude; can
attach to a running Chrome but defaults to a dedicated profile, and its own docs warn it
exposes all browser data to the client. Different job.

**browser-use** (~102k stars) -- the biggest OSS "make the browser do things" framework.
Drives its own Playwright browser by default (real-profile and extension modes exist),
and its enterprise controls live in the paid cloud, not the local server. The one to
watch: if it ships local governance it becomes the most credible competitor.

**Generic MCP gateways** (Lasso, ToolHive, MintMCP, and others) -- real policy and audit
for ANY MCP tool as a proxy in front. A gateway sees opaque tool calls; it cannot
classify `computer(left_click)` versus `javascript_tool` by intrinsic capability, bind
grants to the tab's actual host at decision time, or filter the advertised tool set.
Fused governance can. (A gateway composes fine in front of Ghostlight if you already run
one.)

**Enterprise browsers** (Island, LayerX, Prisma Access Browser, and others) -- they
govern and audit agent activity inside the browser, credibly. They are closed SaaS,
several replace your browser outright, and they oversee agents rather than expose an
automation API to your own MCP client. Different deployment universe (and price class).

## The grid

| | Real session | Any MCP client | Governance + audit | Open + local, single binary |
|---|---|---|---|---|
| Ghostlight | yes | yes | yes | yes |
| Claude for Chrome | yes | no (Claude only) | yes | no (closed) |
| mcp-chrome | yes | yes | no | no (Node) |
| Playwright MCP (ext. mode) | yes | yes | no | no (Node) |
| agent-browser | no (profile copy) | yes | partial | yes (Rust) |
| browser-use | opt-in | yes | cloud-only | no (framework) |
| MCP gateways | n/a (proxy) | yes | partial (opaque calls) | varies |
| Enterprise browsers | yes | no | yes | no (closed SaaS) |

Star counts and activity are as of 2026-07 and will drift; the research note carries the
sources. Corrections welcome: hello@sylin.org.
