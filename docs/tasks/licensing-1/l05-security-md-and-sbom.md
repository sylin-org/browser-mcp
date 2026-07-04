# L05: SECURITY.md and release SBOM

## Goal

Two procurement-pack artifacts (docs/business/PLAN.md "document pack"): a vulnerability
disclosure policy at the repo root, and a CycloneDX SBOM generated and uploaded with
every release artifact.

## Authority

ADR-0028 (context: procurement documentation); docs/business/PLAN.md section "The
procurement document pack"; 00-design.md "SECURITY.md and SBOM (l05)".

## Depends on

Nothing in this batch (independent of l01-l04; run even if they are BLOCKED). STOP
preconditions: SECURITY.md does not exist at the repo root;
`rg -n "cyclonedx" .github/workflows/release.yml` prints nothing;
`rg -n "if-no-files-found: error" .github/workflows/release.yml` matches exactly once.
If any fails, STOP.

## Current behavior (verified 2026-07-03; re-read before editing)

- No SECURITY.md anywhere.
- .github/workflows/release.yml has one job `build` (matrix over four targets) whose
  final step is `actions/upload-artifact@v4` with:

      - uses: actions/upload-artifact@v4
        with:
          name: ghostlight-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/ghostlight.exe
            target/${{ matrix.target }}/release/ghostlight
          if-no-files-found: error

## Required behavior

### 1. SECURITY.md (new, repo root; no SPDX header, it is a doc)

Exactly this content:

    # Security policy

    ## Reporting a vulnerability

    Email security@sylin.org. Do not open a public issue for a suspected vulnerability.

    - Acknowledgement within 48 hours.
    - Assessment and severity triage within 7 days.
    - Fix target for confirmed critical issues: 30 days, with a coordinated release.
    - You will be credited in the release notes unless you ask not to be.

    ## Scope

    The `ghostlight` binary, the bundled Chromium extension, and the install scripts in
    this repository. The reference/ directory is third-party study material and out of
    scope.

    ## What to expect from the product

    Ghostlight is a local-only tool: it never phones home, carries no telemetry, and
    initiates no network traffic beyond the user's own tool calls and configured audit
    destinations (ADR-0028 Decision 9). The extension holds no policy logic; enforcement
    and audit live in the binary (docs/SPEC.md). License state never changes behavior
    (ADR-0028 Decision 1).

    ## Supported versions

    The latest tagged release. Pre-1.0, fixes land on the tip; there are no backport
    branches.

### 2. Release SBOM (.github/workflows/release.yml; sole owner in this batch)

Insert BETWEEN the `- run: cargo build --release --target ${{ matrix.target }}` step and
the `- uses: actions/upload-artifact@v4` step, at the same indentation as the other
steps, exactly:

      - run: cargo install cargo-cyclonedx --locked
      - run: cargo cyclonedx --format json

and add ONE line to the upload step's `path:` block, after the two existing binary
lines, at the same indentation:

            ghostlight.cdx.json

## Constraints

Transcribe byte-for-byte; two-space YAML indentation, no tabs. Only SECURITY.md and
.github/workflows/release.yml change. Do not touch ci.yml. No em-dashes or smart quotes.

## Tests (from repo root)

- `rg -c "security@sylin.org" SECURITY.md` prints `1`.
- `rg -c "cargo cyclonedx --format json" .github/workflows/release.yml` prints `1`.
- `rg -c "ghostlight.cdx.json" .github/workflows/release.yml` prints `1`.
- `rg -n "\t" .github/workflows/release.yml` prints nothing.

## Verification

The rg assertions; `cargo test` unchanged (no compiled change; a spot-run of
`cargo test --test config_schema_golden` suffices); ASCII diff scan; ledger entry noting
that the SBOM steps are validated live on the first tagged release; commit.

Commit subject: `chore(security): SECURITY.md and CycloneDX SBOM on release artifacts`

## Out of scope

ci.yml; any dependency change; SBOM for the extension zip; signing the SBOM.
