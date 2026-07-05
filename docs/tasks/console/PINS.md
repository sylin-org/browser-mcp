# Ghostlight Console batch: PINS (author oracle sheet)

Every value here is PINNED by the batch author. The executor TRANSCRIBES these; it never derives
or invents one (the ORACLE RULE, `docs/tasks/console/BOOTSTRAP.md`). Where a task file says
"PINNED in PINS.md CS<n>", use the value below verbatim. Semantics live here in one place; the
task files cite, they do not re-decide. Sections are numbered `CS1..CS11` ("Console Section") to
avoid colliding with the Hub batch's own `SS`-numbering in `docs/tasks/hub/PINS.md`, which a reader
may have open at the same time.

All facts below were verified against the live tree on 2026-07-05. RE-READ the named files before
relying on any line number.

## CS1 -- Route table (shared by K2, K3, K4, K5)

The Console's OWN non-sacred, versioned REST vocabulary (ADR-0030 Decision 9), served by
`src/hub/webapi.rs`'s existing TCP listener, gated by the SAME `channels.webapi.from`
`ChannelsPdp` decision the WS-upgrade path already uses (`docs/tasks/hub/H8-web-api-loopback-policy.md`
Required behavior item 4). A path is matched on the portion BEFORE any `?` character (a query
string, if present, is ignored by every route in this batch -- none of them read one).

| Method | Path | Added by | Content-Type | Auth | Body read |
| --- | --- | --- | --- | --- | --- |
| GET | `/` | K2 | `text/html; charset=utf-8` | channels.webapi.from | none |
| GET | `/console.css` | K2 | `text/css; charset=utf-8` | channels.webapi.from | none |
| GET | `/console.js` | K2 | `application/javascript; charset=utf-8` | channels.webapi.from | none |
| GET | `/api/v1/config` | K3 | `application/json` | channels.webapi.from | none |
| GET | `/api/v1/sessions` | K4 | `application/json` | channels.webapi.from | none |
| POST | `/api/v1/config/webapi-enable-remote` | K5 | `application/json` | channels.webapi.from | ignored (see CS5) |

Any other path, or a known path with the wrong method, is answered per CS1.1/CS1.2 below. A
request that carries `Upgrade: websocket` and `Sec-WebSocket-Key` (the existing WS-upgrade shape)
is UNCHANGED by this batch: it is dispatched to the EXISTING WS-upgrade code path exactly as
today, with the EXISTING 400/403/101 responses, byte-for-byte. The NEW routing added by this batch
runs BEFORE the existing `Sec-WebSocket-Key`-required check and only claims a request that:
(a) has NO `Upgrade: websocket` header (or its value is not `websocket`), AND
(b) matches one of the table's `(method, path)` pairs, OR is a GET/POST to `/` or under `/api/v1/`
    that does not match any pair (CS1.1/CS1.2).
Any request matching neither (a WS-upgrade attempt, or a plain HTTP request outside `/` and
`/api/v1/**`) falls through UNCHANGED to the existing logic, which today answers it with
`400 Bad Request` (the existing catch-all for "not a valid WS upgrade"). Do not change that
catch-all's behavior for a request this batch's new routing does not claim.

### CS1.1 -- 404 Not Found

A GET/POST under `/` or `/api/v1/**` that does not match any row in the table above (e.g.
`GET /api/v1/unknown`) gets:

```
HTTP/1.1 404 Not Found
Connection: close
Content-Type: text/plain
Content-Length: <n>

not found
```

(Body is the literal 9-byte ASCII string `not found`, no trailing newline.)

### CS1.2 -- 405 Method Not Allowed

A known path requested with a method not listed for it in the table (e.g. `POST /` or
`GET /api/v1/config/webapi-enable-remote`) gets:

```
HTTP/1.1 405 Method Not Allowed
Connection: close
Content-Type: text/plain
Content-Length: <n>

method not allowed
```

(Body is the literal 18-byte ASCII string `method not allowed`, no trailing newline.)

### CS1.3 -- 403 Forbidden (the channels.webapi.from denial)

Identical shape to the EXISTING WS-upgrade denial in `handle_connection` today
(`write_http_error(&mut stream, 403, "Forbidden")`), reused verbatim for a Console route: the
SAME function, the SAME status line and headers. Do not add a JSON body to the 403; it stays the
existing plain `Connection: close` response with no body (matching `write_http_error`'s current
implementation exactly).

### CS1.4 -- HttpRequest gains a `path` field (K2, required plumbing)

`src/hub/webapi.rs`'s `HttpRequest` struct (as-of-authoring: `struct HttpRequest { method: String,
headers: Vec<(String, String)> }`) and `parse_http_request` currently discard the request-line's
path entirely (`let mut parts = request_line.split_whitespace(); let method = parts.next()?...`;
the path token, `parts.next()`, is never read). K2 adds a `path: String` field, populated as the
SECOND whitespace-separated token of the request line (`GET /api/v1/config HTTP/1.1` -> path
`/api/v1/config`), with the query string (everything from and including a `?`, if present) NOT
stripped by `parse_http_request` itself -- stripping happens once, in the new router (CS1), so
every route match consistently ignores a query string.

## CS2 -- `GET /api/v1/config` response shape (K3)

```json
{
  "keys": [
    {
      "key": "audit.enabled",
      "value": true,
      "source": "org_mandatory",
      "locked": true,
      "description": "Record one audit line per tool call (the flight recorder)."
    }
  ]
}
```

- `keys` is an array, one entry per registered key, in `KEYS` registry order (the SAME order
  `layers::Resolution::iter()` yields and `ghostlight config list` renders).
- `key`: the key's dotted name (`KeyDef.key`).
- `value`: the resolved value, exactly as `Resolved.value` (a `serde_json::Value`) already is --
  no re-encoding.
- `source`: exactly `Source::as_str()`'s existing four-or-five-way output: one of
  `"org_mandatory"`, `"user"`, `"org_recommended"`, `"preset"`, `"builtin"` (`src/governance/config/layers.rs`,
  verified 2026-07-05). Transcribe verbatim; do not invent a different wire vocabulary.
- `locked`: `Resolved.locked` (`true` iff `source == "org_mandatory"`).
- `description`: `KeyDef.description`, verbatim.

No other top-level field. This is a READ of `layers::Resolution` (CS6); it is never a manifest
document and never includes a `grants`/`channels`/`tools` axis.

## CS3 -- `GET /api/v1/sessions` response shape (K4)

```json
{
  "live_session_count": 2,
  "adapter_bindings": [
    { "guid": "3fa85f64", "pid": 12345, "owned_tab_ids": [101, 202] }
  ],
  "note": "adapter_bindings lists sessions admitted since the service started; a listed binding may no longer be currently connected. Web/Console HTTP sessions are not yet individually tracked."
}
```

- `live_session_count`: `ctx.live_sessions.load(std::sync::atomic::Ordering::Relaxed)` (the
  EXISTING `ServiceContext` field, `src/hub/mod.rs`, incremented/decremented by the RAII guard in
  `transport::mcp::server::serve_session` -- the only accurate "how many sessions are live RIGHT
  NOW" signal that exists anywhere in the tree today). This counts EVERY live session regardless of
  source (adapter or web/WS), per its own existing doc comment.
- `adapter_bindings`: an array built from CS9's new `live_session_summaries` accessor. HONEST
  LIMITATION (verified 2026-07-05, see CS9): `SessionRegistry`'s `bindings` map is never pruned on
  disconnect (H3's same-user-reconnect design requires the binding to persist so a reconnecting
  adapter can re-present its GUID), so this array can include a binding for an adapter that has
  since disconnected. It NEVER includes a web/WS session (H8's own forward guidance, PINS.md SS9
  in the Hub batch: a web session never calls `SessionRegistry::admit`). Each entry:
  - `guid`: the FIRST 8 CHARACTERS of the session's canonical GUID string ONLY (ADR-0030 Decision
    4: "treat the GUID as secret in logs/audit"; the sole existing precedent for showing any part
    of it outside the wire protocol is `session::group_title`'s own `[..8]` slice). NEVER the full
    36-character canonical string.
  - `pid`: `PeerCred.pid` (`src/hub/session.rs`) -- not secret, already visible via any OS process
    list on this same machine.
  - `owned_tab_ids`: the FULL, sorted set from `session::owned_tab_ids`, cross-referenced from the
    SAME `ServiceContext.owned_tabs` map every tool-call ownership gate already reads.
- `note`: the exact string above, verbatim (documents the honest limitation directly in the API
  response, not just in a doc comment nobody calling the API would see).

No other top-level field. Never includes a raw/full GUID, a `PeerUser` (OS SID/uid) string, or a
manifest/grant reference.

## CS4 -- `config_changed` session event (K5)

A NEW `SessionEventRecord.event` discriminator string, added the SAME way `"manifest_reload"` and
`"user_manifest_ignored"` were (ADR-0025; `src/governance/ports.rs`'s own doc comment: "Each new
session event adds its own string here, never a new record shape"):

- PINNED event string: `"config_changed"`.
- The record is built and recorded DIRECTLY via `ctx.recorder.record_session_event(&record)`
  (`ServiceContext.recorder: Arc<Recorder>`, `Recorder` already implements
  `governance::ports::AuditSink`, verified 2026-07-05: `impl AuditSink for Recorder { fn
  record_session_event(&self, record: &SessionEventRecord) { self.write_serialized(record,
  "session_event"); } }`). This is DIFFERENT from every existing session-event producer
  (`Governance::record_session_killed`/`record_manifest_reload`/`record_user_manifest_ignored`,
  `src/governance/dispatch.rs`), which are methods on a per-SESSION `Governance` facade -- the
  Console's POST handler has no `Governance` instance (it is a plain HTTP action on the shared
  service, not a tool-call dispatch through `serve_session`), so it calls the underlying
  `AuditSink` directly, exactly as `Governance::record_session_killed` itself ultimately does
  (`self.audit.record_session_event(&record)`) -- same sink, one call frame shallower. Do NOT add
  a method to `Governance` for this; `Governance` is per-session state the Console does not have.
- Field values (the frozen 6-key `SessionEventRecord` order,
  `event_id, ts, identity, client, event, manifest` -- UNCHANGED, transcribed from ADR-0030
  "Preserved invariants", the SAME oracle `docs/tasks/hub/H8-web-api-loopback-policy.md` transcribes):
  - `event_id`: `uuid::Uuid::new_v4().to_string()` (matches every existing producer).
  - `ts`: `chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)` (matches every
    existing producer).
  - `identity`: `None` (no authenticated principal model exists yet; see BOOTSTRAP's "must NEVER
    implement token mint/revoke" fence -- this is the SAME `None` every non-principal path uses,
    PINS.md SS2 in the Hub batch).
  - `client`: `None` (the Console's HTTP caller has no MCP `clientInfo`; there is no
    `Governance::current_client()` to read from since there is no per-session `Governance`).
  - `event`: `"config_changed"` (the literal above).
  - `manifest`: `None` (this event describes a CONFIG write, not a manifest swap; `manifest_reload`
    already owns the manifest-identity field's non-`None` case).
- This record is written on a SUCCESSFUL write only (mirroring `record_manifest_reload`'s own
  "callers only invoke this on a successful swap" rule) -- a refused (locked) write records
  nothing here (the HTTP 409 response IS the record of the refusal, to the caller; there is no
  audit trail requirement for a refused config write in this batch, matching the CLI's own
  `config set` refusal, which is also unaudited).

## CS5 -- `POST /api/v1/config/webapi-enable-remote` (K5)

Request: any body is IGNORED (never parsed, never required; a caller may send an empty body, `{}`,
or nothing meaningful under `Content-Length: 0` -- the handler must not attempt to read a request
body at all for this route in this batch). The value written is NOT caller-supplied; it is the ONE
PINNED literal below (BOOTSTRAP's "the Console must never write anything ... except the single ...
key" fence: this also means the Console never lets an HTTP caller choose an arbitrary
`channels.webapi.from` value).

- PINNED key: `channels.webapi.from` (CS8's new `CHANNELS_WEBAPI_FROM` constant).
- PINNED value written: `serde_json::json!(["*"])` (opens to every source; matches
  `docs/tasks/hub/PINS.md` SS7's own example of what "enabling remote" writes: `channels.webapi.from:
  [allow: "*"]`).
- Call: `crate::governance::config::cli::set_user_value(governance::config::CHANNELS_WEBAPI_FROM,
  serde_json::json!(["*"]), crate::browser::pattern::is_valid_pattern)` (CS7's extracted function;
  the `domain_pattern_valid` argument is IGNORED by this key's validation path -- `channels.webapi.from`
  is registered with `KeyConstraint::None`, CS8 -- but every call site threads the SAME function
  pointer, matching every other caller in the tree).

Response on success (`Ok(path)` from `set_user_value`): `200 OK`, `application/json`:

```json
{
  "key": "channels.webapi.from",
  "value": ["*"],
  "written_to": "<absolute path written, as returned>",
  "note": "takes effect the next time the Ghostlight service restarts"
}
```

(`written_to` is `path.display().to_string()`; the exact string is platform-dependent and NOT
itself asserted byte-for-byte by any pinned test -- assert only that it is present, non-empty, and
that `key`/`value`/`note` match the literals above exactly.)

Response on a locked key (`Err(crate::Error::Config(msg))` where `msg` is the EXISTING lock-refusal
message `run_set` already produces, verbatim: `"{key} is managed by your organization (source:
org_mandatory); 'config set' cannot override it"`): `409 Conflict`, `application/json`:

```json
{ "error": "channels.webapi.from is managed by your organization (source: org_mandatory); 'config set' cannot override it" }
```

Any OTHER `Err` from `set_user_value` (e.g. no writable user config directory on this platform;
`crate::Error::Config` with a different message) is also `409 Conflict` with `{"error": "<the
exact message>"}` -- there is no separate 5xx branch in this batch; every failure of this single
write action is a client-visible conflict, matching the CLI's own uniform `crate::Error::Config`
treatment.

No "disable remote" route exists in this batch (out of scope; ADR-0030 Decision 9 names only
"Enable remote connections" as the Console's write surface -- disabling is already possible via
`ghostlight config set channels.webapi.from '["localhost"]'` or by editing the user config file,
neither of which this batch touches).

Console UI requirement (K5, ADR-0030 Decision 5: "with a plain disclaimer"): the enable-remote
control MUST render, next to or above the action button, this PINNED disclaimer text verbatim:

> Enabling remote connections allows any device that can reach this machine's network address to
> connect to Ghostlight with no login. Only enable this on a trusted network. This takes effect
> the next time the Ghostlight service restarts.

The Console UI (any page K1-K5 render) MUST ALSO show this PINNED token note verbatim, per
BOOTSTRAP's "must NEVER implement token mint/revoke" fence, so the ADR's described feature is not
silently absent:

> Token mint/revoke: coming in a future release.

## CS6 -- `ConfigStore` gains a live `Resolution` accessor (K1)

`src/governance/config/reload.rs`'s `ConfigStore` (verified 2026-07-05) currently holds ONLY the
derived `Config` behind `snapshot: Mutex<Arc<Config>>`; the `layers::Resolution` that carries
per-key provenance is computed locally inside `load_initial_with_policy` and `apply_plan` and then
discarded (only `Config::from_resolution(&resolution)` is retained). There is NO live way to read
provenance from a running `ConfigStore` today -- the CLI gets it by calling
`resolve_with_warnings` fresh (`cli.rs`), an INDEPENDENT one-shot disk read in a separate process,
never the running service's own state.

PINNED addition, added the SAME way `snapshot`/`generation` already are:

- NEW field: `resolution: Mutex<Arc<layers::Resolution>>`.
- NEW accessor, mirroring `current()` exactly:
  ```rust
  /// The current in-force resolution (per-key provenance: value, source layer, lock state).
  /// Mirrors `Self::current()`'s exact Arc-clone-and-release shape; kept in sync with `snapshot`
  /// at the SAME two write sites (never a separate re-resolve).
  pub fn current_resolution(&self) -> Arc<layers::Resolution> {
      self.resolution
          .lock()
          .unwrap_or_else(PoisonError::into_inner)
          .clone()
  }
  ```
- Written at the SAME two call sites `snapshot` is written, with the SAME resolution value already
  computed there (never a second `layers::resolve` call):
  - `load_initial_with_policy`: immediately after `let resolution = layers::resolve(&inputs);`
    (before it is consumed by `Config::from_resolution(&resolution)`), wrap it in
    `Arc::new(resolution.clone())`... AUTHOR NOTE (transcribe exactly): `layers::Resolution`
    already derives `Clone` (verified 2026-07-05, `#[derive(Debug, Clone)] pub struct Resolution`),
    so store `Arc::new(resolution.clone())` in the new field and pass the ORIGINAL `resolution` by
    reference into `Config::from_resolution(&resolution)` unchanged -- do not change
    `from_resolution`'s call site or signature.
  - `apply_plan`: immediately after `let resolution = layers::resolve(&plan.inputs);` (before it is
    consumed by `Config::from_resolution(&resolution)`), same pattern: swap
    `Arc::new(resolution.clone())` into the `resolution` field's `Mutex` UNCONDITIONALLY (even when
    `changed` is `false` for the `Config` snapshot) -- provenance can differ even when the
    DERIVED typed `Config` compares equal (e.g. a value's SOURCE layer changed but its effective
    value did not), so the resolution field must always reflect the latest resolve, not be gated
    by `Config`'s `PartialEq`.
- Both test-only constructors (`for_test`, `for_test_with_config`, `for_test_with_user_source`,
  `cfg(test)` block at the bottom of `reload.rs`) must also seed the new field (a compile-forced
  deviation if you find a call site this list missed -- log it, do not guess a shortcut). Seed with
  `Arc::new(layers::resolve(&layers::LayerInputs::default()))` for the plain `for_test`/
  `for_test_with_config` paths (an empty-inputs resolution: every key at `Source::Builtin`,
  matching `for_test`'s own all-open/no-overlay posture) -- an exact literal is not asserted by any
  pinned test, so this default is not itself an oracle; just make it compile and be non-panicking.

## CS7 -- The extracted user-config write function (K1)

`src/governance/config/cli.rs`'s `run_set` (verified 2026-07-05, ~line 278) does, in order: look up
the `KeyDef` (`unknown_key_error` if absent), re-resolve current state via
`resolve_with_warnings`, refuse if `resolved.locked` (exact message transcribed in CS5 above),
parse the raw CLI string via `parse_cli_value`, validate via `def.parse_value(&parsed,
domain_pattern_valid)` (return value discarded -- it exists to validate, not to transform), then
call the PRIVATE `fn write_user_value(key: &str, value: &serde_json::Value) ->
crate::Result<std::path::PathBuf>` (same file, ~line 227) and print CLI output.

PINNED extraction: everything from the lock-check through `write_user_value`'s call (i.e.
EVERYTHING except `parse_cli_value` -- a CLI-string-to-JSON step the Console never needs, since its
one write in this batch is a compile-time literal, CS5 -- and the two `println!` lines) becomes a
NEW `pub(crate)` function in the SAME file (`cli.rs`), so the CLI and the Console call the exact
SAME code:

```rust
/// Lock-check, validate, and write ONE key to the user layer (the shared body of `run_set`,
/// pulled out so the Console (`src/hub/webapi.rs`) writes through the identical path the CLI
/// does -- never a second implementation of "write one key to the user layer").
pub(crate) fn set_user_value(
    key: &str,
    value: serde_json::Value,
    domain_pattern_valid: fn(&str) -> bool,
) -> crate::Result<std::path::PathBuf> {
    let def = key_def(key).ok_or_else(|| unknown_key_error(key))?;

    let (resolution, _warnings, _loaded_policy) = resolve_with_warnings(domain_pattern_valid)?;
    let resolved = resolution.get(key).expect("registered key resolves");
    if resolved.locked {
        return Err(crate::Error::Config(format!(
            "{key} is managed by your organization (source: org_mandatory); \
             'config set' cannot override it"
        )));
    }

    def.parse_value(&value, domain_pattern_valid)
        .map_err(|e| crate::Error::Config(format!("invalid value for {key}: {e}")))?;

    write_user_value(key, &value)
}
```

`run_set` is then reduced to: `parse_cli_value` -> `set_user_value(key, parsed, domain_pattern_valid)`
-> the two `println!` lines on `Ok`, propagating `Err` unchanged via `?`. This must be a PURE
extraction (identical behavior, identical error messages) -- the existing `cli.rs` unit tests
(`lock_refusal_exact_message_and_no_file_touched`, the `write_*` tests) must stay green with NO
assertion changed.

`src/hub/webapi.rs` (K5) calls `crate::governance::config::cli::set_user_value(...)` directly (the
`cli` module is already `pub mod cli;` from `config/mod.rs`; `pub(crate)` visibility reaches any
module in this crate, including `src/hub`).

## CS8 -- The `channels.webapi.from` config key (K1)

`channels.webapi.from` exists TODAY only as: (a) a raw manifest-grant-schema concept (ADR-0030's
deferred `channels` axis) and (b) `src/hub/webapi.rs`'s own HARDCODED `builtin_webapi_from()`
(`vec!["localhost".to_string()]`), which `webapi::run()` uses UNCONDITIONALLY -- it is NOT read
from `ConfigStore` at all (confirmed 2026-07-05 by reading `webapi.rs`'s own module doc: "This
task does not yet wire a `ConfigStore`-resolved override for `channels.webapi.from`/`webapi.bind`
(deferred -- see the H8 ledger entry); today the running service always resolves to the builtin
default"). Without closing this gap, K5's write action would have NO live effect at all (the
already-running service would keep enforcing the hardcoded builtin forever). This batch closes
JUST ENOUGH of that gap for the write action to be real, without reopening the full deferred
recursive grant grammar (ADR-0030 "Governance schema section": that grammar stays deferred to its
own core-only ADR).

PINNED registration, `src/governance/config/mod.rs` (same shape as every existing `KeyDef`; NO
`Config` struct field or accessor is added for it -- nothing needs a typed `Config` accessor for
this key; both consumers, K3's `/api/v1/config` and K1's live-webapi-read wiring below, read the
raw `layers::Resolution` directly via CS6's new accessor):

```rust
/// `channels.webapi.from` -- sources allowed to connect to the local web API (Console/HTTP
/// listener, ADR-0030 Decision 5/9). `["localhost"]` (loopback only) unless the machine owner
/// explicitly opens it via the Console's "Enable remote connections" (`src/hub/webapi.rs`) or an
/// org override. Governs WHO may connect, never which tools exist (ADR-0030 Decision 6 is
/// preserved: this key has no bearing on the tool/resource axes).
pub const CHANNELS_WEBAPI_FROM: &str = "channels.webapi.from";
```

Added as a new entry in the `KEYS` array:

```rust
KeyDef {
    key: CHANNELS_WEBAPI_FROM,
    description: "Sources allowed to connect to the local web API (Console/HTTP). \"localhost\" only, unless opened to \"*\" or specific hosts.",
    constraint: KeyConstraint::None,
    default_fully_open: KeyValue::StrList(&["localhost"]),
    default_safe: KeyValue::StrList(&["localhost"]),
    default_restricted: KeyValue::StrList(&["localhost"]),
},
```

(All three preset defaults are `["localhost"]`, matching `builtin_webapi_from()`'s value exactly,
so the Builtin/layer-5 default for this key is byte-identical to today's hardcoded constant --
this registration changes NO resolved value for any session that has not touched the Console.
`KeyConstraint::None` is a deliberate, pinned choice: `channels.webapi.from`'s own fail-closed
grammar validator, `governance::channels::validate_webapi_from`, already rejects a non-array or an
empty-string member when the value arrives via a MANIFEST's grant refinement; this config-registry
path only ever writes the ONE pinned literal `["*"]` (CS5) or the ONE pinned default `["localhost"]`,
so the weaker `KeyType::StrList` base-type check (rejects a non-array, a non-string member, and a
duplicate member; does not reject an empty-string member) is sufficient and consistent with how
`AUDIT_SYSLOG_ADDRESS` -- a Str key with no format constraint beyond "is a string" -- is already
registered.)

### CS8.1 -- Golden files must be regenerated (expected, non-sacred ripple)

Adding this `KeyDef` changes `render_config_schema()`/`render_key_reference()`'s output.
`tests/config_schema_golden.rs` pins these against checked-in files
(`tests/golden/config-schema.json`, `tests/golden/config-keys.md`) BY DESIGN ("Any registry change
... must fail HERE until [they] are regenerated and reviewed deliberately"). This is NOT a
NEVER-touch fence; it is the sanctioned, expected update path. Regenerate with:

```
cargo run -- config schema > tests/golden/config-schema.json
cargo run -- config docs > tests/golden/config-keys.md
```

then run `cargo test --test config_schema_golden` and diff-review the two files before committing
(verify both are LF-only, no `\r`; the repo pins LF via `tests/golden/.gitattributes` -- if your
shell introduced CRLF, strip it before committing). This IS part of K1's own commit (the golden
files change because K1's own registry change caused it), not a separate task.

### CS8.2 -- `src/hub/webapi.rs::run` reads the live resolved value at startup (K1)

`webapi::run(ctx: ServiceContext)` currently opens with `let allowlist = builtin_webapi_from();`
(hardcoded, ignoring `ctx` entirely for this purpose even though `ctx: ServiceContext` is already
its only parameter). PINNED change: resolve the STARTUP allowlist from the live `ConfigStore`
instead:

```rust
let allowlist = live_channels_webapi_from(&ctx.store);
```

with a new pure-ish helper in `webapi.rs`:

```rust
/// The live `channels.webapi.from` allowlist (PINS.md CS8), read from the store's current
/// resolution (CS6). Every registered key always resolves (`layers::resolve` is infallible), so
/// this never falls back to `builtin_webapi_from()` in practice; `expect` matches the existing
/// idiom in `governance::config::mod` (`resolution.get(key).expect("registered key")`).
fn live_channels_webapi_from(store: &crate::governance::config::reload::ConfigStore) -> Vec<String> {
    let resolution = store.current_resolution();
    let resolved = resolution
        .get(crate::governance::config::CHANNELS_WEBAPI_FROM)
        .expect("registered key resolves");
    resolved
        .value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(builtin_webapi_from)
}
```

This is read ONCE at `run()`'s startup, exactly where `builtin_webapi_from()` was read before (the
`resolve_bind`/`TcpListener::bind` call immediately after it is UNCHANGED -- the bind address is
still decided once, at process start; REBINDING the TCP listener live is explicitly OUT OF SCOPE
for this batch, see CS5's disclaimer text "takes effect the next time the Ghostlight service
restarts"). PER-CONNECTION authorization, however, SHOULD reflect a live edit without a restart
(narrowing or widening WHO may connect is a policy decision, not a bind decision): change the
accept loop so EACH accepted connection re-reads `live_channels_webapi_from(&ctx.store)` fresh
(cheap: one `Arc` clone plus a linear scan of ~9 registered keys) instead of reusing the
loop-hoisted `allowlist.clone()` it uses today. `bind` itself (used only for the `Host`-header
DNS-rebind check, `host_is_expected`) stays the ONE value resolved at startup -- do not re-resolve
`bind` per connection.

`builtin_webapi_from()` itself is UNCHANGED and stays exported (existing tests
`tests/webapi_auth.rs`, `tests/channels_policy.rs`, and `webapi.rs`'s own unit tests call it
directly as a pure function describing the adapter's default policy fragment; this remains true
and their existing assertions are unaffected by this task, since a fresh `ConfigStore` with no
user/org overlay resolves this key to the SAME value `builtin_webapi_from()` returns).

## CS9 -- `SessionRegistry` read-only accessor (K1)

`src/hub/session.rs`'s `SessionRegistry` (verified 2026-07-05) holds a PRIVATE `bindings:
HashMap<String, PeerCred>` (full canonical GUID string -> bound peer), written only by `admit`,
NEVER pruned on disconnect (by design -- H3's same-user reconnect path needs the binding to
persist). There is no live way to enumerate it today.

PINNED addition, `src/hub/session.rs` (same module, so it may read `SessionRegistry.bindings`'s
private field and `SessionGuid`'s private inner string directly, following `group_title`'s own
established precedent for the ONE sanctioned partial-GUID exposure):

```rust
/// One admitted binding's Console-safe summary (PINS.md CS3/CS9): the FIRST 8 CHARACTERS of the
/// GUID ONLY (never the full canonical string, ADR-0030 Decision 4), the peer's OS process id
/// (not secret), and its full current owned-tab set.
pub struct SessionSummary {
    pub guid: String,
    pub pid: u32,
    pub owned_tab_ids: Vec<i64>,
}

/// Read-only snapshot for the Console's sessions view (PINS.md CS3). HONEST LIMITATION
/// (transcribed into the API response too, CS3): `registry`'s bindings are never pruned on
/// disconnect, so an entry here may no longer be live right now; pair this with
/// `ServiceContext.live_sessions` for an accurate CURRENT count. Acquires `registry`'s lock only
/// long enough to clone the (guid, PeerCred) pairs out, then drops it before acquiring
/// `owned_tabs`'s SEPARATE lock per entry (via the existing `owned_tab_ids`), so the two locks are
/// never held simultaneously.
pub fn live_session_summaries(
    registry: &Mutex<SessionRegistry>,
    owned_tabs: &Mutex<HashMap<i64, SessionGuid>>,
) -> Vec<SessionSummary> {
    let bindings: Vec<(String, PeerCred)> = {
        let reg = registry.lock().unwrap_or_else(PoisonError::into_inner);
        reg.bindings
            .iter()
            .map(|(g, c)| (g.clone(), c.clone()))
            .collect()
    };
    bindings
        .into_iter()
        .map(|(full_guid, cred)| {
            let guid = SessionGuid::parse(&full_guid)
                .expect("registry keys are valid canonical guids (only admit() inserts them)");
            SessionSummary {
                guid: full_guid[..8].to_string(),
                pid: cred.pid,
                owned_tab_ids: owned_tab_ids(owned_tabs, &guid),
            }
        })
        .collect()
}
```

`full_guid[..8]` never panics: every key in `bindings` was inserted via
`guid.as_str().to_string()` where `guid` was itself minted or `SessionGuid::parse`d (both produce
a canonical, hyphen-at-index-8, >= 36-character string) -- the SAME safety argument `group_title`'s
own doc comment already makes for its identical slice.

`src/hub/webapi.rs` (K4) calls `crate::hub::session::live_session_summaries(&ctx.session_registry,
&ctx.owned_tabs)` (both fields already `pub` on `ServiceContext`).

## CS10 -- Static asset embedding (K2)

Follow the existing, sole embedding convention in this codebase (verified 2026-07-05:
`src/transport/mcp/tools.rs`'s `TOOLS_JSON` via `include_str!`; CLAUDE.md: "define tool schemas as
const string literals ... rather than building them programmatically"; ADR-0001's zero-new-dependency
posture -- no `rust-embed` or similar crate is used anywhere in this tree and none is added here).

- NEW files, checked into the repo (plain static text, not generated at build time beyond
  `include_str!` itself):
  - `src/hub/console/index.html`
  - `src/hub/console/console.css`
  - `src/hub/console/console.js`
- NEW module `src/hub/console_assets.rs` (add `pub mod console_assets;` to `src/hub/mod.rs`'s
  existing alphabetized `pub mod` block):
  ```rust
  //! Embedded static assets for the Console (ADR-0030 Decision 9; PINS.md CS10). Plain
  //! `include_str!` const literals, matching the sole embedding convention already used for
  //! `TOOLS_JSON` (`src/transport/mcp/tools.rs`) -- no new crate dependency.
  pub const INDEX_HTML: &str = include_str!("console/index.html");
  pub const CONSOLE_CSS: &str = include_str!("console/console.css");
  pub const CONSOLE_JS: &str = include_str!("console/console.js");
  ```
- `src/hub/webapi.rs`'s new router (K2) serves these three constants verbatim for `GET /`,
  `GET /console.css`, `GET /console.js` respectively, with the Content-Type from CS1's table and
  `Content-Length` computed from the actual byte length of the constant (UTF-8 byte length, not
  character count).
- Content requirements (K2 writes the actual markup/script/style; no specific byte-for-byte HTML is
  pinned by this batch beyond what CS2/CS3/CS5 require the RENDERED page to eventually show once
  K3/K4/K5 land): `index.html` MUST link `console.css` and `console.js` by the exact paths
  `/console.css` and `/console.js` (relative paths that resolve to those routes); it MUST render
  the CS5 token-mint/revoke note somewhere visible (K2 may add a placeholder container K3/K4/K5
  fill in, or render the whole page's content progressively across K2-K5 -- either is acceptable,
  as no task's own test asserts exact HTML byte-content, only that the named JSON/text routes
  return their pinned shapes and that the page is fetchable at all, per K2's own named test).

## CS11 -- Test-unique web API port (K2, required test plumbing)

`src/hub/webapi.rs::run` currently binds the HARDCODED `DEFAULT_WEBAPI_PORT` (`4180`) with no
override, so two concurrently-spawned real `ghostlight service` processes (as every test that uses
`tests/support::spawn_service` already spawns) would collide on the same TCP port when `cargo
test` runs test binaries/tests in parallel (the default). No existing test connects to this port
today (grepped 2026-07-05, zero matches for `4180`/`webapi::run`/`DEFAULT_WEBAPI_PORT` under
`tests/`), so this collision has never been hit -- K2 is the first task that needs a REAL TCP fetch
against a REAL spawned service and must close it.

PINNED mechanism, mirroring the EXISTING `GHOSTLIGHT_ENDPOINT` env-override convention
(`src/transport/native/ipc.rs::default_endpoint`) exactly:

- `src/hub/webapi.rs` gains:
  ```rust
  /// The web API TCP port: the `GHOSTLIGHT_WEBAPI_PORT` env override (tests and advanced
  /// deployments that run more than one isolated instance on a host), else `DEFAULT_WEBAPI_PORT`.
  /// Mirrors `native::ipc::default_endpoint`'s exact override convention.
  pub fn resolve_webapi_port() -> u16 {
      std::env::var("GHOSTLIGHT_WEBAPI_PORT")
          .ok()
          .and_then(|s| s.parse().ok())
          .unwrap_or(DEFAULT_WEBAPI_PORT)
  }
  ```
  `run()`'s `let addr = format!("{bind}:{DEFAULT_WEBAPI_PORT}");` becomes
  `let port = resolve_webapi_port(); let addr = format!("{bind}:{port}");`.
- `tests/support/mod.rs` gains ONE new additive helper (every EXISTING function in that file --
  `spawn_service`, `spawn_service_with_manifest`, `spawn_service_with_program_data`,
  `spawn_adapter` -- is UNCHANGED; this batch never edits their bodies or signatures):
  ```rust
  /// Like `spawn_service`, but with `GHOSTLIGHT_WEBAPI_PORT` forwarded (PINS.md CS11: avoids a
  /// fixed-port collision between concurrently-spawned real services in `cargo test`'s default
  /// parallel test execution). Console-batch tests that fetch the real web API over TCP use this
  /// instead of `spawn_service`.
  pub fn spawn_service_with_webapi_port(endpoint: &str, port: u16) -> Child {
      let log_dir = log_dir_for(endpoint);
      let _ = std::fs::remove_dir_all(&log_dir);
      let child = Command::new(bin())
          .arg("service")
          .env("GHOSTLIGHT_ENDPOINT", endpoint)
          .env("GHOSTLIGHT_WEBAPI_PORT", port.to_string())
          .env("GHOSTLIGHT_DEBUG", "1")
          .env("GHOSTLIGHT_LOG_DIR", &log_dir)
          .stdin(Stdio::null())
          .stdout(Stdio::null())
          .stderr(Stdio::null())
          .spawn()
          .expect("spawn ghostlight service");
      wait_for_debug_state(&log_dir, Duration::from_secs(15));
      child
  }
  ```
- Test-unique port derivation (author-pinned; not itself asserted by any test, just needs to avoid
  collision in practice): the SAME `AtomicU32` sequence-plus-pid pattern
  `tests/hub_completion_criteria.rs` already uses to make `endpoint` unique, mapped into a
  private, unlikely-to-be-in-use range:
  ```rust
  fn test_webapi_port(seq: u32) -> u16 {
      20000 + ((std::process::id() as u32).wrapping_add(seq) % 10000) as u16
  }
  ```
  Each new test that needs a real TCP fetch declares its OWN `static SEQ: AtomicU32` (or reuses one
  already declared in the SAME test file, if a task adds more than one such test to the same file)
  the same way `tests/hub_completion_criteria.rs` does; this is test-file-local scaffolding, not a
  cross-file shared oracle.
