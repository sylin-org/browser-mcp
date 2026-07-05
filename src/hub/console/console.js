// Ghostlight Console client script. Fetches this machine's own local API (never a remote
// control plane) and renders read-mostly views: live sessions, the provenance-aware config
// table, and the single "enable remote connections" write action. Populated incrementally
// (config: K3, sessions: K4, enable-remote: K5); this file is the page-load entry point only.

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  }[c]));
}

async function loadConfig() {
  const el = document.getElementById("config-placeholder");
  try {
    const res = await fetch("/api/v1/config");
    if (!res.ok) {
      el.textContent = "Could not load configuration (" + res.status + ").";
      return;
    }
    const data = await res.json();
    const rows = data.keys.map((k) => {
      const locked = k.locked ? '<span class="locked-badge">org-locked</span>' : "";
      return "<tr><td>" + escapeHtml(k.key) + "</td><td>" + escapeHtml(JSON.stringify(k.value)) +
        "</td><td>" + escapeHtml(k.source) + "</td><td>" + locked + "</td></tr>";
    }).join("");
    el.outerHTML = "<table><thead><tr><th>Key</th><th>Value</th><th>Layer</th><th></th></tr></thead>" +
      "<tbody>" + rows + "</tbody></table>";
  } catch (e) {
    el.textContent = "Could not load configuration.";
  }
}

document.addEventListener("DOMContentLoaded", () => {
  loadConfig();
  // Filled in by K4 (sessions section), K5 (enable-remote control).
});
