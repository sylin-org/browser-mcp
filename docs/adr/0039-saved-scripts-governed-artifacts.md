# 0039. Saved scripts as governed artifacts

- Status: Proposed (direction ratified 2026-07-06; implementation deferred until ADR-0035's
  `script` has landed and real scripts exist to learn from)

## Relationship to other decisions

- BUILDS ON ADR-0035 (`script`): the saved artifact IS the `steps` shape, named and
  parameterized; the `batch_id` audit correlation pinned there is the recording substrate.
- BUILDS ON ADR-0022 / ADR-0019 (grants, layered configuration): a saved script becomes a
  grantable, org-lockable object like any other policy surface.
- BUILDS ON ADR-0025 (manifest hot-reload): the scripts directory is watched the same way.
- BUILDS ON ADR-0034 (capability manifest): saved scripts are advertised at handshake, so a
  model discovers the house's approved workflows the way it discovers tools.
- ALIGNS WITH ADR-0027 / ADR-0028 (open-core, tripwire licensing): approved-workflow
  governance is a natural commercial-tier surface; the engine-side run/save mechanics stay in
  the open engine. No behavioral gating either way, per the Continuity Promise.

## Context

`script` (ADR-0035) is ephemeral: the model composes a workflow, runs it once, and the
composition evaporates. But a workflow that worked is knowledge, and today that knowledge has
nowhere to live -- every session re-derives "log into the portal, open billing, export the
CSV" from scratch.

Meanwhile, on the governance side, orgs do not actually want to reason about primitives. A
security team asked to allow `javascript_tool` on the billing domain will say no; the same
team shown a NAMED, REVIEWED, FROZEN five-step workflow that uses it will say yes. Nobody in
the MCP browser space has an object that is simultaneously the model's saved competence and
the org's unit of approval. Ghostlight's architecture (grants, manifests, hot-reload, audit
correlation) already has every ingredient.

This ADR is deliberately direction-level: it pins the shape of the bet, not the
implementation. It is ratified as a destination so that ADR-0035's execution does not
foreclose it (the `batch_id` correlation and the steps-array shape are the two load-bearing
dependencies), and its details are expected to be re-pinned against reality once real scripts
exist.

## Decision (direction-level)

### Decision 1: the artifact

A saved script is a named JSON file: metadata (name, description, created_by, created_at,
version) + an ADR-0035 `steps` array + a parameter declaration block. Names are flat,
kebab-case, unique per installation.

### Decision 2: parameterization

Steps may reference `$param.name` alongside `$prev`/`$N`; the artifact declares its parameters
(name, type, required, description). Invocation supplies values; the resolver substitutes them
with the same grammar, escaping, and corrective-failure rules as ADR-0035 Decision 2.

### Decision 3: storage and reload

Artifacts live in a `scripts/` directory under the existing config root, watched for
hot-reload exactly as manifests are (ADR-0025). File-based, diffable, reviewable in a PR --
the org's change-control instincts apply without new machinery.

### Decision 4: governance -- the approved-workflow object

- Manifests can grant or deny saved scripts BY NAME, per domain, like any grant subject.
- The decisive property: a granted saved script may run on a domain where its constituent
  primitives are individually denied. The org approved the WORKFLOW -- the frozen, named,
  reviewed sequence -- not the open-ended primitives. This is the entire point of the object,
  and it is what no primitive-level policy system can express.
- An artifact edit invalidates standing approval (the grant binds to a content hash alongside
  the name; a changed file is a new approval question). Details deferred, principle pinned.
- Execution audits per-step with `orchestrator: "saved:<name>"` and a `batch_id`, so the
  audit stream shows exactly which approved workflow ran and what it did.

### Decision 5: advertisement

Granted saved scripts appear in the capability manifest at handshake (name, description,
parameters), so the model discovers "this house has `export-billing-csv`" the way it
discovers tools. Whether they surface as entries under a single `run_saved` tool or as
individual manifest entries is deferred to implementation.

### Decision 6: creation paths

Two, both deferred in mechanism but pinned in direction: explicit (`script` gains a
`save_as` argument, or a management-zone verb) and retrospective ("these last six calls
worked; save them as a script" -- reconstructing a steps array from the session's own audit
trail via `batch_id` and call records). The retrospective path is the delight path: the model
converts exploration into competence without having planned to.

## Consequences

### If taken

- Competence accumulates: a workflow taught once becomes a named capability next session.
- Governance gets its missing unit: reviewable, approvable, hash-bound workflows instead of
  all-or-nothing primitive grants.
- The audit story closes the loop: approval names a workflow; the stream shows that workflow,
  step by step, every time it runs.

### Cost (at implementation time)

- The artifact format, its validation, and the hash-binding of grants.
- Grant-subject plumbing for script names in the manifest schema and the policy engine.
- The advertisement surface and its capability-manifest rendering.
- The creation paths, especially retrospective reconstruction.

### Risks, named now

- **Scope creep toward a programming language.** Parameters yes; conditionals, loops, and
  includes NO -- the same fence ADR-0035 pinned. A workflow needing logic is a semantic
  helper (ADR-0036 pattern) or it is the model's job.
- **Approval theater.** A script granted-by-name that internally runs `javascript_tool` must
  be hash-bound and reviewed as content, or the name is a fig leaf. The hash-binding
  principle in Decision 4 is not optional.

## Open questions (all of them, essentially)

- Grant syntax in the manifest schema; interaction with host polarity (ADR-0022).
- `run_saved` tool vs per-script manifest entries.
- Versioning and migration of artifacts across engine upgrades.
- Whether saved scripts may call `form_fill` and future semantic helpers (presumptively yes,
  same rule as ADR-0035 steps).
- Commercial-tier boundary: which side of the open-core line the approval workflow tooling
  lands on (the run mechanics stay open regardless).
