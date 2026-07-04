# Ghostlight Hub batch: PINS (author oracle sheet)

Every value here is PINNED by the batch author. The executor TRANSCRIBES these; it never derives or
invents one (the ORACLE RULE, BOOTSTRAP). Where a task file says "PINNED in PINS.md SS<n>", use the
value below verbatim. Semantics live here in one place; the task files cite, they do not re-decide.

## SS1 -- The Hub handshake (shared by H2, H3, H7)

Every connection to the core endpoint opens with ONE "hello" frame, carried ON TOP OF the existing
4-byte-LE `host.rs` framing (NEVER a change to that framing). The hello is a JSON object:

```
{ "hub": 1, "role": "<role>", "guid": "<uuid-v4>"? }
```

- `hub`: the protocol major. PINNED constant `pub const HUB_PROTO: u32 = 1;`, defined in a new module
  `src/hub/handshake.rs` (created by H2).
- `role`: exactly one of the PINNED strings `"ext"` (the Chrome-spawned native-host RELAY),
  `"adapter"` (an MCP stdio adapter), `"control"` (doctor/console; reserved, not used before H8).
  PINNED constants `ROLE_EXT = "ext"`, `ROLE_ADAPTER = "adapter"`, `ROLE_CONTROL = "control"` in
  `src/hub/handshake.rs`.
- `guid`: present ONLY for `role == "adapter"` (and the H8 web session); it is the adapter-minted
  session GUID (SS see H3). Absent for `"ext"` and `"control"`.

H2: `serve` reads the hello frame first and demuxes: `"ext"` -> `Browser::attach` (unchanged single
physical-link path); `"adapter"` -> `serve_session` on the shared `ServiceContext`. H2 also updates
`relay_native_host` to send `{"hub":1,"role":"ext"}` on connect, and the new `relay_adapter` to send
`{"hub":1,"role":"adapter","guid":<guid>}`.
H3: the adapter's `guid` in this same hello frame is the session GUID; do not invent a second frame.
H7: the group-request (SS6) is a session-scoped message AFTER the hello, never part of it.

## SS2 -- The authenticated subject's audit home (resolves the H8 vs 14-key tension)

The authenticated subject does NOT add a 15th audit key. It populates the EXISTING `identity` field
(position 3 of the frozen 14-key order; `AuditRecord.identity: Option<Identity>` where
`Identity { principal, resolved_by }` already exists in `src/governance/ports.rs`, currently always
built as `None` in `dispatch.rs::build_record`).

- Local adapter session, or an anonymous web caller, or any all-open session: `identity = None`
  (BYTE-IDENTICAL to today; `all_open_golden` and `audit_recorder` stay green untouched).
- A web session whose policy named a principal: `identity = Some(Identity { principal: <the named
  principal>, resolved_by: "webapi" })`.

So "distinct from the self-reported `clientInfo`" (ADR-0030 Decision 9) means the existing `identity`
field, which is separate from the `client` field. No new key; the 14-key order is preserved.

## SS3 -- H4 unowned-tab refusal

- Uniform, leak-free result (IDENTICAL for ANY tabId not in the session's owned set -- whether it
  exists in another session or does not exist at all; the gate runs BEFORE any extension query and
  cannot distinguish the two, so it is uniform by construction): a SUCCESSFUL MCP text result, NOT an
  error. This follows the system's denial convention -- denials render as a normal text result, never
  `isError` (see the hold/deny path at pipeline.rs:109/193 and `hold_message`). It carries only the
  PINNED text `unknown tab` -- no host, no tabId echo.
- It IS recorded, as a deny: `decision = "deny"`, `domain = null` (the host is NEVER resolved for an
  unowned tab -- resolving it is the very leak being closed), `held = false`, `duration_ms = 0`.
- `denial_id`: computed by the existing scheme (`denial.rs`: `"D-"` + 8 lowercase hex); the rule
  label is PINNED as `cross_session/unowned_tab`. Do not hardcode a literal id (it derives from the
  manifest hash at runtime); assert the `"D-"` prefix + 8 hex shape, mirroring existing denial tests.

## SS4 -- H5 constants

- `pub const GRACE_WINDOW: Duration = Duration::from_secs(10);` (strictly < the 60s `TOOL_TIMEOUT`).
- `pub const PER_PEER_MINT_CAP: usize = 32;` (max concurrent GUID sessions per minting peer identity).
- `pub const PER_PEER_GROUP_CAP: usize = 32;` (max live tab groups per peer identity; equal to the
  mint cap by design).
- Quota-exceeded result: a plain tool error, PINNED text `session limit reached for this client`
  (no global lockout -- a flooding peer is denied while other peers are unaffected; the test asserts
  a second, different peer still succeeds).
- `pub const SCREENSHOT_CHUNK_THRESHOLD: usize = 8 * 1024 * 1024;` (payloads at/above 8 MiB are
  chunked; well under the `host.rs` `MAX_MESSAGE_LEN`). Chunking is on the SERVICE<->adapter/web hop
  only, NEVER the frozen extension `host.rs` wire.
- The `oversized_screenshot_is_chunked_not_head_of_line_blocking` test's completion bound for the
  small interleaved call: PINNED at `< 2s` (a tiny call must complete while a chunked large payload
  streams).

## SS5 -- H6 constants

- `pub const IDLE_GRACE: Duration = Duration::from_secs(30);` (the service exits only after no
  sessions AND the extension link gone for this window).
- Anti-squat: the service proves possession of a per-install secret. PINNED shape: the secret is 32
  random bytes at `<data-dir>/hub-key` (0600 / DPAPI-per-user), generated on first service start; on
  connect the service sends `{"hub":1,"role":"service-proof","mac":<hex hmac-sha256(secret, the
  adapter's hello bytes)>}` and the adapter verifies it reads the same file. On mismatch the adapter
  aborts with PINNED text `refusing to connect: the Ghostlight service on this endpoint is not the
  one this user installed`. (If a task cannot read the same file, that is the impostor case.)
- `data-dir`: the existing `%ProgramData%\ghostlight` / platform equivalent already used by the
  debug/session files -- RE-READ `src/debug.rs` / the session-dir helper; do not invent a new dir.
- Debug/session role labels: the SERVICE keeps the existing `"mcp-server"` role label (so `doctor`'s
  session listing and its parsing are undisturbed); the ADAPTER gets a new `"adapter"` label at its
  `build_debug_sink` call site. `doctor::reap` (doctor.rs:600; role filter at doctor.rs:86/465) is
  re-scoped to reap orphaned `"adapter"` sessions (parent editor dead), NEVER the service (idle-grace
  only, never parent-reaped).

## SS6 -- H7 group request

- Message type: PINNED `"group_request"` (additive; alongside the existing native-messaging message
  types in `messages.rs` -- must not alter any existing shape). Fields:
  `{ "type": "group_request", "guid": <session guid>, "tabIds": [<i64>...], "title": <string> }`.
  The extension replies with `{ "type": "group_response", "guid": <guid>, "ok": <bool> }`.
- Per-session group title: PINNED format `\u{1F47B} Ghostlight <short>` where `<short>` is the first
  8 chars of the GUID -- matches the existing `GROUP_TITLE` ghost-glyph convention in
  `service-worker.js` (RE-READ it; keep the glyph as the `\u{1F47B}` escape, ASCII source).
- Grouping module (extension side): a PURE module (e.g. `extension/lib/grouping.js`, following the
  existing `extension/lib/` IIFE pattern) that `service-worker.js` imports and calls ON a
  `group_request` ONLY, to run `chrome.tabs.group`/`tabGroups` for the named tabs and title the
  group. It makes NO policy decision (owns durable group state only) and is unit-testable in
  isolation (the `tests/extension/grouping.test.js` target). Service side: `src/hub/session.rs` sends
  the request for a session's owned tabs (from H4); reuse of the same GUID reuses the group.

## SS7 -- H8 channels + web bind

- `channels.webapi.from` denial: rule label PINNED `channel/webapi_from`; result a plain deny with
  `decision = "deny"`, `denial_id` the existing `"D-"` + 8-hex scheme (assert the shape, not a
  literal). The web adapter's BUILTIN default fragment is `channels.webapi.from: { allow: ["localhost"] }`.
- Bind representation: a resolved config value `webapi.bind` (string). PINNED default `"127.0.0.1"`
  (bound EXPLICITLY; never `0.0.0.0`). The Console "Enable remote connections" writes a user-layer
  `webapi.bind` (e.g. `"0.0.0.0"`) AND the matching `channels.webapi.from` entry -- both are ordinary
  policy/config writes, never a code gate. The port: PINNED default `webapi.port = 4180`.
- The authenticated subject is recorded via the `identity` field per SS2 -- NOT a new audit key.

## Resolved AUTHOR-MUST-PIN index (so none is left open)

| Task | value | pinned in |
| --- | --- | --- |
| H2 | role discriminator / hello frame + relay_native_host ext_hello | SS1 |
| H2 | distinct client-name constructor | use `Governance::all_open` + `set_client(name, version)` as today (RE-READ H1; no new constructor) |
| H4 | uniform "unknown tab" string + audited-as-deny + domain/denial | SS3 |
| H5 | grace window, per-peer caps, quota message, oversize threshold + chunk, completion bound | SS4 |
| H6 | idle-grace, anti-squat failure string, per-install secret storage + proof shape | SS5 |
| H7 | group_request type + fields + reply, grouping fn, group title format | SS6 |
| H8 | channels denial rule/message/id, remote-bind representation, trusted-subject audit field | SS7 + SS2 |
