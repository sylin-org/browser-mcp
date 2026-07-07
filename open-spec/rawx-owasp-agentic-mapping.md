# Mapping RAWX governance to the OWASP agentic threat taxonomy

Status: informational mapping, first published 2026-07-07 (ADR-0041 Decision 1: meet external
vocabularies with bridges). Licensed Apache-2.0 OR MIT like everything in open-spec/.

Agent-governance evaluations increasingly use the OWASP Agentic Security Initiative's threat
taxonomy as their checklist (Microsoft's open-source Agent Governance Toolkit, for one, maps
itself against all ten of its risks). This note maps the [RAWX capability
model](rawx-capability-model.md) and the governance overlay of its reference implementation
(Ghostlight) onto that taxonomy's themes, plus the 2026 University of Washington findings on
agentic-browser security. The OWASP taxonomy's exact naming evolves; check the initiative's
current publication for canonical wording. This mapping is written honestly in both
directions: it also states plainly what a governance overlay does NOT mitigate.

## The threat themes and what governs them

**Tool misuse / excessive agency.** The core RAWX case. Every action is classified by
intrinsic capability (read / action / write / execute), and grants set per-host capability
floors. An agent on a read-only grant cannot type, submit, or execute script anywhere the
grant applies, no matter what it was talked into. Advertisement filtering removes ungranted
tools from the agent's view entirely.

**Goal hijacking / intent manipulation (prompt injection).** A governance layer cannot stop a
model from BELIEVING injected instructions; that battle happens inside the model and its
harness. What it does is cap the blast radius: an injected agent still cannot exceed its
capability floor, cross host polarity, touch sacred never-touch domains, or act while a
take-the-wheel hold is in effect. Injection turns from "attacker controls the browser" into
"attacker controls a session confined to what the human granted."

**Privilege compromise / identity abuse.** Grants are identity-bound and host-scoped; the
audit record carries the identity, client, grant id, and decision for every call. There is no
ambient authority to steal: a session holds the manifest's grants and nothing else.

**Cross-origin data movement (the UW findings; taxonomy: data exfiltration / cascading
effects).** The 2026 UW study showed four of seven agentic browsers create same-origin-policy
bypass conditions: content from one origin steering actions or data on another. Host-polarity
grants confine which hosts can be read versus written at all, and origin-flow provenance
(Ghostlight ADR-0042) records, per orchestrated step, which prior steps' results fed its
arguments, making in-band cross-host flows visible and auditable, with flow-level enforcement
as the recorded next step. The honesty fence: only IN-BAND flows (through the engine's own
reference substitution) are attestable. Data the model carries in its context between calls is
out of band for any local mechanism, and any product claiming otherwise is selling
content-inspection theater.

**Repudiation / untraceability.** The structured audit stream is the spine: every call --
allowed, denied, shadow-denied, or held -- produces one record with identity, tool, action,
capability, domain, grant or denial id, timing, and orchestration correlation (parent,
batch id, step, dry-run, sources). Denials carry stable ids the security team can reference.

**Human-in-the-loop bypass / overwhelming.** The take-the-wheel pause and the panic kill
switch are user gestures enforced ahead of all policy machinery; a held call never queues or
replays. Write-class actions can require explicit grants rather than per-call nagging, which
is what makes the human checkpoint sustainable instead of click-through.

**Unexpected code execution.** `javascript_tool` and script-bearing paths are the `execute`
class, the highest RAWX tier, grantable separately from everything else and deniable per host.
Named, hash-bound saved workflows (ADR-0039) are the reviewed alternative to open-ended
execute grants.

**Memory poisoning.** Mostly out of scope: Ghostlight holds no cross-session agent memory to
poison. The adjacent surface it does govern: saved scripts are hash-bound, so a tampered
artifact invalidates its standing approval instead of running under it.

**Supply chain.** Out of scope for the governance layer itself; addressed at the project
level (single auditable binary, reproducible releases, signed provenance) rather than by RAWX.

## What this layer does not do

No content inspection or DLP; no protection against the model being deceived within a granted
scope (a write grant misused on the granted host is within policy); no attestation of
out-of-band data flows through model context; no substitute for browser-level origin
isolation, which is the browser vendor's layer. Governance bounds agency and makes actions
attributable; it does not make the agent smart or the page trustworthy.

## Sources

- OWASP Agentic Security Initiative (threats and mitigations): https://owasp.org
- Microsoft Agent Governance Toolkit: https://github.com/microsoft/agent-governance-toolkit
- UW agentic-browser study coverage (2026-07): https://www.technology.org/2026/07/03/some-agentic-ai-browsers-come-with-major-cybersecurity-risks-uw-study-finds/
- Ghostlight ADRs 0011, 0013, 0018, 0022, 0039, 0042 (docs/adr/) for the mechanisms named here.
