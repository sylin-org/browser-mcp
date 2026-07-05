# K2: Console static GET routes in src/hub/webapi.rs

Cites: `docs/adr/0030-ghostlight-hub-orchestrator.md` Decision 9; `docs/tasks/console/PINS.md`
CS1, CS1.1, CS1.2, CS1.3, CS1.4, CS10, CS11. Needs K1 DONE (this task's router calls nothing from
K1 directly, but K3/K4/K5 extend THIS task's route table, so landing K1 first keeps every prefix
of the batch coherent per BOOTSTRAP.md's stated sequence). Read `docs/tasks/console/BOOTSTRAP.md`
in full first.

## What this task is

Serves the Console's own embedded HTML/CSS/JS as new GET routes on the SAME TCP listener H8's web
API already runs, gated by the SAME `channels.webapi.from` policy decision the WS upgrade already
uses -- and closes a real port-collision gap (CS11) that blocks any test in this batch from
actually fetching over real TCP. No config or session DATA is served yet (that is K3/K4); this
task is the page shell + routing + auth wiring only.

## Current-tree facts (re-verify against the live tree; do not trust a stale line number)

- `src/hub/webapi.rs`'s `handle_connection` (verify its current line range with `grep -n "async fn
  handle_connection" src/hub/webapi.rs`) does, today, in order: read the request head via
  `parse_http_request`, require a `Sec-WebSocket-Key` header (400 if absent), require
  `GET` + `Upgrade: websocket` (400 if not), require an expected `Host` (400 if not), decide
  `channels.webapi.from` via `ChannelsPdp` on the Origin-or-classified-peer source (403 if
  denied), then complete the RFC 6455 handshake and hand off to `serve_session`.
- `HttpRequest` (same file) is `struct HttpRequest { method: String, headers: Vec<(String,
  String)> }`; `parse_http_request` currently reads only the method token from the request line,
  discarding the path token entirely.
- `write_http_error(stream, status, reason)` (same file) already builds a generic
  `Connection: close`, no-body HTTP error response for any `(status, reason)` pair -- it is not
  hardcoded to 400/403 text, it takes them as parameters.
- `DEFAULT_WEBAPI_PORT` (`4180`) is currently the ONLY value `run()` binds to; nothing in
  `tests/` references port `4180` or `DEFAULT_WEBAPI_PORT` today (re-confirm with `grep -rn
  "4180\|DEFAULT_WEBAPI_PORT\|webapi::run" tests/`) -- this task is the first to need a REAL TCP
  fetch against a REAL spawned service, so the fixed-port collision under `cargo test`'s parallel
  execution must be closed now, per PINS.md CS11.
- `tests/support/mod.rs`'s `spawn_service`/`spawn_adapter`/etc. are used, unmodified, by every
  existing test file that spawns the real binary. This task adds ONE new function to that file
  (CS11's `spawn_service_with_webapi_port`) and touches nothing else in it.

## STOP preconditions

- If `handle_connection`'s current control flow does not match the summary above closely enough
  that CS1's "runs BEFORE the existing Sec-WebSocket-Key-required check" placement is ambiguous,
  STOP and describe the actual control flow found rather than guessing where to insert the router.
- If inserting the new router changes the byte-for-byte behavior of an actual WS-upgrade request
  (verify by re-running `tests/webapi_auth.rs` and `tests/channels_policy.rs` after your change --
  every existing assertion in both files must still pass unmodified), STOP; do not adjust an
  existing assertion to make it pass.

## Required behavior

1. **CS1.4**: add a `path: String` field to `HttpRequest`, populated from the request line's
   second whitespace-separated token, in `parse_http_request`. Do not strip a query string here;
   stripping happens once in the new router (CS1).
2. **CS1**: before the existing `Sec-WebSocket-Key`-required check, add a router that claims a
   request ONLY when it has no `Upgrade: websocket` header (or a non-`websocket` value) AND its
   path (with any `?...` suffix stripped) matches a row in CS1's table, OR falls under `/` or
   `/api/v1/**` with no matching row (CS1.1/CS1.2). Anything the router does NOT claim (a WS
   upgrade attempt, or any other plain HTTP path) falls through completely unchanged to the
   EXISTING logic. For a claimed request in THIS task's scope (K2 only wires `GET /`,
   `GET /console.css`, `GET /console.js`; K3/K4/K5 add their own rows to the SAME table later):
   authorize via the SAME `channel_decision_request`/`classify_source`/`origin_hostname`/
   `ChannelsPdp` sequence the WS-upgrade path already uses (403 per CS1.3 on denial, using the
   SAME `write_http_error` call, never a new response shape), then serve the matching embedded
   constant (CS10) with the Content-Type from CS1's table and a correct `Content-Length` (UTF-8
   byte length).
3. **CS1.1/CS1.2**: implement the 404 and 405 responses exactly as specified (literal ASCII
   bodies, exact byte lengths given in PINS.md -- transcribe them, do not recompute).
4. **CS10**: create `src/hub/console/index.html`, `src/hub/console/console.css`,
   `src/hub/console/console.js` (plain static files) and `src/hub/console_assets.rs` (the three
   `include_str!` const literals), add `pub mod console_assets;` to `src/hub/mod.rs`'s existing
   alphabetized `pub mod` block. `index.html` must link `console.css` and `console.js` at exactly
   `/console.css` and `/console.js`. Render the CS5 token-mint/revoke note ("Token mint/revoke:
   coming in a future release.") somewhere visible in this task's own shell (a later task may
   restructure the page, but the note must be present starting now, not deferred silently).
5. **CS11**: add `resolve_webapi_port()` to `webapi.rs` and change `run()` to bind
   `{bind}:{resolve_webapi_port()}` instead of the hardcoded `DEFAULT_WEBAPI_PORT`. Add
   `spawn_service_with_webapi_port(endpoint, port)` to `tests/support/mod.rs` exactly as CS11
   shows, touching no existing function in that file.

## Tests to write FIRST

New file `tests/console_static_routes.rs` (`mod support;`), spawning a real service via
`support::spawn_service_with_webapi_port` on a test-unique port (CS11's `test_webapi_port`
pattern, a private `static SEQ: AtomicU32` in this file, mirroring
`tests/hub_completion_criteria.rs`):

- `console_index_page_is_served_over_a_real_http_get`: a real TCP `GET / HTTP/1.1` with a `Host:
  127.0.0.1:<port>` header (no `Upgrade`/`Sec-WebSocket-Key`) gets back `200 OK`,
  `Content-Type: text/html; charset=utf-8`, and a body containing the literal substrings
  `/console.css` and `/console.js` (proving CS10's linking requirement, not exact byte content).
- `console_css_and_js_are_served_with_correct_content_type`: real GETs to `/console.css` and
  `/console.js` each return `200 OK` with the Content-Type CS1's table pins.
- `unknown_path_under_root_is_404`: a real GET to a path outside CS1's table (e.g. `/nope`) and
  under `/api/v1/` (e.g. `/api/v1/nope`) each return `404 Not Found` with the exact 9-byte body
  `not found` (transcribe CS1.1 verbatim).
- `wrong_method_on_a_known_path_is_405`: a real `POST /` and a real `GET
  /api/v1/config/webapi-enable-remote` (a path this task does not itself implement yet, but whose
  METHOD mismatch this task's router logic must still handle correctly once K5 registers the
  route -- if K5 has not landed yet when this test runs, assert the CURRENT behavior for a path
  not yet in the table instead, i.e. 404, and leave a comment for K5 to add the true 405 case once
  its own route exists) return `405 Method Not Allowed` with the exact 18-byte body `method not
  allowed` (transcribe CS1.2 verbatim) for any row THIS task's own table already has (i.e. test
  `POST /` only, in this task; K5 adds the `POST`-path 405 case for its own new GET-only siblings
  if any, as part of ITS OWN named tests).
- `a_real_ws_upgrade_request_is_unaffected`: re-run (or newly write, if none exists in this exact
  shape) a real WS-upgrade handshake against `/` on the SAME spawned service and confirm it still
  succeeds exactly as `tests/webapi_auth.rs`/`tests/channels_policy.rs` already prove elsewhere --
  this is a belt-and-suspenders confirmation local to this new file, not a replacement for running
  the existing suites (which you must also run and keep green).

## Out of scope

- No config or session data endpoints (K3, K4).
- No write action (K5).
- No change to `MAX_HANDSHAKE_BYTES`, the RFC 6455 handshake/frame code, or any existing
  WS-upgrade response text.
