# L04: mcp-server startup wiring and the doctor License section

## Goal

Connect the pieces: resolve the license once at mcp-server startup, set the Recorder
stamp, warn when abnormal; add the read-only `License:` section to `ghostlight doctor`.

## Authority

ADR-0028 Decisions 1, 3, 4; 00-design.md "Startup wiring (l04)" and "Doctor section (l04)".

## Depends on

l01-l03. STOP preconditions:
`rg -n "set_license_stamp" src/governance/audit/mod.rs` matches;
`rg -n "Recorder::from_config" src/transport/mcp/server.rs` matches exactly once;
`rg -n "License:" src/doctor.rs` prints nothing. If any fails, STOP.

## Current behavior (verified 2026-07-03 pre-batch; locate by content)

- src/transport/mcp/server.rs builds the recorder inside the server startup function:
  `let recorder = Arc::new(Recorder::from_config(&store.current()));` followed by a
  `tokio::spawn` block subscribing to config changes and calling `recorder.reload(...)`.
- src/doctor.rs `pub fn run` prints sections in this order: Binary, Browsers,
  MCP clients, Policy manifest, Governance, IPC endpoint, sessions, Verdict. Section
  rows use `println!("  {:<9}{}", "label", value)`. The Governance section is rendered
  via `governance_section_lines()` and the IPC section begins with
  `println!("IPC endpoint:");`.
- Doctor findings (`fn findings`) do not mention licensing anywhere.

## Required behavior

### 1. Startup wiring (src/transport/mcp/server.rs; only these lines)

Directly after the `let recorder = Arc::new(Recorder::from_config(&store.current()));`
statement (BEFORE the reload-subscription `tokio::spawn`), insert:

    let (license_state, license_path) = crate::governance::license::resolve_from_disk();
    let org_present = crate::governance::config::load::org_policy_path().exists();
    let license_stamp = crate::governance::license::stamp_for(&license_state, org_present);
    recorder.set_license_stamp(license_stamp);
    if let Some(stamp) = license_stamp {
        tracing::warn!(
            stamp,
            path = ?license_path,
            "license state is abnormal; audit records will carry a license stamp"
        );
    }

(Adjust ONLY the path-qualification prefixes if the file's existing imports make a
shorter form idiomatic; the statements and their order are pinned.)

### 2. Doctor section (src/doctor.rs)

Between the Governance section loop and the `let endpoint = ipc::default_endpoint();`
line, insert a License section following the file's existing style:

    println!();
    println!("License:");
    for line in license_section_lines() {
        println!("{line}");
    }

Implement `fn license_section_lines() -> Vec<String>` in doctor.rs: call
`resolve_from_disk` and `org_policy_path().exists()`, render the `state` row via the
SHARED `crate::governance::license::state_row` helper l03 created (never a local copy),
and the `file` row (resolved path or `-`), using the `format!("  {:<9}{}", ...)` row
style. The section NEVER adds a finding: do not touch `fn findings` or `Observations`.
The state vocabulary is already covered by l03's `state_row_vocabulary_is_stable` unit
test; this task adds no new test.

## Constraints

Only src/transport/mcp/server.rs (the pinned statements) and src/doctor.rs change.
Doctor stays read-only and its exit code/findings are untouched. No behavioral gating.
ASCII only.

## Verification

`cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
`cargo clippy --all-targets --features license-admin -- -D warnings`; `cargo test`
(delta: previous + 0 new; the full suite stays green); MANUAL: run
`CARGO_TARGET_DIR=target/it cargo run -- doctor`
and record the printed License section in the ledger entry (expected on this machine:
state `none (personal use: no license required)` unless an org policy file exists);
ASCII diff scan; ledger entry; commit.

Commit subject: `feat(license): startup stamp wiring and doctor License section (ADR-0028 D4)`

## Out of scope

Any doctor finding/verdict change; explain; MCP responses; license hot-reload;
tests/audit_recorder.rs.
