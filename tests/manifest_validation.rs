//! Integration tests for the G12 manifest engine's public API: the three example manifests
//! under `examples/` parse and validate, and the all-open invariant holds. The exhaustive
//! invalid-field matrix, the hash pins, and the source-grammar/selection tests live as inline
//! unit tests in `governance::manifest::document`/`governance::manifest::source` (pure
//! functions, no real files or environment touched); this file exercises the public API against
//! real example files on disk, the one thing inline unit tests cannot do without reaching
//! outside the crate.

use browser_mcp::browser::pattern;
use browser_mcp::governance::manifest::document::parse_manifest;
use browser_mcp::transport::mcp::tools;

fn read_example(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn assert_valid_hash(hash: &str) {
    assert_eq!(hash.len(), 64, "hash: {hash}");
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "hash: {hash}"
    );
}

#[test]
fn enterprise_healthcare_example_parses() {
    let text = read_example("enterprise-healthcare.json");
    let m = parse_manifest(
        &text,
        "enterprise-healthcare.json",
        pattern::is_valid_pattern,
        tools::is_known_tool,
    )
    .expect("enterprise-healthcare.json should parse and validate");
    assert_eq!(m.schema, 2);
    assert_eq!(m.name, "enterprise-healthcare");
    assert_eq!(m.grants.len(), 4);
    assert_valid_hash(&m.hash);
}

#[test]
fn developer_observe_example_parses() {
    let text = read_example("developer-observe.json");
    let m = parse_manifest(
        &text,
        "developer-observe.json",
        pattern::is_valid_pattern,
        tools::is_known_tool,
    )
    .expect("developer-observe.json should parse and validate");
    assert_eq!(m.schema, 2);
    assert_eq!(m.name, "developer-observe");
    assert_eq!(m.grants.len(), 0);
    assert_valid_hash(&m.hash);
}

/// `qa-staging.json`'s own scenario is a Linux CI runner, so its `audit.file.path` is
/// authored as a Unix-shaped absolute path (`/var/log/browser-mcp/qa-audit.jsonl`), exactly as
/// the g12 task doc specifies byte-for-byte. `std::path::Path::is_absolute()` requires a drive
/// letter to consider a path absolute on Windows, so the pre-existing `EmptyOrAbsolutePath`
/// registry constraint (G01, unrelated to and unchanged by this task) correctly rejects this
/// one value on a Windows dev machine. Assert the platform-appropriate outcome precisely
/// rather than silently skipping the file either way.
#[test]
fn qa_staging_example_parses() {
    let text = read_example("qa-staging.json");
    let result = parse_manifest(
        &text,
        "qa-staging.json",
        pattern::is_valid_pattern,
        tools::is_known_tool,
    );

    #[cfg(windows)]
    {
        let err = result.unwrap_err();
        let message = err.to_string();
        assert!(message.contains("config[2]"), "{message}");
        assert!(message.contains("absolute path"), "{message}");
    }
    #[cfg(not(windows))]
    {
        let m = result.expect("qa-staging.json should parse and validate");
        assert_eq!(m.schema, 2);
        assert_eq!(m.name, "qa-staging");
        assert_eq!(m.grants.len(), 2);
        assert_valid_hash(&m.hash);
    }
}

/// All-open invariant (g12 constraint 3): loading with no org file and no user source yields
/// `LoadedPolicy { manifest: None, origin: None, user_manifest_ignored: false }`.
/// `tests/mcp_protocol.rs` (unchanged by this task) already proves the binary's byte-identical
/// wire behavior end to end; this test proves the loader's own return value directly, through
/// the exact public entry point `server::run` uses. Confirms no real org policy file exists on
/// this machine first (as G02/G09's own manual-verification passes did) so the strict
/// assertion is never a false failure caused by unrelated local machine state.
#[test]
fn no_manifest_sources_yields_all_open() {
    let org_path = browser_mcp::governance::config::load::org_policy_path();
    if org_path.exists() {
        eprintln!(
            "skipping the strict all-open assertion: a real org policy file exists at {}",
            org_path.display()
        );
        return;
    }

    let loaded = browser_mcp::governance::manifest::source::load_policy(
        None,
        pattern::is_valid_pattern,
        tools::is_known_tool,
    )
    .expect("no sources present: loading must not fail");
    assert_eq!(loaded.manifest, None);
    assert_eq!(loaded.origin, None);
    assert!(!loaded.user_manifest_ignored);
}
