// SPDX-License-Identifier: Apache-2.0 OR MIT
//! H9 installer auto-start: pure builder tests for the per-user OS supervisor registration
//! (ADR-0030 Decision 8 amendment; docs/tasks/hub/H9-installer-autostart.md). These NEVER run
//! `schtasks`/`launchctl`/`systemctl` -- they only assert on the pure argv/plist/unit builders.
//! Real OS registration is verified by manual smoke (see the H9 LEDGER entry), not by cargo.

use ghostlight::install::supervisor::{register_steps, SupervisorStep};
use ghostlight::install::PlanCtx;
use std::path::PathBuf;

fn test_ctx() -> PlanCtx {
    PlanCtx {
        current_exe: PathBuf::from("/abs/ghostlight"),
        home: PathBuf::from("/home/u"),
        config: PathBuf::from("/home/u/.config"),
        local: PathBuf::from("/home/u/.local/share"),
    }
}

/// PINNED (H9): `schtasks /create /tn "Ghostlight Service" /tr "\"<exe>\" service" /sc onlogon
/// /rl limited /f`.
#[cfg(windows)]
#[test]
fn windows_task_register_command_is_pinned() {
    let ctx = test_ctx();
    let exe = std::path::Path::new(r"C:\Program Files\Ghostlight\ghostlight.exe");
    let steps = register_steps(exe, &ctx);
    let create = steps
        .iter()
        .find_map(|s| match s {
            SupervisorStep::Run(c) if c.program == "schtasks" => Some(c),
            _ => None,
        })
        .expect("a schtasks step exists");

    for expected in [
        "/tn",
        "Ghostlight Service",
        "/rl",
        "limited",
        "/sc",
        "onlogon",
    ] {
        assert!(
            create.args.iter().any(|a| a == expected),
            "missing arg {expected:?} in {:?}",
            create.args
        );
    }
    assert!(
        create.args.iter().any(|a| a.contains("service")),
        "the /tr launch string must invoke the 'service' subcommand: {:?}",
        create.args
    );
}

/// PINNED (H9): the rendered plist names the `service` subcommand and the pinned launchd label.
#[cfg(target_os = "macos")]
#[test]
fn macos_plist_names_the_service_subcommand() {
    let ctx = test_ctx();
    let exe = std::path::Path::new("/usr/local/bin/ghostlight");
    let steps = register_steps(exe, &ctx);
    let plist = steps
        .iter()
        .find_map(|s| match s {
            SupervisorStep::WriteFile { contents, .. } => Some(contents.clone()),
            _ => None,
        })
        .expect("a WriteFile step renders the plist");

    assert!(plist.contains("<string>service</string>"));
    assert!(plist.contains("org.sylin.ghostlight.service"));
}

/// PINNED (H9): the rendered unit names the `service` subcommand and restarts on failure.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn linux_unit_names_the_service_subcommand() {
    let ctx = test_ctx();
    let exe = std::path::Path::new("/usr/local/bin/ghostlight");
    let steps = register_steps(exe, &ctx);
    let unit = steps
        .iter()
        .find_map(|s| match s {
            SupervisorStep::WriteFile { contents, .. } => Some(contents.clone()),
            _ => None,
        })
        .expect("a WriteFile step renders the unit");

    assert!(unit.contains("ExecStart="));
    assert!(unit.contains("service"));
    assert!(unit.contains("Restart=on-failure"));
}
