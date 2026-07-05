// Ghostlight Console client script. Fetches this machine's own local API (never a remote
// control plane) and renders read-mostly views: live sessions, the provenance-aware config
// table, and the single "enable remote connections" write action. Populated incrementally
// (config: K3, sessions: K4, enable-remote: K5); this file is the page-load entry point only.

document.addEventListener("DOMContentLoaded", () => {
  // Filled in by K3 (config table), K4 (sessions section), K5 (enable-remote control).
});
