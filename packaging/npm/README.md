# ghostlight (npm launcher)

Governed browser automation over your own authenticated Chromium session, for AI coding
agents: any MCP client, your real logged-in browser, with capability grants, sacred domains,
and a structured audit trail. All-open by default; governance when you want it.

This npm package is a thin launcher: on first run it downloads the version-matched Ghostlight
executables from the GitHub release and caches them under `~/.ghostlight/bin/` (zero runtime
dependencies). Since ADR-0046 (as amended by ADR-0051 Phase 3) there are two: `ghostlight` (the CLI +
the persistent service) plus the single thin pass-through `ghostlight-relay`, which carries both
roles. A bare `npx ghostlight` runs `ghostlight-relay --role agent` (what your client relays
through); `npx ghostlight install` runs the CLI installer. Everything real lives in the binaries.

## Quick start

Add to any MCP client as a stdio server:

```json
{ "command": "npx", "args": ["-y", "ghostlight"] }
```

Then connect the browser side (once, idempotent):

```
npx ghostlight install
```

and add the "Ghostlight in Browser" extension from the Chrome Web Store. Full walkthrough,
one-click client buttons, and the manual paths:
https://sylin-org.github.io/ghostlight/install.html

## Links

- Project: https://github.com/sylin-org/ghostlight
- What it is and why: https://sylin-org.github.io/ghostlight/
- License: engine Apache-2.0 OR MIT; the governance module's source is readable under the
  Ghostlight Commercial License (see the repository's LICENSE for the split).
