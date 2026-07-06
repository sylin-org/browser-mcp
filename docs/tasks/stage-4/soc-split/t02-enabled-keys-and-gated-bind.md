# t02: Add `*.enabled` keys + policy-gated listener bind

Cites: ADR-0033 Decision 5 + Decision 8. Needs t01 DONE (consumes the renamed `inbound` axis).

## What this task is

Introduces the per-adapter enablement keys that finally make listener enablement a policy-
controlled decision (the "deny the web adapter" case ADR-0030 Decision 5 promised but the code
never implemented). After this task, an org-mandatory `inbound.web.enabled = false` means the
listener never binds. Also adds the `manage.web.*` keys the management plane (t04) will consume.

This task lands the keys + the bind gate ONLY. The management plane's separate routing context
lands in t04; this task just makes the web adapter's bind consult `enabled`.

## Why second

It depends on t01's renamed axis (the new keys live under `inbound.*` and `manage.*`). It is
sequenced before the module moves (t03/t04) so the relocated adapters consume the gate directly.

## Current-tree facts (re-verify)

- `run_service_loop` in `src/hub/mod.rs` does `tokio::spawn(webapi::run(ctx.clone()))`
  unconditionally (grep `tokio::spawn(webapi::run`).
- `webapi::run` in `src/hub/webapi.rs` resolves the allowlist + bind address, then
  `TcpListener::bind`, returning early only if the bind itself fails (grep `pub async fn run`).
- The config key registry is in `src/governance/config/mod.rs` (the `KEYS` slice). Existing keys
  for the channel axis: `inbound.web.from` (post-t01). There is no `*.enabled` key today.
- Preset defaults table is in the same file (`default_fully_open` / `default_safe` /
  `default_restricted` per `KeyDef`).

## What changes

1. **New registry keys** in `src/governance/config/mod.rs`:
   - `inbound.web.enabled` — bool, `default_fully_open: true`, `default_safe: true`,
     `default_restricted: true`. (Local-on per Decision 5; an org-mandatory layer sets `false` to
     deny the adapter.)
   - `inbound.pipe.enabled` — bool, `default_*: true` (the primary path; pipe authz is OS
     same-user, but the key exists for symmetry and a paranoid deployment).
   - `outbound.browser.enabled` — bool, `default_*: true`.
   - `manage.web.enabled` — bool, `default_*: true`.
   - `manage.web.from` — StrList, `default_*: ["localhost"]`, with a constraint marking it
     non-user-widenable beyond localhost (Decision 3: the management plane is permanently
     loopback; an org layer can set `enabled = false` but cannot widen `from`). Enforce this in
     `validate_*` (reject any member other than `localhost` / `"*"`-as-loopback — actually reject
     `*` outright here; the management plane has no remote posture).
2. **Bind gate in `run_service_loop`**: before spawning the web listener, resolve
   `inbound.web.enabled`. If false, log `info!` ("inbound.web adapter disabled by policy; not
   binding") and skip the spawn. The pipe and extension endpoints get the same gate treatment
   against `inbound.pipe.enabled` (the extension endpoint stays server-speaks-first and unchanged
   in wire shape — the gate only decides whether to bind it).
3. **`webapi::run` (or its relocated successor in t03)** resolves `enabled` fresh per accepted
   connection is NOT needed for `enabled` (it's a bind-time decision, like the bind address);
   only `from` is per-connection (already the case). Document this asymmetry: `enabled` is
   bind-time (restart required), `from` is per-connection (live).

## Tests

- New unit test in `src/governance/config/mod.rs`: `inbound.web.enabled` resolves true in all
  three presets; an org-mandatory override to false resolves false and locked.
- New test: `manage.web.from` validator rejects `["*"]` and `["example.com"]` (the management
  plane is permanently loopback — Decision 3).
- A real-process smoke (add to a new `tests/adapter_enablement.rs` or extend
  `tests/webapi_auth.rs`): spawn a service with `inbound.web.enabled = false` forced via a test
  org-policy overlay, assert the port is NOT bound (`TcpListener::bind` to the same port
  succeeds, meaning the service did not claim it). This is the "deny the web adapter" acceptance
  test — the very thing ADR-0030 Decision 5 promised.
- `tests/config_schema_golden.rs`: regenerate the golden (new keys present).

## Verification

- All four gates green.
- The "deny the web adapter" smoke passes: with `inbound.web.enabled = false`, the listener does
  not bind.
- `cargo run -- config docs` lists all five new keys with correct descriptions.

## Out of scope

- The management plane's separate routing context — t04 (this task adds the keys; t04 uses them).
- The Console's loopback-lock enforcement at the route layer — t04 (this task adds the validator
  on `manage.web.from`; t04's router additionally hard-codes the loopback check as
  defense-in-depth).
- Renaming the assets (`console.*` → `manage.*`) — t04.
