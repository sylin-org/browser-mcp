# LEDGER -- official-rebaseline batch (ADR-0050)

Durable progress log. One task = one entry. The executor updates this file as the LAST step of each
task (or when marking BLOCKED). A human reads RESUME HERE to pick up.

## RESUME HERE

- Status: T1 DONE. Next task: **T2 -- browser_batch**.
- Base commit for the batch: `d52e0df` (the ADR-0050 + batch authoring commit).
- Advertised tool count is now **18** (`file_upload` inserted before `explain`). Before T2, re-confirm
  `tests/tool_schema_fidelity.rs` pins `names[16] == "file_upload"` and `names[17] == "explain"`.
- IMPORTANT verification note (see ADR-0051 + docs/design/verification-topology-evaluation.md): the
  advertised count/name set is pinned in MANY scattered spawn tests the prompts do NOT all enumerate
  (adapter_override, adapter_reconnect x3, hot_reload's expanded+full_set, pipeline.rs's explain
  literal, plus the 8 count sites). Before committing an additive-tool task, grep the WHOLE tree for
  the old count AND the tail name pair (`"form_fill"`, `"explain"`) and `Some(<oldcount>)`. Run the
  local spawn tier serially with closed stdin and no live `ghostlight service`
  (`cargo test ... -- --test-threads=1 < /dev/null`), else it hangs/flakes environmentally.

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
- Status: pending
- Commit(s):
- V-ALL:
- Deviations:
- Notes:

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
