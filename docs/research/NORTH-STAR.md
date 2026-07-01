# North Star & Design Principles

**Date:** 2026-07-01 · **Status:** GOVERNING — set by the project owner. Overrides the emphasis
of [00-synthesis-and-decisions.md](00-synthesis-and-decisions.md) and governs how every report in
this folder is used.

> Prior art in this folder is a **concern surface**, not a feature catalog. We harvest the
> *problems* others hit and the *design questions* they were forced to answer, then answer them
> **our own way** against the north star below. We do **not** import other vendors' paradigms,
> feature sets, or alignments — they optimize for their own concerns, which are not ours.

---

## North Star

Browser MCP gives an AI agent governed access to the user's **own, authenticated, live browser
context** — their session, cookies, SSO, tabs. **The value *is* that it's the user's real
context.** Anything that moves away from that context (cloud browsers, fresh/clean profiles,
separate `--user-data-dir`s, stealth/anti-bot personas) is **off-mission by definition** and is
rejected no matter how common it is in prior art.

---

## Principle 1 — The engine is unconstrained; governance is an overlay

The MCP↔CDP engine enables **full capability with no built-in limits**. Access control is a
**separable overlay** with its own lifecycle. The engine never bakes in policy.

> Microsoft-product model: in an enterprise space, behavior is set by policy; in a user space,
> the user chooses. Same binary, different overlay.

## Principle 2 — "All open" is a first-class mode, not a degraded one

For personal use, **zero restrictions is a valid, fully-supported configuration** — the default,
even. It is *not* "enterprise minus governance." The unrestricted experience must be excellent on
its own terms.

Three postures, one engine:

| Posture | Who sets limits | Default stance |
|---|---|---|
| **All-open (personal default)** | nobody | a great unrestricted browser-automation MCP |
| **User-chosen** | the user | whatever limits *they* opt into |
| **Policy-enforced (enterprise)** | deployment channel (Intune/GPO) | default-deny |

## Principle 3 — User delight is co-equal with governance

Token efficiency, install-just-works, real-session magic, and agent-friendly ergonomics are
**first-order goals for every mode** — not enterprise afterthoughts. **Delight must not depend on
the overlay being present.**

> **Corollary (un-mix the concerns):** token efficiency must come from the *engine* — lean
> element refs, screenshot discipline — so the **all-open user gets it too**. It must NOT come
> from tool-filtering, which only exists when a restrictive overlay is on. Do not sell "governance
> filters tools → fewer tokens" as the delight story; that only helps restricted users.

## Principle 4 — The user's context is sacred

We attach to the real, logged-in browser. We **never relocate** the user's work to a clean/cloud
session to gain a technical property (e.g., an independent CDP oracle). Where a hardening
technique requires leaving the user's context, it is at most an **optional, opt-in deployment
profile** — never the default, never a requirement of the core value.

## Principle 5 — Separation of concerns

Engine, policy overlay, identity resolution, audit — **independent lifecycles, no bleed.** A
change to one must not force a change to another.

## Principle 6 — Prior art is a concern surface, not a paradigm to copy

Every idea harvested from the reports here must **earn its place against this north star.** We
take *questions* and *hazards* — not *paradigms*:

| We take (concern/hazard) | We reject (paradigm) |
|---|---|
| npx/Windows install pain → build a self-registering single binary | copying anyone's distribution model wholesale |
| service-worker death, requested-vs-committed-URL bug → correctness lessons | their architecture |
| prompt-injection reality, policy-shadowing footguns, PHI-in-URLs → design constraints | their RBAC shape / audit schema as-is |
| Stagehand's `extract` *idea* (schema-typed page data) → maybe an engine capability | Stagehand's cloud/CUA execution model |
| — | **cloud/fresh-session execution (violates Principle 4)** |

---

## How this re-weights the synthesis (00)

The eight forks split cleanly once engine and overlay are separated. This is the correct lens:

| Fork | Layer | Applies in all-open mode? |
|---|---|---|
| 1 — semantic `extract` | **Engine capability** | ✅ yes — benefits every user |
| 2 — snapshot-first / token efficiency | **Engine capability** | ✅ yes — *this* is the real token lever |
| 4 — self-registering installer | **Delight (universal)** | ✅ yes |
| 7a — true committed-URL reporting (`frameNavigated`, per-frame) | **Engine correctness** | ✅ yes — the engine should always know the real URL |
| 3 — risk annotations / HITL approval | **Overlay (user- or policy-chosen)** | optional — user may opt in even personally |
| 5 — audit + standards (OCSF/CEF/hash-chain) | **Overlay (governance)** | off by default; stderr/none in personal |
| 7b — domain enforcement + extension-trust posture | **Overlay (governance)** | inactive when all-open |
| 8 — policy resolution semantics | **Overlay (governance)** | inactive when all-open |
| 6 — positioning | **Corrected below** | — |

**Positioning, corrected:** the delight is *the user's own context + an unconstrained, efficient
engine*. Governance is an **optional overlay** that some users (enterprises) need. 00's "governance
as delight" framing is superseded by this doc — it conflated the two layers.

---

## Where this should graduate to

These principles belong in `SPEC.md §1` (they sharpen the existing "Engine / Policy / Identity
have independent lifecycles" statement) and should seed an ADR. This doc is the discovery-time
capture; the spec is the durable home.
