//! The schema-2 manifest document: format types, parsing, and validation (ADR-0018 step 3;
//! shared format doc section 4). Domain-agnostic core: this module knows the manifest's SHAPE
//! (grants, config entries, identity, mode) but resolves no domain pattern grammar and looks up
//! no tool name itself -- both are injected as function pointers supplied by the composition
//! root (`browser::pattern::is_valid_pattern` for domain syntax,
//! `transport::mcp::tools::is_known_tool` for the sacred tool surface), so this module never
//! names `browser::` or `transport::` directly (the a7 arch-test forbids it). Grant EVALUATION
//! (matching a resolved host against a grant's domains) is G13's job; this module validates
//! pattern SYNTAX only.
//!
//! Supersedes SPEC sections 4.1/4.2/Appendix A's older schema-1 format
//! (`access: "observe"|"mutate"`, `defaults`/`audit` blocks, `unlisted_domains`); a schema-1
//! manifest fails here with a precise unsupported-schema error, never silent compatibility
//! parsing.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::governance::ports::EffectiveMode;

/// The schema-2 manifest document (shared format doc section 4.1). `hash` is never authored;
/// it is computed by [`parse_manifest`] from the canonical bytes (section 4.2) and is the one
/// field excluded from both serialization and deserialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Must be exactly `2`; any other value is rejected before shape validation runs.
    pub schema: u32,
    /// Required, non-empty.
    pub name: String,
    /// Required, non-empty. A free-form label, not a semver requirement.
    pub version: String,
    /// The manifest-level default enforcement mode; `None` defers to the resolved
    /// `governance.mode` config key (G15's precedence: per-grant > manifest > registry).
    pub mode: Option<EffectiveMode>,
    /// Informational identity block (section 4.1); all fields optional and untyped strings,
    /// never validated against an enum -- the reconciled format keeps this informational.
    pub identity: Option<IdentityBlock>,
    /// Required (may be empty).
    pub grants: Vec<Grant>,
    /// Optional (defaults to empty).
    #[serde(default)]
    pub config: Vec<ConfigEntry>,
    /// SHA-256 content hash, 64 lowercase hex characters, computed by [`parse_manifest`] from
    /// the canonical bytes (section 4.2). Never authored; an authored `hash` key is rejected as
    /// an unknown field by `deny_unknown_fields`, since this field is `#[serde(skip)]`.
    #[serde(skip)]
    pub hash: String,
}

/// The manifest's informational `identity` block (shared format doc section 4.1). Every field
/// is optional and, when present, type-checked but not otherwise validated (`resolved_by` is a
/// free string, not an enum). Distinct from
/// [`crate::governance::ports::Identity`] (the audit record's derived `{principal,
/// resolved_by}` pair): this is the full authored block a later task derives that pair from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IdentityBlock {
    pub resolved_by: Option<String>,
    pub principal: Option<String>,
    pub groups: Option<Vec<String>>,
    pub resolved_at: Option<String>,
}

/// One resolved-at-load-time grant (shared format doc section 4.3). Consumed unchanged by
/// [`crate::governance::ports::DecisionRequest`] (g13): this IS the type a2 anticipated when it
/// called its own placeholder `Grant` "the manifest engine fleshes this out to
/// `{ domains, access, tools, mode }`" -- there is exactly one `Grant` type in the crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    /// Required, non-empty, unique within the manifest.
    pub id: String,
    /// Required, at least one section-5.1 pattern (exact host or a single leading `*.`
    /// wildcard). Syntax validated at load; matching semantics are G13's.
    pub domains: Vec<String>,
    pub access: Access,
    /// `None`/`null` means every tool. Mutually exclusive with `exclude_tools` (both non-null
    /// is a validation error).
    pub tools: Option<Vec<String>>,
    /// Mutually exclusive with `tools`.
    pub exclude_tools: Option<Vec<String>>,
    pub description: Option<String>,
    /// Per-grant override of the manifest-level `mode`.
    pub mode: Option<EffectiveMode>,
}

/// A grant's access level (shared format doc section 4.3). Distinct from
/// [`crate::governance::ports::RwClass`] (the observe/mutate classification axis): `access` is
/// what a grant PERMITS; `RwClass` is what a call IS (RECONCILIATION.md section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    Read,
    Write,
    All,
}

/// One manifest `config` entry (shared format doc section 4.4): a registry key, a value, and
/// the layer it targets when the manifest is the org policy file (an entry from a
/// user-supplied manifest always lands in the user layer regardless of its declared level;
/// see `governance::manifest::source`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigEntry {
    /// Must name a key registered in the typed key registry (`governance::config::KEYS`).
    pub key: String,
    /// Must satisfy the key's declared type and constraint.
    pub value: serde_json::Value,
    pub level: Level,
}

/// A config entry's declared layer (shared format doc section 2, 4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Mandatory,
    Recommended,
}

/// Why a manifest failed to parse or validate. Every variant's `Display` names the source
/// label and enough detail to fix the manifest without reading Rust code.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The text is not valid JSON at all.
    #[error("{source_label}: syntax error at line {line}, column {column}: {message}")]
    Syntax {
        source_label: String,
        line: usize,
        column: usize,
        message: String,
    },
    /// The `schema` field is missing, not an integer, or not `2`.
    #[error("{source_label}: unsupported schema version {found} (only schema 2 is supported)")]
    UnsupportedSchema { source_label: String, found: String },
    /// Valid JSON, wrong shape: an unknown field, a wrong type, or a missing required field.
    /// serde's own message already names the field and (when available) the position.
    #[error("{source_label}: {message}")]
    Shape {
        source_label: String,
        message: String,
    },
    /// Valid shape, invalid content. `path` is a dotted/indexed field path (e.g.
    /// `grants[1].domains[0]`); no line number is available at this validation stage.
    #[error("{source_label}: {path}: {reason}")]
    Field {
        source_label: String,
        path: String,
        reason: String,
    },
}

/// Parse and validate manifest JSON text (shared format doc section 4) and compute its
/// content hash (section 4.2). `source_label` names the origin (a file path or `env://VAR`)
/// for error messages. `domain_pattern_valid` and `is_known_tool` are the browser plugin's and
/// the MCP tool surface's real checkers, injected so this core module never names
/// `browser::`/`transport::` directly.
///
/// Pipeline, in this exact order so every failure class gets its most precise error: strip an
/// optional leading BOM; parse to a `Value` (a `Syntax` error carries serde's line/column);
/// check `schema == 2` BEFORE shape validation (so a schema-1 manifest fails with
/// `UnsupportedSchema`, never a confusing unknown-field error); typed-deserialize `Manifest`
/// FROM THE STRING (not the `Value`, so serde's shape errors keep their line/column); run
/// semantic validation (field-path errors); compute the hash from the same stripped bytes via
/// [`super::identity::canonical_hash`] (the shared primitive g09 already established).
pub fn parse_manifest(
    text: &str,
    source_label: &str,
    domain_pattern_valid: fn(&str) -> bool,
    is_known_tool: fn(&str) -> bool,
) -> Result<Manifest, ManifestError> {
    let stripped = text.strip_prefix('\u{feff}').unwrap_or(text);

    let value: serde_json::Value =
        serde_json::from_str(stripped).map_err(|e| ManifestError::Syntax {
            source_label: source_label.to_string(),
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
        })?;

    let schema_ok = value
        .get("schema")
        .and_then(serde_json::Value::as_u64)
        .is_some_and(|s| s == 2);
    if !schema_ok {
        let found = value
            .get("schema")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "<missing>".to_string());
        return Err(ManifestError::UnsupportedSchema {
            source_label: source_label.to_string(),
            found,
        });
    }

    let mut manifest: Manifest =
        serde_json::from_str(stripped).map_err(|e| ManifestError::Shape {
            source_label: source_label.to_string(),
            message: e.to_string(),
        })?;

    validate_semantics(&manifest, source_label, domain_pattern_valid, is_known_tool)?;

    manifest.hash = super::identity::canonical_hash(stripped.as_bytes())
        .expect("already validated as a JSON object by the shape-validation step above");

    Ok(manifest)
}

fn field_error(
    source_label: &str,
    path: impl Into<String>,
    reason: impl Into<String>,
) -> ManifestError {
    ManifestError::Field {
        source_label: source_label.to_string(),
        path: path.into(),
        reason: reason.into(),
    }
}

fn validate_semantics(
    manifest: &Manifest,
    source_label: &str,
    domain_pattern_valid: fn(&str) -> bool,
    is_known_tool: fn(&str) -> bool,
) -> Result<(), ManifestError> {
    if manifest.name.is_empty() {
        return Err(field_error(source_label, "name", "must not be empty"));
    }
    if manifest.version.is_empty() {
        return Err(field_error(source_label, "version", "must not be empty"));
    }

    let mut seen_ids = HashSet::new();
    for (i, grant) in manifest.grants.iter().enumerate() {
        validate_grant(
            grant,
            i,
            source_label,
            domain_pattern_valid,
            is_known_tool,
            &mut seen_ids,
        )?;
    }

    for (i, entry) in manifest.config.iter().enumerate() {
        validate_config_entry(entry, i, source_label, domain_pattern_valid)?;
    }

    Ok(())
}

fn validate_grant(
    grant: &Grant,
    index: usize,
    source_label: &str,
    domain_pattern_valid: fn(&str) -> bool,
    is_known_tool: fn(&str) -> bool,
    seen_ids: &mut HashSet<String>,
) -> Result<(), ManifestError> {
    let prefix = format!("grants[{index}]");

    if grant.id.is_empty() {
        return Err(field_error(
            source_label,
            format!("{prefix}.id"),
            "must not be empty",
        ));
    }
    if !seen_ids.insert(grant.id.clone()) {
        return Err(field_error(
            source_label,
            format!("{prefix}.id"),
            format!("duplicate grant id '{}'", grant.id),
        ));
    }

    if grant.domains.is_empty() {
        return Err(field_error(
            source_label,
            format!("{prefix}.domains"),
            "must have at least one domain pattern",
        ));
    }
    for (j, pattern) in grant.domains.iter().enumerate() {
        if !domain_pattern_valid(pattern) {
            return Err(field_error(
                source_label,
                format!("{prefix}.domains[{j}]"),
                format!("invalid domain pattern '{pattern}'"),
            ));
        }
    }

    if grant.tools.is_some() && grant.exclude_tools.is_some() {
        return Err(field_error(
            source_label,
            &prefix,
            "'tools' and 'exclude_tools' are mutually exclusive",
        ));
    }
    if let Some(tools) = &grant.tools {
        for (j, name) in tools.iter().enumerate() {
            if !is_known_tool(name) {
                return Err(field_error(
                    source_label,
                    format!("{prefix}.tools[{j}]"),
                    format!("unknown tool '{name}'"),
                ));
            }
        }
    }
    if let Some(exclude) = &grant.exclude_tools {
        for (j, name) in exclude.iter().enumerate() {
            if !is_known_tool(name) {
                return Err(field_error(
                    source_label,
                    format!("{prefix}.exclude_tools[{j}]"),
                    format!("unknown tool '{name}'"),
                ));
            }
        }
    }

    Ok(())
}

fn validate_config_entry(
    entry: &ConfigEntry,
    index: usize,
    source_label: &str,
    domain_pattern_valid: fn(&str) -> bool,
) -> Result<(), ManifestError> {
    let prefix = format!("config[{index}]");
    let Some(def) = crate::governance::config::key_def(&entry.key) else {
        return Err(field_error(
            source_label,
            format!("{prefix}.key"),
            format!("unknown config key '{}'", entry.key),
        ));
    };
    crate::governance::config::layers::validate_value(def, &entry.value, domain_pattern_valid)
        .map_err(|reason| field_error(source_label, format!("{prefix}.value"), reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn always_valid_pattern(_: &str) -> bool {
        true
    }

    fn no_tools_known(_: &str) -> bool {
        false
    }

    /// A test-local mirror of the 13-tool sacred surface (never the transport module's own
    /// tool-list path -- the a7 arch-test forbids that edge even in test code). The
    /// authoritative list and its own exhaustive cross-check against the live fixture live in
    /// that module's own tests.
    const SACRED_TOOLS: &[&str] = &[
        "tabs_context_mcp",
        "tabs_create_mcp",
        "navigate",
        "computer",
        "find",
        "form_input",
        "get_page_text",
        "javascript_tool",
        "read_console_messages",
        "read_network_requests",
        "read_page",
        "resize_window",
        "update_plan",
    ];
    fn is_known_tool(name: &str) -> bool {
        SACRED_TOOLS.contains(&name)
    }

    /// A test-local mirror of the section-5.1 domain-pattern grammar (never the browser
    /// plugin's own pattern module -- same reason as [`is_known_tool`] above). The
    /// authoritative implementation and its own exhaustive bypass-class tests live there.
    fn is_valid_pattern(p: &str) -> bool {
        if p.is_empty() || !p.is_ascii() {
            return false;
        }
        if p.contains('/')
            || p.contains(':')
            || p.contains('@')
            || p.chars().any(char::is_whitespace)
        {
            return false;
        }
        let host = match p.strip_prefix("*.") {
            Some(rest) if !rest.is_empty() && !rest.contains('*') => rest,
            Some(_) => return false,
            None if p.contains('*') => return false,
            None => p,
        };
        if host.starts_with('.') || host.ends_with('.') {
            return false;
        }
        host.split('.').all(|label| !label.is_empty())
    }

    fn minimal_json() -> String {
        r#"{"schema":2,"name":"a","version":"1","grants":[]}"#.to_string()
    }

    #[test]
    fn minimal_manifest_parses_with_expected_defaults() {
        let m =
            parse_manifest(&minimal_json(), "test", always_valid_pattern, is_known_tool).unwrap();
        assert_eq!(m.schema, 2);
        assert_eq!(m.name, "a");
        assert_eq!(m.version, "1");
        assert_eq!(m.mode, None);
        assert_eq!(m.identity, None);
        assert!(m.grants.is_empty());
        assert!(m.config.is_empty());
        assert_eq!(m.hash.len(), 64);
        assert!(m
            .hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn tools_null_alongside_exclude_tools_is_legal() {
        let json = r#"{
            "schema": 2, "name": "a", "version": "1",
            "grants": [{
                "id": "g1", "domains": ["example.com"], "access": "read",
                "tools": null, "exclude_tools": ["navigate"]
            }]
        }"#;
        let m = parse_manifest(json, "test", is_valid_pattern, is_known_tool).unwrap();
        assert_eq!(m.grants[0].tools, None);
        assert_eq!(
            m.grants[0].exclude_tools,
            Some(vec!["navigate".to_string()])
        );
    }

    #[test]
    fn missing_schema_is_unsupported() {
        let json = r#"{"name":"a","version":"1","grants":[]}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::UnsupportedSchema { .. }));
        assert!(err.to_string().contains("only schema 2"));
    }

    #[test]
    fn non_integer_schema_is_unsupported() {
        let json = r#"{"schema":"2","name":"a","version":"1","grants":[]}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::UnsupportedSchema { .. }));
    }

    #[test]
    fn schema_1_is_unsupported_not_a_shape_error() {
        let json = r#"{"schema":1,"name":"a","version":"1"}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        match err {
            ManifestError::UnsupportedSchema { found, .. } => assert_eq!(found, "1"),
            other => panic!("expected UnsupportedSchema, got {other:?}"),
        }
    }

    #[test]
    fn missing_name_is_a_shape_error() {
        let json = r#"{"schema":2,"version":"1","grants":[]}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn empty_name_is_a_field_error() {
        let json = r#"{"schema":2,"name":"","version":"1","grants":[]}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Field { ref path, .. } if path == "name"));
    }

    #[test]
    fn missing_version_is_a_shape_error() {
        let json = r#"{"schema":2,"name":"a","grants":[]}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn unknown_top_level_field_is_rejected() {
        let json = r#"{"schema":2,"name":"a","version":"1","grants":[],"defaults":{}}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
        assert!(err.to_string().contains("defaults"));
    }

    #[test]
    fn authored_hash_field_is_rejected() {
        let json = r#"{"schema":2,"name":"a","version":"1","grants":[],"hash":"deadbeef"}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
        assert!(err.to_string().contains("hash"));
    }

    #[test]
    fn invalid_mode_enum_value_is_a_shape_error() {
        let json = r#"{"schema":2,"name":"a","version":"1","mode":"audit","grants":[]}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    /// g15 required test 2: `"observe"`/`"enforce"` parse into the correct `EffectiveMode` at
    /// both the manifest level and the grant level (absent-yields-`None` is already pinned by
    /// `minimal_manifest_parses_with_expected_defaults`; the invalid-string cases are pinned by
    /// `invalid_mode_enum_value_is_a_shape_error` and `grant_mode_shadow_is_a_shape_error`).
    #[test]
    fn mode_observe_and_enforce_parse_at_manifest_and_grant_level() {
        let json = r#"{"schema":2,"name":"a","version":"1","mode":"observe","grants":[
            {"id":"g1","domains":["example.com"],"access":"read","mode":"enforce"}
        ]}"#;
        let m = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap();
        assert_eq!(
            m.mode,
            Some(crate::governance::ports::EffectiveMode::Observe)
        );
        assert_eq!(
            m.grants[0].mode,
            Some(crate::governance::ports::EffectiveMode::Enforce)
        );
    }

    #[test]
    fn missing_grants_is_a_shape_error() {
        let json = r#"{"schema":2,"name":"a","version":"1"}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn grants_not_an_array_is_a_shape_error() {
        let json = r#"{"schema":2,"name":"a","version":"1","grants":{}}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    fn grant_json(body: &str) -> String {
        format!(r#"{{"schema":2,"name":"a","version":"1","grants":[{body}]}}"#)
    }

    #[test]
    fn grant_missing_id_is_a_shape_error() {
        let json = grant_json(r#"{"domains":["example.com"],"access":"read"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn duplicate_grant_ids_are_rejected() {
        let json = r#"{"schema":2,"name":"a","version":"1","grants":[
                {"id":"g1","domains":["example.com"],"access":"read"},
                {"id":"g1","domains":["other.com"],"access":"read"}
            ]}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Field { .. }));
        assert!(err.to_string().contains("duplicate grant id 'g1'"));
    }

    #[test]
    fn grant_missing_domains_is_a_shape_error() {
        let json = grant_json(r#"{"id":"g1","access":"read"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn grant_empty_domains_is_a_field_error() {
        let json = grant_json(r#"{"id":"g1","domains":[],"access":"read"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(
            matches!(err, ManifestError::Field { ref path, .. } if path == "grants[0].domains")
        );
    }

    #[test]
    fn invalid_domain_patterns_are_each_rejected_with_the_pattern_named() {
        let cases = [
            "https://example.com",
            "example.com:8443",
            "example.com/path",
            "user@example.com",
            "ex*mple.com",
            "*",
            "*.",
            "foo.*.com",
            ".example.com",
            "example.com.",
            "example..com",
            "",
        ];
        for pattern in cases {
            let json = grant_json(&format!(
                r#"{{"id":"g1","domains":[{}],"access":"read"}}"#,
                serde_json::to_string(pattern).unwrap()
            ));
            let err = parse_manifest(&json, "test", is_valid_pattern, is_known_tool).unwrap_err();
            match err {
                ManifestError::Field { path, reason, .. } => {
                    assert_eq!(path, "grants[0].domains[0]");
                    assert!(
                        reason.contains(pattern),
                        "reason: {reason}, pattern: {pattern}"
                    );
                }
                other => panic!("pattern {pattern:?}: expected a Field error, got {other:?}"),
            }
        }
    }

    #[test]
    fn access_missing_is_a_shape_error() {
        let json = grant_json(r#"{"id":"g1","domains":["example.com"]}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn access_mutate_is_a_shape_error_naming_the_allowed_values() {
        let json = grant_json(r#"{"id":"g1","domains":["example.com"],"access":"mutate"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn unknown_tool_name_in_tools_is_rejected() {
        let json = grant_json(
            r#"{"id":"g1","domains":["example.com"],"access":"read","tools":["upload_image"]}"#,
        );
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Field { .. }));
        assert!(err.to_string().contains("upload_image"));
    }

    #[test]
    fn sub_action_name_in_exclude_tools_is_rejected() {
        let json = grant_json(
            r#"{"id":"g1","domains":["example.com"],"access":"read","exclude_tools":["left_click"]}"#,
        );
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Field { .. }));
        assert!(err.to_string().contains("left_click"));
    }

    #[test]
    fn tools_and_exclude_tools_both_present_is_rejected() {
        let json = grant_json(
            r#"{"id":"g1","domains":["example.com"],"access":"read","tools":["navigate"],"exclude_tools":["read_page"]}"#,
        );
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Field { .. }));
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn grant_mode_shadow_is_a_shape_error() {
        let json =
            grant_json(r#"{"id":"g1","domains":["example.com"],"access":"read","mode":"shadow"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    fn config_json(entry: &str) -> String {
        format!(r#"{{"schema":2,"name":"a","version":"1","grants":[],"config":[{entry}]}}"#)
    }

    #[test]
    fn config_entry_unregistered_key_is_rejected() {
        let json = config_json(r#"{"key":"no.such.key","value":true,"level":"mandatory"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Field { .. }));
        assert!(err.to_string().contains("no.such.key"));
    }

    #[test]
    fn config_entry_wrong_value_type_is_rejected() {
        let json = config_json(r#"{"key":"audit.enabled","value":"yes","level":"mandatory"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Field { .. }));
    }

    #[test]
    fn config_entry_missing_level_is_a_shape_error() {
        let json = config_json(r#"{"key":"audit.enabled","value":true}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn config_entry_level_optional_is_a_shape_error() {
        let json = config_json(r#"{"key":"audit.enabled","value":true,"level":"optional"}"#);
        let err = parse_manifest(&json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn identity_groups_as_a_string_is_a_shape_error() {
        let json = r#"{"schema":2,"name":"a","version":"1","grants":[],
            "identity":{"groups":"not-an-array"}}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn unknown_field_inside_identity_is_rejected() {
        let json = r#"{"schema":2,"name":"a","version":"1","grants":[],
            "identity":{"resolved_by":"x","unexpected":"y"}}"#;
        let err = parse_manifest(json, "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Shape { .. }));
    }

    #[test]
    fn syntax_error_carries_line_and_column() {
        let err =
            parse_manifest("{not json", "test", always_valid_pattern, is_known_tool).unwrap_err();
        assert!(matches!(err, ManifestError::Syntax { .. }));
    }

    // Silence an unused-function warning when `no_tools_known` is not exercised by every
    // configuration of the test module (kept as a documented stub for future negative cases).
    #[test]
    fn no_tools_known_stub_rejects_every_name() {
        assert!(!no_tools_known("navigate"));
    }
}
