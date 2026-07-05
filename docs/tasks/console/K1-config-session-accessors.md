# K1: Config + session read accessors; shared config-write function

Cites: `docs/adr/0030-ghostlight-hub-orchestrator.md` Decision 9, Decision 5, "Governance schema
section"; `docs/tasks/console/PINS.md` CS6, CS7, CS8, CS8.1, CS9. Read `docs/tasks/console/
BOOTSTRAP.md` in full first (ground rules, authority order, per-task procedure, NEVER-touch).

## What this task is

Pure data-layer plumbing. No HTTP, no UI, no route. This task makes three additions the LATER
tasks (K2-K5) all depend on, so it must land first:

1. `ConfigStore` gains a way to read the LIVE `layers::Resolution` (per-key provenance: value,
   source layer, locked), not just the derived typed `Config` it already exposes (PINS.md CS6).
2. `run_set`'s lock-check + validate + write sequence is extracted into a reusable `pub(crate)`
   function both the CLI and (starting at K5) the Console call (PINS.md CS7).
3. A new `channels.webapi.from` config key is registered in the typed key registry, and
   `src/hub/webapi.rs::run` is wired to read it live instead of the hardcoded
   `builtin_webapi_from()` (PINS.md CS8, CS8.1, CS8.2) -- without this, K5's write action would
   have zero effect on the running service.
4. `SessionRegistry` gains a read-only snapshot accessor for the Console's future sessions view
   (PINS.md CS9).

## Current-tree facts (verify against the live tree before writing anything; PINS.md's own dates
are "verified 2026-07-05" -- re-verify, do not trust blindly)

- `src/governance/config/reload.rs`'s `ConfigStore` holds `snapshot: Mutex<Arc<Config>>` and
  computes but discards a `layers::Resolution` inside `load_initial_with_policy` and `apply_plan`
  before converting it via `Config::from_resolution(&resolution)`. `layers::Resolution` derives
  `Clone` (confirm this yourself: `grep -n "pub struct Resolution" -A2
  src/governance/config/layers.rs` should show `#[derive(Debug, Clone)]` immediately above it).
- `src/governance/config/cli.rs`'s `run_set` (~line 278) is exactly as PINS.md CS7 describes it;
  `write_user_value` (~line 227) is a private fn in the same file.
- `src/governance/config/mod.rs`'s `KeyDef` struct (~line 226) has exactly six fields, in order:
  `key, description, constraint, default_fully_open, default_safe, default_restricted` (confirmed
  against the live tree by the batch author; re-confirm yourself with `grep -n "pub struct
  KeyDef" -A15 src/governance/config/mod.rs`). `KeyConstraint::None` and `KeyValue::StrList` both
  already exist and are already used by at least one other key (PINS.md CS8 names
  `AUDIT_SYSLOG_ADDRESS` as the closest existing precedent for an unconstrained string-shaped key
  -- confirm a StrList key with `KeyConstraint::None` or the closest analog actually compiles the
  way CS8's literal expects before treating this as settled).
- `src/hub/webapi.rs`'s `run(ctx: ServiceContext)` opens with `let allowlist =
  builtin_webapi_from();`, never reading `ctx` for this purpose. `builtin_webapi_from()` itself
  must remain exported and unchanged (existing tests call it directly as a pure function).
- `src/hub/session.rs`'s `SessionRegistry` has a private `bindings: HashMap<String, PeerCred>`
  field and only `new()`/`admit()` are public. `owned_tab_ids(&owned_tabs, guid)` already exists
  and gives one guid's full owned-tab set. `SessionGuid::parse`/`as_str` are already public in
  this same module.
- `tests/config_schema_golden.rs` pins `render_config_schema()`/`render_key_reference()`'s output
  against `tests/golden/config-schema.json` and `tests/golden/config-keys.md` BY DESIGN -- these
  two files WILL need regenerating as part of this task's own commit (PINS.md CS8.1); this is not
  a NEVER-touch violation, it is the documented, sanctioned update path for a registry change.

## STOP preconditions

- If `layers::Resolution` does NOT derive `Clone`, STOP (CS6's accessor design requires cloning it
  cheaply into an `Arc`; re-deriving a resolution on every read would duplicate
  `resolve_with_warnings`'s disk-reading logic, which PINS.md CS6 explicitly forbids).
- If `ConfigStore`'s test-only constructors (`for_test`, `for_test_with_config`,
  `for_test_with_user_source`, or any other `#[cfg(test)]` constructor you find) cannot all be
  updated to seed the new `resolution` field without inventing a pinned literal PINS.md does not
  supply, use PINS.md CS6's own fallback (`Arc::new(layers::resolve(&layers::LayerInputs::
  default()))`) -- this is explicitly NOT itself an oracle (no test asserts its exact content), so
  do not STOP over it; just make every constructor compile.
- If registering `CHANNELS_WEBAPI_FROM` as a `KeyDef` requires a `KeyConstraint` variant that does
  not compile as `KeyConstraint::None` against the real `KeyValue::StrList` validation path (i.e.
  if the base-type check for `StrList` is not what PINS.md CS8 describes), STOP and report the
  actual validation behavior found -- do not weaken or bypass `def.parse_value`'s existing checks
  to force a green build.
- If `write_user_value`'s signature or body has meaningfully changed from PINS.md CS7's
  transcription (not just moved lines), STOP rather than adapt the extraction speculatively.

## Required behavior

1. **CS6**: add `resolution: Mutex<Arc<layers::Resolution>>` to `ConfigStore` and the
   `current_resolution()` accessor, exactly as PINS.md CS6 specifies, written at the SAME two call
   sites `snapshot` is written (`load_initial_with_policy`, `apply_plan`), using the resolution
   value ALREADY computed there (never a second `layers::resolve` call). The `apply_plan` write is
   UNCONDITIONAL (not gated by `Config`'s `changed`/`PartialEq` check) -- transcribe this exactly,
   it is the whole point of exposing provenance separately from the derived value.
2. **CS7**: extract `run_set`'s lock-check + validate + write sequence into `pub(crate) fn
   set_user_value(key: &str, value: serde_json::Value, domain_pattern_valid: fn(&str) -> bool) ->
   crate::Result<std::path::PathBuf>` in `src/governance/config/cli.rs`, exactly as PINS.md CS7's
   code block shows. `run_set` becomes: `parse_cli_value` -> `set_user_value(...)` -> the two
   `println!` lines on `Ok`, propagating `Err` via `?`. This must be a byte-identical-behavior
   extraction: every existing `cli.rs` test must stay green with NO assertion changed.
3. **CS8 + CS8.2**: add the `CHANNELS_WEBAPI_FROM` constant and its `KeyDef` registration to
   `src/governance/config/mod.rs` exactly as PINS.md CS8's code blocks show (all three preset
   defaults `["localhost"]`, matching today's hardcoded value byte-for-byte, so this changes NO
   resolved value for any session that has not touched the Console). Add `live_channels_webapi_
   from` to `src/hub/webapi.rs` per CS8.2 and change `run()`'s startup allowlist read to use it;
   change the accept loop to re-read `live_channels_webapi_from(&ctx.store)` fresh per accepted
   connection (never the loop-hoisted value) so a live policy edit takes effect without a service
   restart, while the TCP `bind` address itself stays resolved ONCE at startup (never re-bound).
4. **CS8.1**: regenerate `tests/golden/config-schema.json` and `tests/golden/config-keys.md` via
   `cargo run -- config schema` / `cargo run -- config docs` (redirected to those paths), verify
   both are LF-only (no `\r`), and confirm `cargo test --test config_schema_golden` passes against
   the regenerated files. Diff-review both files before committing: the ONLY change should be the
   new `channels.webapi.from` entry.
5. **CS9**: add `SessionSummary` and `live_session_summaries(registry, owned_tabs)` to
   `src/hub/session.rs` exactly as PINS.md CS9 shows (never holding `registry`'s lock and
   `owned_tabs`'s lock simultaneously). This function is UNUSED by any route until K4 -- an
   unused-function warning under `cargo clippy --all-targets -- -D warnings` would fail the build;
   if this happens, mark it `#[allow(dead_code)]` with a comment citing "consumed at K4" (the same
   latitude `tests/support/mod.rs`'s own `#![allow(dead_code)]` already takes for exactly this
   forward-reference reason), rather than treating it as a STOP.

## Tests to write FIRST (transcribe pinned shapes verbatim; never invent an expected value)

- `src/governance/config/reload.rs`'s existing `#[cfg(test)]` module: a new test asserting
  `current_resolution()` reflects the SAME source/locked data a fresh `layers::resolve` over the
  same `LayerInputs` would produce, and that a re-resolve (simulate via whatever the existing test
  module's own `apply_plan`-exercising helper already does) updates BOTH `current()` and
  `current_resolution()`. Also add a case proving the "unconditional" write: craft two resolves
  where the DERIVED `Config` compares equal but a key's `Source` differs, and assert
  `current_resolution()` reflects the second resolve's source even though `current()`'s underlying
  `Config` did not change (this is the concrete, pinned behavior CS6 requires, not just its
  written description).
- `src/governance/config/cli.rs`'s existing `#[cfg(test)]` module: the EXISTING lock-refusal and
  write-path tests must stay green unmodified. Add ONE new test calling `set_user_value` directly
  (not through `run_set`) confirming it returns `Ok` with the written path and `Err` with the
  EXACT lock-refusal message string transcribed in PINS.md CS5, using the SAME test-isolation
  approach the existing tests already use (read the existing lock-refusal test's setup before
  writing a new one -- do not introduce a second way of testing this).
- `src/hub/session.rs`'s existing `#[cfg(test)]` module: a new test constructing a
  `SessionRegistry` with one `admit`-ted binding and an `owned_tabs` map with that guid owning two
  tabIds, asserting `live_session_summaries` returns exactly one `SessionSummary` whose `guid` is
  the first 8 characters of the admitted GUID (never the full string), whose `pid` matches the
  admitted `PeerCred.pid`, and whose `owned_tab_ids` is the full sorted set.
- `tests/config_schema_golden.rs`: no new test function; the EXISTING test(s) there must pass
  against the regenerated golden files (CS8.1).

## Out of scope (fences for this task specifically)

- No HTTP route, no static asset, no UI -- those are K2/K3/K4/K5.
- No change to `Config`'s own typed fields/accessors for `channels.webapi.from` -- nothing needs a
  typed `Config` accessor for this key; every consumer reads the raw `Resolution` (CS6).
- No change to `governance::channels::ChannelsPdp`, `validate_webapi_from`, or any file under
  `tests/webapi_auth.rs`/`tests/channels_policy.rs` -- this task only makes the ALLOWLIST live-
  read-from-config; it does not touch how the allowlist is DECIDED against once resolved.
