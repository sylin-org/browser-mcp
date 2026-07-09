# ADR-0051 Phase 1: run the test suite reliably on a Windows developer machine.
#
# The spawn-based integration tests build and launch the real ghostlight binaries. On a dev box two
# things otherwise make a local run flaky (neither happens in CI, which has no live service and a
# closed stdin):
#   1. A running `ghostlight service` and Chrome's respawned native host hold target\debug\*.exe, so
#      the incremental linker cannot replace them mid-build.
#   2. The real-stdio relay test (hub_identity::relay_adapter_*) inherits the interactive terminal's
#      stdin, which never signals EOF, so it hangs.
#
# This script removes both without disturbing a running dev session: it builds into an ISOLATED
# CARGO_TARGET_DIR the live service never touches, and closes stdin (via cmd's `< NUL`, which
# PowerShell cannot express directly) so the relay tests see EOF. Extra args pass through to
# `cargo test` (e.g. `-- --test-threads=1` for a serial run).
#
# The DURABLE fix is ADR-0051 Phase 4 (move the ~40 wiring tests in-process so `cargo test` rarely
# builds or spawns a service at all); this is the reliable runner for the genuinely end-to-end tests.
$ErrorActionPreference = 'Stop'
if (-not $env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR = Join-Path $env:TEMP 'ghostlight-e2e-target'
}
Write-Host "test-e2e: isolated CARGO_TARGET_DIR=$($env:CARGO_TARGET_DIR) (a live dev service will not lock it)"
$extra = if ($args) { ' ' + ($args -join ' ') } else { '' }
cmd /c "cargo test --locked --no-fail-fast --workspace$extra < NUL"
exit $LASTEXITCODE
