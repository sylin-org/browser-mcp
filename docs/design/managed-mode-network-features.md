# Design note: managed-mode network + identity features

Status: **partly superseded** (2026-07-14). This note preserves the design exploration for two SAPS
findings. ADR-0075 still owns SEC-HIGH-03. ADR-0077 closes SEC-HIGH-02 by removing browser-control
web ingress entirely and supersedes the proposed remote design in ADR-0076.

## The unifying frame

The Continuity Promise governs the **default personal posture**: all-open / personal manifests
never phone home, carry no telemetry, and make no network calls beyond the user's own tool calls
and configured audit destinations (ADR-0028 Decision 9).

**Managed mode is already the exception, by the organization's own choice.** `managed://`
(ADR-0055) has a device fetch a signed policy bundle from an org endpoint over the network. An org
that opts into managed mode has already accepted a network dependency and an identity-aware posture
for its own devices.

That makes managed mode the natural home for **any feature that needs the network or an identity** --
without touching the promise the personal default makes. Both designs below live there:

- **Org-declared confirm-required actions** (SEC-HIGH-03) -- an admin-authored map, in the signed
  policy.
- **Org-opt-in remote authentication** (SEC-HIGH-02) -- e.g. IdP / device-flow verification the org
  configures, parallel to how it already configures `managed://`.

Personal use keeps the offline, no-phone-home default; remote reach for personal use is a tunnel.

## SEC-HIGH-03 -- managed intent descriptors (org-declared confirm gates)

### Why this escapes the "don't infer intent" prior art

The security research (Anthropic computer-use guidance, OWASP MCP, Wiz 2025 review, browser-use,
Claude Code's permission model) is consistent: a tool below the model must **not** try to infer
whether an action is consequential -- intent-based and DOM/page-content heuristics are unreliable
and injection-evadable, and confirmation belongs in the client/model where the user's intent lives.

Managed intent descriptors do **not** infer. An **admin declares** the rule, in the **signed**
policy, targeting the org's **own known apps** (whose pages the org controls and whose selectors are
stable). That is the endorsed "capability + destination, human-authored, surfaced client-side"
pattern, extended to element granularity for known apps -- not the anti-pattern.

### Where the map lives (ranked)

1. **Signed managed-policy descriptor -- authoritative / load-bearing.** Central, admin-controlled,
   tamper-evident (rides the ADR-0055 signature), offline after fetch.
2. **Page-published hints -- advisory only, never load-bearing.** A site could publish its own
   sensitive-action markers, but the page is exactly what an attacker controls, so a page hint may
   only *raise* friction, never *remove* it. Trusting the page for a security gate is the precise
   anti-pattern.
3. Hybrid: policy authoritative, page hints additive-only. Likely landing spot.

### Descriptor shape (sketch)

Keyed on things that are stable on an org-controlled app and that injection cannot cheaply forge
there:

- `host`: pattern / path (e.g. `profile.company.com`, path `profile.aspx`).
- `element`: a **structural** selector (`#save`, `[name=send]`, `[data-testid=...]`) -- not visible
  text / ARIA-name, which is relabelable even on your own page. (Known limitation: on a fully
  attacker-controlled page a selector is weak; managed descriptors target org apps, so it holds.)
- `capability`: reuse the existing `write` / `execute` classification.
- `on`: the trigger (`click` / `submit`).
- `rationale`: human-readable reason shown to the operator ("this changes the user's saved data").
- `require`: `confirm` | `block` | `audit-only`.

### How it surfaces

MCP **elicitation** (client-side). On a descriptor match, Ghostlight emits an elicitation request to
the client carrying the `rationale` plus **structural** facts (host, matched selector, capability) --
never a page-derived summary (injection could poison that). The human confirms in the client, where
the intent lives. Opt-in, managed-mode only; all-open / personal stays untouched.

### Open questions

- Selector language + how brittle it is across an app's own releases; do we support a small stable
  set (id / name / data-testid / role) only?
- Do we need the page-hint tier at all in v1, or policy-only?
- Elicitation UX when the client does not support elicitation (fallback: block, or audit-only?).
- Relationship to `enforce` mode and the existing capability classification -- is `require: confirm`
  just a third outcome alongside allow/deny at the chokepoint?

## SEC-HIGH-02 -- remote access authentication (superseded)

### Final current decision

ADR-0077 removes `inbound.web`, its `enable-remote` action, and its supporting configuration and
policy surface. Ghostlight has no remote browser-control listener. The read-only Console remains
loopback-only and does not accept WebSocket upgrades. Any future remote capability starts from a
new threat model and ADR rather than reviving this design.

### Direction (under discussion)

- **Personal use: tunnel-first.** Keep the service loopback + owner-only; reach it remotely via SSH
  port-forward / Tailscale / WireGuard. Zero bespoke auth to maintain, no exposed port, encrypted by
  construction, on-brand for a minimal security-first binary (this is Tailscale's own guidance: bind
  localhost, let the network prove identity).
- **Managed use: org-opt-in identity.** An org that already runs `managed://` can accept an
  IdP-verified remote path (OAuth2 device flow / their IdP). Because the org already chose a
  network/identity posture, verifying a token against the org's IdP does **not** violate the
  personal default's no-phone-home promise -- it is the same class of org choice as `managed://`.

### Open questions

- **Who is the remote principal** -- the same user's other device, a teammate, or an org member?
  (Device-flow/IdP fits the org-member case; tunnel fits the same-user case.)
- **IdP verification model:** per-connection token introspection (a network call per connect) vs a
  short-lived verified session; offline/last-known-good behavior when the IdP is unreachable
  (mirror `managed://`'s last-known-good cache?).
- **mTLS** is stronger (mutual identity, no bearer-in-transit; Syncthing shows it working) but its
  cert-provisioning UX does not fit a solo minimal binary and MCP clients are not set up to present
  certs. Shelved unless enterprises demand it.
- Does native remote earn its keep at all if the tunnel story is strong enough to be the only
  personal answer and managed handles the org case?

## Cross-links

- Transaction-bound managed confirmation: ADR-0075.
- Local-only ingress and removal of the remote proposal: ADR-0077 (supersedes ADR-0076).
- Continuity Promise / no-phone-home: ADR-0028.
- Signed policy distribution: ADR-0055 (`managed://`), docs/design or the managed-scheme notes.
- Capability classification: the read/action/write/execute model (ADR-0022).
- The honest threat-model statement shipped for SEC-HIGH-03: `SECURITY.md` -> "What governance can
  and cannot stop".
