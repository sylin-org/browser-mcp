# Verification topology evaluation: fewer, more meaningful moving parts

Status: evaluation (2026-07-09). Feeds ADR-0051. Scope chosen with the owner: the RUNTIME process
topology and the TEST architecture that exercises it, treated as one coupled problem. The trigger was
concrete: executing the official-rebaseline batch (ADR-0050) T1, the code was correct and every
deterministic test was green, yet `cargo test --workspace` still could not be driven to a clean local
pass -- it hung, then flaked, on spawn-based integration tests that T1 never touched.

This document maps the moving parts, isolates the fragility to its root causes, separates what is
irreducibly end-to-end from what is incidentally end-to-end, and recommends a direction. It proposes
no code here; the decision is captured in ADR-0051.

## 1. What "this process" actually is

Two layers of moving parts, coupled because the second exists only to exercise the first:

Runtime topology (ADR-0046, ADR-0030):
- Three role executables: `ghostlight` (CLI + persistent Hub service), `ghostlight-adapter-agent`
  (per MCP client; relays stdio to the service), `ghostlight-adapter-browser` (Chrome native host;
  relays stdio to the service). The two adapters are thin byte relays (118 and 77 lines) that link
  `ghostlight-transport` only and hold no governance.
- Four wire seams: MCP client <-> adapter-agent (newline JSON-RPC over stdio); adapter-agent <->
  service (named-pipe/UDS adapter/control endpoint, framed hello+proof then newline JSON-RPC);
  adapter-browser <-> service (named-pipe/UDS extension endpoint, 4-byte framed); Chrome <->
  adapter-browser (native-messaging 4-byte framing over stdio).
- Five process types at play (three roles + the Chrome extension SW + the OS supervisor that keeps
  the service warm), plus many long-lived background tasks (parent-death watchdog 1.5s poll,
  idle-grace 30s watcher, config file-watcher, per-session stdout/policy tasks, per-connection
  Browser writer + 10s grace-drain).
- The transport is NOT behind a unifying trait: it is concrete tokio named-pipe / UnixStream types,
  cfg-split, with only `impl AsyncRead + AsyncWrite` bounds threaded through the relay/serve
  functions. A de-facto in-process seam already exists (a real `Browser` attached to one half of a
  `tokio::io::duplex`, with a fake extension on the other half).

Test architecture:
- 125 integration tests across 34 files in `tests/`, plus roughly 600 inline unit tests across the
  crates.
- Of the 125: about 74 spawn an OS child process (about 50 use the two-process service+adapter
  topology; about 24 spawn a single CLI subprocess), and about 51 run purely in-process.
- The full `cargo test --workspace` wall-clock is dominated entirely by the spawning tests; the ~600
  unit tests and ~51 in-process integration tests finish in a couple of seconds combined.

## 2. Where the fragility actually comes from

Executing T1 surfaced the failure modes directly. None are in tool-registration code; all are in the
spawn-based E2E tier. Five root causes, in descending leverage:

1. Real-stdio coupling in the relay. `relay_adapter` binds `tokio::io::stdin()/stdout()` and must
   `process::exit` because the blocking Windows ReadFile thread is unjoinable. Under an interactive
   terminal the test process inherits a live console stdin that never signals EOF, so the one
   stdio-driving test (`hub_identity::relay_adapter_sends_a_real_guid_not_a_placeholder`) hangs
   forever. With a closed stdin (`< /dev/null`, or an isolated job) the same test passes in 0.00s.
   This is a test-environment dependency baked into the production relay's shape.

2. Exe-lock churn from live peers. A persistent/scheduled `ghostlight service` relaunches itself
   after being killed, and Chrome respawns `adapter-browser` on every extension reconnect. On Windows
   a running exe cannot be replaced, so `cargo build`/`cargo test` intermittently fails to relink
   `ghostlight*.exe` mid-run. This is not a test bug; it is the cost of a persistent-service topology
   plus a browser that owns one of the role binaries, colliding with an incremental compiler.

3. Kill/respawn races on reused endpoints. `adapter_reconnect` and `adapter_override` kill and rebind
   a service on the SAME named-pipe endpoint, then assert reconnect within timing windows. Named-pipe
   teardown/rebind ordering (especially on Windows) and the reconnect poll cadence make these
   inherently timing-sensitive.

4. Non-isolatable shared OS state. `hot_reload::org_policy_hot_swap_end_to_end` takes over the
   machine's REAL default audit path (the `dirs`/`SHGetKnownFolderPath` lookup ignores env), guarded
   only by a backup/restore `Drop`. Two concurrent runs on one machine contend on that real file.
   Endpoint, log dir, ProgramData, user-config dir, and webapi port ARE env-isolable; the default
   audit path and the OS-user identity are not.

5. Assertions on log TEXT and hard sleeps. The reconnect/override tests string-match debug-event
   `.jsonl` log lines ("session identity minted...", "override resolution: connected to candidate
   1/2") and use hard `sleep(5s)` plus 20s poll ceilings. Log-line lag and wall-clock waits are a
   flakiness multiplier layered on top of the real IPC being tested.

The correction worth recording: the per-user anti-squat `hub-key` is NOT a contention source. Each
test's service and adapter share the same endpoint-derived `GHOSTLIGHT_LOG_DIR`, so every pair reads
its own key; parallel tests do not fight over it.

## 3. Irreducible vs incidental end-to-end

The central finding: most spawning tests spawn to prove WIRING, not to prove a property that requires
real processes. The recon classifies them:

Genuinely end-to-end (a real process boundary IS the thing under test) -- about a dozen:
- `adapter_reconnect` (2), `adapter_override` (1): reconnect/failover across a real service
  kill/respawn.
- `hub_lifecycle` (2): service survives adapter exit; impostor anti-squat refusal.
- `peer_death` (1): the native host self-exits when the service dies (OS peer-death detection).
- `hot_reload` (1): filesystem-watcher-driven live policy reload.
- `hub_completion_criteria` (1): two real clients multiplex; kill fans out.
- `bare_invocation` (1): the CLI-contract exit code.
- the redaction chokepoint spawn test (1), the late-extension wait note (1).

Incidentally end-to-end (spawns only to prove wiring already provable in-process) -- the majority:
- `tool_enforcement` (12), `tool_advertisement` (3), `shadow_mode` (1), `script_tool` (2), most of
  `mcp_protocol` (8), the `manage_web_*` HTTP tests (12, which could target an in-process router), and
  the CLI-plan subprocess tests (`install_instance`, `policy_*`: ~24, whose plan/render cores are
  already unit-tested inline).

Four in-process seams already exist to serve the fast tests, and they generalize:
1. `StepRunner` + `StubRunner` (script interpreter over a stub dispatch, zero processes).
2. `Browser` attached to a `tokio::io::duplex` with a fake extension task -- a real `Browser` over a
   fake pipe. This is the most generalizable transport seam.
3. The `Governance` facade constructed directly with a fake audit sink -- the entire decision + audit
   chokepoint with no server and no IPC.
4. The pure code-declared surface (`advertised_tools_json`, `REGISTRY`, `render_config_schema`) and
   source-text scans.

## 4. Options

Option A -- Harden the harness only (no topology change).
- Add a `GHOSTLIGHT_AUDIT_DIR` env override so `hot_reload` stops seizing the real audit path; make
  the E2E tier run serially with closed stdin in CI and locally; replace log-text assertions with
  structured debug-state assertions; replace hard sleeps with polled deadlines.
- Pros: smallest change; no runtime risk. Cons: leaves ~74 tests paying real-process tax for
  properties that are pure; leaves the relay's real-stdio coupling; does not reduce runtime moving
  parts, which was the explicit goal.

Option B -- Fewer runtime parts + a first-class in-process seam, then re-tier the tests (recommended).
- Runtime: merge the two thin adapters into ONE relay binary (`ghostlight-relay --role agent|browser`).
  They are near-identical stdio-over-pipe relays; the differences (newline vs 4-byte framing;
  reconnect+handshake-replay vs exit-on-close) are a role flag. Three executables become two
  (service + relay). Make the relay take its streams as `impl AsyncRead + AsyncWrite` parameters
  instead of hardcoding `tokio::io::stdin/stdout`, so the relay logic unit-tests over `duplex` and
  only ONE thin real-stdio smoke test remains.
- Seam: promote the `Browser`-over-`duplex` + `Governance`-with-fake-sink pattern into a first-class,
  documented in-process harness (a small `tests/support` in-process fixture, and/or a real
  `Listener`/`Stream` trait so the fake transport is not ad hoc).
- Tests: adopt an explicit three-tier pyramid -- (1) unit (keep ~600 inline), (2) in-process
  integration over the promoted seam (migrate the ~40 wiring-proof tests here), (3) a SMALL,
  quarantined E2E tier (~12 tests) run serially, closed-stdin, fully env-isolated, asserting on
  structured state, gated as its own CI job.
- Pros: reduces runtime moving parts (3 exes -> 2), makes the relay testable, cuts the flaky surface
  from ~74 tests to ~12, and makes V-ALL fast and trustworthy. Cons: a real (bounded) migration; the
  adapter merge touches ADR-0046 territory and must preserve its "adapters link transport only,
  connectivity-only" property.

Option C -- Collapse further (re-merge adapters into the service as a dual-role binary).
- Rejected. ADR-0002 was already superseded by ADR-0046 for good reasons (independent relink;
  editor-death cleanup; Chrome owning a role binary). Re-merging trades the current clarity for the
  exe-lock and lifecycle problems ADR-0046 solved. Not "more meaningful" -- just fewer.

## 5. Recommendation

Adopt Option B, phased so every prefix leaves a green tree:

- Phase 1 (harness hygiene, no runtime change): `GHOSTLIGHT_AUDIT_DIR` isolation; standardize the E2E
  tier as serial + closed-stdin + structured-state assertions; document the in-process seam. This
  alone makes V-ALL deterministic and is the immediate unblock for the ADR-0050 batch.
- Phase 2 (relay testability): parameterize the relay's streams; move relay logic under `duplex`
  unit tests; keep one real-stdio smoke test.
- Phase 3 (adapter merge): fold `adapter-agent` and `adapter-browser` into one `ghostlight-relay`
  with a role flag; update installers, the doctor, and ADR-0046's consequences.
- Phase 4 (test re-tier): migrate the incidentally-E2E tests to the in-process seam; leave the ~12
  irreducible E2E tests as the quarantined tier.

Sequencing note: Phase 1 is the one that matters for the batch in flight. Phases 2-4 are a separate,
independently-landable track and must not block T2-T5 of ADR-0050.

## 6. Impact on the current batch (ADR-0050 T2-T5)

Until Phase 1 lands, local V-ALL for the batch must treat the spawn tier as environment-sensitive:
run the workspace tests with closed stdin (`< /dev/null`), serially (`--test-threads=1`), with no
live `ghostlight service` and, ideally, Chrome's extension disconnected, and rely on CI as the
authority for the E2E tier. The deterministic tiers (fmt, clippy, build, the ~600 unit tests, the
in-process integration tests, the tool-count/fidelity oracles, and the extension `node --test` suite)
are fast and trustworthy and remain the per-task gate.
