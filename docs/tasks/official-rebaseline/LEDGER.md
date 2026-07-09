# LEDGER -- official-rebaseline batch (ADR-0050)

Durable progress log. One task = one entry. The executor updates this file as the LAST step of each
task (or when marking BLOCKED). A human reads RESUME HERE to pick up.

## RESUME HERE

- Status: T1 + T2 DONE. Next task: **T3 -- upload_image**.
- Base commit for the batch: `d52e0df` (the ADR-0050 + batch authoring commit). T2 = `72f9b8a`.
- Advertised tool count is now **19** (`file_upload`, then `browser_batch`, both before `explain`).
  `tests/tool_schema_fidelity.rs` pins `names[16]=="file_upload"`, `[17]=="browser_batch"`,
  `[18]=="explain"`. Before T3, re-read the tree.
- BUILD NOTE (post dev re-install): live MCP clients continuously respawn `ghostlight-relay` and lock
  the normal `target/debug`, so the FULL V-ALL (which builds relay + spawns for the e2e tier) must run
  in an ISOLATED `CARGO_TARGET_DIR` (`CARGO_TARGET_DIR=$TMP/gl-target cargo build --workspace && cargo
  test --workspace -- --include-ignored --test-threads=1 < /dev/null`). Kill orphan `ghostlight.exe`
  first if a prior isolated run left a service locking the isolated dir. Core-lib-only checks
  (`cargo test -p ghostlight-core --lib`) run fine in the normal dir (no exe link).
- IMPORTANT verification note (see ADR-0051 + docs/design/verification-topology-evaluation.md): the
  advertised count/name set is pinned in MANY scattered spawn tests the prompts do NOT all enumerate
  (adapter_override, adapter_reconnect x3, hot_reload's expanded+full_set, pipeline.rs's explain
  literal, plus the 8 count sites). Before committing an additive-tool task, grep the WHOLE tree for
  the old count AND the tail name pair (`"form_fill"`, `"explain"`) and `Some(<oldcount>)`. Run the
  local spawn tier serially with closed stdin and no live `ghostlight service`
  (`cargo test ... -- --test-threads=1 < /dev/null`), else it hangs/flakes environmentally.

- **RE-PIN (ADR-0051 P1.1/P4.2 landed AFTER this batch was authored; supersedes every task's
  count-bump steps):** the advertised count now DERIVES from
  `directory::advertised_tool_count()` / `advertised_tool_names()` at ALL behavior sites, which no
  longer carry a literal to bump: `tests/mcp_protocol.rs`, `tests/tool_enforcement.rs`,
  `crates/core/src/hub/outbound/mod.rs` (x2), `tests/adapter_override.rs`,
  `tests/adapter_reconnect.rs`, `tests/hot_reload.rs`. So T2 Part D items 6/8/9, and the analogous
  count-assert steps in T3/T4/T5, are OBSOLETE -- do NOT edit those assertions. The ONLY sites an
  additive tool still hand-edits are: (1) `crates/core/src/browser/directory.rs` -- the REGISTRY row,
  the `EXPECTED` + `EXPECTED_TOOLS` `#[cfg(test)]` tables, the `total_variants` literal, and the two
  doc-comment counts (`N descriptors`, `N rows`); (2) `tests/tool_schema_fidelity.rs` -- `names.len()`
  + `all.len()` literals and the tail position asserts; (3) `tests/all_open_golden.rs` --
  `GOLDEN_TOOL_NAMES` array + its `[&str; N]` len + count message + doc; (4)
  `crates/core/src/mcp/pipeline.rs` -- the frozen `pinned_explain_text()` literal (the prompts OMIT
  this; add the new tool's `"<tool>: requires <cap>. <directory_description>"` line before `explain`).
  Stale DOC-COMMENT counts elsewhere (e.g. `tool_enforcement.rs`'s "18 tools" narration,
  hub/outbound's "N-declaration REGISTRY" prose) are cosmetic -- update for accuracy, but they are not
  assertions and never block V-ALL.

## Task log

(Each entry, filled on completion or BLOCK:)

### T1 -- file_upload
- Status: DONE
- Commit(s): (filled at commit)
- V-ALL: pass. fmt/clippy/build clean; ~600 unit tests + directory/hub pins (32) + the four oracle
  suites (tool_schema_fidelity, all_open_golden incl. the new governance test, mcp_protocol,
  tool_enforcement) + both pipeline.rs explain-text pins + the extension node --test (fileset 4/4)
  all green. The spawn tests that initially failed were fixed (see deviations) and re-run to green in
  isolation (adapter_override 2, adapter_reconnect 2, hot_reload 1). Local full-workspace green
  requires the Phase-1 procedure (serial + closed stdin + no live service; ADR-0051).
- Deviations:
  1. The prompt did not enumerate `crates/core/src/mcp/pipeline.rs`'s frozen `pinned_explain_text()`
     literal. `explain`'s output is DERIVED from the directory, so adding file_upload changed it.
     Added the `"file_upload: requires write. Upload files (base64 bytes) ..."` line before explain,
     matching the real formatter (`requires.first()` -> "write").
  2. The prompt AND the C1 red-team both missed four hardcoded advertised-COUNT asserts in spawn
     tests: `tests/adapter_override.rs:227` and `tests/adapter_reconnect.rs:{174,200,307}`, all
     `Some(17)` -> `Some(18)`. (These only fail through the E2E tier, which is why they were missed.)
  3. The prompt missed two advertised-NAME-set arrays in `tests/hot_reload.rs`: the `expanded`
     write-grant set and the `full_set` all-open set both needed `"file_upload"` before `"explain"`
     (file_upload requires [write], a subset of the [read,action,write] grant). `governed_read_only`
     correctly excludes it. Also bumped two stale doc counts (a "(17 tools)" -> 18 and a pre-existing
     stale "all-open 14" -> 18).
  4. (Process, not code) Local V-ALL's spawn tier is environment-sensitive: it hangs on interactive
     stdin and flakes on a relaunching persistent service / Chrome exe-lock. Ran it serially with
     `< /dev/null` and no live service. This fragility motivated ADR-0051 + the eval doc (authored in
     the same working session but a SEPARATE track from the ADR-0050 batch).
- Notes: file_upload is ExtensionForward (no new Rust arg struct/wire type); extension path is
  `lib/fileset.js decodeFiles` -> content.js `setFiles` -> service-worker `file_upload` handler;
  `paths` advertised-but-rejected (no host FS). New `tests/extension/fileset.test.js` added to ci.yml
  + BOOTSTRAP V-ALL. Two ADR-0050-unrelated files also landed for the verification eval
  (docs/design/verification-topology-evaluation.md, docs/adr/0051-*.md, README index) -- these are
  the owner-requested architecture evaluation, committed separately from T1.

### T2 -- browser_batch (overload; script kept)
- Status: DONE
- Commit(s): (filled at commit)
- V-ALL: pass (isolated CARGO_TARGET_DIR -- a live client relay locks the normal target/debug after
  the dev re-install). fmt --check clean; clippy --workspace --all-targets -D warnings clean; full
  workspace `cargo test -- --include-ignored --test-threads=1` = 44/44 binaries green (core lib 483
  incl. the 5 new browser_batch tests + all script.rs regression tests unchanged; the four oracle
  suites; the batch-reject test now asserting `browser_batch`; and the e2e tier).
- Deviations:
  1. Per the RESUME-HERE RE-PIN (ADR-0051 P1.1/P4.2 landed after authoring): Part D items 6/8/9 were
     OBSOLETE -- mcp_protocol/hub-outbound/tool_enforcement count asserts DERIVE from
     `advertised_tool_count()` now and carry no literal to bump. Left untouched (only their cosmetic
     doc-comment "18 tools" narration updated to 19).
  2. The prompt (like T1) omitted `crates/core/src/mcp/pipeline.rs`'s frozen `pinned_explain_text()`
     literal. Added the `browser_batch: requires nothing. Run a sequence of tool calls ...` line
     before explain (matching the real formatter: `&[]` -> "requires nothing").
  3. The prompt omitted `crates/core/src/browser/advertise.rs`'s OWN inline unit tests (the
     read-only + empty-grants advertised-set goldens the tool_advertisement.rs integration test defers
     to). browser_batch requires nothing, so it joins EVERY advertised set; added it to both.
  4. The prompt omitted the scattered advertised-set pins in the e2e/spawn tests: `hot_reload.rs`
     (`governed_read_only` + `expanded`), `manifest_validation.rs` (read-only), and
     `tool_advertisement.rs` (read-only + empty-grants). Added `browser_batch` before `explain` in all.
     (This is exactly the class the RESUME-HERE note warns about; the grep-the-whole-tree step found
     them.)
  5. SANCTIONED design deviation: `run_batch`'s signature gained `orchestrator: &'static str` (the
     prompt's A1 signature omitted it, hardcoding "script"). browser_batch's internal step audit
     records must be attributed to `"browser_batch"`, not `"script"` -- honest audit attribution in a
     governance tool. `interpret` (script) passes "script", so script's audit + compact output are
     byte-identical (proven by the unchanged script.rs regression suite).
- Notes: Part A refactor is behavior-preserving for `script`: the shared loop is now
  `run_batch -> BatchRun{steps: Vec<StepOutcome>, summary, duration_ms, batch_id}`, where
  `StepOutcome.result` keeps each step's FULL MCP result (content + structuredContent) so
  browser_batch preserves images; `build_compact(BatchRun)` derives the compact text/structured from
  it. `interpret = build_compact(run_batch(.., "script"))`. `StepRunner`/`PipelineRunner` are now
  `pub(crate)` so browser_batch wires the same engine. Nesting is symmetric (a `script` OR
  `browser_batch` step is rejected in either batcher).

### T3 -- upload_image (screenshot cache + drag-drop)
- Status: pending
- Commit(s):
- V-ALL:
- Deviations:
- Notes:

### T4 -- gif_creator (phased; Phase 1 floor)
- Status: pending
- Commit(s):
- V-ALL:
- Deviations:
- Notes:

### T5 -- 13-tool re-baseline vs 1.0.80 + retire reference/
- Status: pending
- Commit(s):
- V-ALL:
- Deviations:
- Notes:

## Deviation index (cross-task, for the next-batch review)

(Append one line per numbered deviation as they occur: `T<n>.<k>: <what and why>`.)

- T1.1: pipeline.rs `pinned_explain_text()` frozen literal not in the prompt; explain derives from the directory, so added the file_upload line (formatter uses `requires.first()` -> "write").
- T1.2: four `Some(17)` advertised-count asserts in adapter_override/adapter_reconnect not in the prompt or C1 red-team -> Some(18); only observable via the E2E tier.
- T1.3: hot_reload `expanded` (write-grant) + `full_set` (all-open) name arrays needed file_upload before explain; two stale doc counts corrected.
- T1.4: (process) local spawn-tier V-ALL is environment-sensitive (interactive stdin hang; persistent-service/Chrome exe-lock); ran serial + closed-stdin; motivated ADR-0051 (separate track).
