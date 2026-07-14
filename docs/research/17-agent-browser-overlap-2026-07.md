# agent-browser overlap and free-product opportunities, 2026-07

Status: current capability rebaseline and product recommendation.

Checked on 2026-07-14 against agent-browser v0.31.2 and its official command reference. Primary
sources:

- https://agent-browser.dev/commands
- https://agent-browser.dev/changelog
- https://github.com/vercel-labs/agent-browser/blob/main/README.md

This is a mutual capability map, not a winner table. agent-browser is a broad browser runtime and
testing CLI. Ghostlight is a local MCP browser bridge for the user's existing authenticated
Chromium context, with an optional organization governance layer. The products overlap strongly
in ordinary agent work but optimize different boundaries.

## Reading the map

- `Yes` means the normal outcome is available, even when the interface differs.
- `Partial` means the useful core exists but not the full upstream breadth.
- `No -- boundary` means adding it would contradict a standing Ghostlight product decision.
- `No -- candidate` means it may be useful, but it needs its own value and risk case.

## One-to-one mutual capability table

| User or model job | agent-browser v0.31.2 | Ghostlight today | Strategic reading |
| --- | --- | --- | --- |
| Use MCP over stdio | Yes, with core and optional tool profiles | Yes, 25 additive tools | Mutual core |
| Use an already open, authenticated browser | Optional through CDP auto-connect or a chosen profile | Yes, the primary mode; only managed Ghostlight tabs | Ghostlight's defining default |
| Launch a fresh isolated browser | Yes, default daemon workflow | No -- boundary | Do not copy |
| Run headless, in a sandbox, or through a cloud provider | Yes | No -- boundary | Do not copy |
| Navigate, back, forward, and reload | `open`, `back`, `forward`, `reload` | `navigate`, `tab_control` | Mutual core |
| Read page text | `read`, `snapshot` | `get_page_text`, `read_page` | Mutual; Ghostlight adds provenance and bounded payloads |
| Get an accessibility snapshot with reusable refs | `snapshot` with `@eN` refs | `read_page` with session refs | Mutual core |
| Find by role, text, label, placeholder, alt, title, or test id | `find` variants | `find` deterministic semantic ranking | Mutual; Ghostlight returns bounded ambiguity evidence |
| Act directly on a semantic target | `find ... <action>` | `act_on` | Mutual; Ghostlight closes observe-act-observe in one governed call |
| Click, double-click, hover, focus, and scroll | Direct commands | `computer`, `act_on` | Mutual core |
| Type and send keyboard input | `fill`, `type`, `press`, keyboard commands | `computer`, `form_input`, `form_fill`, `act_on` | Mutual core |
| Select, check, and uncheck form controls | Direct commands | `form_input`, `form_fill`, `act_on` | Mutual outcome, different surface |
| Drag and drop | `drag` | `computer` drag actions | Mutual core |
| Upload a local file | `upload` | `file_upload` | Mutual core |
| Reuse a captured image as page input | No dedicated screenshot-cache workflow | `upload_image` | Ghostlight advantage |
| Wait for element, text, URL, load, or condition | `wait` variants | `wait_for`, `computer` wait | Mutual; exact condition vocabulary differs |
| Take screenshots | PNG/JPEG, full page, optional ref annotations | Token-budgeted screenshots on screenshot/scroll/zoom | Mutual; annotated output is a candidate |
| Save a page as PDF | Yes | No -- candidate | Useful mainly for testing/export, not the core loop |
| Query text, HTML, value, attribute, count, box, or styles | Dedicated `get` commands | Partial through targeted `read_page`, `find`, and JavaScript | Avoid adding a call-per-fact API; enrich existing observations only when it saves turns |
| Check visible, enabled, and checked state | Dedicated `is` commands | Included in actionable element summaries | Mutual outcome; Ghostlight batches the facts |
| Run JavaScript | `eval` | `javascript_tool` | Mutual; Ghostlight classifies it Execute |
| Handle JavaScript dialogs | `dialog` | `dialog` | Mutual core |
| List, create, focus, reload, and close tabs | `tab` | `tabs_context_mcp`, `tabs_create_mcp`, `tab_control` | Mutual; Ghostlight enforces session ownership |
| Open arbitrary windows | Yes | No -- candidate | Weak agent value; can break the managed-workspace contract |
| Read and act through iframes | Same- and cross-origin frame support | Same-origin only; cross-origin deferred | Real gap, but governance must authorize every frame origin first |
| Batch several commands in one transport turn | `batch` and shell chaining | `browser_batch`, `script`, `form_fill` | Mutual; Ghostlight adds correlated audit and structured stop reasons |
| Read console messages and page errors | `console`, `errors` | `read_console_messages` | Mutual for ordinary diagnosis |
| Inspect network requests | Request detail and filters | `read_network_requests` | Mutual for bounded observation |
| Intercept, abort, or mock network traffic; export HAR | Yes | No -- boundary for general use | Testing/runtime feature that can rewrite the user's real session |
| Control downloads | Click-and-wait with download paths | No -- candidate | Requires an explicit local-filesystem and retention design |
| Read or write the system clipboard | Yes | No -- candidate | High local-data surface for modest browser-loop value |
| Read and mutate cookies or web storage | Yes | No -- boundary for routine agent use | Directly changes authentication state outside page intent |
| Save, restore, and encrypt browser state | Yes | No -- boundary | Ghostlight deliberately uses the user's current state, not copied state |
| Emulate viewport, device, geolocation, offline mode, headers, auth, or media | Yes | Only window resize | No for the ordinary product; belongs in a testing product |
| Record the session | WebM recording and streaming | Memory-only GIF recording with truthful REC state | Mutual goal, different artifact and privacy contract |
| Trace, profile, highlight, inspect DevTools, or probe WebGPU | Yes | Target glow only | Specialist developer-testing breadth, not low-hanging agent value |
| Diff snapshots, screenshots, or URLs | Yes | No -- candidate | Useful for QA journeys; separate evidence and retention design needed |
| Inspect React internals and Web Vitals | Yes | No -- candidate | Framework-specialist tooling, not general browser agency |
| Run built-in AI chat | Yes, optional AI Gateway | No -- boundary | MCP client already supplies the model; no account or gateway needed |
| Show live browser/session dashboard | Yes | Read-only local Console plus extension popup | Partial; Ghostlight focuses on governance and recovery state |
| Diagnose installation | `doctor` | `ghostlight doctor` | Mutual product hygiene |
| Restrict domains | Allowed-domain patterns | Grants, sacred domains, and exact host resolution | Mutual baseline; Ghostlight is deeper |
| Restrict action classes | Action policy and optional confirmations | RAWX requirements, grants, observe/enforce, org locks | Mutual concept; different policy model |
| Pause repeated blocked behavior | No equivalent documented burst circuit | Per-session denial attention circuit | Ghostlight advantage |
| Attribute actions to a declared subject and MCP client | Session names, not an org identity model | Manifest identity plus MCP `clientInfo` | Ghostlight governance advantage |
| Produce durable policy-decision audit | No equivalent identity-bound audit contract | JSONL/syslog decisions, denial ids, policy provenance, attention transitions | Ghostlight governance advantage |
| Bound untrusted page output for the model | Content boundaries and max output | Service-authored nonce boundaries, provenance, and per-tool budgets | Mutual goal; Ghostlight integrates it with receipts |
| Keep the everyday model surface small | Default core profile; paginated optional breadth | Fixed 25-tool registry plus policy-filtered advertisement | Mutual priority |

## The overlap map

```text
Shared ordinary agent loop
  navigation, semantic refs, reads, clicks, forms, waits, screenshots,
  JavaScript, dialogs, tabs, console/network observation, batching, recording

agent-browser specialist side
  fresh/headless/cloud sessions, copied/restored state, cookies/storage,
  interception/HAR, emulation, downloads/clipboard, PDF, traces/profiling,
  React/Vitals, visual diffs, providers, built-in AI chat

Ghostlight specialist side
  existing local user context by default, managed-tab ownership, RAWX grants,
  sacred domains, observe/enforce, declared identity, structured audit,
  content-free interaction receipts, denial burst pause, no account/gateway
```

## Strategic opportunities through the three delight lenses

### 1. Make visual and semantic evidence converge

Candidate: an optional annotated screenshot that uses the same refs as `read_page` and `find`.
The image should label only the bounded interactive set already exposed to the model, return the
ref legend as structured content, and keep raw screenshots available.

- Model delight: visual layout and exact semantic handles arrive in one result.
- User delight: the visible target vocabulary matches what the model says it sees.
- Governance delight: the annotation adds no new action authority or page-content logging.

This is the best small capability gap exposed by the comparison. It needs an ADR amendment because
it changes screenshot payload shape and token budgeting, but not a new browser mode.

### 2. Add memorable labels to owned tabs

Candidate: optional model-chosen labels on tab creation, returned by tab context and accepted by
tab control in addition to numeric ids. Labels remain session-owned presentation metadata and are
never inferred from page content.

- Model delight: `invoice` survives tab-list changes better in a plan than a large composite id.
- User delight: popup and narration can say which workspace tab is active.
- Governance delight: ownership and authorization still resolve to the exact internal tab id.

This is additive and low risk, but its token savings should be measured before it displaces other
work. Stable ids already prevent the correctness failure; labels improve comprehension.

### 3. Prefer richer existing observations over command breadth

Do not clone `get text`, `get value`, `is visible`, `get box`, and `get styles` as separate MCP
tools. Extend the actionable summary only when a fact repeatedly causes another roundtrip. Keep
full computed styles and arbitrary HTML behind explicit JavaScript or a future diagnostic mode.

- Model delight: one bounded observation answers the next decision.
- User delight: fewer mechanical flashes and retries.
- Governance delight: fewer calls and audit rows for one intent, with no page meaning in policy.

ADR-0078 already implements the highest-value version of this strategy. The next step is journey
measurement, not another general-purpose inspector.

### 4. Turn recovery state into the governance feature people feel

The denial attention circuit, compact narration, transient sticker, and truthful capture signals
are the immediate response. The next governance decision is not a generic prompt on every action.
ADR-0075 proposes risk-scoped confirmation, where policy can require a human decision for selected
high-risk calls. It remains proposed because prompt lifetime, client support, tab races, and audit
semantics must be settled before it can be safe or pleasant.

The strategic sequence is:

1. prove that repeated-denial recovery is calm and understandable;
2. collect evidence about where users actually want confirmation;
3. amend or replace ADR-0075 with a narrow, measured design;
4. never turn all-open personal use into approval fatigue.

### 5. Keep the testing side deliberately porous, not built in

Downloads, PDF, visual diff, performance diagnostics, and framework inspection are useful. They
are not all browser-agency primitives. Ghostlight should compose with agent-browser, Playwright,
or ordinary developer tools for those jobs until repeated MCP journeys show a clear benefit from
bringing one inside the live-user-context boundary.

This preserves a free, capable Ghostlight without making the model load or govern a testing suite
on every ordinary task.

## Recommendation

Ship the implemented experience closure first. Then prototype only two free-surface additions:
ref-linked annotated screenshots and optional owned-tab labels. Gate both on measured reductions in
calls or recovery turns. Keep headless, isolated, cloud, state-copy, cookie/storage mutation,
network interception, and built-in model hosting outside Ghostlight. Revisit cross-origin frames
only with a multi-origin authorization design.

The strategic target is not feature parity. It is a smaller surface that lets a model understand
more, act with fewer turns, show the user what is happening, and leave governance evidence that is
useful without collecting the page.
