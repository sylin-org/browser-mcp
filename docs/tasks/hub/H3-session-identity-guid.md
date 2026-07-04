# H3: Adapter-minted GUID identity + local peer-cred binding

> Batch: Ghostlight Hub. Normative: docs/adr/0030-ghostlight-hub-orchestrator.md (Decision 4; also
> "Preserved invariants (and the pinned oracles the batch transcribes)"). One task = one commit.
> Facts below are as-of-authoring 2026-07-04 -- RE-READ the named files before relying on any line
> number or signature.

## Goal
Give every session an opaque, unguessable identity that the SERVICE routes and isolates by, while
the governance core stays PID/GUID-agnostic. The thin ADAPTER mints a CSPRNG UUIDv4 GUID and
presents it in the connection handshake (the first framed message). The LOCAL accept layer in
`src/hub` captures the connecting peer's OS credential and binds the GUID to that minting peer
(refusing a GUID re-presented by a different OS user, except the sanctioned same-user reuse path),
and holds that credential as the per-peer rate-limit key. Why: ADR-0030 Decision 4 ("identity model
(adapter-minted GUID; core stays PID-agnostic)") plus its "Amendment to the transport-side".

## Authority
1. docs/adr/0030-ghostlight-hub-orchestrator.md Decision 4 -- NORMATIVE. Cite it; never restate its
   semantics.
2. BOOTSTRAP.md ground rules.
3. This task file.
If they conflict, the higher wins.

## Current-tree facts (as-of-authoring; RE-READ before relying)

- `src/hub/` is created by H0-H2 (the composition root + `ServiceContext` + per-session state +
  `serve_session<S>(stream, ctx)` + the multiplex accept loop). As of THIS authoring the directory
  does not yet exist. RE-READ `src/hub/mod.rs` after H2 lands: this task adds a `guid` field to
  H2's per-session record and hooks the accept path; it does NOT invent the session record.
- `src/hub/session.rs` -- NEW file this task creates.
- `src/transport/native/messages.rs` -- currently a DOC-ONLY module (no Rust types; "only the
  mcp-server constructs and parses them, so they are documented here"). It documents the
  binary<->extension vocabulary (`tool_request`/`tool_response`/`tool_error`, hold, `session_killed`,
  `tab_url_*`). H2 introduces the adapter<->service connection handshake: the SS1 "hello" frame
  (PINS.md SS1) that H2's adapter role already sends on the adapter/control endpoint. This task ADDS a
  documentation section for that hello frame's `guid` member; it does NOT invent a second or separate
  handshake frame. No Rust types are added here.
- `src/proc.rs` -- ADR-0029 process-liveness primitives (`ProcId {pid, created}`, `parent`,
  `is_alive`, `orphaned`, `terminate`). It STAYS (adapter lifecycle + doctor reap). The SERVICE core
  gains NO dependency on it for identity: the GUID carries no pid/ancestor/creation-time.
- `src/governance/dispatch.rs` -- `Governance` holds `mode`, `audit`, and `client: Mutex<Option<
  ClientInfo>>`; `set_client(&self, name, version)` (first-capture-wins) at ~line 386;
  `current_client` at ~line 402. The audit `identity` field is written as `None` today
  (`record_session_killed` ~line 417, `record_manifest_reload` ~line 433; also `audit/mod.rs`
  ~lines 194/213). Governance has NO subject/GUID concept and MUST NOT gain one (a7).
- Coupling that pins scope: Decision 2 places "the opaque subject GUID" in PER-SESSION state held
  in `src/hub` ALONGSIDE the `Governance` facade, NOT inside `Governance`. So the GUID lives in
  H2's `src/hub` per-session record; `Governance` is NOT modified by this task, which keeps the a7
  arch-test and the all-open byte-identity trivially green. The audit `identity` field STAYS `None`
  in H3 (stamping an authenticated subject into audit is Decision 9 / H8, not this task).

## Required behavior

### 1. Mint (ADAPTER side; ADR-0030 Decision 4 "minted by the thin ADAPTER")
Add to the NEW `src/hub/session.rs`:

```
/// An opaque, unguessable session identity minted by the adapter and presented to the service.
/// Canonical lowercase hyphenated UUIDv4 (36 chars). Secret material (ADR-0030 Decision 4:
/// "Treat the GUID as secret in logs/audit").
pub struct SessionGuid(String);

impl SessionGuid {
    /// Mint a fresh CSPRNG UUIDv4. Uses `uuid::Uuid::new_v4()` (the crate is already a dep).
    pub fn mint() -> Self;
    /// Parse a presented string; `Some` iff it is a valid version-4 UUID in canonical form.
    pub fn parse(s: &str) -> Option<Self>;
    /// The raw canonical string (for the wire handshake and the routing-map key ONLY).
    pub fn as_str(&self) -> &str;
}
```

- The adapter role mints via `SessionGuid::mint()` ONCE per adapter PROCESS and reuses that same
  value for the process lifetime (Decision 4: "Same adapter process reuses its GUID (same group); a
  new adapter process mints a new one"). A new adapter process calls `mint()` again -> a different
  GUID -> a different group (D7).
- `Display`/`Debug` for `SessionGuid` MUST render a REDACTED form that does NOT contain the raw
  canonical string, so the GUID never reaches a `tracing` log or audit sink verbatim (Decision 4:
  "Treat the GUID as secret in logs/audit"; if persisted for reuse it is owner-only, never client
  config -- at-rest persistence is OUT OF SCOPE for H3, see fences). The exact redacted string form
  is AUTHOR MUST PIN before execution; the TEST asserts only the non-leak invariant below.

### 2. Peer credential + binding (LOCAL accept layer; Decision 4 "Amendment to the transport-side")
Add to `src/hub/session.rs`:

```
/// The connecting peer's OS credential, captured by the LOCAL accept layer purely for admission
/// control and as the per-peer rate-limit key (ADR-0030 Decision 4 amendment). Lives in `src/hub`,
/// NEVER in `src/governance` (a7). `user` is the peer's OS user principal: the SID string on
/// Windows, the uid on Unix. `pid` distinguishes processes for logging; admission compares `user`.
#[derive(Clone, PartialEq, Eq)]
pub struct PeerCred { pub user: PeerUser, pub pid: u32 }

/// Opaque OS-user principal; same-user comparison is `==`.
#[derive(Clone, PartialEq, Eq)]
pub struct PeerUser(String);

/// The service's GUID -> bound-peer routing map (Decision 2: per-session state in `src/hub`).
pub struct SessionRegistry { /* map SessionGuid canonical string -> PeerCred */ }

pub enum Admission { Admitted, Refused }

impl SessionRegistry {
    pub fn new() -> Self;
    /// Admit a peer presenting a GUID. First presentation records the binding and returns
    /// `Admitted`. A re-presentation is `Admitted` iff the presenter is the SAME OS user as the
    /// bound peer (the sanctioned reuse path re-verifies same-user); a DIFFERENT user is `Refused`
    /// and the existing binding is left unchanged (ADR-0030 Decision 4: "refuse a GUID presented by
    /// a different peer, except the sanctioned reuse path which re-verifies same-user").
    pub fn admit(&mut self, guid: &SessionGuid, peer: &PeerCred) -> Admission;
}
```

- The real OS capture (Windows `GetNamedPipeClientProcessId` + token SID; Unix `SO_PEERCRED` /
  `getpeereid`) happens in the accept path in `src/hub/mod.rs` on the raw pipe/UDS handle H2 already
  owns. `admit` itself is a PURE function of `(guid, peer)` so the tests drive it with synthesized
  `PeerCred` values (no second real OS user needed).
- `PeerCred` is ALSO retained on the session record as the per-peer rate-limit key. H3 only
  PROVIDES the key; the mint/group quota ENFORCEMENT is Decision 3 / H5 -- do not add quota logic
  here.

### 3. Service routing (Decision 4 "routes and isolates by that opaque GUID only")
- In `src/hub/mod.rs`, after H2's handshake reads the presented GUID and the accept layer captures
  the `PeerCred`, call `SessionRegistry::admit`. On `Refused`, drop the connection without creating
  a session (the exact refusal log/return string is AUTHOR MUST PIN; do NOT surface the GUID). On
  `Admitted`, key the H2 per-session record (its `Governance` facade + owned-handle set) by the
  GUID's canonical string.
- The `Governance` facade is NOT modified. Do NOT add a subject/GUID setter to `src/governance/**`.
  If GUID routing appears to need the governance core, STOP (see STOP preconditions) -- the mapping
  is in `src/hub` and the core stays handle-agnostic (a7).

### 4. Wire handshake documentation (`src/transport/native/messages.rs`, doc-only)
Add a documentation section describing the adapter->service connection handshake's identity member:
the SS1 "hello" frame the ADAPTER sends (PINS.md SS1: `{ "hub": 1, "role": "adapter", "guid":
"<uuid-v4>" }`) carries the session GUID in its `guid` member, a canonical lowercase hyphenated
UUIDv4 (`SessionGuid`). The GUID rides in H2's existing hello frame; do NOT invent a second or
separate handshake frame. The hello frame's `hub`/`role` members are DEFINED BY H2 -- RE-READ
messages.rs and H2's `src/hub/handshake.rs` constants; document the `guid` member against that
existing SS1 hello frame. The EXTENSION link uses NO hello at all (it is on its own endpoint,
server-speaks-first; PINS.md SS1 as amended 2026-07-04), so there is no `ext` role and nothing about
the extension link to document here. Keep this section doc-only (no Rust types), matching the file's
existing style.

### 5. a7 scanner extension (SANCTIONED `tests/architecture.rs` edit)
H3 is the ONE task in this batch sanctioned to edit `tests/architecture.rs` (ADR-0030 "Preserved
invariants" as amended names H3 as the extender; it is the single sanctioned edit to that file in
this batch). EXTEND `governance_core_has_no_forbidden_back_edges` so its scan of `src/governance/**`
ALSO rejects the identifiers `tabId`, `token`, and `socket` (belt-and-suspenders for the type
discipline: the governance core must name no transport/handle/credential type). Keep every existing
back-edge rule in the scanner intact (the current browser/transport/mcp/native/url forbidden set);
this edit is PURELY ADDITIVE. Make no other change to `tests/architecture.rs`.

### 6. Role marker + governance-chokepoint assertion (ADR-0030 Decision 1 addendum; PINS.md SS8)

Added 2026-07-04 after H2 landed the two-endpoint split. Create `src/hub/role.rs` per PINS.md SS8's
PINNED shape (`Role`, `set_role`, `role`, `assert_role`, `assert_service_role`, `assert_adapter_role`,
verbatim panic message). This is a fail-loud backstop, not a substitute for H2's structural
separation: it must be a no-op (no output, no behavior change) whenever the role is already correct,
so it does not touch the all-open byte-identity invariant.

Wire it at the two seams H2's landed code already makes obvious (RE-READ `src/hub/mod.rs` to confirm
these are still the actual function names/shapes before relying on them; H2 landed them as of this
writing):
- `run_as_service` (`src/hub/mod.rs`, the async fn entered when `ipc::claim_adapter_endpoint` returns
  `Ok`): call `hub::role::set_role(hub::role::Role::Service)` as the ABSOLUTE first line of its body,
  before the `Browser::with_debug` call.
- `run_as_adapter` (`src/hub/mod.rs`, the async fn entered on `Err(crate::Error::SessionBusy)`): call
  `hub::role::set_role(hub::role::Role::Adapter)` as the ABSOLUTE first line of its body, before the
  `ipc::relay_adapter` call.
- `serve_session` (`src/transport/mcp/server.rs`, the governance chokepoint every transport calls per
  ADR-0030 Decision 2): call `hub::role::assert_service_role("serve_session")` as the ABSOLUTE first
  line of its body, before any other setup.

Add `tests/hub_role_wiring.rs::governance_chokepoint_asserts_service_role` (PINS.md SS8): a text-scan
test (a7-style) asserting the source of `serve_session` in `src/transport/mcp/server.rs` contains the
literal substring `assert_service_role`. This guards the WIRING; `role.rs`'s own unit tests (below)
guard the assertion LOGIC. H6 later adds the symmetric adapter-side wiring test to
`tests/hub_lifecycle.rs` when it builds the spawn-on-demand function -- do not attempt that half here.

## Tests (BY NAME; assertions pinned)

- Keep green: `tests/all_open_golden.rs`, `tests/audit_recorder.rs` (do not modify).
  `tests/architecture.rs::governance_core_has_no_forbidden_back_edges` stays green but is EXTENDED by
  this task (the single sanctioned edit to `tests/architecture.rs` in the batch -- see the a7 scanner
  extension item above); every existing back-edge rule in it must remain intact.
  A lone all-open session mints/binds a GUID but its OUTPUT and audit records stay byte-identical:
  the GUID is a routing key in `src/hub`, never stamped into audit in H3.

- Add: `tests/hub_identity.rs::guid_is_v4_csprng_and_bound_to_minting_peer`
  - Mint two GUIDs with `SessionGuid::mint()`. Assert each `as_str()` parses via
    `uuid::Uuid::parse_str` with `get_version() == Some(uuid::Version::Random)` (version-4) and the
    RFC-4122 variant. Assert the two GUIDs are NOT equal (CSPRNG, not a counter).
  - Non-leak invariant (transcribed from ADR-0030 Decision 4 "Treat the GUID as secret in
    logs/audit"): assert `!format!("{}", guid).contains(guid.as_str())` AND
    `!format!("{:?}", guid).contains(guid.as_str())` for a minted guid.
  - Binding: build `let a = PeerCred { user: PeerUser("user-A".into()), pid: 100 };`, a fresh
    `SessionRegistry`, and assert `registry.admit(&g, &a)` is `Admission::Admitted` on first
    presentation and `Admission::Admitted` again when the SAME peer `a` re-presents `g` (the reuse
    path).

- Add: `tests/hub_identity.rs::foreign_peer_presenting_a_guid_is_refused`
  - Mint `g`, admit it bound to `let a = PeerCred { user: PeerUser("user-A".into()), pid: 100 };`
    (assert `Admitted`).
  - Present `g` with `let b = PeerCred { user: PeerUser("user-B".into()), pid: 200 };` (a DIFFERENT
    OS user) and assert `registry.admit(&g, &b) == Admission::Refused`.
  - Assert the original binding is unchanged: `let a2 = PeerCred { user: PeerUser("user-A".into()),
    pid: 999 };` (same user, different pid -- the sanctioned same-user reuse path) admits:
    `registry.admit(&g, &a2) == Admission::Admitted`.

- Add (the SANCTIONED `tests/architecture.rs` edit):
  `tests/architecture.rs::governance_core_rejects_tabid_token_socket_identifiers`
  - Assert the extended scanner FLAGS a synthetic `src/governance/**` source naming `tabId` (and
    likewise one naming `token`, and one naming `socket`) as a forbidden back-edge, and that a
    source naming none of the three passes. This proves the `tabId`/`token`/`socket` extension is
    live, not dead code, without weakening any existing rule.

- Transcribed oracles kept intact (asserted GREEN via the keep-green tests; do NOT re-derive):
  - Audit record field order, exactly 14 keys, in order:
    `event_id, ts, identity, client, tool, action, capability, domain, decision, grant_id,
    denial_id, duration_ms, manifest, held` (`tests/audit_recorder.rs`). H3 adds no key and leaves
    `identity` as `None`.
  - Session-event record field order, exactly 6 keys, in order:
    `event_id, ts, identity, client, event, manifest`. H3 stamps no GUID into it.

- Add (PINS.md SS8, transcribe verbatim; `src/hub/role.rs`'s own `#[cfg(test)]` module):
  - `adapter_role_hitting_the_governance_chokepoint_panics`:
    `#[should_panic(expected = "must only run when this process's role is Service")]`; calls
    `assert_role(Role::Adapter, Role::Service, "test")`.
  - `service_role_hitting_spawn_on_demand_panics`:
    `#[should_panic(expected = "must only run when this process's role is Adapter")]`; calls
    `assert_role(Role::Service, Role::Adapter, "test")`.
  - `matching_roles_do_not_panic`: calls `assert_role(Role::Service, Role::Service, "test")` and
    `assert_role(Role::Adapter, Role::Adapter, "test")`; a plain test asserting neither panics.

- Add: `tests/hub_role_wiring.rs::governance_chokepoint_asserts_service_role` (see item 6 above).

## Verification (literal commands)
```
cargo build --all-targets
cargo test --test hub_identity
cargo test --test hub_role_wiring
cargo test --lib role
cargo test --test all_open_golden
cargo test --test architecture governance_core_has_no_forbidden_back_edges
cargo test --test architecture governance_core_rejects_tabid_token_socket_identifiers
cargo test --test audit_recorder
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## STOP preconditions
- If H2's `serve_session` (or its accept loop) has NO per-connection handshake point at which the
  adapter's first framed message can carry the GUID, STOP -- the handshake seam is H2's to build;
  do not bolt one on here.
- If the accept layer in `src/hub` has NO access to the connecting peer's raw pipe/UDS handle to
  read its OS credential, STOP -- the peer-cred capture seam belongs to the transport/accept in
  `src/hub`; build it there, never by reaching into `src/governance`.
- If GUID routing would require `use crate::transport::...` inside `src/governance/`, STOP -- put
  the key mapping in `src/hub` and leave `Governance` untouched (Decision 4 amendment: "This lives
  in `src/hub`, never in `src/governance`").
- If H2 already introduced a session-identity type or a per-session GUID field, STOP and reconcile
  with it rather than duplicating (do not create a second identity type).
- If `run_as_service`, `run_as_adapter`, or `serve_session` no longer exist under those names or no
  longer cleanly separate the two roles (item 6), STOP and reconcile against H2's ACTUAL landed shape
  before wiring the role marker -- do not guess a different call site.
- If satisfying this task would require moving or weakening any NEVER-touch fence below, STOP.

## NEVER touch (this task)
- `src/transport/mcp/tools.rs` (TOOLS_JSON: the 13 trained schemas + `explain`), byte-frozen. No
  exception.
- `tests/tool_schema_fidelity.rs`. No exception; keep green untouched.
- `tests/all_open_golden.rs` + the all-open byte-identity invariant. No exception; the GUID
  mint/bind path MUST be a no-op for a lone all-open session's output and audit.
- `tests/architecture.rs::governance_core_has_no_forbidden_back_edges` (a7): `src/governance/**`
  names no browser/transport/mcp/native/url and no tabId/token/socket type, and gains NO PID/GUID
  concept. `SessionGuid`/`PeerCred`/`SessionRegistry` land in `src/hub` ONLY. H3 SANCTIONED
  EXCEPTION: H3 is the ONE task in this batch allowed to edit `tests/architecture.rs`, solely to
  EXTEND this scanner to ALSO reject the `tabId`/`token`/`socket` identifiers in `src/governance/**`
  (ADR-0030 "Preserved invariants" as amended names H3 as the extender). Every other back-edge rule
  in the scanner stays intact; no other edit to `tests/architecture.rs` is sanctioned.
- `src/transport/native/host.rs` framing (4-byte LE prefix, `MAX_MESSAGE_LEN`,
  `encode`/`read_message`). No exception; H3 adds documentation only, not framing.
- The MCP JSON-RPC wire and the `notifications/tools/list_changed` line (`server.rs`); the adapter
  is a byte relay, never a rewriter.
- `Browser::attach` single-EXTENSION-link rejection (`AttachOutcome::AlreadyAttached`). Retained;
  H3 does not touch the extension link.
- `src/proc.rs` STAYS (adapter lifecycle + doctor reap) -- do NOT delete or repurpose it here; the
  SERVICE core gains no pid/ancestor/creation-time identity from it.
- At-rest GUID persistence (owner-only 0600 / DPAPI-per-user reuse across processes) is OUT OF
  SCOPE for H3 -- H3 reuse is in-process only. Do not add file/registry persistence here.
- Per-peer quota ENFORCEMENT (Decision 3 / H5) is OUT OF SCOPE -- H3 only provides `PeerCred` as the
  rate-limit key.
