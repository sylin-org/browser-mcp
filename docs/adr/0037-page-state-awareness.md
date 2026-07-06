# 0037. Page-state awareness: `wait_for`, consequence digests, and diff reads

- Status: Accepted
- Date: 2026-07-06

## Relationship to other decisions

- BUILDS ON ADR-0005 (policy-free extension, DOM reads in a content script): all three
  mechanisms here are the content script observing the page over time. No policy enters the
  extension; it reports observations, the binary decides nothing new.
- BUILDS ON ADR-0034 (capability registry): `wait_for` is a new browser-capability tool,
  declared in the browser directory, additive under ADR-0034 Decision 7.
- COMPANION TO ADR-0035 (`script`): sequential scripts against dynamic pages are dead on
  arrival without a condition wait between navigate and read (a `navigate` step "completing"
  means the load event fired, not that a hydrating SPA is ready). `wait_for` is an ordinary
  script step; consequence digests make sequential steps self-evidencing.
- BUILDS ON ADR-0031 (corrective errors): the stale-ref enrichment (Decision 4) and
  `wait_for`'s timeout error follow the corrective-error discipline.
- RE-PINS the all-open output-identity invariant (ADR-0013 lineage): that invariant separates
  the governance OVERLAY from the engine -- overlay present must not change engine output. It
  does not freeze the engine itself. Consequence digests are an engine evolution, applied
  identically in every mode (all-open, safe, restricted), so overlay-vs-engine identity holds.

## Context

Watch any model drive a browser and three flail loops dominate:

1. **Read too early.** Navigate, read, get a skeleton, screenshot, wait, read again. Three to
   four round trips burned learning the page was not ready. The existing `computer` `wait`
   action is a fixed sleep -- the model guesses a duration, and guesses again.
2. **Did my click work?** Every mutating action is followed by a verify read or screenshot,
   because the confirmation says "clicked" but the model's actual question is "what did the
   click CAUSE?" The verify step is often half the token spend of a workflow.
3. **What changed?** After an action, the model re-reads the whole page and diffs it inside
   its own context -- the most expensive possible way to answer "what is different now."

All three have the same shape: the machine knows something about page state over time, and the
model pays inference round-trips to reconstruct it. The content script is already in the page;
let it watch.

## Decision

### Decision 1: the `wait_for` tool

A new browser-capability tool: wait until a condition holds on the page AND the page has settled.

```json
{ "tool": "wait_for", "args": { "tabId": 0, "text": "Results", "state": "visible", "timeout_ms": 10000, "min_ms": 0 } }
```

- **Condition:** exactly one of `selector` (CSS) or `text` (visible-text substring). Both may
  be omitted (bare settle-wait -- see Decision 5).
- **`state`:** `"visible"` (default; present AND visually rendered), `"present"` (in the DOM),
  `"gone"` (absent or hidden), `"settled"` (page mutation rate has decayed below an adaptive
  threshold for N consecutive windows -- see Decision 5). A condition (`text`/`selector`) plus
  `"settled"` is allowed: the condition must match AND the page must be settled.
- **`timeout_ms`:** default 10000, hard cap 30000 (inside the 60s `TOOL_TIMEOUT` with margin).
- **`min_ms`:** default 0; a minimum elapsed time the wait must observe before returning,
  regardless of whether the condition matched and the page settled. A `min_ms` of 3000 means
  "even if the page settled at 2s, do not return until 3s" -- giving a late hydrating widget
  its window. Under `timeout_ms`, always.
- **RAWX class:** `Read` (it observes the DOM, touches nothing).
- **Mechanism:** content-script `MutationObserver` with a 250ms polling fallback (observers
  miss visibility-only changes from CSS/layout).
- **Result** (structured per ADR-0038): `{ "found": true, "elapsed_ms": 640, "ref": "ref_12" }`
  -- when the condition matched an element, its ref is minted and returned, so
  `wait_for -> computer(left_click, $prev.ref)` chains directly in a script.
- **Timeout is a ToolError** (isError), with a corrective message reporting what WAS on the
  page (title, a short excerpt near the closest fuzzy match if any). Failing loudly matters:
  under `script`'s `onError: "stop"`, a timed-out wait halts the chain instead of letting
  subsequent steps execute against a page that never arrived.

`computer` `wait` (fixed sleep) stays untouched; `wait_for` is the condition-based sibling.

### Decision 2: consequence digests on mutating actions

After each mutating, non-screenshot-returning action -- `computer` actions `left_click`,
`right_click`, `double_click`, `triple_click`, `type`, `key`, `left_click_drag`, `hover`,
`scroll_to`, plus `form_input` -- the content script samples the page over a short settle
window (pinned: 300ms) and the action's text confirmation gains a compact digest:

```
observation: url changed to /dashboard; 47 DOM mutations; focus moved to "Search";
alert appeared: "Changes saved"
```

- **Signals sampled** (cheap, no tree walk): URL change, title change, focused element's
  accessible name, DOM mutation count (a `MutationObserver` counter, subtree-wide, counters
  only -- no node retention), newly appeared `role="alert"` / `role="status"` text (first
  200 chars), newly appeared `role="dialog"` presence.
- **Format pinned:** a single `observation:` block, at most 400 chars, appended to the
  existing confirmation text. Omitted entirely when nothing observable happened ("no observable
  change" is itself reported, because a silent click is a signal the model needs).
- **Structured twin** under ADR-0038 for the same fields.
- **Cost accepted:** +300ms latency on mutating actions. That is the price of killing the
  verify round-trip, which costs seconds and thousands of tokens. Screenshot-returning actions
  (`screenshot`, `scroll`, `zoom`) are unchanged -- the screenshot IS the observation.
- **Always on, every mode.** This is engine behavior, identical under all-open and under
  governance; the overlay-vs-engine identity invariant is preserved.

`form_fill` (ADR-0036) surfaces the submit click's digest in its own result (`observation`
field), so a submitted form reports what submission caused with zero extra round trips.

### Decision 3: `read_page` diff mode

An additive optional boolean property `diff` on `read_page`'s inputSchema (sanctioned by
ADR-0034 Decision 7; the property is optional, the trained fields and the filter enum are
untouched, trained models simply never send it):

- The content script keeps, per tab, the last rendered tree it returned and a render serial
  (bumped by the mutation counter crossing a render-relevant threshold).
- `read_page(diff: true)` returns only lines added, removed, or changed (keyed by ref
  identity) since this session's previous `read_page` on that tab, prefixed `+` / `-` / `~`.
- No baseline (first read on a tab, or the content script was reinjected): falls back to a
  full read, marked `(no baseline; full tree)`.
- The model's second-most-common question after "did it work" is "what is different now";
  answering it server-side keeps a second full accessibility tree out of the context window.

### Decision 4: stale-ref corrective enrichment

Refs dangle: the model reads a page, the page re-renders, `ref_3` no longer resolves. Today
the deref failure says the ref was not found; the model cannot tell "wrong ref" from "world
moved". Pinned: the content script stamps its render serial into ref-consuming failures, and
the error becomes corrective per ADR-0031:

```
ref_3 no longer resolves: the page re-rendered since your last read (render serial 4 -> 9).
Call read_page (or read_page with diff: true) and use a fresh ref.
```

No retry logic, no ref healing (content-addressed refs are an open question) -- just an error
that teaches, which is the cheapest reliability feature that exists.

### Decision 5: the settle detector -- adaptive rate-of-change decay

`state: "settled"` and the default-on settle check (Decision 6) both run the same detector: a
rate-of-change decay evaluator over the mutation counter that Decision 2 already mandates. No
content diffing, no tree walks -- just the per-window mutation count, which is free.

**Window size:** 500ms (reuses the ADR-0037 poll interval).

**Adaptive threshold:** `T = max(floor(peak_rate * 0.05), 3)` where `peak_rate` is the highest
mutation count seen across all windows so far. The threshold adapts to the page: a heavy
hydration (400 mutations/window) settles when the rate drops below 20/window; a light cached
page (30 mutations/window) settles when it drops below 3. The floor of 3 absorbs low-frequency
background updates (an SSE tick adding 1-2 nodes every 500ms stays below the floor).

**Settled condition:** mutation rate < T for **3 consecutive windows** (1.5s of stability). The
3-candidate rule prevents a brief quiet gap mid-hydration from false-positive; it also means a
cached page that barely changes takes ~2s minimum, which is the right floor for "the page is
actually here."

**Minimum observation:** 1 window (500ms) must elapse before the first "candidate" -- a
0-mutation reading on the very first poll (content script not yet rendered) does not count.

**What it cannot catch:** CSS-only animations and layout shifts produce 0 DOM mutations. The
3-candidate window (1.5s) absorbs brief animations; for perpetual-layout-animation pages the
model falls back to `wait_for(text: "...")` or `computer(wait)` -- the existing escape hatches.
A layout-stability check (Chrome's LCP / web-vitals CLS) is a possible v2.

**Result enrichment:** when the settle detector fires, the result carries diagnostic fields:
`{ "settled": true, "elapsed_ms": 2500, "peak_mutations": 450, "final_rate": 3 }`. The model
sees "this was a heavy page that settled in 2.5s" vs. "this was a light page" -- building its
own mental model of the site for future visits.

**Timeout on bare settle:** a page that never settles (perpetual SSE feed above the threshold)
times out, and the corrective error reports the sustained rate: "the page did not settle within
10s (still changing at ~30 mutations/500ms)."

### Decision 6: settle is ON by default for every `wait_for`

The settle check is not an opt-in; it is the default posture. Every `wait_for` call waits for
BOTH its explicit condition (if any) AND page settlement -- the model does not need to know it
should check for settlement.

This is a deliberate UX call: a `wait_for(text: "Results")` that returns the instant the text
appears, while the page is still hydrating the table below it, sets the model up for a
read-too-early flail on the very next step. Defaulting to "condition AND settled" makes the
readiness signal honest.

- **`settle: false`** (opt-out): the explicit condition alone gates the return. For when the
  model knows the page is static (cached page, simple server-rendered content) and the settle
  delay is pure waste.
- **`min_ms`** (Decision 1): a floor on elapsed time, independent of settlement. `min_ms: 3000`
  means "even if the page settled at 2s, do not return until 3s." This covers the late-hydrating
  widget that finishes its main content at 1.5s but adds a secondary panel at 2.8s -- the
  settle detector might fire at 2s (the main content's mutation burst ended), but the model
  wants the secondary panel too, so it asks for 3s minimum. If at 3s the page is still mutating
  significantly, `wait_for` keeps going until it settles or `timeout_ms` fires.
- **Bare settle-wait** (`wait_for(state: "settled")` with no `text`/`selector`): the simplest
  form. "I navigated; tell me when the page has stopped churning." This is the natural
  post-navigate step in a `script` when the model doesn't know what text to wait for.

The combination that makes `script` viable against SPAs:

```json
[
  { "tool": "navigate", "args": { "url": "https://example.com" } },
  { "tool": "wait_for", "args": { "min_ms": 3000 } },
  { "tool": "read_page", "args": { "filter": "interactive" } }
]
```

Navigate, wait at least 3 seconds AND until the page settles (whichever is later), then read.
The model expresses intent ("give the page at least 3 seconds, and don't read until it's
done"); the service evaluates both conditions in parallel and returns when both are satisfied.

## Consequences

### Fixed

- The read-too-early flail loop dies: `wait_for` makes readiness a machine concern, and makes
  `script` viable against SPAs. Settle-by-default (Decision 6) means the model doesn't even need
  to ask for settlement -- every wait is honest about page readiness.
- The settle detector (Decision 5) adapts to the page's own pace: heavy hydrations and light
  cached pages both settle correctly without a fixed threshold the model would have to guess.
- The verify round-trip dies for most actions: every mutating action reports what it caused.
- Re-read costs collapse: diffs instead of full trees for "what changed".
- Stale refs stop being a mystery: the error names the re-render and the fix.

### Cost

- One new tool declaration (`wait_for`), its content-script observer + poll loop.
- The settle detector (~40 lines: per-window mutation count, adaptive threshold, 3-candidate
  window, peak tracking -- all on the counter Decision 2 already maintains).
- The per-tab observation state in the content script (mutation counter, render serial, last
  rendered tree for diffing) and its lifecycle across content-script reinjection.
- +300ms settle latency on mutating actions (pinned, revisitable).
- The digest sampler and its format discipline (400-char cap).
- Diff computation and the `(no baseline)` fallback.

### Preserved invariants

- The extension stays policy-free (ADR-0005): it observes and reports; it decides nothing.
- Overlay-vs-engine output identity: digests appear identically in every mode.
- The trained tool schemas' existing fields and enums are untouched; `diff` is additive.
- The screenshot pipeline and coordinate model (ADR-0010) are untouched.

## Open questions (deferred)

- **Network-idle wait condition** (`wait_for` on outstanding requests): deferred; needs the
  CDP Network domain wired into the wait path, and visible-text conditions cover most real
  readiness checks.
- **Digest opt-out config key**: deferred until someone objects to 300ms; the settle window
  may also prove tunable per action type.
- **Content-addressed refs** (refs as role+name hashes that survive re-renders): deferred;
  Decision 4's honest error is v1.
- **Hostile-page observer cost** (pages generating millions of mutations): counters only, no
  node retention, but a pathological page may still warrant a sampling cutoff; measure first.
