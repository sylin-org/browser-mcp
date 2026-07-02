# T03: get_page_text official semantics (largest-candidate innerText, Source header, max_chars)

## Goal

Rework the `get_page_text` extraction so it picks the content region with the LARGEST
`innerText` among a fixed candidate selector set (falling back to `document.body`), reads
layout-aware `innerText` instead of cloned `textContent`, honors the `max_chars` argument
(default 50000), and starts its output with a `Source element:` header. When the page has
no readable text or the text exceeds `max_chars`, return an actionable message instead of
silence.

## Project context

Browser MCP is a governed browser automation system. A single Rust binary is both the MCP
server (JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the Chrome native-messaging host.
A thin Manifest V3 extension executes CDP commands and DOM reads. Architecture:

    MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser

The two binary roles run as separate OS processes bridged by tokio-native named-pipe/UDS
IPC. All policy, redaction, and audit decisions live in the Rust binary. The extension is
mechanism only.

How a `get_page_text` call flows today (all verified by reading the code):

1. The MCP client sends `tools/call` with `name: "get_page_text"` and `arguments` such as
   `{ "tabId": 123, "max_chars": 20000 }`. The binary (`src/mcp/server.rs`, around lines
   121-135) extracts `arguments` and forwards it VERBATIM to the extension via
   `browser.call(name, &args)`. So `max_chars` already reaches the extension untouched;
   no Rust change is needed for this task.
2. The extension service worker receives `{ type: "tool_request", id, tool, args }` on the
   native port (`extension/service-worker.js`, lines 31-34) and calls
   `dispatch(id, tool, args)` (line 558), which invokes `handlers[tool](args)`.
3. The `get_page_text` handler (service-worker.js, lines 484-488) checks tab-group
   membership, then bridges to the content script via `content(tabId, message)`
   (lines 197-204; injects `content.js` on demand if messaging fails).
4. The content script (`extension/content.js`) answers the `{ type: "pageText" }` message
   (line 299) by calling `pageText()` (lines 195-204) and returns a plain string, which
   the service worker wraps in an MCP text result via `text()` (line 207).

Files involved in this task:

- `extension/content.js` (edit). Content script injected into pages. Implements the
  accessibility tree for `read_page`, plus `find`, `form_input`, and page text
  extraction. Vanilla JS, no bundler, no libraries.
- `extension/service-worker.js` (edit ONE line). Its `get_page_text` handler must start
  forwarding `max_chars` to the content script.
- `src/mcp/schemas/tools.json` (NEVER edit). Byte-frozen official Claude-in-Chrome
  v1.0.78 tool schemas. The `get_page_text` schema (lines 169-187) already advertises
  `max_chars` as an optional number whose description states the 50000 default; only
  `tabId` is required.
- `tests/tool_schema_fidelity.rs` (never edit). Guard test; lines 138-143 assert that
  `get_page_text` advertises `max_chars`. Must keep passing.
- `extension/agent-visual-indicator.js` (do not edit). Its overlay elements use
  `browser-mcp-*` ids and render no readable text (a cursor glyph and a glow border,
  both `aria-hidden`), so `innerText` extraction needs no special handling for them.

Build and test: run `cargo test` from the repo root; all tests must pass. This task
changes only extension JS, so no Rust rebuild is required, but run `cargo test` anyway to
prove nothing regressed. Extension changes only take effect after the user reloads the
unpacked extension at chrome://extensions (a reload covers both the service worker and
the content script). Binary or schema changes (none expected here) would require an MCP
client restart. If you ever need to rebuild and `target/debug/browser-mcp.exe` is locked
by a running session, rename it aside first (for example:
`mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and rebuild.

## Current behavior

All facts below were verified by reading the files at the time this prompt was written.
Re-verify before editing; line numbers may have drifted.

`extension/content.js`, the `// --- Page text ---` section (lines 194-204):

- Line 196 defines the candidate list:
  `["article", "main", '[role="main"]', '[class*="article"]', '[class*="post-content"]', ".content", "#content"]`
- Line 198 picks the FIRST selector that matches anything:
  `for (const sel of selectors) { source = document.querySelector(sel); if (source) break; }`
  A page with a tiny `<article>` teaser and its real content in `.entry-content` returns
  the teaser.
- Line 199 falls back to `document.body` when nothing matched.
- Lines 200-201 clone the node (`cloneNode(true)`) and remove
  `script, style, noscript, template, svg` from the clone.
- Line 202 reads `clone.textContent`, collapses ALL whitespace runs to single spaces
  (`.replace(/\s+/g, " ")`), trims, and hard-slices at 100000 characters. Consequences:
  `textContent` includes text hidden by CSS (`display:none` menus, cookie banners,
  collapsed sections), and the whitespace collapse destroys every paragraph break, so
  the model receives one giant line.
- Line 203 returns `Title: ${document.title}\nURL: ${location.href}\n\n${t}`. There is
  no `Source element:` header.
- `pageText()` takes no parameters; `max_chars` is ignored end to end.
- Line 299, the message handler, forwards nothing:
  `case "pageText": sendResponse({ result: pageText() }); return true;`

`extension/service-worker.js`:

- Lines 484-488, the handler:

      async get_page_text(a) {
        if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
        const r = await content(a.tabId, { type: "pageText" });
        return text((r && r.result) || "Could not extract page text.");
      },

  Line 486 drops `a.max_chars` on the floor.

Precedent worth knowing: `accessibilityTree(options)` in content.js already honors
`options.max_chars || 50000` (line 123), so a `max_chars` default of 50000 is an
established convention in this file. Do NOT reuse or modify that function; `get_page_text`
gets its own independent handling.

## Required behavior

Two files change. Everything else stays untouched.

### 1. `extension/service-worker.js`: forward `max_chars`

In the `get_page_text` handler, change only the bridge line so the content script
receives the argument:

    const r = await content(a.tabId, { type: "pageText", max_chars: a.max_chars });

Keep the `inGroup` gate, the `text(...)` wrapping, and the
`"Could not extract page text."` fallback exactly as they are.

### 2. `extension/content.js`: replace the `// --- Page text ---` section

Replace the current `pageText()` implementation (and its inline `selectors` array) with
the following. The code below is the contract; reproduce its behavior exactly (you may
adjust trivial formatting to match the file, but not logic, strings, or defaults).

    // --- Page text ---
    // Main-content candidates. An element can match several selectors; the FIRST selector in
    // this list that finds it is the one reported in the "Source element:" header, and ties
    // on innerText length go to the earlier selector.
    const PAGE_TEXT_SELECTORS = [
      "article",
      "main",
      '[role="main"]',
      '[itemprop="articleBody"]',
      ".entry-content",
      ".content-body",
      ".article-body",
      ".articleBody",
      ".post-content",
      ".story-body",
      "#content",
      ".content",
    ];
    // Conservative cleanup only: innerText already excludes hidden text and preserves layout
    // line breaks, so just tidy line endings and keep paragraph breaks intact.
    function normalizePageText(t) {
      return t
        .replace(/\r\n?/g, "\n")
        .replace(/[ \t]+\n/g, "\n")
        .replace(/\n{3,}/g, "\n\n")
        .trim();
    }
    function pageText(maxCharsArg) {
      const maxChars = typeof maxCharsArg === "number" && Number.isFinite(maxCharsArg) && maxCharsArg >= 1
        ? Math.floor(maxCharsArg)
        : 50000;
      let bestEl = null, bestText = "", bestSel = "body";
      const seen = new Set();
      for (const sel of PAGE_TEXT_SELECTORS) {
        for (const el of document.querySelectorAll(sel)) {
          if (seen.has(el)) continue;
          seen.add(el);
          const t = el.innerText || "";
          if (t.length > bestText.length) { bestEl = el; bestText = t; bestSel = sel; }
        }
      }
      if (!bestEl || bestText.length === 0) {
        bestSel = "body";
        bestText = (document.body && document.body.innerText) || "";
      }
      const body = normalizePageText(bestText);
      if (body.length < 10) {
        return `No readable text content found (source element: ${bestSel}). The page may be mostly visual or may render text dynamically. Use read_page to inspect the page structure instead.`;
      }
      const header = `Source element: ${bestSel}\n\n`;
      if (body.length > maxChars) {
        return header + body.slice(0, maxChars) + `\n\n[Truncated at ${maxChars} characters. Retry with a larger max_chars, or use read_page to get a structured view with element refs.]`;
      }
      return header + body;
    }

Then change the message handler case (currently line 299) to pass the argument through:

    case "pageText": sendResponse({ result: pageText(msg.max_chars) }); return true;

Properties the implementation must guarantee (all already encoded in the snippet; keep
every one of them if you rephrase anything):

- Candidate selection scans ALL matches of every selector (`querySelectorAll`, not
  `querySelector`), dedupes elements via the `seen` set, and picks the element with the
  strictly largest `innerText` length. Because comparison is strict (`>`), ties go to
  the element found first, and the reported selector for a multi-matching element is the
  first selector in list order that found it.
- Fallback to `document.body` happens in exactly two cases: no selector matched anything,
  or the best candidate's raw `innerText` is empty. The header descriptor is then the
  literal string `body`. If `document.body` itself is missing, treat the text as empty.
- `innerText` only. No `textContent`, no `cloneNode`, no manual removal of
  `script`/`style`/etc. (`innerText` does not render those, and it excludes CSS-hidden
  text and preserves paragraph breaks by itself; that is the point of switching).
- Normalization is conservative and happens in this order: CR/CRLF to LF, strip trailing
  spaces and tabs at each line end, collapse runs of 3 or more newlines to exactly 2,
  trim the whole string. Nothing else. Single newlines and internal spacing survive.
- `max_chars` validation lives in the content script only: any finite number >= 1 is
  floored and used; anything else (absent, null, string, NaN, zero, negative) means
  50000. The service worker forwards the raw value without judging it.
- The three output shapes, checked in this order on the NORMALIZED text:
  1. Length < 10: return exactly the one-line no-readable-content message shown above
     (with `${bestSel}` substituted). No header line, no page text.
  2. Length > maxChars: return the header, then the first `maxChars` characters of the
     normalized text (plain `slice`, no word-boundary logic), then a blank line, then
     the bracketed truncation line shown above (with `${maxChars}` substituted).
  3. Otherwise: header plus full normalized text.
- `max_chars` budgets the normalized body text ONLY. The `Source element:` header and
  the truncation notice do not count against it.
- The old `Title:` and `URL:` lines are GONE. Output begins with `Source element:` (or
  the no-readable-content message). Do not re-add title or URL anywhere.
- The truncation and no-content messages are normal text results, not protocol errors:
  the string flows back through the existing `sendResponse` / `text()` path unchanged.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or description
   strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction decisions in
   extension JS. Content-region selection and truncation are mechanism; do not add any
   policy-flavored switches around them.
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly quotes, anywhere,
   including comments.
4. The engine is truthful: never fake success, never silently substitute behavior. That
   is why the truncation notice and the no-readable-content message exist; do not omit
   them, reword them, or return partial text without saying it was truncated.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler, no
   libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc comments on
   public items, rustfmt clean, clippy with deny warnings. (No Rust changes are expected
   in this task; the rule applies if you touch any.)
7. Comments only for constraints the code cannot express; match the surrounding comment
   density and style. The two short comments in the snippet above are the ceiling; do
   not add more.
8. Do NOT copy code from the official Anthropic extension or any other project;
   implement the behavior described above from scratch.

Task-specific:

9. Only `extension/content.js` and `extension/service-worker.js` may change, and in the
   service worker only the single bridge line inside the `get_page_text` handler.
10. The three output formats (header, no-readable-content message, truncation line) are
    contracts; reproduce them character for character with only the placeholders
    substituted.
11. Do not touch `accessibilityTree`, `find`, `setFormValue`, `refCoordinates`, the ref
    machinery, `visible()`, `sensitive()`, or any message-handler case other than
    `"pageText"`.
12. The default when `max_chars` is absent or invalid is exactly 50000, matching the
    sacred schema description.

## Verification

1. `cargo test` from the repo root: all tests pass, including
   `tests/tool_schema_fidelity.rs`, with zero changes to Rust files.
2. Re-read your final diff and confirm: `PAGE_TEXT_SELECTORS` contains exactly the twelve
   selectors listed, in that order; `pageText` uses `innerText` and never `textContent`
   or `cloneNode`; the under-10 check runs before the truncation check; the header and
   the two message strings match the contracts exactly; `Title:`/`URL:` lines are gone;
   both changed files are pure ASCII.
3. Manual end-to-end (requires the user, who must reload the extension at
   chrome://extensions after your edit; the binary and MCP client do not need a
   restart):
   - Navigate to a text-heavy article (for example a Wikipedia article) and call
     `get_page_text` with only `tabId`. The output starts with `Source element: `
     followed by one of the candidate selectors, and paragraphs are separated by blank
     lines (not one giant single-spaced line).
   - Call `get_page_text` with `max_chars: 500` on the same page. The output is the
     header, roughly 500 characters of text, a blank line, and the bracketed line
     `[Truncated at 500 characters. ...]` naming 500.
   - Navigate to `about:blank` and call `get_page_text`. The output is the single
     no-readable-content line naming `body` and suggesting `read_page`.
   - Hidden-text regression check: navigate to a simple page (for example example.com),
     run `javascript_tool` with
     `const d = document.createElement("div"); d.style.display = "none"; d.textContent = "HIDDEN_MARKER_XYZ"; document.body.appendChild(d); "ok"`,
     then call `get_page_text`. The output must NOT contain `HIDDEN_MARKER_XYZ` (the old
     `textContent` implementation would have included it).

## Out of scope

- Any pagination, offset, or cursor mechanism. The sacred schema has no `offset`
  parameter; do not add one to the schema, the native message, the handler, or the
  content script, and do not keep any state between calls to support resuming.
- Changing `read_page` (`accessibilityTree`) in any way, including its own `max_chars`
  handling, even though it looks similar.
- Readability heuristics beyond the fixed selector list plus the largest-innerText rule:
  no text-density scoring, no link-density filters, no Readability-style algorithms, no
  per-site rules.
- Re-adding `Title:` or `URL:` lines, or adding any other metadata lines to the output.
- Special-casing the `browser-mcp-*` overlay elements inside `pageText` (they render no
  readable text) or manually stripping `script`/`style` content (`innerText` already
  excludes both).
- Any edit to `extension/agent-visual-indicator.js`, `src/mcp/schemas/tools.json`,
  `tests/`, or any Rust file.
- Changing the service worker's tab-group gate, its `text()` result wrapper, or the
  `"Could not extract page text."` fallback string.
- New tool parameters, new message types, new configuration keys, or emitting the result
  as a protocol-level error instead of a text result.
