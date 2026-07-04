# L01: license core (envelope, claims, verify, sign, stamp table)

## Goal

ADR-0028 Decisions 1-3: the pure license module -- parse and verify the envelope format,
resolve a LicenseState, compute the stamp. No I/O, no wiring, no CLI in this task.

## Authority

ADR-0028 Decisions 1-3; 00-design.md "Dependencies" and "Module: src/governance/license.rs".

## Depends on

Nothing. STOP preconditions: `rg -n "\[features\]" Cargo.toml` prints nothing (no
features table exists); `rg -l "ed25519" Cargo.toml src/` prints nothing;
src/governance/license.rs does not exist; `rg -n "pub mod ports;" src/governance/mod.rs`
matches (anchor for the module list). If any fails, STOP.

## Current behavior (verified 2026-07-03; re-read before editing)

- Cargo.toml `[dependencies]` contains tokio, serde, serde_json (with `preserve_order`),
  clap, tracing, tracing-subscriber, thiserror, anyhow, dirs, sha2, uuid, chrono, url;
  there is no `[features]` table and no ed25519/base64 dependency.
- src/governance/mod.rs lists modules alphabetically: audit, config, denial, dispatch,
  enforcement, explain, manifest, ports, simulate, templates.
- No file named license.rs exists anywhere under src/.

## Required behavior

### 1. Cargo.toml (sole owner in this batch)

Append to `[dependencies]` (keep the existing entries untouched):

    ed25519-dalek = "2"
    base64 = "0.22"

Add at the end of the file:

    [features]
    # Gates the license-authoring CLI subcommands (sign, pubkey). Never enabled in
    # release builds; needs no extra dependencies.
    license-admin = []

### 2. src/governance/mod.rs

Insert `pub mod license;` into the module list in alphabetical position (between
`pub mod explain;` and `pub mod manifest;`).

### 3. src/governance/license.rs (new)

Line 1: `// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial`
Then a module doc comment (`//!`) explaining: offline Ed25519 license verification per
ADR-0028; purely observational (Decision 1: license state never affects behavior); never
any network I/O.

Implement EXACTLY the API pinned in 00-design.md "Module: src/governance/license.rs":
`DEV_SEED`, `verifying_key`, `Claims`, `LicenseState`, `resolve_bytes`, `sign`,
`build_envelope`, `stamp_for`, with the validation rules 1-9 and the stamp truth table
pinned there. Implementation notes (binding):

- `verifying_key(0)` derives from `ed25519_dalek::SigningKey::from_bytes(DEV_SEED)`
  (`.verifying_key()`); any other generation returns None (production keys are added
  later by the founder as hex constants; leave a `// keygen 1+: production keys land
  here (ADR-0028 Decision 2)` comment in the lookup).
- base64: `base64::engine::general_purpose::STANDARD` with `base64::Engine` in scope.
- Verification uses `VerifyingKey::verify_strict`.
- `Invalid` reasons are short lowercase phrases; pin these exact strings, used by the
  tests below: `not valid json`, `unsupported envelope version`, `malformed envelope`,
  `bad base64`, `signature must be 64 bytes`, `unknown key generation`,
  `signature verification failed`, `malformed claims`, `unknown tier`,
  `license does not cover this product`, `malformed expiry date`.
- Today's date for rule 9: `chrono::Utc::now().format("%Y-%m-%d").to_string()`.
- `sign` and `build_envelope` are plain functions (NOT feature-gated); only CLI
  subcommands are gated (l03). `build_envelope` output is
  `serde_json::json!({"v":1,"keygen":keygen,"claims":<b64>,"sig":<b64>})` serialized
  with `serde_json::to_string_pretty` plus one trailing `\n`.

### 4. Unit tests, in `#[cfg(test)] mod tests` of license.rs, by name

Use this helper claims JSON (one line; escape it as a Rust raw string) as the base for
signing tests:

    {"id":"00000000-0000-4000-8000-000000000001","licensee":"Test Org","org":"test","tier":"team","seats":10,"products":["browser"],"issued":"2026-07-03","expires":"2126-01-01"}

- `dev_seed_is_exactly_32_bytes_and_derives_a_key`: `DEV_SEED.len() == 32` and
  `verifying_key(0).is_some()`.
- `unknown_generation_has_no_key`: `verifying_key(1).is_none()` and
  `verifying_key(u32::MAX).is_none()`.
- `envelope_round_trips_and_verifies`: build_envelope(DEV_SEED, 0, base claims bytes),
  resolve_bytes on its bytes, assert `LicenseState::Valid(c)` with `c.tier == "team"`,
  `c.seats == 10`, `c.org == "test"`.
- `tampered_claims_fail_verification`: take the round-trip envelope JSON, decode the
  claims field, replace `"seats":10` with `"seats":9999`, re-encode WITHOUT re-signing,
  resolve, assert `Invalid` with reason exactly `signature verification failed`.
- `expired_license_resolves_expired`: sign claims with `"expires":"2020-01-01"`, assert
  `LicenseState::Expired(_)`.
- `expiry_boundary_is_inclusive_today`: sign claims whose expires is EXACTLY today
  (compute today with the same chrono format call), assert `Valid(_)` (rule 9 uses
  strictly-less-than).
- `wrong_product_is_invalid`: claims with `"products":["desktop"]`, assert `Invalid`
  reason `license does not cover this product`.
- `unknown_tier_is_invalid`: `"tier":"platinum"`, reason `unknown tier`.
- `garbage_bytes_are_invalid_not_panic`: resolve_bytes(b"not json at all") is `Invalid`
  with reason `not valid json`; resolve_bytes(&[0u8, 159, 146, 150]) does not panic and
  is `Invalid`.
- `wrong_version_is_invalid`: a well-formed envelope with `"v":2`, reason
  `unsupported envelope version`.
- `stamp_table_matches_adr_0028`: assert all six rows of the 00-design.md truth table,
  including `stamp_for(&Valid(dev claims), false) == Some("development")` and
  `stamp_for(&NoLicense, false) == None`.

## Constraints

Pure module: no filesystem, no network, no CLI. No behavioral gating (ADR-0028 Decision
1). ASCII only; the module compiles without the license-admin feature (the feature gates
nothing in this task).

## Verification

`cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
`cargo clippy --all-targets --features license-admin -- -D warnings`; `cargo test`
(record delta: baseline + 11 new); ASCII diff scan; ledger entry; commit.

Commit subject: `feat(license): offline ed25519 license core with dev generation 0 (ADR-0028)`

## Out of scope

Disk paths, Recorder, CLI, server wiring, doctor (l02-l04); any golden; any change to
existing dependencies.
