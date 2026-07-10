// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! The licensing bounded context (ADR-0028).
//!
//! ONE module owns everything license-related: the composite crypto ([`crypto`]), the claims and
//! state domain, envelope parsing and armoring, disk resolution, the observability decision
//! ([`stamp_for`]), and the CLI ([`cli`]). The rest of the codebase touches licensing only through
//! thin composition-root seams (the CLI subcommand, the doctor section, the recorder's opaque stamp,
//! and the hub startup call) and holds no license logic. This is deliberate SoC (owner-directed):
//! there are no scattered `if license...` checks anywhere else.
//!
//! Licensing is purely OBSERVATIONAL (ADR-0028 Decision 1): nothing here ever enables, disables, or
//! degrades any behavior. The only effect is a marker appended to the user's own audit records, and
//! only when governance is actually operating -- see [`stamp_for`] and its callers.

pub mod cli;
mod crypto;

pub use crypto::DEV_SEED;

/// The claims carried inside a license envelope (ADR-0028 Decision 2). `seats` and `licensee` are
/// legal terms only; they are never enforced at runtime (runtime seat counting would require a
/// phone-home, which Decision 9 forbids).
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

/// The valid tier names (ADR-0028 Decision 5 as amended: `development` was renamed `evaluation`).
pub const TIERS: &[&str] = &["evaluation", "community", "founding", "team", "enterprise"];

/// The product identifier this build's licenses must cover.
const PRODUCT: &str = "browser";

/// A resolved license state. `keygen` rides on the dated states for display and so the stamp logic
/// can reason about the (public) development generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseState {
    /// No license file present.
    NoLicense,
    /// A valid, in-date license.
    Valid { claims: Claims, keygen: u32 },
    /// A validly-signed but expired license.
    Expired { claims: Claims, keygen: u32 },
    /// Present but unusable, with a short human-readable reason.
    Invalid(String),
}

/// The on-disk envelope (ADR-0028 Decisions 2 and 11). `sig_mldsa` is absent for the Ed25519-only
/// development generation and present for composite production generations.
#[derive(serde::Deserialize)]
struct Envelope {
    v: u32,
    keygen: u32,
    claims: String,
    sig: String,
    #[serde(default)]
    sig_mldsa: Option<String>,
}

/// Parse and verify one envelope's bytes. Never panics on any input; every failure becomes
/// `Invalid(reason)`.
pub fn resolve_bytes(bytes: &[u8]) -> LicenseState {
    let env: Envelope = match serde_json::from_slice(bytes) {
        Ok(e) => e,
        Err(e) => return LicenseState::Invalid(format!("not a license envelope: {e}")),
    };
    if env.v != 1 {
        return LicenseState::Invalid(format!("unsupported envelope version {}", env.v));
    }
    let Some(claims_bytes) = crate::b64::decode(&env.claims) else {
        return LicenseState::Invalid("claims are not valid base64".into());
    };
    let Some(sig) = crate::b64::decode(&env.sig) else {
        return LicenseState::Invalid("signature is not valid base64".into());
    };
    if sig.len() != crypto::ED_SIG_LEN {
        return LicenseState::Invalid("ed25519 signature has the wrong length".into());
    }
    let sig_mldsa = match &env.sig_mldsa {
        Some(s) => match crate::b64::decode(s) {
            Some(d) if d.len() == crypto::MLDSA_SIG_LEN => Some(d),
            Some(_) => {
                return LicenseState::Invalid("ml-dsa signature has the wrong length".into())
            }
            None => return LicenseState::Invalid("ml-dsa signature is not valid base64".into()),
        },
        None => None,
    };
    let Some(key) = crypto::verifying_key(env.keygen) else {
        return LicenseState::Invalid(format!("unknown key generation {}", env.keygen));
    };
    if !crypto::verify(&key, &claims_bytes, &sig, sig_mldsa.as_deref()) {
        return LicenseState::Invalid("signature verification failed".into());
    }
    let claims: Claims = match serde_json::from_slice(&claims_bytes) {
        Ok(c) => c,
        Err(e) => return LicenseState::Invalid(format!("malformed claims: {e}")),
    };
    if !TIERS.contains(&claims.tier.as_str()) {
        return LicenseState::Invalid(format!("unknown tier {:?}", claims.tier));
    }
    if !claims.products.iter().any(|p| p == PRODUCT) {
        return LicenseState::Invalid("license does not cover the browser product".into());
    }
    // The public development generation may only sign evaluation licenses; a gen-0 license claiming
    // any other tier is a forgery attempt (the seed is public) and is rejected here, so the "valid
    // production" state is reachable only through a private (composite) generation.
    if env.keygen == 0 && claims.tier != "evaluation" {
        return LicenseState::Invalid(
            "development-key licenses may only claim the evaluation tier".into(),
        );
    }
    if chrono::NaiveDate::parse_from_str(&claims.expires, "%Y-%m-%d").is_err() {
        return LicenseState::Invalid("expires is not a YYYY-MM-DD date".into());
    }
    // Lexicographic comparison on YYYY-MM-DD against today's UTC date (ISO dates compare correctly
    // as strings; ADR-0028 Decision 2).
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    if claims.expires.as_str() < today.as_str() {
        LicenseState::Expired {
            claims,
            keygen: env.keygen,
        }
    } else {
        LicenseState::Valid {
            claims,
            keygen: env.keygen,
        }
    }
}

/// The audit stamp for a resolved state (ADR-0028 Decision 3, refined 2026-07-10). Callers invoke
/// this ONLY when governance is operationally in effect via an org-deployed policy; in the free
/// all-open path the licensing engine stays dormant and no stamp is produced. `None` means no
/// marker (a normal, licensed production deployment, or nothing to say).
pub fn stamp_for(state: &LicenseState) -> Option<&'static str> {
    match state {
        LicenseState::NoLicense => Some("unlicensed"),
        LicenseState::Invalid(_) => Some("invalid"),
        LicenseState::Expired { .. } => Some("expired"),
        // A gen-0 valid license is always evaluation-tier (enforced above); a founder-signed
        // evaluation license stamps the same. Either way it is not a paid production license.
        LicenseState::Valid { claims, .. } if claims.tier == "evaluation" => Some("evaluation"),
        LicenseState::Valid { .. } => None,
    }
}

// --- ASCII-armored block (ADR-0028 Decision 11) ---------------------------------------------------

const ARMOR_BEGIN: &str = "-----BEGIN GHOSTLIGHT LICENSE-----";
const ARMOR_END: &str = "-----END GHOSTLIGHT LICENSE-----";

/// Wrap envelope JSON bytes as an ASCII-armored block (base64 wrapped at 64 columns). The armored
/// payload decodes to the EXACT envelope bytes, so both forms verify identically.
pub fn armor(envelope_json: &[u8]) -> String {
    let b64 = crate::b64::encode(envelope_json);
    let mut out = String::with_capacity(b64.len() + ARMOR_BEGIN.len() + ARMOR_END.len() + 16);
    out.push_str(ARMOR_BEGIN);
    out.push('\n');
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).expect("base64 is ascii"));
        out.push('\n');
    }
    out.push_str(ARMOR_END);
    out.push('\n');
    out
}

/// Extract envelope JSON bytes from an ASCII-armored block, or `None` if the markers are absent or
/// the body is not valid base64. Whitespace between the markers is ignored.
pub fn dearmor(block: &str) -> Option<Vec<u8>> {
    let start = block.find(ARMOR_BEGIN)? + ARMOR_BEGIN.len();
    let end = block[start..].find(ARMOR_END)? + start;
    let body: String = block[start..end].split_whitespace().collect();
    crate::b64::decode(&body)
}

/// True when the input looks like an armored license block (vs. a raw JSON envelope).
pub fn is_armored(s: &str) -> bool {
    s.contains(ARMOR_BEGIN)
}

// --- Disk resolution ------------------------------------------------------------------------------

/// The org license file: `license.json` in the org policy directory.
pub fn org_license_path() -> std::path::PathBuf {
    let org = crate::governance::config::load::org_policy_path();
    match org.parent() {
        Some(dir) => dir.join("license.json"),
        None => org.with_file_name("license.json"),
    }
}

/// The user license file: `license.json` beside the user config file; `None` when the platform
/// config directory is unavailable.
pub fn user_license_path() -> Option<std::path::PathBuf> {
    crate::governance::config::load::user_config_path().map(|p| p.with_file_name("license.json"))
}

/// Resolve the license from disk: the org path first, then the user path (the first file that exists
/// is THE license; no merging). `NoLicense` when neither exists. Returns the state and the source
/// path (`None` when no file was found).
pub fn resolve_from_disk() -> (LicenseState, Option<std::path::PathBuf>) {
    for path in [Some(org_license_path()), user_license_path()]
        .into_iter()
        .flatten()
    {
        if path.exists() {
            let state = match std::fs::read(&path) {
                Ok(bytes) => resolve_bytes(&bytes),
                Err(e) => LicenseState::Invalid(format!("unreadable license file: {e}")),
            };
            return (state, Some(path));
        }
    }
    (LicenseState::NoLicense, None)
}

// --- Read-only display (shared by `doctor` and `license status`) ----------------------------------

/// A one-line human summary of a resolved state. Read-only: never stamps, never a doctor finding.
pub fn state_summary(state: &LicenseState) -> String {
    match state {
        LicenseState::NoLicense => {
            "none (no license installed; not required for personal or all-open use)".into()
        }
        LicenseState::Invalid(reason) => format!("invalid: {reason}"),
        LicenseState::Expired { claims, .. } => format!(
            "expired {} ({}, {})",
            claims.expires, claims.tier, claims.licensee
        ),
        LicenseState::Valid { claims, .. } if claims.tier == "evaluation" => {
            format!(
                "evaluation ({}, expires {})",
                claims.licensee, claims.expires
            )
        }
        LicenseState::Valid { claims, .. } => format!(
            "valid ({}, {}, expires {})",
            claims.tier, claims.licensee, claims.expires
        ),
    }
}

/// The `License:` section lines for `ghostlight doctor` (read-only; never contributes a finding).
pub fn doctor_section_lines() -> Vec<String> {
    let (state, path) = resolve_from_disk();
    vec![
        format!("  {:<9}{}", "state", state_summary(&state)),
        format!(
            "  {:<9}{}",
            "file",
            path.map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".into())
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(tier: &str, expires: &str) -> Claims {
        Claims {
            id: "00000000-0000-4000-8000-000000000001".into(),
            licensee: "Test Licensee".into(),
            org: "test".into(),
            tier: tier.into(),
            seats: 1,
            products: vec!["browser".into()],
            issued: "2026-07-10".into(),
            expires: expires.into(),
        }
    }

    /// Build a gen-0 (Ed25519) envelope over `claims`, signed with the public dev seed.
    fn dev_envelope(claims: &Claims) -> Vec<u8> {
        let claims_bytes = serde_json::to_vec(claims).unwrap();
        let sig = crypto::admin::ed_sign(crypto::DEV_SEED, &claims_bytes);
        let env = serde_json::json!({
            "v": 1,
            "keygen": 0,
            "claims": crate::b64::encode(&claims_bytes),
            "sig": crate::b64::encode(&sig),
        });
        serde_json::to_vec(&env).unwrap()
    }

    #[test]
    fn valid_evaluation_license_resolves_and_stamps_evaluation() {
        let bytes = dev_envelope(&claims("evaluation", "2126-01-01"));
        let state = resolve_bytes(&bytes);
        assert!(matches!(state, LicenseState::Valid { .. }), "got {state:?}");
        assert_eq!(stamp_for(&state), Some("evaluation"));
    }

    #[test]
    fn expired_license_is_expired_and_stamps_expired() {
        let state = resolve_bytes(&dev_envelope(&claims("evaluation", "2000-01-01")));
        assert!(
            matches!(state, LicenseState::Expired { .. }),
            "got {state:?}"
        );
        assert_eq!(stamp_for(&state), Some("expired"));
    }

    #[test]
    fn dev_key_cannot_claim_a_paid_tier() {
        // The seed is public, so a gen-0 "enterprise" license must be rejected, not treated as a
        // fully-licensed production deployment.
        let state = resolve_bytes(&dev_envelope(&claims("enterprise", "2126-01-01")));
        assert!(matches!(state, LicenseState::Invalid(_)), "got {state:?}");
        assert_eq!(stamp_for(&state), Some("invalid"));
    }

    #[test]
    fn tampered_claims_fail_verification() {
        let mut bytes = dev_envelope(&claims("evaluation", "2126-01-01"));
        // Flip a byte inside the base64 claims field region; the signature no longer matches.
        let idx = bytes.iter().position(|&b| b == b'T').unwrap(); // 'Test Licensee' -> claims b64 differs
        bytes[idx] ^= 0x01;
        assert!(matches!(resolve_bytes(&bytes), LicenseState::Invalid(_)));
    }

    #[test]
    fn unknown_generation_is_invalid() {
        let claims_bytes = serde_json::to_vec(&claims("team", "2126-01-01")).unwrap();
        let env = serde_json::json!({
            "v": 1, "keygen": 99,
            "claims": crate::b64::encode(&claims_bytes),
            "sig": crate::b64::encode(&[0u8; crypto::ED_SIG_LEN]),
        });
        let state = resolve_bytes(&serde_json::to_vec(&env).unwrap());
        match state {
            LicenseState::Invalid(r) => assert!(r.contains("unknown key generation"), "{r}"),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn wrong_product_is_invalid() {
        let mut c = claims("evaluation", "2126-01-01");
        c.products = vec!["something-else".into()];
        assert!(matches!(
            resolve_bytes(&dev_envelope(&c)),
            LicenseState::Invalid(_)
        ));
    }

    #[test]
    fn garbage_is_invalid_not_a_panic() {
        assert!(matches!(
            resolve_bytes(b"not json"),
            LicenseState::Invalid(_)
        ));
        assert!(matches!(resolve_bytes(b"{}"), LicenseState::Invalid(_)));
    }

    #[test]
    fn armor_round_trips_through_resolution() {
        let bytes = dev_envelope(&claims("evaluation", "2126-01-01"));
        let block = armor(&bytes);
        assert!(is_armored(&block));
        assert!(block.contains(ARMOR_BEGIN) && block.contains(ARMOR_END));
        let recovered = dearmor(&block).expect("dearmor");
        assert_eq!(recovered, bytes, "armored payload is the exact envelope");
        assert!(matches!(
            resolve_bytes(&recovered),
            LicenseState::Valid { .. }
        ));
    }

    #[test]
    fn no_license_stamps_unlicensed() {
        assert_eq!(stamp_for(&LicenseState::NoLicense), Some("unlicensed"));
    }
}
