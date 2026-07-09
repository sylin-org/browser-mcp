# T3 -- one extension host (ADR-0048 D5)

## Goal

The extension always connects to the single host name `org.sylin.ghostlight`; the browser-side
adapter behind it (T2) decides which service traffic reaches. The installType-based dev-host
selection (2026-07-08, cd77bf5) and the popup/options instance badges are removed -- with
adapter-side resolution a static extension-side label would lie about routing. Normative:
ADR-0048 D5 (supersession recorded in the ADR). Oracles: PINS.md P3.

## Files this task owns (touch nothing else)

- `extension/service-worker.js`
- `extension/popup.js`
- `extension/options.js`
- `docs/tasks/dev-override/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ 3928a74 -- re-read every anchor before editing)

- service-worker.js has the block starting at the comment
  `// Native-messaging host name. An unpacked/dev extension (installType "development") targets the`
  containing `NATIVE_HOST_DEFAULT`, `NATIVE_HOST_DEV`, `resolvedNativeHost`,
  `async function nativeHost()`, and `function boundInstance()`.
- `connect()` contains `const host = await nativeHost();` then
  `if (nativePort) return; // re-check: another caller may have won an await above` then
  `nativePort = chrome.runtime.connectNative(host);`.
- The `GET_SESSION_STATE` handler contains
  `await nativeHost(); // resolve the instance label before answering` and
  `instance: boundInstance(),`.
- popup.js `renderLinkDot` has the `state.instance` ternary title; `renderSession` has
  `const inst = state.instance ? ...` and the `Connected to Ghostlight${inst}.` template.
- options.js `renderLink` has the `const inst = ...` line and
  `linkText.textContent = ` + backtick `Connected${inst}` + backtick; `refreshLink`'s fallback
  object carries `instance: null`.
- popup.html/options.html carry the dot/pill MARKUP -- they are NOT edited (labels change in JS
  only).

## STOP preconditions

- STOP if any anchor above is absent or materially different.
- STOP if `grep -rn "org.sylin.ghostlight.dev" extension/` matches anything OTHER than
  service-worker.js's `NATIVE_HOST_DEV` line (an unexpected second dependency on the dev host).

## Changes (transcribe from PINS P3)

1. service-worker.js: replace the host-selection block with the pinned single `NATIVE_HOST`
   const; fix `connect()`; strip the two `GET_SESSION_STATE` lines.
2. popup.js: the two pinned label simplifications.
3. options.js: the two pinned label simplifications + the fallback-object field removal.
4. Run the pinned post-condition grep (P3): zero matches.

## Verification (all green)

```
node --check extension/service-worker.js
node --check extension/popup.js
node --check extension/options.js
node --test tests/extension/grouping.test.js
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

(The cargo commands are unaffected by this task but pin that the tree stays green.)

## Out of scope (fences)

- NO change to popup.html/options.html, the kill switch, the FX vocabulary, grouping, or any
  handler.
- NO change to the green-dot/pill STATE machinery (connected/waiting/killed states stay; only
  the instance SUFFIX goes).
- NO Rust changes.

## Commit

Stage exactly the three named files. Pinned message (PINS P3):

```
feat(extension): one native host -- adapter-side resolution replaces installType selection (ADR-0048 D5)
```

Then update LEDGER.md and commit as `docs(dev-override): ledger T3`.
