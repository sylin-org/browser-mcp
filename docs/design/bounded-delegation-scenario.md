# Bounded delegation: three scenarios and a prototype script

Status: Scenario exploration. Not an accepted design. Human prototype validation remains open.

## Why this scenario

"Give the agent access" is too vague to design a responsible experience. A useful delegation has a
purpose, a boundary, a lifetime, and an understandable end state. This scenario tests whether the
existing tighten-only session overlay can become a user-facing delegation contract without turning
ordinary work into policy administration.

## The person and the job

Mira maintains an open-source library. She asks her MCP client:

> Prepare the 0.6 release candidate. Review the open GitHub pull requests and CI failures, compare
> them with the Linear release checklist, and update the checklist with your findings. Do not merge,
> publish, delete, change repository settings, or message anyone.

Mira is already signed into GitHub and Linear in Chromium. She wants the agent to do useful work in
that real context, but she does not intend to delegate release authority.

## What Ghostlight should make visible

Before the work begins, the client or Ghostlight presents a compact contract:

```text
Purpose: prepare the 0.6 release candidate
For: this Codex session
Until: 30 minutes from now or session end

May use:
  github.com       read
  linear.app       read, write

Will stop before:
  merge or close pull requests
  publish a release or package
  delete or change settings
  send comments, messages, or review decisions

Limits:
  at most 12 writes
  no execute capability
  no other hosts
```

The summary is the primary interface. A manifest-shaped detail view is available for inspection,
copying, or organizational review, but Mira does not need to author it.

## Expected journey

1. The agent translates Mira's request into a proposed boundary. It explains that updating Linear
   requires write capability, while merging, publishing, and communication remain outside scope.
2. Mira accepts the proposal once. The resulting policy can only tighten the authority already
   available from user, organization, and managed tiers.
3. Ghostlight binds the contract to the authenticated MCP subject and current session. A copied
   session identifier is not authority.
4. The agent reads the release checklist, pull requests, and CI state. The activity indicator stays
   visible in the browser without interrupting every read.
5. The agent updates checklist fields and adds a private release note inside the specified Linear
   project. Each write consumes the visible budget.
6. A pull request looks ready. The agent attempts to merge it. Ghostlight stops the operation and
   explains: "Merging is outside this delegation. The current session may inspect pull requests but
   may not merge or close them."
7. The agent continues with the work still inside scope instead of treating the denial as a fatal
   error.
8. At completion, Mira receives a digest: hosts visited, findings, checklist changes, unused write
   budget, denied attempts, and the contract's expiration.

## What makes this delightful

- Mira describes the job, not access-control syntax.
- The proposed boundary uses the same vocabulary as the work.
- Routine reads do not trigger repetitive approval prompts.
- A denial preserves momentum and explains the next valid move.
- The agent cannot turn a request for more authority into authority by itself.
- Expiry is automatic and visible.
- The final digest answers "what did I entrust, and what happened?" in one place.

## How the current architecture helps

ADR-0060 already provides the core safety property: a session overlay composes by intersection and
can only reduce authority granted by higher tiers. Identity-bound policy, host polarity, RAWX
classification, audit correlation, and the persistent service supply most of the remaining
substrate.

The scenario deliberately uses read and write but excludes execute. It also distinguishes a Linear
record update from external communication, showing why a useful contract may need intent descriptors
more specific than RAWX alone. RAWX remains the capability floor; named consequences refine it.

## Questions the scenario exposes

1. Who proposes the contract: the MCP client, Ghostlight, or the model through an additive tool?
2. How does a client establish or replace the session overlay after initialization without
   reconnecting?
3. Which clients can render a native confirmation through MCP elicitation, and what is the graceful
   fallback for clients that cannot?
4. Are time and write budgets part of the manifest, a separate delegation envelope, or both?
5. How are semantic consequences such as merge, publish, delete, and communicate declared and
   verified across built-in and future WebMCP tools?
6. Can the user extend a contract without creating an escalation path controlled by the agent?
7. What minimum digest remains useful without retaining sensitive page content?
8. How should saved scripts request a delegation that is narrower than their hash-bound approval?

## Disposition

Do not write a delegation ADR from vocabulary alone. Test all three scenarios below with the paper
prototype script. The ADR should pin the user journey and authority transition together; an elegant
policy envelope with an awkward approval flow would miss the point.

## Personal scenario: trip research without purchase

### The job

Nadia asks her agent:

> Compare three train options for Friday, save the best two to my private trip draft, and stop
> before booking, messaging anyone, or changing my loyalty account.

She is signed into a train site and a personal planning app. There is no organization manifest.
All-open would technically permit the work, but Nadia wants a narrow promise for this session.

### Proposed contract

```text
Purpose: prepare Friday train options
For: this session
Until: 20 minutes from now or session end

May use:
  trains.example       read
  planner.example      read, write

Will stop before:
  purchasing or reserving travel
  sending messages or invitations
  changing account, payment, or loyalty settings

Limits:
  at most 4 writes
  no execute capability
  no other hosts
```

### What this scenario tests

- Whether personal users understand why a voluntary boundary helps even in all-open mode.
- Whether `save to private draft` is distinguishable from booking or communication.
- Whether a small write budget feels reassuring or like unexplained accounting.
- Whether expiry and teardown are understandable without organization language.
- Whether the user expects the contract to persist into the next conversation.

The expected answer to the last question is no: the contract is session-bound and erased at end.

## Organization-managed scenario: incident triage below the org ceiling

### The job

Ishan is on call. The organization policy already permits read access to observability and read/write
access to its incident system, but denies production mutation and external communication. He asks:

> Investigate alert INC-204, attach your findings to its private incident timeline, and prepare a
> remediation checklist. Do not restart services, change production, page anyone, or post publicly.

### Proposed contract

```text
Purpose: investigate INC-204
For: Ishan's current MCP session
Until: 45 minutes from now or session end

Organization ceiling:
  observability.example    read
  incidents.example        read, write
  production.example       denied

This session narrows that to:
  alert INC-204 and its linked dashboards
  private incident timeline and remediation checklist

Will stop before:
  production mutation or service restart
  paging, email, chat, or public status updates
  closing the incident

Limits:
  at most 10 writes
  no execute capability
```

### What this scenario tests

- Whether the interface clearly separates the organization ceiling from the session's tighter
  contract.
- Whether users mistake session acceptance for an override of an organization denial.
- Whether resource narrowing such as one incident can be represented and enforced truthfully.
- Whether a digest is useful without retaining alert text, log bodies, or timeline content.
- Whether the agent can continue investigation after a denied production action.

The session contract can only intersect with the active organization policy. It cannot add a host,
capability, or consequence the organization denied.

## Paper prototype script

Use the same six-state script for the release, personal, and organization-managed scenarios. A
facilitator reads the user request and shows one state at a time. The participant should think
aloud. Do not explain the intended model until after the first pass.

### State 1: proposed boundary

Show the compact contract only: purpose, session, expiry, allowed resources, stop conditions, and
limits. Controls:

- `Accept for this session`
- `Adjust`
- `Cancel`
- `View policy details`

Ask the participant what will happen if they accept, what remains forbidden, and how long it lasts.

### State 2: adjustment

`Adjust` exposes job-language choices rather than a raw manifest:

- remove one host or capability;
- lower the time or write budget;
- add a stop condition from a bounded consequence vocabulary; or
- cancel.

The prototype does not offer `add more access`. Expansion requires a new proposal evaluated against
the higher-tier ceiling and accepted from the trusted surface.

Ask whether the missing expansion control feels safe, confusing, or obstructive.

### State 3: active scope

Show one quiet status card:

```text
Active: prepare Friday train options
18 minutes left | 1 of 4 writes used
trains.example read | planner.example read, write
```

The controlled-tab border remains the ambient scope signal. Ordinary reads do not prompt.

Ask where the participant expects to reopen details, end the delegation, or see used budget.

### State 4: boundary event

Show one denied booking, merge, or production action:

```text
Stopped: booking travel is outside this session's delegation.
The research and private draft work can continue.
```

Controls:

- `Continue within scope`
- `End session`
- `Review delegation`

There is no one-click `allow anyway` on the denial. Ask whether the participant understands what
did and did not happen and what the agent can do next.

### State 5: expiry warning

At two minutes remaining, show a quiet, non-blocking notice. The only extension path is `Review and
propose another session boundary`; it never silently renews. Ask whether this arrives too early,
too late, or at the right level of interruption.

### State 6: digest

Show hosts used, capability counts, writes consumed, content-free outcome categories, denials, end
reason, and unused budget. Do not show page text, form values, messages, or screenshots by default.

Ask the participant to explain what they entrusted, what happened, and what did not happen.

## Validation record

For each scenario record:

| Question | Evidence |
|---|---|
| Can the participant state the purpose, expiry, and stop conditions after State 1? | Exact paraphrase |
| Do they understand that the contract narrows rather than expands authority? | Yes/no plus explanation |
| Can they find how to reduce scope, end it, and inspect details? | First control chosen |
| Does the write budget communicate consequence or create anxiety? | Participant language |
| After the denial, do they know what can continue? | Exact next action |
| Does the expiry flow feel like renewal pressure? | Participant language |
| Does the digest answer what was entrusted and done without page content? | Missing facts named |

Reject or redesign the prototype if a participant believes acceptance overrides organization
policy, persists into another session, authorizes an unlisted consequence, or makes the agent the
authority that approves its own expansion.

## Remaining decision gates

The repository can prepare the scenarios and script, but it cannot supply human comprehension
evidence. Before an ADR:

1. Run all three scenarios with at least one non-author participant and record consented notes.
2. Prototype both a client-native elicitation path and a local trusted-surface fallback.
3. Verify which supported clients expose the necessary MCP elicitation semantics.
4. Decide the bounded consequence vocabulary and whether it is enforceable for built-in tools.
5. Define expiry, budget consumption, digest, and mid-session policy-change semantics together.
