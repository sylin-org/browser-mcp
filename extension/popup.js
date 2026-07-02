// Browser MCP -- popup. Renders binary-reported hold state and submits user gestures. Caches
// nothing (no chrome.storage, no persisted state) and decides nothing: the binary holds the
// flag and answers every render() call fresh.

const statusEl = document.getElementById("status");
const toggleEl = document.getElementById("toggle");

function render(state) {
  if (!state.session) {
    statusEl.textContent = "No active browsing session.";
    toggleEl.textContent = "Pause agent browsing (take the wheel)";
    toggleEl.disabled = true;
    return;
  }
  toggleEl.disabled = false;
  if (state.held) {
    statusEl.textContent = "Agent browsing is PAUSED.";
    toggleEl.textContent = "Resume agent browsing";
  } else {
    statusEl.textContent = "Agent browsing is allowed.";
    toggleEl.textContent = "Pause agent browsing (take the wheel)";
  }
}

function refresh() {
  chrome.runtime.sendMessage({ type: "getHoldState" }, (state) => {
    render(state || { session: false, held: false });
  });
}

toggleEl.addEventListener("click", () => {
  const nextHeld = toggleEl.textContent.indexOf("Resume") === -1;
  chrome.runtime.sendMessage({ type: "setHold", held: nextHeld }, (state) => {
    render(state || { session: false, held: false });
  });
});

refresh();
