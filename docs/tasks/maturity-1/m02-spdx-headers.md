# M02: per-file SPDX license headers

## Goal

Make the ADR-0027 license boundary machine-readable per file: every Rust and
extension JavaScript source file carries an SPDX header on line 1, so license
scanners classify the engine and the governance module correctly.

## Authority

ADR-0027 Decision 4; 00-design.md "SPDX headers (m02)".

## Depends on

Nothing. STOP precondition: `rg -l "SPDX-License-Identifier" src/ tests/
extension/` prints nothing (no headers exist yet), and the repo-root LICENSE
file exists. If either fails, STOP.

## Current behavior (verified 2026-07-03; re-read before editing)

- 50 .rs files under src/, of which 21 are under src/governance/.
- 16 .rs files under tests/ (one of which, tests/tool_schema_fidelity.rs, is
  never-touch and is EXCLUDED from this task).
- 4 .js files under extension/ (service-worker.js, content.js,
  agent-visual-indicator.js, popup.js).
- Zero SPDX lines anywhere in the repo.
- Many .rs files begin with `//!` module docs; a `//` comment line above them
  is valid Rust.

Re-count before editing (`Get-ChildItem -Recurse -Filter *.rs` or
`rg --files -g "*.rs" src/ tests/ | wc -l` under Git Bash); if the counts
differ from the above, proceed with the actual counts and record the
difference as a numbered deviation.

## Required behavior

Insert as LINE 1 of each in-scope file (existing content shifts down one
line; add no blank line unless the file previously started with code on
line 1, in which case keep exactly the header then the original content):

- Files under src/governance/ (recursive):
  `// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial`
- All other .rs files under src/ and tests/ (EXCEPT
  tests/tool_schema_fidelity.rs, which is not modified at all):
  `// SPDX-License-Identifier: Apache-2.0 OR MIT`
- The 4 .js files DIRECTLY under extension/ (service-worker.js, content.js,
  agent-visual-indicator.js, popup.js): `// SPDX-License-Identifier: Apache-2.0
  OR MIT`. extension/ has an icons/ subdirectory, but it holds no .js files, so
  the .js scope is exactly these four.

No other content changes in any file.

## Constraints

Pure line-1 insertions. The classification rule is the PATH, nothing else.
ASCII only. Do not touch .ps1, .json, .md, .html, or anything under
reference/ or docs/.

## Tests (run under Git Bash from repo root)

- `rg -l --no-ignore "SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial" src/ | wc -l`
  prints `21` (or the re-counted governance file count; record deviation).
- `rg -l "SPDX-License-Identifier: Apache-2.0 OR MIT" src/ tests/ | wc -l`
  prints `44` (29 engine src + 15 tests; adjust to re-count).
- `rg -l "SPDX-License-Identifier" extension/ | wc -l` prints `4`.
- `rg -c "SPDX-License-Identifier" tests/tool_schema_fidelity.rs` prints
  nothing (exit 1: the file has no header).
- Every match is on line 1: `rg -n "SPDX-License-Identifier" src/ tests/
  extension/ | rg -v ":1:"` prints nothing.

## Verification

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
all green (comment-only change; test count unchanged from baseline). The rg
assertions above. ASCII diff scan. Ledger entry; commit.

Commit subject: `chore(spdx): per-file SPDX headers on the ADR-0027 license boundary`

## Out of scope

License file content; Cargo.toml; any non-.rs/.js file; tests/tool_schema_fidelity.rs;
reordering or reformatting anything below line 1.
