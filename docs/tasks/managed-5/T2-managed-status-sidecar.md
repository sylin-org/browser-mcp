# T2: ManagedStatus sidecar -- single writer in managed::activate (ADR-0055 Impl.8)

## Goal
Every managed resolve writes a versioned, no-secrets `managed-status.json` sidecar beside the cache.
The sidecar IS the ManagedStatus store (ADR-0055 Impl.8 refinement): one writer (`activate`), three
readers (doctor T4, explain T5, Console later). No ConfigStore changes.

## Preconditions (verify, else STOP)
- `crates/core/src/governance/managed/mod.rs` has
  `pub fn activate(paths: &crate::governance::paths::GovernancePaths, domain_pattern_valid: fn(&str) -> bool) -> Result<Option<cache::Reconciled>, ManagedError>`
  which calls `cache::resolve_managed(&bootstrap, cache_path, domain_pattern_valid)`.
- `crates/core/src/governance/managed/cache.rs` has `pub struct Reconciled { pub active:
  Option<VerifiedManaged>, pub freshness: Freshness, pub persist_fresh: bool }`,
  `pub enum Freshness { Fresh, LastKnownGood(StaleReason), NoPolicy }`,
  `pub enum StaleReason { SourceUnreachable, UpdateRejected, RollbackRefused }`, and
  `pub fn write_cache(path, bytes)` using the temp+rename atomic pattern.
- `GovernancePaths` (`crates/core/src/governance/paths.rs`) has `managed_cache: Option<PathBuf>`.
- `chrono` is a ghostlight-core dependency (it is; used by license).

## Required behavior
1. New file `crates/core/src/governance/managed/status.rs` (SPDX LicenseRef-Ghostlight-Commercial;
   module doc citing ADR-0055 Impl.8). Public struct, serde Serialize+Deserialize:
   ```
   pub struct ManagedStatus {
       pub v: u32,                       // always 1
       pub freshness: String,            // "fresh" | "last_known_good" | "no_policy"
       pub stale_reason: Option<String>, // "source_unreachable" | "update_rejected" | "rollback_refused"
       pub seq: Option<u64>,
       pub fetched_at: String,           // chrono::Utc::now().to_rfc3339() at write time
       pub source: String,               // bootstrap.source verbatim
       pub presentation: Option<crate::governance::manifest::bundle::Presentation>,
       pub last_error: Option<String>,
   }
   ```
2. `pub fn from_reconciled(r: &cache::Reconciled, source: &str, last_error: Option<String>) ->
   ManagedStatus` mapping Freshness/StaleReason to EXACTLY the snake_case strings above
   (Fresh->"fresh"/None; LastKnownGood(x)->"last_known_good"/Some(snake of x); NoPolicy->
   "no_policy"/None), `seq`/`presentation` from `r.active` (presentation CLONED from the active
   VerifiedManaged).
3. `pub fn sidecar_path(cache_path: &Path) -> PathBuf` = cache_path's parent joined
   `"managed-status.json"` (fall back to `cache_path.with_file_name("managed-status.json")`).
4. `pub fn write_sidecar(path: &Path, s: &ManagedStatus) -> std::io::Result<()>` -- serde_json
   pretty bytes, atomic temp+rename (mirror `cache::write_cache`; reuse it if visibility allows,
   else duplicate the two lines).
5. `pub fn read_sidecar(path: &Path) -> Option<ManagedStatus>` -- None on absent/unparseable
   (readers degrade gracefully; the sidecar carries no trust).
6. In `managed/mod.rs`: `pub mod status;` and extend `activate` so that AFTER `resolve_managed`
   succeeds it builds `status::from_reconciled(&reconciled, &bootstrap.source, None)` and
   best-effort writes the sidecar (`if let Err(e) = ... { tracing::warn!(...) }` -- a sidecar write
   failure NEVER fails activation).

## Tests (in status.rs `mod tests`; pinned)
- `snake_case_mapping_is_exact`: construct Reconciled values for all three freshness states (reuse
  cache.rs test helpers pattern: sign a bundle via `bundle::sign_bundle`, verify via
  `verify_and_parse`) and assert the exact strings above, e.g. RollbackRefused ->
  `("last_known_good", Some("rollback_refused"))`.
- `sidecar_round_trips`: write_sidecar then read_sidecar; assert v==1, seq==Some(n), source
  matches.
- `read_sidecar_absent_or_garbage_is_none`: missing path -> None; a file containing `not json` ->
  None.
- In managed/mod.rs tests, extend `activate_resolves_a_configured_local_bundle`: after activate,
  assert `status::read_sidecar(&status::sidecar_path(paths.managed_cache.as_ref().unwrap()))`
  is Some with freshness=="fresh" and seq==Some(4).

## Verification (literal)
- `cargo test -p ghostlight-core --lib -- governance::managed` -> all pass.
- `cargo test -p ghostlight-core --lib --no-default-features -- governance::managed` -> all pass
  (air-gap build; status.rs must not touch ureq/rustls).
- Global verification per BOOTSTRAP.

## Out of scope
- No ConfigStore/reload.rs changes. No doctor/explain/denial changes (T4-T6). No encryption.

## Commit message (pinned)
`feat(managed): T2 ManagedStatus sidecar, single writer in activate (ADR-0055 Impl.8)`
