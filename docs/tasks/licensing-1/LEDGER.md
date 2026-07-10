# Licensing-1 ledger

Durable execution record for the l01-l06 batch. The Task log is append-only (one entry
per task, newest at the bottom); the RESUME HERE block below is updated in place each
task. Each task commits its own ledger changes as part of that task's single commit.

## RESUME HERE

- Progress: **DONE -- implemented directly 2026-07-10 (frontier model, not the l01-l06 batch), with
  enhancements the batch predates.** The engine now lives in one bounded-context module
  `crates/core/src/governance/license/` (crypto.rs + mod.rs + cli.rs), remapped from the batch's
  pre-crate-split paths (`src/governance/license.rs` etc.). Four thin composition-root seams only
  (recorder opaque stamp, hub org-origin gate, doctor section, `main.rs` subcommand + `license-admin`
  feature forward). Committed dev fixture `tests/fixtures/license/dev-license.json` (generated via the
  real `license sign` CLI) + integration test `tests/license.rs`. Full workspace green: 530 core lib +
  16 license unit/integration tests, fmt + clippy (both cfgs) clean; audit_recorder untouched (the
  stamp-absent path is byte-identical).
- Enhancements beyond 00-design (ratified in-session with the owner; ADR-0028 Decisions 3/10/11):
  1. **Composite Ed25519 + ML-DSA-65 signatures** (post-quantum, AND-verify) for production
     generations; keygen 0 stays pure-Ed25519 public dev key. (`fips204` crate.)
  2. **ASCII-armored `-----BEGIN GHOSTLIGHT LICENSE-----` block** as a first-class form (`sign`
     emits it; `install` accepts a file, an armored block, or stdin).
  3. **Stamp gate refined** to "governance operationally in effect via an ORG-deployed policy"
     (`ManifestOrigin::OrgPolicyFile`), not `org_present`-file-exists; dormant in all-open and for
     user `--manifest`. gen-0 licenses are capped to the evaluation tier (closes the public-key
     forge-a-paid-tier hole).
  4. Reused `crate::b64` instead of adding the `base64` crate (lean-internals posture).
- STILL DEFERRED (founder, offline): generate real production composite key(s) and embed them in
  `crypto::verifying_key` (the `Composite` arm + `ed_verifying_key`/`mldsa_verifying_key` helpers are
  `#[allow(dead_code)]` scaffolding until then); the l05 SBOM step and l06 business templates were
  NOT part of this build.
- The l01-l06 task prompts + 00-design.md are kept as the original intent; where they disagree with
  the shipped code, the shipped code + ADR-0028 (Decisions 3/10/11) win.

## Task log

(Append one entry per completed task. Shape:)

### <task-id> <title> -- <date>
- Commit: (see this task's commit)
- Files touched: <list>
- Summary: <what landed, key decisions, any conservative choice made>
- Deviations from the prompt/design: <numbered, each with reasoning; "none" if none>
- Verification: <fmt/clippy/test status; test counts before -> after; the prompt's own
  verification command outcomes>
- Notes for the reviewer: <anything a human should double-check, or "none">

## RUN SUMMARY

(Write after the last task: tasks landed vs BLOCKED, test counts baseline -> final,
deviations rolled up, anything left for a human.)
