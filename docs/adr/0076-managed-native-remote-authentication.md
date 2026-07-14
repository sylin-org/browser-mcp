# ADR-0076: Managed native remote authentication

Date: 2026-07-14
Status: Superseded by ADR-0077
Builds on: ADR-0028, ADR-0033, ADR-0034, ADR-0055

## Context

Ghostlight's `inbound.web` adapter is a local HTTP/1.1 WebSocket ingestion path. Its management
action for broadening `inbound.web.from` is currently disabled: the former behavior could expose a
plaintext, unauthenticated browser-automation service on all interfaces. Loopback plus a user-owned
SSH, WireGuard, or Tailscale tunnel is the current remote-access answer.

SEC-HIGH-02 asks what would be required before native non-loopback access could return. The answer
is larger than adding a static bearer string. A remote browser controller needs transport security,
verified user and client identity, resource-bound least privilege, session binding, replay
resistance, safe key discovery, and fail-closed startup. Authentication must remain separate from
Ghostlight governance: a valid org identity does not imply permission to read, write, or execute in
the browser.

The current MCP authorization specification defines an OAuth resource-server profile for
HTTP-based transports, including protected-resource discovery, Authorization Code plus PKCE,
resource indicators, audience validation, per-request tokens, and least-privilege scopes. The
existing Ghostlight WebSocket transport predates that profile and is not Streamable HTTP. A native
remote feature should adopt the standard transport and authorization boundary rather than extend a
local compatibility adapter into a second security protocol.

## Proposed decision

### 1. Loopback plus tunnel remains the personal remote architecture

The default service stays loopback-only and network-silent. Personal, all-open, user-file, and
user-env configurations cannot enable a native non-loopback listener. Users reach the local service
through a tunnel whose operator already owns authentication, encryption, device enrollment, and
revocation.

This is a complete supported architecture, not a temporary insecure fallback. It preserves the
ADR-0028 Continuity Promise and avoids a vendor identity service.

### 2. Native remote is managed-only and remains disabled until complete

A native remote listener may exist only when a valid ADR-0055 signed managed bundle supplies the
entire remote-auth profile. At minimum it names:

- the canonical HTTPS resource URI;
- allowed authorization-server issuer(s);
- required scopes and principal rules;
- TLS identity configuration;
- a sender-constrained-token method;
- bounded token and metadata lifetimes.

No individual config key, Console toggle, environment variable, wildcard source rule, or command
line flag can bypass those requirements. The current `enable-remote` action continues to return 403
and write nothing until an accepted implementation proves the full profile. There is no anonymous,
plaintext, bearer-only, or fail-open non-loopback mode.

### 3. Native remote uses a separate standards-conformant HTTP transport

The local WebSocket adapter remains local compatibility infrastructure. It is never exposed to a
non-loopback peer.

If built, native remote is a separate transport registered through ADR-0034 and conforming to the
then-current MCP Streamable HTTP and authorization specifications. Ghostlight acts only as the
OAuth protected resource/resource server. The organization's authorization server handles user
login, consent, client registration, and token issuance. Ghostlight does not become an IdP, store
passwords, or operate a vendor control plane.

The resource publishes OAuth Protected Resource Metadata and points clients at the org's approved
authorization server. Clients use the standard MCP authorization flow with Authorization Code and
PKCE. Device Authorization Grant is not a Ghostlight-specific fallback; it may be used only if a
future MCP authorization profile and the org authorization server both advertise it. URL
elicitation is not used to authenticate an MCP client to Ghostlight because the MCP specification
assigns that job to MCP authorization.

### 4. TLS and sender constraint are mandatory

Every non-loopback byte uses TLS. Plain HTTP is refused before MCP parsing. Certificate validation
cannot be disabled. The exact certificate-provisioning model and whether Ghostlight terminates TLS
directly or sits behind a mutually authenticated local sidecar must be pinned before acceptance; a
forwarded identity header from an ordinary reverse proxy is not sufficient.

Bearer tokens over TLS reduce passive interception but remain replayable when copied from a log,
memory dump, or compromised client. Native remote therefore requires a sender-constrained access
token. DPoP is the preferred interoperable direction; mTLS-bound tokens are acceptable for an org
that controls client certificates. A plain bearer token alone is insufficient for this high-impact
surface.

The sender constraint must be carried by a standardized MCP authorization extension supported by
the selected clients. Ghostlight does not invent a private DPoP variant around the core Bearer
profile.

If supported Ghostlight clients cannot present DPoP proofs or mTLS-bound tokens, native remote does
not ship. The tunnel architecture remains available.

### 5. Validate every request as a resource server

Authorization is present and verified on every HTTP request. Sessions are never authentication
credentials. A session is bound to the verified principal and sender key and cannot be resumed by a
different principal or key.

Ghostlight validates at least:

- TLS and sender proof;
- token signature against an allowed algorithm and current org key;
- exact issuer;
- exact audience/resource for this Ghostlight endpoint;
- expiry, not-before, and issued-at with a small bounded clock skew;
- stable subject or approved workload principal;
- required scope;
- managed policy generation and remote-listener authorization.

Invalid, expired, unknown-key, wrong-audience, wrong-sender, or missing tokens receive 401. A valid
token with insufficient scope receives 403. Tokens received from clients are never passed to the
browser, extension, audit sink, policy endpoint, or another upstream service.

Authentication answers who is calling. OAuth scopes set an outer capability ceiling. Existing
Ghostlight policy then independently decides what that principal's session may do. Both must allow:

```text
TLS + sender proof
  AND verified org principal
  AND token scope ceiling
  AND managed/org/user/session governance
  AND ordinary ownership, sacred, pause, and panic checks
```

The initial scope vocabulary should reuse Ghostlight's capability model (`read`, `action`, `write`,
`execute`) plus the minimum connection/discovery scope required by MCP. Scopes are additive grants,
not an implication hierarchy, unless a later schema explicitly says otherwise. The authorization
server should issue the smallest initial set and use MCP's insufficient-scope challenge for step-up.

### 6. Prefer local validation with bounded public-key caching

The first design validates signed, short-lived access tokens locally. Ghostlight discovers org
authorization metadata and JWKS from only the issuer pinned by signed managed policy. Redirects,
scheme changes, private-address pivots, unexpected hosts, and unapproved issuers are refused.

Verified public metadata and keys may be cached with their HTTP freshness bounds so an existing
short-lived token can survive a brief IdP outage. This is not ADR-0055 last-known-good
authorization:

- an expired token never becomes valid because the IdP is unavailable;
- an unknown signing key fails closed;
- stale metadata cannot add an issuer, algorithm, audience, or scope;
- no cache can extend a token lifetime.

Online token introspection is deferred. It adds a network dependency to every admission and makes
availability behavior harder to reason about. Short access-token lifetime bounds revocation delay;
the acceptable maximum and any emergency revocation mechanism must be decided with real IdP and
client evidence before acceptance. Ghostlight never stores refresh tokens.

### 7. Identity and privacy are minimized

The remote principal is an org-authorized human or workload represented by verified `iss` plus
`sub`, never an email or name supplied by the client. Policy may map that stable principal to local
roles. Authentication and authorization state is attached to the session from the token, not from
MCP `clientInfo`.

Raw access tokens, authorization codes, DPoP private keys, cookies, and full claims are never
logged, audited, written to debug state, placed in crash fixtures, or persisted by Ghostlight.
Request buffers containing credentials are bounded and zeroed where practical after validation.
Audit records use a stable local pseudonymous principal id derived with a machine-held key, plus
issuer id, scope decision, and failure class. They omit token values, email, display name, group
lists, and unneeded claims.

The authorization server and managed policy endpoint are org-configured destinations. No request
goes to Sylin or any developer-operated service.

### 8. Binding and startup fail closed

The non-loopback socket binds only after all static requirements validate: signed managed policy,
TLS identity, issuer pin, protected-resource metadata, scope map, sender-constraint mode, and safe
limits. A later policy expiry, removal, or invalidation stops accepting new connections and closes
remote sessions according to a bounded drain rule. Loopback stdio and browser relay operation remain
available under their ordinary policy.

Host and Origin checks, request-size limits, rate limits, connection caps, timeouts, and DNS-rebind
defenses remain required defense in depth. They do not substitute for authentication.

### 9. Verification and acceptance gates

This ADR remains Proposed until all of the following are complete:

1. A transport ADR pins the exact Streamable HTTP version and retirement or coexistence plan for
   the local WebSocket adapter.
2. At least two supported MCP clients complete Authorization Code plus PKCE against a representative
   enterprise IdP.
3. The same clients prove DPoP or another sender-constrained method; otherwise native remote stays
   disabled.
4. TLS termination, certificate rotation, reverse-proxy trust, and source-address semantics are
   pinned without trusting an unauthenticated forwarded header.
5. Token lifetime, JWKS caching, key rotation, outage, and emergency revocation behavior pass a
   threat review.
6. Lightbox covers missing/invalid/expired tokens, wrong issuer/audience/sender/scope, key rotation,
   policy expiry, reconnect, session fixation, rate limits, and token non-persistence.
7. A real network test proves no plaintext listener and no authentication bypass on any bind path.
8. Trust-center claims distinguish authentication, OAuth scope, governance policy, and audit.

## Rejected alternatives

- Re-enable `inbound.web.from: ["*"]` behind a warning. A warning is not an access control.
- Put a static bearer token in user config. It has poor identity, rotation, revocation, audience,
  and replay properties.
- Treat a VPN source address, `Origin`, `Host`, session id, or MCP `clientInfo` as identity. These
  are routing or presentation facts, not verified principals.
- Accept any token issued by the configured IdP. Tokens must be intended for this resource and
  bounded by scope and sender.
- Use URL elicitation as MCP client authentication. The MCP authorization profile already defines
  the correct boundary.
- Phone home to Sylin for token verification, licensing, discovery, or revocation.
- Mirror managed-policy last-known-good continuity by accepting expired authentication. Policy
  availability and caller freshness have different failure semantics.

## Consequences

- Personal remote use stays simple, private, and available through established tunnels.
- Managed native remote becomes standards-based and interoperable, but it is intentionally gated on
  real client support for strong token binding.
- A separate Streamable HTTP adapter is more work than adding auth to the old WebSocket handshake.
  It avoids making a local compatibility protocol into a permanent remote security surface.
- Brief IdP outages may preserve already-issued, still-valid sessions, but never extend token or
  policy validity.
- The design introduces no vendor-operated server and no new network behavior for default users.

## References

- MCP 2025-11-25 authorization:
  https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization
- MCP security best practices:
  https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices
- RFC 9700, Best Current Practice for OAuth 2.0 Security:
  https://www.rfc-editor.org/rfc/rfc9700
- RFC 9449, OAuth 2.0 Demonstrating Proof of Possession:
  https://www.rfc-editor.org/rfc/rfc9449
- RFC 9728, OAuth 2.0 Protected Resource Metadata:
  https://www.rfc-editor.org/rfc/rfc9728
- RFC 8252, OAuth 2.0 for Native Apps:
  https://www.rfc-editor.org/rfc/rfc8252
- RFC 8628, OAuth 2.0 Device Authorization Grant:
  https://www.rfc-editor.org/rfc/rfc8628
- ADR-0028: the Continuity Promise.
- ADR-0033: inbound, outbound, and management zones.
- ADR-0055: signed managed policy distribution.
