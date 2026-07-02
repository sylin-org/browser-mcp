# T08: computer type dispatches real keyDown/keyUp per character with Enter mapping

## Goal

The `computer` tool's `type` action currently inserts each character with
`Input.insertText`, which sets the field value but fires no keyboard events,
so pages listening to keydown/keyup (search-as-you-type, React controlled
inputs, key-driven validation, Enter-to-submit) never react. Rework the type
loop to dispatch a real `Input.dispatchKeyEvent` keyDown/keyUp pair per
printable ASCII character, map newlines to a real Enter press, and fall back
to `Input.insertText` only for characters with no key mapping.

## Project context

Browser MCP is a governed browser automation system. A single Rust binary is
both the MCP server (JSON-RPC 2.0 over stdio, hand-rolled, tokio) and the
Chrome native-messaging host; a thin Manifest V3 extension executes CDP
commands. Architecture:

```
MCP Client <--stdio--> Binary <--native messaging--> Extension <--CDP--> Browser
```

The two binary roles run as separate OS processes bridged by tokio-native
named-pipe/UDS IPC. The extension holds mechanism only: all policy, access,
and redaction decisions live in the Rust binary.

Key files:

- `src/mcp/server.rs`: JSON-RPC loop in the binary. Not touched by this task.
- `src/mcp/schemas/tools.json`: SACRED. Byte-frozen official tool schemas.
  Never edit. Guarded by `tests/tool_schema_fidelity.rs`.
- `extension/service-worker.js`: CDP dispatch, screenshot pipeline,
  console/network buffers, keyboard/mouse dispatch. This is the ONLY file
  this task touches.
- `extension/content.js`: accessibility tree, find, form_input, page text.
  Not touched by this task.
- `extension/agent-visual-indicator.js`: phantom cursor and glow overlays.
  Not touched by this task.

Build and test: run `cargo test` from the repo root; all tests must pass.
This task changes only extension JavaScript, so no Rust rebuild is required,
but run `cargo test` anyway to prove nothing regressed. Extension changes
require the user to reload the extension at chrome://extensions to take
effect. If you ever need to rebuild the binary and
`target/debug/browser-mcp.exe` is locked by a running session, rename it
aside first (for example:
`mv target/debug/browser-mcp.exe target/debug/browser-mcp.exe.old-1`) and
rebuild.

## Current behavior

All facts below were verified by reading `extension/service-worker.js`
(568 lines).

- The `type` action lives in the `computer(a)` dispatcher (function starts at
  line 357), at lines 390-395:

  ```js
  case "type": {
    if (!a.text) return text("text is required for type.");
    await ensureAttached(tabId);
    for (const ch of a.text) { await cdp(tabId, "Input.insertText", { text: ch }); await sleep(8); }
    return text(`Typed ${a.text.length} character(s).`);
  }
  ```

  Every character goes through `Input.insertText`, so no keydown, keypress,
  or keyup events reach the page, and newlines are inserted as literal text
  instead of pressing Enter.

- `computer(a)` declares `const modifiers = modifierBits(a.modifiers);` at
  line 360. The `type` case does not use it. `modifierBits` (lines 260-268)
  and `pressKey` (lines 295-298) both use the CDP modifier bit values:
  alt = 1, ctrl = 2, meta = 4, shift = 8.

- `pressKey(tabId, combo)` at lines 288-320 is the existing single-key /
  chord dispatcher used by the `key` action (lines 396-404). It already
  builds real key events: it computes `const code = keyCode(key);` and
  `const vk = vkCode(key);`, builds
  `const evt = { key, code, modifiers, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };`,
  and dispatches `Input.dispatchKeyEvent` with `type: "keyDown"` then
  `type: "keyUp"` (lines 314-318), followed by `await sleep(20)`.

- `keyCode(key)` at lines 322-328 maps a resolved key to a best-effort DOM
  `code`: single letters become `"Key" + upper`, single digits become
  `"Digit" + d`, everything else returns the key name unchanged (so
  punctuation like `,` currently gets a wrong `code` of `","`, and `" "`
  gets `" "`).

- `VK_NAMED` at lines 330-334 maps named keys to Windows virtual key codes:
  Enter 13, Tab 9, Escape 27, Backspace 8, Delete 46, `" "` 32, ArrowUp 38,
  ArrowDown 40, ArrowLeft 37, ArrowRight 39, Home 36, End 35, PageUp 33,
  PageDown 34, Insert 45.

- `vkCode(key)` at lines 335-342 returns the Windows virtual key code:
  single letters map to 65-90 via `toUpperCase().charCodeAt(0)`, single
  digits map to 48-57, otherwise `VK_NAMED[key] || 0`. Punctuation
  characters currently return 0.

- `KEY_MAP` at lines 253-259 normalizes human key names (enter, esc, up,
  space, ...) for `pressKey`. It is not involved in `type`.

- `sleep(ms)` is at lines 242-244. `cdp(tabId, method, params)` is at lines
  114-117 and calls `ensureAttached` itself. `text(t)` at lines 207-209
  builds the `{ content: [{ type: "text", text: t }] }` result envelope.

## Required behavior

All changes go in `extension/service-worker.js`. Implement the behavior from
scratch exactly as specified below; this document is the complete
specification and nothing else needs to be consulted.

### 1. Extend the existing keyboard helpers (do not duplicate them)

Add two module-level lookup tables next to `VK_NAMED` (around lines
330-334), in the same compact style, covering the eleven US-QWERTY
punctuation keys:

```js
// Windows virtual key codes for US-QWERTY punctuation keys (VK_OEM_*).
const VK_PUNCT = {
  ";": 186, "=": 187, ",": 188, "-": 189, ".": 190, "/": 191,
  "`": 192, "[": 219, "\\": 220, "]": 221, "'": 222,
};
// DOM `code` values for US-QWERTY punctuation keys (and Space).
const CODE_PUNCT = {
  ";": "Semicolon", "=": "Equal", ",": "Comma", "-": "Minus",
  ".": "Period", "/": "Slash", "`": "Backquote", "[": "BracketLeft",
  "\\": "Backslash", "]": "BracketRight", "'": "Quote", " ": "Space",
};
```

Extend `keyCode(key)` so that, for single characters, after the existing
letter and digit branches it returns `CODE_PUNCT[key]` when present (this
also fixes `" "` to `"Space"`); the final fallback (return the key name)
stays as is.

Extend `vkCode(key)` so that, for single characters, after the existing
letter and digit branches it returns `VK_PUNCT[key]` when present; the final
`VK_NAMED[key] || 0` fallback stays as is.

Do not change the body of `pressKey` itself. It picks up the improved
punctuation `code`/vk values automatically through the shared helpers; that
side effect is intended.

### 2. Add a shifted-character table and a per-character resolver

Add, next to the tables above:

```js
// US-QWERTY: shifted printable -> the unshifted character on the same key.
const SHIFT_BASE = {
  "!": "1", "@": "2", "#": "3", "$": "4", "%": "5", "^": "6",
  "&": "7", "*": "8", "(": "9", ")": "0",
  "_": "-", "+": "=", "{": "[", "}": "]", "|": "\\", ":": ";",
  '"': "'", "<": ",", ">": ".", "?": "/", "~": "`",
};
```

Add a resolver function (place it after `vkCode`) named `charKeyInfo(ch)`
that maps one character of typed text to real key event fields, or `null`
when the character has no key mapping:

- If `ch` is `"\n"` or `"\r"`, return
  `{ key: "Enter", code: "Enter", vk: 13, shift: false, text: "\r", unmodifiedText: "\r" }`.
- If `ch` is outside printable ASCII (`ch < " " || ch > "~"`), return
  `null`. This covers control characters such as `"\t"` and all non-ASCII
  text; they take the insertText fallback described below. Do NOT map
  `"\t"` to a Tab key press (a Tab press moves focus, which is wrong while
  typing text).
- Otherwise compute the unshifted base character and the shift flag:
  uppercase letters `"A"`-`"Z"` have `base = ch.toLowerCase()` and
  `shift = true`; characters present in `SHIFT_BASE` have
  `base = SHIFT_BASE[ch]` and `shift = true`; everything else has
  `base = ch` and `shift = false`.
- Return
  `{ key: ch, code: keyCode(base), vk: vkCode(base), shift, text: ch, unmodifiedText: base }`.

Note the field meanings: `key` and `text` are the character actually
produced (for example `":"`), `code` and `vk` describe the physical key (for
example `"Semicolon"` / 186), and `unmodifiedText` is what that key would
produce without Shift (for example `";"`). With the helper extensions from
step 1, every one of the 95 printable ASCII characters (letters, digits,
space, and all 32 punctuation marks) resolves to a non-null entry with a
non-zero vk; only control characters and non-ASCII return `null`.

### 3. Rewrite the type loop

Replace the body of `case "type"` (lines 390-395) with the following exact
behavior. Keep the guard line and the return message byte-identical to
today: the guard is `if (!a.text) return text("text is required for type.");`
and the success return is
`return text(\`Typed ${a.text.length} character(s).\`);`. Keep the
`await ensureAttached(tabId);` call.

Iterate the text by Unicode code point with lookahead. Use
`const chars = Array.from(a.text);` and an indexed `for` loop (a plain
indexed loop over `a.text` would split surrogate pairs; `Array.from`
preserves the code-point iteration the current `for...of` loop has while
allowing lookahead).

For each character `ch = chars[i]`:

1. CRLF collapsing: if `ch === "\r"` and `chars[i + 1] === "\n"`, `continue`
   (skip the `"\r"`; the following `"\n"` produces the single Enter press).
   Without this, Windows-style newlines would press Enter twice.
2. Resolve `const info = charKeyInfo(ch);`.
3. If `info` is `null`, fall back for this character only:
   `await cdp(tabId, "Input.insertText", { text: ch });` then
   `await sleep(8);` and `continue;`.
4. Otherwise dispatch a real keyDown/keyUp pair. Use the modifier-bits
   approach uniformly: do NOT dispatch separate Shift keyDown/keyUp events;
   instead set bit 8 on both events of a shifted character. Name the local
   variable `mods` (NOT `modifiers`) so it does not shadow the `modifiers`
   binding declared at the top of `computer()` (line 360):

   ```js
   const mods = info.shift ? 8 : 0;
   const evt = {
     key: info.key, code: info.code, modifiers: mods,
     windowsVirtualKeyCode: info.vk, nativeVirtualKeyCode: info.vk,
   };
   await cdp(tabId, "Input.dispatchKeyEvent", { type: "keyDown", ...evt, text: info.text, unmodifiedText: info.unmodifiedText });
   await cdp(tabId, "Input.dispatchKeyEvent", { type: "keyUp", ...evt });
   await sleep(8);
   ```

   The keyDown carries `text` and `unmodifiedText` (this is what makes CDP
   deliver keydown plus keypress plus the text insertion); the keyUp carries
   no text fields. The 8 ms inter-character delay matches the current
   implementation's pacing; keep it exactly.

Do not add a try/catch around the dispatch calls. If a CDP call throws, let
the error propagate exactly as the current code does; the engine reports
failures truthfully rather than pretending the text was typed.

### 4. What stays the same

- The `key` action and `pressKey` bodies are untouched (the shared helper
  extensions in step 1 are the only thing they feel).
- The result messages of `type` are byte-identical to today (both the guard
  message and the success message quoted in step 3).
- `type` continues to ignore `a.modifiers` (line 360's `modifiers`); the
  only modifier applied is the per-character Shift bit.

## Constraints

1. NEVER modify `src/mcp/schemas/tools.json`, tool names, parameters, or
   description strings. `tests/tool_schema_fidelity.rs` must pass unchanged.
2. The extension holds mechanism only: no policy, access, or redaction
   decisions in extension JS.
3. ASCII only in ALL code and docs: no em-dashes, no arrows, no curly
   quotes, anywhere, including comments.
4. The engine is truthful: never fake success, never silently substitute
   behavior; when something failed or was recovered, say so in the tool
   result text. For this task that means: no swallowing CDP errors, and no
   changing the result text to claim more than what happened.
5. No new runtime dependencies. The extension stays vanilla JS (no bundler,
   no libraries).
6. Rust: 2021 edition, thiserror for typed errors in library code, doc
   comments on public items, rustfmt clean, clippy with deny warnings.
   (No Rust changes are expected in this task.)
7. Comments only for constraints the code cannot express; match the
   surrounding comment density and style (short `//` lines, like the ones
   above `VK_NAMED` and inside `pressKey`).
8. Do NOT copy code from the official Anthropic extension or any other
   project; implement the behavior described above from scratch.
9. This task touches `extension/service-worker.js` and nothing else.

## Verification

1. `cargo test` from the repo root: all tests pass (this task changes no
   Rust, so this is a regression check, including
   `tests/tool_schema_fidelity.rs`).
2. Ask the user to reload the extension at chrome://extensions (extension
   changes are not picked up otherwise). No MCP client restart is needed
   because no binary or schema changed.
3. Manual end-to-end check through an MCP client:
   - `navigate` to https://example.com.
   - `javascript_tool` to prepare a probe input:

     ```js
     const inp = document.createElement("input");
     inp.id = "t08probe";
     document.body.prepend(inp);
     window.__ev = [];
     for (const t of ["keydown", "keyup", "input"]) {
       inp.addEventListener(t, (e) => window.__ev.push(
         t + "|" + (e.key || "") + "|" + (e.code || "") + "|" + (e.shiftKey ? 1 : 0)));
     }
     inp.focus();
     ```

   - `computer` with `{ "action": "type", "text": "Ab1!;:\n" }` on that tab.
   - `javascript_tool` to read
     `JSON.stringify({ v: document.getElementById("t08probe").value, ev: window.__ev })`
     and verify:
     - the value is `Ab1!;:` (Enter adds no character to a single-line
       input);
     - every typed character produced a keydown and a keyup;
     - `A` shows `keydown|A|KeyA|1` (shift bit set), `b` shows
       `keydown|b|KeyB|0`, `1` shows `keydown|1|Digit1|0`, `!` shows
       `keydown|!|Digit1|1`, `;` shows `keydown|;|Semicolon|0`, and `:`
       shows `keydown|:|Semicolon|1`;
     - the final entries are `keydown|Enter|Enter|0` and
       `keyup|Enter|Enter|0`.
   - Fallback check: `computer` with
     `{ "action": "type", "text": "caf\u00e9" }` (the JSON escape keeps
     this file ASCII; the argument decodes to a word ending in an accented
     e). Verify the input value ends with the accented character
     and that `window.__ev` shows keydown/keyup pairs for `c`, `a`, `f` but
     only an `input` entry (no keydown) for the accented character, proving
     the per-character insertText fallback fired for it alone.
   - CRLF check: type `"a\r\nb"` and confirm exactly one Enter
     keydown/keyup pair between `a` and `b`.
4. Confirm the tool result text still reads exactly
   `Typed N character(s).` with N equal to the raw length of the `text`
   argument.

## Out of scope

- The `key` action and the `pressKey` function body. The key action was
  fixed separately; do not rework, refactor, or "improve" it beyond the
  shared `keyCode`/`vkCode`/table extensions specified in step 1.
- `form_input` (in `extension/content.js`) and any other tool. Do not touch
  content.js at all.
- Any IME or composition support (no `Input.imeSetComposition`, no
  composition events).
- Wiring `a.modifiers` into the type action, holding a physical Shift key
  down across characters, or emitting separate Shift keyDown/keyUp events.
- Changing the type action's result message, the 8 ms pacing, the
  `Input.insertText` fallback semantics for control characters such as
  `"\t"`, or the screenshot policy (type returns text only, never an
  image).
- Non-US keyboard layouts. The keymap is US-QWERTY by design.
- Any Rust code, schema, test, or documentation changes.
