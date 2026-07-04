# L05: release SBOM

AMENDED 2026-07-04 (by the batch author, before execution): SECURITY.md was created
directly in the public-content pass and is now NEVER-TOUCH for this batch. This task is
SBOM-only.

## Goal

One procurement-pack artifact (docs/business/PLAN.md "document pack"): a CycloneDX SBOM
generated and uploaded with every release artifact.

## Authority

ADR-0028 (context: procurement documentation); docs/business/PLAN.md "The procurement
document pack"; 00-design.md "SBOM (l05)".

## Depends on

Nothing in this batch (independent of l01-l04; run even if they are BLOCKED). STOP
preconditions: `rg -n "cyclonedx" .github/workflows/release.yml` prints nothing;
`rg -n "if-no-files-found: error" .github/workflows/release.yml` matches exactly once.
If either fails, STOP. (SECURITY.md EXISTS at the repo root; that is expected and is not
a precondition failure. Do not edit it.)

## Current behavior (verified 2026-07-04; re-read before editing)

- SECURITY.md exists at the repo root (created outside this batch; never-touch).
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

### Release SBOM (.github/workflows/release.yml; sole owner in this batch)

Insert BETWEEN the `- run: cargo build --release --target ${{ matrix.target }}` step and
the `- uses: actions/upload-artifact@v4` step, at the same indentation as the other
steps, exactly:

      - run: cargo install cargo-cyclonedx --locked
      - run: cargo cyclonedx --format json

and add ONE line to the upload step's `path:` block, after the two existing binary
lines, at the same indentation:

            ghostlight.cdx.json

## Constraints

Transcribe byte-for-byte; two-space YAML indentation, no tabs. Only
.github/workflows/release.yml changes. Do not touch ci.yml or SECURITY.md. No em-dashes
or smart quotes.

## Tests (from repo root)

- `rg -c "cargo cyclonedx --format json" .github/workflows/release.yml` prints `1`.
- `rg -c "ghostlight.cdx.json" .github/workflows/release.yml` prints `1`.
- `rg -n "\t" .github/workflows/release.yml` prints nothing.

## Verification

The rg assertions; `cargo test` unchanged (no compiled change; a spot-run of
`cargo test --test config_schema_golden` suffices); ASCII diff scan; ledger entry noting
that the SBOM steps are validated live on the first tagged release; commit.

Commit subject: `chore(release): CycloneDX SBOM on release artifacts`

## Out of scope

SECURITY.md (exists; never-touch); ci.yml; any dependency change; SBOM for the extension
zip; signing the SBOM.
