# T5 -- Re-baseline the 13 trained schemas vs v1.0.80, and retire the community reference

Goal: ADR-0050 Decision 1 + Decision 6. (A) Make the official extension the SOLE reference and remove
the community reimplementation. (B) Re-verify the 13 trained tool schemas against official v1.0.80 and
apply any still-unapplied additive/description-only delta. Read ADR-0050 Decisions 1 and 6; normative.

This task has TWO halves. Half A (retirement + doc edits) is FULLY PINNED and MAY land first as a pure
docs/removal change. Half B (schema re-baseline) is a bounded re-harvest with fences. Read BOOTSTRAP.md.
Runs after T4 (or Half A may run any time; Half B does not depend on T1-T4).

## Half A -- Retire the community reference (PINNED)

A1. Remove the community clone from the tree: delete `reference/open-claude-in-chrome/` and
    `reference/ANALYSIS.md` (`git rm -r reference/open-claude-in-chrome reference/ANALYSIS.md`). Git
    history preserves them. If `reference/` becomes empty, remove it too. RE-READ: if anything OUTSIDE
    `reference/` imports or includes files from it (grep the repo for `reference/open-claude` and
    `reference/ANALYSIS`), STOP -- a live dependency means the deletion is not purely a retirement.

A2. Rewrite the CLAUDE.md "## Origin" section to EXACTLY this (ASCII):

    ## Origin

    This is a clean-room Rust rewrite. Its sole reference is Anthropic's official Claude in Chrome
    extension (installed for study; interface and technique are harvested, never code -- see
    docs/research/12 and ADR-0050 Decision 1). An earlier community reimplementation
    (open-claude-in-chrome) informed the initial clean-room build and has since been retired as a
    reference (it was a lossy proxy of the official surface). We do not fork or vendor either; we
    understand the observable interface and rebuild the concept in Rust with a governance-first,
    single-binary architecture.

A3. Remove the `upload_image` exclusion from CLAUDE.md's "## What NOT To Build" list: delete the line
    `- No \`upload_image\` tool.` (ADR-0050 D4 adds `upload_image`). Leave the other exclusions.

A4. `docs/SPEC.md`: find section 10's `upload_image` exclusion (grep `upload_image` in docs/SPEC.md)
    and remove/annotate it as superseded by ADR-0050 D4 (a one-line "(superseded by ADR-0050)" note is
    fine; do not rewrite the section). If SPEC section 10 does not mention `upload_image`, skip and note
    it in the LEDGER.

A5. RE-READ CLAUDE.md for any other reference to `reference/open-claude-in-chrome` (e.g. the Phase 0
    "Reference Study" text, the Repository Structure tree). Update prose that presents the community
    clone as the live reference to point at the official extension; leave clearly-historical
    "Implementation Phases" text (it is labeled historical). Do NOT touch the "Critical constraint"
    trained-schema paragraphs.

## Half B -- Re-baseline the 13 trained schemas against v1.0.80 (bounded re-harvest, fenced)

Context: `docs/research/12-official-extension-parity.md` section A lists schema corrections harvested
from official v1.0.78; some are marked [DONE]. The installed official is now v1.0.80. Re-verify and
apply anything still divergent.

B1. Re-extract the official tool schemas from the installed extension (recipe in doc 12
    "Re-extracting"; `assets/mcpPermissions-*.js`, the `toAnthropicSchema()` returns). If the extension
    is not installed / not extractable, STOP Half B and mark it BLOCKED (Half A still lands).

B2. For EACH of the 13 trained tools, diff our advertised schema (`directory.rs` REGISTRY) against the
    official `toAnthropicSchema`. Apply a delta ONLY IF it is one of:
    - an ADDITIVE optional parameter (e.g. navigate `force`, get_page_text `max_chars` per doc 12), OR
    - a DESCRIPTION-ONLY edit (prose/enum-order wording), rendered ASCII with `--`.
    NEVER rename or remove a trained parameter or enum value, and NEVER change a parameter's type
    (NEVER list). If a delta would require any of those, STOP and record it in the LEDGER for a human
    -- do not apply it.

B3. Work the doc-12 section-A checklist as the starting set (navigate `force`; get_page_text
    `max_chars` + over-limit message; computer `duration` max 10; javascript_tool `action` const
    removal + REPL wording; tabs_create_mcp description; bare-name prose; computer.action enum order;
    read_page "by default"). For each: if v1.0.80 still shows it AND our tree has not applied it, apply
    it; if already applied or no longer present, skip and note.

B4. After applying deltas: `EXPECTED_TRAINED` (names) is UNCHANGED (never touched). The
    `tool_schema_fidelity.rs` structural asserts do not pin per-tool trained descriptions, so a
    description edit will not break them; but if you ADD an optional param, re-read the fidelity/golden
    tests for anything that would notice (it should not -- they pin names + count + explain). Run V-ALL.

## Verify (V-ALL)

Run the BOOTSTRAP V-ALL block. Additionally grep the repo to confirm no non-test file references
`reference/open-claude-in-chrome`.

## Out of scope

- No new tools (T1-T4 own those). No changes to `directory.rs` trained ROWS beyond the sanctioned
  additive-param / description-only edits of Half B. No governance/pipeline changes.

## Commit

Half A: `chore(reference): retire the community reimplementation; official extension is the sole reference (ADR-0050 D1)`.
Half B: `feat(schemas): re-baseline the 13 trained tools against official v1.0.80 (ADR-0050 D6)`.
Update the LEDGER T5 entry (list every applied delta and every STOP/BLOCKED).
