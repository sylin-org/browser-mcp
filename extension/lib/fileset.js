// SPDX-License-Identifier: Apache-2.0 OR MIT
// Ghostlight -- file_upload byte decoding (ADR-0050 Decision 2): validate and base64-decode the
// `files` array supplied in a file_upload call into raw bytes the page-world content script can
// wrap in File objects. Pure: no DOM, no chrome.*, no timers -- the caller (content.js) builds the
// DataTransfer/File from the decoded bytes. Ghostlight NEVER reads the host filesystem; the caller
// supplies the bytes, so this introduces no local-filesystem trust boundary.
//
// IIFE-wrapped and exposed as a single namespace per lib/constants.js's pattern (idempotent under
// MV3 worker re-evaluation; loadable as a content-script global via the manifest and under
// node --test).
(function () {
// decodeFiles(files): validate and base64-decode an array of {data, name, mimeType?}.
// Returns { ok: true, decoded: [{ name, type, bytes: Uint8Array }], totalBytes }
//      or { ok: false, error: "<message>" }.
// Rules: each item's `data` and `name` must be non-empty strings; `type` defaults to
// "application/octet-stream" when mimeType is absent/empty; bytes = Uint8Array from atob(data).
function decodeFiles(files) {
  if (!Array.isArray(files) || files.length === 0) {
    return { ok: false, error: "files parameter is required and must be a non-empty array" };
  }
  const decoded = [];
  let totalBytes = 0;
  for (const item of files) {
    if (!item || typeof item.data !== "string" || item.data.length === 0
        || typeof item.name !== "string" || item.name.length === 0) {
      return { ok: false, error: "each file must have `data` and `name`" };
    }
    const type = (typeof item.mimeType === "string" && item.mimeType.length > 0)
      ? item.mimeType
      : "application/octet-stream";
    let bin;
    try {
      bin = atob(item.data);
    } catch (e) {
      return { ok: false, error: "file `" + item.name + "` has invalid base64 `data`" };
    }
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    decoded.push({ name: item.name, type, bytes });
    totalBytes += bytes.length;
  }
  return { ok: true, decoded, totalBytes };
}

const GhostlightFileset = { decodeFiles };
if (typeof module !== "undefined" && module.exports) {
  module.exports = GhostlightFileset;
} else {
  self.GhostlightFileset = GhostlightFileset;
}
})();
