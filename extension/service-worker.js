// Browser MCP -- background service worker.
//
// Policy-free CDP executor + native-messaging endpoint + tab-group manager. It holds MECHANISM
// only; all governance (domains, tool classification, audit) lives in the Rust binary. It receives
// { id, type: "tool_request", tool, args } and replies { id, type: "tool_response", result } or
// { id, type: "tool_error", error, hop?, detail? }. `hop` (only ever "cdp" or "page") and `detail`
// are optional and are mechanism tags (which layer threw), never policy; an absent `hop` means the
// binary attributes the failure to the extension itself. Chrome frames native messages (4-byte LE)
// for us via the Port.

const NATIVE_HOST = "org.sylin.browser_mcp";
const GROUP_TITLE = "Browser MCP";

let nativePort = null;
let groupId = null;
const attached = new Map(); // tabId -> { domains: Set<string> }
const consoleBuffer = new Map(); // tabId -> { host, items: [{ level, text }] }
const networkBuffer = new Map(); // tabId -> { host, items: [{ requestId, method, url, status, mimeType, errorText, canceled }] }
const screenshotCtx = new Map(); // tabId -> { vpW, vpH, shotW, shotH } (set on each screenshot)
const tabHost = new Map(); // tabId -> hostname of the tab's current URL ("" when none)

// A rejected promise must not tear down the service worker.
self.addEventListener("unhandledrejection", (e) => e.preventDefault());

// --- Native messaging + Manifest V3 keepalive ---
chrome.alarms.create("keepalive", { periodInMinutes: 0.4 });
chrome.alarms.onAlarm.addListener((a) => {
  if (a.name === "keepalive" && !nativePort) connect();
});

function connect() {
  if (nativePort) return;
  try {
    nativePort = chrome.runtime.connectNative(NATIVE_HOST);
    nativePort.onMessage.addListener((msg) => {
      if (msg && msg.type === "tool_request" && msg.id) {
        dispatch(msg.id, msg.tool, msg.args || {});
      }
    });
    nativePort.onDisconnect.addListener(() => {
      nativePort = null;
      setTimeout(connect, 2000);
    });
  } catch {
    nativePort = null;
    setTimeout(connect, 2000);
  }
}

function reply(id, result) {
  try { nativePort && nativePort.postMessage({ id, type: "tool_response", result }); } catch { /* port gone */ }
}
// Tag an error with the hop (mechanism, not policy) that threw it, plus optional debug-only detail.
function hopError(hop, message, detail) {
  const err = new Error(message);
  err.hop = hop;
  if (detail) err.detail = String(detail);
  return err;
}
function fail(id, error) {
  const msg = { id, type: "tool_error", error: (error && error.message) || String(error) };
  if (error && error.hop) msg.hop = error.hop;
  if (error && error.detail) msg.detail = error.detail;
  try { nativePort && nativePort.postMessage(msg); } catch { /* port gone */ }
}

// --- CDP ---
const attaching = new Map(); // tabId -> in-flight attach promise (prevents concurrent double-attach)
async function ensureAttached(tabId) {
  if (attached.has(tabId)) return;
  if (attaching.has(tabId)) return attaching.get(tabId);
  const p = (async () => {
    try {
      await chrome.debugger.attach({ tabId }, "1.3");
    } catch (e) {
      throw hopError("cdp", `debugger attach failed: ${(e && e.message) || e}`);
    }
    attached.set(tabId, { domains: new Set() });
    try {
      const t = await chrome.tabs.get(tabId);
      tabHost.set(tabId, hostOf(t.url || ""));
    } catch { /* tab gone */ }
  })();
  attaching.set(tabId, p);
  try { await p; } finally { attaching.delete(tabId); }
}
// Coordinate model (harvest step 4, official v1.0.78): NO device-metrics override. Each screenshot
// probes the CSS viewport + DPR, captures at native resolution, downscales to a token budget, and
// records a per-tab ScreenshotContext. Model coordinates (read off that downscaled image) are then
// rescaled back to CSS viewport pixels before Input dispatch. ref-derived coordinates are already
// CSS px and are NOT rescaled.
const PX_PER_TOKEN = 28, MAX_TOKENS = 1568, MAX_SIDE = 1568, MAX_SCREENSHOT_B64 = 1100000;

async function probeViewport(tabId) {
  const r = await cdp(tabId, "Runtime.evaluate", {
    expression: "({w:innerWidth,h:innerHeight,d:window.devicePixelRatio||1})",
    returnByValue: true,
  });
  const v = r && r.result && r.result.value;
  if (!v || !v.w || !v.h) throw hopError("page", "failed to probe viewport");
  return { vpW: v.w, vpH: v.h, dpr: v.d || 1 };
}
// Target screenshot dimensions (derived from the CSS viewport) under the token + longest-side budget.
function targetDims(vpW, vpH) {
  let w = vpW, h = vpH;
  const tokens = Math.ceil(w / PX_PER_TOKEN) * Math.ceil(h / PX_PER_TOKEN);
  if (tokens > MAX_TOKENS) { const s = Math.sqrt(MAX_TOKENS / tokens); w = Math.round(w * s); h = Math.round(h * s); }
  const longest = Math.max(w, h);
  if (longest > MAX_SIDE) { const s = MAX_SIDE / longest; w = Math.round(w * s); h = Math.round(h * s); }
  return { w: Math.max(1, w), h: Math.max(1, h) };
}
function bytesFromBase64(b64) {
  const bin = atob(b64), bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}
function base64FromBytes(bytes) {
  let bin = "";
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  return btoa(bin);
}
async function encodeJpeg(bitmap, w, h, quality) {
  const canvas = new OffscreenCanvas(w, h);
  const ctx = canvas.getContext("2d");
  ctx.drawImage(bitmap, 0, 0, w, h);
  const blob = await canvas.convertToBlob({ type: "image/jpeg", quality });
  return base64FromBytes(new Uint8Array(await blob.arrayBuffer()));
}
// Map a model-provided coordinate (read off the downscaled screenshot) back to CSS viewport px.
// Passthrough when no screenshot has been taken for the tab (nothing to map against).
function rescaleCoord(tabId, x, y) {
  const c = screenshotCtx.get(tabId);
  if (!c || !c.shotW || !c.shotH) return [Math.round(x), Math.round(y)];
  return [Math.round((x * c.vpW) / c.shotW), Math.round((y * c.vpH) / c.shotH)];
}
async function cdp(tabId, method, params) {
  await ensureAttached(tabId);
  try {
    return await chrome.debugger.sendCommand({ tabId }, method, params || {});
  } catch (e) {
    throw hopError("cdp", `${method} failed: ${(e && e.message) || e}`);
  }
}
async function enableDomain(tabId, domain) {
  const state = attached.get(tabId);
  if (!state) throw new Error("not attached");
  if (state.domains.has(domain)) return;
  await chrome.debugger.sendCommand({ tabId }, domain + ".enable", {});
  state.domains.add(domain);
}
chrome.tabs.onRemoved.addListener((tabId) => {
  if (attached.has(tabId)) {
    try { chrome.debugger.detach({ tabId }); } catch { /* already gone */ }
    attached.delete(tabId);
  }
  consoleBuffer.delete(tabId);
  networkBuffer.delete(tabId);
  screenshotCtx.delete(tabId);
  tabHost.delete(tabId);
});
chrome.debugger.onDetach.addListener((src) => attached.delete(src.tabId));

// --- Console / network buffering (join network events by requestId, unlike the reference) ---
function hostOf(url) {
  try { return new URL(url).hostname; } catch { return ""; }
}
chrome.tabs.onUpdated.addListener((tabId, info) => {
  if (info.url !== undefined) tabHost.set(tabId, hostOf(info.url));
});
// Render an uncaught-exception CDP event as one single-line string: base message, then an
// optional (url:line) location, then an optional compact [at frame, frame, ...] stack.
function exceptionText(details) {
  const exc = details.exception;
  let base;
  if (exc && typeof exc.description === "string" && exc.description) {
    base = exc.description.split("\n")[0];
  } else if (exc && exc.value !== undefined) {
    base = String(exc.value);
  } else if (typeof details.text === "string" && details.text) {
    base = details.text;
  } else {
    base = "Uncaught exception";
  }
  let out = base;
  if (typeof details.url === "string" && details.url) {
    // CDP line numbers are 0-based; add 1 for the human-readable line reported here.
    out += typeof details.lineNumber === "number" ? ` (${details.url}:${details.lineNumber + 1})` : ` (${details.url})`;
  }
  const frames = details.stackTrace && Array.isArray(details.stackTrace.callFrames) ? details.stackTrace.callFrames : [];
  if (frames.length) {
    const rendered = frames.slice(0, 3).map((f) => `${f.functionName || "<anonymous>"}@${f.url}:${f.lineNumber + 1}`);
    out += ` [at ${rendered.join(", ")}]`;
  }
  return out;
}
chrome.debugger.onEvent.addListener((src, method, params) => {
  const tabId = src.tabId;
  if (method === "Runtime.consoleAPICalled") {
    // Single console source. Both the Runtime domain (Runtime.consoleAPICalled) and the
    // deprecated Console domain (Console.messageAdded) report the same console.* call, so
    // enabling and buffering both double-counts every message. We keep only the richer
    // Runtime event (structured args + method-accurate `type`) and never enable Console.
    const text = (params.args || []).map((a) => a.value !== undefined ? a.value : (a.description || "")).join(" ");
    pushCapped(consoleBuffer, tabId, { level: params.type || "log", text });
  } else if (method === "Runtime.exceptionThrown") {
    pushCapped(consoleBuffer, tabId, { level: "exception", text: exceptionText(params.exceptionDetails || {}) });
  } else if (method === "Network.requestWillBeSent" && params.request) {
    pushCapped(networkBuffer, tabId, { requestId: params.requestId, method: params.request.method, url: params.request.url, status: 0 });
  } else if (method === "Network.responseReceived" && params.response) {
    const buf = bufferFor(networkBuffer, tabId, tabHost.get(tabId));
    const existing = buf.items.find((r) => r.requestId === params.requestId);
    if (existing) { existing.status = params.response.status; existing.mimeType = params.response.mimeType; }
    else pushCapped(networkBuffer, tabId, { requestId: params.requestId, method: "?", url: params.response.url, status: params.response.status, mimeType: params.response.mimeType });
  } else if (method === "Network.loadingFailed" && params.requestId) {
    const buf = bufferFor(networkBuffer, tabId, tabHost.get(tabId));
    const existing = buf.items.find((r) => r.requestId === params.requestId);
    if (existing) {
      existing.status = 503;
      if (params.errorText) existing.errorText = params.errorText;
      existing.canceled = !!params.canceled;
    }
  }
});
// Buffers are owned by the tab's current hostname, per the read_console_messages /
// read_network_requests schema contract; a hostname change replaces the buffer with a fresh one.
function bufferFor(map, tabId, host) {
  let buf = map.get(tabId);
  if (!buf || (host !== undefined && buf.host !== undefined && buf.host !== host)) {
    buf = { host, items: [] };
    map.set(tabId, buf);
  } else if (buf.host === undefined && host !== undefined) {
    buf.host = host; // entries captured before the host was known belong to the first host learned
  }
  return buf;
}
function pushCapped(map, tabId, item) {
  const buf = bufferFor(map, tabId, tabHost.get(tabId));
  buf.items.push(item);
  if (buf.items.length > 1000) buf.items.splice(0, buf.items.length - 1000);
}

// --- Tab group (created lazily; recovered from live state after a service-worker restart) ---
async function ensureGroup(create) {
  if (groupId !== null) {
    try { await chrome.tabGroups.get(groupId); return; } catch { groupId = null; }
  }
  const groups = await chrome.tabGroups.query({ title: GROUP_TITLE });
  if (groups.length) { groupId = groups[0].id; return; }
  if (!create) return;
  const win = await chrome.windows.create({ focused: true, url: "about:blank" });
  const gid = await chrome.tabs.group({ tabIds: [win.tabs[0].id] });
  await chrome.tabGroups.update(gid, { title: GROUP_TITLE, color: "blue" });
  groupId = gid;
}
async function groupTabs() {
  return groupId === null ? [] : chrome.tabs.query({ groupId });
}
async function inGroup(tabId) {
  // Always consult live state; the in-memory groupId can be stale after a restart.
  try {
    const tab = await chrome.tabs.get(tabId);
    if (tab.groupId !== -1 && groupId === null) {
      const g = await chrome.tabGroups.get(tab.groupId);
      if (g.title === GROUP_TITLE) groupId = g.id;
    }
    return tab.groupId === groupId;
  } catch {
    return false;
  }
}
function tabContext(tabs) {
  const available = tabs.map((t) => ({ tabId: t.id, title: t.title || "", url: t.url || "" }));
  return text(JSON.stringify({ mcpGroupId: groupId, tabs: available }, null, 2));
}

// --- Content-script bridge (inject on demand) ---
async function content(tabId, message) {
  try {
    return await chrome.tabs.sendMessage(tabId, message);
  } catch {
    try {
      await chrome.scripting.executeScript({ target: { tabId }, files: ["content.js"] });
      return await chrome.tabs.sendMessage(tabId, message);
    } catch (e) {
      throw hopError(
        "page",
        "content script unavailable on this page (script injection blocked)",
        (e && e.message) || e
      );
    }
  }
}

// --- MCP result helpers ---
function text(t) {
  return { content: [{ type: "text", text: t }] };
}
function textImage(t, base64) {
  return { content: [{ type: "text", text: t }, { type: "image", data: base64, mimeType: "image/jpeg" }] };
}

// --- Screenshot pipeline: capture native, downscale to the token budget, record ScreenshotContext ---
async function screenshot(tabId) {
  await ensureAttached(tabId);
  const { vpW, vpH, dpr } = await probeViewport(tabId);
  // Hide the phantom cursor / glow so they never appear in the model's screenshot.
  await sendToTab(tabId, { type: "HIDE_FOR_TOOL_USE" });
  await sleep(40);
  let cap;
  try {
    cap = await cdp(tabId, "Page.captureScreenshot", { format: "jpeg", quality: 80, captureBeyondViewport: false });
  } finally {
    sendToTab(tabId, { type: "SHOW_AFTER_TOOL_USE" });
  }
  const { w, h } = targetDims(vpW, vpH);
  // Default to the raw native capture (dims = CSS viewport * DPR) if canvas downscaling is unavailable.
  let base64 = cap.data, shotW = Math.round(vpW * dpr), shotH = Math.round(vpH * dpr);
  try {
    const bitmap = await createImageBitmap(new Blob([bytesFromBase64(cap.data)], { type: "image/jpeg" }));
    base64 = await encodeJpeg(bitmap, w, h, 0.55);
    if (base64.length > MAX_SCREENSHOT_B64) base64 = await encodeJpeg(bitmap, w, h, 0.3);
    shotW = w; shotH = h;
    if (bitmap.close) bitmap.close();
  } catch { /* OffscreenCanvas/createImageBitmap unavailable: keep the raw native capture */ }
  screenshotCtx.set(tabId, { vpW, vpH, shotW, shotH });
  return base64;
}

// --- Input helpers ---
function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}
// --- Visual indicator (best-effort; the content script is absent on chrome:// and similar pages) ---
function sendToTab(tabId, msg) {
  return chrome.tabs.sendMessage(tabId, msg).catch(() => {});
}
function showActivity(tabId) { sendToTab(tabId, { type: "SHOW_AGENT_INDICATORS" }); }
// Move the phantom cursor to a (rescaled, CSS-px) point and wait for it to settle, so the user sees
// the pointer arrive before the action fires. Resolves immediately if no indicator is present.
function moveCursor(tabId, x, y) { return sendToTab(tabId, { type: "UPDATE_PHANTOM_CURSOR", x, y }); }
const KEY_MAP = {
  enter: "Enter", return: "Enter", tab: "Tab", escape: "Escape", esc: "Escape",
  backspace: "Backspace", delete: "Delete", space: " ",
  up: "ArrowUp", down: "ArrowDown", left: "ArrowLeft", right: "ArrowRight",
  arrowup: "ArrowUp", arrowdown: "ArrowDown", arrowleft: "ArrowLeft", arrowright: "ArrowRight",
  home: "Home", end: "End", pageup: "PageUp", pagedown: "PageDown",
};
function modifierBits(str) {
  let bits = 0;
  for (const p of (str || "").toLowerCase().split("+").map((x) => x.trim())) {
    if (p === "ctrl" || p === "control") bits |= 2;
    else if (p === "alt") bits |= 1;
    else if (p === "shift") bits |= 8;
    else if (["meta", "cmd", "command", "win", "windows"].includes(p)) bits |= 4;
  }
  return bits;
}
async function click(tabId, x, y, opts) {
  const modifiers = opts.modifiers || 0, button = opts.button || "left", clickCount = opts.clickCount || 1;
  await cdp(tabId, "Input.dispatchMouseEvent", { type: "mouseMoved", x, y, modifiers });
  await sleep(40);
  await cdp(tabId, "Input.dispatchMouseEvent", { type: "mousePressed", x, y, button, clickCount, modifiers });
  await sleep(40);
  await cdp(tabId, "Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button, clickCount, modifiers });
}
async function resolveCoords(tabId, args) {
  // Model-provided coordinates are read off the (downscaled) screenshot -> rescale to CSS px.
  if (args.coordinate) return rescaleCoord(tabId, args.coordinate[0], args.coordinate[1]);
  // ref coordinates come from getBoundingClientRect (already CSS viewport px) -> do NOT rescale.
  if (args.ref) {
    const r = await content(tabId, { type: "refCoordinates", ref: args.ref });
    if (r && r.result) return [r.result.x, r.result.y];
    // The engine is truthful: a stale ref is a failure, never a silent [0, 0] substitution.
    throw hopError("page", `Element ${args.ref} not found; the page may have changed since it was read`);
  }
  return null;
}
async function pressKey(tabId, combo) {
  const parts = combo.split("+").map((p) => p.trim().toLowerCase());
  let modifiers = 0;
  let key = combo;
  if (parts.length > 1) {
    key = "";
    for (const p of parts) {
      if (p === "ctrl" || p === "control") modifiers |= 2;
      else if (p === "alt") modifiers |= 1;
      else if (p === "shift") modifiers |= 8;
      else if (["meta", "cmd", "command", "win", "windows"].includes(p)) modifiers |= 4;
      else key = KEY_MAP[p] || p;
    }
  } else {
    key = KEY_MAP[parts[0]] || combo;
  }
  // Reload chords (ctrl/cmd+r, F5): Chrome will not reload from a synthetic key event delivered to
  // the renderer, so intercept and drive the reload directly (shift => bypass cache / hard reload).
  const bare = (key || "").toLowerCase();
  const ctrlOrCmd = (modifiers & 2) !== 0 || (modifiers & 4) !== 0;
  if ((ctrlOrCmd && bare === "r") || bare === "f5") {
    await chrome.tabs.reload(tabId, { bypassCache: (modifiers & 8) !== 0 });
    return;
  }
  // Include the Windows virtual key code so Chrome maps modified combos (ctrl+a, ctrl+c, ...) to
  // real editing commands; without it a modified keyDown arrives but triggers no edit action.
  const code = keyCode(key);
  const vk = vkCode(key);
  const evt = { key, code, modifiers, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };
  await cdp(tabId, "Input.dispatchKeyEvent", { type: "keyDown", ...evt });
  await cdp(tabId, "Input.dispatchKeyEvent", { type: "keyUp", ...evt });
  await sleep(20);
}
// Best-effort DOM `code` for a resolved key, so pages that branch on event.code / keyCode work.
function keyCode(key) {
  if (key.length === 1) {
    if (/[a-zA-Z]/.test(key)) return "Key" + key.toUpperCase();
    if (/[0-9]/.test(key)) return "Digit" + key;
  }
  return key; // named keys (Enter, Tab, ArrowUp, ...) use the key name as their code
}
// Windows virtual key codes, so Chrome interprets shortcuts (ctrl+a select-all, etc.) as commands.
const VK_NAMED = {
  Enter: 13, Tab: 9, Escape: 27, Backspace: 8, Delete: 46, " ": 32,
  ArrowUp: 38, ArrowDown: 40, ArrowLeft: 37, ArrowRight: 39,
  Home: 36, End: 35, PageUp: 33, PageDown: 34, Insert: 45,
};
function vkCode(key) {
  if (key.length === 1) {
    const up = key.toUpperCase();
    if (up >= "A" && up <= "Z") return up.charCodeAt(0); // A-Z -> 65-90
    if (key >= "0" && key <= "9") return key.charCodeAt(0); // 0-9 -> 48-57
  }
  return VK_NAMED[key] || 0;
}
function waitForLoad(tabId) {
  return new Promise((resolve) => {
    const listener = (id, info) => {
      if (id === tabId && info.status === "complete") {
        chrome.tabs.onUpdated.removeListener(listener);
        resolve();
      }
    };
    chrome.tabs.onUpdated.addListener(listener);
    setTimeout(() => { chrome.tabs.onUpdated.removeListener(listener); resolve(); }, 10000);
  });
}

// --- computer (13 actions; screenshots only on screenshot/scroll/zoom) ---
async function computer(a) {
  const tabId = a.tabId;
  if (!(await inGroup(tabId))) return text(`Tab ${tabId} is not in the ${GROUP_TITLE} group.`);
  const modifiers = modifierBits(a.modifiers);
  showActivity(tabId); // best-effort "agent active" glow for the watching user

  switch (a.action) {
    case "screenshot":
      return textImage("Screenshot captured (jpeg).", await screenshot(tabId));
    case "zoom":
      return textImage(`Zoom region ${JSON.stringify(a.region || [])} (jpeg).`, await screenshot(tabId));
    case "wait": {
      const s = Math.min(a.duration || 1, 30);
      await sleep(s * 1000);
      return text(`Waited ${s}s.`);
    }
    case "left_click":
    case "right_click":
    case "double_click":
    case "triple_click":
    case "hover": {
      const c = await resolveCoords(tabId, a);
      if (!c) return text("coordinate or ref is required.");
      await moveCursor(tabId, c[0], c[1]); // show the pointer arrive before acting
      if (a.action === "hover") {
        await cdp(tabId, "Input.dispatchMouseEvent", { type: "mouseMoved", x: c[0], y: c[1], modifiers });
        return text(`Hovered at (${c[0]}, ${c[1]}).`);
      }
      const button = a.action === "right_click" ? "right" : "left";
      const clickCount = a.action === "double_click" ? 2 : a.action === "triple_click" ? 3 : 1;
      await click(tabId, c[0], c[1], { button, clickCount, modifiers });
      return text(`${a.action} at (${c[0]}, ${c[1]}).`);
    }
    case "type": {
      if (!a.text) return text("text is required for type.");
      await ensureAttached(tabId);
      for (const ch of a.text) { await cdp(tabId, "Input.insertText", { text: ch }); await sleep(8); }
      return text(`Typed ${a.text.length} character(s).`);
    }
    case "key": {
      if (!a.text) return text("text is required for key.");
      await ensureAttached(tabId);
      const repeat = Math.min(a.repeat || 1, 100);
      for (let i = 0; i < repeat; i++) {
        for (const combo of a.text.split(" ").filter(Boolean)) await pressKey(tabId, combo);
      }
      return text(`Pressed: ${a.text} (x${repeat}).`);
    }
    case "scroll": {
      const c = (await resolveCoords(tabId, a)) || [0, 0];
      const dir = a.scroll_direction || "down";
      const amount = Math.min(a.scroll_amount || 3, 10);
      const deltaX = dir === "left" ? -amount * 100 : dir === "right" ? amount * 100 : 0;
      const deltaY = dir === "up" ? -amount * 100 : dir === "down" ? amount * 100 : 0;
      await moveCursor(tabId, c[0], c[1]);
      await cdp(tabId, "Input.dispatchMouseEvent", { type: "mouseWheel", x: c[0], y: c[1], deltaX, deltaY, modifiers });
      await sleep(250);
      return textImage(`Scrolled ${dir} by ${amount}.`, await screenshot(tabId));
    }
    case "scroll_to": {
      if (a.ref) {
        const r = await content(tabId, { type: "scrollToRef", ref: a.ref });
        // The engine is truthful: a stale ref is a failure, never a false "Scrolled to target.".
        if (!(r && r.result)) {
          throw hopError("page", `Element ${a.ref} not found; the page may have changed since it was read`);
        }
      } else if (a.coordinate) {
        await cdp(tabId, "Runtime.evaluate", { expression: `window.scrollTo(${a.coordinate[0]}, ${a.coordinate[1]})` });
      }
      await sleep(250);
      return text("Scrolled to target.");
    }
    case "left_click_drag": {
      if (!a.start_coordinate || !a.coordinate) return text("start_coordinate and coordinate are required.");
      // Both endpoints are model-provided (read off the screenshot) -> rescale to CSS px.
      const [sx, sy] = rescaleCoord(tabId, a.start_coordinate[0], a.start_coordinate[1]);
      const [ex, ey] = rescaleCoord(tabId, a.coordinate[0], a.coordinate[1]);
      await moveCursor(tabId, sx, sy);
      await cdp(tabId, "Input.dispatchMouseEvent", { type: "mouseMoved", x: sx, y: sy, modifiers });
      await sleep(40);
      await cdp(tabId, "Input.dispatchMouseEvent", { type: "mousePressed", x: sx, y: sy, button: "left", modifiers });
      await sleep(40);
      for (let i = 1; i <= 10; i++) {
        await cdp(tabId, "Input.dispatchMouseEvent", { type: "mouseMoved", x: sx + ((ex - sx) * i) / 10, y: sy + ((ey - sy) * i) / 10, modifiers });
        await sleep(16);
      }
      await moveCursor(tabId, ex, ey);
      await cdp(tabId, "Input.dispatchMouseEvent", { type: "mouseReleased", x: ex, y: ey, button: "left", modifiers });
      return text(`Dragged (${sx}, ${sy}) -> (${ex}, ${ey}).`);
    }
    default:
      return text(`Unknown computer action: ${a.action}`);
  }
}

// --- Tool handlers ---
const handlers = {
  async tabs_context_mcp(a) {
    await ensureGroup(a.createIfEmpty);
    if (groupId === null) return text("No Browser MCP tab group. Call with createIfEmpty: true.");
    return tabContext(await groupTabs());
  },
  async tabs_create_mcp() {
    await ensureGroup(true);
    const tab = await chrome.tabs.create({ active: true });
    await chrome.tabs.group({ tabIds: [tab.id], groupId });
    const r = tabContext(await groupTabs());
    r.content[0].text = `Created tab ${tab.id}.\n` + r.content[0].text;
    return r;
  },
  async navigate(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the ${GROUP_TITLE} group.`);
    if (a.url === "back") {
      await chrome.tabs.goBack(a.tabId);
    } else if (a.url === "forward") {
      await chrome.tabs.goForward(a.tabId);
    } else {
      let url = a.url;
      if (!/^https?:\/\//i.test(url) && !/^(about|chrome|edge|brave):/i.test(url)) {
        url = "https://" + url.replace(/^[a-z]{1,6}:\/+/i, "");
      }
      try { new URL(url); } catch { return text(`Invalid URL: "${a.url}".`); }
      await chrome.tabs.update(a.tabId, { url });
    }
    await waitForLoad(a.tabId);
    const tab = await chrome.tabs.get(a.tabId);
    return text(`Navigated to ${tab.url}${tab.status !== "complete" ? " (still loading)" : ""}.`);
  },
  computer,
  async read_page(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    const r = await content(a.tabId, { type: "accessibilityTree", options: a });
    return text((r && r.result) || "Could not read the page.");
  },
  async get_page_text(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    const r = await content(a.tabId, { type: "pageText", max_chars: a.max_chars });
    return text((r && r.result) || "Could not extract page text.");
  },
  async find(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    const r = await content(a.tabId, { type: "find", query: a.query });
    const data = (r && r.result) || { results: [] };
    const results = data.results || [];
    if (!results.length) return text(`No elements matching "${a.query}".`);
    let out = `Found ${results.length} element(s):\n` + results.map((e) => `[${e.ref}] ${e.role} "${e.name}" at (${e.x}, ${e.y})`).join("\n");
    if (data.more) out += "\n(more than 20 matches; refine your query for the rest)";
    return text(out);
  },
  async form_input(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    const r = await content(a.tabId, { type: "setFormValue", ref: a.ref, value: a.value });
    // The engine is truthful: a content-script failure is a failure, never a masqueraded success.
    if (r && r.result && r.result.error) {
      const msg = r.result.error.endsWith(".") ? r.result.error.slice(0, -1) : r.result.error;
      throw hopError("page", msg);
    }
    return text(`Set ${a.ref} = ${JSON.stringify(a.value)}.`);
  },
  async javascript_tool(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    const r = await cdp(a.tabId, "Runtime.evaluate", { expression: a.text, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) return text(`Error: ${r.exceptionDetails.text || "exception"}`);
    const v = r.result;
    return text(v.value !== undefined ? JSON.stringify(v.value) : (v.description || String(v.type)));
  },
  async read_console_messages(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    await ensureAttached(a.tabId);
    // Only enable Runtime; the Console domain is the deprecated duplicate source (see onEvent).
    await enableDomain(a.tabId, "Runtime");
    const tab = await chrome.tabs.get(a.tabId);
    const host = hostOf(tab.url || "");
    tabHost.set(a.tabId, host);
    const buf = bufferFor(consoleBuffer, a.tabId, host);
    const total = buf.items.length;
    let msgs = buf.items;
    if (a.onlyErrors) msgs = msgs.filter((m) => ["error", "exception"].includes(m.level));
    if (a.pattern) {
      try { const re = new RegExp(a.pattern, "i"); msgs = msgs.filter((m) => re.test(m.text) || re.test(m.level)); }
      catch { msgs = msgs.filter((m) => m.text.includes(a.pattern)); }
    }
    msgs = msgs.slice(-(a.limit || 100));
    if (a.clear) consoleBuffer.set(a.tabId, { host, items: [] });
    if (msgs.length) return text(msgs.map((m) => `[${m.level}] ${m.text}`).join("\n"));
    const primary = total
      ? `${total} console message(s) recorded for this tab, but none matched your filter.`
      : "No console messages recorded for this tab.";
    return text(`${primary}\nNote: console tracking begins when this tool is first used on a tab. Reload the page to capture messages emitted during page load.`);
  },
  async read_network_requests(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    await ensureAttached(a.tabId);
    await enableDomain(a.tabId, "Network");
    const tab = await chrome.tabs.get(a.tabId);
    const host = hostOf(tab.url || "");
    tabHost.set(a.tabId, host);
    const buf = bufferFor(networkBuffer, a.tabId, host);
    const total = buf.items.length;
    let reqs = buf.items;
    if (a.urlPattern) reqs = reqs.filter((r) => r.url.includes(a.urlPattern));
    reqs = reqs.slice(-(a.limit || 100));
    if (a.clear) networkBuffer.set(a.tabId, { host, items: [] });
    if (reqs.length) return text(reqs.map((r) => `${r.method || "?"} ${r.url} ${r.status ? "-> " + r.status + (r.errorText ? " (" + r.errorText + ")" : "") : "(pending)"}`).join("\n"));
    const primary = total
      ? `${total} network request(s) recorded for this tab, but none matched your filter.`
      : "No network requests recorded for this tab.";
    return text(`${primary}\nNote: network tracking begins when this tool is first used on a tab. Reload the page to capture requests made during page load, or interact with the page to trigger new requests.`);
  },
  async resize_window(a) {
    if (!(await inGroup(a.tabId))) return text(`Tab ${a.tabId} is not in the group.`);
    const tab = await chrome.tabs.get(a.tabId);
    await chrome.windows.update(tab.windowId, { width: a.width, height: a.height });
    // The viewport changed; drop any stale ScreenshotContext for this window's tabs so the next
    // screenshot re-establishes the coordinate mapping.
    for (const tabId of attached.keys()) {
      try {
        const t = await chrome.tabs.get(tabId);
        if (t.windowId === tab.windowId) screenshotCtx.delete(tabId);
      } catch { /* tab gone */ }
    }
    return text(`Resized window to ${a.width}x${a.height}.`);
  },
  async update_plan(a) {
    const domains = (a.domains || []).join(", ");
    const approach = (a.approach || []).map((s) => `- ${s}`).join("\n");
    return text(`Plan (auto-approved by the v1.0 engine):\nDomains: ${domains}\n${approach}`);
  },
};

async function dispatch(id, tool, args) {
  const handler = handlers[tool];
  if (!handler) return fail(id, `Unknown tool: ${tool}`);
  try {
    reply(id, await handler(args));
  } catch (e) {
    // Hop-tagged errors (cdp/page) pass through as-is; untagged errors keep the tool-name prefix.
    if (e && e.hop) fail(id, e);
    else fail(id, `${tool} failed: ${(e && e.message) || e}`);
  }
}

connect();
