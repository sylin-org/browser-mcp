# K3: GET /api/v1/config + the config table UI

Cites: `docs/adr/0019-layered-configuration-model.md` Decision 2 and its 2026-07-05 amendment;
`docs/tasks/console/PINS.md` CS1 (route table), CS2 (response shape). Needs K1 (the
`current_resolution()` accessor) and K2 (the router + embedded page shell) DONE. Read
`docs/tasks/console/BOOTSTRAP.md` in full first.

## What this task is

The provenance-aware config view: for every registered key, its resolved value, which layer set
it, and whether it is locked -- the `chrome://policy` analog named in ADR-0019's own amendment.
READ ONLY. This is a view of the ADR-0019 five-layer KEY REGISTRY (`layers::Resolution`), never a
manifest document -- it must not read, render, or accept anything shaped like a manifest grant.

## Current-tree facts

- K1 landed `ConfigStore::current_resolution() -> Arc<layers::Resolution>` on `ServiceContext`'s
  `store` field. `layers::Resolution::iter()` yields `(key: &'static str, resolved: &Resolved)` in
  `KEYS` registry order; `Resolved { value: serde_json::Value, source: layers::Source, locked:
  bool }`; `Source::as_str()` renders exactly `"org_mandatory"`/`"user"`/`"org_recommended"`/
  `"preset"`/`"builtin"`. `KeyDef.description` is the human-readable description string, keyed by
  the SAME `key` string (look up via the registry's existing `key_def`-style helper, or iterate
  `KEYS` in parallel with `Resolution::iter()` -- verify which is simpler by reading the actual
  current code, both are equally correct).
- K2 landed the router in `src/hub/webapi.rs`'s `handle_connection` and the `CS1` route table
  infrastructure this task extends with one more row.

## STOP preconditions

- If `layers::Resolution` has no way to also retrieve each key's `description` alongside its
  `(value, source, locked)` triple (i.e. if `KeyDef` is not otherwise reachable from the route
  handler), STOP and report the actual mismatch -- do not invent a duplicate description string.

## Required behavior

1. Add ONE row to CS1's route table (already reserved there): `GET /api/v1/config`, gated by the
   SAME `channels.webapi.from` decision K2's router already applies to every claimed route.
2. The handler calls `ctx.store.current_resolution()`, iterates it in registry order, and builds
   the EXACT JSON shape PINS.md CS2 pins: a top-level `{"keys": [...]}` object, each entry
   `{"key", "value", "source", "locked", "description"}` in that field order, with `source`
   exactly `Source::as_str()`'s own string (never re-worded) and `value` the `Resolved.value`
   `serde_json::Value` re-serialized verbatim (no re-encoding, no stringification of non-string
   values).
3. Update `src/hub/console/index.html`/`console.js`/`console.css` (K2's shell) to fetch
   `/api/v1/config` and render it as a table: one row per key, columns for value, source layer,
   and a locked badge when `locked` is true. No specific byte-for-byte markup is pinned -- only
   the underlying JSON route's shape and the fact that the page renders it are tested.

## Tests to write FIRST

In `tests/console_static_routes.rs` (the file K2 created) or a new `tests/console_config_api.rs`
(your choice; if new, add `mod support;` and follow K2's spawn pattern) -- pick whichever keeps
the file focused, but do not duplicate K2's port-uniqueness helper, reuse it:

- `config_api_returns_every_registered_key_in_registry_order`: a real GET to `/api/v1/config`
  against a real spawned all-open service returns `200 OK`, `application/json`, and a `keys` array
  whose length equals the CURRENT total count of registered keys (read this count from the live
  registry yourself -- e.g. via the same schema/docs generation path CS8.1 exercises -- never
  hardcode a guessed number that could drift), in the SAME order `KEYS` is declared.
  Pin one concrete row from the ACTUAL current registry (a key you have personally verified exists
  today, e.g. one already used elsewhere in this codebase's own tests) and assert its `source` is
  `"builtin"` and `locked` is `false` for a lone all-open service with no user/org overlay.
- `config_api_reflects_a_locked_org_mandatory_key`: spawn a service with an org-mandatory override
  on one key (reuse whichever existing test helper/pattern the Hub batch's own tests already use
  for org-policy overrides -- e.g. `tests/manifest_validation.rs`'s or
  `tests/hot_reload.rs`'s `ProgramData`-pointed org file convention; verify it still applies to
  the CONFIG org file, not just the manifest, before reusing it) and assert that key's `source` is
  `"org_mandatory"` and `locked` is `true` in the JSON response.
- `config_api_is_refused_when_channels_webapi_from_denies_the_source`: reuse K2's/`tests/
  channels_policy.rs`'s existing pattern for a denied source and confirm `/api/v1/config` returns
  the SAME `403 Forbidden` shape CS1.3 pins (i.e. this route is not accidentally left ungated).

## Out of scope

- No write action (K5).
- No session data (K4).
- No manifest-grant rendering of any kind.
