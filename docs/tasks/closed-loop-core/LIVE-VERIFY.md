# LIVE VERIFY: closed-loop browser core

Run only after C1-C6 and all automated gates pass. Use the normal visible Chrome profile through the
one-stack dev loop. Do not use a headless or isolated browser.

## Setup

- Start the fresh engine with `scripts/dev-loop.ps1` and reload the unpacked extension.
- Restart the MCP client so the additive tools are advertised.
- Use a disposable local test page containing unique and duplicate controls, a form field, a
  delayed status update, and a JavaScript dialog trigger. The page must not contain real secrets.
- Start an ordinary debug/audit observation session. Do not enable payload persistence.

## Journey A: semantic success

1. `find` a unique named control and inspect the actionable summary.
2. Call `act_on` by name with an expected delayed status.
3. Confirm one model call returns the resolved target, semantic assurance, observed expectation,
   page facts, provenance, and no unnecessary full-page dump.
4. Confirm the user saw one short target treatment before the visible action.

## Journey B: ambiguity refusal

1. Create two same-tier matching controls.
2. Call `act_on` with the ambiguous name.
3. Confirm neither control changes and the recovery capsule lists bounded candidate facts plus one
   narrow next step.

## Journey C: dialog recovery

1. Trigger a JavaScript dialog.
2. Confirm the next relevant interaction reports `dialog_open` and does not continue blindly.
3. Inspect with `dialog status`, resolve explicitly, then complete the interaction.

## Journey D: owned-tab lifecycle

1. Open a Ghostlight-owned tab.
2. Focus and reload it.
3. Explicitly close it and confirm the client group and user's pre-existing tabs remain.
4. Attempt the same operation on a user tab and confirm refusal.

## Journey E: trust and audit

1. Confirm page text is inside a nonce-bearing untrusted boundary and structured provenance names
   the correct origin.
2. Put a fake boundary string in the page and confirm it cannot match the real session nonce.
3. Inspect audit records: assurance and outcome categories are present; query, name, values, text,
   dialog text, geometry, href, nonce, and screenshots are absent.

## Record

Write the date, browser version, client, engine commit, extension version, observed call/byte
comparison, and pass/fail notes into `LEDGER.md`. Remove the disposable overlay/page state and run
`scripts/dev-loop.ps1 -Restore` when complete.
