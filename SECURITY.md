# Security policy

## Reporting a vulnerability

Email hello@sylin.org with "SECURITY" in the subject. Do not open a public issue for a
suspected vulnerability.

- Acknowledgement within 48 hours.
- Assessment and severity triage within 7 days.
- Fix target for confirmed critical issues: 30 days, with a coordinated release.
- You will be credited in the release notes unless you ask not to be.

## Scope

The `ghostlight` binary, the bundled Chromium extension, and the install scripts in this
repository. The reference/ directory is third-party study material and out of scope.

## What to expect from the product

Ghostlight is a local-only tool: it never phones home, carries no telemetry, and
initiates no network traffic beyond the user's own tool calls and configured audit
destinations (ADR-0028 Decision 9). The extension holds no policy logic; enforcement and
audit live in the binary (docs/SPEC.md). License state never changes behavior (ADR-0028
Decision 1).

## Supported versions

The latest tagged release. Pre-1.0, fixes land on the tip; there are no backport
branches.
