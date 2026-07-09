# ADR-0051: Fewer, more meaningful moving parts in the verification topology

Status: Proposed (2026-07-09). Companion to the evaluation in
`docs/design/verification-topology-evaluation.md`, which carries the full moving-parts census and the
option analysis this ADR decides. Refines ADR-0046 (role executables) by merging the two thin
adapters, and refines ADR-0003's concrete-transport stance by introducing an in-process test seam. It
supersedes nothing; it re-shapes how the ADR-0046 topology is packaged and tested.

## Context

Executing ADR-0050 T1 (`file_upload`) exposed that the test suite -- not the product code -- is the
fragile part of the delivery loop. The T1 code was correct and every deterministic test passed, yet a
full local `cargo test --workspace` could not be driven green: it hung on a stdio-relay test, flaked
on adapter kill/respawn timing, and intermittently failed to relink because a persistent service and
Chrome kept the role binaries locked.

The measured shape (see the evaluation doc for citations):

- Runtime: three role executables (`ghostlight` service + two thin adapters), four wire seams, five
  process types, and many long-lived background tasks. The two adapters are near-identical stdio
  relays (118 and 77 lines) that link `ghostlight-transport` only.
- Tests: 125 integration tests; about 74 spawn OS processes, about 51 run in-process; plus about 600
  in-process unit tests. The spawning tests are the entire wall-clock and the entire flaky surface.
- The transport is concrete tokio named-pipe/UnixStream types with no unifying trait, but a de-facto
  in-process seam already exists (a real `Browser` over a `tokio::io::duplex`, plus the `Governance`
  facade with a fake sink, plus the `StepRunner`/`StubRunner` interpreter stub).
- The fragility root causes, in leverage order: (1) the relay hardcodes real `stdin`/`stdout` and must
  `process::exit`, so a live console stdin hangs it; (2) a persistent/relaunching service and
  Chrome's native host lock the role exes against the incremental linker; (3) kill/respawn races on
  reused named-pipe endpoints; (4) one test seizes the machine's REAL default audit path because it
  is not env-isolable; (5) assertions on debug-LOG-TEXT plus hard sleeps.
- The decisive finding: most spawning tests spawn to prove WIRING, not a property that needs real
  processes. Only about a dozen are irreducibly end-to-end (reconnect/failover, peer-death, live
  policy reload, multiplex kill fan-out, the CLI exit contract, the redaction chokepoint).

A concrete corroboration surfaced during T1 itself: the advertised tool count and the advertised
tool-NAME sets are pinned in scattered spawn tests (`adapter_override`, `adapter_reconnect`,
`hot_reload`) and in a frozen `explain`-text literal (`pipeline.rs`) that neither the task prompt nor
its red-team enumerated. A one-tool additive change had to chase pins across seven-plus files, several
only discoverable by running the flaky E2E tier. The pin surface is diffuse precisely because the test
tier that owns it is diffuse.

## Decision

Adopt "Option B" from the evaluation: reduce the runtime moving parts, promote the existing in-process
seam to first-class, and re-tier the tests so the flaky surface shrinks to the irreducible core.
Phased so every prefix leaves a green tree; only Phase 1 is in scope for unblocking the ADR-0050 batch.

1. Phase 1 -- harness hygiene (no runtime change; the immediate unblock).
   - Add a `GHOSTLIGHT_AUDIT_DIR` env override so the default audit path is test-isolable; stop the
     hot-swap test seizing the machine's real audit file.
   - Standardize the E2E tier as serial (`--test-threads=1`) with closed stdin (`< /dev/null`) and no
     live `ghostlight service`, and document this as the local V-ALL procedure for spawn tests.
   - Replace debug-LOG-TEXT string assertions and hard sleeps in the E2E tests with structured
     debug-state polling against deadlines.
   - Centralize the advertised-count / advertised-name-set oracle so an additive tool updates ONE
     place, not scattered `Some(N)` and name arrays across spawn tests. (This directly prevents the
     T1 pin-chase from recurring in T2-T5.)

2. Phase 2 -- relay testability (separate track). Parameterize the relay over
   `impl AsyncRead + AsyncWrite` streams instead of hardcoding `tokio::io::stdin/stdout`, so the relay
   logic unit-tests over `duplex` and only ONE thin real-stdio smoke test remains.

3. Phase 3 -- adapter merge (separate track). Fold `ghostlight-adapter-agent` and
   `ghostlight-adapter-browser` into one `ghostlight-relay --role agent|browser` (framing +
   reconnect-vs-exit become a role flag). Three executables become two; ADR-0046's
   "adapters link transport only, connectivity-only, independent relink" property is preserved and
   re-stated for the merged binary. Installers, the doctor, and native-messaging registration update
   accordingly.

4. Phase 4 -- test re-tier (separate track). Promote the `Browser`-over-`duplex` +
   `Governance`-with-fake-sink pattern into a documented in-process fixture (optionally behind a real
   `Listener`/`Stream` trait), migrate the ~40 incidentally-E2E wiring tests onto it, and leave the
   ~12 irreducible E2E tests as a small, quarantined, separately-gated CI tier.

Rejected -- "Option C" (re-merge the adapters into the service as a dual-role binary). ADR-0002 was
superseded by ADR-0046 for reasons that still hold (independent relink, editor-death cleanup, Chrome
owning a role binary). Re-merging would be fewer parts but less meaningful -- it reintroduces the
exact exe-lock and lifecycle coupling ADR-0046 removed.

## Consequences

- The delivery loop gains a fast, deterministic gate (unit + in-process integration + fidelity
  oracles + the extension `node --test` suite) that fully validates additive tool work without the
  spawn tier. The spawn tier becomes a small, honest, separately-run confidence check.
- Runtime moving parts drop from three executables to two, with the merged relay explicitly testable
  in-process -- fewer parts, and the remaining ones more meaningful.
- The advertised-surface oracle is centralized, so additive-tool tasks (the rest of ADR-0050, and
  future registry growth via ADR-0034 D7) touch one pinned place instead of many.
- Phases 2-4 are independently landable and MUST NOT block ADR-0050 T2-T5. Until Phase 1 lands, the
  batch treats the spawn tier as environment-sensitive: run it serially, closed-stdin, no live
  service, and rely on CI as its authority; the deterministic tiers remain the per-task gate.
- ADR-0046 is not superseded; its role-separation rationale stands. This ADR changes the packaging
  (two adapters into one) and the test strategy around it, and should be cross-referenced from
  ADR-0046's consequences when Phase 3 lands.

## Provenance (decided; do not re-litigate)

- Fewer parts must stay meaningful: the adapter split's VALUE (independent relink, lifecycle) is kept;
  only its DUPLICATION (two near-identical relays) is removed. That is why Option C is rejected and
  the merge is a role flag, not a re-absorption into the service.
- In-process first, E2E as confidence: the seam already exists and is proven (hub_multiplex runs the
  real Browser over a duplex). Generalizing it is lower-risk than it looks and removes the majority of
  the flaky surface. A dozen true E2E tests remain because some properties (peer-death, reconnect,
  live reload) only exist across real process boundaries.
- Env-isolate the audit path rather than redesign audit: the single non-isolable path
  (`SHGetKnownFolderPath`/`dirs`) is a narrow, surgical fix (`GHOSTLIGHT_AUDIT_DIR`), matching the
  existing `GHOSTLIGHT_LOG_DIR` / `GHOSTLIGHT_USER_CONFIG_DIR` / `ProgramData` override precedent.
