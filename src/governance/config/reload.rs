//! The ADR-0019 hot-reload substrate: the in-force resolved [`Config`] held behind an atomic
//! swap, a validate-then-swap re-resolve, a debounced file-watch on the three configuration
//! sources (user config file, org policy file, active manifest source), and a change signal
//! for the tool-advertisement layer (G14). The source-specific invalid-on-reload rule (lenient
//! user, fail-closed org) is a SECURITY rule, not a preference: a malformed org push never
//! drops an org lock or relaxes a value to a weaker layer.
//!
//! The swap slot is `Mutex<Arc<Config>>`, not `ArcSwap`: the read is a per-call event on the
//! dispatch chokepoint, not a hot inner loop, and the critical section is a single `Arc` clone
//! (an atomic refcount bump) followed by an immediate unlock, so reads never contend for more
//! than a few nanoseconds. `Mutex<Arc<Config>>` is `std`-only and needs zero new dependencies,
//! preserving the single-binary / zero-runtime-dependencies posture (ADR-0001); `arc-swap`
//! would be a second new crate for a lock-free property this call site does not need.
//!
//! The watcher is a zero-dependency debounced mtime poll, not the `notify` crate: it watches
//! exactly three known file paths (not recursive directory trees) that change rarely, so
//! polling `std::fs::metadata` on three paths every [`POLL_INTERVAL`] is negligible cost and
//! needs no new crate. `notify` pulls a platform-backend dependency tree disproportionate for
//! three files. The watcher is written behind a small abstraction so `notify` would be a
//! drop-in replacement without touching [`ConfigStore::reresolve`] if sub-second latency ever
//! matters.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;
use tokio::sync::watch;

use super::load::{OrgConfig, UserConfig};
use super::{layers, load, Config};

/// Poll interval for the source watcher. The config files change rarely (a user edit, a
/// `config set`, or an MDM push), so a sub-second poll on three known paths is negligible cost.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(750);

/// The in-force resolved configuration, held behind a single swappable slot so a re-resolve
/// replaces it atomically and every subsequent per-call read sees the new snapshot. Also holds
/// the last-good layer inputs the reloader falls back to (fail-closed for org policy) and the
/// change-signal channel G14 subscribes to.
pub struct ConfigStore {
    /// The in-force snapshot. A per-call read clones the `Arc` and releases the lock
    /// immediately; a reload stores a fresh `Arc` in one operation.
    snapshot: Mutex<Arc<Config>>,
    /// Monotonic reload generation; bumped on every successful swap. Lets a subscriber cheaply
    /// answer "did the snapshot change since I last looked".
    generation: AtomicU64,
    /// Broadcasts the new snapshot on every successful swap. G14 subscribes here to recompute
    /// the advertised tool set and emit `list_changed` when it differs.
    tx: watch::Sender<Arc<Config>>,
    /// Last successfully-applied layer inputs per source, retained so an invalid reload of one
    /// source keeps that source's last-good contribution.
    last_good: Mutex<LastGoodInputs>,
    /// The three fixed source paths watched for change.
    sources: WatchSources,
    /// Validates `content.security.sacred_domains` entries. Supplied by the caller (the
    /// browser plugin's real pattern-syntax checker) since this core module cannot name the
    /// browser plugin directly (the a7 arch-test forbids a `governance -> browser` edge; see
    /// RECONCILIATION.md section 2, the same integration point G01/G02 resolved).
    domain_pattern_valid: fn(&str) -> bool,
}

/// The last-good layer inputs, per source. On a reload where one source fails to load or
/// validate, the store re-composes from these so a failed source never weakens the resolved
/// posture (this is what makes org-policy failure fail-closed).
#[derive(Debug, Clone, Default)]
struct LastGoodInputs {
    /// Last-good org contribution (mandatory + recommended maps). Never dropped on an invalid
    /// org reload.
    org: OrgConfig,
    /// Last-good user-layer values.
    user: serde_json::Map<String, serde_json::Value>,
}

/// The fixed source paths the watcher polls. The manifest slot is an integration point for
/// G12: today it is `None` (no manifest engine); when G12 lands a file:// manifest source, its
/// path is set here so an edit triggers a re-resolve. An env:// or in-memory manifest source
/// has no file to watch and is left `None`.
#[derive(Debug, Clone)]
struct WatchSources {
    user_config: Option<PathBuf>,
    org_policy: PathBuf,
    // INTEGRATION POINT (G12): set to the active file:// manifest path so an edit triggers a
    // re-resolve and the G14 list_changed signal.
    manifest: Option<PathBuf>,
}

impl ConfigStore {
    /// The current in-force snapshot. This is the per-call read on the dispatch path: clone
    /// the `Arc` (cheap) and use it for the whole call, so a reload mid-call does not tear the
    /// snapshot the call already started with.
    pub fn current(&self) -> Arc<Config> {
        self.snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The current reload generation. Bumps by one on every successful swap.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Subscribe to snapshot changes. The receiver observes the new `Arc<Config>` after every
    /// successful swap. G14 uses this to recompute the advertised tool set and (when it
    /// changed) emit `notifications/tools/list_changed`; this store fires the signal on every
    /// config change, deciding whether the TOOL SET changed is G14's job. A subscriber created
    /// before any reload sees the startup snapshot as its current value and only wakes on
    /// subsequent swaps. This module never emits any MCP notification itself.
    pub fn subscribe(&self) -> watch::Receiver<Arc<Config>> {
        self.tx.subscribe()
    }

    /// Build the store from the initial layered load, called once at mcp-server startup.
    /// Startup keeps the G02 FAIL-LOUD semantics: an invalid org policy file or a structurally
    /// broken user file at startup is a hard error and the server refuses to start (it must
    /// never boot open on a broken org push). The lenient, keep-last-good behavior is for
    /// RELOAD only ([`Self::reresolve`]), where a server is already running on a known-good
    /// snapshot. `domain_pattern_valid` validates `content.security.sacred_domains` entries
    /// (the browser plugin's real pattern-syntax checker); it is retained for every later
    /// reload.
    pub fn load_initial(domain_pattern_valid: fn(&str) -> bool) -> crate::Result<Arc<ConfigStore>> {
        let sources = WatchSources {
            user_config: load::user_config_path(),
            org_policy: load::org_policy_path(),
            manifest: None,
        };

        let org = read_and_parse_org(&sources.org_policy, domain_pattern_valid);
        let user = read_and_parse_user(sources.user_config.as_deref(), domain_pattern_valid);
        let (last_good, warnings, preset) = compose_initial(org, user)?;

        for w in &warnings {
            tracing::warn!("config: {w}");
        }
        if let Some(name) = &preset {
            tracing::warn!(
                "config: preset '{name}' is declared in the user config file but preset \
                 defaults are not implemented yet, so it has no effect"
            );
        }

        let inputs = compose_inputs(&last_good);
        let resolution = layers::resolve(&inputs);
        let config = Arc::new(Config::from_resolution(&resolution));

        let (tx, _rx) = watch::channel(config.clone());
        Ok(Arc::new(ConfigStore {
            snapshot: Mutex::new(config),
            generation: AtomicU64::new(0),
            tx,
            last_good: Mutex::new(last_good),
            sources,
            domain_pattern_valid,
        }))
    }

    /// Re-run the layered load and resolver and, only if a full candidate parses and
    /// validates, swap it into the snapshot slot. This is validate-then-swap: a half-written or
    /// invalid file never becomes the in-force snapshot. Applies the source-specific rule via
    /// [`plan_reload`]: a failed user source keeps the last-good user layer (WARN); a failed
    /// org source keeps the last-good org layer (ERROR, fail-closed). Returns a report for
    /// logging, the control-plane, and tests. Never returns an error: a running server is never
    /// taken down by a reload; it keeps its last-good snapshot.
    pub fn reresolve(&self) -> ReloadReport {
        let org = read_and_parse_org(&self.sources.org_policy, self.domain_pattern_valid)
            .map_err(|e| e.to_string());
        let user = read_and_parse_user(
            self.sources.user_config.as_deref(),
            self.domain_pattern_valid,
        )
        .map_err(|e| e.to_string());

        let last_good = self
            .last_good
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let plan = plan_reload(org, user, &last_good);
        self.apply_plan(plan)
    }

    /// Trigger an immediate re-resolve now, bypassing the poll interval. This is the hook for
    /// IN-PROCESS config writers: the future options-page settings protocol (native-messaging
    /// `set_config_key`) calls this so an edit takes effect immediately. `config set` (G03)
    /// runs in a SEPARATE CLI process and writes the file, so ITS trigger is the file-watch
    /// seeing the write, not this method.
    pub fn notify_local_edit(&self) -> ReloadReport {
        self.reresolve()
    }

    /// Apply a reload plan: log its messages, resolve and build the candidate `Config`, retain
    /// the new last-good inputs, and swap the snapshot only if it changed.
    fn apply_plan(&self, plan: ReloadPlan) -> ReloadReport {
        for w in &plan.warnings {
            tracing::warn!("config reload: {w}");
        }
        for e in &plan.errors {
            tracing::error!("config reload: {e}");
        }

        // "Validate" = the candidate parsed and resolved cleanly. Resolution values are
        // already validated by the loaders, so from_resolution cannot fail.
        let resolution = layers::resolve(&plan.inputs);
        let candidate = Arc::new(Config::from_resolution(&resolution));

        // Retain the new last-good regardless of swap (a failed source contributed its own
        // last-good back into the plan, so this never weakens org posture).
        *self
            .last_good
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = plan.new_last_good;

        let changed = {
            let mut slot = self.snapshot.lock().unwrap_or_else(PoisonError::into_inner);
            if **slot == *candidate {
                false
            } else {
                *slot = candidate.clone();
                true
            }
        };

        let generation = if changed {
            let g = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
            // watch::send only errs if there are no receivers, which is fine.
            let _ = self.tx.send(candidate);
            g
        } else {
            self.generation.load(Ordering::Acquire)
        };

        ReloadReport {
            swapped: changed,
            org_failed: plan.org_failed,
            user_failed: plan.user_failed,
            generation,
            warnings: plan.warnings,
            errors: plan.errors,
        }
    }

    /// Spawn the debounced source watcher. mcp-server role ONLY (the native-host relay and the
    /// installer/config CLI roles must never start it). Polls the three source fingerprints
    /// every [`POLL_INTERVAL`]; when any source settles on a changed fingerprint, calls
    /// [`Self::reresolve`] once. Runs until the process exits. Takes `Arc<Self>` so the loop
    /// holds a strong reference to the store.
    pub fn spawn_watcher(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(POLL_INTERVAL);
            let mut watches: [PathWatch; 3] = Default::default();
            // Seed last_applied with the current fingerprints so the first poll does not
            // spuriously re-resolve the state we already loaded at startup.
            let paths = self.watched_paths();
            for (i, p) in paths.iter().enumerate() {
                let fp = p.as_deref().and_then(fingerprint);
                watches[i] = PathWatch {
                    last_seen: fp,
                    last_applied: fp,
                };
            }
            loop {
                interval.tick().await;
                let paths = self.watched_paths(); // the manifest slot may change under G12
                let mut trigger = false;
                for (i, p) in paths.iter().enumerate() {
                    let fp = p.as_deref().and_then(fingerprint);
                    let (next, fire) = settle(&watches[i], fp);
                    watches[i] = next;
                    trigger |= fire;
                }
                if trigger {
                    self.reresolve();
                }
            }
        });
    }

    /// The three watched paths in fixed order [user, org, manifest]; a `None` slot (no user
    /// config dir, or no file-based manifest source) is simply never a change. Recomputed each
    /// poll so a G12 manifest-source change is picked up.
    fn watched_paths(&self) -> [Option<PathBuf>; 3] {
        [
            self.sources.user_config.clone(),
            Some(self.sources.org_policy.clone()),
            self.sources.manifest.clone(),
        ]
    }
}

/// The pure reload plan: given fresh load attempts for each source and the current last-good
/// inputs, decide the new layer inputs, the new last-good, and the per-source outcome. No I/O.
/// This function encodes the security rule:
///
/// - User source Ok  -> adopt its values (and as new last-good); its per-entry warnings are
///   surfaced (WARN, not error).
/// - User source Err -> keep last-good user values; the structural failure is a WARNING (a
///   user file is user-serviceable; a broken one is stale, not fatal, once the server is
///   already running).
/// - Org source Ok   -> adopt it (and as new last-good).
/// - Org source Err  -> KEEP last-good org for BOTH the applied inputs and the new last-good,
///   and record an ERROR. FAIL-CLOSED: a malformed org push never drops an org lock or relaxes
///   an org value to a weaker layer. An org policy that silently fails open is worse than a
///   stale one.
fn plan_reload(
    org: Result<OrgConfig, String>,
    user: Result<(UserConfig, Vec<String>), String>,
    last_good: &LastGoodInputs,
) -> ReloadPlan {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let (org_result, org_failed) = match org {
        Ok(o) => (o, false),
        Err(e) => {
            errors.push(format!(
                "org policy failed to load/validate, keeping last-good: {e}"
            ));
            (last_good.org.clone(), true)
        }
    };

    let (user_values, user_failed) = match user {
        Ok((parsed, entry_warnings)) => {
            warnings.extend(entry_warnings);
            (parsed.values, false)
        }
        Err(e) => {
            warnings.push(format!(
                "user config failed to load, keeping last-good: {e}"
            ));
            (last_good.user.clone(), true)
        }
    };

    let new_last_good = LastGoodInputs {
        org: org_result.clone(),
        user: user_values.clone(),
    };
    let inputs = layers::LayerInputs {
        org_mandatory: org_result.mandatory,
        user: user_values,
        org_recommended: org_result.recommended,
        preset: serde_json::Map::new(),
    };

    ReloadPlan {
        inputs,
        new_last_good,
        warnings,
        errors,
        org_failed,
        user_failed,
    }
}

/// The outcome of [`plan_reload`]: the inputs to resolve, the new last-good to retain, and
/// human-readable messages split by severity.
struct ReloadPlan {
    inputs: layers::LayerInputs,
    new_last_good: LastGoodInputs,
    /// User per-entry problems and user structural failure (logged at WARN).
    warnings: Vec<String>,
    /// Org structural/validation failure (logged at ERROR; posture unchanged).
    errors: Vec<String>,
    org_failed: bool,
    user_failed: bool,
}

/// The result of a re-resolve, for logging, the control-plane, and tests.
#[derive(Debug, Clone)]
pub struct ReloadReport {
    /// True if a new, different snapshot was swapped in.
    pub swapped: bool,
    /// True if the org policy source failed to load/validate (last-good kept).
    pub org_failed: bool,
    /// True if the user config source failed structurally (last-good kept).
    pub user_failed: bool,
    /// The reload generation after this call.
    pub generation: u64,
    /// User-file warnings surfaced this reload.
    pub warnings: Vec<String>,
    /// Org-file errors surfaced this reload (posture unchanged; fail-closed).
    pub errors: Vec<String>,
}

/// Build the layer inputs from the last-good state (used by startup and when every source
/// fails on reload). The preset layer stays empty; presets are G18's job.
fn compose_inputs(last_good: &LastGoodInputs) -> layers::LayerInputs {
    layers::LayerInputs {
        org_mandatory: last_good.org.mandatory.clone(),
        user: last_good.user.clone(),
        org_recommended: last_good.org.recommended.clone(),
        preset: serde_json::Map::new(),
    }
}

/// The startup composition: given the raw org/user load results, compose the initial last-good
/// state or fail loud. Fail-loud on ANY error (org or user structural failure): a server that
/// has not started cannot serve a stale-but-safe snapshot. Factored out so the fail-loud
/// decision is testable without touching real files (contrast [`plan_reload`], which is the
/// keep-last-good decision used once the server is already running).
fn compose_initial(
    org: crate::Result<OrgConfig>,
    user: crate::Result<(UserConfig, Vec<String>)>,
) -> crate::Result<(LastGoodInputs, Vec<String>, Option<String>)> {
    let org = org?;
    let (user, warnings) = user?;
    let last_good = LastGoodInputs {
        org,
        user: user.values,
    };
    Ok((last_good, warnings, user.preset))
}

/// Read and parse the org policy file. `ErrorKind::NotFound` is normal (absence yields the
/// empty default); any other I/O error is a hard error (an org file that exists but is
/// unreadable must not yield a weaker posture).
fn read_and_parse_org(
    path: &Path,
    domain_pattern_valid: fn(&str) -> bool,
) -> crate::Result<OrgConfig> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            load::parse_org_config(&content, &path.display().to_string(), domain_pattern_valid)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(OrgConfig::default()),
        Err(e) => Err(crate::Error::Config(format!("{}: {e}", path.display()))),
    }
}

/// Read and parse the user config file. `None` path, or `ErrorKind::NotFound`, is normal
/// (absence yields the empty default); any other I/O error is a hard error.
fn read_and_parse_user(
    path: Option<&Path>,
    domain_pattern_valid: fn(&str) -> bool,
) -> crate::Result<(UserConfig, Vec<String>)> {
    let Some(path) = path else {
        return Ok((UserConfig::default(), Vec::new()));
    };
    match std::fs::read_to_string(path) {
        Ok(content) => {
            load::parse_user_config(&content, &path.display().to_string(), domain_pattern_valid)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok((UserConfig::default(), Vec::new()))
        }
        Err(e) => Err(crate::Error::Config(format!("{}: {e}", path.display()))),
    }
}

/// A cheap change fingerprint for a watched path: `None` when the file is absent, or
/// `(mtime, len)` when present. Absence is a distinct state, so a file being created or deleted
/// is detected, not just modified-in-place.
type Fingerprint = Option<(SystemTime, u64)>;

/// Compute the current fingerprint of a path. A metadata or mtime error is treated as absence
/// (`None`): an unreadable file is handled by the re-resolve's strict IO-error path, not by the
/// fingerprint.
fn fingerprint(path: &Path) -> Fingerprint {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| Some((m.modified().ok()?, m.len())))
}

/// Per-path watch state: the last fingerprint the loop saw, and the fingerprint that was in
/// force at the last applied re-resolve.
#[derive(Debug, Clone, Copy, Default)]
struct PathWatch {
    last_seen: Fingerprint,
    last_applied: Fingerprint,
}

/// Decide whether a path's change has SETTLED and should trigger a re-resolve. Debounce rule: a
/// change fires only once the current fingerprint (a) differs from `last_applied` (something
/// changed since we last resolved) AND (b) equals the immediately previous poll's fingerprint
/// (the file has stopped changing). This coalesces the multiple writes an editor or an MDM push
/// emits and lets the validate-then-swap backstop catch any half-written state that still slips
/// through. Returns the new `PathWatch` and whether to trigger.
fn settle(prev: &PathWatch, current: Fingerprint) -> (PathWatch, bool) {
    let stable = current == prev.last_seen;
    let changed = current != prev.last_applied;
    if stable && changed {
        (
            PathWatch {
                last_seen: current,
                last_applied: current,
            },
            true,
        )
    } else {
        (
            PathWatch {
                last_seen: current,
                last_applied: prev.last_applied,
            },
            false,
        )
    }
}

#[cfg(test)]
impl ConfigStore {
    /// Crate-visible test constructor for other modules' test suites (for example
    /// `transport::mcp::server`'s server-wiring tests): seeds a store at `config` with empty
    /// last-good inputs, touching no filesystem. `LastGoodInputs` stays private to this module,
    /// so this is the seam other modules use instead.
    pub(crate) fn for_test_with_config(config: Config) -> Arc<ConfigStore> {
        Self::for_test(
            config,
            LastGoodInputs {
                org: OrgConfig::default(),
                user: serde_json::Map::new(),
            },
        )
    }

    /// Test-only constructor: seeds the store without touching the filesystem.
    fn for_test(initial: Config, last_good: LastGoodInputs) -> Arc<ConfigStore> {
        let config = Arc::new(initial);
        let (tx, _rx) = watch::channel(config.clone());
        Arc::new(ConfigStore {
            snapshot: Mutex::new(config),
            generation: AtomicU64::new(0),
            tx,
            last_good: Mutex::new(last_good),
            sources: WatchSources {
                user_config: None,
                org_policy: PathBuf::new(),
                manifest: None,
            },
            domain_pattern_valid: |_| true,
        })
    }

    /// Test-only: drive a reload deterministically with injected org/user load results,
    /// bypassing the filesystem reads `reresolve` performs.
    fn reload_with(
        &self,
        org: Result<OrgConfig, String>,
        user: Result<(UserConfig, Vec<String>), String>,
    ) -> ReloadReport {
        let last_good = self
            .last_good
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let plan = plan_reload(org, user, &last_good);
        self.apply_plan(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_reload_adopts_both_sources() {
        let last_good = LastGoodInputs {
            org: OrgConfig {
                mandatory: serde_json::Map::from_iter([("x".to_string(), json!("old"))]),
                recommended: serde_json::Map::new(),
            },
            user: serde_json::Map::from_iter([("y".to_string(), json!("old"))]),
        };
        let org_a = OrgConfig {
            mandatory: serde_json::Map::from_iter([("a".to_string(), json!(1))]),
            recommended: serde_json::Map::from_iter([("b".to_string(), json!(2))]),
        };
        let user_a = UserConfig {
            preset: None,
            values: serde_json::Map::from_iter([("c".to_string(), json!(3))]),
        };
        let warns = vec!["some warning".to_string()];

        let plan = plan_reload(
            Ok(org_a.clone()),
            Ok((user_a.clone(), warns.clone())),
            &last_good,
        );
        assert!(!plan.org_failed);
        assert!(!plan.user_failed);
        assert_eq!(plan.inputs.org_mandatory, org_a.mandatory);
        assert_eq!(plan.inputs.org_recommended, org_a.recommended);
        assert_eq!(plan.inputs.user, user_a.values);
        assert_eq!(plan.new_last_good.org, org_a);
        assert_eq!(plan.new_last_good.user, user_a.values);
        assert_eq!(plan.warnings, warns);
        assert!(plan.errors.is_empty());
    }

    #[test]
    fn invalid_user_keeps_last_good_user_and_warns() {
        let last_good = LastGoodInputs {
            org: OrgConfig::default(),
            user: serde_json::Map::from_iter([("keep".to_string(), json!(true))]),
        };
        let org_a = OrgConfig {
            mandatory: serde_json::Map::from_iter([("m".to_string(), json!(1))]),
            recommended: serde_json::Map::new(),
        };
        let plan = plan_reload(Ok(org_a.clone()), Err("bad user".to_string()), &last_good);
        assert!(plan.user_failed);
        assert!(!plan.org_failed);
        assert_eq!(plan.inputs.user, last_good.user);
        assert_eq!(plan.inputs.org_mandatory, org_a.mandatory);
        assert!(plan.warnings.iter().any(|w| w.contains("bad user")));
        assert!(plan.errors.is_empty());
    }

    #[test]
    fn invalid_org_is_fail_closed() {
        let last_good = LastGoodInputs {
            org: OrgConfig {
                mandatory: serde_json::Map::from_iter([(
                    super::super::AUDIT_ENABLED.to_string(),
                    json!(true),
                )]),
                recommended: serde_json::Map::new(),
            },
            user: serde_json::Map::new(),
        };
        let plan = plan_reload(
            Err("bad org".to_string()),
            Ok((UserConfig::default(), Vec::new())),
            &last_good,
        );
        assert!(plan.org_failed);
        assert!(!plan.user_failed);
        assert_eq!(
            plan.inputs.org_mandatory.get(super::super::AUDIT_ENABLED),
            Some(&json!(true))
        );
        assert!(plan.errors.iter().any(|e| e.contains("bad org")));
        assert!(plan.warnings.is_empty());
        assert_eq!(plan.new_last_good.org, last_good.org);

        // End to end through the resolver: the mandatory value must still be in force.
        let resolution = layers::resolve(&plan.inputs);
        let config = Config::from_resolution(&resolution);
        assert!(config.audit_enabled());
    }

    #[test]
    fn both_sources_invalid_keeps_both_last_good() {
        let last_good = LastGoodInputs {
            org: OrgConfig {
                mandatory: serde_json::Map::from_iter([("m".to_string(), json!(1))]),
                recommended: serde_json::Map::new(),
            },
            user: serde_json::Map::from_iter([("u".to_string(), json!(2))]),
        };
        let plan = plan_reload(
            Err("bad org".to_string()),
            Err("bad user".to_string()),
            &last_good,
        );
        assert!(plan.org_failed && plan.user_failed);
        let expected = compose_inputs(&last_good);
        assert_eq!(plan.inputs.org_mandatory, expected.org_mandatory);
        assert_eq!(plan.inputs.user, expected.user);
        assert_eq!(plan.inputs.org_recommended, expected.org_recommended);
        assert!(plan.errors.iter().any(|e| e.contains("bad org")));
        assert!(plan.warnings.iter().any(|w| w.contains("bad user")));
    }

    fn org_with_audit_enabled(value: bool) -> OrgConfig {
        OrgConfig {
            mandatory: serde_json::Map::from_iter([(
                super::super::AUDIT_ENABLED.to_string(),
                json!(value),
            )]),
            recommended: serde_json::Map::new(),
        }
    }

    #[test]
    fn current_returns_last_swapped() {
        let initial = Config::minimal();
        let store = ConfigStore::for_test(initial.clone(), LastGoodInputs::default());
        let old = store.current();
        assert_eq!(*old, initial);

        let report = store.reload_with(
            Ok(org_with_audit_enabled(false)),
            Ok((UserConfig::default(), Vec::new())),
        );
        assert!(report.swapped);
        let new = store.current();
        assert_ne!(*new, *old);
        // The previously-held Arc is still valid and still holds the old value.
        assert_eq!(*old, initial);
    }

    #[tokio::test]
    async fn generation_and_signal_fire_only_on_change() {
        let store = ConfigStore::for_test(Config::minimal(), LastGoodInputs::default());
        let mut rx = store.subscribe();
        assert_eq!(store.generation(), 0);

        // A reload that resolves to the SAME config: no bump, no wake.
        let report = store.reload_with(
            Ok(OrgConfig::default()),
            Ok((UserConfig::default(), Vec::new())),
        );
        assert!(!report.swapped);
        assert_eq!(store.generation(), 0);
        let woke = tokio::time::timeout(std::time::Duration::from_millis(50), rx.changed()).await;
        assert!(woke.is_err(), "receiver must not wake on a no-op reload");

        // A reload that resolves to a DIFFERENT config: bumps generation and wakes the receiver.
        let report = store.reload_with(
            Ok(org_with_audit_enabled(false)),
            Ok((UserConfig::default(), Vec::new())),
        );
        assert!(report.swapped);
        assert_eq!(store.generation(), 1);
        let woke = tokio::time::timeout(std::time::Duration::from_millis(50), rx.changed()).await;
        assert!(woke.is_ok(), "receiver must wake on a real change");
        assert!(!rx.borrow().audit_enabled());
    }

    #[test]
    fn no_receivers_reload_still_swaps() {
        // for_test's watch::channel receiver is dropped immediately, so there are zero
        // receivers when reload_with runs.
        let store = ConfigStore::for_test(Config::minimal(), LastGoodInputs::default());
        let report = store.reload_with(
            Ok(org_with_audit_enabled(false)),
            Ok((UserConfig::default(), Vec::new())),
        );
        assert!(report.swapped);
        assert_eq!(report.generation, 1);
    }

    #[test]
    fn settle_debounces_until_stable() {
        let fp0 = Some((SystemTime::UNIX_EPOCH, 10));
        let fp1 = Some((
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1),
            20,
        ));
        let watch0 = PathWatch {
            last_seen: fp0,
            last_applied: fp0,
        };

        let (watch1, fired) = settle(&watch0, fp1);
        assert!(
            !fired,
            "first sighting of a new fingerprint must not fire yet"
        );
        assert_eq!(watch1.last_seen, fp1);
        assert_eq!(watch1.last_applied, fp0);

        let (watch2, fired) = settle(&watch1, fp1);
        assert!(
            fired,
            "a second poll seeing the same new fingerprint must fire"
        );
        assert_eq!(watch2.last_applied, fp1);

        let (_, fired) = settle(&watch2, fp1);
        assert!(
            !fired,
            "an unchanged fingerprint after applying must not fire again"
        );
    }

    #[test]
    fn settle_detects_create_and_delete() {
        let fp = Some((SystemTime::UNIX_EPOCH, 5));

        // None -> Some(fp): create (needs two polls to settle).
        let w0 = PathWatch::default();
        let (w1, fired) = settle(&w0, fp);
        assert!(!fired);
        let (w2, fired) = settle(&w1, fp);
        assert!(fired);

        // Some(fp) -> None: delete (needs two polls to settle).
        let (w3, fired) = settle(&w2, None);
        assert!(!fired);
        let (w4, fired) = settle(&w3, None);
        assert!(fired);

        // A flicker (one poll only) must not fire.
        let (w5, fired) = settle(&w4, fp);
        assert!(!fired, "first sighting, not yet stable");
        let (_, fired) = settle(&w5, None);
        assert!(!fired, "flickered back before stabilizing");
    }

    #[test]
    fn initial_load_is_fail_loud_on_org_error() {
        let org_err: crate::Result<OrgConfig> = Err(crate::Error::Config("bad org".into()));
        let user_ok: crate::Result<(UserConfig, Vec<String>)> =
            Ok((UserConfig::default(), Vec::new()));
        assert!(
            compose_initial(org_err, user_ok).is_err(),
            "startup must fail loud on an org error"
        );

        // The same failure, presented to the reload planner, must NOT propagate an error and
        // must keep the last-good org contribution instead.
        let last_good = LastGoodInputs {
            org: org_with_audit_enabled(true),
            user: serde_json::Map::new(),
        };
        let plan = plan_reload(
            Err("bad org".to_string()),
            Ok((UserConfig::default(), Vec::new())),
            &last_good,
        );
        assert!(plan.org_failed);
        assert!(plan.errors.iter().any(|e| e.contains("bad org")));
        assert_eq!(
            plan.inputs.org_mandatory.get(super::super::AUDIT_ENABLED),
            Some(&json!(true))
        );
    }
}
