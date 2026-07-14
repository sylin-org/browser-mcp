// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- pure actionable-element vocabulary and semantic ranking (ADR-0078 D2).
//
// DOM collection stays in content.js. This module accepts plain facts, bounds the model-facing
// result, and ranks candidates without browser or policy dependencies. `mechanicalActions` says
// only what the page mechanism can attempt; authorization remains in the service.
(function () {
const MAX_NAME = 120;
const MAX_VALUE = 120;
const MAX_HREF = 240;

function bounded(value, max) {
  if (value === undefined || value === null) return undefined;
  const text = String(value).replace(/\s+/g, " ").trim();
  if (!text) return undefined;
  return text.slice(0, max);
}

function normalize(value) {
  return String(value || "").toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
}

function tokenContains(haystack, needle) {
  const wanted = normalize(needle).split(" ").filter(Boolean);
  if (!wanted.length) return false;
  const have = new Set(normalize(haystack).split(" ").filter(Boolean));
  return wanted.every((token) => have.has(token));
}

// Lower is stronger. Accessible-name matches lead; the broader search text preserves find's
// useful role/placeholder/tag matching without letting it outrank an exact name.
function matchRank(query, candidate) {
  const q = normalize(query);
  if (!q) return null;
  const name = normalize(candidate.name);
  const search = normalize(candidate.searchText || candidate.name);
  if (name === q) return 0;
  if (name && name.startsWith(q)) return 1;
  if (tokenContains(name || search, q)) return 2;
  if ((name && name.includes(q)) || search.includes(q)) return 3;
  return null;
}

function rankCandidates(query, candidates, role) {
  const wantedRole = normalize(role);
  return (candidates || [])
    .map((candidate, index) => ({ candidate, index, rank: matchRank(query, candidate) }))
    .filter((entry) => entry.rank !== null && (!wantedRole || normalize(entry.candidate.role) === wantedRole))
    .sort((a, b) => a.rank - b.rank || a.index - b.index)
    .map((entry) => Object.assign({}, entry.candidate, { matchRank: entry.rank }));
}

function makeSummary(facts) {
  const f = facts || {};
  const out = {
    ref: String(f.ref || ""),
    role: bounded(f.role, 64) || "generic",
    name: bounded(f.name, MAX_NAME) || "",
    visible: !!f.visible,
    enabled: !!f.enabled,
  };
  if (typeof f.checked === "boolean") out.checked = f.checked;
  if (typeof f.selected === "boolean") out.selected = f.selected;
  const value = bounded(f.value, MAX_VALUE);
  if (value !== undefined) out.value = value;
  if (f.secret) out.secret = true;
  const href = bounded(f.href, MAX_HREF);
  if (href !== undefined) out.href = href;
  if (f.box) {
    out.box = {
      x: Math.round(Number(f.box.x) || 0),
      y: Math.round(Number(f.box.y) || 0),
      width: Math.max(0, Math.round(Number(f.box.width) || 0)),
      height: Math.max(0, Math.round(Number(f.box.height) || 0)),
    };
  }
  if (Number.isFinite(f.renderSerial)) out.renderSerial = f.renderSerial;
  if (f.frameOrigin) out.frameOrigin = bounded(f.frameOrigin, MAX_HREF);
  if (Array.isArray(f.mechanicalActions) && f.mechanicalActions.length) {
    out.mechanicalActions = f.mechanicalActions.slice(0, 8).map((action) => String(action));
  }
  return out;
}

const GhostlightActionable = { makeSummary, matchRank, normalize, rankCandidates, tokenContains };
if (typeof module !== "undefined" && module.exports) {
  module.exports = GhostlightActionable;
} else {
  self.GhostlightActionable = GhostlightActionable;
}
})();
