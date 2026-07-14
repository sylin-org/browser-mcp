# ADR-0075: Transaction-bound confirmation for managed actions

Date: 2026-07-14
Status: Proposed
Builds on: ADR-0022, ADR-0055, ADR-0060

## Context

Ghostlight can restrict tools by capability and destination, but an allowed browser action can
still have a consequence that policy cannot infer from a generic click: sending a message,
deleting a record, approving a release, or placing an order. Page text is not a trustworthy policy
input. It can be stale, localized, relabeled, or controlled by an attacker. The model's explanation
of its own intent is also not an authorization credential.

SEC-HIGH-03 therefore asks for a human confirmation gate for selected irreversible actions. The
gate must preserve three existing boundaries:

- the extension stays policy-free;
- all-open and personal use stay friction-free;
- governance reasons from capability, destination, and signed policy, never from page prose.

OWASP's transaction-authorization guidance supplies the useful shape: show the significant facts,
make authorization distinct from login, bind it to one transaction, expire it quickly, and check
it again immediately before execution. MCP form elicitation supplies a standard server-to-client
request with explicit accept, decline, and cancel outcomes. It does not, however, cryptographically
prove that a person operated the client's UI. That limitation is part of this proposal, not hidden
by it.

## Proposed decision

### 1. Only signed managed policy can require confirmation

A confirmation descriptor is an optional rule in an ADR-0055 signed managed bundle. It applies
only to an organization's known application and names:

- a stable rule id;
- exact host plus an optional exact path or bounded path prefix;
- one capability and one action kind;
- one structural element key: `id`, `name`, or `data-testid`;
- an admin-authored short title and rationale.

Visible text, ARIA labels, arbitrary CSS, XPath, regex selectors, and page-supplied rationale are
not supported in v1. Page hints are deferred. If added later, they may only raise friction; they
may never bypass or weaken a managed rule.

The descriptor targets org-owned applications whose DOM contract the org can maintain. It is not a
general claim that an element identifier is trustworthy on an attacker-controlled site. Existing
host and capability policy still applies first.

All-open, user-file, user-env, org-file, and session policy cannot introduce this gate in v1. This
keeps the personal path unchanged and makes every load-bearing descriptor tamper-evident through
the managed bundle signature.

### 2. Confirmation is a final execution gate, not a new grant

The ordinary pipeline remains authoritative:

1. validate the tool call and establish session, surface, and target ownership;
2. classify capability and domain;
3. run the composed policy decision;
4. if denied, return the denial without prompting;
5. if allowed and a managed descriptor matches, enter the confirmation gate;
6. dispatch only after the gate accepts the exact pending action.

Confirmation never turns a deny into an allow. It is a post-allow execution precondition at the
single browser-dispatch chokepoint. In `observe` mode a matching action records `would_confirm` and
continues without prompting. In `enforce` mode it pauses for confirmation. `script`,
`browser_batch`, and future compositions pass through the same child-action chokepoint; an approval
for one child cannot cover another.

This is internal governance growth. It adds no parameter, result field, or wording to a sacred
trained tool schema.

### 3. Resolve first, then bind one immutable pending action

Before prompting, the extension performs a mechanism-only target preflight and returns:

- browser slot, native tab id, and current document generation;
- canonical host and path;
- the matched structural element key and a bounded element fingerprint;
- the concrete action kind.

The service compares those facts to the signed descriptor and constructs an immutable
`PendingAction` containing the session guid, policy generation and rule id, capability, normalized
tool arguments, target facts, a cryptographically random approval id, and an expiry. This object
exists only in memory.

The prompt exposes only the admin-authored title/rationale and minimized structural facts: action,
host/path, and control key. It does not include page text, DOM content, form values, screenshots,
credentials, or model-authored prose. This is a meaningful confirmation of the action Ghostlight
can actually identify. It is not represented as full business-transaction signing: if a user must
verify a recipient, amount, or record contents, the application must supply its own trusted
confirmation surface.

### 4. Use MCP form elicitation with an explicit boolean

For a supported client, Ghostlight sends `elicitation/create` in form mode with one required
boolean such as `confirm_action`. It has no default. Dispatch requires both:

- response action `accept`; and
- `confirm_action: true` after schema validation.

`decline`, `cancel`, malformed content, timeout, session loss, or client error blocks the action.
No fallback converts those outcomes into audit-only behavior in enforce mode.

Form mode is used because the response contains no secret or payment credential. URL elicitation is
not used for this gate: it would add a second web application and authorization state without
strengthening the action binding.

Client capability advertisement is necessary but not sufficient for an enforce deployment. Before
this ADR can be accepted, each supported client must be observed presenting the request as a clear,
human-operated prompt that identifies Ghostlight as the requester and offers distinct confirm,
decline, and cancel controls. An unknown client, a client without form elicitation, or a client
whose elicitation can be silently answered by the model fails closed. The protocol provides no
cryptographic proof of human presence, so Ghostlight must not claim one.

### 5. Approval is one-time, short-lived, and stale-sensitive

The first implementation uses a fixed 60-second approval lifetime. Approval is consumed exactly
once. It cannot be cached for the session, rule, host, or tool.

Immediately before dispatch, the service and extension jointly verify that the pending action is
still current. Any change to the following cancels it and requires a new prompt:

- normalized tool arguments;
- session or surface ownership;
- tab, host, path, document generation, structural key, or element fingerprint;
- policy generation or matching rule;
- pause, take-the-wheel, panic, session termination, or expiry.

The extension receives a conditional dispatch carrying the approved target facts and refuses the
action if its document or element no longer matches. This is mechanism, not policy: the extension
does not know why the condition exists or whether the action is consequential.

The approval response is tied to the random approval id and the in-memory pending object. The audit
log may record the approval id, rule id, disposition, and timing, but never the normalized
arguments or an unsalted digest of them. This avoids turning low-entropy personal data into a
dictionary-testable audit artifact.

### 6. Captured confirmation data is ephemeral and minimized

Pending actions, target fingerprints, and elicitation responses remain in bounded process memory.
They are erased on every terminal path and are never written to debug state, crash fixtures, audit
payloads, temporary files, or remote services.

Audit records contain only content-free control evidence:

- managed policy generation and rule id;
- random approval id;
- `would_confirm`, `accepted`, `declined`, `cancelled`, `expired`, or `stale`;
- elapsed time and final execution outcome.

The policy bundle already persists the admin-authored title and rationale, so the audit record does
not duplicate them. Existing redaction and retention rules continue to apply.

### 7. Verification and acceptance gates

This ADR remains Proposed until all of the following are complete:

1. A supported-client matrix proves human-mediated form elicitation behavior.
2. The managed manifest schema and strict structural-selector grammar receive a separate schema
   review.
3. The browser preflight and conditional-dispatch wire additions are designed additively.
4. A privacy review inventories every in-memory and persisted field.
5. Lightbox proves allow, deny, observe, accept, decline, cancel, timeout, stale target, policy
   reload, pause, panic, reconnect, and composition boundaries.
6. A real browser test proves that navigation or DOM replacement between approval and dispatch
   cannot execute the stale action.
7. Public claims call this transaction-bound confirmation, not phishing prevention, transaction
   signing, or proof of human presence.

## Rejected alternatives

- Infer consequence from page text, button labels, model prose, or DOM semantics. These are
  attacker-influenceable and do not belong in the policy decision.
- Confirm every write or execute action. That destroys the low-friction personal path and trains
  users to approve prompts without reading them.
- Treat login, an MCP session, or a previous confirmation as blanket consent. Authentication and
  transaction authorization answer different questions.
- Dispatch after approval without rechecking the target. That creates a time-of-check/time-of-use
  window.
- Persist full action arguments or their plain digest. Browser arguments may contain personal,
  financial, authentication, or communication data.
- Put the policy matcher in the extension. The extension remains a thin mechanism layer.

## Consequences

- Managed administrators can name the small set of org-owned controls that deserve visible human
  friction without taxing all-open or personal use.
- The prompt is honest about what Ghostlight knows. Application-specific business facts remain the
  application's responsibility.
- Unsupported or opaque clients cannot run confirm-required actions in enforce mode.
- The service gains bidirectional MCP request handling and an ephemeral pending-action state
  machine; the extension gains only preflight and conditional-dispatch mechanics.
- A managed app DOM change can fail closed until its descriptor is updated. That brittleness is
  preferable to silently bypassing the gate.

## References

- MCP 2025-11-25 elicitation:
  https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation
- OWASP Transaction Authorization Cheat Sheet:
  https://cheatsheetseries.owasp.org/cheatsheets/Transaction_Authorization_Cheat_Sheet.html
- ADR-0022: intent-calibrated capabilities.
- ADR-0055: signed managed policy distribution.
- ADR-0060: composed policy tiers.
