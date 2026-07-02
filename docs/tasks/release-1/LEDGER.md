# Release-1 execution ledger

This file is the working memory of the unattended run defined in BOOTSTRAP.md.
The agent updates it before and after every task and commits it with each
task's changes. Humans read it to understand exactly what happened.

## RUN SUMMARY

(Written by the agent at the end of the run. Empty until then.)

## RESUME HERE

- Current task: T13 (next pending). T04, T06, T07, T01, T02, T03, T12 are done.
- Branch: release-1-hardening (create from main if absent).
- Last commit: feat(extension): T12 per-domain console/network buffer reset
  (this run)
- Open concerns: pre-existing `cargo fmt` drift (unrelated to T04/T06/T07/T01/T02/T03/T12) in
  `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs` -- both reformat under the installed
  rustfmt 1.9.0 but were left untouched again because they are out of scope / forbidden. A
  whole-repo `cargo fmt --check` will report these two files; this has no bearing on T12 (which
  touched no Rust files at all -- `git status --short -- '*.rs' src/ tests/` was empty before
  committing). A human may want to run `cargo fmt` repo-wide in its own dedicated commit at some
  point; do not fold that into an unrelated task's commit.

## Sequence and status

Order: T04, T06, T07, T01, T02, T03, T12, T13, T14, T15, T08, T09, T10, T11, T18, T16, T17, T05.

| # | Task | Title | Depends on | Status |
|---|------|-------|-----------|--------|
| 1 | T04 | Extension-channel warmup + bounded first-call wait | - | done |
| 2 | T06 | Hop-attributed error reporting | T04 (binary half) | done |
| 3 | T07 | Extend installer doctor with runtime/debug-state fusion | - | done |
| 4 | T01 | read_page structural pagination + caps | - | done |
| 5 | T02 | read_page viewport culling (filter=interactive) | - | done |
| 6 | T03 | get_page_text official semantics | - | done |
| 7 | T12 | Per-domain console/network buffer reset | - | done |
| 8 | T13 | Runtime.exceptionThrown capture | - | pending |
| 9 | T14 | Network.loadingFailed status | - | pending |
| 10 | T15 | Empty-result guidance notes | - | pending |
| 11 | T08 | type via real keyDown/keyUp | - | pending |
| 12 | T09 | Mouse click fidelity (clickCount sequence, buttons, force) | - | pending |
| 13 | T10 | Scroll verify + scrollable-ancestor fallback | - | pending |
| 14 | T11 | Real zoom region crop + coordinate-context update | - | pending |
| 15 | T18 | Background-tab screenshot via clip+scale | T11 helpful, not required | pending |
| 16 | T16 | javascript_tool REPL semantics + 50KB cap | - | pending |
| 17 | T17 | Effective-tabId fallback + valid-ID errors | - | pending |
| 18 | T05 | Service-worker state recovery (runs LAST) | after all service-worker tasks | pending |

Status values: pending, in_progress, done, blocked (with reason in the log).

## Task log

Append one entry per task using this template. Newest at the bottom.

```
### T<NN> <title> -- <done|blocked> -- <timestamp>
- Commit: <hash or n/a>
- Files touched:
- Tests added:
- Drift reconciled: (prompt facts that no longer matched the code, and what was actually true)
- Decisions made: (conservative choices taken without a human, and why)
- Notes for later tasks:
- Browser checks queued: (section ids added to BROWSER-TESTS.md, or none)
```

### T04 Extension-channel warmup + bounded first-call wait -- done -- 2026-07-02
- Commit: (recorded after commit; see git log for `feat(mcp): T04 ...`)
- Files touched: src/browser.rs, src/mcp/server.rs, tests/mcp_protocol.rs,
  docs/tasks/release-1/BROWSER-TESTS.md, docs/tasks/release-1/LEDGER.md
- Tests added:
  - src/browser.rs: `wait_connected_times_out_without_a_connection`,
    `wait_connected_wakes_when_the_extension_attaches`
  - tests/mcp_protocol.rs: `tools_call_waits_for_a_late_extension_and_notes_the_wait` (new);
    updated `initialize_tools_list_and_tool_call_over_stdio` to assert the exact bounded-timeout
    message instead of a substring match
- Drift reconciled: none of consequence. The prompt's line-number references had already drifted
  slightly from the working tree (e.g. exact line numbers for `is_connected`/`attach` moved by a
  few lines versus the prompt's "lines 64-66" etc.), but every function name, doc comment, and
  code shape the prompt described was present and matched; all snippets in the prompt were used
  essentially verbatim.
- Decisions made:
  - Left `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs` untouched even though a
    repo-wide `cargo fmt` reformatted both (pre-existing rustfmt drift unrelated to this task,
    likely a rustfmt-version difference from whenever those files were last formatted). Reverted
    those two files with `git checkout --` after running `cargo fmt`, and verified fmt cleanliness
    on only the files this task actually touched via `rustfmt --check src/browser.rs
    src/mcp/server.rs tests/mcp_protocol.rs` (clean). `tests/tool_schema_fidelity.rs` was run
    unchanged via `cargo test` and still passes (6/6). See "Open concerns" above; flagging for the
    run summary / a human to decide on a dedicated repo-wide fmt pass later.
  - In `handle_tools_call`'s error arm, `append_wait_note` is called after the `isError` insertion
    (order between the two does not matter per the prompt; only that the note is the last content
    block, which it is since `isError` is a sibling key, not a content entry).
  - The new integration test's fake extension sleeps 1000ms (well inside the 5000ms
    `FIRST_CALL_WAIT_MS` window) before connecting, matching the prompt's spec exactly.
- Notes for later tasks:
  - T06 (hop-attributed error reporting, binary half) touches error text in the same call path
    (`handle_tools_call` / `Browser::call`); the bounded-wait timeout message and the
    `(waited N.Ns ...)` note are new text surfaces introduced here -- do not clobber their exact
    wording (tests assert on exact strings).
  - `run` in src/mcp/server.rs is now concurrent: `tools/call` responses arrive via a single
    writer task fed by an mpsc channel, and out-of-order arrival relative to other in-flight
    `tools/call`s is expected and correct (correlated by JSON-RPC id). Any future change to the
    read loop must keep funneling all stdout writes through that one writer task (constraint 13
    in this prompt).
  - Pre-existing `cargo fmt` drift in `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs`
    remains unfixed (see Open concerns above); a future task that legitimately edits either file
    will likely have its own diff intermixed with this reformatting the moment it runs `cargo
    fmt` -- reconcile deliberately (keep only the lines relevant to that task's own change, or do
    a clean dedicated repo-wide fmt commit first) rather than accepting the reformat silently.
- Browser checks queued: T04-1, T04-2, T04-3 in docs/tasks/release-1/BROWSER-TESTS.md.

### T06 Hop-attributed error reporting across the full dispatch path -- done -- 2026-07-02
- Commit: (recorded after commit; see git log for `feat(mcp): T06 ...`)
- Files touched: src/error.rs, src/lib.rs, src/browser.rs, src/mcp/server.rs, src/mcp/tools.rs,
  src/native/messages.rs, extension/service-worker.js, tests/mcp_protocol.rs,
  docs/tasks/release-1/BROWSER-TESTS.md, docs/tasks/release-1/LEDGER.md
- Tests added:
  - src/error.rs (`tool_error_tests` module): one Display test per variant (extension, invalid-
    request, binary, ipc, cdp, page) checking the exact `[hop: ...] ... Next step: ...` text;
    `from_extension_wire` mapping tests for `Some("cdp")`, `Some("page")`, `None`, and an unknown
    hop string; a `next_step(...)` override test.
  - src/browser.rs: updated `call_surfaces_a_tool_error` and `call_without_a_connection_fails_fast`
    to assert the `[hop: extension]` prefix (in addition to their prior substring checks); added
    `call_surfaces_a_cdp_tagged_tool_error_without_leaking_detail` (asserts `[hop: cdp]`, the CDP
    method text, and that `detail` never appears in the rendered message) and
    `call_surfaces_a_page_tagged_tool_error` (asserts `[hop: page]`).
  - src/mcp/tools.rs: `is_known_tool_recognizes_advertised_names`,
    `is_known_tool_rejects_unknown_names`.
  - tests/mcp_protocol.rs: strengthened the no-extension assertion in
    `initialize_tools_list_and_tool_call_over_stdio` to check the exact new hop-attributed text
    (superseding the old "after 5s..." wording); added `unknown_tool_name_is_rejected_before_dispatch`
    (no extension connected, sends `tools/call` for `bogus_tool`, asserts `[hop: invalid-request]`
    + "Unknown tool: bogus_tool", and asserts the round trip took well under the 5s extension-wait
    window, proving the pre-check runs before the wait/dispatch).
- Drift reconciled:
  - The prompt's "Current behavior" section describes `Browser::call` and `attach` as they existed
    BEFORE T04 landed (e.g. it does not mention the bounded first-call wait T04 added in
    `handle_tools_call`, or the concurrent per-call spawn/writer-task architecture). All the error-
    mapping sites the prompt names (`Browser::call`'s four failure arms, the `attach` read loop,
    `route_reply`) matched the actual code exactly aside from this omission; every function name,
    line-content shape, and message string cited was present and correct once cross-referenced
    against the real file.
  - The prompt's own Verification step 3 ("Chrome closed, call any tool" -> exactly
    "[hop: extension] Browser extension not connected. Next step: ...") and its Tests section
    ("strengthen the existing no-extension assertion ... starts with [hop: extension]") only make
    sense if the T04 bounded-wait timeout branch in `handle_tools_call` (which the prompt's Current
    Behavior section never mentions) is ALSO folded into the new hop-error contract, not left as
    its bespoke "Browser extension not connected after {}s. Check that Chrome is running ..."
    text. Reconciled by removing that bespoke early-return entirely: when the bounded wait times
    out, `waited` simply stays `None` and control falls through to `Browser::call`, which (being
    genuinely unconnected) fails fast with the canonical `ToolError::extension("Browser extension
    not connected")` -- one hop-attributed message to maintain, not two. This exactly produces the
    prompt's canonical example string and needed no separate formatting logic in
    `handle_tools_call`. No extra latency: the fallthrough call fails immediately (`sent` is
    `false`), it does not wait a second bounded window.
- Decisions made:
  - `ToolError` derives `Clone` (prompt did not say either way). Needed so `attach`'s read-loop-end
    drain can fan the same error out to every pending caller without hand-rolling a clone helper
    (thiserror variants here are plain owned `String` fields, so `Clone` is free and does not
    change `Display`/`Error` semantics). Rejected alternative: re-render `.to_string()` and wrap in
    a fresh `ToolError::ipc(...)` per pending caller -- discarded because that double-wraps the
    `[hop: ...] ... Next step: ...` text (the constraint says the hop/message/next-step strings are
    an exact contract; nesting them would violate it).
  - `attach`'s reader loop distinguishes `Ok(None)` vs `Err(e)` by `break`-ing a `let drain_err = ...`
    value out of the loop and running ONE shared post-loop cleanup+drain block, rather than
    duplicating the `outgoing = None` / `set_connected(false)` / `connected.send_replace(false)` /
    `writer.abort()` bookkeeping in both arms (the prompt's own pseudocode showed them as separate
    inline blocks; consolidated for DRY-ness per this repo's coding-style rule, with identical
    observable behavior).
  - `handle_tools_call`'s new unknown-tool pre-check and the pre-existing error arm now share one
    `error_result(ToolError) -> Value` helper (builds the `{content, isError:true}` shape from
    `err.to_string()`) instead of duplicating the `text_content` + `isError` insertion inline
    twice; not mentioned by the prompt but a direct, low-risk simplification.
  - `src/native/messages.rs` and the module doc at the top of `src/browser.rs` were both updated to
    document the new optional `hop`/`detail` wire fields (the prompt only explicitly required the
    `src/native/messages.rs` change in step 7, but leaving `browser.rs`'s own doc comment
    describing the old error-only wire shape would have made the two docs disagree).
  - Content-script error message trimming in `form_input` (drop exactly one trailing period before
    handing the message to `hopError`, per the prompt) was implemented as
    `msg.endsWith(".") ? msg.slice(0, -1) : msg`; verified against content.js's actual
    `setFormValue` error text ("Element ref_N not found or was garbage-collected.") which does end
    in a period, so this path is exercised by the documented example.
  - Left `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs` untouched again (same pre-
    existing rustfmt-version drift noted by T04); reverted both with `git checkout --` after
    `cargo fmt` reformatted them as a side effect of formatting the crate root. Verified fmt
    cleanliness on exactly the files this task touched with
    `rustfmt --check --edition 2021 src/browser.rs src/error.rs src/mcp/server.rs src/mcp/tools.rs
    src/native/messages.rs tests/mcp_protocol.rs` (clean; `--edition 2021` is required when
    invoking `rustfmt` directly on individual files, otherwise it defaults to the 2015 edition and
    fails to parse `async fn`). `src/lib.rs` was excluded from that direct-file check because
    passing a crate root (a file with `pub mod ...` declarations) to standalone `rustfmt` makes it
    recurse into every reachable module -- including the two drifted files -- so `src/lib.rs`'s
    one-line diff was instead verified by inspection (`git diff -- src/lib.rs`).
- Notes for later tasks:
  - The dispatch order in `handle_tools_call` is now: extract name/args -> unknown-tool pre-check
    (`ToolError::invalid_request`, no extension wait) -> `dispatch::policy_check`/`dispatch::audit`
    -> bounded extension-channel wait (falls through to `Browser::call` on timeout, no separate
    message) -> `Browser::call`. Any future change to this function must preserve that the unknown-
    tool check runs before ANY extension-channel interaction (a later task's own test asserts this
    via elapsed-time).
  - The `[hop: <hop>] <message>. Next step: <next step>.` format, the six hop names, and every
    default next-step string are now a byte-exact contract asserted by multiple tests in
    src/error.rs, src/browser.rs, and tests/mcp_protocol.rs. Do not casually reword any of them.
  - `extension/service-worker.js` now has a `hopError(hop, message, detail)` helper (near `fail`);
    any NEW content-script-backed or CDP-backed failure site added by a later task should use it
    (`"cdp"` for `chrome.debugger.*` failures, `"page"` for content-script/DOM failures) rather than
    throwing a plain `Error`, to keep failures hop-attributed. Untagged throws still work (they
    fall back to the `extension` hop via `dispatch`'s catch), but lose the more specific
    attribution.
  - `extension/content.js` was NOT touched (out of scope per the prompt); its own error message
    text (e.g. the `setFormValue` "not found or was garbage-collected." string) is now surfaced
    verbatim (minus one trailing period) as the `[hop: page]` message text for `form_input`. If a
    later task changes content.js's error strings, the trailing-period-trim behavior in
    `form_input`'s handler in service-worker.js should be re-checked against the new text.
  - Four new/updated BROWSER-TESTS.md entries (T06-1..T06-4) depend on a live browser; T04-2's
    expected text was also corrected in place (it documented the now-superseded "after 5s ..."
    wording) rather than left stale, since a human running the checklist top-to-bottom would
    otherwise hit a real mismatch there.
- Browser checks queued: T06-1, T06-2, T06-3, T06-4 in docs/tasks/release-1/BROWSER-TESTS.md
  (T04-2's expected text was also updated in place; see above).

### T07 Doctor subcommand fusing debug state into one diagnosis -- done -- 2026-07-02
- Commit: (recorded after commit; see git log for `feat(cli): T07 ...`)
- Files touched: src/debug.rs, src/mcp/server.rs, src/main.rs, src/native/ipc.rs,
  src/install/mod.rs, src/lib.rs, src/doctor.rs (new), docs/tasks/release-1/BROWSER-TESTS.md,
  docs/tasks/release-1/LEDGER.md
- Tests added:
  - src/debug.rs: updated both existing `DebugSink::enabled(&dir)` calls to
    `enabled(&dir, "mcp-server")`; added `enabled_sink_records_role_and_client` (asserts
    `snap["role"] == "mcp-server"` and `snap["client"] == "claude-code 1.2.3"` after `set_client`
    + `flush`).
  - src/native/ipc.rs: `probe_reports_absent_for_an_unused_endpoint` (plain `#[test]`, pid-unique
    endpoint), `probe_reports_accepts_against_a_live_server` (`#[tokio::test]`, spawns `serve`,
    polls `probe_endpoint` via `spawn_blocking` until `Accepts` or a 5s deadline).
  - src/doctor.rs (new, `#[cfg(test)] mod tests`): `all_healthy_observations_produce_no_findings`,
    `unregistered_browser_and_client_each_produce_their_own_finding`,
    `absent_with_no_sessions_fires_exactly_rules_3_and_7_in_order`,
    `rejects_embeds_a_known_pid_and_falls_back_to_process_manager_without_one`,
    `accepts_with_no_server_session_fires_rule_5`,
    `accepts_with_a_disconnected_extension_distinguishes_never_connected_from_dropped`,
    `parse_session_extracts_full_new_format_fields`,
    `parse_session_defaults_role_and_client_for_old_format_files`,
    `parse_session_returns_none_for_garbage_or_a_missing_pid` -- all 9 cover every case the
    prompt's Verification/unit-test list named.
- Drift reconciled: none of consequence. Every function/struct/line-content the prompt named in
  "Current behavior" (src/main.rs's `DoctorArgs`/role dispatch/`build_debug_sink`, src/debug.rs's
  private helpers and `Snapshot`/`Inner`, src/install/mod.rs's old `run_doctor`/`DoctorOptions`,
  src/native/ipc.rs's `serve`/`connect`/`socket_path`/`pipe_path`, src/mcp/server.rs's
  `initialize` arm) matched the working tree exactly; only exact line numbers had drifted by a
  few lines from T04/T06 landing first, as the prompt itself warned they would.
- Decisions made:
  - `status_report()`'s old "debug state at <path> is unreadable" failure text is retired (folded
    into the new "no mcp-server debug state under <dir> (state files exist for other roles or are
    unreadable)" message when no file both parses AND has an mcp-server-or-absent role). The
    prompt's Part A.7 names exactly two new failure texts and says "everything else... keeps the
    existing messages" -- read as: the old two-branch "is unreadable" message (there were two
    identical `return format!(...)` arms for read-failure vs parse-failure) is not one of the
    messages being kept, since the prompt's replacement logic no longer distinguishes "newest file
    unreadable" from "no candidate at all" -- both simply produce no candidate. Grepped the repo
    first to confirm no test asserts on the old "is unreadable" string; none does.
  - The Debug-sessions row cap ("show at most 6 session rows... if more were parsed, `(and <n>
    older...)`") is ambiguous about whether "rows" includes "(skipping unreadable state file: ...)"
    lines in the cap-of-6 count. Implemented: the cap of 6 (non-verbose) applies to the *first 6
    files in the newest-first list* (parsed or unreadable, one row each), and the trailing "and <n>
    older" note counts only *additional successfully-parsed sessions* beyond what was shown (i.e.
    total-parsed-across-all-files minus parsed-shown-within-the-cap) -- so a run of unreadable
    files near the cap boundary can silently drop a couple of skip-lines without a trailing note,
    but a real session is never silently dropped without being counted in "older". This is not
    unit-tested (the prompt's own unit-test list only requires `findings` and `parse_session`
    coverage, not row-cap rendering) -- flagging for a human/future task if stricter behavior is
    wanted. The "extension last seen" line always scans the FULL parsed list (not just the shown,
    possibly-capped rows), by design, so it never goes stale under the cap.
  - `EndpointProbe`'s doc comment adds "(see [`probe_endpoint`])" to the prompt's literal text;
    this is elaboration only (not one of the byte-exact-contract strings like `ToolError`'s
    `Display` text), so it is not a deviation from any tested/asserted string.
  - `browser-mcp doctor`'s Verdict "no debug instrumentation found" (rule 7) and the `Absent`/
    `Rejects` rules (3/4) are independent findings per the prompt's own text ("fires in addition to
    rule 3 or 4"); implemented as unconditional pushes in sequence, not an `else`, matching that
    literally -- verified by `absent_with_no_sessions_fires_exactly_rules_3_and_7_in_order`.
  - Left `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs` untouched again (same pre-
    existing rustfmt-version drift T04/T06 flagged); reverted both with `git checkout --` after
    `cargo fmt` reformatted them as a side effect of formatting the crate root. Verified fmt
    cleanliness on exactly the files this task touched with `rustfmt --check --edition 2021
    src/debug.rs src/install/mod.rs src/main.rs src/mcp/server.rs src/native/ipc.rs src/doctor.rs`
    (clean); `src/lib.rs`'s one-line diff (`pub mod doctor;`) was verified by inspection instead
    (same crate-root caveat as T04/T06).
- Notes for later tasks:
  - `DebugSink::enabled` now takes `(dir: &Path, role: &'static str)`, not just `(dir: &Path)`.
    `DebugSink::set_client(&self, client: &str)` and `DebugSink::ipc_note(&self, summary: &str)`
    are new public methods (both force a snapshot write). `frame_in`/`frame_out` now also refresh
    `updated_ms` (throttled via the new private `Inner::touch`), so a session that is only relaying
    frames (no MCP requests) no longer looks stale in `status`/`doctor`.
  - `Snapshot`/state-file JSON gained two additive fields: `role` (always present, "mcp-server" or
    "native-host") and `client` (present only after `set_client` was called; omitted via
    `skip_serializing_if` otherwise). Any later task reading `debug-state-*.json` by hand (tests,
    tooling) should tolerate both fields being absent (old-format files) as well as present.
  - `crate::debug::{now_ms, fmt_ms, session_state_files}` are now `pub(crate)` (were private) --
    available to any future in-crate module, not just `doctor`.
  - `browser_mcp::install::run_doctor` and `browser_mcp::install::DoctorOptions` are GONE (moved to
    `browser_mcp::doctor::run` / `browser_mcp::doctor::DoctorOptions`). `browser_mcp::install::
    host_file_path` is now `pub(crate)` (was private) so `doctor` can reuse it; `yesno` was deleted
    from `install::mod` (only caller was the removed `run_doctor`) -- `doctor.rs` has its own
    private `yn` helper, not shared.
  - `native::ipc::relay_native_host` signature changed: `(endpoint: &str)` ->
    `(endpoint: &str, debug: &crate::debug::DebugSink)`. Any future caller (there is currently only
    `main::run_native_host_role`) must pass a sink (use `DebugSink::disabled()` if none is wanted).
    New public `native::ipc::{EndpointProbe, probe_endpoint, endpoint_display}` (per-platform
    `#[cfg(windows)]`/`#[cfg(unix)]` implementations, like `serve`/`connect`) are synchronous (no
    tokio) and safe to call from `doctor`'s non-async context.
  - `main::run_native_host_role` now takes `(debug: bool)` and `main::build_debug_sink` now takes
    `(debug: bool, role: &'static str)`. The native-host role's debug sink is genuinely env-gated:
    Chrome inherits its own launch environment and never passes `--debug` to the process it spawns,
    so a native-host `debug-state-<pid>.json` only appears when Chrome ITSELF was started with
    `BROWSER_MCP_DEBUG=1` in its environment -- doctor's rule set intentionally never treats a
    missing native-host row as a problem by itself (see `doctor::findings`, which has no rule keyed
    on native-host presence at all).
  - `browser-mcp doctor`'s exit code is now truthful (0 = healthy/no findings, 1 = at least one
    problem line), a behavior change from before (old `run_doctor` always returned `Ok(())` ->
    exit 0 unconditionally). Any script that shells out to `browser-mcp doctor` and previously
    ignored its exit code should be aware it can now be 1.
  - Six new BROWSER-TESTS.md entries (T07-1..T07-6) depend on a live browser + a real MCP client
    session; while inserting them, also moved the pre-existing T04-3 entry (which a prior run had
    left stranded after the T06 block, out of task order) to sit directly after T06-4 and before
    the new T07 entries, restoring "in task order" top-to-bottom without altering T04-3's content.
- Browser checks queued: T07-1, T07-2, T07-3, T07-4, T07-5, T07-6 in
  docs/tasks/release-1/BROWSER-TESTS.md.

### T01 read_page structural pagination with element and char caps -- done -- 2026-07-02
- Commit: (recorded after commit; see git log for `feat(extension): T01 ...`)
- Files touched: extension/content.js, docs/tasks/release-1/BROWSER-TESTS.md,
  docs/tasks/release-1/LEDGER.md
- Tests added: none in the Rust sense (this task touches only extension JS, which has no test
  harness per project constraints). Verification performed instead:
  - `node --check extension/content.js` (syntax only).
  - A standalone throwaway Node script (not committed) that mirrored the pass-1/pass-2
    measure/emit algorithm in isolation (synthetic records with controlled `chars`/`show`
    values, not the real DOM helpers) to exercise: (a) everything-fits producing no markers,
    (b) a deep subtree overflowing and collapsing behind a marker while a LATER SIBLING at the
    same level still gets emitted (the breadth-over-depth property), (c) a subtree so large that
    even its own collapse marker does not fit, correctly halting the whole emit pass with no
    partial/gap output. All three matched the spec's described behavior exactly.
  - Full-file diff review confirming: (1) the diff is scoped entirely to lines inside
    `accessibilityTree` (verified via `git diff` hunk headers -- only three hunks, all within
    the function, nothing touched before or after it); (2) the per-line construction code (the
    element-line and select-option-line builders) was moved into pass 1 character-for-character
    unchanged, only the `add(...)` calls were replaced with direct string concatenation into
    `unit`; (3) the literal string `"... (truncated)"` no longer appears anywhere in the file
    (`grep -n "truncated"` returns nothing); (4) `cargo test` (all 91 tests across the workspace,
    including `tests/tool_schema_fidelity.rs`) passes unchanged, confirming no Rust surface was
    touched.
  - `git status --short -- '*.rs' src/ tests/` was empty throughout -- this task made zero Rust
    changes, exactly as the prompt's "Project context" predicted ("no Rust rebuild is required").
- Drift reconciled: none. Every line number, function name, and code shape the prompt's "Current
  behavior" section cited (accessibilityTree at lines 119-192, the `add` helper at 126-135, `walk`
  at 136-183, the ref_id re-rooting at 184-189, the service-worker forwarding at its cited lines)
  matched the actual working tree exactly -- this prompt's line numbers had not drifted at all
  from T04/T06/T07 (none of those touched extension/content.js).
- Decisions made:
  - Added an explicit `show` boolean field to each pass-1 record (not named in the prompt's field
    list: unit, ref, indent, children, unitChars, subtreeChars, elements) so pass 2 can branch on
    "is this record shown" without relying on `ref !== null` as an implicit proxy. This is an
    additive, non-observable implementation detail (does not change output), added for
    readability/robustness; every field the prompt DID require is present with the exact
    described semantics.
  - Kept the `collapsed` boolean flag in pass 2 even though no trailing-line decision reads it
    directly (only `capped` and `omitted > 0` gate the two trailing lines, per the prompt's own
    closing note "collapsed or stopped each imply omitted > 0, so this one condition covers every
    degraded outcome"). The prompt's Pass 2 preamble explicitly lists `collapsed` as required
    mutable state, so it is tracked for spec fidelity even though it is presently
    write-only; a future task could read it without restructuring the function.
  - `measure`'s guard-failure return value is `null` (a sentinel meaning "this node and its whole
    subtree do not exist in the render tree"), matching the original `walk`'s early-`return`
    semantics exactly: guard failure (depth exceeded, non-element node, `browser-mcp-` id,
    script/style/noscript/template tag, or the `filter==="interactive"` prune) skips the node AND
    everything under it, never just suppresses its own line. This was verified against the
    original code's control flow before writing pass 1, not assumed.
  - Did not special-case `<select>` records in pass 2 (no `if (tag === "select") ...` branch
    anywhere in `emit`). The "a select can never emit a marker, only stop" behavior the prompt
    describes falls out of the general algorithm automatically: a childless record (select's
    `children` is always `[]`, per the leaf rule preserved from pass 1) has
    `subtreeChars === unitChars`, so whenever rule 4 (does-not-fit) is reached for it,
    `unitChars` alone already exceeds `remaining`, which makes `unitChars + markerLine.length`
    exceed `remaining` too -- the marker-fits branch is therefore unreachable for any childless
    record, select or otherwise, without needing a dedicated check. Verified by direct algebraic
    reasoning (documented in the session) rather than assumed.
- Notes for later tasks:
  - T02 (viewport culling, filter=interactive) touches the SAME function
    (`accessibilityTree`/`measure` in extension/content.js) next. The `show` computation this task
    preserved verbatim is exactly what T02 will extend with position-in-viewport logic; do not
    reintroduce the old serialize-as-you-walk shape when adding that -- extend the `measure`
    function's guard/show logic in place, keep the pass-1/pass-2 split intact.
  - The three new literal line formats introduced here (`[subtree collapsed: ... to expand]`,
    `[element cap reached: ...]`, `[showing M of T elements; ...]`) are now a byte-exact contract
    of this file, same tier as T06's `[hop: ...]` contract in the Rust side -- do not reword them
    in a later task without updating this note and the T01 BROWSER-TESTS.md entries.
  - `MAX_ELEMENTS = 10000` is declared as a local `const` inside `accessibilityTree`, not at
    module scope -- there was no existing module-scope constant section in this file to join, and
    the prompt allowed either placement ("Declare it as a const at the top of accessibilityTree
    (or module scope next to the function)").
  - No `src/mcp/schemas/tools.json` edits were made or needed; `tests/tool_schema_fidelity.rs`
    passed unchanged (6/6), confirming the frozen `read_page` schema and its description (which
    still describes the now-superseded error-on-overflow behavior, deliberately -- see the
    prompt's Out of scope section) were left untouched.
- Browser checks queued: T01-1, T01-2, T01-3, T01-4, T01-5, T01-6 in
  docs/tasks/release-1/BROWSER-TESTS.md (appended after T07-6, preserving task order).

### T02 read_page viewport culling for filter=interactive -- done -- 2026-07-02
- Commit: (recorded after commit; see git log for `feat(extension): T02 ...`)
- Files touched: extension/content.js, docs/tasks/release-1/BROWSER-TESTS.md,
  docs/tasks/release-1/LEDGER.md
- Tests added: none in the Rust sense (extension JS has no test harness per project constraints).
  Verification performed instead:
  - `node --check extension/content.js` (syntax only).
  - Full re-read of the final `accessibilityTree`/`measure` function against every constraint in
    the prompt's Verification step 2: `intersectsViewport` exists with the exact strict-inequality
    formula given; `culled` is set only via `if (wouldShow && !show) culled = true;` (the
    wouldShow-but-not-shown case, and no other); the note string
    "Note: interactive results are limited to the current viewport; scroll or use filter=all for
    the full document." matches the contract character for character; line 152's early return
    (`if (filter === "interactive" && !isInteractive && !isContainer) return null;`) is byte-
    identical to before this task; `visible()` (lines 97-101) is untouched; the file is pure ASCII
    (confirmed by the BOOTSTRAP.md ASCII-scan command, empty output).
  - Traced the short-circuit algebra by hand: `show = wouldShow && (filter === "all" ||
    intersectsViewport(el))` -- when `filter === "all"`, the right operand short-circuits to `true`
    without evaluating `intersectsViewport`, so `show === wouldShow` always and `culled` can never
    become true for `filter=all` (satisfies "filter=all byte-identical, zero new
    getBoundingClientRect calls"). When `wouldShow` is `false` (excluded by role/name, interactive-
    ness, or `visible()`), the left operand of the outer `&&` is `false`, so JS never evaluates the
    right operand either -- `intersectsViewport` is only ever called for elements that would
    otherwise be shown, and `culled` is never set for any other exclusion reason.
  - `cargo test` (all 91 tests across the workspace, including `tests/tool_schema_fidelity.rs`,
    6/6) passes unchanged, confirming no Rust surface was touched.
  - `git status --short -- '*.rs' src/ tests/` was empty throughout -- this task made zero Rust
    changes, matching the prompt's "Project context" prediction ("no Rust rebuild is required").
  - `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` reports only the same
    two pre-existing drifted files noted by every prior task (see Decisions made below), neither of
    which this task touched.
- Drift reconciled: the prompt's entire "Current behavior" section describes the PRE-T01
  single-pass `walk()` function (a single `show` computation at "line 147", a `truncated` flag next
  to an `out` accumulator, `add(s)` for the character budget). T01 (which runs earlier in the fixed
  sequence and landed first) rewrote `accessibilityTree` into a two-pass `measure`/`emit` design
  with no `walk()` and no `add()`; `truncated` no longer exists (replaced by `capped`/`stopped`/
  `collapsed`/`omitted` in the pass-2 `emit` closure). Reconciled by mapping every required change
  onto its structural analog in the new code: (1) the exact `show` formula the prompt describes
  (`((filter === "all" && (r || n)) || (filter === "interactive" && isInteractive)) && isVisible`)
  is verbatim present inside `measure()` (pass 1) at the equivalent point in the walk -- this is
  where the culling logic was applied, unchanged from the prompt's literal formula. (2) `let culled
  = false;` was declared at the top of `accessibilityTree`, immediately after `const MAX_ELEMENTS =
  10000;` (not "next to `truncated`", which no longer exists) -- this is the earliest point in the
  new structure where `measure()` (defined and first invoked several lines later) can close over
  it; pass 2's own flags (`collapsed`/`stopped`/`capped`) are declared later, after pass 1 already
  ran, so `culled` could not sit next to them and still be visible to `measure()`. (3) the note-
  append logic was applied to the actual final `return` statement (now building `let result = out +
  ... ; if (culled) { result += ... } return result;`), which is the exact same statement the
  prompt calls "the return statement (currently line 191)" -- T01 did not change this statement's
  shape (it still ends the function with `out + Viewport line`), only what feeds into `out` earlier
  in pass 2. Every property the prompt requires of the new logic (show/culled semantics, note
  placement outside the char budget, filter=all short-circuit, the untouched early-return prune,
  the untouched children-descent code, the untouched `visible()`) was independently re-verified
  against the ACTUAL two-pass code, not assumed to still hold from the prompt's stale description.
- Decisions made:
  - Placed the one-line comment on `intersectsViewport` ("getBoundingClientRect is viewport-
    relative for every element, so this is correct at any scroll position and for position:fixed
    elements without special cases") wrapped as two short lines to stay under the file's existing
    line-length norms; this is the single comment the prompt's constraint 7 permits on the new
    helper ("At most one short comment on the new helper... is acceptable; more is not"). No
    comment was added at the `culled` declaration site beyond a short trailing note, and no
    comment was added at the `wouldShow`/`show`/`culled` lines inside `measure()` (the code is
    read as self-explanatory there, matching the prompt's own preference to express the logic in
    code rather than prose wherever possible).
  - Did not touch the pass-1 doc comment above `measure()` ("Same entry guards, same show
    computation, same recursion order as a single-pass walk would use...") even though "show
    computation" is now technically two lines (`wouldShow`/`show`) instead of one -- the comment's
    claim (this pass reproduces what a single-pass walk would compute) remains true; rewording it
    was not required by the prompt and would be an unrequested, unscoped comment change.
  - Left `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs` untouched (same pre-existing
    rustfmt-version drift every prior task in this run has flagged). This task touched no Rust
    files at all, so there was nothing to run `cargo fmt` on and no reformatting side effect to
    revert this time; confirmed via `cargo fmt --check`, whose only reported diffs are in exactly
    those same two files, byte-for-byte the same diffs T01's log already described.
- Notes for later tasks:
  - The `intersectsViewport(el)` helper (declared directly after `visible()`, lines ~102-107) is
    now available to any later task in this file; it is intentionally NOT used by `find()` or
    `pageText()` per this task's Out of scope section -- do not wire it in elsewhere without a new
    task prompt actually requiring it.
  - `culled` and the `wouldShow`/`show` split inside `measure()` are now part of the same render-
    tree record shape T01 introduced (`{ unit, ref, indent, children, unitChars, subtreeChars,
    elements, show }`); `show` in that record still reflects the POST-culling decision (i.e. the
    viewport-aware value), so pass 2 (`emit`) automatically treats a culled element exactly like
    any other not-shown node (skip its own line, still walk its children) with zero changes to
    `emit` itself -- verified by inspection, not just assumed, since `emit` reads `record.show`
    directly.
  - The exact note string "Note: interactive results are limited to the current viewport; scroll
    or use filter=all for the full document." is now a byte-exact contract at the same tier as
    T01's three marker-line formats and T06's `[hop: ...]` contract -- do not reword it in a later
    task without updating this note and the T02 BROWSER-TESTS.md entries.
  - No `src/mcp/schemas/tools.json` edits were made or needed; `tests/tool_schema_fidelity.rs`
    passed unchanged (6/6), confirming the frozen `read_page` schema was left untouched, per this
    task's Constraints section.
- Browser checks queued: T02-1, T02-2, T02-3, T02-4 in docs/tasks/release-1/BROWSER-TESTS.md
  (appended after T01-6, preserving task order).

### T03 get_page_text official semantics -- done -- 2026-07-02
- Commit: (recorded after commit; see git log for `feat(extension): T03 ...`)
- Files touched: extension/content.js, extension/service-worker.js,
  docs/tasks/release-1/BROWSER-TESTS.md, docs/tasks/release-1/LEDGER.md
- Tests added: none in the Rust sense (this task touches only extension JS, which has no test
  harness per project constraints). Verification performed instead:
  - `node --check extension/content.js` and `node --check extension/service-worker.js` (syntax
    only), both clean.
  - Full diff review confirming: `PAGE_TEXT_SELECTORS` contains exactly the twelve selectors from
    the prompt's contract, in that exact order; `pageText` reads `el.innerText` /
    `document.body.innerText` only, with zero occurrences of `textContent` or `cloneNode` anywhere
    in the new code; the `body.length < 10` no-readable-content check runs strictly before the
    `body.length > maxChars` truncation check (verified by reading the `if`/`if` sequence); the
    header (`Source element: <sel>\n\n`), the no-readable-content message, and the truncation
    notice all match the prompt's contract strings character for character (only the `${bestSel}`/
    `${maxChars}` placeholders substituted); the old `Title:`/`URL:` lines are gone (grepped for
    `Title:` and `URL:` in the new `pageText` body -- zero matches); the service worker's
    `get_page_text` handler changed on exactly one line (the `content(...)` call), keeping the
    `inGroup` gate, the `text(...)` wrap, and the `"Could not extract page text."` fallback
    untouched; the content script's message-handler `case "pageText"` line is the only case
    touched.
  - `cargo test` (all 91 tests across the workspace -- 80 unit + 4 mcp_protocol + 1 peer_death + 6
    tool_schema_fidelity -- plus 0 doc-tests) passes unchanged, confirming no Rust surface was
    touched and the frozen `get_page_text` schema (including its `max_chars` advertisement) is
    intact.
  - `git status --short -- '*.rs' src/ tests/` was empty throughout -- this task made zero Rust
    changes, exactly as the prompt's "Project context" predicted ("no Rust rebuild is required").
  - `cargo clippy --all-targets -- -D warnings` clean (nothing to lint; no Rust changed).
  - `cargo fmt --check` reports only the same two pre-existing drifted files every prior task in
    this run has flagged (`src/policy/redact.rs`, `tests/tool_schema_fidelity.rs`); neither was
    touched by this task, and there was nothing to run `cargo fmt` on (zero Rust changes), so no
    reformatting side effect occurred this time.
- Drift reconciled: only line-number drift, exactly as the prompt itself warned ("re-verify before
  editing; line numbers may have drifted"). The prompt's Current-behavior section cited the
  `// --- Page text ---` section at content.js lines 194-204 and the message-handler case at line
  299; the actual working tree (after T01/T02 extended `accessibilityTree` earlier in the file)
  had them at lines 279-289 and 428 respectively. Likewise the prompt's service-worker handler
  citation (lines 484-488) was actually at lines 522-526. In every case the CODE SHAPE, selector
  list contents/order, and exact string literals the prompt described matched the working tree
  verbatim; only the line numbers had moved. No logic-level drift.
- Decisions made:
  - Reproduced the prompt's contract snippet verbatim (selectors, `normalizePageText`, `pageText`,
    the message-handler case, the service-worker bridge line) rather than paraphrasing, per the
    prompt's own instruction ("reproduce its behavior exactly... not logic, strings, or
    defaults"). No trivial formatting adjustments were needed; the snippet's style (2-space indent,
    double quotes for strings needing interpolation-safe quoting, template literals) already
    matched the surrounding file.
  - Kept the two comments from the contract snippet (`PAGE_TEXT_SELECTORS` selector-priority note,
    `normalizePageText` conservative-cleanup note) as the only comments added, per constraint 7
    ("the two short comments in the snippet above are the ceiling; do not add more"). No comment
    was added to `pageText` itself.
  - Left `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs` untouched (same pre-existing
    rustfmt-version drift every prior task in this run has flagged). This task touched no Rust
    files at all, so there was nothing to run `cargo fmt` on and no reformatting side effect to
    revert this time; confirmed via `cargo fmt --check`, whose only reported diffs are in exactly
    those same two files, byte-for-byte the same diffs T01's and T02's logs already described.
- Notes for later tasks:
  - `get_page_text`'s output contract (`Source element: <sel>\n\n<body>`, the no-readable-content
    one-liner, and the `[Truncated at N characters. ...]` notice) is now a byte-exact contract at
    the same tier as T01's marker-line formats, T02's Note line, and T06's `[hop: ...]` contract --
    do not reword any of the three strings in a later task without updating this note and the T03
    BROWSER-TESTS.md entries.
  - `PAGE_TEXT_SELECTORS` and `normalizePageText` are module-scope (inside the content script's
    IIFE, alongside `accessibilityTree`'s helpers) and are NOT wired into `accessibilityTree`,
    `find`, or any other function -- per the prompt's Out of scope section, this task intentionally
    does not touch structural/interactive extraction, only free-text extraction.
  - `content(tabId, { type: "pageText", max_chars })` and the content script's `case "pageText"`
    now both pass `max_chars` through; `msg.max_chars` on the content-script side is validated
    entirely inside `pageText()` (any non-finite/non->=1 value silently falls back to 50000) -- the
    service worker performs zero validation of its own, by design (mechanism only, no policy in
    the extension).
  - No `src/mcp/schemas/tools.json` edits were made or needed; `tests/tool_schema_fidelity.rs`
    passed unchanged (6/6), confirming the frozen `get_page_text` schema (and its existing
    `max_chars` advertisement, already present before this task) was left untouched, per this
    task's Constraints section.
- Browser checks queued: T03-1, T03-2, T03-3, T03-4 in docs/tasks/release-1/BROWSER-TESTS.md
  (appended after T02-4, preserving task order).

### T12 Console/network buffers reset on same-tab domain change -- done -- 2026-07-02
- Commit: (recorded after commit; see git log for `feat(extension): T12 ...`)
- Files touched: extension/service-worker.js, docs/tasks/release-1/BROWSER-TESTS.md,
  docs/tasks/release-1/LEDGER.md
- Tests added: none in the Rust sense (this task touches only extension JS, which has no test
  harness per project constraints). Verification performed instead:
  - `node --check extension/service-worker.js` (syntax only), clean.
  - Full diff review confirming: `hostOf` matches the prompt's snippet verbatim; `tabHost` is a
    new module-level `Map` declared alongside the other buffer declarations; the persistent
    `chrome.tabs.onUpdated` listener and `bufferFor` match the prompt's snippets verbatim (byte
    for byte, including the exact reset/adopt/keep-as-is branching); `pushCapped` now routes
    through `bufferFor` and stays capped at 1000 via `buf.items.splice`; the attach closure in
    `ensureAttached` seeds `tabHost` right after `attached.set(tabId, { domains: new Set() })`
    inside its own try/catch that cannot fail the attach; `chrome.tabs.onRemoved` gained exactly
    one new line (`tabHost.delete(tabId);`); both read handlers resolve the tab's live hostname
    fresh via `chrome.tabs.get`, refresh `tabHost`, call `bufferFor`, and read `buf.items` before
    any filter/slice; the two `clear` lines were updated to the new `{ host, items: [] }` shape;
    grepped the two zero-entries strings ("No console messages matching the pattern." / "No
    network requests matching the pattern.") and both `[level] text` / `METHOD url -> status`
    format strings -- byte-identical to the pre-task code, confirmed via `git diff` (no lines
    inside either return statement's template literal changed).
  - `cargo test` (all 91 tests across the workspace, including `tests/tool_schema_fidelity.rs`,
    6/6) passes unchanged, confirming no Rust surface was touched.
  - `git status --short -- '*.rs' src/ tests/` was empty throughout -- this task made zero Rust
    changes, matching the prompt's "Build and test" note ("no Rust rebuild is needed").
  - `cargo clippy --all-targets -- -D warnings` clean (nothing to lint; no Rust changed).
  - `cargo fmt --check` reports only the same two pre-existing drifted files every prior task in
    this run has flagged (`src/policy/redact.rs`, `tests/tool_schema_fidelity.rs`); neither was
    touched by this task, and there was nothing to run `cargo fmt` on (zero Rust changes).
  - ASCII scan (the BOOTSTRAP.md python one-liner) on both edited files (`extension/service-
    worker.js`, `docs/tasks/release-1/BROWSER-TESTS.md`) returned empty lists.
- Drift reconciled: only line-number drift, as the prompt itself warned ("line numbers verified
  against extension/service-worker.js as of this writing" -- earlier tasks in this run had already
  landed and shifted them). The prompt cited the attach closure at "lines 58-61"; the actual
  working tree (after T04/T06/T07 landed) had it at lines 71-78 before this task's edit. The
  prompt cited `chrome.tabs.onRemoved` at "lines 125-133" and the buffering section
  (`chrome.debugger.onEvent`/`pushCapped`) at "lines 137-160"; actual lines were 146-181. The
  prompt cited the two read handlers at "lines 512-536"; actual lines were 554-578. In every case
  the function names, code shape, comment text, and the exact strings the prompt quoted (the
  console/network zero-entries strings, the two schema-description phrases it names in "Project
  context" for `src/mcp/schemas/tools.json`, which was not touched) matched the working tree
  verbatim; only line numbers had moved. No logic-level drift, and `src/mcp/schemas/tools.json`
  itself was never opened for edits (out of scope, and the prompt only cites it for context).
- Decisions made:
  - Kept the read-handler local variable names exactly as the prompt's own snippet uses them
    (`tab`, `host`, `buf`) in both `read_console_messages` and `read_network_requests`, even
    though each name is reused across the two independent handler functions -- there is no
    collision risk since each is its own function scope (verified by reading both handlers in
    full; neither had a pre-existing local named `tab`, `host`, or `buf`), and matching the
    prompt's snippet verbatim minimizes any risk of silently diverging from its documented
    semantics.
  - Placed the new `hostOf` function and the persistent `chrome.tabs.onUpdated` listener at the
    top of the "Console / network buffering" section (immediately after the section's `---`
    header comment, before the pre-existing `chrome.debugger.onEvent` listener), and placed the
    new `bufferFor` helper between the `chrome.debugger.onEvent` listener and `pushCapped` (which
    now calls it). The prompt names exact line ranges to touch but leaves placement of the four
    "new" additions (`hostOf`, `tabHost`, `bufferFor`, `chrome.tabs.onUpdated`) unspecified beyond
    "in the console/network buffering section near the chrome.debugger.onEvent listener" for the
    listener specifically; this placement satisfies that literally and keeps the whole
    buffer-ownership concern (hostname helper -> live tracking -> event-driven append -> ownership
    rule -> capped append) in one readable top-to-bottom block. `tabHost` itself was declared next
    to the other buffer declarations (line 20, after `screenshotCtx`), per the prompt's explicit
    instruction ("Add module-level state next to the buffer declarations").
  - In the `Network.responseReceived` branch of `chrome.debugger.onEvent`, call `bufferFor`
    directly (not `pushCapped`) to look up-or-reset the buffer before searching by `requestId`,
    exactly as the prompt's step 5 specifies; the not-found fallback still calls the existing
    `pushCapped(networkBuffer, tabId, {...})`, which internally calls `bufferFor` a second time --
    this second call is idempotent (the buffer's `host` was already resolved/adopted by the first
    call in this same event tick), so there is no double-reset or lost-append risk. Verified by
    tracing `bufferFor`'s branches by hand for this exact call sequence.
  - Left `src/policy/redact.rs` and `tests/tool_schema_fidelity.rs` untouched (same pre-existing
    rustfmt-version drift every prior task in this run has flagged). This task touched no Rust
    files at all, so there was nothing to run `cargo fmt` on and no reformatting side effect to
    revert this time; confirmed via `cargo fmt --check`, whose only reported diffs are in exactly
    those same two files, byte-for-byte the same diffs T01/T02/T03's logs already described.
- Notes for later tasks:
  - Both buffers are now `tabId -> { host, items: [...] }` instead of `tabId -> [...]`. Any later
    task reading `consoleBuffer`/`networkBuffer` directly (none of the remaining prompts in this
    run appear to) must go through `.items`, not treat the map's value as an array.
  - `bufferFor(map, tabId, host)` is the single choke point for "get or reset-or-adopt a buffer for
    this tab against this hostname"; both the event listener's append path (via `pushCapped`) and
    the two read handlers route through it. A later task adding a new event source that appends to
    either buffer should call `pushCapped`, not touch the maps directly.
  - `tabHost` is refreshed three ways (event-driven via `chrome.tabs.onUpdated`, seeded on attach,
    and refreshed fresh on every read-handler call via `chrome.tabs.get`) but is deliberately never
    persisted (no `chrome.storage`); a service-worker restart starts it empty again, same as
    `attached`/`consoleBuffer`/`networkBuffer`.
  - T15 (Empty-result guidance notes) will touch the exact same two zero-entries return strings
    this task deliberately left untouched ("No console messages matching the pattern." / "No
    network requests matching the pattern."); this task changed nothing about wording, only which
    entries are visible when those strings are chosen.
  - T13 (Runtime.exceptionThrown capture) and T14 (Network.loadingFailed status) both add new
    branches to the same `chrome.debugger.onEvent` listener this task modified. Any new branch
    that appends to `consoleBuffer` or `networkBuffer` must go through `pushCapped` (which now
    routes through `bufferFor`/`tabHost` automatically) to stay domain-scoped; do not append via a
    raw `map.get(tabId).items.push(...)` or reintroduce a bare-array buffer shape.
  - The accepted CDP-race limitation (a cross-domain navigation's main-document
    `Network.requestWillBeSent` can land in the old domain's buffer and be discarded on the next
    reset) is intentional per the prompt's Required-behavior item 8; do not "fix" it with
    `Page.frameNavigated`, `webNavigation`, or URL heuristics without a new task prompt requiring
    it.
- Browser checks queued: T12-1, T12-2, T12-3, T12-4, T12-5 in docs/tasks/release-1/BROWSER-TESTS.md
  (appended after T03-4, preserving task order).
