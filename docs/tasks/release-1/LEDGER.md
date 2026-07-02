# Release-1 execution ledger

This file is the working memory of the unattended run defined in BOOTSTRAP.md.
The agent updates it before and after every task and commits it with each
task's changes. Humans read it to understand exactly what happened.

## RUN SUMMARY

(Written by the agent at the end of the run. Empty until then.)

## RESUME HERE

- Current task: T07 (next pending). T04 and T06 are done.
- Branch: release-1-hardening (create from main if absent).
- Last commit: feat(mcp): T06 hop-attributed error reporting across the full dispatch path (this run)
- Open concerns: pre-existing `cargo fmt` drift (unrelated to T04/T06) in `src/policy/redact.rs`
  and `tests/tool_schema_fidelity.rs` -- both reformat under the installed rustfmt 1.9.0 but were
  left untouched again in T06 because they are out of scope / forbidden. A whole-repo
  `cargo fmt --check` will report these two files; `rustfmt --check --edition 2021` on only the
  files T06 touched (src/browser.rs, src/error.rs, src/mcp/server.rs, src/mcp/tools.rs,
  src/native/messages.rs, tests/mcp_protocol.rs) is clean; src/lib.rs was excluded from that
  targeted check because passing it to standalone `rustfmt` treats it as a crate root and pulls
  in every `mod`-reachable file (including the two drifted ones) -- its one-line diff (adding
  `ToolError` to the re-export list) was verified by inspection instead. A human may want to run
  `cargo fmt` repo-wide in its own dedicated commit at some point; do not fold that into an
  unrelated task's commit.

## Sequence and status

Order: T04, T06, T07, T01, T02, T03, T12, T13, T14, T15, T08, T09, T10, T11, T18, T16, T17, T05.

| # | Task | Title | Depends on | Status |
|---|------|-------|-----------|--------|
| 1 | T04 | Extension-channel warmup + bounded first-call wait | - | done |
| 2 | T06 | Hop-attributed error reporting | T04 (binary half) | done |
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
