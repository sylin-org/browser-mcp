// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! The customer-facing `ghostlight policy sign` / `pubkey` / `publish` CLI (ADR-0055 Phase 1d): an
//! organization signs its OWN policy bundle with its OWN composite keypair.
//!
//! This is the customer-facing analog of the founder-only `license sign`
//! ([`crate::governance::license::cli`]). Unlike that one it ships in EVERY build, because managed://
//! is customer-operated (ADR-0055 Implementation Decision 1). All signing and bundle LOGIC lives in
//! [`crate::governance::manifest::bundle`] and [`crate::governance::crypto`]; this file only maps a
//! command to a read/sign/print action. Seeds are provisioned by the org out of band (for example
//! `openssl rand 32`), exactly as license seeds are (ADR-0028 Decision 10).

use std::path::PathBuf;

use crate::governance::crypto::admin as crypto_admin;
use crate::governance::manifest::bundle;

fn read_seed(path: &PathBuf) -> anyhow::Result<[u8; 32]> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("cannot read seed {}: {e}", path.display()))?;
    <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| anyhow::anyhow!("seed {} must be exactly 32 bytes", path.display()))
}

fn read_manifest_value(path: &PathBuf) -> anyhow::Result<serde_json::Value> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("cannot read manifest {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("manifest {} is not valid JSON: {e}", path.display()))
}

fn signed_bundle(
    seed: &PathBuf,
    mldsa_seed: &Option<PathBuf>,
    seq: u64,
    manifest: &PathBuf,
) -> anyhow::Result<Vec<u8>> {
    let ed = read_seed(seed)?;
    let mldsa = mldsa_seed.as_ref().map(read_seed).transpose()?;
    let manifest_value = read_manifest_value(manifest)?;
    Ok(bundle::sign_bundle(&ed, mldsa.as_ref(), seq, manifest_value, None))
}

fn write_bundle(bytes: &[u8], out: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let out_path = out.unwrap_or_else(|| PathBuf::from("policy.bundle.json"));
    std::fs::write(&out_path, bytes)
        .map_err(|e| anyhow::anyhow!("cannot write {}: {e}", out_path.display()))?;
    Ok(out_path)
}

/// `ghostlight policy sign`: sign a manifest into a policy bundle at publish sequence `seq`. Writes
/// the raw envelope to `out` (default `policy.bundle.json`) and prints the armored block to stdout.
pub fn sign(
    seed: PathBuf,
    mldsa_seed: Option<PathBuf>,
    seq: u64,
    manifest: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bytes = signed_bundle(&seed, &mldsa_seed, seq, &manifest)?;
    let out_path = write_bundle(&bytes, out)?;
    print!("{}", bundle::armor(&bytes));
    eprintln!("wrote {} (seq {seq})", out_path.display());
    Ok(())
}

/// `ghostlight policy pubkey`: print the org verifying key(s) as hex, for the managed.json bootstrap.
pub fn pubkey(seed: PathBuf, mldsa_seed: Option<PathBuf>) -> anyhow::Result<()> {
    let ed = read_seed(&seed)?;
    println!("pubkey_ed25519 {}", hex(&crypto_admin::ed_public(&ed)));
    if let Some(mpath) = mldsa_seed {
        let m = read_seed(&mpath)?;
        println!("pubkey_mldsa {}", hex(&crypto_admin::mldsa_public(&m)));
    }
    Ok(())
}

/// `ghostlight policy publish`: sign the manifest AND emit a ready-to-use managed.json bootstrap
/// snippet (org public key filled in) plus deployment guidance. The one-command org path.
pub fn publish(
    seed: PathBuf,
    mldsa_seed: Option<PathBuf>,
    seq: u64,
    manifest: PathBuf,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let bytes = signed_bundle(&seed, &mldsa_seed, seq, &manifest)?;
    let out_path = write_bundle(&bytes, out)?;

    let ed_hex = hex(&crypto_admin::ed_public(&read_seed(&seed)?));
    let mldsa_line = match &mldsa_seed {
        Some(m) => format!(",\n  \"pubkey_mldsa\": \"{}\"", hex(&crypto_admin::mldsa_public(&read_seed(m)?))),
        None => String::new(),
    };

    println!("Signed policy bundle written to {} (seq {seq}).", out_path.display());
    println!();
    println!("Host it anywhere your fleet can reach -- an HTTPS URL, an object store, a file share,");
    println!("or a USB stick -- then drop this managed.json in the admin policy directory");
    println!("(%ProgramData%\\ghostlight on Windows, /etc/ghostlight on Linux) via your MDM:");
    println!();
    println!("{{");
    println!("  \"source\": \"<where you hosted {}>\",", out_path.display());
    println!("  \"pubkey_ed25519\": \"{ed_hex}\"{mldsa_line}");
    println!("}}");
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_produces_a_loadable_bundle() {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let seed_path = dir.join(format!("gl-policy-cli-{pid}-seed"));
        let manifest_path = dir.join(format!("gl-policy-cli-{pid}-manifest.json"));
        let out_path = dir.join(format!("gl-policy-cli-{pid}-bundle.json"));
        std::fs::write(&seed_path, [55u8; 32]).unwrap();
        std::fs::write(
            &manifest_path,
            br#"{"schema":3,"name":"cli-test","version":"1","grants":[]}"#,
        )
        .unwrap();

        let result = sign(
            seed_path.clone(),
            None,
            3,
            manifest_path.clone(),
            Some(out_path.clone()),
        );

        // The bundle the CLI wrote must verify and parse against the key derived from the same seed.
        let outcome = result.and_then(|()| {
            let bytes = std::fs::read(&out_path)?;
            let key = bundle::org_key(&crypto_admin::ed_public(&[55u8; 32]), None).unwrap();
            let v = crate::governance::managed::verify_and_parse(&bytes, &key, |_| true)?;
            Ok::<_, anyhow::Error>(v)
        });

        for p in [&seed_path, &manifest_path, &out_path] {
            std::fs::remove_file(p).ok();
        }
        let v = outcome.expect("signed bundle loads");
        assert_eq!(v.manifest.name, "cli-test");
        assert_eq!(v.seq, 3);
    }
}
