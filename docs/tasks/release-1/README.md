# Release 1 hardening: implementation prompts

Eighteen self-contained implementation prompts, one per task, decided in
[ADR-0017](../../adr/0017-release-1-engine-hardening.md) (the bounded-wait
constant anticipates [ADR-0019](../../adr/0019-layered-configuration-model.md)).
Each prompt is written for delegation to a smaller model: it carries the full
project context, verified current-behavior facts, exact required behavior with
message formats, the non-negotiable constraints, verification steps, and an
explicit out-of-scope fence. Hand a model exactly one file and the repository;
nothing else is needed.

Every prompt's file and line anchors were verified against the repository at
authoring time. Line numbers drift as sibling tasks land in the same file:
trust function names and prose over line numbers, and re-read the target file
before editing.

## Tasks

| Task | Title | Touches |
|---|---|---|
| [T01](t01-read-page-structural-pagination.md) | read_page structural pagination with element and char caps | content.js |
| [T02](t02-read-page-viewport-culling.md) | read_page viewport culling for filter=interactive | content.js |
| [T03](t03-get-page-text-official-semantics.md) | get_page_text largest-candidate innerText, Source header, max_chars | content.js |
| [T04](t04-first-call-warmup-bounded-wait.md) | Extension-channel warmup at initialize + bounded first-call wait | Rust binary |
| [T05](t05-sw-state-recovery.md) | Service-worker death recovery (rehydrate tab group, reattach lazily) | service-worker.js |
| [T06](t06-hop-attributed-errors.md) | Hop-attributed error reporting across the dispatch path | Rust binary + service-worker.js |
| [T07](t07-doctor-subcommand.md) | Extend the installer doctor with runtime/debug-state fusion | Rust binary |
| [T08](t08-type-real-key-events.md) | computer type via real keyDown/keyUp with Enter mapping | service-worker.js |
| [T09](t09-mouse-click-fidelity.md) | Incrementing clickCount sequence, buttons bitmask, force | service-worker.js |
| [T10](t10-scroll-verify-fallback.md) | Scroll effectiveness verification + scrollable-ancestor fallback | service-worker.js |
| [T11](t11-zoom-region-crop.md) | Real zoom region crop with coordinate-context update | service-worker.js |
| [T12](t12-per-domain-buffer-reset.md) | Console/network buffers reset on same-tab domain change | service-worker.js |
| [T13](t13-exception-thrown-capture.md) | Runtime.exceptionThrown as console exception entries | service-worker.js |
| [T14](t14-loading-failed-status.md) | Network.loadingFailed marks requests failed | service-worker.js |
| [T15](t15-empty-result-guidance-notes.md) | Empty-result guidance notes for console/network reads | service-worker.js |
| [T16](t16-javascript-tool-repl-and-cap.md) | javascript_tool REPL semantics + 50KB output cap | service-worker.js |
| [T17](t17-tabid-fallback-valid-ids.md) | Effective-tabId fallback + valid-ID error listing | service-worker.js |
| [T18](t18-background-tab-screenshot-clip.md) | Non-visible tab screenshots via clip+scale single pass | service-worker.js |

## Execution order

Tasks in different streams are independent and can run in parallel worktrees;
tasks within a stream share a file and run one at a time, in order.

- Stream A (Rust binary): T04, then T06, then T07.
- Stream B (content.js): T01, then T02, then T03.
- Stream C (service-worker reads): T12, then T13, then T14, then T15.
- Stream D (service-worker input): T08, then T09, then T10.
- Stream E (service-worker screenshots): T11, then T18.
- Stream F (service-worker misc): T16, then T17.
- T05 (state recovery) touches the whole service worker: run it last, after
  streams C through F.

T06 spans the binary and the extension's error responses; land its binary half
after T04 and its extension half before the C through F streams finish, or
re-verify anchors.

After each task: `cargo test` must pass (including
`tests/tool_schema_fidelity.rs`, always unchanged), extension changes need a
reload at chrome://extensions, and binary changes need an MCP client restart.
