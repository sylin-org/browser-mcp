# Latest coordination request

- Updated: 2026-07-20
- From: windows-codex
- To: linux-codex
- Status: requested
- Repository: `/home/leo/repo/github/sylin-org/ghostlight`
- Branch: `dev`
- Required head: `656c4d2`
- Subject: final Linux release-candidate verification.
- Deployment: pull `dev`, rebuild and deliberately activate the user-level candidate, then
  explicitly reload the unpacked extension. Verify the active binary and extension both reflect
  the required head before drawing conclusions.
- Test boundary: fresh real Codex MCP session plus visible Chrome in leo's ordinary graphical
  profile. No Playwright, headless browser, isolated profile, virtual display, or emulation.
- Workspace gate: with two eligible normal Chrome windows, prove first-touch work reuses the
  last-clicked existing window without spawning another, remains pinned after focus moves, and
  survives the natural Linux WINDOW_ID_NONE transition.
- Core smoke: prove the controlled-tab border persists, navigation and page reading work, ordinary
  typing inserts characters, protected typing stays visually private, JavaScript shows its
  workwheel, screenshot shows its post-capture camera treatment, and ref-based `scroll_to` shows
  chevrons settling into the exact destination halo.
- Destination smoke: capture a real screenshot, dispatch it through coordinate `upload_image`, and
  prove the fixed photo tile settles into the destination halo. The tool result must separately
  preserve whether the page signaled handling; the cue must not display a filename or page data.
- Input regression smoke: repeat one ordinary click, one shortcut, and both pointer-only and native
  HTML drag paths on visible pages. Confirm page-observed outcomes, not just tool acknowledgements.
- Gate: run formatting, strict clippy, the full Rust workspace, all extension tests and syntax
  checks. Record exact versions, counts, and visible outcomes in durable status.
- Authority: diagnose and fix product defects with regression coverage, commit logical fixes, and
  push `dev`. Do not merge `main`, tag, publish, or release.
