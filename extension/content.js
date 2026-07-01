// Browser MCP -- content script.
//
// Policy-free DOM mechanism: accessibility-tree generation, element-ref mapping (WeakRef), text
// extraction, element finding, and form input with shadow-DOM traversal. Runs in the page; the
// service worker calls in via chrome.tabs.sendMessage. No governance here.

(function () {
  if (window.__browserMcpLoaded) return;
  window.__browserMcpLoaded = true;

  // --- Element refs (persist across calls; WeakRef so the page can still GC) ---
  let refSeq = 0;
  const refToEl = {}; // ref -> WeakRef<Element>
  const elToRef = new WeakMap();
  function refFor(el) {
    const existing = elToRef.get(el);
    if (existing && refToEl[existing] && refToEl[existing].deref() === el) return existing;
    const ref = "ref_" + ++refSeq;
    refToEl[ref] = new WeakRef(el);
    elToRef.set(el, ref);
    return ref;
  }
  function deref(ref) {
    const wr = refToEl[ref];
    if (!wr) return null;
    const el = wr.deref();
    if (!el) { delete refToEl[ref]; return null; }
    return el;
  }

  // --- Role / name / interactivity / visibility ---
  const TAG_ROLE = {
    a: "link", button: "button", input: "textbox", textarea: "textbox", select: "combobox",
    img: "img", h1: "heading", h2: "heading", h3: "heading", h4: "heading", h5: "heading",
    h6: "heading", nav: "navigation", main: "main", form: "form", ul: "list", ol: "list",
    li: "listitem", table: "table", tr: "row", th: "columnheader", td: "cell", dialog: "dialog",
    section: "region", article: "article", summary: "button",
  };
  function role(el) {
    if (el.getAttribute("role")) return el.getAttribute("role");
    const tag = el.tagName.toLowerCase();
    if (tag === "input") {
      const t = (el.type || "text").toLowerCase();
      return ({ checkbox: "checkbox", radio: "radio", range: "slider", button: "button",
        submit: "button", reset: "button", search: "searchbox", number: "spinbutton" })[t] || "textbox";
    }
    return TAG_ROLE[tag] || null;
  }
  function accessibleName(el) {
    const ariaLabel = el.getAttribute("aria-label");
    if (ariaLabel) return ariaLabel.trim();
    const labelledBy = el.getAttribute("aria-labelledby");
    if (labelledBy) {
      const names = labelledBy.split(/\s+/).map((id) => {
        const t = document.getElementById(id);
        return t && t.textContent ? t.textContent.trim() : "";
      }).filter(Boolean);
      if (names.length) return names.join(" ");
    }
    if (el.placeholder) return el.placeholder.trim();
    if (el.title) return el.title.trim();
    if (el.alt) return el.alt.trim();
    if (el.id) {
      const label = document.querySelector(`label[for="${CSS.escape(el.id)}"]`);
      if (label) return label.textContent.trim();
    }
    const wrapping = el.closest && el.closest("label");
    if (wrapping) { const t = wrapping.textContent.trim(); if (t) return t; }
    const tag = el.tagName.toLowerCase();
    if (["a", "button", "h1", "h2", "h3", "h4", "h5", "h6", "li", "summary", "label", "th", "td", "span"].includes(tag)) {
      const t = el.textContent && el.textContent.trim();
      if (t && t.length < 200) return t;
    }
    return "";
  }
  function interactive(el) {
    const tag = el.tagName.toLowerCase();
    if (["a", "button", "input", "textarea", "select", "summary", "details"].includes(tag)) return true;
    const r = el.getAttribute("role");
    if (r && ["button", "link", "textbox", "checkbox", "radio", "tab", "menuitem", "switch", "combobox", "slider", "spinbutton", "searchbox", "option"].includes(r)) return true;
    if (el.tabIndex >= 0) return true;
    if (el.onclick || el.getAttribute("onclick")) return true;
    if (el.isContentEditable) return true;
    return false;
  }
  function visible(el) {
    if (el.offsetParent === null && el.tagName.toLowerCase() !== "body" && getComputedStyle(el).position !== "fixed") return false;
    const s = getComputedStyle(el);
    return s.display !== "none" && s.visibility !== "hidden";
  }

  // --- Accessibility tree (custom walk incl. shadow DOM) ---
  function accessibilityTree(options) {
    options = options || {};
    const filter = options.filter || "all";
    const maxDepth = options.depth || 15;
    const maxChars = options.max_chars || 50000;
    let out = "";
    let truncated = false;
    function add(s) {
      if (truncated) return false;
      if (out.length + s.length > maxChars) {
        out += s.slice(0, maxChars - out.length) + "\n... (truncated)";
        truncated = true;
        return false;
      }
      out += s;
      return true;
    }
    function walk(el, depth, indent) {
      if (truncated || depth > maxDepth || !el || el.nodeType !== 1) return;
      const tag = el.tagName.toLowerCase();
      if (["script", "style", "noscript", "template"].includes(tag)) return;
      const r = role(el);
      const n = accessibleName(el);
      const isInteractive = interactive(el);
      const isVisible = visible(el);
      const isContainer = el.children.length > 0;
      if (filter === "interactive" && !isInteractive && !isContainer) return;
      const show = ((filter === "all" && (r || n)) || (filter === "interactive" && isInteractive)) && isVisible;
      if (show) {
        let line = indent + (r || tag);
        if (n) line += ` "${n.slice(0, 100)}"`;
        line += ` [${refFor(el)}]`;
        if (tag === "a" && el.href) line += ` href="${el.href}"`;
        if (["input", "textarea"].includes(tag) && el.value) line += ` value="${String(el.value).slice(0, 80)}"`;
        if (tag === "input") line += ` type="${el.type || "text"}"`;
        if (el.disabled) line += " disabled";
        if (!add(line + "\n")) return;
      }
      const nextIndent = show ? indent + "  " : indent;
      if (el.shadowRoot) for (const c of el.shadowRoot.children) walk(c, depth + 1, nextIndent);
      for (const c of el.children) walk(c, depth + 1, nextIndent);
    }
    let root = document.body;
    if (options.ref_id) {
      const el = deref(options.ref_id);
      if (!el) return `Error: ref_id "${options.ref_id}" not found or was garbage-collected.`;
      root = el;
    }
    walk(root, 0, "");
    return out + `\nViewport: ${window.innerWidth}x${window.innerHeight}`;
  }

  // --- Page text ---
  function pageText() {
    const selectors = ["article", "main", '[role="main"]', '[class*="article"]', '[class*="post-content"]', ".content", "#content"];
    let source = null;
    for (const sel of selectors) { source = document.querySelector(sel); if (source) break; }
    if (!source) source = document.body;
    const clone = source.cloneNode(true);
    clone.querySelectorAll("script, style, noscript, template, svg").forEach((el) => el.remove());
    const t = clone.textContent.replace(/\s+/g, " ").trim().slice(0, 100000);
    return `Title: ${document.title}\nURL: ${location.href}\n\n${t}`;
  }

  // --- Find (traverses shadow roots) ---
  function collectAll(rootNode) {
    const out = [];
    for (const el of rootNode.querySelectorAll("*")) {
      out.push(el);
      if (el.shadowRoot) out.push(...collectAll(el.shadowRoot));
    }
    return out;
  }
  function find(query) {
    const q = (query || "").toLowerCase();
    const results = [];
    let more = false;
    for (const el of collectAll(document)) {
      if (!visible(el)) continue;
      const tag = el.tagName.toLowerCase();
      if (["script", "style", "noscript", "template"].includes(tag)) continue;
      const hay = `${role(el) || ""} ${accessibleName(el) || ""} ${(el.textContent || "").slice(0, 200)} ${el.placeholder || ""} ${el.getAttribute("aria-label") || ""} ${el.title || ""} ${el.type || ""} ${tag}`.toLowerCase();
      if (!hay.includes(q)) continue;
      if (results.length >= 20) { more = true; break; }
      const rect = el.getBoundingClientRect();
      results.push({
        ref: refFor(el),
        role: role(el) || tag,
        name: (accessibleName(el) || el.textContent || "").trim().slice(0, 80),
        x: Math.round(rect.x + rect.width / 2),
        y: Math.round(rect.y + rect.height / 2),
      });
    }
    return { results, more };
  }

  // --- Form input (shadow-DOM traversal + native setter so framework inputs register) ---
  function innerInput(el) {
    const tag = el.tagName.toLowerCase();
    if (["input", "textarea", "select"].includes(tag)) return el;
    const root = el.shadowRoot || el;
    const inner = root.querySelector("input, textarea, select");
    if (inner) return inner;
    for (const child of root.querySelectorAll("*")) {
      if (child.shadowRoot) {
        const deep = child.shadowRoot.querySelector("input, textarea, select");
        if (deep) return deep;
      }
    }
    return null;
  }
  function setFormValue(ref, value) {
    const el = deref(ref);
    if (!el) return { error: `Element ${ref} not found or was garbage-collected.` };
    el.scrollIntoView({ block: "center", behavior: "instant" });
    const target = innerInput(el) || el;
    const tag = target.tagName.toLowerCase();
    const type = (target.type || "").toLowerCase();
    if (tag === "select") {
      const opt = Array.from(target.options).find((o) => o.value === String(value) || o.textContent.trim() === String(value));
      target.value = opt ? opt.value : String(value);
    } else if (type === "checkbox" || type === "radio") {
      const want = typeof value === "boolean" ? value
        : typeof value === "number" ? value !== 0
        : value === "true" || value === "1";
      if (type === "radio" && !want) {
        return { error: "cannot uncheck a radio button; set another radio in the same group instead" };
      }
      if (target.checked !== want) target.click();
      return { success: true, checked: target.checked };
    } else if (target.isContentEditable) {
      target.textContent = String(value);
    } else if (["input", "textarea"].includes(tag)) {
      const proto = tag === "textarea" ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(proto, "value") && Object.getOwnPropertyDescriptor(proto, "value").set;
      if (setter) setter.call(target, String(value));
      else target.value = String(value);
    } else {
      try { target.value = String(value); } catch { return { error: `Cannot set value on <${tag}>.` }; }
    }
    target.dispatchEvent(new Event("input", { bubbles: true, composed: true }));
    target.dispatchEvent(new Event("change", { bubbles: true, composed: true }));
    return { success: true, value: target.value };
  }

  function refCoordinates(ref) {
    const el = deref(ref);
    if (!el) return null;
    const rect = el.getBoundingClientRect();
    return { x: Math.round(rect.x + rect.width / 2), y: Math.round(rect.y + rect.height / 2) };
  }

  // --- Message handler ---
  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    switch (msg.type) {
      case "accessibilityTree": sendResponse({ result: accessibilityTree(msg.options) }); break;
      case "pageText": sendResponse({ result: pageText() }); break;
      case "find": sendResponse({ result: find(msg.query) }); break;
      case "setFormValue": sendResponse({ result: setFormValue(msg.ref, msg.value) }); break;
      case "refCoordinates": sendResponse({ result: refCoordinates(msg.ref) }); break;
      case "scrollToRef": {
        const el = deref(msg.ref);
        if (el) el.scrollIntoView({ block: "center", behavior: "instant" });
        sendResponse({ result: Boolean(el) });
        break;
      }
      default: sendResponse({ error: "unknown message type" });
    }
    return true;
  });
})();
