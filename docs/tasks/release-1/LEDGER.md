# Release-1 execution ledger

This file is the working memory of the unattended run defined in BOOTSTRAP.md.
The agent updates it before and after every task and commits it with each
task's changes. Humans read it to understand exactly what happened.

## RUN SUMMARY

(Written by the agent at the end of the run. Empty until then.)

## RESUME HERE

- Current task: T06 (next pending). T04 is done.
- Branch: release-1-hardening (create from main if absent).
- Last commit: feat(mcp): T04 extension-channel warmup + bounded first-call wait (this run)
- Open concerns: pre-existing `cargo fmt` drift (unrelated to T04) in `src/policy/redact.rs` and
  `tests/tool_schema_fidelity.rs` -- both reformat under the installed rustfmt 1.9.0 but were left
  untouched because they are out of scope / forbidden for T04. A whole-repo `cargo fmt --check`
  will report these two files; `rustfmt --check` on only the files T04 touched
  (src/browser.rs, src/mcp/server.rs, tests/mcp_protocol.rs) is clean. A human may want to run
  `cargo fmt` repo-wide in its own dedicated commit at some point; do not fold that into an
  unrelated task's commit.

## Sequence and status

Order: T04, T06, T07, T01, T02, T03, T12, T13, T14, T15, T08, T09, T10, T11, T18, T16, T17, T05.

| # | Task | Title | Depends on | Status |
|---|------|-------|-----------|--------|
| 1 | T04 | Extension-channel warmup + bounded first-call wait | - | done |
| 2 | T06 | Hop-attributed error reporting | T04 (binary half) | pending |
| 3 | T07 | Extend installer doctor with runtime/debug-state fusion | - | pending |
| 4 | T01 | read_page structural pagination + caps | - | pending |
| 5 | T02 | read_page viewport culling (filter=interactive) | - | pending |
| 6 | T03 | get_page_text official semantics | - | pending |
| 7 | T12 | Per-domain console/network buffer reset | - | pending |
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
