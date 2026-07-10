// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! The `ghostlight license` CLI (ADR-0028 Decisions 3 and 10-11). `status` and `install` ship in
//! every build; `sign` and `pubkey` are the founder's offline authoring tools, gated behind the
//! `license-admin` feature (never enabled in a release build). All license LOGIC lives in the parent
//! module; this file only maps a command to a read/display/write action.

use std::path::PathBuf;

use super::LicenseState;

/// A parsed `ghostlight license` subcommand. `main.rs` builds this from clap and calls [`run`]; it
/// carries no license logic itself.
#[derive(Debug)]
pub enum LicenseCommand {
    /// Show the resolved license state (read-only; never stamps, never a finding).
    Status { file: Option<PathBuf> },
    /// Install a license from a file, an armored block, or stdin, into the user (or org) location.
    Install { source: Option<PathBuf>, org: bool },
    /// Sign claims into a license envelope (founder, offline; `license-admin`).
    #[cfg(feature = "license-admin")]
    Sign {
        seed: PathBuf,
        mldsa_seed: Option<PathBuf>,
        keygen: u32,
        claims: PathBuf,
        out: Option<PathBuf>,
    },
    /// Print the verifying key(s) for a seed, for embedding (founder, offline; `license-admin`).
    #[cfg(feature = "license-admin")]
    Pubkey {
        seed: PathBuf,
        mldsa_seed: Option<PathBuf>,
    },
}

/// Execute a `ghostlight license` subcommand.
pub fn run(cmd: LicenseCommand) -> anyhow::Result<()> {
    match cmd {
        LicenseCommand::Status { file } => status(file),
        LicenseCommand::Install { source, org } => install(source, org),
        #[cfg(feature = "license-admin")]
        LicenseCommand::Sign {
            seed,
            mldsa_seed,
            keygen,
            claims,
            out,
        } => admin::sign(seed, mldsa_seed, keygen, claims, out),
        #[cfg(feature = "license-admin")]
        LicenseCommand::Pubkey { seed, mldsa_seed } => admin::pubkey(seed, mldsa_seed),
    }
}

/// `ghostlight license status`: resolve from `--file` or from disk, print the state, exit 0.
fn status(file: Option<PathBuf>) -> anyhow::Result<()> {
    let (state, path) = match file {
        Some(p) => {
            let bytes = std::fs::read(&p)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", p.display()))?;
            let text = String::from_utf8_lossy(&bytes);
            let envelope = if super::is_armored(&text) {
                super::dearmor(&text).unwrap_or_default()
            } else {
                bytes
            };
            (super::resolve_bytes(&envelope), Some(p))
        }
        None => super::resolve_from_disk(),
    };

    println!("state: {}", super::state_summary(&state));
    if let LicenseState::Valid { claims, keygen } | LicenseState::Expired { claims, keygen } =
        &state
    {
        println!("  {:<10}{}", "tier", claims.tier);
        println!("  {:<10}{}", "licensee", claims.licensee);
        println!("  {:<10}{}", "org", claims.org);
        println!("  {:<10}{}", "seats", claims.seats);
        println!("  {:<10}{}", "issued", claims.issued);
        println!("  {:<10}{}", "expires", claims.expires);
        println!("  {:<10}{}", "keygen", keygen);
    }
    println!(
        "  {:<10}{}",
        "file",
        path.map(|p| p.display().to_string())
            .unwrap_or_else(|| "-".into())
    );
    // Licensing is observational: status is always a clean exit (ADR-0028 Decision 1).
    Ok(())
}

/// `ghostlight license install`: read a license (file arg, armored block, or stdin), validate it,
/// and copy it to the user (default) or org location.
fn install(source: Option<PathBuf>, org: bool) -> anyhow::Result<()> {
    let raw = match &source {
        Some(p) => {
            std::fs::read(p).map_err(|e| anyhow::anyhow!("cannot read {}: {e}", p.display()))?
        }
        None => {
            use std::io::Read as _;
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf)?;
            buf
        }
    };
    let text = String::from_utf8_lossy(&raw);
    let envelope = if super::is_armored(&text) {
        super::dearmor(&text)
            .ok_or_else(|| anyhow::anyhow!("the armored license block is malformed"))?
    } else {
        raw.clone()
    };

    match super::resolve_bytes(&envelope) {
        LicenseState::Invalid(reason) => {
            anyhow::bail!("refusing to install an invalid license: {reason}");
        }
        LicenseState::Expired { .. } => {
            eprintln!("warning: this license is expired; installing it anyway (ADR-0028: license state never affects behavior).");
        }
        LicenseState::Valid { .. } | LicenseState::NoLicense => {}
    }

    let dest = if org {
        super::org_license_path()
    } else {
        super::user_license_path()
            .ok_or_else(|| anyhow::anyhow!("no user config directory on this platform"))?
    };
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("cannot create {}: {e}", parent.display()))?;
    }
    // Store the exact envelope JSON bytes (de-armored if needed), so the on-disk file is always the
    // canonical form the resolver reads.
    std::fs::write(&dest, &envelope)
        .map_err(|e| anyhow::anyhow!("cannot write {}: {e}", dest.display()))?;
    println!("installed license to {}", dest.display());
    Ok(())
}

/// Founder-only offline authoring (ADR-0028 Decision 10). Compiled only under `license-admin`.
#[cfg(feature = "license-admin")]
mod admin {
    use super::*;
    use crate::governance::license::{crypto, Claims, TIERS};

    fn read_seed(path: &PathBuf) -> anyhow::Result<[u8; 32]> {
        let bytes = std::fs::read(path)
            .map_err(|e| anyhow::anyhow!("cannot read seed {}: {e}", path.display()))?;
        <[u8; 32]>::try_from(bytes.as_slice())
            .map_err(|_| anyhow::anyhow!("seed {} must be exactly 32 bytes", path.display()))
    }

    /// `license sign`: build an envelope over the claims. keygen 0 is Ed25519-only; keygen >= 1 is
    /// composite and requires an ML-DSA seed too.
    pub(super) fn sign(
        seed: PathBuf,
        mldsa_seed: Option<PathBuf>,
        keygen: u32,
        claims: PathBuf,
        out: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        let ed_seed = read_seed(&seed)?;
        let claims_json = std::fs::read(&claims)
            .map_err(|e| anyhow::anyhow!("cannot read claims {}: {e}", claims.display()))?;
        let parsed: Claims = serde_json::from_slice(&claims_json)
            .map_err(|e| anyhow::anyhow!("claims are not valid: {e}"))?;
        if !TIERS.contains(&parsed.tier.as_str()) {
            anyhow::bail!("unknown tier {:?}; one of {TIERS:?}", parsed.tier);
        }
        if keygen == 0 && parsed.tier != "evaluation" {
            anyhow::bail!("the development key (keygen 0) may only sign evaluation licenses");
        }
        // Canonicalize the claims bytes: sign and embed the SAME serialization.
        let claims_bytes = serde_json::to_vec(&parsed)?;
        let sig = crypto::admin::ed_sign(&ed_seed, &claims_bytes);

        let mut envelope = serde_json::Map::new();
        envelope.insert("v".into(), serde_json::json!(1));
        envelope.insert("keygen".into(), serde_json::json!(keygen));
        envelope.insert(
            "claims".into(),
            serde_json::json!(crate::b64::encode(&claims_bytes)),
        );
        envelope.insert("sig".into(), serde_json::json!(crate::b64::encode(&sig)));
        if keygen >= 1 {
            let mseed =
                read_seed(&mldsa_seed.ok_or_else(|| {
                    anyhow::anyhow!("keygen >= 1 is composite; pass --mldsa-seed")
                })?)?;
            let sig_mldsa = crypto::admin::mldsa_sign(&mseed, &claims_bytes);
            envelope.insert(
                "sig_mldsa".into(),
                serde_json::json!(crate::b64::encode(&sig_mldsa)),
            );
        }
        let envelope_bytes = serde_json::to_vec(&envelope)?;

        let out_path = out.unwrap_or_else(|| PathBuf::from("license.json"));
        std::fs::write(&out_path, &envelope_bytes)
            .map_err(|e| anyhow::anyhow!("cannot write {}: {e}", out_path.display()))?;
        // Emit both forms (ADR-0028 Decision 11): the JSON file, and the armored block to stdout.
        print!("{}", crate::governance::license::armor(&envelope_bytes));
        eprintln!("wrote {}", out_path.display());
        Ok(())
    }

    /// `license pubkey`: print the verifying key(s) as lowercase hex, for embedding in the table.
    pub(super) fn pubkey(seed: PathBuf, mldsa_seed: Option<PathBuf>) -> anyhow::Result<()> {
        let ed_seed = read_seed(&seed)?;
        let ed = crypto::admin::ed_public(&ed_seed);
        println!("ed25519 {}", hex(&ed));
        if let Some(mpath) = mldsa_seed {
            let mseed = read_seed(&mpath)?;
            let m = crypto::admin::mldsa_public(&mseed);
            println!("ml-dsa-65 {}", hex(&m));
        }
        Ok(())
    }

    fn hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}
