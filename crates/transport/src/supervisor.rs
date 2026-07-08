// SPDX-License-Identifier: Apache-2.0 OR MIT
//! The OS supervisor identifiers + best-effort self-heal start (ADR-0030 Decision 8 amendment;
//! PINS.md SS5.2). The installer (H9) registers a per-user, zero-admin OS supervisor under these
//! SAME names (Windows Task Scheduler; macOS launchd; Linux systemd --user) that keeps
//! `ghostlight service` warm and restarts it on crash. When a thin ADAPTER's first dial to the
//! service fails, it asks this SAME supervisor to start the service (idempotent, out-of-job)
//! before retrying the dial -- never spawning an in-job child itself (that mechanism is deleted;
//! ADR-0030 Decision 8 Provenance, "the always-ready-service amendment").

use crate::role;
use std::time::Duration;

/// Windows Task Scheduler task name for the active instance (ADR-0044). The default instance
/// yields `Ghostlight Service` (the PINNED name H9 registers, PINS.md SS5.2); a named instance
/// yields `Ghostlight Service (<n>)`.
pub fn supervisor_task_name() -> String {
    crate::instance::Instance::resolve().supervisor_task_name()
}

/// macOS launchd label for the active instance (ADR-0044). The default instance yields
/// `org.sylin.ghostlight.service` (the PINNED label, PINS.md SS5.2); a named instance yields
/// `org.sylin.ghostlight.<n>.service`.
pub fn supervisor_label() -> String {
    crate::instance::Instance::resolve().supervisor_label()
}

/// Linux systemd --user unit for the active instance (ADR-0044). The default instance yields
/// `ghostlight.service` (the PINNED unit, PINS.md SS5.2); a named instance yields
/// `ghostlight-<n>.service`.
pub fn supervisor_unit() -> String {
    crate::instance::Instance::resolve().supervisor_unit()
}

/// Self-heal retry window (PINNED, PINS.md SS5.2): after asking the supervisor to start the
/// service, the adapter retries its dial for up to this long before giving up.
pub const SELF_HEAL_RETRY_WINDOW: Duration = Duration::from_secs(3);

/// Self-heal retry interval (PINNED, PINS.md SS5.2): how often the adapter retries its dial
/// within [`SELF_HEAL_RETRY_WINDOW`].
pub const SELF_HEAL_RETRY_INTERVAL: Duration = Duration::from_millis(200);

/// The pinned self-heal failure message (PINS.md SS5.2), logged verbatim when the retry window
/// elapses with the service still unreachable.
pub const SELF_HEAL_FAILURE_MESSAGE: &str = "the Ghostlight service is not running and could not be started automatically; start it with 'ghostlight service' (or reinstall to enable auto-start)";

/// The pure program+args to idempotently (re)start the registered supervisor unit for the current
/// platform (PINS.md SS5.2). `None` on a platform with no supervisor mechanism. NEVER executed by
/// this function -- see [`start_service`] -- so it stays unit-testable as a pure string.
#[cfg(windows)]
pub fn supervisor_start_command() -> Option<(String, Vec<String>)> {
    Some((
        "schtasks".to_string(),
        vec![
            "/run".to_string(),
            "/tn".to_string(),
            supervisor_task_name(),
        ],
    ))
}

/// See the Windows doc above; macOS variant (PINS.md SS5.2).
#[cfg(target_os = "macos")]
pub fn supervisor_start_command() -> Option<(String, Vec<String>)> {
    Some((
        "launchctl".to_string(),
        vec![
            "kickstart".to_string(),
            "-k".to_string(),
            format!("gui/{}/{}", unsafe { libc::getuid() }, supervisor_label()),
        ],
    ))
}

/// See the Windows doc above; Linux (non-macOS Unix) variant (PINS.md SS5.2).
#[cfg(all(unix, not(target_os = "macos")))]
pub fn supervisor_start_command() -> Option<(String, Vec<String>)> {
    Some((
        "systemctl".to_string(),
        vec!["--user".to_string(), "start".to_string(), supervisor_unit()],
    ))
}

/// Best-effort ask the OS supervisor to start the service (ADR-0030 Decision 8; PINS.md SS5.2):
/// spawn + wait, ignoring any failure -- this is a hint, not a guarantee; the adapter's own
/// bounded dial retry (`ipc::relay_adapter`), not this call, decides whether the service ever
/// came up. Asserts the ADAPTER role first (PINS.md SS8): a SERVICE must never trigger a service
/// start (that would mean the SoC boundary already failed elsewhere).
pub fn start_service() {
    role::assert_adapter_role("start_service");
    let Some((program, args)) = supervisor_start_command() else {
        tracing::debug!("no OS supervisor mechanism on this platform; nothing to start");
        return;
    };
    match std::process::Command::new(&program).args(&args).status() {
        Ok(status) if status.success() => {
            tracing::info!(
                program,
                "asked the OS supervisor to start the Ghostlight service"
            );
        }
        Ok(status) => tracing::debug!(
            program,
            code = ?status.code(),
            "OS supervisor start command exited non-zero (best-effort; ignored)"
        ),
        Err(e) => tracing::debug!(
            program,
            error = %e,
            "could not run the OS supervisor start command (best-effort; ignored)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SS5.2 pins the exact program+args for the current platform; NEVER executes the command
    /// (pure, string-only).
    #[test]
    fn supervisor_start_command_is_pinned_for_this_platform() {
        let (program, args) =
            supervisor_start_command().expect("a supervisor mechanism exists on every CI target");

        #[cfg(windows)]
        {
            assert_eq!(program, "schtasks");
            assert_eq!(
                args,
                vec![
                    "/run".to_string(),
                    "/tn".to_string(),
                    supervisor_task_name()
                ]
            );
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            assert_eq!(program, "systemctl");
            assert_eq!(
                args,
                vec!["--user".to_string(), "start".to_string(), supervisor_unit()]
            );
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(program, "launchctl");
            assert_eq!(args[0], "kickstart");
            assert_eq!(args[1], "-k");
            assert!(args[2].starts_with("gui/"));
            assert!(args[2].ends_with(supervisor_label().as_str()));
        }
    }

    #[test]
    fn self_heal_window_is_wider_than_its_own_retry_interval() {
        assert!(SELF_HEAL_RETRY_WINDOW > SELF_HEAL_RETRY_INTERVAL);
    }
}
