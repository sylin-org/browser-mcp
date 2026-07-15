# ADR-0082: Linux user-session discovery for scrubbed relay environments

- Status: Accepted
- Date: 2026-07-14
- Amends: ADR-0030 Decision 8, ADR-0045, ADR-0062

## Context

Ghostlight's Linux service owns its Unix sockets under the user's XDG runtime directory, normally
`/run/user/<uid>/ghostlight`. The relay used `dirs::runtime_dir()` and then fell back to the user
cache. That is correct for an ordinary shell, but not for every process launcher.

A live Ubuntu Desktop test found two stripped launch environments:

- VS Code started through the SSH/systemd test harness did not pass `XDG_RUNTIME_DIR` or
  `DBUS_SESSION_BUS_ADDRESS` to Cline's MCP child.
- Chrome Stable launched the native-messaging relay with no usable session environment.

In both cases the service correctly listened under `/run/user/1000`, while the relay resolved a
different socket under the cache. If the service was down, `systemctl --user start` also failed
because it could not find the user bus. Installation, MCP initialization, and the extension could
each appear healthy in isolation while the complete chain remained disconnected.

Passing environment values in one client configuration fixes only that client and cannot fix
Chrome's native-host launch. The transport substrate must recover the same-user session location
without trusting caller-controlled path text.

## Decision

1. **One transport-owned session resolver.** `ghostlight-transport` owns Unix user-session
   discovery for both socket paths and supervisor self-heal. Relay roles remain policy-free and do
   not duplicate platform logic.
2. **The standard runtime directory remains first.** When `dirs::runtime_dir()` resolves a value,
   Ghostlight uses it unchanged. This preserves normal XDG behavior and non-Linux Unix behavior.
3. **Linux adds a security-checked fallback.** When the standard value is absent, Ghostlight may
   use `/run/user/<effective-uid>` only when `symlink_metadata` proves that the path is a real
   directory, is owned by the effective user, and has no group or other permission bits. A
   symlink, ownership mismatch, permissive mode, missing path, or metadata error rejects the
   fallback.
4. **The cache remains the final socket fallback.** If secure Linux session discovery fails, Unix
   socket resolution retains the existing user-cache behavior. Ghostlight does not weaken
   filesystem isolation to force a connection.
5. **Linux supervisor commands receive only missing session values.** Before executing
   `systemctl --user`, transport fills a missing `XDG_RUNTIME_DIR` from the resolved runtime path
   and a missing `DBUS_SESSION_BUS_ADDRESS` as `unix:path=<runtime>/bus`. Existing environment
   values always win. Other platforms are unchanged.
6. **No wire or policy change.** Endpoint names, native-host identity, MCP schemas, governance,
   audit, extension logic, and the one-stack model remain unchanged.

## Consequences

- A Chrome native host and an MCP client launched with a scrubbed environment resolve the same
  Linux endpoint as the per-user service.
- Cold-start self-heal can reach the existing systemd user manager from those launch contexts.
- The fallback cannot redirect Ghostlight into a group-writable, world-writable, foreign-owned,
  or symlinked runtime directory.
- Linux-specific unit tests cover secure-directory acceptance, permission and ownership rejection,
  symlink rejection, and command environment completion. The visible Linux lifecycle test remains
  the integration proof for Chrome's real native-host environment.

## Provenance

Observed during the 2026-07-14 Linux lifecycle run on Ubuntu Desktop 26.04, Chrome Stable 150, VS
Code 1.128.1, and Cline 4.0.8. Cline successfully read the public `install.md`, installed v0.5.7,
and initialized the relay after a client-local environment workaround. The first demo call then
reported that the browser was not connected. A native-host reload reached the service briefly but
the v0.5.7 browser relay resolved its socket from the stripped environment and exited. A direct
MCP initialization with the correct runtime environment passed, isolating the defect to Linux
user-session discovery rather than MCP compatibility.
