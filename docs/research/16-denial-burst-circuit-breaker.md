# Denial bursts and an attention circuit breaker

**Date:** 2026-07-14
**Status:** Prior-art study and ADR input. No implementation is authorized by this document.

## Question

How should Ghostlight report isolated governance denials quietly, then protect the user when a
runaway or malicious MCP client repeatedly attempts denied actions?

The desired behavior has two different jobs:

- preserve the policy decision while spending little user attention on an isolated denial; and
- stop an abnormal burst before it becomes an endless stream of calls and notices.

Those jobs must not collapse into a single "dismiss" control. Hiding an explanation is not granting
the denied capability.

## Current behavior and gap

The current extension shows a persistent, center-screen governance ribbon. It is pointer-transparent
except for its close button. Only the current tool call is denied. A later mutating call may proceed
and clears the prior ribbon; a later read-only call does not.

This creates a semantic mismatch. The ribbon looks like a modal safety state, but the service has
not latched the session. Conversely, turning every isolated denial into a real pause would make
normal policy boundaries needlessly disruptive.

## Prior art

### Quiet browser intervention with a separate policy control

Chrome's pop-up blocker suppresses the event, leaves a compact address-bar indication, and lets the
user inspect it. A site-scoped "always allow" decision is a separate explicit action. The useful
pattern is quiet enforcement plus a discoverable control surface. Ghostlight should not copy the
permission override: its notice controls must never rewrite governance policy.

Source: [Chrome pop-up controls](https://support.google.com/chrome/answer/95472?co=GENIE.Platform%3DDesktop&hl=en)

Chrome's one-time permissions make scope and lifetime explicit: temporary and persistent choices
are separate, and site controls remain available after the prompt. The lesson is to label session
and site scope precisely. It does not justify a user-side override of a Ghostlight denial.

Source: [One-time permissions in Chrome](https://developer.chrome.com/blog/one-time-permissions)

### A blocking surface should represent actual restricted state

VS Code Workspace Trust enters Restricted Mode, actually disables or limits risky features, and
keeps both a banner and a durable status indicator. Its management UI makes the scope of a trust
decision explicit. The important Ghostlight lesson is correspondence: if an overlay says the
agent is paused, dispatch must already be paused in the service.

Source: [VS Code Workspace Trust](https://code.visualstudio.com/docs/editing/workspaces/workspace-trust)

### Repetition can be cooled without deleting evidence

Android's notification cooldown reduces the prominence of rapid repeated notifications while they
remain available in notification history. Critical notifications can remain exempt. Ghostlight can
similarly collapse identical visible denials while retaining every decision in the audit stream and
allowing materially different or higher-severity events to escalate.

Source: [Android notification cooldown](https://developer.android.com/develop/ui/compose/notifications)

### Circuit breakers need explicit state, counters, and recovery

The Azure Circuit Breaker pattern uses closed, open, and half-open states, with a failure threshold
inside a time window and observable transitions. The time window avoids treating old failures as a
current burst. Ghostlight is protecting human attention and browser control, not probing a remote
service, so automatic half-open trial calls are the wrong recovery behavior. A human resume action
is the safer equivalent.

Sources: [Azure Circuit Breaker pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/circuit-breaker),
[Azure transient-fault guidance](https://learn.microsoft.com/en-us/azure/architecture/best-practices/transient-faults)

Fail2Ban supplies the closest common vocabulary: a maximum retry count within a find-time window
causes a bounded ban. Its operational defaults are not suitable UX defaults for Ghostlight. The
pattern is useful; the numbers are not.

Source: [Fail2Ban jail configuration](https://github.com/fail2ban/fail2ban/blob/master/config/jail.conf)

### Lockout mechanisms can become denial-of-service mechanisms

OWASP's account-lockout guidance separates threshold, observation window, and lockout duration,
binds counters to the protected identity rather than an easily rotated source, and warns that an
attacker can intentionally trigger lockouts. For Ghostlight, the protected identity is the admitted
MCP session. Counting only by site would let a client rotate sites; counting globally would let one
client pause unrelated sessions.

Source: [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)

## Proposed model for an ADR

### State machine

`Quiet` is the normal state. An enforced denial returns the normal structured denial to the MCP
client and shows one short, non-blocking user notice. The service also updates memory-only counters.

`AttentionRequired` is entered when a configured burst rule is met. The transition is committed in
the service before the extension is asked to render it. While this state holds, the service rejects
new browser-dispatch work from that MCP session. The human browser itself remains usable. The state
ends only through an explicit user recovery control, session termination, panic, or service exit.

There is no automatic half-open state. Ghostlight must not send a trial action into the user's real
browser merely to see whether a client has calmed down.

### Count scope

Maintain two rolling, session-bound counters:

1. A matching-signature counter keyed by admitted session, top-level origin, capability, and denial
   category. This catches a client hammering the same boundary.
2. A session-wide counter across enforced denials. This catches origin or capability rotation.

Do not aggregate across MCP sessions. A malicious client should be able to pause only its own
browser authority. Native tab id alone is not an identity and must not own a counter.

Only actual enforced governance denials count. Exclude observe/shadow decisions, dry runs,
validation errors, browser failures, ownership conflicts, calls rejected because the pause is
already active, and user-generated control actions. Sacred-surface denials may deserve a distinct
severity, but any immediate-pause rule needs separate evidence and review.

### Candidate thresholds for evaluation, not defaults

A useful first live-test matrix is:

- matching signature: 3 denials in 60 seconds; or
- any enforced denial in the same session: 5 denials in 120 seconds.

These numbers are hypotheses chosen to make evaluation observable, not product defaults. Test them
against legitimate workflows, scripted retry behavior, site rotation, slow repeated errors, and a
deliberately hostile loop. The eventual values should be named constants, versioned in audit facts,
and tunable in dev/test builds before any public commitment.

### Ordinary denial notice

An isolated denial should render as a centered, pointer-transparent sticker for about three
seconds. A new notice replaces the current one. It contains:

- a precise title, such as `Write blocked`;
- one short reason that does not expose page content;
- a shield or category icon; and
- an affordance to open details outside the page layer, if one exists.

The service still returns the structured denial to the client and audits it. Closing or letting the
sticker expire changes no authority.

### Escalated pause

The overlay may say `Agent paused after repeated denied actions` only after the service has entered
`AttentionRequired`. It should dim or soften the page and expose a small set of unambiguous human
controls:

- `Keep paused`: preserve the latch and minimize the overlay to a persistent shield indicator.
- `Resume agent`: clear the latch and burst counters for this session.
- `Resume and quiet repeats for this site`: clear the latch and collapse identical notices for
  this origin and denial signature for the rest of this session. Enforcement and audit continue.
- `End session`: revoke this MCP session's browser ownership and connection.

The site-scoped quiet option is an attention preference, not an allow rule. A materially different
denial still appears and contributes to the session-wide counter. Identical quieted denials remain
audited but do not reopen the overlay during that explicit session-scoped suppression. The UI must
avoid verbs such as `Allow`, `Trust`, or `Approve` unless a future policy design actually grants
authority.

### Recovery without a working page renderer

The service state is authoritative. If the extension cannot render the overlay, the session must
remain paused. A recovery path must also exist in trusted Ghostlight chrome, such as the extension
popup or a local status/control surface. Otherwise a renderer failure can strand a legitimate
session with no clear recovery.

The service must accept only an authenticated, session-bound disposition message. The extension
relays the human's choice but owns no threshold, classification, policy, or audit decision.

### Audit and privacy

Each state transition should record content-free facts:

- session GUID and client identity already admitted by Ghostlight;
- denial category and capability, not page text, selectors, values, or model arguments;
- threshold rule/version, count, and window;
- `Quiet -> AttentionRequired` and recovery transition; and
- the selected recovery disposition and scope.

Collapsed notices remain separately audited as the underlying denials. The audit record must not
imply that a quieted notice was an allowed action.

## Threat boundary

This mechanism constrains a runaway or malicious MCP client while the Ghostlight service remains
trusted. It does not protect against a compromised Ghostlight service, extension, browser, or OS
user account. That limit should be stated wherever the feature is described.

## Failure modes to test

- Two clients act concurrently; one crosses a threshold and the other continues normally.
- A client rotates through origins to evade a matching-signature counter.
- An allowed action races the denial that opens the latch; no dispatch begins after the committed
  transition.
- The tab navigates or closes while the overlay is active.
- The extension service worker restarts while the service remains latched.
- The overlay cannot inject because the tab is a sacred browser surface.
- The user selects each disposition by mouse and keyboard.
- Reduced-motion and high-zoom users receive an equivalent, non-color-only signal.
- A legitimate client receives repeated denials and can recover without changing policy.
- A hostile loop continues sending calls while latched; replies stay bounded and do not create a
  notification storm or inflate counters indefinitely.

## Decision gates

Implementation should begin only after:

1. an ADR defines the state machine, exact counter inputs, ownership, control messages, recovery,
   audit facts, and initial thresholds;
2. the visual-language amendment defines ordinary and escalated treatments;
3. Lightbox scenarios cover races, isolation, restart, missing-renderer, and hostile-loop cases;
4. a live browser rehearsal confirms the overlay corresponds to real service behavior; and
5. one non-author review verifies that the difference between notice suppression and authority is
   understood.
