# 0044. Named instances: one identity parameter for dev/prod coexistence

- Status: Proposed (2026-07-08)

## Relationship to other decisions

- REFINES ADR-0030 (the hub): the hub is singular PER IDENTITY -- one service owns the one
  browser link, the extension talks to one native host. This ADR makes that identity a single
  named parameter instead of scattered hardcoded constants, so a machine can run more than one
  isolated stack (a `dev` alongside the default).
- BUILDS ON the existing env-seam convention (`GHOSTLIGHT_ENDPOINT`, `GHOSTLIGHT_USER_CONFIG_DIR`,
  `GHOSTLIGHT_LOG_DIR`, `GHOSTLIGHT_WEBAPI_PORT`): the instance follows the same "resolve from the
  environment at the point of use" pattern, adding `GHOSTLIGHT_INSTANCE`.
- HONORS ADR-0019/0020 (layered config, org policy): org policy stays fixed-per-instance and
  never flag-overridable. An instance is a separate install, not a bypass.

## Context

One developer machine needs two things at once: a fast local loop (edit -> build -> restart,
never gated on the Chrome Web Store) and the ability to validate the real deployed path
(npm-installed binary, store extension). Today the stack's identity is hardcoded in ~6 places
(endpoint, native-host name, MCP server name, three supervisor names, config dir), all singular,
so a second stack cannot coexist without colliding.

Two clarifications remove most of the apparent difficulty:

1. **Build profile is orthogonal.** `debug` vs `release` is only optimization + whether
   observability is on. It never determines a clash. The only thing that clashes is IDENTITY.
2. **The clash cannot happen by accident.** An MCP client's config is keyed by server name, so a
   second `ghostlight` entry overwrites the first -- it does not run in parallel. And the browser
   link is singular by construction. A true two-stack situation only exists when someone
   DELIBERATELY creates a second identity. This ADR makes that deliberate act clean.

The extension dev loop is already solved and orthogonal: load `extension/` unpacked (its id is
pinned by the committed manifest `key`), edit, reload at `chrome://extensions` -- instant. The
store build has a different id, so both can be installed at once.

## Decision

### Decision 1: identity is one named parameter

Introduce a single `Instance` concept, resolved ONCE from `--instance <name>` (a global CLI
flag) or `GHOSTLIGHT_INSTANCE` (the env seam, for Chrome-launched and test paths), defaulting to
the canonical instance. Every identity string derives from it through one module
(`src/instance.rs`), which becomes the single source of truth that today's scattered constants
are folded into. This is a net REDUCTION in identity surface: many hardcoded literals collapse
into one derivation.

### Decision 2: the default is byte-identical

The default instance MUST reproduce every current identifier exactly, or the published product,
existing installs, and the install/fidelity tests break. The derivation is defined so the
default yields today's literals unchanged:

| Identifier | Default instance (unchanged) | A non-default instance `<n>` |
|---|---|---|
| IPC endpoint | `org.sylin.ghostlight.v1` | `org.sylin.ghostlight.<n>.v1` |
| Native host name | `org.sylin.ghostlight` | `org.sylin.ghostlight.<n>` |
| MCP server name | `ghostlight` | `ghostlight-<n>` |
| Supervisor task (Win) | `Ghostlight Service` | `Ghostlight Service (<n>)` |
| Supervisor label (mac) | `org.sylin.ghostlight.service` | `org.sylin.ghostlight.<n>.service` |
| Supervisor unit (linux) | `ghostlight.service` | `ghostlight-<n>.service` |
| Config / policy / log dir leaf | `ghostlight` | `ghostlight-<n>` |

The default path takes no `<n>` branch at all: `Instance::default()` returns the literals above,
verified by a test that pins each one.

### Decision 3: instances are isolated, and org policy stays fixed-per-instance

A non-default instance suffixes the config dir leaf everywhere (`ghostlight` -> `ghostlight-<n>`),
so its user config, org policy, and log/observability files never touch the default's. The org
policy path remains fixed and never flag-overridable WITHIN an instance (the ADR-0020 security
property): `--instance` selects which install you are, it does not bypass that install's policy.
A fresh `dev` instance simply has no policy file at its path, so it is all-open -- the right dev
default.

### Decision 4: the native-host wrapper (the one new moving part)

The MCP-server and supervisor roles are launched by the installer, so they carry `--instance`
directly on the command line. The native host is launched by CHROME, which passes only the
calling extension's origin and `--parent-window`; the manifest `path` is a bare executable with
no room for an argument. So for a NON-DEFAULT instance the installer writes a tiny per-instance
wrapper (in that instance's data dir) that execs `ghostlight --instance <n>` with Chrome's args
appended, and points the manifest `path` at the wrapper. The DEFAULT instance keeps pointing the
manifest straight at the binary -- no wrapper, no change. This mirrors the wrapper the e2e smoke
harness already writes, so it is a known, contained pattern, not a new concept.

### Decision 5: CLI surface

- `--instance <name>` is a global flag (like `--manifest`), read before subcommand dispatch and
  used to seed the process's `Instance`. `install`/`uninstall`/`service`/`doctor`/`status` all
  honor it, so `ghostlight --instance dev install` registers the whole dev stack and
  `ghostlight --instance dev doctor` inspects it.
- `doctor` reports the active instance name and its resolved paths, so "which stack is this?" is
  always answerable.

## Consequences

### If taken

- One machine cleanly runs the default (npm/store deploy validation) and `dev` (local build,
  unpacked extension) side by side; an agent sees `ghostlight` and `ghostlight-dev` as distinct
  servers with no tool-name ambiguity because they are distinct servers.
- The identity, today smeared across ~6 files, gets one home. Future identity questions have one
  place to look.
- Deleting a dev instance is `ghostlight --instance dev uninstall` plus removing its config dir.

### Cost

- The derivation module + threading the instance to each former-constant site.
- The per-instance wrapper for the native-host role (non-default only).
- Tests: the byte-identical-default pin, and a dev-instance derivation test.

### Risks

- **Default drift.** If any default derivation diverges from today's literal, the product breaks.
  Mitigation: Decision 2's pinned test, plus the existing install/fidelity tests.
- **Scope creep into "profiles".** An instance is only an identity + isolated dirs. It is NOT a
  place to hang behavioral config (that is the layered config's job). Keep it to identity.

## Open questions

- Whether `doctor` should list ALL installed instances (scan the native-host dir) or only the
  active one. Presumably active-plus-a-hint; deferred.
- Whether a non-default instance should default observability ON (dev usually wants it). Leaning
  yes as an install-time default for non-default instances, but it stays orthogonal (a `--debug`
  choice), not baked into identity.
