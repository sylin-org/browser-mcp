# 0034. The Capability & Transport Registry

- Status: Accepted
- Date: 2026-07-06

## Relationship to other decisions

- AMENDS ADR-0007 (sacred tool surface): the byte-frozen mandate is deprecated. The browser's
  tool declarations become the reference shape for the current capability, not a frozen contract.
  The fidelity test becomes a regression snapshot, not a drift-prevention contract. Future
  capabilities (shell, fs, desktop) add their own declarations additively.
- BUILDS ON ADR-0024 (one-loader pipeline): that decision already consolidated the per-tool
  directory (`directory::REGISTRY`) into the single enforcement/advertisement/audit authority.
  This ADR lifts that directory from a flat `browser::` constant into a per-capability trait,
  realizing the "plugin-manifest-like shape" ADR-0024's module doc explicitly anticipated ("so
  that a future sibling plugin can declare the same kind of table").
- BUILDS ON ADR-0033 (inbound/outbound/manage zones): that decision established the three-zone
  module tree. This ADR populates the zones with trait-bearing first-class modules and a
  composition-root registry that iterates them, replacing the hardcoded `ServiceContext.browser`
  field and the three special-case listener spawns in `run_service_loop`.
- PRESERVES ADR-0031 (agent onboarding contract): the per-tool examples and the corrective-error
  discipline stay; they migrate from `tools.json` into code declarations and become
  capability-scoped. The agentGuide composes each capability's guidance additively.
- CROSS-REFERENCES ADR-0030 (Hub orchestrator): the "matrix of input adapters × tool adapters
  through one governed chokepoint" vision (ADR-0030:34, :370) becomes real in the type system.
  The extension endpoint (`ipc::serve`) moves out of `run_service_loop`'s spawns and INTO the
  browser capability's `attach()` -- the browser's backend connection stops being transport
  plumbing.

## Context

Ghostlight today has one capability (browser) and two ingestion transports (pipe, web). The
codebase treats them asymmetrically:

- **One capability, hardcoded.** `ServiceContext.browser` is a special-case field, not one-of-many.
  `run_service_loop` hardcodes `Browser::with_debug(sink)`, the pipeline hardcodes
  `browser.call(name, &args)` at one dispatch site, and adding a second capability would mean
  editing the struct, the composition root, the pipeline, and the directory.
- **Two parallel tool authorities, bound only by mirror tests.** `tools.json` (the sacred schemas
  + agentGuide + examples, the advertisement authority) and `browser::directory::REGISTRY` (the
  RAWX requirements + dispatch kind + resource shape, the enforcement/explain authority) are two
  sources of truth in two different media. They cannot drift in a way the compiler catches; only
  the fidelity test catches drift, post-hoc.
- **The pipe transport is not a first-class zone module.** `inbound/web.rs` exists; `inbound/pipe.rs`
  does not. The pipe listener (`ipc::serve_adapters`, `handle_adapter_connection`) still lives in
  `transport/native/ipc.rs`, and the extension endpoint (`ipc::serve`) is spawned from
  `run_service_loop` as transport plumbing -- when it is in fact the browser capability's backend
  connection.
- **No transport/registry trait.** The "matrix" exists in ADR prose, not in the type system.
  Adding `inbound/cli` or `outbound/desktop` is "edit four files," not "implement the trait,
  register, done."

Meanwhile, the model-delight opportunity is real: a model connecting today gets a flat array of
N tools and reverse-engineers the system from names. A model connecting under the registry could
learn the *landscape* at handshake -- what capabilities exist, what each is for, how to use each
well -- organized by purpose, with targeted coaching. That self-describing surface is what makes
Ghostlight genuinely easier for any model (Claude, GLM, future ones) to drive well.

## Decision

Realize the matrix: discrete inbound transports and outbound capabilities as first-class,
trait-bearing zone modules behind one composition-root registry. Each capability owns its tool
declarations + RAWX requirements + agent guidance in code; the registry aggregates them.

### Decision 1: `ICapability` trait

A `Send + Sync` trait (out-of-process stays a future option; the trait's methods take
serializable inputs and return serializable outputs so a remote impl is a future redesign, not a
rewrite). Each capability owns:

- `code()` -- a stable identifier (`"browser"`).
- `descriptor()` -- a one-line human description for the manifest.
- `directory()` -- the slice of `ToolDeclaration`s this capability owns (name, inputSchema,
  description, RAWX `requires` per action, `example`, per-tool guidance).
- `agent_guide()` -- the capability's `CapabilityGuide` (summary, workflow hints, cost notes).
- `initialize()` -- sync construction from static declarations (cheap, fast).
- `attach()` -- async backend readiness (the browser waits for the extension link; a future fs
  capability's might wait for a path check). Not-ready is reported via the `CapabilityNotReady`
  ToolError, not held as a snapshot.
- `shutdown()` -- graceful teardown, symmetric with `initialize()`.

The browser capability's `attach()` absorbs the extension endpoint (`ipc::serve`) -- the
browser's backend connection stops being transport plumbing and becomes the capability's own
concern.

### Decision 2: `ITransport` (InboundChannel) trait

A blackbox that binds a listener, accepts connections, translates wire bytes into a session the
pipeline speaks, and stamps the call with its transport identity. Two impls today:
`inbound/pipe.rs` (the adapter/control endpoint, lifted out of `ipc.rs`), `inbound/web.rs`
(already discrete). The trait captures the common denominator: produce a `ServiceContext`, an
accepted `AsyncRead + AsyncWrite` stream, and a `SessionGuid`, then hand them to `serve_session`.
The pre-`serve_session` handshake differs per transport (the pipe carries a session-hello +
peer-cred + anti-squat proof; the web mints its own GUID and runs the WS upgrade) and stays in
each transport's module, not in the trait.

Shared registry/iterate-init pattern with capabilities, but a distinct contract -- a transport
owns a long-lived listener task; a capability owns backend state until the pipeline dispatches to
it. Two traits, not one.

### Decision 3: routing by registry lookup (bare names stay on the wire)

The wire keeps bare tool names (`navigate`, not `browser.navigate`) -- the trained names every
model has been against stay stable. The registry builds a `HashMap<&str, &dyn ICapability>` at
startup, keyed on each tool's declared name. A duplicate claim (two capabilities declaring the
same tool name) is a fail-closed startup error, never a silent misroute. The pipeline does an O(1)
lookup per call to find the owning capability, then dispatches.

### Decision 4: tool declarations in code, not JSON

Each tool is declared in code (`outbound/browser/tools.rs`), one declaration per tool, co-locating:
name, description, inputSchema (inline `json!`), RAWX `requires` per action, `example`, per-tool
guidance, and the existing directory fields (`action_key`, `ResourceShape`, `Handler`,
`postprocess`, `post_dispatch`). The declaration and the RAWX requirement live in the same place,
type-checked by the compiler, impossible to drift. `tools.json` deletes; `transport/mcp/schemas/`
deletes. The aggregated renderer produces the MCP `tools/list` response from the registry's
composition; the fidelity test becomes a regression snapshot, not a drift-prevention contract.

The choice of inline `json!` over a typed schema builder or a proc-macro is deliberate: the tool
set is small (14 today), changes rarely, and inline JSON-Schema is the target wire format (no DSL
to maintain, no escape hatches). A proc-macro that harvests function signatures would be the
right call at 50 tools across 5 capabilities; at 14 across 1, it is premature infrastructure.

### Decision 5: aggregated directory

`Registry::aggregated_directory()` composes every capability's declared slice into one logical
directory. `tools/list`, `explain`, the enforcement `requires` lookup, and the schema validator
all consume the aggregation. One source, no drift. `explain` becomes capability-aware: it renders
the union with per-capability descriptors, so a model that gets denied on a future `exec` learns
the shell capability's posture, not a generic denial.

### Decision 6: capability manifest at MCP handshake

`initialize.result.capabilities` (additive, alongside the existing `instructions`): an array of
per-capability entries, each `{ code, descriptor, tools[], guidance }`. This is the model-delight
lever -- a model learns the *landscape* at handshake, organized by purpose, with each capability's
tools and coaching co-located. **No status field** -- readiness is a runtime fact (the browser can
lose connection mid-session), reported via the `CapabilityNotReady` ToolError, not a snapshot that
can lie. The scoped agentGuide composes each capability's guidance additively into
`initialize.instructions`, so adding a capability automatically enriches the onboarding.

### Decision 7: deprecate ADR-0007's byte-frozen mandate

The browser's tool declarations become the reference shape for the current capability, not a
frozen contract. The sacred-surface discipline made sense for Claude-Cowork parity; it makes less
sense for a multi-capability, multi-model world. ADR-0007 is amended to "reference, not contract":
the fidelity test still pins structural invariants (name order, enum order, agentGuide fields,
example validity) for regression visibility, but it no longer forbids growth. Future capabilities
add their declarations additively to the aggregated directory.

### Decision 8: audit gains `transport` and `capability_origin` fields

Additive fields (after `held`, preserving existing byte-order for old-record compatibility). The
existing `capability` field (the ADR-0022 RAWX primitive: read/action/write/execute/none) stays
unchanged; the new field is named `capability_origin` to avoid collision. The honest job shape
becomes: `transport: "web", capability_origin: "browser", tool: "navigate"`. This improves
observability and makes `policy simulate` replay-faithful across transports.

### Decision 9: `CapabilityNotReady` ToolError

A first-class `ToolError` variant with a `next_step` (reusing ADR-0031's corrective-error
discipline): "the browser capability isn't connected -- run `ghostlight doctor`, or retry once
Chrome is open." Covers both the initial-attach race and the lost-connection-mid-session case with
one consistent signal. Not a queue-on-not-ready (that changes call semantics and complicates the
honest-singleton-queue story); the model retries on its own schedule.

### Decision 10: extension endpoint absorbs into the browser capability

`ipc::serve` (the listener that Chrome's native-host connects to) moves out of
`run_service_loop`'s spawn block and INTO the browser capability's `attach()`. The pipe
transport's "dual role" dissolves: `inbound/pipe.rs` is the ingestion adapter (thin MCP adapters
dial in → pipeline); the browser's backend connection (the extension link) is the browser
capability's own concern. `run_service_loop`'s three special-case spawns become an iterate-over-
transports loop plus the capability registry's `attach()` calls.

## Consequences

### Fixed

- The matrix is real in the type system: `ICapability` and `ITransport` are first-class traits;
  adding a capability or transport is "implement the trait, register at the composition root,"
  not "edit four files."
- One source of truth per tool: the declaration and the RAWX requirement live together, type-
  checked, impossible to drift. The mirror tests delete; the fidelity test becomes a snapshot.
- The model sees a self-describing surface at handshake: the capability manifest + scoped
  agentGuide, organized by purpose, with targeted coaching per capability.
- The pipe transport becomes a first-class zone module (`inbound/pipe.rs`), symmetric with
  `inbound/web.rs`.
- The extension endpoint stops being transport plumbing -- it lives where it belongs, in the
  browser capability.
- Honest job shape in the audit record (transport + capability_origin + tool).

### Cost

- `tools.json` deletes; the declarations migrate to code. The fidelity test is reworked from a
  byte-contract into a regression snapshot. ADR-0007 is amended.
- `ServiceContext.browser` becomes a registry; the dispatch site routes through a lookup. Every
  consumer of `ctx.browser` updates.
- `ipc.rs` loses its listener functions (they move to `inbound/pipe.rs`); it keeps the wire-level
  primitives. The platform-split (`#[cfg(windows)]` / `#[cfg(unix)]`) moves with the listeners.
- The extension endpoint spawn moves into `Browser::attach`; the spawn's lifecycle (idle-grace
  interaction, error handling) is re-homed.

### Preserved invariants

- All-open output-identity: a lone all-open session's client-visible output stays byte-identical.
- The native-messaging wire and the extension-facing contract.
- The governance core boundary (a7 arch test).
- The dispatch stage order (pipeline.rs pins).
- No behavioral gating: the registry changes structure, not what a session is allowed to do.

## Open questions (deferred)

- A formal capability/transport discovery mechanism (inventory/linkme macros) -- explicitly NOT
  taken; the composition root is explicit (`Vec<Box<dyn ...>>` built in dependency order).
- A typed schema builder DSL or a proc-macro that harvests function signatures -- deferred until
  the tool count grows 5× and the inline-JSON duplication actually hurts.
- Cross-capability orchestration hints -- explicitly NOT taken; each capability's guidance stays
  internal; cross-capability flows are the model's job.
- Queue-on-not-ready -- explicitly NOT taken; the honest not-ready error with a `next_step` is v1.
