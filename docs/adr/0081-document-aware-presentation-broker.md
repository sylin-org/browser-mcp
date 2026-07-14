# ADR-0081: Document-aware Presentation Broker

Date: 2026-07-14
Status: Accepted
Builds on: ADR-0005 (policy-free extension), ADR-0012 (visible interaction), ADR-0072
(agent narration), ADR-0079 (denial attention), and ADR-0080 (separate presentation lane).
Amends ADR-0072's worker-local replay mechanism and ADR-0079's page-delivery mechanism. Preserves
the trained tool schemas, the service-owned governance boundary, and ADR-0028's no-phone-home
Continuity Promise.

## Context

Ghostlight has a coherent visual vocabulary, but its delivery mechanism is not yet a coherent
system. Narration, denial stickers, attention overlays, screenshot cues, navigation pills, action
effects, and capture hiding all call `chrome.tabs.sendMessage` directly. Each caller decides for
itself whether to retain state, replay after navigation, await a response, or ignore an error.

That arrangement fails at the exact boundary users care about. A tool can complete successfully
while its visible explanation disappears. The failure is especially easy to produce after an
unpacked-extension reload: the service worker is new, the already-open page did not navigate, and
its prior content-script receiver may no longer belong to the live extension generation. Chrome
accepting a worker call is not proof that the current page document rendered anything. The direct
send helpers swallow that distinction.

`tabs.onUpdated(status == complete)` is also not a document-ready protocol. It may fire before the
visual content script has registered its message listener, may not fire at all for an extension
reload on an unchanged page, and does not identify which document acknowledged a replay. Retrying
blindly can deliver a stale message into a replaced document or render a replacement twice.

ADR-0080 separated presentation traffic from page execution so narration cannot wait behind a
slow browser command. This ADR gives that separate lane its own lifecycle and delivery semantics.
It does not put presentation into the page FIFO and does not move policy into the extension.

## Decision

### D1. One policy-free Presentation Broker owns page-presentation delivery

The extension service worker owns one `PresentationBroker` application service. Tool handlers,
governance-message handlers, and browser-mechanism helpers publish presentation intents to it.
They do not call `chrome.tabs.sendMessage` directly for Ghostlight visual messages.

The broker owns only mechanism:

- the target native tab and current document identity;
- a named presentation channel and monotonic revision;
- replacement, deadline, and replay state;
- bounded transient-event queues;
- delivery attempts and exact acknowledgements;
- on-demand visual-script activation.

It does not authorize a tool, classify a denial, calculate attention thresholds, choose a grant,
write audit, inspect page content, or reinterpret service-provided text. Governance remains in the
Rust service. The page renderer remains a visual adapter. The broker is a pure JavaScript domain
module with injected Chrome delivery, activation, clock, and timer seams.

### D2. Presentation channels have explicit semantics

The initial channel vocabulary is:

| Channel | Shape | Replacement and replay |
| --- | --- | --- |
| `attention:<session>` | authoritative state | Retained until the service resolves it; replayed into every current document; never gated by visual-effect preferences |
| `notification` | substantive timed state | One per tab; replacement is immediate; retained only to its three-second deadline |
| `narration` | optional timed state | One per tab; replacement is immediate; retained only to its narration deadline |
| `effect` | bounded transient event | FIFO while the current document becomes ready; never replayed after a document change; expires quickly |
| `capture` | immediate barrier event | Bypasses ordinary effect backlog; an acknowledged hide is required before capture; a missing receiver means there is no live Ghostlight renderer to hide |

The extension action badge and popup remain the non-page renderer for recording, hold, and
attention status. Recording is deliberately not rendered inside the page because CDP screencast
would recursively record it. The broker does not invent a simulated picture-in-picture view.

Channels are presentation isolation, not browser-execution locks. Different tabs may deliver in
parallel. A channel replacement never blocks a page tool. Attention has visual priority in the
renderer but does not gain governance authority from the broker; the service already established
that authority before publishing the state.

### D3. The current document explicitly announces readiness

After the visual content script installs its listener, it sends a
`GHOSTLIGHT_PRESENTATION_READY` message to the service worker. Chrome supplies the sender tab id
and `documentId`; page-provided ids are never trusted.

The broker records exactly one ready document per tab. `tabs.onUpdated(status == loading)` marks
the prior document unavailable and retires its transient effects. A ready message carrying a new
Chrome document id performs the same transition even if the loading event was missed. Stateful
channels become due for replay in the new document. A stale ready signal never replaces a newer
ready document.

`status == complete` may prompt activation, but it is not delivery proof and does not itself
replay presentation.

### D4. Delivery requires an exact document and revision acknowledgement

Every broker delivery adds internal metadata containing the channel, revision, and target Chrome
document id. The renderer responds only after it has accepted and applied that exact message. The
response echoes the same metadata.

The broker treats delivery as complete only when all three values match. A successful Promise
from `chrome.tabs.sendMessage`, a response from an older renderer that lacks the metadata, or an
acknowledgement from a replaced document is not completion.

State may be delivered once per document revision. Transient effects are removed after their
exact acknowledgement. Duplicate acknowledgement is harmless. Replacement retires the old
revision and an old acknowledgement cannot satisfy the new one.

The `narrate` tool uses this boundary for its truthful `shown` result. Fire-and-forget callers do
not wait for an MCP result, but they still use the same acknowledged broker path.

### D5. Missing receivers trigger bounded on-demand activation

When a presentation is due and the tab has no ready document, the worker asks Chrome to inject the
committed visual scripts into that tab. This is the same packaged extension code declared in the
manifest, not remote code. Concurrent activation requests for one tab coalesce.

The injected renderer announces readiness after its listener exists. If the page already has the
current renderer, reinjection only repeats the ready announcement. If an extension reload left
stale Ghostlight DOM, the new renderer removes the old Ghostlight-owned roots before installing
its own state.

Restricted browser pages may refuse injection. The broker reports the visual layer as unavailable
to a waiting narration call and lets transient events expire. Browser execution remains truthful
and usable; presentation failure never masquerades as browser-action failure. Stateful narration
may still render with its remaining duration if the tab later navigates to an eligible document.

Activation and event queues are bounded. There is no retry loop on an ineligible page and no
unbounded collection of visual payloads.

### D6. Navigation separates state from events

Stateful channels describe what remains true and therefore replay into a new document:

- an unresolved attention pause;
- an unexpired denial notification;
- an unexpired narration.

Transient action effects describe what happened in one document and do not cross navigation. A
click ripple, field glow, read scan, or screenshot cue queued for document A is retired when
document B begins. A navigation pill published after document B becomes current belongs to B and
may wait briefly for B's ready announcement.

Clear is a revisioned state transition. It removes retained state and delivers the matching clear
message to the current document when one exists. A later document starts clear without needing a
replayed clear event.

### D7. Capture hiding is a separate immediate barrier

The capture channel is not an ordinary decorative event. `HIDE_FOR_TOOL_USE` is delivered directly
to the current acknowledged document and awaited before screenshot or zoom capture. The renderer
acknowledges only after every Ghostlight-owned page layer is hidden. `SHOW_AFTER_TOOL_USE` restores
the current state after capture.

If no renderer is present, there is no live Ghostlight page layer from the current extension
generation to hide. Activation is not required merely to take a clean screenshot. Stale roots from
an invalidated extension generation are removed during the next activation.

This ADR does not solve the separate screencast recursion problem by drawing an in-page REC badge.
Recording truth stays in extension chrome and the popup as ADR-0079 requires.

### D8. State is bounded and browser-session local

The broker retains at most a named number of tabs, state records per tab, transient events per
tab, and total estimated payload bytes. Overflow retires the oldest optional event before refusing
new optional events. Authoritative attention state is never silently evicted; if its bounded slot
cannot be represented, the popup remains the recovery surface and the worker records a local
payload-free diagnostic.

Narration text, notification text, and attention labels remain only in memory and
`chrome.storage.session`. They are never written to `storage.local`, synced, logged, audited,
uploaded, or sent to a server. `storage.session` lets active state survive a Manifest V3 worker
restart within one browser process and is cleared by a browser restart. Expired state is discarded
during restore. Transient effects and capture messages are never persisted.

Tab close, session kill, service-directed clear, and browser-session end erase applicable state.
Persisted snapshots contain only the minimum active presentation records and deadlines.

### D9. The page renderer remains one visual adapter

`agent-visual-indicator.js` keeps ownership of DOM, CSS, reduced-motion behavior, pointer
transparency, focus handling, and the established visual vocabulary. The broker never generates
HTML or CSS.

The renderer has one presentation-envelope entry point that routes to the existing named visual
functions. Wire text still enters DOM through `textContent`. Ghostlight-owned nodes remain
`ghostlight-` prefixed, excluded from reads, and hidden during capture. Governance presentation
continues to render when decorative effects are disabled.

Existing direct `AGENT_*` message handling remains temporarily additive for content-script version
skew during reload. New worker code uses the acknowledged envelope. A later cleanup may remove the
compatibility branches only after supported reload paths cannot leave an older live renderer.

### D10. Diagnostics are local, bounded, and content-free

Broker diagnostics may record channel, tab identity, opaque document identity, revision, queue
depth, delivery state, retry reason, and timing. They must not record narration text, denial text,
page content, form values, screenshots, recording frames, or full URLs. No presentation metric
phones home.

## Acceptance criteria

1. Pure tests prove state replacement, stale-ack immunity, exact-document delivery, deadline
   expiry, navigation replay, transient-event retirement, capture bypass, queue bounds, and tab
   cleanup.
2. A content-script ready handshake is emitted only after the presentation listener exists and the
   worker derives tab/document identity from Chrome's sender metadata.
3. A live extension reload on an unchanged HTTP page is followed by successful on-demand renderer
   activation without requiring page navigation.
4. Narration returns `shown: true` only after the exact revision is acknowledged; a restricted
   page returns a truthful not-shown reason without blocking browser work.
5. Denial notifications and attention overlays are broker state, survive document replacement
   within their lifetimes, and remain independent from the decorative-effects preference.
6. Action effects use the broker event path and do not replay into a different document.
7. Screenshot and zoom captures await the capture-hide acknowledgement and never include current
   Ghostlight page layers.
8. Active state survives a service-worker restart through bounded `storage.session` state;
   transient effects do not.
9. The extension remains policy-free, no trained tool schema changes, no remote code or telemetry
   is added, and the full Rust, Lightbox, and extension gates remain green.
10. Visible Chrome verification covers narration during a slow page command, navigation followed
    by an effect, extension reload on a stable page, screenshot feedback, and denial presentation.

## Consequences

- Presentation becomes a named subsystem with a truthful delivery boundary instead of scattered
  best-effort sends.
- Extension reload and document navigation become ordinary lifecycle transitions rather than
  special cases each visual feature must rediscover.
- Stateful presentation replays; document-local effects do not. That distinction is explicit and
  testable.
- `narrate` may wait briefly for renderer activation before reporting whether it was shown. It
  still bypasses the browser page FIFO and does not delay the browser command already in flight.
- The worker retains a small bounded amount of active presentation text in browser-session memory.
  This is the minimum cost of replay across worker restart and is narrower than persistent local
  storage.
- Restricted pages remain incapable of hosting page UI. The popup and extension badge continue to
  provide trusted recovery and recording state outside the page.

## Rejected alternatives

- Keep direct `tabs.sendMessage` calls and add retries at each caller. Rejected because every
  visual feature would continue to invent document, replacement, and expiry semantics.
- Treat `tabs.onUpdated(complete)` as readiness. Rejected because it neither proves listener
  installation nor identifies the acknowledging document.
- Put the broker in the Rust service. Rejected because Chrome document identity, content-script
  activation, and page-message acknowledgements are extension mechanism. Policy and semantic
  authority remain in Rust.
- Put governance thresholds or grant choices in the broker. Rejected because presentation cannot
  become authorization.
- Replay every action effect after navigation. Rejected because a click or scan from the old page
  would misdescribe the new page.
- Draw an in-page REC or picture-in-picture preview. Rejected because the screencast would record
  its own indicator and imply an independent capture surface that does not exist.
- Persist presentation in `storage.local` or sync storage. Rejected because active workflow text
  does not need disk or cross-device retention.
- Use a global presentation FIFO. Rejected because different tabs and channels do not share one
  visual consistency boundary, and capture barriers must not wait behind decoration.

## Amendment: capture activates an unknown renderer generation

Implementation review found one case D7's original wording did not cover: an extension reload can
invalidate the old isolated world while leaving its Ghostlight-owned DOM roots in the page. A
capture with no ready current-generation renderer therefore cannot assume there is nothing to
hide. Before capture, the broker makes one bounded activation attempt and waits for the ready
handshake. The new renderer removes stale roots during activation, then acknowledges the hide
barrier. Restricted pages still degrade truthfully after the bounded attempt. No capture waits on
an unbounded retry loop.

## Provenance

On 2026-07-14, live testing proved that browser commands still worked while all page signage could
disappear after extension changes. The owner rejected a spot fix and directed a reliable,
efficient, DDD/SoC architecture. After ADR-0080 established per-resource command scheduling and a
separate presentation lane, the owner fully authorized the document-aware Presentation Broker and
its implementation.
