//! Loads the two configuration files of shared format doc section 1 and produces the
//! [`LayerInputs`] for [`layers::resolve`]. Applies the per-file strictness matrix: lenient
//! per entry for the user config file (shared format 1.1), strict for the org policy file
//! (shared format 1.2, 4.4). Parsing is pure over `&str`; only the path functions and
//! [`load_and_resolve`] touch the filesystem.

use super::layers::{self, LayerInputs};
use super::{key_def, Preset};
use crate::{Error, Result};

/// Path of the user config file; `None` when the platform config dir is unavailable.
pub fn user_config_path() -> Option<std::path::PathBuf> {
    Some(dirs::config_dir()?.join("browser-mcp").join("config.json"))
}

/// Path of the org policy file (fixed per platform; shared format section 1.2). No flag,
/// environment variable, or config key relocates or bypasses this path.
pub fn org_policy_path() -> std::path::PathBuf {
    #[cfg(not(any(windows, target_os = "macos", unix)))]
    compile_error!("unsupported target platform");

    #[cfg(windows)]
    let path = {
        let program_data =
            std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        std::path::PathBuf::from(program_data)
            .join("browser-mcp")
            .join("policy.json")
    };
    #[cfg(target_os = "macos")]
    let path = std::path::PathBuf::from("/Library/Application Support/browser-mcp/policy.json");
    #[cfg(all(unix, not(target_os = "macos")))]
    let path = std::path::PathBuf::from("/etc/browser-mcp/policy.json");

    path
}

/// The parsed user config file (shared format section 1.1).
#[derive(Debug, Clone, Default)]
pub struct UserConfig {
    /// Validated preset name if one was declared: "fully_open", "safe", or "restricted".
    /// Mapped to its layer-4 defaults by [`layer_inputs`] (G18); `None` when no preset (or an
    /// unregistered one) is declared, leaving layer 4 empty.
    pub preset: Option<String>,
    /// Validated user-layer values by dotted key name.
    pub values: serde_json::Map<String, serde_json::Value>,
}

/// Parse the user config file content. `path` is used only in messages. Returns the parsed
/// file plus per-entry warnings for the caller to log.
///
/// The user file is user-serviceable, so one bad entry must not take the whole session down;
/// but an unreadable or structurally broken file is a hard error, because silently continuing
/// without the user's own settings (for example a sacred-domains list) would be fail-open on a
/// user-authored protection, and the engine is truthful.
pub fn parse_user_config(
    content: &str,
    path: &str,
    domain_pattern_valid: fn(&str) -> bool,
) -> Result<(UserConfig, Vec<String>)> {
    let stripped = content.strip_prefix('\u{feff}').unwrap_or(content);
    let root: serde_json::Value = serde_json::from_str(stripped)
        .map_err(|e| Error::Config(format!("{path}: invalid JSON: {e}")))?;
    let obj = root
        .as_object()
        .ok_or_else(|| Error::Config(format!("{path}: top level must be a JSON object")))?;

    let mut warnings = Vec::new();
    let mut preset = None;
    if let Some(p) = obj.get("preset") {
        let s = p
            .as_str()
            .ok_or_else(|| Error::Config(format!("{path}: 'preset' must be a string")))?;
        match Preset::from_name(s) {
            Some(_) => preset = Some(s.to_string()),
            None => warnings.push(format!("{path}: unknown preset '{s}', ignoring")),
        }
    }

    let mut values = serde_json::Map::new();
    if let Some(cfg) = obj.get("config") {
        let cfg_obj = cfg
            .as_object()
            .ok_or_else(|| Error::Config(format!("{path}: 'config' must be an object")))?;
        for (key, value) in cfg_obj {
            match key_def(key) {
                None => warnings.push(format!("{path}: unknown config key '{key}', ignoring")),
                Some(def) => match layers::validate_value(def, value, domain_pattern_valid) {
                    Ok(()) => {
                        values.insert(key.clone(), value.clone());
                    }
                    Err(reason) => {
                        warnings.push(format!("{path}: key '{key}': {reason}, ignoring"));
                    }
                },
            }
        }
    }

    for member in obj.keys() {
        if member != "preset" && member != "config" {
            warnings.push(format!(
                "{path}: unknown top-level member '{member}', ignoring"
            ));
        }
    }

    Ok((UserConfig { preset, values }, warnings))
}

/// The org-layer values extracted from the org policy file (shared format 1.2, 4.4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrgConfig {
    /// Entries with "level": "mandatory" -- layer 1, locked.
    pub mandatory: serde_json::Map<String, serde_json::Value>,
    /// Entries with "level": "recommended" -- layer 3.
    pub recommended: serde_json::Map<String, serde_json::Value>,
}

/// Parse the org policy file content. `path` is used only in messages. This consumes ONLY the
/// `schema` and `config` members; grants, `name`, `version`, `mode`, and `identity` belong to
/// the manifest tasks (G12+) and are neither read nor validated here.
///
/// EVERY violation is a hard error: org policy that cannot be applied exactly must stop the
/// server rather than silently degrade.
pub fn parse_org_config(
    content: &str,
    path: &str,
    domain_pattern_valid: fn(&str) -> bool,
) -> Result<OrgConfig> {
    let stripped = content.strip_prefix('\u{feff}').unwrap_or(content);
    let root: serde_json::Value = serde_json::from_str(stripped)
        .map_err(|e| Error::Config(format!("{path}: invalid JSON: {e}")))?;
    let obj = root
        .as_object()
        .ok_or_else(|| Error::Config(format!("{path}: top level must be a JSON object")))?;

    let schema = obj
        .get("schema")
        .ok_or_else(|| Error::Config(format!("{path}: missing 'schema'")))?;
    let schema_num = schema
        .as_u64()
        .ok_or_else(|| Error::Config(format!("{path}: 'schema' must be an integer")))?;
    if schema_num != 2 {
        return Err(Error::Config(format!(
            "{path}: unsupported schema version {schema_num} (expected 2)"
        )));
    }

    let mut mandatory = serde_json::Map::new();
    let mut recommended = serde_json::Map::new();
    let mut seen_keys = std::collections::HashSet::new();

    if let Some(config) = obj.get("config") {
        let entries = config
            .as_array()
            .ok_or_else(|| Error::Config(format!("{path}: 'config' must be an array")))?;
        for (idx, entry) in entries.iter().enumerate() {
            let entry_obj = entry
                .as_object()
                .ok_or_else(|| Error::Config(format!("{path}: config[{idx}] must be an object")))?;
            for member in entry_obj.keys() {
                if member != "key" && member != "value" && member != "level" {
                    return Err(Error::Config(format!(
                        "{path}: config[{idx}] has an unexpected member '{member}'"
                    )));
                }
            }
            let key = entry_obj
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    Error::Config(format!("{path}: config[{idx}] missing string 'key'"))
                })?;
            let level = entry_obj
                .get("level")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    Error::Config(format!(
                        "{path}: config[{idx}] ('{key}') missing string 'level'"
                    ))
                })?;
            let value = entry_obj.get("value").ok_or_else(|| {
                Error::Config(format!("{path}: config[{idx}] ('{key}') missing 'value'"))
            })?;

            let def = key_def(key).ok_or_else(|| {
                Error::Config(format!("{path}: config[{idx}]: unknown key '{key}'"))
            })?;
            layers::validate_value(def, value, domain_pattern_valid).map_err(|reason| {
                Error::Config(format!("{path}: config[{idx}] ('{key}'): {reason}"))
            })?;

            if !seen_keys.insert(key.to_string()) {
                return Err(Error::Config(format!(
                    "{path}: duplicate config key '{key}'"
                )));
            }

            match level {
                "mandatory" => {
                    mandatory.insert(key.to_string(), value.clone());
                }
                "recommended" => {
                    recommended.insert(key.to_string(), value.clone());
                }
                other => {
                    return Err(Error::Config(format!(
                        "{path}: config[{idx}] ('{key}'): invalid level '{other}' \
                         (expected 'mandatory' or 'recommended')"
                    )));
                }
            }
        }
    }

    Ok(OrgConfig {
        mandatory,
        recommended,
    })
}

/// Read `path`; `Ok(None)` when the file does not exist. Any other I/O error (for example
/// permission denied) is a hard error: a config file that exists but cannot be read must not
/// silently yield an all-open session.
fn read_optional(path: &std::path::Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Config(format!("{}: {e}", path.display()))),
    }
}

/// Both configuration files, read and parsed from their platform paths. Absence of either file
/// is normal (yields the type's default); `warnings` carries the user file's per-entry
/// warnings (shared format doc section 1.1, lenient-per-entry).
pub struct LoadedLayers {
    pub org: OrgConfig,
    pub user: UserConfig,
    pub warnings: Vec<String>,
}

/// Read and parse both configuration files from their platform paths. The one I/O entry point
/// every layered-load call site (server startup, the `config` CLI, hot-reload) shares, so the
/// two-file read is implemented exactly once.
pub fn read_layers(domain_pattern_valid: fn(&str) -> bool) -> Result<LoadedLayers> {
    let (user, warnings) = match user_config_path().map(|p| read_optional(&p).map(|c| (p, c))) {
        Some(Ok((path, Some(content)))) => {
            let path_str = path.display().to_string();
            parse_user_config(&content, &path_str, domain_pattern_valid)?
        }
        Some(Ok((_, None))) | None => (UserConfig::default(), Vec::new()),
        Some(Err(e)) => return Err(e),
    };

    let org_path = org_policy_path();
    let org = match read_optional(&org_path)? {
        Some(content) => {
            let path_str = org_path.display().to_string();
            parse_org_config(&content, &path_str, domain_pattern_valid)?
        }
        None => OrgConfig::default(),
    };

    Ok(LoadedLayers {
        org,
        user,
        warnings,
    })
}

/// Compose [`LayerInputs`] from parsed org/user state, mapping `preset_name` to its layer-4
/// defaults via [`super::preset_layer`] when it names a registered preset (G18). `preset_name`
/// is `None`, or names an unregistered preset, when no preset layer applies at all: layer 4
/// stays empty and resolution falls through to layer 5 (the built-in Minimal default).
pub fn layer_inputs(
    org: OrgConfig,
    user_values: serde_json::Map<String, serde_json::Value>,
    preset_name: Option<&str>,
) -> LayerInputs {
    let preset = preset_name
        .and_then(super::Preset::from_name)
        .map(super::preset_layer)
        .unwrap_or_default();
    LayerInputs {
        org_mandatory: org.mandatory,
        user: user_values,
        org_recommended: org.recommended,
        preset,
    }
}

/// Load both configuration files from their platform paths, log warnings, and resolve all
/// layers. Called once at mcp-server startup. Absence of either file is normal.
///
/// `domain_pattern_valid` validates `content.security.sacred_domains` entries. Callers outside
/// the governance core supply the browser plugin's real pattern-syntax checker; this module
/// cannot name it directly (the a7 arch-test forbids a `governance -> browser` edge).
pub fn load_and_resolve(domain_pattern_valid: fn(&str) -> bool) -> Result<layers::Resolution> {
    let loaded = read_layers(domain_pattern_valid)?;
    for w in &loaded.warnings {
        tracing::warn!("{w}");
    }
    let preset_name = loaded.user.preset.clone();
    let inputs = layer_inputs(loaded.org, loaded.user.values, preset_name.as_deref());
    Ok(layers::resolve(&inputs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn always_valid(_: &str) -> bool {
        true
    }

    #[test]
    fn missing_files_resolve_to_builtin_and_config_equals_minimal() {
        let resolution = layers::resolve(&LayerInputs::default());
        let config = super::super::Config::from_resolution(&resolution);
        assert_eq!(config, super::super::Config::minimal());
    }

    #[test]
    fn layer_inputs_maps_a_registered_preset_name_to_its_full_defaults() {
        let inputs = layer_inputs(
            OrgConfig::default(),
            serde_json::Map::new(),
            Some("fully_open"),
        );
        assert_eq!(
            inputs.preset,
            super::super::preset_layer(super::super::Preset::FullyOpen)
        );
        let resolution = layers::resolve(&inputs);
        assert_eq!(
            resolution.get(super::super::GOVERNANCE_MODE).unwrap().value,
            json!("observe")
        );
        assert_eq!(
            resolution
                .get(super::super::GOVERNANCE_MODE)
                .unwrap()
                .source,
            layers::Source::Preset
        );
    }

    #[test]
    fn layer_inputs_leaves_the_preset_layer_empty_for_none_or_an_unknown_name() {
        for name in [None, Some("extreme")] {
            let inputs = layer_inputs(OrgConfig::default(), serde_json::Map::new(), name);
            assert!(inputs.preset.is_empty(), "{name:?}");
            let resolution = layers::resolve(&inputs);
            assert_eq!(
                resolution
                    .get(super::super::GOVERNANCE_MODE)
                    .unwrap()
                    .source,
                layers::Source::Builtin
            );
        }
    }

    #[test]
    fn layer_inputs_never_lets_the_preset_layer_override_user_or_org() {
        let org = OrgConfig {
            mandatory: serde_json::Map::from_iter([(
                super::super::AUDIT_ENABLED.to_string(),
                json!(true),
            )]),
            recommended: serde_json::Map::new(),
        };
        let user = serde_json::Map::from_iter([(
            super::super::CONTENT_SECURITY_SECRETS_REDACT.to_string(),
            json!(false),
        )]);
        let inputs = layer_inputs(org, user, Some("restricted"));
        let resolution = layers::resolve(&inputs);
        assert_eq!(
            resolution.get(super::super::AUDIT_ENABLED).unwrap().source,
            layers::Source::OrgMandatory
        );
        assert_eq!(
            resolution
                .get(super::super::CONTENT_SECURITY_SECRETS_REDACT)
                .unwrap()
                .source,
            layers::Source::User
        );
    }

    #[test]
    fn malformed_user_file_is_an_error() {
        for content in ["not json", "[]", r#"{"preset": 3}"#, r#"{"config": []}"#] {
            let err = parse_user_config(content, "PATH_MARKER", always_valid).unwrap_err();
            assert!(err.to_string().contains("PATH_MARKER"), "{content}: {err}");
        }
    }

    #[test]
    fn unknown_user_key_warns_and_is_skipped() {
        let content = json!({
            "config": {
                "no.such.key": true,
                super::super::AUDIT_ENABLED: true,
            }
        })
        .to_string();
        let (parsed, warnings) = parse_user_config(&content, "p", always_valid).unwrap();
        assert_eq!(parsed.values.len(), 1);
        assert!(parsed.values.contains_key(super::super::AUDIT_ENABLED));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("no.such.key"));
    }

    #[test]
    fn invalid_user_value_warns_and_is_skipped() {
        let content = json!({
            "config": {
                super::super::AUDIT_ENABLED: "not a bool",
                super::super::CONTENT_SECURITY_SECRETS_REDACT: true,
            }
        })
        .to_string();
        let (parsed, warnings) = parse_user_config(&content, "p", always_valid).unwrap();
        assert_eq!(parsed.values.len(), 1);
        assert!(parsed
            .values
            .contains_key(super::super::CONTENT_SECURITY_SECRETS_REDACT));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains(super::super::AUDIT_ENABLED));
    }

    #[test]
    fn unknown_preset_warns_and_is_treated_as_absent() {
        let (parsed, warnings) =
            parse_user_config(r#"{"preset": "extreme"}"#, "p", always_valid).unwrap();
        assert_eq!(parsed.preset, None);
        assert_eq!(warnings.len(), 1);

        let (parsed, warnings) =
            parse_user_config(r#"{"preset": "safe"}"#, "p", always_valid).unwrap();
        assert_eq!(parsed.preset, Some("safe".to_string()));
        assert!(warnings.is_empty());
    }

    #[test]
    fn org_entries_populate_layers_by_level() {
        let mandatory_key = super::super::AUDIT_ENABLED;
        let recommended_key = super::super::CONTENT_SECURITY_SECRETS_REDACT;
        let content = json!({
            "schema": 2,
            "name": "acme",
            "version": "1",
            "config": [
                { "key": mandatory_key, "value": true, "level": "mandatory" },
                { "key": recommended_key, "value": true, "level": "recommended" },
            ]
        })
        .to_string();
        let org = parse_org_config(&content, "p", always_valid).unwrap();
        assert!(org.mandatory.contains_key(mandatory_key));
        assert!(org.recommended.contains_key(recommended_key));

        let inputs = LayerInputs {
            org_mandatory: org.mandatory,
            user: serde_json::Map::from_iter([(recommended_key.to_string(), json!(false))]),
            org_recommended: org.recommended,
            preset: serde_json::Map::new(),
        };
        let resolution = layers::resolve(&inputs);
        assert_eq!(
            resolution.get(mandatory_key).unwrap().source,
            layers::Source::OrgMandatory
        );
        assert!(resolution.get(mandatory_key).unwrap().locked);
        // A user-layer value overrides the org-recommended one.
        assert_eq!(
            resolution.get(recommended_key).unwrap().source,
            layers::Source::User
        );

        let inputs2 = LayerInputs {
            org_mandatory: serde_json::Map::new(),
            user: serde_json::Map::from_iter([(mandatory_key.to_string(), json!(false))]),
            org_recommended: serde_json::Map::from_iter([(
                recommended_key.to_string(),
                json!(true),
            )]),
            preset: serde_json::Map::new(),
        };
        let resolution2 = layers::resolve(&inputs2);
        assert_eq!(
            resolution2.get(recommended_key).unwrap().source,
            layers::Source::OrgRecommended
        );
        assert!(!resolution2.get(recommended_key).unwrap().locked);
    }

    #[test]
    fn org_entries_do_not_override_a_mandatory_key_from_the_user_layer() {
        let key = super::super::AUDIT_ENABLED;
        let inputs = LayerInputs {
            org_mandatory: serde_json::Map::from_iter([(key.to_string(), json!(true))]),
            user: serde_json::Map::from_iter([(key.to_string(), json!(false))]),
            org_recommended: serde_json::Map::new(),
            preset: serde_json::Map::new(),
        };
        let resolution = layers::resolve(&inputs);
        assert_eq!(
            resolution.get(key).unwrap().source,
            layers::Source::OrgMandatory
        );
        assert_eq!(resolution.get(key).unwrap().value, json!(true));
    }

    #[test]
    fn org_file_violations_are_errors() {
        let good_key = super::super::AUDIT_ENABLED;
        let cases = vec![
            "not json".to_string(),
            json!({ "name": "a", "version": "1" }).to_string(),
            json!({ "schema": 3, "name": "a", "version": "1" }).to_string(),
            json!({ "schema": "2", "name": "a", "version": "1" }).to_string(),
            json!({ "schema": 2, "config": "not an array" }).to_string(),
            json!({ "schema": 2, "config": [{ "key": "no.such.key", "value": true, "level": "mandatory" }] })
                .to_string(),
            json!({ "schema": 2, "config": [{ "key": good_key, "value": "bad", "level": "mandatory" }] })
                .to_string(),
            json!({ "schema": 2, "config": [{ "key": good_key, "value": true, "level": "optional" }] })
                .to_string(),
            json!({ "schema": 2, "config": [{ "key": good_key, "value": true }] }).to_string(),
            json!({
                "schema": 2,
                "config": [
                    { "key": good_key, "value": true, "level": "mandatory" },
                    { "key": good_key, "value": false, "level": "recommended" },
                ]
            })
            .to_string(),
            json!({ "schema": 2, "config": [{ "key": good_key, "value": true, "level": "mandatory", "extra": 1 }] })
                .to_string(),
        ];
        for content in cases {
            let err = parse_org_config(&content, "ORG_PATH_MARKER", always_valid).unwrap_err();
            assert!(
                err.to_string().contains("ORG_PATH_MARKER"),
                "{content}: {err}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn paths_follow_the_shared_format_locations() {
        let user = user_config_path().expect("config dir resolvable in CI/dev");
        assert!(user.to_string_lossy().ends_with(r"browser-mcp\config.json"));
        let org = org_policy_path();
        assert!(org.to_string_lossy().ends_with(r"browser-mcp\policy.json"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn paths_follow_the_shared_format_locations() {
        let org = org_policy_path();
        assert_eq!(
            org,
            std::path::PathBuf::from("/Library/Application Support/browser-mcp/policy.json")
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn paths_follow_the_shared_format_locations() {
        let org = org_policy_path();
        assert_eq!(
            org,
            std::path::PathBuf::from("/etc/browser-mcp/policy.json")
        );
    }
}
