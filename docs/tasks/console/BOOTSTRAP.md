# Ghostlight Console batch: BOOTSTRAP

Ground rules for the executor implementing "the Console" (ADR-0030 Decision 9). Assume ZERO
conversational context survives to you. Follow instructions literally; resolve nothing by
judgment. Read this file fully before touching any code.

## What you are building

The Console is a loopback-pinned static site, embedded in the `ghostlight` binary and served from
the SAME HTTP stack the local web API already runs (`src/hub/webapi.rs`, landed by the Hub batch's
H8, commit `af1d0f8`). It is a READ-MOSTLY operational view: live sessions, a provenance-aware
config view (per key: value, which of the five ADR-0019 layers set it, whether an org-mandatory
lock renders it read-only), and the single "Enable remote connections" write action that flips the
user-layer `channels.webapi.from` policy open. You implement it in five tasks, K1 through K5, one
task = one commit.

The Console is NOT a manifest editor, NOT a fleet/multi-machine control plane, and does NOT ship
token mint/revoke in this batch (see "Reconciliation and non-goals" below and NEVER-touch).

## Reconciliation and non-goals (read before K1)

Two ADRs were amended 2026-07-05 to reconcile ADR-0030 Decision 9 with an earlier "no web console"
non-goal:

- `docs/adr/0020-org-policy-experience.md`'s "Amendment (2026-07-05, ADR-0030)" section: that ADR's
  "no web console, no remote policy service, no SaaS control plane" line was about the
  ORGANIZATION policy experience specifically (authoring/deploying org policy). It stays in force
  UNCHANGED for what it actually rejected. The Console is categorically different: local not
  remote (loopback-pinned; remote only if the machine owner deliberately flips the one policy key,
  and an org-mandatory lock on that key renders the control read-only and shuts remote down
  immediately), a VIEW not an authoring surface (renders the already-resolved effective
  value/layer/lock per key, the `chrome://policy` analog; never a manifest editor; cannot write,
  edit, or push a manifest, mandatory layer, or org-recommended default), and single-machine not a
  control plane (no fleets, no deployment, no cross-machine state).
- `docs/adr/0019-layered-configuration-model.md`'s matching amendment: this is the anticipated
  revisit its own Decision 5 named as the precondition ("if the product family needs a shared local
  dashboard"). The Console renders exactly the effective-value/source/lock data Decision 2 already
  models; `config list | get | set` remains the source of truth, the Console renders the SAME
  registry, never a second one.
- Both amendments end with: "Follow-up: `docs/tasks/console/` (a task batch in the same
  BOOTSTRAP/LEDGER/PINS shape as `docs/tasks/hub/`) implements the Console against ADR-0030
  Decision 9's description." This batch is that follow-up.

The load-bearing distinction for every task in this batch: the Console is LOCAL-ONLY, READ-MOSTLY,
and NEVER authors or deploys organization policy. If a task would make the Console write anything
other than the single "enable remote connections" user-layer key, or read/render anything
resembling manifest-grant authoring, STOP (see Failure protocol) rather than build it.

## Authority order (higher wins on conflict)

1. `docs/adr/0030-ghostlight-hub-orchestrator.md` Decision 9 (and its "Governance schema section",
   "Preserved invariants", and the ADR-0019/0020 amendments above) -- the NORMATIVE design. Cite
   it; never restate or re-decide its semantics.
2. This BOOTSTRAP -- ground rules and procedure.
3. `docs/tasks/console/PINS.md` -- every pinned value (routes, JSON shapes, function signatures,
   constants) this batch's tasks need. The task files CITE PINS.md sections (`CS<n>`); they do not
   re-derive the values.
4. The per-task file `docs/tasks/console/K<N>-<slug>.md`.
5. The LIVE TREE. Every task file records facts as-of-authoring (2026-07-05). RE-READ the named
   files before relying on any line number or signature. If the tree contradicts a task's
   load-bearing assumption, follow that task's STOP precondition (see Failure protocol); do NOT
   improvise around it.

The Hub batch's own "Preserved invariants" (ADR-0030) and the NEVER-touch list below OVERRIDE
everything.

## Environment facts

- Rust stable, one Cargo workspace, single portable binary `ghostlight`, zero runtime deps, no
  dylib. This batch adds NO new Cargo dependency (static assets are embedded via `include_str!`,
  matching the existing `TOOLS_JSON` convention in `src/transport/mcp/tools.rs` and CLAUDE.md's
  "define ... as const string literals ... rather than building them programmatically" rule; JSON
  responses are built with the already-present `serde_json`).
- Work on the `dev` branch. One task = one commit. Confirmed by `git log --oneline -20` in this
  repo (2026-07-05): the Hub batch used a TWO-commit-per-task convention -- a `feat(hub): H<N> ...`
  commit landing the code, followed by a separate `docs(hub): record H<N> commit hash` commit
  updating `LEDGER.md` with the just-landed hash. Follow the SAME two-commit convention here:
  `feat(console): K<N> <short title>` (code + tests), then `docs(console): record K<N> commit
  hash` (the LEDGER update, step 8 below).
- Code is ASCII only; docs use no em-dashes.
- Verification commands (a task is not done until all four pass):
  - `cargo build --all-targets`
  - `cargo test` (plus the task's specific new test targets)
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`

## Task sequence (linear; do them in order)

K1 config + session read accessors and the shared config-write function (no HTTP, no UI) ->
K2 Console static GET routes wired into `src/hub/webapi.rs` (the page shell; no data yet) ->
K3 `GET /api/v1/config` + the config table in the UI ->
K4 `GET /api/v1/sessions` + the sessions section in the UI ->
K5 `POST /api/v1/config/webapi-enable-remote` (the one write action) + its UI control.

Dependencies, also encoded as STOP preconditions: K1 before K2 (K2's routing needs nothing from
K1, but K3/K4/K5 do, and landing K1 first keeps every prefix of the batch coherent); K2 before
K3/K4/K5 (they extend K2's route table and page shell); K1 before K5 (K5 reuses K1's extracted
write function and the `config_changed` audit call). K3 and K4 are independent of each other and
may be done in either order, but this batch lists them K3-then-K4; do not skip ahead.

## Per-task procedure

For each K<N>:

1. Read `docs/tasks/console/K<N>-<slug>.md` fully, and the PINS.md sections (`CS<n>`) and
   ADR-0030 sections it cites.
2. RE-READ every source file the task names. Verify each as-of-authoring fact. If any STOP
   precondition's assumption is absent, STOP (Failure protocol).
3. Write the named tests FIRST (RED). Transcribe every pinned assertion / JSON shape / constant
   from `PINS.md` verbatim -- never derive an expected value. If the task marks a value "AUTHOR
   MUST PIN" and it is still unpinned in `PINS.md`, STOP.
4. Implement to GREEN with the minimum change the task describes; keep the change inside the files
   the task names.
5. Run the full verification block (Environment facts, above). All four commands must pass.
6. Confirm you did not move a NEVER-touch fence and that the sacred tests
   (`tests/tool_schema_fidelity.rs`, `tests/all_open_golden.rs`,
   `tests/architecture.rs::governance_core_has_no_forbidden_back_edges`) are green and unmodified.
   Also confirm `tests/webapi_auth.rs` and `tests/channels_policy.rs` (H8's own sanctioned tests)
   are green and their EXISTING assertions are byte-unmodified (you may ADD new tests to
   `tests/webapi_auth.rs` if a task names one there; you may not edit an existing assertion in it).
7. Commit exactly this task's code + tests: `feat(console): K<N> <short title>`.
8. Update `LEDGER.md`: move RESUME HERE to the next task, set this task's status to DONE with the
   commit hash, and log any numbered deviations. Commit this LEDGER update separately:
   `docs(console): record K<N> commit hash`.

## Completion criteria

- K1..K5 each landed as its own commit (two commits per task per the convention above); every
  prefix left a green tree.
- The full suite is green, including the untouched sacred tests, `tests/webapi_auth.rs` and
  `tests/channels_policy.rs` unmodified in their existing assertions, `tests/config_schema_golden.rs`
  passing against DELIBERATELY regenerated golden files (K1), and every new test named by the
  K1-K5 task files.
- All-open output is byte-identical to before the batch (the Console's own GET/POST routes are a
  strictly additive branch ahead of the existing WS-upgrade path in `handle_connection`; a WS
  upgrade request's behavior -- 400/403/101 -- is untouched).
- A real spawned `ghostlight service` process answers `GET /` with the Console's embedded HTML,
  `GET /api/v1/config` with the live provenance-aware config JSON, and `GET /api/v1/sessions` with
  the live session JSON, each gated by the SAME `channels.webapi.from` policy decision the WS
  upgrade already uses; `POST /api/v1/config/webapi-enable-remote` writes the single user-layer
  `channels.webapi.from` key (refusing cleanly under an org-mandatory lock) and records one
  `config_changed` session-event audit record.

## Failure protocol (when a task cannot complete)

If a STOP precondition fires, the tree contradicts a load-bearing assumption, a NEVER-touch fence
would have to move, or an AUTHOR-MUST-PIN oracle is still unpinned in `PINS.md`:

1. REVERT the working-tree changes for this task (`git restore` / discard) so the tree stays green
   at the last completed task.
2. In `LEDGER.md`, set the task's status to BLOCKED and record: the exact assumption that failed
   (with the file/symbol you actually found), which STOP precondition or fence triggered, and what
   you would need to proceed.
3. HALT. Do NOT skip ahead -- later tasks depend on earlier ones. The frontier author reviews the
   ledger and re-issues or amends the task.

Never bypass a hook, never weaken a sacred invariant to make a task pass, and never invent an
oracle to make a test go green.

## NEVER touch (global; each names its single sanctioned exception if any)

Carried forward, re-verified against the live `docs/tasks/hub/BOOTSTRAP.md` on 2026-07-05, with NO
fresh sanctioned exception granted to any of them by this batch:

- `src/transport/mcp/tools.rs` (`TOOLS_JSON`: the 13 trained schemas + `explain`) -- byte-frozen.
  No exception. The Console's HTTP vocabulary is its own, non-sacred, versioned surface (ADR-0030
  Decision 9) and NEVER re-serializes these schemas.
- `tests/tool_schema_fidelity.rs` -- no exception; keep green untouched.
- `tests/all_open_golden.rs` -- the all-open CLIENT-VISIBLE assertions are FROZEN. No exception.
  The Console is a strictly additive branch in `handle_connection`; a lone all-open MCP-stdio
  session's output is untouched by this batch (the Console has no MCP-stdio surface at all).
- `tests/peer_death.rs`, `tests/mcp_protocol.rs` -- untouched; this batch never spawns a different
  process topology than the Hub batch already landed.
- `tests/architecture.rs` a7 (`governance_core_has_no_forbidden_back_edges`) -- `src/governance/**`
  names no browser/transport/mcp/native/url and no bare `tabId`/`token`/`socket` identifier. This
  batch's ONE sanctioned `src/governance/**` addition is the new `channels.webapi.from` `KeyDef`
  registration in `src/governance/config/mod.rs` (PINS.md CS8) and the small `pub(crate)`
  write-extraction in `src/governance/config/cli.rs` (PINS.md CS7) -- both are plain config-registry
  work of the same shape every existing `KeyDef`/`cli.rs` entry already is; neither names a
  browser/transport/hub/native type. All session/socket/HTTP code for the Console lands in
  `src/hub` (mainly `src/hub/webapi.rs` and `src/hub/session.rs`), exactly as H7/H8 did. No other
  `src/governance/**` file changes.
- `src/transport/native/host.rs` framing (4-byte LE prefix, `MAX_MESSAGE_LEN`, `encode`/
  `read_message`) -- no exception.
- The MCP JSON-RPC wire + the pinned `notifications/tools/list_changed` line (`server.rs`) -- no
  exception.
- `Browser::attach` single-EXTENSION-link rejection (`AttachOutcome::AlreadyAttached`) -- retained;
  not touched by this batch.
- The EXISTING WS-upgrade path in `src/hub/webapi.rs::handle_connection` (the 400/403/101
  responses, `MAX_HANDSHAKE_BYTES`, the `Sec-WebSocket-Key`/`Host`/`Origin` checks, the RFC 6455
  handshake and frame tunnel) -- byte-for-byte UNCHANGED for any request that IS a WS upgrade. K2
  adds a NEW branch that runs for a plain GET/POST request BEFORE the existing
  `Sec-WebSocket-Key`-required check, never inside it.

## NEVER touch (this batch's own fences, from the GROUNDING that commissioned it)

- The Console must NEVER write anything to the user config layer except the single "enable remote
  connections" key, `channels.webapi.from` (PINS.md CS8). No task in this batch adds a second
  writable key, a free-text policy editor, or any path that lets an HTTP caller choose an arbitrary
  config value. K5's POST body is IGNORED; the value written is the one PINNED literal in PINS.md
  CS5.
- The Console must NEVER implement token mint/revoke in this batch. ADR-0030 Decision 9 names it as
  part of the Console's eventual surface, but the ADR's own Consequences section defers "the
  authenticated REMOTE adapter as a product (mTLS/PoP, per-principal scoped manifests, threat-model
  ADR)" to its own future ADR; there is no principal/token data model anywhere in the tree today. If
  you find yourself inventing a token type, a credential store, or a `channels.webapi.from` member
  meaning anything beyond a bare string pattern, STOP. K5 instead renders a plain, visible "token
  mint/revoke: coming in a future release" note in the Console UI (PINS.md CS5) so the ADR's
  described feature is not silently absent with no explanation.
- The Console must NEVER read, render, or write anything resembling manifest-grant authoring (the
  `grants`/`channels`/`tools` axis model, ADR-0030's "Governance schema section"). Its config view
  (K3) renders ONLY the ADR-0019 five-layer resolved KEY registry (`layers::Resolution`), never a
  manifest document.
