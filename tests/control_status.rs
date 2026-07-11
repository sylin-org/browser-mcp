// SPDX-License-Identifier: Apache-2.0 OR MIT
//! CAP-MED-01: the control-plane `status` request over the ADAPTER/CONTROL endpoint. A real
//! spawned service answers `ghostlight_transport::ipc::query_status` with a liveness snapshot --
//! the mechanism `ghostlight doctor` uses to render a real extension verdict without `--debug`.

mod support;

use std::time::{Duration, Instant};

/// A freshly spawned service (no browser extension attached) answers the control `status` request
/// with `extension_connected == false` and zero live sessions. Polling `query_status` until it
/// answers also serves as the readiness wait for the adapter/control endpoint.
#[ignore = "e2e: spawns a real ghostlight service; run via the e2e tier -- cargo test -- --ignored"]
#[test]
fn control_status_reports_no_extension_on_a_fresh_service() {
    let endpoint = format!(
        "ghostlight-control-status-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let mut service = support::spawn_service(&endpoint);

    let deadline = Instant::now() + Duration::from_secs(15);
    let reply = loop {
        if let Some(r) = ghostlight_transport::ipc::query_status(&endpoint) {
            break r;
        }
        assert!(
            Instant::now() < deadline,
            "the control status request never answered"
        );
        std::thread::sleep(Duration::from_millis(100));
    };

    assert_eq!(reply.hub, ghostlight_transport::handshake::HUB_PROTO);
    assert!(
        !reply.extension_connected,
        "no browser extension is attached in this test, so the service must report disconnected"
    );
    assert_eq!(
        reply.live_sessions, 0,
        "no tool sessions are live in this test"
    );

    let _ = service.kill();
    let _ = service.wait();
}
