# W8: SBOM in release CI + security-insights.yml + SECURITY.md alignment (ADR-0057 D12)

## Goal
The three adopted artifacts that are code/config rather than prose. The ONLY task allowed to touch
files outside docs/ (BOOTSTRAP NEVER list names them).

## Preconditions (verify, else STOP)
- `.github/workflows/release.yml` exists; read its job/step structure fully before editing.
- `SECURITY.md` exists; read it fully.
- Confirm a cargo SBOM tool is installable in CI: the pinned choice is `cargo-cyclonedx`
  (`cargo install cargo-cyclonedx --locked`). Do NOT install locally; CI installs it.

## Required change 1: release.yml SBOM step
Add to the release workflow, in a job that runs once per release (not per target if a
matrix exists -- pick the packaging/checksum job; if none is obviously once-per-release, STOP and
record BLOCKED): a step that installs cargo-cyclonedx and runs
`cargo cyclonedx --format json --override-filename ghostlight-v${VERSION}-sbom.cyclonedx` (adapt
the version variable to what the workflow already uses; re-use its existing variable, do not
invent a new one), then uploads the resulting `.cyclonedx.json` as a release asset alongside the
existing ones. Mirror the workflow's existing step style (names, action versions).

## Required change 2: `security-insights.yml` at the repo root
OpenSSF Security Insights v2 shape (header: `header:` with `schema-version`, `last-updated`,
`last-reviewed`, `url`); fill honestly: project name, repo URL, vulnerability reporting via
SECURITY.md + support contact, no bug-bounty, distribution points (GitHub releases, npm), SBOM
present-as-of-this-batch, license split. Keep to fields you can state truthfully; omit optional
fields rather than guessing. Validate YAML parses: `python -c "import yaml,sys;yaml.safe_load(open('security-insights.yml'))"`
(if python/yaml is unavailable, `cargo` is irrelevant -- use any available YAML check and record
which; a clean `rg -n "schema-version" security-insights.yml` plus careful indentation review is
the fallback, recorded as a deviation).

## Required change 3: SECURITY.md alignment
Do NOT rewrite. Append/adjust minimally so it: names the private reporting channel; states the
no-bounty fact in the absences register (a fact with a reason); commits to the advisory window
(same 3-business-day acknowledgment as support, security-critical advisories on confirmed
vendor-side compromise); links docs/trust/security-overview.md. Preserve all existing content
unless it contradicts these (record any contradiction as a deviation with the original text).

## Verification (literal)
- `rg -n "cyclonedx" .github/workflows/release.yml` -> >=1.
- `CARGO_TARGET_DIR="$TEMP/gl-trust1-ct" cargo build --workspace` -> green (workflow edit cannot
  break the build, this is the belt-and-suspenders gate).
- `rg -n "schema-version" security-insights.yml` -> 1.
- `rg -n "docs/trust/security-overview.md" SECURITY.md` -> >=1.
- Global verification per BOOTSTRAP.

## Commit message (pinned)
`chore(trust): W8 release SBOM, OpenSSF security-insights, SECURITY.md alignment (ADR-0057 D12)`
