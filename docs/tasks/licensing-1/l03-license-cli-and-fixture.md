# L03: license CLI, admin signing, and the committed dev fixture

## Goal

The `ghostlight license` command family (status, install; feature-gated sign and pubkey)
plus the committed development-license fixture and its integration tests.

## Authority

ADR-0028 Decisions 2 and 4; 00-design.md "CLI (l03)" and "Dev license fixture (l03)".

## Depends on

l01 and l02. STOP preconditions:
`rg -n "pub fn resolve_from_disk" src/governance/license.rs` matches;
`rg -n "enum Command" src/main.rs` matches;
`rg -n "LicenseArgs" src/main.rs` prints nothing (no variant yet; note the bare word
"License" DOES already appear in main.rs line 1's SPDX header, which is why the probe
targets the args-struct name);
tests/fixtures/license/ does not exist (tests/fixtures/ itself exists, with explain/
and simulate/ subdirectories). If any fails, STOP.

## Current behavior (verified 2026-07-03 pre-batch; locate by content)

- src/main.rs: `#[derive(Debug, Subcommand)] enum Command` with variants Install,
  Uninstall, Doctor, Status, Config, Policy, each carrying an Args struct; the pattern
  for a sub-subcommand is `PolicyArgs { #[command(subcommand)] command: PolicyCommand }`.
- Integration tests that spawn the binary live in tests/ and use
  `Command::new(env!("CARGO_BIN_EXE_ghostlight"))` (see tests/policy_explain.rs and the
  `fn drive` helper in tests/mcp_protocol.rs for the conventions: stdio pipes, assert on
  exit status and output).
- tests/fixtures/ may or may not have other subdirectories; tests/fixtures/license/ does
  not exist.

## Required behavior

### 1. The shared state-row helper (append to src/governance/license.rs)

    /// One-line human rendering of a license state, shared verbatim by
    /// `ghostlight license status` and the doctor License section (ADR-0028 D4).
    pub fn state_row(state: &LicenseState, org_present: bool) -> String;

Pinned outputs (exact strings, used by the l03 unit test and by l04):

    (NoLicense, false)      -> none (personal use: no license required)
    (NoLicense, true)       -> unlicensed (org policy present, no license file)
    (Invalid(r), _)         -> invalid: {r}
    (Expired(c), _)         -> expired {expires} ({tier}, {licensee})
    (Valid(c) dev tier, _)  -> development (self-signed evaluation license)
    (Valid(c) other, _)     -> valid ({tier}, {licensee}, expires {expires})

### 2. CLI (src/main.rs; sole owner in this batch)

Add the `License(LicenseArgs)` variant and subcommands exactly per 00-design.md
"CLI (l03)": `status [--file PATH]`, `install <FILE> [--org]`, and behind
`#[cfg(feature = "license-admin")]` the `sign` and `pubkey` subcommands. Keep the
handlers thin in main.rs (parse, call into `ghostlight::governance::license`, print);
follow the file's existing From-impl conversion style only if a natural fit, otherwise
match arms directly in main().

`status` prints `state: ` followed by `state_row(...)` (org_present computed as
`org_policy_path().exists()`; when `--file` is given org_present is passed as false),
then when a license is present the `{:<10}{}` claim rows per 00-design.md. `status`
exits 0 always. `install` exits nonzero for an `Invalid` license with the reason on
stderr; an `Expired` license installs with a warning line on stderr.

### 3. Dev fixture (tests/fixtures/license/dev-license.json)

Create the claims file EXACTLY as pinned in 00-design.md "Dev license fixture (l03)"
(write the one-line claims JSON to a temp file first), then generate the committed
fixture with the admin CLI (Git Bash, isolated target dir if ground rule 4 applies):

    CARGO_TARGET_DIR=target/it cargo run --features license-admin -- license sign \
      --key <(printf 'ghostlight development key gen0!') --keygen 0 \
      --claims /tmp/dev-claims.json --out tests/fixtures/license/dev-license.json

If process substitution (`<(...)`) fails on this platform, write the 32 seed bytes to a
temp file with `printf 'ghostlight development key gen0!' > /tmp/dev-seed.bin` (NO
trailing newline; verify with `wc -c` = 32) and pass that path. Verify the fixture then
resolves as a development license:

    CARGO_TARGET_DIR=target/it cargo run -- license status --file tests/fixtures/license/dev-license.json

must print a line containing `development (self-signed evaluation license)`.

### 4. Unit test (license.rs cfg(test))

- `state_row_vocabulary_is_stable`: assert all six pinned rows of section 1 exactly,
  building Expired/Valid claims with tier `team`, licensee `Test Org`, and expires
  `2020-01-01` / `2126-01-01` respectively, plus a Valid development-tier claims.

### 5. Integration tests (tests/license_cli.rs, new; engine SPDX header)

By name, spawning the binary like tests/policy_explain.rs does:

- `status_with_explicit_missing_file_reports_invalid_and_exits_zero`: run
  `license status --file <nonexistent path>`; exit code 0; stdout contains `invalid` or
  `unreadable` (the unreadable-file reason from l02).
- `status_reads_the_committed_dev_fixture`: run
  `license status --file tests/fixtures/license/dev-license.json` (absolute path via
  `env!("CARGO_MANIFEST_DIR")`); exit 0; stdout contains
  `development (self-signed evaluation license)`, `Ghostlight Development`, and
  `2126-01-01`.
- `install_rejects_a_garbage_file_nonzero`: write garbage bytes to a temp file; run
  `license install <temp>`; nonzero exit; stderr contains `not valid json`.
- `binary_without_admin_feature_has_no_sign_subcommand`: run `license sign --help`;
  nonzero exit (clap unknown-subcommand error). (CARGO_BIN_EXE builds WITHOUT the
  feature, which is exactly what this asserts.)

## Constraints

Only src/main.rs, src/governance/license.rs (the state_row helper and its test),
tests/fixtures/license/dev-license.json, and tests/license_cli.rs change. The fixture is
generated by the sanctioned command, never hand-typed. No gating: `install` writing a
file and `status` printing are the only side effects. ASCII only (the fixture's base64
payload is ASCII by construction).

## Verification

`cargo fmt --check`; `cargo clippy --all-targets -- -D warnings`;
`cargo clippy --all-targets --features license-admin -- -D warnings`; `cargo test`
(delta: previous + 5 new); the two literal commands in section 3; ASCII diff scan;
ledger entry; commit.

Commit subject: `feat(license): ghostlight license CLI, admin signing, dev fixture (ADR-0028)`

## Out of scope

server.rs, doctor.rs (l04); any org-path write test that would touch the real
%ProgramData% (never write there in tests; `install --org` is covered by code review
plus the user-path tests).
