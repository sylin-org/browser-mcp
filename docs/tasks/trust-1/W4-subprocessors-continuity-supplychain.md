# W4: sub-processors.md + continuity.md + supply-chain.md (ADR-0057 D8/D11/D12)

## Goal
Three short, high-leverage documents: the empty subprocessor list, the continuity story, and the
supply-chain evidence page.

## Preconditions (verify, else STOP)
- W1 DONE. `rg -n "Continuity Promise" docs/adr/0028-tripwire-licensing-and-continuity-promise.md`
  hits; `.github/workflows/release.yml` exists.

## Required content: `docs/trust/sub-processors.md`
Short by design. H1 + one section stating: Ghostlight engages NO subprocessors; no third party
receives customer data because the vendor itself receives none; the only third parties in the
picture are the customer's own choices (their MCP client/model, their SIEM, their policy host).
Changes to this page would be announced in release notes and visible in this file's git history.
Footer.

## Required content: `docs/trust/continuity.md`
1. `## The Continuity Promise` -- quote the promise text VERBATIM from ADR-0028 (locate with
   `rg -n "The Continuity Promise" docs/adr/0028-*.md`; copy the blockquote exactly).
2. `## Why this holds structurally` -- license state never gates behavior; no vendor runtime in
   the customer's path; last-known-good policy cache; fail-closed (never fail-open) when nothing
   is available.
3. `## Verify it yourself` -- the runnable commands for `continuity-source-unreachable`,
   `fail-closed-cold-boot`, `rollback-guardian`, each with one sentence saying what it proves.
4. `## If the vendor ceases to exist` -- source-available governance module + Apache/MIT engine;
   what the customer can keep doing (everything), and the one thing that changes over time
   (no new releases). MUST NOT promise future maintenance or a foundation handoff.
5. Footer.

## Required content: `docs/trust/supply-chain.md`
1. `## Releases` -- signed artifacts, per-file checksums, provenance attestations, the
   package-manager spread; cite the release workflow path.
2. `## SBOM` -- per-release CycloneDX SBOM: reference it as introduced by this batch (W8); state
   the asset naming that W8 pins.
3. `## Dependencies` -- lean-tree posture; pure-Rust signature crypto; the isolated, feature-gated
   network stack (managed-fetch); Socket.dev score claim ONLY as "scored 100/100 on all axes at
   publication (2026-07)" with the npm package link (a dated fact, not a standing promise).
4. `## Build and change management` -- ADR discipline, CI gates (fmt/clippy/tests + the lightbox
   scenario runner), branch model, air-gapped signing keys.
5. Footer.

## Verification (literal)
- `rg -n "no subprocessors|NO subprocessors" -i docs/trust/sub-processors.md` -> >=1.
- `rg -n "> " docs/trust/continuity.md` -> >=1 (the verbatim blockquote).
- `rg -n "ghostlight-lightbox -- run" docs/trust/continuity.md` -> exactly 3.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`docs(trust): W4 sub-processors, continuity, supply chain (ADR-0057)`
