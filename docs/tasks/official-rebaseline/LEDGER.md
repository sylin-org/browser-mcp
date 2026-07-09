# LEDGER -- official-rebaseline batch (ADR-0050)

Durable progress log. One task = one entry. The executor updates this file as the LAST step of each
task (or when marking BLOCKED). A human reads RESUME HERE to pick up.

## RESUME HERE

- Status: NOT STARTED.
- Next task: **T1 -- file_upload**.
- Base commit: (fill with `git rev-parse --short HEAD` at batch start).
- Preconditions to re-confirm before T1: advertised tool count is 17; `tests/tool_schema_fidelity.rs`
  still pins `names.len() == 17` and `names[16] == "explain"`; `extension/content.js` still exports
  `deref(ref)`. If any is false, STOP (BOOTSTRAP Failure protocol).

## Task log

(Each entry, filled on completion or BLOCK:)

### T1 -- file_upload
- Status: pending
- Commit(s):
- V-ALL: (pass/fail)
- Deviations: (numbered; "none" if literal compliance)
- Notes:

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
