# Browser MCP -- Extension (Manifest V3)

The thin, **policy-free** Chromium extension: a CDP executor + native-messaging endpoint. It holds
mechanism only; all governance lives in the `browser-mcp` binary. Not a port of the reference --
a clean re-implementation that harvests its proven mechanics (MV3 keepalive, live-state tab-group
recovery, `deviceScaleFactor:1` coordinate normalization, JPEG 55->30 screenshot fallback, the
shadow-DOM `form_input` fix) and fixes a couple of its bugs (network events joined by `requestId`,
cleaner key handling).

## Files
- `manifest.json` -- MV3 manifest (permissions, native-messaging host, background SW, content script).
- `service-worker.js` -- native messaging, CDP tool execution, tab-group management, keepalive/recovery.
- `content.js` -- DOM reads: accessibility tree, `find`, `form_input` (shadow DOM), `get_page_text`.
- `native-messaging-host.json` -- host-manifest template (fill in the binary path + extension ID).

## Manual setup (until the self-registering installer, Fork 4, lands)

Until `browser-mcp install` exists, wire it by hand:

1. **Build the binary:** `cargo build --release` (or `--debug`). Note the absolute path to the
   `browser-mcp` executable.
2. **Load the extension:** open `chrome://extensions` (or `brave://`, `edge://`), enable Developer
   mode, click **Load unpacked**, and select this `extension/` directory. Copy the **extension ID**
   shown under the name.
3. **Register the native-messaging host:** copy `native-messaging-host.json`, replace `path` with
   the absolute binary path and the `allowed_origins` id with your extension ID, and drop it in the
   browser's `NativeMessagingHosts` directory (Windows: register a registry key whose default value
   is the manifest's absolute path -- see `docs/research/11-install-detection.md` for exact paths
   per OS/browser).
4. **Restart the browser** (native-messaging host configs are read at startup).
5. **Add to your MCP client**, e.g. `claude mcp add browser-mcp -- /absolute/path/to/browser-mcp`.

> A build-time extension `key` (for a deterministic ID, so `allowed_origins` can be a compile-time
> constant) is intentionally omitted for now; the installer work adds it. See
> `docs/research/11-install-detection.md`.

## Verify
Ask the agent to *navigate to a page and take a screenshot* -- the "Browser MCP" tab group opens
and the screenshot returns.
