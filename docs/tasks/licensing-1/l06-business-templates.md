# L06: business operation templates (transcription only)

## Goal

The license-operations templates ADR-0028 Decision 8 names: two renewal emails, the
founding-org agreement, and the private-repo expiry-reminder workflow. Pure
transcription; every byte is pinned below.

## Authority

ADR-0028 Decisions 6-8; docs/business/PLAN.md "License operations".

## Depends on

Nothing (run even if l01-l05 are BLOCKED). STOP precondition:
docs/business/templates/ does not exist. If it exists, STOP.

## Required behavior

Create docs/business/templates/ with exactly these four files.

### 1. renewal-t30.md

    # Renewal reminder -- 30 days (template)

    Subject: Ghostlight license renewal -- {org}, expires {expires}

    Hi {name},

    Your Ghostlight license ({tier}, {seats} seats) expires on {expires}.

    First, the part that matters: nothing will stop working. Ghostlight never phones
    home, and license state never affects behavior. Enforcement, audit, and your
    production workflows are never interrupted, degraded, or disabled by license
    expiry. That is the Continuity Promise, and it is permanent.

    If the license does lapse, exactly one thing changes: license-state notices appear
    in `ghostlight doctor`, `ghostlight license status`, and your own audit records
    until it is renewed.

    Renewal link: {stripe_link}
    Renewal price (locked at your signup rate): {price}

    Whenever procurement is ready is fine. If anything about the product, the paperwork,
    or the price needs a conversation first, just reply.

    Thanks for running Ghostlight,
    {founder_name}

### 2. renewal-t0.md

    # Renewal reminder -- expiry day (template)

    Subject: Ghostlight license for {org} expired today -- everything keeps working

    Hi {name},

    Your Ghostlight license ({tier}) expired today, {expires}.

    Rest assured everything keeps working: your production is never affected by license
    state. As of today your audit records carry a `"license":"expired"` marker and
    `ghostlight doctor` shows the expired state; renewing clears both.

    Renewal link: {stripe_link}
    Renewal price (locked at your signup rate): {price}

    Reply any time if you need paperwork, a quote, or a conversation.

    {founder_name}

### 3. founding-org-agreement.md

    # Ghostlight founding organization agreement (template)

    Between Sylin ("vendor") and {org} ("member"), effective {date}.

    1. Grant. The vendor issues the member an enterprise-equivalent Ghostlight license,
       free of charge, for twelve (12) months from the effective date.
    2. In exchange, the member agrees to:
       a. one 30-minute feedback call per quarter with the vendor, and
       b. one reference: a named case study if the member's policies allow it, or an
          anonymized-but-quotable reference otherwise.
    3. Founding price lock. After the free year, the member may renew at fifty percent
       (50%) of the then-current list price, and that discount is permanent for as long
       as the member holds a continuous license.
    4. The Continuity Promise applies in full: license state never affects the
       product's behavior, and the member's deployment keeps working regardless of the
       commercial relationship.
    5. Either party may end participation with 30 days written notice; clause 4
       survives; the license issued under clause 1 runs to its stated expiry regardless.
    6. No other terms of the Ghostlight Commercial License are modified by this
       agreement.

    Signed,
    {founder_name} (Sylin) / {member_signatory} ({org})

### 4. expiry-reminder-workflow.yml

    # Template for the PRIVATE ghostlight-licensing repo (never runs in this repo).
    # Reads issued/*.json claims files and opens a reminder issue at T-30 and T-7.
    name: License expiry reminders

    on:
      schedule:
        - cron: "17 6 * * *"
      workflow_dispatch:

    jobs:
      remind:
        runs-on: ubuntu-latest
        permissions:
          issues: write
          contents: read
        steps:
          - uses: actions/checkout@v4
          - uses: actions/github-script@v7
            with:
              script: |
                const fs = require("fs");
                const today = new Date();
                const msPerDay = 86400000;
                for (const f of fs.readdirSync("issued")) {
                  if (!f.endsWith(".json")) continue;
                  const claims = JSON.parse(fs.readFileSync(`issued/${f}`, "utf8"));
                  const days = Math.round((new Date(claims.expires) - today) / msPerDay);
                  if (days !== 30 && days !== 7) continue;
                  const title = `Renewal: ${claims.org} expires in ${days} days (${claims.expires})`;
                  const open = await github.rest.issues.listForRepo({
                    owner: context.repo.owner, repo: context.repo.repo, state: "open"
                  });
                  if (open.data.some(i => i.title === title)) continue;
                  await github.rest.issues.create({
                    owner: context.repo.owner, repo: context.repo.repo,
                    title,
                    body: `${days}-day reminder for ${claims.licensee}. Adapt docs/business/templates/renewal-t30.md (renewal-t0.md is for expiry day). Tier: ${claims.tier}, seats: ${claims.seats}.`
                  });
                }

## Constraints

Byte-for-byte transcription (strip the 4-space code-block indentation; each file's
content starts at column 0). No em-dashes, no smart quotes (the templates use " -- ").
Only docs/business/templates/ is created; nothing else changes.

## Tests (from repo root)

- `ls docs/business/templates` shows exactly the four file names above.
- `rg -c "Continuity Promise" docs/business/templates/renewal-t30.md` prints `1`.
- `rg -c "everything keeps working" docs/business/templates/renewal-t0.md` prints >= 1.
- `rg -c "fifty percent" docs/business/templates/founding-org-agreement.md` prints `1`.
- `rg -n "\t" docs/business/templates/expiry-reminder-workflow.yml` prints nothing.
- `rg -n "[^\x00-\x7F]" docs/business/templates/` prints nothing.

## Verification

The checks above; ASCII diff scan; `cargo test` untouched (docs only; no spot-run
needed); ledger entry; commit.

Commit subject: `docs(business): license-ops templates (renewals, founding agreement, reminder workflow)`

## Out of scope

Any file outside docs/business/templates/; the PRIVATE repo itself (founder creates it;
FOUNDER-TODO.md item); pricing numbers (the templates use placeholders).
