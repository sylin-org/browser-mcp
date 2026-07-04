# Maturity-1 ledger

Durable execution record for the m01-m06 batch. The Task log is append-only
(one entry per task, newest at the bottom); the RESUME HERE block below is
updated in place each task. Each task commits its own ledger changes as part of
that task's single commit.

## RESUME HERE

- Branch: `maturity-1` (created from `dev` tip, base commit
  f66fbf02ae4a3b54c8b9cf92a8f448519be0662a)
- Baseline: `cargo test` (via `CARGO_TARGET_DIR=target/it`, see deviation in
  m01 entry) = 475 passed, 0 failed
- Progress: m01, m02, m03, m04, m05 done
- NEXT TASK: m06 (docs/tasks/maturity-1/m06-headless-smoke.md)
- Authority: BOOTSTRAP.md, then the task prompt, then 00-design.md, then
  ADR-0026/0027
- Invariants: tree green and clean between tasks; no push; ASCII diff scan per
  task

## Task log

(Append one entry per completed task. Shape:)

### <task-id> <title> -- <date>
- Commit: (see this task's commit)
- Files touched: <list>
- Summary: <what landed, key decisions, any conservative choice made>
- Deviations from the prompt/design: <numbered, each with reasoning; "none" if none>
- Verification: <fmt/clippy/test status; test counts before -> after; the
  prompt's own verification command outcomes>
- Notes for the reviewer: <anything a human should double-check, or "none">

### m01 stage-4 ledger post-run correction -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: docs/tasks/stage-4/LEDGER.md, docs/tasks/maturity-1/LEDGER.md
- Summary: Appended the pinned POST-RUN CORRECTION block to the end of
  docs/tasks/stage-4/LEDGER.md, verbatim per m01's Required behavior, noting
  that the t-live-1 consolidated live pass (commit 44db1f3) has since run and
  passed, while still owed: g13-1 steps 4-5, g13-3's governed half, g15-1/g15-2,
  and macOS/Linux live checks. No existing byte of the file changed. Filled in
  the maturity-1 LEDGER.md RESUME HERE block (branch, base commit, baseline).
- Deviations from the prompt/design: 1. Three ghostlight.exe processes were
  running (target/debug/ghostlight.exe locked), so all `cargo test` runs in
  this batch use `CARGO_TARGET_DIR=target/it` per BOOTSTRAP ground rule 4
  rather than closing the running processes (one is this session's own
  connected MCP server). 2. The appended block was written via a small Python
  one-liner (not the Edit tool) to guarantee byte-exact CRLF line endings
  matching the rest of the file, since the repo has `core.autocrlf=true` and
  no .gitattributes.
- Verification: `rg -c "POST-RUN CORRECTION"` -> 1; `rg -c "44db1f3"` -> 1;
  `rg -c "PLAIN STATEMENT"` -> 1 (unchanged); `git diff` shows only appended
  lines (no existing line changed). ASCII diff scan on staged changes: empty
  (clean). Baseline `cargo test` (isolated target dir): 475 passed, 0 failed.
  Spot-run `cargo test --test hot_reload`: 1 passed (org_policy_hot_swap_end_to_end).
- Notes for the reviewer: none.

### m02 per-file SPDX license headers -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: 21 files under src/governance/ (LicenseRef-Ghostlight-Commercial
  header), 29 non-governance .rs files under src/, 15 .rs files under tests/
  (all except tests/tool_schema_fidelity.rs, untouched), 4 .js files directly
  under extension/ (service-worker.js, content.js, agent-visual-indicator.js,
  popup.js) -- all Apache-2.0 OR MIT header. 69 files total, 69 insertions,
  0 deletions, 0 other changes. Plus docs/tasks/maturity-1/LEDGER.md.
- Summary: Re-counted the in-scope file sets before editing (rule 7); counts
  matched 00-design.md/the prompt exactly (21 governance, 29+15=44 Apache/MIT
  engine files, 4 extension .js), so no recount deviation was needed. Inserted
  the pinned SPDX header as line 1 of every in-scope file via a small Python
  script that detects each file's existing newline convention (CRLF vs LF) and
  reuses it for the header line, so no file's line-ending style changed.
  tests/tool_schema_fidelity.rs was excluded from the file list from the start
  (never modified).
- Deviations from the prompt/design: none.
- Verification: `cargo fmt --check` clean; `cargo clippy --all-targets -- -D
  warnings` clean; `cargo test` -- 475 passed, 0 failed (unchanged from
  baseline, comment-only change). Pinned rg assertions: governance count 21,
  Apache/MIT count 44, extension count 4, tool_schema_fidelity.rs has no
  header (rg exit 1), every match is on line 1 (rg -v ":1:" empty). ASCII diff
  scan on staged changes: empty (clean).
- Notes for the reviewer: none.

### m03 CI workflows (three-OS matrix + release artifacts) -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: .github/workflows/ci.yml (new), .github/workflows/release.yml
  (new), docs/tasks/maturity-1/LEDGER.md.
- Summary: `.github/` did not exist (STOP precondition verified). Created
  ci.yml (fmt job on ubuntu; test job matrixed over ubuntu/macos/windows
  running clippy -D warnings then cargo test) and release.yml (tag-triggered
  v* release building --release for the four pinned targets, uploading
  artifacts), both transcribed byte-for-byte from the prompt's pinned YAML, no
  SPDX header per 00-design.md.
- Deviations from the prompt/design: none.
- Verification: pinned rg assertions all pass --
  `dtolnay/rust-toolchain@stable` count 2, `windows-latest` count 1,
  `cargo clippy --all-targets -- -D warnings` count 1 in ci.yml;
  the four-target alternation regex count 4 and `if-no-files-found: error`
  count 1 in release.yml. No tabs in either file (rg -P "\t" empty); 2-space
  indent confirmed visually. `cargo test` -- 475 passed, 0 failed (unchanged,
  no compiled change). ASCII diff scan on staged changes: empty (clean).
  YAML validity is NOT locally verified (no local GitHub Actions runner);
  it is confirmed live on the first push per the prompt.
- Notes for the reviewer: the two workflow files are unvalidated by GitHub's
  own YAML parser until the first push to a remote that runs Actions.

### m04 audit destinations syslog (RFC 5424/UDP) and none -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: src/governance/config/mod.rs, src/governance/audit/mod.rs,
  src/governance/audit/destinations.rs, tests/golden/config-schema.json,
  tests/golden/config-keys.md, README.md (one line), docs/tasks/maturity-1/LEDGER.md.
- Summary: Both STOP preconditions verified before editing. Added
  `AUDIT_SYSLOG_ADDRESS` const + KeyDef (KeyConstraint::None, Str default
  "127.0.0.1:514" in all three presets) directly after AUDIT_FILE_PATH;
  widened AUDIT_DESTINATION's EnumVariants to
  `["file", "stderr", "syslog", "none"]` and updated its doc comment; added
  the Config field, from_preset/from_resolution population, and
  audit_syslog_address() accessor mirroring audit_file_path() exactly. Added
  `destinations::send_line_to_syslog` (binds 0.0.0.0:0, one socket per call,
  formats `<134>1 {ts} - ghostlight {pid} - - {line}` with chrono
  to_rfc3339_opts(Millis, true)). `Inner` gained `Syslog(SocketAddr)`;
  `resolve_inner` gained "none" -> None and "syslog" -> ToSocketAddrs
  resolution (warn+None on failure), with the `_ =>` file fallback kept
  last and unchanged; `write_serialized` gained the Syslog arm
  (warn-and-swallow on send failure, same as the File arm). Updated
  `enum_key_parse_value` to the four-variant set, "syslog"/"none" now Ok,
  "smoke-signals" as the new invalid probe, and the updated pinned error
  message. Regenerated both goldens via the sanctioned Git Bash commands
  (`cargo run --quiet -- config schema`/`config docs`); hand-reviewed diffs
  matched exactly what 00-design.md predicted (enum gains two variants, one
  new key block). Made the single pinned README.md edit.
- Deviations from the prompt/design: 1. `cargo fmt` reformatted the
  `enum_key_parse_value` assertion (`assert_eq!(err.to_string(), "...")`)
  onto multiple lines after the string grew past rustfmt's line-width
  threshold; this is rustfmt's own formatting, not a hand edit, and
  `cargo fmt --check` is green. 2. All `cargo`/goldens commands in this task
  continue to use `CARGO_TARGET_DIR=target/it` per the ground-rule-4
  deviation recorded in m01 (ghostlight.exe processes still running).
- Verification: `cargo fmt --check` clean (after the rustfmt pass above);
  `cargo clippy --all-targets -- -D warnings` clean; `cargo test` -- 479
  passed, 0 failed (baseline 475 + 4 new named tests). The four named tests
  (`syslog_destination_sends_one_rfc5424_datagram_per_record`,
  `none_destination_discards_records_and_reports_disabled`,
  `invalid_syslog_address_disables_audit_with_a_warning`,
  `reload_switches_file_to_syslog`) all pass. `config_schema_golden` test
  binary: 5/5 pass (goldens byte-match). `rg -c "syslog"
  tests/golden/config-keys.md` = 3 (>= 2, satisfies the pinned assertion).
  `git status --short` showed exactly the six allowed files plus this
  ledger. ASCII diff scan on staged changes: empty (clean).
- Notes for the reviewer: none.

### m05 extract service-worker pure logic into extension/lib/ with node tests -- 2026-07-03
- Commit: (see this task's commit)
- Files touched: extension/lib/geometry.js (new), extension/lib/keys.js (new),
  extension/service-worker.js, tests/extension/geometry.test.js (new),
  tests/extension/keys.test.js (new), .github/workflows/ci.yml (extension-unit
  job appended), docs/tasks/maturity-1/LEDGER.md.
- Summary: All three STOP preconditions verified before editing (rescaleCoord
  present, no importScripts anywhere, extension/lib/ absent). Located every
  cited site by content (rule 7); all matched the prompt's pre-m02 line
  numbers +1 exactly, confirming no drift. Created geometry.js (PX_PER_TOKEN/
  MAX_TOKENS/MAX_SIDE, targetDims, zoomScale moved verbatim, plus new
  rescaleCtxCoord taking the context record directly) and keys.js (KEY_MAP,
  BUTTON_BITS, modifierBits, keyCode, VK_NAMED, VK_PUNCT, CODE_PUNCT, vkCode,
  SHIFT_BASE, charKeyInfo moved verbatim), both with the pinned dual-export
  footer. service-worker.js: added `importScripts("lib/geometry.js",
  "lib/keys.js");` as the first executable statement after the top comment
  block; replaced the combined const line with the GhostlightGeometry
  destructure plus a standalone `MAX_SCREENSHOT_B64`; deleted the moved
  targetDims/zoomScale bodies; replaced rescaleCoord's body with the thin
  `rescaleCtxCoord(screenshotCtx.get(tabId), x, y)` delegate at its original
  location; replaced the KEY_MAP/BUTTON_BITS/modifierBits block with the
  GhostlightKeys destructure (kept CLICK_GAP_MS, which is not an export, in
  place); deleted the moved keyCode/VK_NAMED/VK_PUNCT/CODE_PUNCT/vkCode/
  SHIFT_BASE/charKeyInfo bodies. Every call site (targetDims, zoomScale,
  rescaleCoord, modifierBits, keyCode, vkCode, charKeyInfo) is byte-identical
  to before. Wrote the two pinned node:test files with all 23 named
  assertions transcribed from the prompt (14 tests total: 9 geometry, 5
  keys), including the two control/non-ASCII probes as literal 6-character
  JS escape text (backslash-u-0-0-0-1 and backslash-u-0-0-e-9), never a
  literal byte, per ground rule 5. Appended the extension-unit CI job to
  ci.yml (m03 landed, so this ran normally, not blocked-by-m03).
- Deviations from the prompt/design: 1. The pinned verification command
  `node --test tests/extension/` (bare directory positional argument) fails
  on this machine's Node v24.7.0 with `Error: Cannot find module
  '...\tests\extension'` (MODULE_NOT_FOUND), reproduced identically in both
  Git Bash and PowerShell, and reproduced in a from-scratch, unrelated
  scratch directory with a single trivial test file -- confirming this is an
  environment/Node-version CLI quirk (passing an explicit directory as a
  positional arg to `--test` on this build), not a defect in the test files
  or the extracted code. Two equivalent invocations both pass all 14 tests:
  `node --test tests/extension/geometry.test.js tests/extension/keys.test.js`
  and bare `node --test` run with cwd set to tests/extension/. The appended
  CI job pins `node-version: "22"` (not 24), which may not hit this quirk at
  all; that is unverified locally (only Node 24 is installed here) and is
  left for the live CI run to confirm, consistent with 00-design.md's
  general acknowledgment that these workflows are validated live on first
  push. 2. `cargo test`/goldens commands continue using
  `CARGO_TARGET_DIR=target/it` per the ground-rule-4 deviation from m01.
- Verification: `node --test tests/extension/geometry.test.js
  tests/extension/keys.test.js` -- 14/14 pass (9 geometry + 5 keys, matching
  the prompt's "14 tests" pin). rg checks: `rg -c "function targetDims"
  extension/service-worker.js` -- no match (exit 1); `rg -c "function
  targetDims" extension/lib/geometry.js` -- 1; `rg -c "importScripts"
  extension/service-worker.js` -- 1; `rg -c "GhostlightKeys"
  extension/lib/keys.js` -- 3 (>= 2). `rg -n "[^\x00-\x7F]" extension/lib/
  tests/extension/` -- empty (pure ASCII). `cargo test` (isolated target
  dir) -- 479 passed, 0 failed, unchanged from m04 (no Rust files touched).
  `git status --short` showed exactly the six expected paths. ASCII diff
  scan on staged changes: empty (clean). OPTIONAL live-Chrome sanity
  (reload unpacked extension + live-demo.ps1) was NOT run; not required for
  acceptance per the prompt.
- Notes for the reviewer: the CI `extension-unit` job is unvalidated until
  the first push (consistent with m03's note); given deviation 1 above, if
  it fails on `ubuntu-latest`/`macos-latest`/`windows-latest` with a
  MODULE_NOT_FOUND on the `tests/extension/` arg specifically, that is this
  same Node CLI quirk (likely version- or platform-specific) and the fix is
  to invoke node --test with an explicit glob/file list rather than a bare
  directory arg; it is not evidence of a bug in the extracted code (the
  content itself is fully verified above).

## RUN SUMMARY

(Write after the last task: tasks landed vs BLOCKED, test counts baseline ->
final, deviations rolled up, anything left for a human.)
