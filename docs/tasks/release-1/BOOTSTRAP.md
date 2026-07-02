# Bootstrap: unattended execution of the release-1 task prompts

You are an autonomous implementation agent working overnight, unattended, in
this repository. Your job is to execute the release-1 hardening tasks in
docs/tasks/release-1/ one at a time, in the exact sequence below, fully
implementing each prompt including its tests, while keeping durable written
records so that a context wipe never loses work.

Read this whole file before doing anything.

## Ground rules

1. Your context may be compacted or reset AT ANY TIME. The files
   docs/tasks/release-1/LEDGER.md and docs/tasks/release-1/BROWSER-TESTS.md
   are your memory. At the start of every task, after any interruption, and
   whenever you are unsure of your state: read LEDGER.md (the RESUME HERE
   section first), then the task prompt you are on, then continue. Never rely
   on remembering earlier work; re-read files.
2. There is NO human available. Never ask questions; never wait for input.
   Make the conservative choice, record it in the ledger, and continue.
3. There is NO live browser available. You cannot reload the extension or
   click anything in Chrome. Every verification step that needs a real
   browser is DEFERRED: write it into BROWSER-TESTS.md (protocol below)
   instead of attempting it.
4. Work on the branch release-1-hardening. Create it from main if it does not
   exist. Never push. Never merge. Never commit to main.
5. One task = one commit. The commit includes the code, its tests, and the
   ledger/browser-tests updates for that task. Message format:
   `feat(<area>): T<NN> <short title>` (use fix/refactor if more accurate).
6. Never modify src/mcp/schemas/tools.json. tests/tool_schema_fidelity.rs
   must pass after every task. If a change you made breaks it, your change is
   wrong; revert and rethink.
7. ASCII only in everything you write (code, tests, docs, ledger entries):
   no em-dashes, no arrows, no curly quotes.
8. Never leave the tree dirty between tasks. Commit it or revert it.

## Environment facts

- Windows 11. Shell: prefer bash-compatible commands; PowerShell also works.
- Repository root: this repo (you are already in it).
- Build/test: `cargo test` from the repo root runs everything. Also run
  `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
  (fix formatting with `cargo fmt` before committing).
- If target/debug/browser-mcp.exe is locked by a running session, rename it
  aside and rebuild: `mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`
- The extension is vanilla JS with no test harness. For every touched JS file
  run a syntax check: `node --check extension/service-worker.js` (and each
  other touched .js file). Do NOT introduce a JS test framework or any new
  dependency.
- ASCII scan for files you created or edited (run before each commit):
  `python -c "import sys;[print(f,[c for c in open(f,encoding='utf-8').read() if ord(c)>127][:5]) for f in sys.argv[1:]]" <files>`
  Any output other than empty lists means you must fix the file.

## Task sequence

Execute in exactly this order (it groups tasks by file to limit drift, per
docs/tasks/release-1/README.md; T05 runs last because it touches the whole
service worker):

T04, T06, T07, T01, T02, T03, T12, T13, T14, T15, T08, T09, T10, T11, T18, T16, T17, T05

Prompt files are docs/tasks/release-1/t<nn>-<slug>.md. Each prompt is
self-contained: Goal, Project context, Current behavior, Required behavior,
Constraints, Verification, Out of scope. Line numbers inside prompts were
verified at authoring time and DRIFT as earlier tasks land: trust function
names and prose over line numbers, and always re-read the target file before
editing. Respect every Out of scope section literally.

## Per-task procedure

For each task, in order:

1. Read LEDGER.md. Update the RESUME HERE block to name this task and set its
   row to in_progress with a timestamp. Commit nothing yet.
2. Read the full task prompt. Read the target files it names. Reconcile any
   drift between the prompt's Current behavior and the actual code (earlier
   tasks may have moved things); note reconciliations in the ledger entry.
3. Implement the Required behavior completely. Follow the prompt's
   Constraints section without exception.
4. Implement the tests the prompt's Verification section calls for:
   - Rust: unit tests inline, integration tests in tests/. They must run and
     pass in `cargo test`.
   - Extension JS: there is no JS harness. Verify with `node --check`, by
     careful re-reading of your diff against the Required behavior, and by
     writing the deferred browser checks (step 5). Do not fake a harness.
5. For every Verification step that needs a live browser (reloading the
   extension, clicking, screenshots, real pages): append a section to
   BROWSER-TESTS.md following its format: task id, what changed, exact
   step-by-step instructions a human can follow in the morning, and the
   expected result for each step. Be specific about URLs and elements.
6. Quality gate, all must pass before committing:
   a. `cargo test` (all, including tool_schema_fidelity)
   b. `cargo clippy --all-targets -- -D warnings`
   c. `cargo fmt --check` (after `cargo fmt` if needed)
   d. `node --check` on every touched .js file
   e. ASCII scan on every file you created or edited
7. Update LEDGER.md: set the task row to done; append a task log entry
   (template in the ledger) recording files touched, tests added, drift
   reconciled, decisions made, and anything the NEXT tasks need to know.
   Update RESUME HERE to point at the next task.
8. `git add` your changes plus LEDGER.md and BROWSER-TESTS.md; commit with
   the message format above. Verify `git status` is clean.

## Failure protocol

If a task cannot be completed (tests will not pass, the prompt contradicts
the code in a way you cannot reconcile, a constraint would be violated):

1. Attempt at most TWO distinct approaches. Do not thrash.
2. Then: revert all uncommitted changes for this task
   (`git checkout -- .` and remove new untracked files you created for it),
   so the tree is clean.
3. Mark the task blocked in LEDGER.md with a precise diagnosis: what you
   tried, exactly where it failed, what a human should decide. Update
   RESUME HERE to the next task.
4. Commit the ledger update alone: `chore(ledger): T<NN> blocked`.
5. Continue with the next task UNLESS it depends on the blocked one (the
   ledger's sequence table marks dependencies); skip dependents to their own
   blocked state with reason `depends on T<NN>`.

If `cargo test` fails BEFORE you change anything (pre-existing breakage):
record it in the ledger, attempt one reasonable fix if it is trivial,
otherwise mark the run blocked-at-baseline and stop the whole run with a
clear ledger entry. Do not build on a red baseline.

## Completion

When all 18 tasks are done or blocked:

1. Ensure the tree is clean and every task row has a final state.
2. Write the RUN SUMMARY section at the top of LEDGER.md: tasks done, tasks
   blocked and why, total commits, anything the human must decide in the
   morning, and a reminder list: restart the MCP client, reload the extension
   at chrome://extensions, then run BROWSER-TESTS.md top to bottom.
3. Commit `chore(ledger): run summary`.
4. Stop. Do NOT continue into docs/tasks/stage-2/ (governance is a separate
   staged run by explicit project decision, ADR-0018).
