# LEDGER: closed-loop browser core (ADR-0078)

Durable progress. One task equals one commit. Update this file before and after each task.

## RESUME HERE

- ADR-0078 implementation tasks C1-C6 are complete.
- Automated gates and public inventory synchronization are complete.
- `LIVE-VERIFY.md` passed on the ordinary Linux desktop profile on 2026-07-15.
- The live run found and fixed retained-intent subrequest deduplication and pre-guard dialog work.
- The Windows handoff review closed cross-platform CI, provenance negotiation, reconnect response
  scoping, and scroll's remaining pre-guard preparation gaps.
- Cross-origin frame refs are out of scope and require a separate ADR.

## Task log

| Task | Commit | Status | Notes |
|------|--------|--------|-------|
| C1 actionable observations | a5a2391 | DONE | Shared summary, ranked matcher, structured secret redaction; all gates green |
| C2 interaction receipts | 50d87e2 | DONE | Bounded observed-after receipt, target assurance, dialog blocker; all gates green |
| C3 act_on | 9c2901b | DONE | Semantic targeting, dynamic RAWX, bounded recovery, adaptive wait, minimized audit; all gates green |
| C4 output provenance | 0c19add | DONE | Session nonce, page-text boundaries, structured provenance, and final service-side budgets; all gates green |
| C5 dialog control | 105c4d0 | DONE | Explicit status/accept/dismiss/respond, CDP lifecycle cleanup, blocker propagation, minimized audit; all gates green |
| C6 tab control | b14b636 | DONE | Explicit focus/reload/close, exact ownership release, group preservation, content-free receipts and audit; all gates green |
| Visible Linux verification | 08786c2, 78487af | DONE | Five live journeys passed; two visible-only defects fixed with regression coverage |
| Cross-platform session gate | 520b324 | DONE | Linux-only imports are cfg-gated; ownership mismatch test reaches the intended branch |
| Demo provenance negotiation | b30026d | DONE | Explicit current/legacy contract negotiation, fail-closed unnegotiated state, full nonce grammar |
| Connection-scoped replies | 6d00b23 | DONE | Tool and auxiliary async replies retain their accepting port under request-ID reuse |
| Scroll dialog guard | b3be776 | DONE | Ref resolution, probes, cursor movement, dispatch, and fallback all follow the blocker check |
| Complete JavaScript CI | fdb4dde | DONE | Full extension discovery and syntax plus npm launcher tests on the OS matrix |

## Batch checks

| Check | Status | Evidence |
|-------|--------|----------|
| Rust format, clippy, workspace tests | PASS | 679 core unit tests plus workspace integration/doc tests |
| Extension syntax and tests | PASS | 108 extension tests; whole-file syntax gate; 4 npm launcher tests |
| Lightbox all scenarios | PASS | 34 of 34 scenarios through the isolated `target-check-closed-loop` runner |
| Visible-browser verification | PASS | Ordinary profile, visible Chrome 150.0.7871.124, real Codex 0.144.4 and raw MCP client |
| Tool count and public docs synchronized | PASS | README and STATUS name the additive 25-tool surface |

## Visible verification record

- Date: 2026-07-15.
- Browser: Google Chrome 150.0.7871.124, ordinary user profile, one enabled Ghostlight 0.5.8
  unpacked extension.
- Clients: Codex CLI 0.144.4 for a real model-driven browser action; raw MCP for deterministic
  journey assertions and byte counts.
- Candidate: engine from ADR-0082 commit `7607ee6`; extension includes Presentation Broker commit
  `b7f2782` and live-fix commits `08786c2` and `78487af`. Nothing was published or released.
- Semantic journey: one `act_on` call returned the expected origin, title, and state in 1,657
  response bytes. The decomposed `find` + `computer` + `wait` path used three calls and 3,604 bytes.
- Ambiguity journey: two matching controls produced `ambiguous_target`; neither control changed.
- Dialog journey: a user-triggered alert blocked the next interaction as `dialog_open`; explicit
  acceptance recovered the flow and the following semantic action met its expectation.
- Lifecycle journey: focus, reload, and close passed. An inferred unowned native tab ID was refused
  with the exact managed-ID correction and remained open until closed visibly by the user.
- Trust journey: the service-authored nonce boundary matched the real page output, a page-authored
  fake boundary could not match it, provenance marked the output page-sourced and untrusted, and
  current-client audit records contained outcome metadata without fixture text, values, geometry,
  URLs, nonces, screenshots, or other page payloads.
- Presentation journey: managed border, navigation pill, read scan, screenshot camera/frame, and
  post-capture border recovery rendered visibly. Narration completed in under one second while a
  same-tab page wait remained active for at least 3.5 seconds.

## Deviations

1. The authored bootstrap said to run `node --test` from `extension/`, but extension tests live in
   `tests/extension/`. C1 corrected the command to the repository's real test location.
2. The first aggregate Lightbox run passed 32 of 34 scenarios. The two organization-policy
   scenarios had exact tool lists that predated the three additive tools. Commit `7ad569e` updated
   those fixtures; the aggregate rerun passed 34 of 34.
