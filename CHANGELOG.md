# Changelog

All notable changes to Ghostlight are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] -- 2026-07-05

The Ghostlight Hub release. The single-session model is replaced by a persistent
background service that owns the one browser link and multiplexes every client through a
single governance chokepoint, plus a local Console for seeing what the service is doing.

### Added

- **The Ghostlight Hub (ADR-0030).** A persistent, standalone `ghostlight service` now
  owns the browser link and the client endpoint for its whole life. Every MCP client runs
  as a thin adapter that connects to it, so any number of clients (Claude Code, Cursor,
  and others) can be connected at once, each multiplexed as its own session through the
  single governance chokepoint. This repeals the previous one-session-at-a-time limit
  (ADR-0004). The service is kept warm by a per-user OS supervisor (Windows Task
  Scheduler, macOS launchd, Linux systemd --user), self-heal-started on first use if it
  is down, and shuts down only after an idle-grace window with no live sessions and no
  browser link.
- **The Console (ADR-0030 Decision 9).** A local, loopback-pinned web page served by the
  service at its web-API address. It shows live sessions (with truncated session ids), a
  provenance-aware view of the layered configuration (value, source layer, and lock state
  per key), and a single "enable remote connections" control. It is never a manifest
  editor and never a remote control plane.
- **Local web API.** An opt-in TCP JSON-RPC endpoint that acts as a second session source
  alongside the stdio adapters, gated by the new `channels.webapi.from` policy key
  (loopback-only by default).
- **Per-session browser tab groups.** Each session's tabs are grouped in the browser so
  concurrent sessions stay visually distinct.
- **Cross-session isolation and admission control.** Binary-authoritative tab ownership so
  one session cannot drive another's tabs; adapter-minted session ids bound to the
  connecting client's OS credential; per-client session and mint quotas (never a single
  global cap); and an anti-squat proof on the client endpoint.
- **Reconnect grace and an honest bounded queue (ADR-0030 Decision 3).** A bounded
  reconnect window over transient extension drops, per-client rate limiting, and
  oversize-reply chunking so one session's large payload cannot head-of-line-block
  another's small one.
- **Extension polish.** Official mascot icons, a per-action visual-feedback vocabulary
  (click ripples, drag trail, type shimmer), and an options page plus popup toggle for
  those preferences and action captions.
- **Installer auto-start.** `ghostlight install` now registers and starts the OS
  supervisor so the service is always ready.

### Changed

- Renamed the browser extension to "Ghostlight in Browser" and recorded its Chrome Web
  Store listing.
- Reorganized the internals into a `src/hub` composition root (HubCore / ServiceContext)
  with transport-generic session serving, so the same governance path serves both the
  stdio adapters and the web API.

### Fixed

- **Lifecycle hardening (ADR-0029).** Cross-platform process-liveness primitives, a
  parent-death watchdog so an orphaned session self-terminates when its editor exits, a
  liveness-aware `doctor` with a `--fix` reaper and a startup orphan sweep, and a single
  shutdown coordinator. Idempotent extension library modules so a re-injected content
  script cannot double-register.

## [0.1.0] -- 2026-07-04

First tagged release: the unconstrained browser-automation engine (all-open) with the
governance overlay available as an opt-in capability manifest. Shipped as four platform
binaries (Windows x86_64, macOS Intel and Apple Silicon, Linux x86_64) plus the extension
zip, with SHA-256 checksums and signed build-provenance attestations.

[0.2.0]: https://github.com/sylin-org/ghostlight/releases/tag/v0.2.0
[0.1.0]: https://github.com/sylin-org/ghostlight/releases/tag/v0.1.0
