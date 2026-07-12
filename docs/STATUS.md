# STATUS -- where the project stands

Last updated: 2026-07-12. This file is a point-in-time snapshot maintained by whoever
finishes significant work. It exists so a fresh agent (or human) can orient without any
prior session context. **Trust the tree, `git log`, and the batch LEDGERs over this file
when they disagree**, and update it when you land something that changes the picture.

## Now

- **Branches**: `main` = releases, `dev` = trunk. Work lands on `dev`; the owner reviews
  `dev -> main` PRs and cuts releases.
- **Latest published release: v0.5.4** (GitHub Release, npm, homebrew tap, scoop, winget
  all aligned at 0.5.4). v0.5.5 was prepared but never published; its content is folded
  into the 0.5.6 changelog entry.
- **v0.5.6 is prepped and unreleased on `dev`** (CHANGELOG `[0.5.6]` dated 2026-07-12).
  It carries: composable policy tiers + session overlay + `ghostlight demo` (ADR-0060),
  extension-owned browser identity (ADR-0061), browser-relay reconnect resilience
  (ADR-0062), the deploy-quiesce lock (ADR-0063), explicit dev isolation then the
  one-stack model (ADR-0064 amended by ADR-0065), the redesigned on-screen governance
  ribbon + unlisted `notify` tool, the field-splash FX pass, and the SAPS
  security-hardening pass.
- **PR #42 (dev -> main) is MERGED**: v0.5.6 is on `main` (merge `53907f7`); `main` and
  `dev` are tree-identical. (PR #41 was an earlier, already-merged dev->main squash; the
  divergence it left was reconciled by merge `1d54def` -- its only unique content was the
  stale `scripts/dev-browser.ps1`, which ADR-0065 removed.) The RELEASE ITSELF IS NOT CUT:
  no `v0.5.6` tag, no npm/homebrew/scoop/winget publish yet -- that is the owner's
  irreversible-publish step below.
- **Working tree**: clean. Full suite green (fast tier + the entire `--ignored` spawn tier
  locally, and CI on the merge). The spawn-tier e2e tests were aligned with the ADR-0061/0062
  contracts (identity-frame admission, tab-URL probe answers, the rewritten relay-lifecycle
  test); the deploy-quiesce lock is now honored in the Unix self-heal too.

## Release pipeline (what shipping 0.5.6 takes)

The complete, canonical channel-by-channel map is now **`docs/RELEASE.md`**. In short:

1. ~~Owner merges the dev -> main PR.~~ DONE (PR #42, merge `53907f7`).
2. `scripts/release.ps1 0.5.6` from `main`. It now automates: tag, watch CI, verify assets,
   fill package-manager sums, homebrew tap, npm publish + smoke, trust-footer restamp,
   extension publish (Chrome Web Store + Edge; auto if `CWS_*`/`EDGE_*` creds are set, else it
   prints exact steps and points at the built zip), and the website install-guide refresh.
3. Manual remainder only: a winget PR to `microsoft/winget-pkgs` (per version, CLA), and the
   MCP Registry `mcp-publisher` step (DNS auth). Both are called out in the script's report.

The old extension-zip trap is fixed: the Release workflow now builds the CWS-ready zip (key
stripped, dev files excluded) via `package-extension.ps1`, so the shipped asset is submittable.
Store API auto-submit needs one-time credential setup (documented in `docs/RELEASE.md`).

## Owed engineering work (in rough priority order)

- **Lightbox legacy-27 migration** (ADR-0056): the 27 `#[ignore = "e2e"]` spawn tests +
  `scripts/test-e2e.*` migrate scenario-by-scenario into the lightbox harness against a
  per-test parity ledger. Not started; CI runs both tiers until the ledger completes.
- **SAPS remediation remainder** (assessment lives in gitignored `saps/`; findings already
  remediated are in git history around 2026-07-11):
  - SEC-HIGH-03 enforce-half: a confirm-gate for irreversible actions (send/delete/
    purchase) needing out-of-band human confirmation. Design captured in
    `docs/design/managed-mode-network-features.md` (managed intent descriptors); build
    pending.
  - SEC-HIGH-02 full fix: token/auth for non-loopback sources once `enable-remote` returns
    (the action is currently disabled as the interim fix). Same design note; build pending.
  - A1 demo GIF for the README hero slot (README has a commented placeholder): drive
    `gif_creator` or `scripts/capture-readme-tour.ps1`, write `docs/assets/demo.gif`.
- **tabs_create prose leaks the un-encoded native tab id** (found in the ADR-0061 live
  verify; pre-existing, non-regression). Small fix in the tabs_create response text.
- **ADR-0047 stage-2 user-supervised e2e re-run** still owed (needs the owner at a real
  browser).
- **FAQ Q17 follow-up**: no license-expiry scenario exists in lightbox; adding one would
  let the trust-center FAQ point at exactly what it claims.
- Parked (deliberately): audit TCP sink (UDP syslog is the standard; revisit only on ask);
  `socket.yml` capability acknowledgments for the npm package (draft-first, owner call).

## Owner-side gates (agents cannot do these)

- Cut the v0.5.6 release (owner: scripts/release.ps1 0.5.6 from main). PR #42 is merged.
- Chrome Web Store: 0.5.0 zip was submitted 2026-07-10; resubmit after 0.5.6 (extension
  changed). Edge Add-ons: same zip, never submitted.
- MCP Registry: needs DNS TXT auth on the sylin.org apex + `mcp-publisher`.
- Trust center legal: vendor entity name in the MSA (blocked on forming the LLC), the
  cyber-insurance yes/no line, counsel skim of MSA/DPA/LICENSE-GOVERNANCE before first
  EXECUTION (publication already happened by design; drafts are marked as drafts).
- `security.txt` on sylin.org (founder-side, ~1h).
- Key backup + a second npm publisher; one non-author human through the install flow.

## Standing context worth knowing

- The trust center (`docs/trust/`, 13 docs) is PUBLIC on `main` since 2026-07-11 (PR #27)
  at `v0.5.4+dev` footers. Its claims were red-teamed against the tree; keep code and
  claims in lockstep.
- managed:// central policy distribution (ADR-0055) is fully implemented through Phase 5.
- The dev workflow is the one-stack model (ADR-0065): no dev install, no `-dev` host;
  `scripts/dev-loop.ps1` swaps the engine, `-Restore` hands back (and refuses pre-v0.5.5
  releases, which are lock-unaware and fight the swap).
- Machine-local state (which engine runs on a given dev box, install quirks) belongs in
  `local/MACHINE-STATE.md` (gitignored), not here.

## How to update this file

Keep it a snapshot, not a journal: overwrite stale facts instead of appending history
(git history is the journal). Update the date at the top. If an item moves from owed to
done, delete it here and make sure the durable record (ADR, LEDGER, CHANGELOG) carries it.
