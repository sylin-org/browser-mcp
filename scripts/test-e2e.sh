#!/usr/bin/env bash
# ADR-0051 Phase 1: run the test suite reliably on a developer machine.
#
# The spawn-based integration tests build and launch the real `ghostlight` binaries. On a dev box
# two things otherwise make a local run flaky (neither happens in CI, which has no live service and
# a closed stdin):
#   1. A running `ghostlight service` and Chrome's respawned native host hold `target/debug/*.exe`,
#      so the incremental linker cannot replace them mid-build (Windows especially).
#   2. The real-stdio relay test (`hub_identity::relay_adapter_*`) inherits the interactive
#      terminal's stdin, which never signals EOF, so it hangs.
#
# This script removes both without disturbing a running dev session: it builds into an ISOLATED
# CARGO_TARGET_DIR the live service never touches, and closes stdin so the relay tests see EOF.
# Extra args pass through to `cargo test` (e.g. `-- --test-threads=1` if you want serial).
#
# The DURABLE fix is ADR-0051 Phase 4 (move the ~40 wiring tests in-process so `cargo test` rarely
# builds or spawns a service at all); this script is the reliable runner for the tests that stay
# genuinely end-to-end.
set -euo pipefail
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/ghostlight-e2e-target}"
echo "test-e2e: isolated CARGO_TARGET_DIR=$CARGO_TARGET_DIR (a live dev service will not lock it)"
cargo test --locked --no-fail-fast --workspace "$@" < /dev/null
