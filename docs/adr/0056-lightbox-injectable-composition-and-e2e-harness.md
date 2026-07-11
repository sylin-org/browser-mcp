# ADR-0056: Lightbox -- injectable composition roots and the dev-only integration harness

Status: Accepted (2026-07-10; owner named it: "Oooh lightbox, I like it! I'd say yes. ADR, AND move
the complete e2e content there too"). Continues ADR-0032 (test at pure seams, inject at the
composition root) and ADR-0051 (verification topology); closes the ADR-0055 Phase-4a owed live-e2e
hermetically. Implementation lands interleaved with the ADR-0055 batch (seams and the skeleton
first); the legacy e2e migration is its own ledgered sub-batch.

## Context

Every "e2e owed" in the ADR-0055 build traced to one root cause: the service reads security-fixed
system boundaries that must not be redirectable at runtime -- the org policy file, the `managed.json`
trust anchor, the license file (fixed per platform; no flag, env var, or config key relocates them;
the only environmental sensitivity is the Windows `ProgramData` location convention itself). That
non-overridability is a deliberate security property: a user who can repoint the trust anchor
bypasses governance. But it also means nothing can stand up the real stack hermetically: integration
tests either touch the real system locations (pollution) or wait for owner-supervised runs on a
clean box (does not scale). The live-client exe lock on `target/debug` (post ADR-0051 reinstall)
compounds it: the e2e tier needs an isolated CARGO_TARGET_DIR, today provided by dual-maintained
`scripts/test-e2e.ps1/.sh`.

ADR-0032 and ADR-0051 already established the cure for two boundaries (config sources; the relay's
stdio) and cut ~74 spawn tests to ~12 irreducible ones plus a 27-test tagged e2e tier. This ADR
finishes the arc for the remaining boundaries -- filesystem locations, the clock, the network
fetcher -- and gives the process-boundary tier a sustainable home.

A harness that merely SPAWNS the production binary cannot work: the fixed trust-anchor paths leave
it only pollution or a runtime override (the exact hole we refuse to open). The resolution is that
the harness is a DIFFERENT COMPOSITION ROOT over the same library crates -- possible only if the
library accepts its environment as parameters. Seams are the enabler; the executable is the
delivery vehicle.

## Decision

1. **Injection at composition roots, never runtime overrides.** A plain `GovernancePaths` struct
(org policy, managed bootstrap, managed cache, license locations) with a `production()` constructor
that becomes the ONLY place the fixed platform paths are computed; a clock/tick seam where timing
logic needs determinism (the ADR-0055 Phase-4b poll loop); the fetch seam that already exists
(`governance::managed::fetch_bytes`) made parameterizable. The production binary composes
`production()` hardcoded: byte-identical behavior, zero test mode, no new environment variables (the
`GHOSTLIGHT_MANAGED_CACHE_DIR` override added in the ADR-0055 batch is retired in favor of injection
when the seams land). The trust-anchor non-overridability property is PRESERVED by construction:
tests construct differently-wired instances; nothing overrides the deployed one.

2. **`ghostlight-lightbox`: the dev-only harness executable.** A workspace crate (`publish = false`,
excluded from release artifacts, never registered as a native host or MCP server) that is a second
thin composition root over the same library: real governance/managed/hub logic wired to temp
directories, an injected clock, and a real localhost fake-org endpoint (so the real ureq/rustls
network path executes). It runs under its own instance names (ADR-0044 isolation) plus injected
paths, so it coexists with a live service. Two modes: a scenario runner (`lightbox list`, `lightbox
run <scenario>`, `lightbox run --all`, CI exit codes) and an interactive sandbox (`lightbox up`) that
stands up a fully governed Ghostlight to poke at. The name is on-brand deliberately: a lightbox is a
controlled-light inspection chamber, and this surface is dev-facing, where the ADR-0055 D9 register
split allows the personality.

3. **The e2e tier consolidates INTO lightbox (owner-directed).** Lightbox absorbs
`scripts/test-e2e.ps1/.sh` by managing the isolated build itself: it builds the exes-under-test into
an isolated target dir on dev boxes (defeating the live-client exe lock) and accepts a
reuse-the-cache flag for CI (no live clients there); the dual scripts are then retired. The 27
`#[ignore=e2e]` spawn tests and the quarantined e2e-smoke harness migrate scenario-by-scenario
against a PER-TEST PARITY LEDGER: every test is accounted for -- migrated to a named scenario or
retired with a written reason -- and CI runs BOTH the old `-- --ignored` job and the lightbox job
until the ledger completes, then flips. No coverage vanishes silently.

4. **Boundaries.** The in-process fast tier (the `serve_session` / inproc fixtures and all
`cargo test` logic tests) STAYS in `cargo test`: moving it across a process boundary would
reintroduce the flake ADR-0051 removed. Real-browser scenarios (actual Chrome, visual FX, live CDP)
remain the small user-supervised tier: lightbox fakes the extension up to the native-messaging
boundary and absorbs everything service-side. Security posture: the deployed binary gains no test
surface; the harness grants an attacker nothing they could not get by compiling their own binary --
the property that matters (THE deployed service cannot be redirected) is untouched.

5. **The scenario library is executable specification.** ADR invariants become named runnable
proofs: `managed-activation` (closes the ADR-0055 Phase-4a owed e2e as one command),
`fail-closed-cold-boot`, `rollback-guardian` (the D6/D9 guardian moment), `poll-update` (Phase 4b
live updates), and a Continuity chaos set (endpoint dies mid-run, 500s, stale-mirror rollback, clock
skew) made deterministic by the injected clock and net. The same scenarios double as the demo
machine (`lightbox up` with a fake org: the Passport, denials-as-doors, and guardian moment with
zero org infrastructure -- README tour, store assets, founder demos, prospect try-before-you-buy),
as CI dogfooding of the exact customer path (`policy sign` -> host -> `managed.json` -> activate on
every push), and as the repro format for bug reports ("run this scenario" over manual step lists).
The pattern is reusable for future family products.

## Consequences

- The ADR-0055 owed live-e2e closes hermetically (`lightbox run managed-activation`); Phase 4b's
  poll/backoff logic is born deterministic instead of time-flaky.
- The dual e2e shell scripts die; one cross-platform Rust orchestrator replaces them; the exe-lock
  workaround lives inside the tool that needs it.
- Migration cost is real and bounded: 27 tagged tests, each ledgered; CI runs dual jobs during the
  transition (temporary runtime cost, deliberate).
- The library's composition points change signature (paths/clock/fetch parameters) -- churn in the
  service crates with byte-identical production behavior, the same trade ADR-0032/0051 made twice.
- A new permanent dev crate to maintain; mitigated by it replacing two shell scripts, a quarantined
  smoke harness, and the owner-supervised e2e checklist.

## Provenance

Grew directly out of the ADR-0055 Phase-4a gap ("live e2e owed: cannot pollute %ProgramData% or
start a second service"). The owner proposed delegating integration tests to a harness service; the
assistant reframed the enabler as injection-at-composition-root (no runtime override, so the
trust-anchor property survives) and initially recommended in-process only; the owner pushed back
that environment surface in the main service smelled like scope creep and proposed a separate
dev-only executable; the synthesis -- seams in the library, the executable as a second composition
root and delivery vehicle -- resolved it (a spawner-harness alone cannot work against fixed
trust-anchor paths). The owner chose the name Lightbox and directed the full e2e consolidation
("move the complete e2e content there too"); objections were scoped to the three conditions in
Decisions 3-4, and the strategic opportunities in Decision 5 were enumerated in session.
