# Licensing-1: shared design and pinned semantics

Normative companion to the l01-l06 task prompts in this directory. Prompts cite this file
and ADR-0028 instead of restating semantics; if a prompt and this file disagree, STOP and
record a BLOCKED entry. ADR-0028 is the decision authority; this file adds the
executor-facing pins (exact types, strings, formats, and file locations).

## Provenance (decided; do not re-litigate)

- ADR-0028 is accepted. Licensing is purely observational (Decision 1): NOTHING is ever
  enabled/disabled by license state. If any task appears to require behavioral gating,
  that is a misreading; STOP.
- The envelope and claims formats, the stamp decision table, the tier enum, the dev
  generation-0 seed, and the four surfaces are pinned in ADR-0028 Decisions 2-4.
- The license is NOT surfaced in the `explain` tool or its CLI goldens
  (tests/golden under policy explain), and NOT in any MCP tool response.
- License-file hot-reload is out of scope for v1 (resolution happens once at mcp-server
  startup).

## Pinned semantics

### Dependencies (l01 owns the Cargo.toml edit)

Add to `[dependencies]`:

    ed25519-dalek = "2"
    base64 = "0.22"

Add a `[features]` table (none exists today):

    [features]
    # Gates the license-authoring CLI subcommands (sign, pubkey). Never enabled in
    # release builds; needs no extra dependencies.
    license-admin = []

No other dependency changes. `uuid` and `chrono` are already present.

### Module: src/governance/license.rs (new; governance SPDX header)

Public API, exactly:

    pub const DEV_SEED: &[u8; 32] = b"ghostlight development key gen0!";

    /// Verifying keys by generation index. Generation 0 is the public development key
    /// (derived from DEV_SEED); production generations are appended by the founder.
    pub fn verifying_key(keygen: u32) -> Option<ed25519_dalek::VerifyingKey>;

    #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    pub struct Claims {
        pub id: String,
        pub licensee: String,
        pub org: String,
        pub tier: String,
        pub seats: u32,
        pub products: Vec<String>,
        pub issued: String,
        pub expires: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LicenseState {
        NoLicense,
        Valid(Claims),
        Expired(Claims),
        Invalid(String),
    }

    /// Parse and verify one envelope file's bytes. Never panics on any input.
    pub fn resolve_bytes(bytes: &[u8]) -> LicenseState;

    /// Sign claims bytes with a 32-byte seed; returns the 64-byte signature. Library
    /// function (not feature-gated); the CLI subcommands that call it are gated.
    pub fn sign(seed: &[u8; 32], claims_bytes: &[u8]) -> [u8; 64];

    /// Build a complete envelope JSON string (v, keygen, claims b64, sig b64) + trailing LF.
    pub fn build_envelope(seed: &[u8; 32], keygen: u32, claims_bytes: &[u8]) -> String;

    /// The stamp for a resolved state per the ADR-0028 Decision 3 table, or None.
    pub fn stamp_for(state: &LicenseState, org_present: bool) -> Option<&'static str>;

Validation rules for `resolve_bytes` (any failure -> `Invalid(reason)` with a short
human-readable reason; never a panic):

1. Bytes parse as JSON object with integer `v == 1`, integer `keygen` (u32), string
   `claims`, string `sig`.
2. `claims` and `sig` are valid standard base64 (`base64::engine::general_purpose::STANDARD`);
   `sig` decodes to exactly 64 bytes.
3. `verifying_key(keygen)` exists (else reason `unknown key generation`).
4. `verify_strict` over the exact decoded claims bytes succeeds.
5. Decoded claims bytes parse as `Claims` (serde).
6. `tier` is one of `development`, `community`, `founding`, `team`, `enterprise`.
7. `products` contains `"browser"`.
8. `expires` parses as `chrono::NaiveDate` `%Y-%m-%d`.
9. If `expires` (string, lexicographic) is strictly less than today's UTC date rendered
   `%Y-%m-%d`: `Expired(claims)`. Else `Valid(claims)`.

`stamp_for` truth table (pin, from ADR-0028 Decision 3):

    (NoLicense, false) -> None
    (NoLicense, true)  -> Some("unlicensed")
    (Invalid(_), _)    -> Some("invalid")
    (Expired(_), _)    -> Some("expired")
    (Valid(c), _) where c.tier == "development" -> Some("development")
    (Valid(_), _)      -> None

### Disk resolution (l02)

In license.rs:

    /// Org license path: license.json in the directory of load::org_policy_path().
    pub fn org_license_path() -> std::path::PathBuf;
    /// User license path: dirs::config_dir()/ghostlight/license.json; None when the
    /// platform config dir is unavailable.
    pub fn user_license_path() -> Option<std::path::PathBuf>;
    /// First existing file of [org, user], resolved via resolve_bytes; NoLicense when
    /// neither exists; Invalid("unreadable license file: ...") on a read error.
    pub fn resolve_from_disk() -> (LicenseState, Option<std::path::PathBuf>);

`org_license_path` is the org policy file's directory joined with `license.json`; derive
it from `crate::governance::config::load::org_policy_path().parent()` (fall back to the
path itself if `parent()` is None, which cannot happen for the pinned paths).

### Recorder stamp (l02)

`src/governance/audit/mod.rs` `Recorder` gains:

    license_stamp: Mutex<Option<&'static str>>,

initialized `None` in every constructor, plus:

    /// Set (or clear) the license stamp appended to every subsequent record
    /// (ADR-0028 Decision 3). Called once at mcp-server startup.
    pub fn set_license_stamp(&self, stamp: Option<&'static str>);

`write_serialized` changes from `serde_json::to_string(record)` to: serialize with
`serde_json::to_value`, and if a stamp is set, insert key `"license"` with the stamp
string into the top-level object (serde_json is built with `preserve_order`, so the
inserted key lands LAST, after `held` on tool-call records), then `to_string` the value.
When no stamp is set the output must be byte-identical to today's.

### Startup wiring (l04, src/transport/mcp/server.rs)

Immediately after the existing line

    let recorder = Arc::new(Recorder::from_config(&store.current()));

add license resolution:

    let (license_state, license_path) = ghostlight::governance::license::resolve_from_disk();

(adjust the path prefix to the crate-internal `crate::governance::license::` form used in
that file), compute `org_present = crate::governance::config::load::org_policy_path().exists()`,
call `recorder.set_license_stamp(license::stamp_for(&license_state, org_present))`, and
when the stamp is `Some(s)` emit exactly one
`tracing::warn!(stamp = s, path = ?license_path, "license state is abnormal; audit records will carry a license stamp")`.

### Doctor section (l04, src/doctor.rs)

A new `License:` section printed between the existing `Governance:` section and the
`IPC endpoint:` section, using the file's existing `println!("  {:<9}{}", ...)` row style:

- row `state`: one of `none (personal use: no license required)` /
  `unlicensed (org policy present, no license file)` / `invalid: <reason>` /
  `expired <expires> (<tier>, <licensee>)` / `valid (<tier>, <licensee>, expires <expires>)` /
  `development (self-signed evaluation license)`
- row `file`: the resolved path, or `-` when none.

The License section NEVER contributes to doctor findings/problems (Decision 1: purely
observational; doctor's verdict is about the engine chain, not licensing).

### CLI (l03, src/main.rs)

`Command` gains one variant (doc comment pinned):

    /// Show or install a Ghostlight license (see ADR-0028; license state never affects behavior).
    License(LicenseArgs),

with subcommands:

- `status [--file PATH]`: resolve from `--file` if given, else `resolve_from_disk`;
  print `state: ` followed by the shared `state_row` helper's output (helper pinned in
  the l03 prompt; doctor reuses it in l04), then when a license is present print `tier`,
  `licensee`, `org`, `seats`, `issued`, `expires`, `keygen`, `file` one per line as
  `{:<10}{}` rows; exit 0 always (status is informational).
- `install <FILE> [--org]`: validate the file with `resolve_bytes` (reject `Invalid`,
  exit nonzero, print the reason to stderr; `Expired` installs with a warning); copy it to
  `user_license_path()` (default) or `org_license_path()` (`--org`), creating parent
  directories; print the destination path.
- Feature-gated (`#[cfg(feature = "license-admin")]`): `sign --key <SEED_FILE> --keygen
  <N> --claims <CLAIMS_JSON_FILE> --out <OUT_FILE>` (reads a raw 32-byte seed file, builds
  the envelope via `build_envelope`, writes it) and `pubkey --key <SEED_FILE>` (prints the
  lowercase-hex verifying key for embedding).

### Dev license fixture (l03)

`tests/fixtures/license/dev-license.json`: an envelope signed with `DEV_SEED`, keygen 0,
over exactly these claims bytes (one line, no trailing newline inside the encoded bytes):

    {"id":"00000000-0000-4000-8000-000000000001","licensee":"Ghostlight Development","org":"dev","tier":"development","seats":1,"products":["browser"],"issued":"2026-07-03","expires":"2126-01-01"}

Generated by running the admin `sign` subcommand once (the prompt gives the literal
command); committed. Integration tests read it.

### SECURITY.md and SBOM (l05)

Full SECURITY.md text is pinned in the l05 prompt. SBOM: one step appended to BOTH matrix
legs of `.github/workflows/release.yml`'s job (install `cargo-cyclonedx`, generate, upload
with the existing artifact step by adding the SBOM file to the `path:` list); exact YAML
pinned in l05.

### Business templates (l06)

Three markdown templates plus one YAML, full text pinned in the l06 prompt, created under
`docs/business/templates/`: `renewal-t30.md`, `renewal-t0.md`,
`founding-org-agreement.md`, `expiry-reminder-workflow.yml` (for the PRIVATE
ghostlight-licensing repo; stored here as a template only).

## Global rules for all tasks

- ASCII only in code; docs use no em-dashes and no smart quotes (double hyphen with
  spaces for the dash).
- New .rs files carry the SPDX header for their side of the boundary
  (license.rs is `// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial`).
- The sacred tool surface (src/transport/mcp/schemas/tools.json,
  tests/tool_schema_fidelity.rs) is never touched.
- `src/governance/explain.rs`, the policy_explain goldens, and `tests/audit_recorder.rs`
  are never touched by this batch (the stamp design makes the latter's assertions hold
  unchanged; if any of its tests fail, that is a STOP, not an edit).
- No behavioral gating anywhere (ADR-0028 Decision 1). No network I/O in the license path.
