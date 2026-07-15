# ADR-0080: Resource-scoped browser command scheduling

Date: 2026-07-14
Status: Accepted
Builds on: ADR-0024 (generic tool pipeline), ADR-0030 (Hub and honest queue), ADR-0053
(thin extension), ADR-0060 (session policy overlay), ADR-0078 (closed-loop browser core), and
ADR-0079 (final attention admission). Amends ADR-0025's snapshot-entry timing, ADR-0030's
definition of the honest queue, and the scheduling isolation of ADR-0035 and ADR-0050
compositions. Preserves ADR-0005's policy-free extension, the trained tool schemas, and
ADR-0028's no-phone-home Continuity Promise.

## Context

Ghostlight accepts concurrent MCP calls by design. The service gives each call its own task, the
browser transport correlates replies, and the native writer serializes bytes. That protects JSON
framing. It does not serialize browser execution. The extension starts each async dispatcher
without waiting for the prior dispatcher to finish, so two handlers for one tab can overlap after
their first awaited Chrome or CDP operation.

That overlap is not merely cosmetic. One call can probe a tab URL and authorize against it while a
second call navigates the tab before the first action is dispatched. Mouse-down and mouse-up
sequences can interleave. Two navigations can observe the same load event. An interaction receipt
can attribute another command's mutation to the wrong intent. Per-tab reference maps, diff
baselines, screenshot coordinate context, dialogs, and console or network buffers all contain
mutable state that concurrent reads as well as writes can disturb. Capture visibility adds a
cross-plane barrier problem that the separate presentation-plane ADR must close.

One global mutex would remove those races by also removing useful concurrency. Work on two tabs
does not inherently compete. Presentation is different again: `narrate` must not wait behind a
slow navigation merely because both concern the same tab. Panic, hold, attention, lease, and
protocol control traffic must be able to preempt ordinary work.

The system therefore needs scheduling by domain resource and traffic class, not a transport lock
or a single application-wide queue.

## Decision

### D1. The service owns a resource-scoped `CommandScheduler`

Command scheduling is an application service in the service process. It is outside governance
policy and outside the native transport. The capability registry declares scheduling metadata
beside each tool's resource and capability metadata; the pipeline does not grow a switch over tool
names. A declaration may resolve its scope from the validated action and arguments.

Registry metadata covers model-facing admission, but it is not the only enforcement point. Every
page-bound send, including `Browser::raw_call`, recording finalization, coordinate probes, capture
mechanisms, and semantic-helper internals, must carry an `ExecutionContext` proving that it owns
the matching resource lease. The only bypass is an explicitly typed presentation, safety/protocol
control, or local operation. An architecture test rejects a new browser-bound path without one of
those types.

The shared vocabulary is:

| Traffic class | Resource key | Execution rule |
| --- | --- | --- |
| Page execution | `BrowserSurface { browser_slot, native_tab }` | FIFO per producer, fair across producers; one active command |
| Client topology | `ClientGroup { browser_slot, client_key }` | FIFO per producer and fair selection for tab/group work |
| Browser/window | Browser slot initially | Exclusive fair queue for window-wide mutations |
| Presentation | Surface plus presentation channel | Separate Presentation Broker; never enters the page FIFO |
| Safety/protocol control | Session or browser control identity | Prompt processing and queue retirement; never waits behind ordinary commands |
| Local | No browser resource | Runs concurrently within its own bounded subsystem |

The model-facing `tab_control` tool is ordinary page or topology work, not safety/protocol control,
and never gains this bypass from its name.

Resource compatibility is hierarchical. A browser-wide operation takes the browser-slot parent in
exclusive mode. Page and topology operations take that parent in shared intent mode plus their
surface or client-group child in exclusive mode. Therefore different child surfaces can run in
parallel, but a browser-wide mutation waits for every child operation and prevents new child
admission until it completes. Locks are acquired root-to-leaf and released leaf-to-root. No code
may acquire a child and then upgrade the parent.

Page execution includes reads initially. Current reads mutate or depend on reference, diff,
observation, buffer, and screenshot state. Read concurrency may be introduced only after those
dependencies become immutable snapshots and tests prove commutativity.

The native writer remains responsible for valid framing and bounded transport. It is not a command
scheduler. Likewise, the presentation channel may define replacement, expiry, revision, and
barrier semantics in its own ADR; this ADR only fixes its separation from browser execution.

The first implementation keeps window-wide work at browser-slot scope. A later window key requires
a stable service-side window identity and revalidation protocol. It must not acquire a browser lock
and migrate to a narrower lock while work is active.

### D2. A surface command binds one concrete target before authorization

The scheduler key uses the browser slot and Chrome's native tab id, not an MCP session, client
name, user-facing tab label, or active-tab position. Different surfaces may execute in parallel,
including surfaces in the same browser. Calls from different sessions that resolve to the same
resource obey the same resource queue.

A page-scoped command with no concrete tab first performs a read-only target-resolution step under
the client topology lane. It binds the selected owned tab to a concrete `BrowserSurface`, releases
the topology lane, and then enters that surface queue. It does not keep both leases. Every later
probe and the dispatched request use the bound native tab id; a focus change cannot silently
retarget the intent. Pure topology operations such as context listing or tab creation remain in the
client topology lane and do not transition to a surface queue. Target resolution binds identity
only. Any URL returned incidentally is ignored and probed again after the surface lease is held.

Security-relevant configuration and governance are atomically published as one immutable
`AuthoritySnapshot { config, governance, epoch }`. Mode, sacred domains, redaction behavior,
manifest policy, and the facts written to audit for one command come from that same snapshot. An
accepted reload of either component publishes a new whole snapshot and increments its epoch; a
failed reload preserves the last-known-good snapshot and epoch.

ADR-0060's session overlay is fixed at session initialization. The execution context carries that
same immutable overlay beside the authority snapshot so an internal or composed call cannot shed
its session ceiling.

The surface execution lease is acquired before the first tab URL or governing-resource probe. It
is retained through:

1. the current authority snapshot;
2. governing-resource resolution and authorization;
3. final hold, panic, attention, ownership, and authority-epoch admission;
4. extension dispatch and definite completion;
5. navigation landing verification, result post-processing, and interaction receipt construction;
6. audit completion.

This is the security boundary that closes the authorization-to-dispatch race. A lock acquired only
around the Chrome call is insufficient.

The lease isolates Ghostlight commands, not the human or page-owned asynchronous work. Receipts
remain bounded observations and must not claim database-style causality or rollback.

### D3. Queue admission uses bounded FIFO with explicit outcomes

Each resource keeps one FIFO subqueue per producer. A producer is an MCP session or an explicitly
named internal service actor. Arrival order is strict within that producer. The resource scheduler
selects ready producers fairly, then assigns a monotonic dispatch ordinal. There is deliberately no
global arrival-order promise across rival sessions; a flooding producer cannot reserve the whole
resource queue ahead of another.

Queues are bounded per resource, per session, and globally. Shared topology and browser queues use
the same fair selection. Queue capacities, wait budgets, scheduling quanta, and execution-lease
budgets are named constants with a sizing note and deterministic tests; no unbounded channel is an
acceptable implementation.

A command's response budget includes its queue wait. A deadline reached before extension dispatch
returns a structured `not_dispatched` outcome. Queue saturation also returns an explicit overload
outcome. Neither case is silently dropped, replayed, or represented as an unknown browser effect.

The scheduler may remove an idle resource queue after its last lease and waiter disappear. Queue
lifecycle must not retain session, URL, argument, result, or page-content data.

### D4. Current authority is bound at scheduled execution, not while waiting

ADR-0025's "snapshot once at call entry" rule is narrowed for scheduled browser work. Call entry
for such work means the point at which the command is fairly selected from its resource queue and
begins the critical section in D2. A queued command does not retain an authority snapshot that may
be stale by the time it can act.

The scheduler records the authority epoch at queue admission. If that epoch changes before
execution, the queued command retires as `not_dispatched`; it is not automatically re-authorized
and replayed under different configuration or policy. The caller may issue a new intent. Once
execution starts, its immutable snapshot remains authoritative through landing verification and
audit, as ADR-0025 requires.

Compositions still take a fresh authority snapshot for each model-authored step. A configuration or
policy reload during a running `script` or `browser_batch` therefore applies to the next step, even
when the composition retains a surface lease under D8.

### D5. Lifecycle and safety/protocol controls bypass and retire ordinary work

Panic, take-the-wheel hold, denial attention, session teardown, recording lease control,
screencast acknowledgements, transport recovery, and current presentation state do not wait in a
page execution queue.

When a hold, panic, attention pause, session teardown, or authority-epoch transition takes effect,
the service retires affected queued-but-not-started commands. It never resumes or replays them
automatically. The final admission check and the state transition share synchronization, extending
ADR-0079's send-boundary rule to the scheduler. Every nested extension send inside a retained
semantic lease rechecks panic, hold, attention, and session liveness immediately before enqueue.

An already-dispatched command is not rolled back. It retains its execution lease until the outcome
is reconciled under D6. "Prompt" or "preemptive" processing means controls bypass ordinary queues
and retire later work. It does not mean interrupting or rolling back an active Chrome effect.

### D6. Caller lifetime and execution lifetime are separate

The execution task, not the MCP caller future, owns the resource lease. The state machine is
explicit:

```text
queued -> executing -> completed
                    -> draining -> completed
                               -> uncertain
```

Four boundaries remain distinct:

- The queue deadline expires before extension dispatch and produces `not_dispatched`.
- The MCP response deadline limits how long a connected caller waits. Expiry after dispatch
  produces `outcome_unknown`, while execution continues to drain.
- The extension execution deadline bounds the expected terminal acknowledgement. Expiry moves the
  surface to `uncertain`; it does not unlock it.
- Surface quarantine has no automatic deadline. Recovery requires proof, not elapsed time.

If the MCP response deadline expires after dispatch, the detached execution task continues
draining the actual extension result. A late result completes post-processing and audit and
releases the lease, but it does not emit an obsolete MCP response. If the client connection or
response channel has already disappeared, there is no result recipient; draining, reconciliation,
and audit still occur.

A caller timeout, removed pending-reply entry, or native-port disconnect must never release the
lease by itself. An uncertain surface returns to service only after one of these proofs:

1. the same executor generation reports the command's exact terminal acknowledgement;
2. Chrome confirms destruction of the affected native tab, and later work uses a new surface id;
3. the browser process generation ends, invalidating both the tab and its executor before a new
   browser generation is admitted.

A new native port or extension worker inside the same browser generation is not sufficient proof.
An older extension without executor reconciliation must direct the user to close and recreate the
owned tab or restart the browser. Unlocking on a timer and allowing potentially overlapping browser
effects is rejected.

Internal command ids and extension execution acknowledgements distinguish not-enqueued, accepted,
running, completed, and generation-lost states. They are additive internal protocol fields and do
not change an MCP tool schema.

### D7. The extension keeps a mechanical `SurfaceExecutor` invariant

The extension maintains one bounded executor mailbox per actual browser surface. It runs one page
handler sequence at a time, deduplicates an internal command id within a bounded generation window,
and emits an exact terminal acknowledgement. The mailbox is bounded by command count and retained
bytes, including reassembled file, image, and GIF payloads. Normal service-scheduled operation has
at most one active item and one admitted successor per surface. Terminal, rejected, expired, and
stale-generation paths erase payload references promptly.

This executor is a final mechanical invariant for reconnects, omitted-target bootstrap, future
internal callers, and version skew. It does not authorize, classify, schedule fairly, audit,
calculate attention thresholds, or decide presentation policy. Those remain service concerns.
Safety/protocol control and presentation messages use separate bounded paths so a slow page command
cannot hide a panic state, starve a screencast acknowledgement, or delay narration.

The service scheduler remains authoritative. An older extension without the executor is safe only
to the degree that the service has one definitely reconciled in-flight command; uncertain recovery
requires the negotiated executor and resynchronization protocol.

#### D7 amendment: executor identity is per extension request

One scheduled command may issue several extension requests while retaining its surface lease under
D8. The executor therefore deduplicates the tuple of connection generation, scheduled command id,
and extension request id. It does not deduplicate the scheduled command id alone. Acceptance and
terminal messages continue to report the scheduled command id so the service can reconcile the
lease. This clarification was added after visible-Chrome verification found that command-only
deduplication completed the first `act_on` helper request and suppressed every later helper request.

#### D7 amendment: response delivery remains bound to the accepting connection

An active extension request may finish after its native port disconnects and a replacement service
connection reuses the same numeric request id. The extension therefore captures an immutable
response scope at admission: request id, scheduled command id when present, and the exact accepting
port. Success, error, acceptance, and terminal messages use that scope directly. They never recover
delivery state from a map keyed only by request id and never fall through to the current native
port. The same rule applies to auxiliary asynchronous replies created by that connection,
including tab URL probes and group responses.

A completion whose original port is gone is dropped at that dead connection boundary. It must not
be relabeled with a later command or delivered to a replacement service. Deterministic tests reuse
one request id across two ports, complete the older request last, and prove that each result keeps
its original port and command metadata.

### D8. One semantic intent may retain a reentrant surface lease

`act_on` and `form_fill` retain one reentrant surface lease from resolution through mutation and
observation. Their registry descriptors explicitly declare `RetainSurface`; a future semantic
helper must opt into the same execution shape rather than gaining reentrancy by convention. Their
internal browser calls reuse the execution context instead of queuing behind themselves, while D5's
safety admission still runs before every extension send. This prevents another model-authored
command from entering between selector resolution, action, and receipt construction.

A `script` or `browser_batch` whose page steps can be preflighted to one surface retains that
surface lease across those steps. Every step still enters the full pipeline and has independent
validation, authorization, audit, timeout, structured result, and stop-on-error behavior. The lease
adds isolation from other Ghostlight commands; it does not make the composition a transaction.

Retention is cooperative, not an eight-minute monopoly. A composition lease has a named scheduling
quantum no greater than the existing 60-second ordinary tool timeout. When the quantum expires, the
active atomic step settles, then the composition releases at that step boundary and re-enters
behind other ready producers before its next step. A composition that finishes within the quantum
keeps its surface isolation throughout. The scheduler never interrupts an active atomic step merely
to enforce the quantum.

A dynamic or multi-surface composition acquires and releases one resource lease per step. It never
holds multiple surface leases, promises cross-tab atomicity, or rolls back completed steps. A local
or presentation step such as `narrate` uses its own lane and cannot deadlock on the retained page
lease. Existing composition limits bound the maximum lease hold.

This amends only the scheduling isolation described by ADR-0035 and ADR-0050. Their model-facing
schemas, step order, per-step authority snapshots, result contracts, and audit semantics remain
unchanged.

### D9. Scheduler outcomes preserve existing composition contracts

Scheduler exit states are typed before MCP rendering. Their edge mapping is fixed:

| Scheduler state | Direct tool result | `script` / `browser_batch` result |
| --- | --- | --- |
| Queue deadline, overload, or authority retirement | `isError: true`; status `not_dispatched`; `retry_safe: true` | Existing step status `error`; normal `onError` behavior |
| Response deadline or unreconciled execution after dispatch | `isError: true`; status `outcome_unknown`; `retry_safe: false` | Existing step status `error`; all later steps `not_run` regardless of `onError` |
| Hold | Existing successful corrective result | Existing `held`, then `not_run` |
| Attention pause | Existing successful corrective result | Existing `attention_required`, then `not_run` |

The two scheduler statuses appear in additive `structuredContent` and in bounded corrective text.
They do not add a model-facing input or a new composition step-status vocabulary. `not_dispatched`
means no browser effect was enqueued; `retry_safe` says only that duplication is impossible, not
that current policy will allow a new call. `outcome_unknown` never invites automatic retry. If no
client response channel remains, the same typed state is retained only for completion and audit.

### D10. Scheduling diagnostics are local and payload-free

Local diagnostics may record the traffic class, opaque resource identity, command id, queue depth,
wait duration, run duration, state transition, and terminal reason. They must not record tool
arguments or results, page text, form values, screenshots, narration text, or full URLs. No
scheduling metric phones home.

## Acceptance criteria

1. Deterministic tests prove strict per-producer same-surface FIFO, fair cross-producer selection,
   and parallel execution on different surfaces.
2. A concurrency test proves that a navigation cannot change the governing origin between a
   command's resource probe, authorization, dispatch, landing check, and audit.
3. Input sequences, observation receipts, diff/reference state, coordinate context, dialogs, and
   read-with-clear buffers cannot interleave on one surface. Capture/presentation barriers are
   explicitly gated on the separate presentation-plane ADR.
4. Topology tests prove concurrent first-use requests cannot create duplicate client groups or
   orphan owned tabs. Compatibility tests prove a browser-wide mutation excludes all child surface
   and topology work while different child surfaces still run in parallel.
5. Capacity and fairness tests prove all queues are count- and byte-bounded as applicable, producer
   order is stable, fair selection prevents starvation, and a retained composition yields at its
   bounded step boundary.
6. Concurrent reload tests prove configuration, governance, authorization, post-processing, and
   audit use one atomic authority snapshot without torn epochs.
7. Reload, hold, panic, attention, and teardown tests prove queued commands retire before dispatch,
   nested sends repeat safety admission, and retired work is never replayed.
8. Timeout tests independently exercise queue, response, execution, and quarantine boundaries. A
   caller cannot release an executing lease, and only the three D6 proofs clear uncertainty.
9. Extension tests prove per-surface ordering, different-surface concurrency, command-id
   deduplication, count/byte overflow, payload erasure, generation recovery, connection-scoped
   response delivery under request-id reuse, and safety/protocol-control and presentation bypass.
10. Architecture tests prove every browser-bound send owns an `ExecutionContext` or uses one of the
    explicitly typed bypass paths.
11. Composition tests prove descriptor-gated reentrancy, bounded single-surface retention, per-step
    current-authority snapshots, truthful scheduler outcome mapping, and deadlock-free
    multi-surface fallback.
12. Lightbox and visible-Chrome scenarios exercise the service and extension together under
    deliberately overlapped calls and disconnect/reconnect faults.
13. Diagnostics tests prove queue metadata is useful while arguments, results, URLs, and page
    content remain absent.
14. The trained tool schemas remain byte-stable and all-open remains first-class. Correctness
    scheduling applies in all-open mode; governance work that is absent there is not introduced.

## Consequences

- Same-tab behavior becomes deterministic at the actual browser-effect boundary instead of merely
  ordered on the wire.
- The governance decision, page effect, landing check, receipt, and audit describe one coherent
  scheduled execution.
- Unrelated tabs and browsers retain useful parallelism. Narration and safety controls remain
  responsive during slow page work.
- Concurrent calls to one tab may wait or receive an explicit overload result. That latency is the
  visible cost of truthful isolation.
- Compound helpers need an execution context that can be passed and reused without layering
  transport concerns into domain logic.
- Rare unknown outcomes may quarantine one surface until recovery rather than risk a second effect.

## Rejected alternatives

- One global FIFO. Rejected because unrelated tabs, browsers, presentation, and local work do not
  share one consistency boundary.
- One FIFO per MCP session. Rejected because it blocks independent tabs while failing to protect a
  resource shared across sessions or internal callers.
- Rely on the native writer or extension port order. Rejected because byte order does not await
  async handler completion and cannot protect authorization before dispatch.
- Put all scheduling in the extension. Rejected because the extension cannot own authority snapshots,
  fairness, admission, audit, or truthful MCP outcomes.
- Release a mutex when the caller times out. Rejected because the browser effect may still be
  running and its outcome is unknown.
- Allow concurrent readers immediately. Rejected because current reads mutate or consume shared
  per-surface state.
- Queue presentation behind page commands. Rejected because presentation has replacement and
  lifetime semantics, not page-mutation ordering; `narrate` does not compete with navigation.
- Automatically replay retired or uncertain work. Rejected because stale browser intent is not
  safe to repeat.
- Treat a multi-tab composition as a transaction. Rejected because Ghostlight cannot lock the
  human or page, roll back browser effects, or safely hold an arbitrary set of tabs.

## Provenance

On 2026-07-14, after a live extension-feedback failure prompted a root architecture review, the
owner directed a DDD and separation-of-concerns approach. The owner then proposed per-tab FIFO and
separate channels, explicitly noting that narration does not compete with page execution. The
owner accepted the resource-scoped scheduler, lifecycle, timeout, compound-intent, and separate
presentation/control-lane design and authorized this ADR and commit.
