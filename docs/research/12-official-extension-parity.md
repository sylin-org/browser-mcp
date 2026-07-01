# Official-Extension Parity + Technique Harvest

Status: in progress (harvest workflow running at last checkpoint). This doc is the durable record
of the parity-verification thread and the plan to re-baseline our tool surface against the
**official** Claude-in-Chrome extension rather than the community reference.

## Why

The sacred contract (CLAUDE.md) is that our tool surface is **byte-identical to what the official
Claude in Chrome extension advertises** -- because the model was trained against those schemas.
Until now our schemas came from the *community reference* (`reference/open-claude-in-chrome`, a
Node.js re-implementation). Verification proved the reference is a **lossy proxy** that carries its
own bugs, so "we match the reference" is weaker than the real goal. The official extension is the
ground truth.

## Parity findings vs the COMMUNITY reference (verify-vs-reference workflow)

Exercised all 13 tools live against Chrome. Three behavioral symptoms observed, and side-by-side
code reading showed **all three are inherited verbatim from the reference, NOT rewrite regressions**:

- **A) `read_network_requests` returns empty on first call** -- both sides enable the CDP `Network`
  domain *lazily* (only inside the handler). `Network.enable` is not retroactive, so the page-load +
  pre-read fetches are never captured. Our wiring is actually *better* than the reference (joins
  `requestWillBeSent`+`responseReceived` by `requestId` vs the reference's method-guessing).
- **B) `read_console_messages` duplicates** -- both listen to `Runtime.consoleAPICalled` AND
  `Console.messageAdded` with no dedup. Inherited double-count. **Fixed** (single Runtime source).
- **C) `find` matches only literal text** -- identical whole-string substring algorithm to the
  reference over `role name text placeholder ariaLabel title type tag`. "Submit button" is not a
  literal substring, so it misses; "Example Domain" hits. The reference's own `find` description
  over-promises "search by purpose"; our sacred schema preserves that gap verbatim.

Real gaps the parity sweep found (relative to the reference), pending the official baseline:
- **`read_page` omits node attributes the reference emits**: `img src`, `aria-expanded/checked/
  selected`, `<select>` options. Medium -- loses state signals the model reads. (`extension/content.js` ~121-130)
- **`form_input` checkbox/radio truthiness**: ours accepts `1`/`"1"`/nonzero as check; reference
  treats them false. This is a **deliberate earlier fix** (commit 0deef1c) -- keep ours.
- Low-severity text/format/timing diffs (navigate `## Pages` list, `get_page_text` `Source:` line,
  tabs shape, hover settle delay, scroll coordinate validation, zoom mimeType) -- mostly deliberate
  lean choices; decide per-tool against the OFFICIAL, not the reference.

## The official extension (ground truth)

- Name "Claude", description "Claude in Chrome (Beta)", **version 1.0.78**, id
  `fcoeoabgfenejglbffodgkkbkcdhcgfn`.
- Installed at:
  `C:\Users\onose\AppData\Local\Google\Chrome\User Data\Default\Extensions\fcoeoabgfenejglbffodgkkbkcdhcgfn\1.0.78_0\`
- Architecture matches ours: MV3, `debugger` (CDP), `tabGroups`, `nativeMessaging`; content scripts
  `accessibility-tree.js` (all_urls) + `agent-visual-indicator.js`; service worker bridges to
  claude.ai / api.anthropic.com / `wss://bridge.claudeusercontent.com`.
- Key files (bundled/minified, but plain JS):
  - `assets/mcpPermissions-E9qdF7bb.js` (693 KB) -- **the MCP tool DEFINITIONS/schemas + the CDP
    execution logic**. The core harvest target (28,715 lines beautified).
  - `assets/accessibility-tree.js-CCweLwU2.js` -- the `read_page`/`find`/`get_page_text` engine
    (220 lines beautified).
  - `assets/service-worker.ts-CRgYaSdM.js` -- bootstrap / native-messaging bridge (2,380 lines).

### Re-extracting the official files for study (they live in the session scratchpad, ephemeral)

```
SRC=".../Extensions/fcoeoabgfenejglbffodgkkbkcdhcgfn/1.0.78_0/assets"
OUT="<scratchpad>/official-ext"; mkdir -p "$OUT"
cp "$SRC/mcpPermissions-E9qdF7bb.js"        "$OUT/mcpPermissions.min.js"
cp "$SRC/accessibility-tree.js-CCweLwU2.js" "$OUT/accessibility-tree.min.js"
cp "$SRC/service-worker.ts-CRgYaSdM.js"     "$OUT/service-worker.min.js"
npx --yes js-beautify "$OUT/mcpPermissions.min.js" > "$OUT/mcpPermissions.pretty.js"   # etc.
```

## Discipline (hard boundary)

We harvest the observable **interface** (tool names/params/enums/description strings) and the
**techniques** (CDP command sequences, algorithms) and **reimplement leanly**. We do **NOT** copy
official code into our repo (it is Anthropic proprietary; our repo is intended open-source). The
beautified official files stay in the throwaway scratchpad, never tracked. Interface + intent, not
code -- consistent with the project's "not a port" principle.

## Next steps (the apply plan)

1. Read the harvest workflow result (study of the official extension) -- schema corrections +
   prioritized technique adoptions + what to keep.
2. Apply **schema corrections** to `src/mcp/schemas/tools.json` to match the official surface, and
   update the golden fixture in `tests/tool_schema_fidelity.rs` accordingly (the fidelity test guards
   the sacred surface, so it must be re-baselined to the official, not the reference).
3. Adopt **battle-tested techniques** in `extension/*.js` where ours diverges (likely: `read_page`
   attributes, console/network capture timing, any `computer`/screenshot coordinate details).
4. Keep the deliberate improvements (`form_input` truthiness, the requestId network join).
5. Reload the unpacked extension in Chrome to test behavior changes (extension changes need a reload;
   the Rust side is unaffected).
