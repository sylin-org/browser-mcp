// SPDX-License-Identifier: Apache-2.0 OR MIT
//! ADR-0044: `--instance <n> install` plans a full per-instance stack (a binary copy Chrome
//! launches by name, an instance-isolated native host + dirs, and a suffixed supervisor), while
//! the default install stays byte-identical. Drives `install --dry-run` as a subprocess (writes
//! nothing, runs no external command) and inspects the printed plan. `--all-browsers`/`--all-clients`
//! force a deterministic plan regardless of what is installed on the test machine.

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_ghostlight")
}

fn install_plan(instance: Option<&str>) -> String {
    let mut cmd = Command::new(bin());
    if let Some(n) = instance {
        cmd.arg("--instance").arg(n);
    }
    let out = cmd
        .args([
            "install",
            "--dry-run",
            "--all-browsers",
            "--all-clients",
            "--extension-id",
            &"a".repeat(32),
        ])
        .output()
        .expect("run ghostlight install --dry-run");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn default_install_plan_is_byte_identical_and_places_no_copy() {
    let plan = install_plan(None);
    assert!(
        plan.contains("Ghostlight Service"),
        "default supervisor is the unsuffixed name: {plan}"
    );
    assert!(
        !plan.contains("(dev)") && !plan.contains("ghostlight-dev"),
        "the default plan carries no instance suffix anywhere: {plan}"
    );
    assert!(
        !plan.contains("instance binary"),
        "the default instance places no per-instance binary copy: {plan}"
    );
}

#[test]
fn dev_install_plan_copies_a_named_binary_and_suffixes_the_whole_stack() {
    let plan = install_plan(Some("dev"));
    // The per-instance binary copy Chrome launches by name (ADR-0044 Decision 4, the multi-call
    // binary): a `ghostlight-dev` copy that the native host reads from its own argv[0].
    assert!(
        plan.contains("instance binary") && plan.contains("ghostlight-adapter-browser-dev"),
        "the dev plan copies a per-instance ghostlight-adapter-browser-dev binary: {plan}"
    );
    // The native-host name/manifest is instance-isolated.
    assert!(
        plan.contains("org.sylin.ghostlight.dev"),
        "the dev plan uses a suffixed native-host name: {plan}"
    );
    // The supervisor is suffixed (its label prefixes every supervisor plan line on all platforms).
    assert!(
        plan.contains("Ghostlight Service (dev)"),
        "the dev plan registers a suffixed supervisor: {plan}"
    );
}
