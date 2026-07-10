# trust-1 batch: BOOTSTRAP

Execution package for the Open Trust Center (ADR-0057, including its Research-ratification section
D10-D12). Executor: a lesser model with ZERO conversational context. Follow literally; resolve
nothing by judgment. Semantics live in ADR-0057; these files pin the HOW. This is mostly a
DOCUMENTATION batch: the oracles are pinned headings, pinned sentences, and banned claims.

## Authority order (conflicts resolve upward)
1. `docs/adr/0057-open-trust-center.md` (all decisions incl. D10-D12) and, where cited, ADR-0028
   (Continuity Promise, never-phone-home D9, tiers), ADR-0055 (managed://, D9 register), ADR-0056
   (lightbox scenarios).
2. This BOOTSTRAP + task files W1-W9.
3. The live tree (re-read before every task; STOP on a failed precondition; do not improvise).

## Environment facts
- Windows 11; repo `f:\Replica\NAS\Files\repo\github\sylin-org\browser-mcp`; branch `dev`.
- Live MCP clients LOCK `target/`: for any cargo command use an isolated target dir
  (bash: `export CARGO_TARGET_DIR="$TEMP/gl-trust1-ct"`).
- All trust documents live in `docs/trust/`. Version to cite in footers: read
  `version` from the root `Cargo.toml` [package] at execution time.

## Shared conventions (every document in docs/trust/)
- REGISTER: professional, plain, precise (ADR-0055 D9 / ADR-0057 D1). No mascot voice, no
  marketing superlatives, no exclamation marks. ASCII only; `--` never an em-dash; no AI-isms
  ("delve", "leverage", "robust", "seamless" are banned words).
- ANSWER SHAPE (ADR-0057 D3): direct, quotable paragraph FIRST (self-contained: a reviewer can
  paste it into an assessment portal); links AFTER; and where the task file specifies, an
  `Evidence:` line naming artifacts (ADR file, source path, test/scenario name, guide).
- ABSENCES are stated as facts with reasons, never apologies (D11b). Example register: "Ghostlight
  has no SOC 2 report. The runtime runs entirely on your infrastructure, so the assurance a SOC 2
  would provide about vendor-side data handling does not apply; the vendor-side assets that DO
  matter (source, signing keys, release pipeline) are addressed below."
- FOOTER (last line of every file, exact shape):
  `Last reviewed: 2026-07-10 against v<version> | Contact: support@sylin.org`
- Cite existing guides (`docs/guides/*.md`), never duplicate their content (one source per fact).
- Runnable evidence commands use exactly:
  `cargo run -p ghostlight-lightbox -- run <scenario>` and name only scenarios that exist
  (verify with `rg -n "\"<scenario>\"" crates/lightbox/src/scenarios.rs`).

## BANNED CLAIMS (the over-claim list; violating any is a task failure)
- Never "SIEM integration" unqualified: say "audit streams to syslog (RFC 5424 over UDP) or JSON
  Lines files today; HTTP delivery is deferred" (verify: docs/guides/siem-integration.md).
- Never claim SOC 2, ISO 27001, ISO 42001, CSA STAR, or a completed penetration test.
- Never "open source" for the governance module: it is "source-available" (ADR-0027); the engine
  is Apache-2.0 OR MIT.
- Seats/licensee are LEGAL terms, never enforced at runtime (ADR-0028); never imply enforcement.
- Support commitments are ACKNOWLEDGMENT times (3 business days Team / 2 Enterprise), never
  resolution times.
- EU AI Act text SUPPORTS the customer's deployer duties; never asserts Ghostlight's or the
  customer's legal compliance (D11e). No legal advice anywhere.
- Never promise encryption-at-rest of local files that are not encrypted (the managed cache is
  SIGNED and verified-on-load, not encrypted; ADR-0055 Impl.5).
- Never state a data flow to the vendor exists, even hypothetically softened ("minimal
  telemetry"): the correct claim is ZERO vendor-bound traffic (ADR-0028 D9).

## Per-task procedure
1. Read the task file fully; verify PRECONDITIONS (exact commands given); mismatch -> STOP.
2. Write exactly the REQUIRED CONTENT (headings pinned; prose is yours WITHIN the conventions and
   banned-claims rules above; pinned sentences are verbatim).
3. Run VERIFICATION (literal commands; mostly `rg` string checks).
4. Commit only the files the task names, with the pinned message; update `LEDGER.md` (status,
   hash, numbered deviations).

## Global verification (after each task)
- `rg -n "—" docs/trust/` -> NO hits (no em-dashes; the pattern is the literal em-dash char).
- `rg -ni "open source" docs/trust/` -> hits ONLY where explicitly about the Apache/MIT engine.
- `rg -n "Last reviewed:" docs/trust/*.md` -> every .md present has the footer.
- For W8 only: `cargo build --workspace` green in the isolated target dir.

## Failure protocol
On any STOP: revert unstaged changes, mark the task BLOCKED in LEDGER.md with the exact failing
precondition/output, HALT the batch.

## NEVER touch
- Anything under `crates/`, `src/`, `extension/`, `tests/` -- EXCEPT W8's two named files
  (`.github/workflows/release.yml` addition; `security-insights.yml` at repo root) and nothing else.
- `docs/SPEC.md`, `docs/adr/**` (cite, never edit), `LICENSE*`, existing `docs/guides/**` content
  (W9 MAY add a cross-link line to `docs/guides/README.md` and the root `README.md` ONLY).
- Never push; never touch versions or package metadata.

## Task sequence (one task = one commit; every prefix leaves a coherent tree)
W1 skeleton/index -> W2 faq -> W3 security-overview + data-flows -> W4 sub-processors + continuity
+ supply-chain -> W5 controls + questionnaire -> W6 support-policy + tiers + PLAN.md pricing sync
-> W7 msa + dpa drafts -> W8 SBOM CI + security-insights.yml + SECURITY.md alignment -> W9
red-team + cross-links.
