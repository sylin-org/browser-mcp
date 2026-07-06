# 0033. Inbound/outbound/manage zones: the honest SoC split

- Status: Accepted
- Date: 2026-07-05

## Relationship to other decisions

- AMENDS ADR-0030 Decision 9 (web API transport) and Decision 5 (authorization is policy):
  the management plane and the web ingestion adapter are separated into two bounded contexts
  with two capabilities; the grant axis `channels` is renamed `inbound`; listener enablement
  becomes a policy-controlled decision, finally implementing Decision 5's "deny the web adapter."
- AMENDS ADR-0030 Decision 2 (the `src/hub` composition root): the composition root stays, but
  its module tree is reorganized into three zones (`inbound/`, `outbound/`, `manage/`) so the
  inbound/outbound symmetry and the management-plane bypass are legible in the code.
- BUILDS ON ADR-0024 (one-loader pipeline): the pipeline (`transport::mcp::pipeline`) remains
  the single governance chokepoint; this ADR does not touch it. Every ingestion adapter still
  converges on `serve_session`.
- PRESERVES ADR-0007 (sacred tool surface) and every byte-frozen invariant in ADR-0030's
  "Preserved invariants" section. The 13 trained tool schemas, the native-messaging wire, the
  extension-facing contract, and all-open output-identity are untouched.
- CROSS-REFERENCED from ADR-0032 (test at pure seams): the truncation failure that exposed this
  was a symptom; the pure-payload-builder tests already land in ADR-0032's direction and are not
  re-litigated here.
- CASHES the first slice of the deferred grant grammar (ADR-0030 "Governance schema section"):
  `inbound` becomes a real capability axis (allow/deny per member), not just a flat allowlist.

## Context

ADR-0030 framed the family vision as "a matrix of input adapters (MCP-stdio, a local web API)
times tool adapters (browser now; shell, filesystem, network later) through one governed
chokepoint." Decision 9 then realized the local web API and the Console together: a second TCP
session source that reuses `serve_session`, plus a loopback static site "served from the same
HTTP stack." Decision 5 promised "Enterprise pushes an org-mandatory layer that… denies the web
adapter."

The implementation did not realize the matrix; it conflated it. Concretely, as verified in the
current source:

1. **Two bounded contexts share one module, one listener, one gate.** `src/hub/webapi.rs` holds
   both the WebSocket/JSON-RPC tool-ingestion path (which calls `serve_session` at line 278 —
   a true ingestion adapter) AND the Console (`route_console_request` at line 331, which reads
   the `ConfigStore` directly and performs one admin write; it never calls `serve_session`).
   These are different capabilities: the web adapter is a data plane (tool-call ingestion); the
   Console is a management plane (observe runtime + administer policy). They share one
   `TcpListener::bind`, one `channels.webapi.from` gate used by both the WS path and the Console
   router, and one routing function.

2. **There is no Adapter abstraction.** The "matrix of input adapters" exists in ADR prose, not
   in the type system. The two ingestion paths are ad hoc: `ipc::serve_adapters` on the pipe side
   and `webapi::run` on the HTTP side, with no shared `InputAdapter` type and no symmetry.

3. **Listener enablement is not policy-controlled.** `run_service_loop` does
   `tokio::spawn(webapi::run(ctx.clone()))` unconditionally (`src/hub/mod.rs:290`). `run()` only
   skips binding if the port itself is taken (`webapi.rs:145`). `resolve_bind` chooses the
   loopback-vs-remote *address*; it has no "do not bind" branch. So Decision 5's "deny the web
   adapter" is unimplementable today — an org-mandatory layer can narrow WHO connects, never
   WHETHER the listener binds.

4. **One key conflates three questions.** `channels.webapi.from` answers "is the listener on?"
   (implicitly: always), "who may connect?" (the allowlist), and "is the Console reachable?"
   (it rides the same key). Three distinct decisions are fused under one label, and the
   overloaded vocabulary (`channels` the axis vs `webapi` the config prefix vs `Console` the
   code) drifts against itself across the grammar, the config registry, and the modules.

A truncation failure in the Console's static-route test surfaced this in CI: a request received
`200 OK` + `text/html` + the early body (`/console.css`, in `<head>`) but not the tail
(`/console.js`, before `</body>`) — the server closed the connection before the full body drained.
That is a real flush/shutdown bug in the shared HTTP path, but it is a *symptom*: every
service-spawning test drags in the always-on, conflated listener and exercises this fragile path.
The root cause is the SoC break, not a missing retry.

## Decision

Split the conflated surface into three zones, each a first-class bounded context, and rename the
grant axis to match the code. The pipeline (governance + dispatch) is untouched.

### Decision 1: three zones — `inbound`, `outbound`, `manage`

```
inbound/    INGESTORS. Per-channel translators: wire/transport → native tool-call.
            Converge on the pipeline (serve_session). Two today: pipe, web.
outbound/   EXECUTORS. Per-capability translators: native tool-call → backend commands.
            One today: browser (→ chromium DevTools). Desktop, shell, fs are future.
manage/     OPERATOR SURFACE. Observe + diagnose + administer the running service.
            Multiple deliveries (web, cli, instrumentation). NEVER flows through the
            pipeline; reads ConfigStore/audit/state directly. Loopback-only by construction.
```

The inbound/outbound mirror (per-X translators facing the pipeline from opposite ends) is the
matrix made real in the type system and the directory tree. `manage` is a peer top-level, NOT
under `inbound`, because it does not ingest tool calls — making its bypass of the pipeline
explicit and honest in the names.

### Decision 2: the `manage` plane is the whole operator surface

`manage` is not just the web UI. It is the bounded context for everything the operator does to
observe, diagnose, and administer the running service, regardless of delivery:

- **`manage.web`** — in-service loopback HTTP UI (config view, sessions view, enable-remote
  write). Today's Console half of `webapi.rs`.
- **`manage.cli`** — `ghostlight doctor`, `ghostlight status`. These read live state and
  diagnose the chain; they never touch the pipeline.
- **`manage.instrumentation`** — the debug sink, event log, metrics read paths that the
  operator inspects. (Mechanism within `manage`, not a peer context: doctor/status *read* the
  sink; they are not alongside it.)

This is a real expansion of scope versus ADR-0030 Decision 9 (which modeled only the embedded
Console). Folding `doctor`/`status` under `manage/` is a coherence win; per the migration plan it
lands in a later phase, because the urgent break is the inbound/manage-web SoC split.

### Decision 3: `manage` is permanently loopback; `inbound.web` is remote-optional

This asymmetry — same transport (http), opposite remote posture — is exactly what the current
fused design makes invisible, and the names must make it obvious:

- **`inbound.web`** is remote-**optional**. A web MCP client on another machine driving the
  browser is a legitimate case the operator opts into (writes `inbound.web.from: ["*"]`),
  per ADR-0030 Decision 5. Loopback + anonymous by default; remote is a deliberate user choice.
- **`manage.web`** is permanently loopback. There is no legitimate case for administering
  Ghostlight remotely — remote policy changes to a service driving an authenticated browser
  session is a security non-starter. `manage.web.from` is locked to localhost, full stop: not
  "default localhost," *fixed* localhost. An org manifest can disable the plane
  (`manage.web.enabled = false`); it can never widen it.

### Decision 4: the grant axis `channels` is renamed `inbound`

`channels` is vague and overloaded; the thing the axis gates is *which transport a request
arrived on*. But "transport" collides with the existing `src/transport/` MCP-protocol module.
The clean resolution is to make the grant axis, the config prefix, and the code zone the SAME
word — `inbound` — so the three layers stop drifting against each other (that drift is the
original sin that produced the conflation). The grammar's recursive `AxisNode` shape is
unchanged; only the axis name and member vocabulary change:

```
grant := { id, inbound: AxisNode, tools: AxisNode }   // inbound members refine with `from`
                                                       // tools members refine with on/except + do
```

The full recursive federated grammar remains deferred to its own core-only ADR; this ADR realizes
the same minimal flat allowlist slice ADR-0030 did, under the honest name. The management plane
gains a parallel `manage` axis (one realized selector today: `manage.web`), separately denyable.

### Decision 5: config keys mirror the code, 1:1

Each question — "is it on?", "who connects?" — gets its own key. No more three-questions-one-key
conflation. One key per adapter per question:

```
inbound.pipe.enabled              bool, default true   (pipe authz is OS same-user; primary path)
inbound.web.enabled               bool, default true   (local-on; ADR-0030 D5 "open means open")
inbound.web.from                  ["localhost"]        (remote blocked by default-policy)
outbound.browser.enabled          bool, default true
manage.web.enabled                bool, default true   (management UI works out of the box)
manage.web.from                   ["localhost"] LOCKED (never remote; not user-widenable)
```

Defaults preserve ADR-0030 Decision 5 verbatim: OPEN MEANS OPEN, the channel/inbound axis
resolves to the adapter's builtin default (web: loopback). The web adapter's builtin fragment is
`inbound.web.from: [allow: "localhost"]`, on by default. Remote is blocked by default-policy;
the operator widens it deliberately.

This is a **breaking rename of the user-facing key** `channels.webapi.from` → `inbound.web.from`.
Given pre-1.0 and the explicit break-and-rebuild mandate, this is intentional: the old name is
one of the things lying about the architecture.

### Decision 6: the module tree mirrors the zones, 1:1

```
src/hub/
  inbound/
    mod.rs     the InboundChannel convergence (all ingestors → pipeline). The matrix made real.
    pipe.rs    the pipe adapter (today's ipc::serve_adapters path, relocated)
    web.rs     the WS/JSON-RPC tool adapter (today's webapi.rs WS half: handshake, WsStream,
               channels decision, → serve_session)
  outbound/
    browser.rs today's transport::executor::Browser (→ chromium DevTools), relocated
  manage/
    web.rs     today's webapi.rs Console half (route_console_request, /api/v1/*, enable-remote),
               with its own capability and loopback-locked routing context
    assets/    index.html, manage.css, manage.js (renamed from console.*)
    (cli.rs, instrumentation.rs land in a later phase)
  pipeline.rs   already exists as transport::mcp::pipeline — the convergence point (unchanged)
```

`webapi.rs` **deletes**; its two halves move to their actual homes. That deletion IS the SoC fix
— the file cannot exist in the honest naming because it conflated two bounded contexts.

### Decision 7: one loopback listener, two gated routing contexts

For the two HTTP surfaces (`inbound.web` and `manage.web`), the implementation uses ONE loopback
`TcpListener` with TWO separately-gated routing contexts, rather than two physical ports. The
separation lives in the capability + routing layer where it belongs (and where the policy keys
already express it); a second TCP port buys little for two local surfaces. The first byte read on
each accepted connection classifies it (WS-upgrade attempt → `inbound.web`; otherwise, if it
matches a management route → `manage.web`), then each side runs its own policy decision
(`inbound.web.enabled`/`from` or `manage.web.enabled`/`from`) and its own routing.

This preserves the existing router partition (today's `is_ws_attempt` check ahead of the Console
router), but with the gating split so "deny the web adapter" and "deny the management plane"
become independently implementable.

### Decision 8: listener enablement is policy-controlled (the "deny the adapter" case)

`run_service_loop` consults the resolved policy for each adapter's `enabled` key BEFORE spawning
its listener. `inbound.web.enabled = false` (set by an org-mandatory lock) means the WS listener
never binds — the inbound.web adapter does not exist for that deployment. `manage.web.enabled =
false` means the management UI is unreachable. This finally implements ADR-0030 Decision 5's
"Enterprise pushes an org-mandatory layer that… denies the web adapter," which today is
unimplementable.

Pipe authz stays OS same-user (no `from` allowlist — the pipe has no hostname axis).

## Consequences

### Fixed

- Two clean bounded contexts (ingestion data plane vs management plane) instead of one conflated
  `webapi.rs`. The SoC break that ADR-0030 Decision 9 baked in is corrected.
- Adapters as first-class ports through one chokepoint — the "matrix of input adapters" is real
  in the type system, not prose.
- Policy-controlled listeners — Decision 5's "deny the web adapter" actually works.
- The management plane and the web ingestion adapter are separately enableable and separately
  authz'd — ADR-0030's implicit single-gate fusion is broken.
- The truncation CI failure largely evaporates as a side effect: service-spawning tests that do
  not enable an adapter do not run its listener, so the fragile shared HTTP path is no longer
  exercised by ~every test.
- Vocabulary coherence: grant axis (`inbound`), config prefix (`inbound.*`), and code zone
  (`src/hub/inbound/`) are the same word across all three layers.

### Cost

- **Breaking config rename**: `channels.webapi.from` → `inbound.web.from`. Existing user/org
  policy files referencing the old key must be updated. Acceptable pre-1.0; called out in the
  release notes.
- **Module relocation**: `transport::executor::Browser` → `outbound::browser`; `webapi.rs`
  splits and deletes. No shims, no re-exports — clean breaks, by mandate.
- **Test migration**: ~4 test files move with `manage/web.rs` (the `console_*` tests); ~4 take a
  one-line `use` update for the Browser move; ~3 take literal string edits for the key rename.
  See the migration plan.
- The deferred recursive grant grammar stays deferred. This ADR realizes only the minimal
  `inbound` allowlist slice (renamed) plus the first `manage` selector.

## Preserved invariants

The same sacred set ADR-0030 names: the 13 trained tool schemas (byte-frozen, pinned by
`tool_schema_fidelity`), the native-messaging wire, the extension-facing contract (the
server-speaks-first ordering, the hello-free extension endpoint), and all-open output-identity.
A lone all-open session's client-visible output stays byte-identical. None of these touch the
adapter/transport split this ADR introduces; the pipeline (`serve_session`) is unchanged.

The `a7` architecture test's governance-core boundary (no `browser`/`transport`/`mcp`/`native`/
`tabId`/`token`/`socket` in `src/governance/**`) is preserved: `channels.rs` (renamed to carry
the `inbound` axis) stays in `src/governance/`, transport-agnostic, exactly as today. The
transport-specific adapters live in `src/hub/inbound/` and `src/hub/manage/`.

## Open questions (deferred)

- The recursive `inbound`/`manage` grammar (allow/deny per member, federated refinements) and
  per-verb channel grants (RAWX over the inbound axis) remain deferred to a later core-only ADR.
- A second TCP port for the management plane (hard physical isolation) is explicitly NOT taken
  here; revisit if defense-in-depth demands it.
- `outbound.desktop` (and shell/fs/network capabilities) are future; this ADR only relocates
  `outbound.browser`.
