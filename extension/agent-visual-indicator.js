// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- agent visual indicator (content script).
//
// User-facing "watching" affordance (mechanism, not policy):
//   - a phantom cursor showing where the agent's pointer is about to act,
//   - a subtle "agent active" glow while a tool runs,
//   - a sky-blue click ripple (one ring per click; a right-click ring is dashed),
//   - a comet-trail along a click-drag path,
//   - a soft shimmer on the focused field when the agent types.
// All are hidden during screenshots so the model's image stays clean, and are excluded from
// read_page/find (their ids are prefixed "ghostlight-" and skipped in content.js). Driven by the
// service worker via chrome.tabs.sendMessage. A lean reimplementation of the concept; no upstream
// extension code is copied.

(function () {
  if (window.__browserMcpIndicator) return;
  window.__browserMcpIndicator = true;

  const CURSOR_ID = "ghostlight-cursor";
  const GLOW_ID = "ghostlight-active";
  const FX_LAYER_ID = "ghostlight-ripples"; // holds all transient effects (rings, trail, shimmer)
  const STYLE_ID = "ghostlight-indicator-styles";
  // Ghostlight brand accent: a luminous sky blue. SKY_RGB is the same color for rgba() shadows.
  const SKY = "#38bdf8";
  const SKY_RGB = "56,189,248";
  const FADE_MS = 4000;
  const RIPPLE_MS = 620; // one click ring's expand-and-fade duration
  const RIPPLE_STAGGER_MS = 140; // gap between rings of a multi-click, so 2/3 read as a rhythm

  let cursorEl = null;
  let glowEl = null;
  let fxLayer = null;
  let fadeTimer = null;
  let fxSeq = 0;
  let glowActive = false; // whether the glow should be visible (independent of capture-hiding)
  let hiddenForTool = false; // suppressed during a screenshot capture

  function reduceMotion() {
    return !!(window.matchMedia && window.matchMedia("(prefers-reduced-motion:reduce)").matches);
  }

  function ensureStyles() {
    if (document.getElementById(STYLE_ID)) return;
    const s = document.createElement("style");
    s.id = STYLE_ID;
    s.textContent =
      "@keyframes ghostlight-pulse{0%,100%{opacity:.5}50%{opacity:.9}}" +
      "#" + GLOW_ID + "{animation:ghostlight-pulse 2s ease-in-out infinite}" +
      "@keyframes ghostlight-ripple{0%{opacity:.85;transform:translate(-50%,-50%) scale(.3)}" +
      "100%{opacity:0;transform:translate(-50%,-50%) scale(2.8)}}" +
      "@keyframes ghostlight-ripple-rm{0%{opacity:.7;transform:translate(-50%,-50%) scale(1)}" +
      "100%{opacity:0;transform:translate(-50%,-50%) scale(1)}}" +
      "@keyframes ghostlight-trail{0%{opacity:.9}100%{opacity:0}}" +
      "@keyframes ghostlight-shimmer{0%{opacity:0}25%{opacity:1}60%{opacity:.7}100%{opacity:0}}" +
      "@keyframes ghostlight-shimmer-rm{0%{opacity:0}50%{opacity:.7}100%{opacity:0}}" +
      "@media (prefers-reduced-motion:reduce){#" + GLOW_ID + "{animation:none}#" + CURSOR_ID + "{transition:none}}";
    (document.head || document.documentElement).appendChild(s);
  }

  function makeCursor() {
    const el = document.createElement("div");
    el.id = CURSOR_ID;
    el.setAttribute("aria-hidden", "true");
    el.style.cssText =
      "position:fixed;top:0;left:0;pointer-events:none;z-index:2147483647;" +
      "transform:translate3d(-100px,-100px,0);transition:transform 150ms cubic-bezier(.2,0,0,1);" +
      "will-change:transform;filter:drop-shadow(0 0 3px rgba(" + SKY_RGB + ",.9)) drop-shadow(0 0 8px rgba(" + SKY_RGB + ",.5))";
    // Own arrow glyph; the tip sits at (0,0) so translate(x,y) places the tip exactly on the target.
    el.innerHTML =
      "<svg width='22' height='28' viewBox='0 0 22 28' style='position:absolute;top:0;left:0;overflow:visible'>" +
      "<path d='M0 0 L0 19 L5 14.5 L8.2 22 L11.4 20.6 L8.3 13.5 L14.5 13.5 Z' " +
      "fill='" + SKY + "' stroke='white' stroke-width='1.5' stroke-linejoin='round'/></svg>";
    return el;
  }

  function makeGlow() {
    const el = document.createElement("div");
    el.id = GLOW_ID;
    el.setAttribute("aria-hidden", "true");
    el.style.cssText =
      "position:fixed;inset:0;pointer-events:none;z-index:2147483646;opacity:0;" +
      "transition:opacity .3s ease-in-out;" +
      "box-shadow:inset 0 0 14px rgba(" + SKY_RGB + ",.7),inset 0 0 26px rgba(" + SKY_RGB + ",.35)";
    return el;
  }

  // A full-viewport, pointer-transparent layer that holds every transient effect. Its own id and
  // each effect's id are "ghostlight-" prefixed, so content.js skips them in read_page/find.
  function ensureFxLayer() {
    if (!fxLayer || !fxLayer.isConnected) {
      fxLayer = document.createElement("div");
      fxLayer.id = FX_LAYER_ID;
      fxLayer.setAttribute("aria-hidden", "true");
      fxLayer.style.cssText = "position:fixed;inset:0;pointer-events:none;z-index:2147483646";
      (document.body || document.documentElement).appendChild(fxLayer);
    }
    return fxLayer;
  }

  // Append a transient effect element to the fx layer and remove it when its animation ends.
  function addEphemeral(el, maxMs) {
    ensureFxLayer().appendChild(el);
    let done = false;
    const remove = () => { if (done) return; done = true; el.remove(); };
    el.addEventListener("animationend", remove, { once: true });
    setTimeout(remove, maxMs); // fallback if animationend never fires
  }

  function addRipple(x, y, dashed) {
    if (hiddenForTool || document.hidden) return;
    const el = document.createElement("div");
    el.id = FX_LAYER_ID + "-r" + fxSeq++; // "ghostlight-" prefix -> excluded from reads
    el.setAttribute("aria-hidden", "true");
    const anim = reduceMotion() ? "ghostlight-ripple-rm" : "ghostlight-ripple";
    el.style.cssText =
      "position:fixed;left:" + Math.round(x) + "px;top:" + Math.round(y) + "px;" +
      "width:34px;height:34px;border-radius:50%;box-sizing:border-box;pointer-events:none;" +
      "border:2px " + (dashed ? "dashed" : "solid") + " rgba(" + SKY_RGB + ",.9);" +
      "box-shadow:0 0 12px rgba(" + SKY_RGB + ",.55),inset 0 0 8px rgba(" + SKY_RGB + ",.35);" +
      "transform:translate(-50%,-50%) scale(.3);" +
      "animation:" + anim + " " + RIPPLE_MS + "ms ease-out forwards";
    addEphemeral(el, RIPPLE_MS + 80);
  }

  // One ring per click: count is the click count (1 single, 2 double, 3 triple), staggered so a
  // multi-click reads as a rhythm. A right-click ring is dashed to read as a secondary action.
  function spawnRipples(x, y, count, button) {
    if (hiddenForTool || document.hidden) return;
    ensureStyles();
    const dashed = button === "right";
    const n = Math.max(1, Math.min((count | 0) || 1, 5));
    for (let i = 0; i < n; i++) {
      if (i === 0) addRipple(x, y, dashed);
      else setTimeout(() => addRipple(x, y, dashed), i * RIPPLE_STAGGER_MS);
    }
  }

  // A soft dot dropped along a drag path; the sequence of fading dots reads as a comet trail.
  function addTrailDot(x, y) {
    if (hiddenForTool || document.hidden) return;
    ensureStyles();
    const el = document.createElement("div");
    el.id = FX_LAYER_ID + "-t" + fxSeq++;
    el.setAttribute("aria-hidden", "true");
    el.style.cssText =
      "position:fixed;left:" + Math.round(x) + "px;top:" + Math.round(y) + "px;" +
      "width:14px;height:14px;border-radius:50%;pointer-events:none;transform:translate(-50%,-50%);" +
      "background:radial-gradient(circle,rgba(" + SKY_RGB + ",.9) 0%,rgba(" + SKY_RGB + ",0) 70%);" +
      "animation:ghostlight-trail 520ms ease-out forwards";
    addEphemeral(el, 600);
  }

  // A gentle sky-blue outline over the currently focused field while the agent types into it.
  function shimmerFocused() {
    if (hiddenForTool || document.hidden) return;
    const target = document.activeElement;
    if (!target || target === document.body || target === document.documentElement) return;
    let rect;
    try { rect = target.getBoundingClientRect(); } catch (e) { return; }
    if (!rect || (rect.width === 0 && rect.height === 0)) return;
    ensureStyles();
    const pad = 3;
    const anim = reduceMotion() ? "ghostlight-shimmer-rm" : "ghostlight-shimmer";
    const el = document.createElement("div");
    el.id = FX_LAYER_ID + "-s" + fxSeq++;
    el.setAttribute("aria-hidden", "true");
    el.style.cssText =
      "position:fixed;box-sizing:border-box;pointer-events:none;border-radius:6px;" +
      "left:" + (rect.left - pad) + "px;top:" + (rect.top - pad) + "px;" +
      "width:" + (rect.width + pad * 2) + "px;height:" + (rect.height + pad * 2) + "px;" +
      "border:1.5px solid rgba(" + SKY_RGB + ",.85);" +
      "box-shadow:0 0 10px rgba(" + SKY_RGB + ",.5),inset 0 0 8px rgba(" + SKY_RGB + ",.25);" +
      "animation:" + anim + " 900ms ease-in-out forwards";
    addEphemeral(el, 1000);
  }

  function showGlow() {
    glowActive = true;
    if (fadeTimer) clearTimeout(fadeTimer);
    fadeTimer = setTimeout(hideGlow, FADE_MS);
    if (hiddenForTool || document.hidden) return;
    ensureStyles();
    if (!glowEl) { glowEl = makeGlow(); (document.body || document.documentElement).appendChild(glowEl); }
    glowEl.style.display = "";
    requestAnimationFrame(() => { if (glowEl && glowActive && !hiddenForTool) glowEl.style.opacity = "1"; });
  }

  function hideGlow() {
    glowActive = false;
    if (fadeTimer) { clearTimeout(fadeTimer); fadeTimer = null; }
    if (glowEl) glowEl.style.opacity = "0";
  }

  function moveCursor(x, y) {
    return new Promise((resolve) => {
      showGlow();
      if (hiddenForTool || document.hidden) return resolve();
      ensureStyles();
      if (!cursorEl) { cursorEl = makeCursor(); (document.body || document.documentElement).appendChild(cursorEl); }
      cursorEl.style.display = "";
      cursorEl.style.transform = "translate3d(" + Math.round(x) + "px," + Math.round(y) + "px,0)";
      let done = false;
      const finish = () => {
        if (done) return;
        done = true;
        if (cursorEl) cursorEl.removeEventListener("transitionend", finish);
        resolve();
      };
      cursorEl.addEventListener("transitionend", finish, { once: true });
      setTimeout(finish, 200); // fallback if no transition fires (e.g. first placement)
    });
  }

  function setHiddenForTool(v) {
    hiddenForTool = v;
    if (cursorEl) cursorEl.style.display = v ? "none" : "";
    if (glowEl) {
      if (v) glowEl.style.display = "none";
      else if (glowActive) { glowEl.style.display = ""; glowEl.style.opacity = "1"; }
    }
    if (fxLayer) {
      if (v) { fxLayer.style.display = "none"; fxLayer.replaceChildren(); } // clear in-flight effects for a clean capture
      else fxLayer.style.display = "";
    }
  }

  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    switch (msg && msg.type) {
      case "UPDATE_PHANTOM_CURSOR":
        moveCursor(msg.x, msg.y).then(() => sendResponse({ success: true }));
        return true; // respond asynchronously (after the cursor settles)
      case "AGENT_CLICK_RIPPLE":
        spawnRipples(msg.x, msg.y, msg.count, msg.button); sendResponse({ success: true }); return true;
      case "AGENT_DRAG_TRAIL":
        addTrailDot(msg.x, msg.y); sendResponse({ success: true }); return true;
      case "AGENT_TYPE_SHIMMER":
        shimmerFocused(); sendResponse({ success: true }); return true;
      case "SHOW_AGENT_INDICATORS":
        showGlow(); sendResponse({ success: true }); return true;
      case "HIDE_AGENT_INDICATORS":
        hideGlow(); sendResponse({ success: true }); return true;
      case "HIDE_FOR_TOOL_USE":
        setHiddenForTool(true); sendResponse({ success: true }); return true;
      case "SHOW_AFTER_TOOL_USE":
        setHiddenForTool(false); sendResponse({ success: true }); return true;
      default:
        return false; // not ours -- let content.js handle it
    }
  });

  window.addEventListener("beforeunload", () => { hideGlow(); });
})();
