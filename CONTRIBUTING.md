# Contributing, questions, and requests

Input is genuinely wanted -- questions, requests, and contributions have three lanes.

## Where to reach us

| Lane                                    | Use it for                                                                                                                                                                 |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [GitHub Issues](../../issues)           | Bugs, defects, anything reproducible.                                                                                                                                      |
| [GitHub Discussions](../../discussions) | Questions, ideas, feature requests, policy patterns, show-and-tell.                                                                                                        |
| hello@sylin.org                         | Anything that cannot be public: security reports (see [SECURITY.md](SECURITY.md)), licensing and founding-program matters, or a compliance team that cannot post publicly. |

Public lanes are preferred when possible: an answered question becomes documentation,
and a discussed request becomes a visible roadmap decision. Founding and enterprise
licensees get the response times in [PRICING.md](PRICING.md); everyone gets best-effort,
honestly.

## How requests are evaluated

Every request gets a disposition, with reasoning: accepted (and roughly when), deferred
(and what would change that), or declined (and why). The filter is the project's
recorded vision, not taste of the day:

- **User delight first; governance that never punishes the ungoverned.** All-open stays
  first-class. Features that make the free path worse to upsell the paid one are
  declined on principle.
- **The sacred tool surface.** The 13 trained tool schemas are byte-pinned (plus
  `explain`); requests to add, rename, or reshape MCP tools on that surface are
  declined, whatever their merit ([ADR-0007](docs/adr/0007-sacred-tool-surface.md),
  [ADR-0022](docs/adr/0022-intent-calibrated-capabilities.md)).
- **Never phone home.** Telemetry, activation servers, and update pings are permanently
  out ([ADR-0028](docs/adr/0028-tripwire-licensing-and-continuity-promise.md)).
- **Lean engine.** Fewer, more meaningful moving parts win over feature count. Scope
  exclusions in [ADR-0014](docs/adr/0014-v1-scope-exclusions.md) stand until an ADR
  supersedes them.

A request that fits the vision and comes with a concrete use case (especially from a
team running Ghostlight governed in anger) carries real weight; the quarterly founding
questionnaire exists precisely to harvest those.

## Contributing code

Contribution terms follow the open-core boundary (ADR-0027 Decision 5):

- **Engine** (everything outside `src/governance/`): contributions are accepted under
  the [Developer Certificate of Origin](https://developercertificate.org/); sign off
  your commits (`git commit -s`). Inbound = outbound under Apache-2.0 OR MIT.
- **Governance module** (`src/governance/`): contributions require a Contributor
  License Agreement (the module is distributed under a commercial license, and only the
  copyright holder can sell that). The CLA will be in place before the first outside
  governance PR is merged; if you want to contribute there, open a Discussion first and
  we will sort the paperwork.

Practical expectations for PRs: `cargo fmt --check`, `cargo clippy --all-targets -- -D
warnings`, and `cargo test` green; ASCII source (escapes for anything else); match the
surrounding code's style; and one logical change per PR. For anything larger than a
small fix, open a Discussion or Issue first so nobody builds the wrong thing.

## What not to report publicly

Suspected vulnerabilities go to hello@sylin.org with "SECURITY" in the subject, per
[SECURITY.md](SECURITY.md). Everything else is fair game in the open.
