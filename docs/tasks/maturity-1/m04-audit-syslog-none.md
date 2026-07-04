# M04: audit destinations syslog (RFC 5424 / UDP) and none

## Goal

ADR-0026 Decision 4: add the `syslog` and `none` audit destinations next to
the existing `file` and `stderr`, hot-reloadable, with the config key and
goldens updated. `http` stays deferred.

## Authority

ADR-0026 Decision 4; 00-design.md "Audit destinations (m04)" (the wire format,
transport, and failure semantics are pinned there and are not restated here).

## Depends on

m02 (headers exist; new code you add sits under files that already carry the
governance SPDX header). STOP preconditions: `rg -n "EnumVariants\(&\[\"file\",
\"stderr\"\]\)" src/governance/config/mod.rs` matches exactly once, and
`rg -n "fn resolve_inner" src/governance/audit/mod.rs` matches. If either
fails, STOP.

## Current behavior (verified 2026-07-03 pre-m02; re-read every site before editing)

NOTE: m02 runs before this task and inserts an SPDX header at line 1 of every
.rs file, so every line number below is +1 (or more) once m02 has landed. Locate
each site by its CONTENT (rg for the symbol), not by line number; the numbers
are the pre-m02 anchors only.


- src/governance/audit/mod.rs: private `enum Inner { File(PathBuf), Stderr }`
  (lines 24-27); `fn resolve_inner(config: &Config) -> Option<Inner>` (lines
  38-59) with a `match config.audit_destination()` whose arms are `"stderr"`
  and a `_ =>` fallback meaning file; `Recorder::reload(&self, config: &Config)`
  (lines 105-107) re-runs resolve_inner; `fn write_serialized(&self, record:
  &impl serde::Serialize, kind: &str)` (lines 115-139) matches on Inner to
  dispatch writes, warn-and-swallow on failure.
- src/governance/audit/destinations.rs: `default_audit_path()`,
  `append_line_to_file(path, line)`, `write_line_to_stderr(line)`.
- src/governance/config/mod.rs: `pub const AUDIT_DESTINATION: &str =
  "audit.destination";` (line 363) with a doc comment saying syslog/http/none
  are deferred; the KEYS entry at lines 413-420 with
  `KeyConstraint::EnumVariants(&["file", "stderr"])` and Enum("file") defaults;
  `AUDIT_FILE_PATH` ("audit.file.path", Str, default "") as the Str-key
  pattern; Config fields at 558-560, populated in from_preset (572-574) and
  from_resolution (600-602); accessors audit_enabled() 628,
  audit_destination() 633, audit_file_path() 638.
- Unit test `enum_key_parse_value` (mod.rs 901-927) asserts `json!("syslog")`
  is an Err and pins the message `expected one of: file, stderr`.
- Goldens: tests/golden/config-schema.json (enum block lines 51-54) and
  tests/golden/config-keys.md (line 54 `- Constraints: one of: file, stderr`),
  pinned byte-exact by tests/config_schema_golden.rs; regeneration procedure in
  00-design.md.
- Reload test template: `reload_reopens_the_sink_on_a_config_change`
  (audit/mod.rs line 251) builds LayerInputs with user-layer AUDIT_* entries,
  resolves, `Config::from_resolution`, then `recorder.reload(&config)`.
- tokio's `net` feature is already on; `std::net::UdpSocket` needs no Cargo
  change (the recorder is synchronous; use std, not tokio, per 00-design.md).

## Required behavior

### 1. Config registry (src/governance/config/mod.rs)

- AUDIT_DESTINATION KEYS entry: constraint becomes
  `KeyConstraint::EnumVariants(&["file", "stderr", "syslog", "none"])`
  (order pinned); defaults stay Enum("file"). Update the AUDIT_DESTINATION
  const doc comment to: `` `audit.destination` -- where audit records are
  written (`file`, `stderr`, `syslog` as RFC 5424 over UDP, or `none`;
  `http` is deferred, ADR-0026 Decision 4). ``
- New key, declared in the same style directly after AUDIT_FILE_PATH:
  `pub const AUDIT_SYSLOG_ADDRESS: &str = "audit.syslog.address";` with doc
  comment `` `audit.syslog.address` -- UDP target for the syslog audit
  destination, as host:port. ``
- Its KEYS entry, transcribed EXACTLY (a plain Str key uses
  `KeyConstraint::None`, the "base-type check only" variant, and
  `KeyValue::Str(...)` defaults -- do NOT reuse AUDIT_FILE_PATH's
  `EmptyOrAbsolutePath`, which would reject `127.0.0.1:514`):

      KeyDef {
          key: AUDIT_SYSLOG_ADDRESS,
          description: "UDP target for the syslog audit destination, as host:port.",
          constraint: KeyConstraint::None,
          default_fully_open: KeyValue::Str("127.0.0.1:514"),
          default_safe: KeyValue::Str("127.0.0.1:514"),
          default_restricted: KeyValue::Str("127.0.0.1:514"),
      },
- Config: add field + from_preset/from_resolution population + accessor
  `pub fn audit_syslog_address(&self) -> &str`, exactly mirroring the
  audit_file_path() pattern at the cited lines.
- Update `enum_key_parse_value` (the test in mod.rs cfg(test)). It builds a
  `KeyDef` (or reuses the AUDIT_DESTINATION def) whose internal
  `constraint`/`variants` list is `&["file", "stderr"]`; change that literal to
  `&["file", "stderr", "syslog", "none"]` so the four-variant set is what the
  test validates against. Then: `json!("syslog")` and `json!("none")` now parse
  Ok; use `json!("smoke-signals")` as the invalid probe; and the pinned error
  message assertion becomes `expected one of: file, stderr, syslog, none`.

### 2. Destinations (src/governance/audit/destinations.rs)

Add `pub fn send_line_to_syslog(addr: std::net::SocketAddr, line: &str) ->
std::io::Result<()>`: format the datagram per 00-design.md
(`<134>1 {ts} - ghostlight {pid} - - {line}`), bind `0.0.0.0:0`, `send_to`,
one socket per call. Timestamp via chrono as pinned in 00-design.md.

### 3. Recorder (src/governance/audit/mod.rs)

- `enum Inner` gains `Syslog(std::net::SocketAddr)`. No variant for none.
- `resolve_inner` arms become: `"stderr"` (unchanged), `"none"` returns None,
  `"syslog"` resolves `config.audit_syslog_address()` via
  `std::net::ToSocketAddrs` (first result; on failure `tracing::warn!` with
  the address and return None), and the existing `_ =>` file fallback stays
  LAST and unchanged.
- `write_serialized` gains the `Inner::Syslog(addr)` arm calling
  `destinations::send_line_to_syslog`, warn-and-swallow exactly like the
  File arm (same fields: error, kind; plus the addr).

### 4. Goldens and docs

- Regenerate both goldens via the sanctioned 00-design.md commands (Git Bash),
  hand-review the diff (expected: the enum gains two variants; one new key
  section/block for audit.syslog.address).
- README.md single pinned edit: the line
  `  still to do, along with `syslog`/`http` audit destinations.`
  becomes
  `  still to do, along with the `http` audit destination.`

### 5. New tests, by name, in src/governance/audit/mod.rs cfg(test)

Follow the LayerInputs pattern of reload_reopens_the_sink_on_a_config_change:

- `syslog_destination_sends_one_rfc5424_datagram_per_record`: bind a
  std::net::UdpSocket on 127.0.0.1:0 with a 2s read timeout; configure
  destination "syslog" + that socket's local_addr as audit.syslog.address;
  record one tool-call record; recv one datagram; assert the payload string
  starts_with `<134>1 `, contains ` ghostlight `, and contains
  `"event_id"`.
- `none_destination_discards_records_and_reports_disabled`: destination
  "none" (audit.enabled true) yields `is_enabled() == false` and recording
  panics nothing and creates no file.
- `invalid_syslog_address_disables_audit_with_a_warning`: destination
  "syslog", address `"not an address"` yields `is_enabled() == false`.
- `reload_switches_file_to_syslog`: build a file recorder, reload with a
  syslog config, assert a subsequent record arrives as a datagram (reuse the
  first test's socket recipe).

## Constraints

Only these files change: src/governance/config/mod.rs,
src/governance/audit/mod.rs, src/governance/audit/destinations.rs,
tests/golden/config-schema.json, tests/golden/config-keys.md, README.md (one
line). No new crate. No async in the recorder. ASCII only.

## Tests

The four named tests above pass; `cargo test` fully green including
config_schema_golden (goldens regenerated) and the updated
enum_key_parse_value; `rg -c "syslog" tests/golden/config-keys.md` prints a
number >= 2.

## Verification

`cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`; `cargo test`
(record count delta: baseline + 4 new); ASCII diff scan; ledger entry; commit.

Commit subject: `feat(audit): syslog (RFC 5424/UDP) and none destinations (ADR-0026 D4)`

## Out of scope

http destination; any change to record shape or JSONL field order (pinned by
tests/audit_recorder.rs); hostname resolution caching; TCP or TLS syslog;
Recorder API surface beyond the listed additions; any other README edit.
