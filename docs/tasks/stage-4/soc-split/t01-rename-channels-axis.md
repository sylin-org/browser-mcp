# t01: Rename the grant axis `channels` → `inbound` (config key + grammar)

Cites: ADR-0033 Decision 4 + Decision 5. Needs ADR-0033 read in full.

## What this task is

The single breaking rename that realigns the grant axis, the config-key prefix, and (in later
phases) the code zone under one word: `inbound`. This task does the config + grammar + PDP layer
ONLY — no module moves yet (those land in t03/t04). It is sequenced first because every later
phase's code references the new key/axis names.

## Why first

The rename touches `src/governance/**` (the renamed axis is transport-agnostic, stays in the
core per the `a7` boundary) and the config registry. Doing it first means t03/t04's relocated
adapters consume the new names directly, with no transitional aliasing. No shims.

## Current-tree facts (re-verify; do not trust stale line numbers)

- The config key constant lives in `src/governance/config/mod.rs`: `pub const
  CHANNELS_WEBAPI_FROM: &str = "channels.webapi.from";` (grep `CHANNELS_WEBAPI_FROM`).
- The registry entry: `key: CHANNELS_WEBAPI_FROM`, with `default_fully_open` / `default_safe` /
  `default_restricted` all `KeyValue::StrList(&["localhost"])` (verify with
  `grep -n -A8 'key: CHANNELS_WEBAPI_FROM' src/governance/config/mod.rs`).
- The PDP + allowlist matcher: `src/governance/channels.rs`. Symbols: `RULE_WEBAPI_FROM`
  (currently `"channel/webapi_from"`), `is_member`, `validate_webapi_from`,
  `decide_webapi_from`, `ChannelsPdp`.
- `DecisionRequest::channel_source: Option<String>` in `src/governance/ports.rs` (grep
  `channel_source`) — the resolved connecting source stamped on the request before the pure
  decision runs.
- The manifest grant grammar's axis is named `channels` in ADR-0030's grammar block and in any
  manifest examples — grep `channels:` across `examples/` and `docs/`.

## What changes

1. **Config key**: `channels.webapi.from` → `inbound.web.from`. Rename the constant
   `CHANNELS_WEBAPI_FROM` → `INBOUND_WEB_FROM` and update the string. The registry entry's
   description updates to match ("Sources allowed to connect to the local inbound.web adapter").
2. **`src/governance/channels.rs` → `src/governance/inbound.rs`** (file rename). Update
   `src/governance/mod.rs`'s `pub mod channels;` → `pub mod inbound;`. Symbols: `RULE_WEBAPI_FROM`
   → `RULE_INBOUND_WEB_FROM` (string `"inbound/web_from"`), `validate_webapi_from` →
   `validate_inbound_web_from`, `decide_webapi_from` → `decide_inbound_web_from`, `ChannelsPdp` →
   `InboundPdp`. Module doc updates to name the `inbound` axis, not `channels`.
3. **`DecisionRequest::channel_source` → `inbound_source`** in `ports.rs`. Update every call site
   (grep `channel_source` across `src/`).
4. **Manifest grammar axis**: the `channels:` key in a grant becomes `inbound:`. Update
   `src/governance/manifest/document.rs` (the `Grant` struct's field if named; the deserialization
   if map-keyed) and any embedded example manifests. Denial-id scheme (`denial::denial_id`) feeds
   off the rule string — re-pin the affected goldens (see Tests below).
5. **ADR-0030 grammar block**: leave the historical ADR text as-is (immutable), but ADR-0033
   records the rename. `docs/SPEC.md` and the key-reference doc (`config docs` output, generated
   from the registry) reflect the new name automatically.

## Tests

- `tests/channels_policy.rs` → rename to `tests/inbound_policy.rs`; update the rule-label
  assertion (`"channel/webapi_from"` → `"inbound/web_from"`) and the `ChannelsPdp::new` call →
  `InboundPdp::new`. The denial-id golden in this file changes (rule string changed) — re-pin it.
- `tests/console_enable_remote.rs`: asserts `parsed["key"] == "channels.webapi.from"`,
  `written["config"]["channels.webapi.from"] == ["*"]`, and the 409 lock message contains the
  key name. Update all three literal strings → `inbound.web.from`. (This test also moves physically
  in t04; here only the strings change.)
- `tests/webapi_auth.rs`: `builtin_webapi_from()` rename follows the symbol rename in t04; here
  only any referenced constant strings change.
- `tests/config_schema_golden.rs`: the generated `config-schema.json` golden changes (the key
  name changed). Re-generate and re-pin.
- The channels decision's unit tests inside `src/governance/channels.rs` (now `inbound.rs`) move
  with the file and update the symbol names.

## Verification (this task green)

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`
  all pass.
- `cargo run -- config docs` output shows `inbound.web.from`, not `channels.webapi.from`.
- `grep -rn "channels" src/governance/ docs/ examples/` returns only historical ADR text
  (ADR-0030's immutable grammar block) and explicit "renamed by ADR-0033" notes.
- The architecture test (`tests/architecture.rs`) still passes — no new forbidden identifiers
  introduced.

## Out of scope

- Module moves (`webapi.rs` split, `Browser` relocation) — t03/t04.
- New keys (`inbound.web.enabled`, `manage.web.*`) — t02 introduces those.
- Renaming `GHOSTLIGHT_WEBAPI_PORT` env var — separate decision, deferred (the env var names the
  port, not the policy axis; no functional coupling to the key rename).
