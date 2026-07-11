# Installing Ghostlight

Ghostlight is one Rust binary plus a thin browser extension. Installation wires three things
together: your MCP client, the binary (which acts as the browser's native-messaging host), and the
extension. This guide covers both install paths, what the installer actually writes, how to verify
the chain, and how to undo it.

If you just want the fast path, the three steps in the
[README](../../README.md#install-in-two-minutes) are the whole story for most people. Come here
when you want a different path, a per-OS detail, or an explanation of what got registered.

## Prerequisites

- A Chromium browser: Chrome, Edge, Brave, or Chromium, version 116 or newer. The 116 floor comes
  from the extension, which Chrome enforces when it loads; the binary itself checks no version.
- An MCP client (Claude Code, Cursor, VS Code, and others).
- For the npm path, Node.js (for `npx`). For the source path, a stable Rust toolchain
  (https://rustup.rs). Either way the binary has no runtime dependencies.

## Path A: the npm launcher

The launcher fetches a single portable binary on first run and caches it. Nothing to compile.

1. **Add the server** to your MCP client as a stdio server:

       { "command": "npx", "args": ["-y", "ghostlight"] }

   For Claude Code: `claude mcp add ghostlight -- npx -y ghostlight`.

2. **Connect the browser side** (idempotent, safe to re-run):

       npx -y ghostlight install

3. **Add the extension.** Download `ghostlight-extension-v*.zip` from the
   [latest release](https://github.com/sylin-org/ghostlight/releases/latest), unzip it, and load it
   unpacked at `chrome://extensions` (Developer mode, then Load unpacked). A Chrome Web
   Store listing ("Ghostlight in Browser") is in preparation.

4. **Restart your client and reload the extension,** then verify:

       npx -y ghostlight doctor

## Path B: build from source

The path when you want to read what you are running.

    git clone https://github.com/sylin-org/ghostlight
    cd ghostlight
    cargo build --release

The build produces two executables. `ghostlight` is the CLI and the persistent service.
`ghostlight-relay` is the thin pass-through your MCP client and Chrome actually launch; it depends
on almost nothing, so rebuilding the service never forces it to relink. Load the extension as in
Path A step 3 (from the local `extension/` directory), then register:

    ./target/release/ghostlight install --extension-id cjcmhepmagomefjggkcohdbfemacojoa

Verify with `./target/release/ghostlight doctor`.

## What `install` actually does

It is worth knowing what gets written, because the answer is "less, and more carefully, than you
might expect." For each browser and client it targets, `install`:

- **Registers the native-messaging host** so the browser can launch Ghostlight. On Windows that is
  a registry entry (per-user under HKCU, or system-wide under HKLM with `--system`) plus a host
  manifest file; on macOS and Linux it is a host manifest file in each browser's host directory.
- **Adds the MCP server to your client's config** with an idempotent, value-level merge. This is
  the part to trust: it re-reads the file at write time and changes only the one entry it owns, so
  it never clobbers a hand-edited config and never duplicates itself if you run it twice.
- **Allow-lists the extension** by id. The Web Store and unpacked-dev ids are always allowed;
  `--extension-id` adds another.
- **Registers an auto-start supervisor** so the service is there when a client asks for it. Skip it
  with `--no-supervisor`.

The client entry it writes points at `ghostlight-relay` with `--role agent`. You never launch the
binary by hand; the client and the browser do.

### Which clients and browsers it knows

`install` auto-detects and registers four clients (`claude-code`, `claude-desktop`, `cursor`,
`vscode`) and four browsers (`chrome`, `edge`, `brave`, `chromium`). That list is smaller than the
set of clients Ghostlight *works* with, and the gap is worth understanding. Any MCP client can use
Ghostlight; the installer only knows how to write config for these four, because each has its own
config location and dialect it handles specifically. For anything else (Zed, Cline, and the rest),
add the stdio server entry from Path A step 1 by hand and it behaves the same. The installer's job
is convenience, not gatekeeping.

### Useful flags

- `--dry-run` computes and prints the plan without writing anything. A good habit before the first
  real run.
- `--browser <id>` / `--client <id>` limit the scope (repeatable); `--all-browsers` /
  `--all-clients` widen it to every known target, detected or not.
- `--system` registers machine-wide (HKLM) instead of per-user.
- `--debug` registers the server to run with observability on.
- `--extension-id <id>` allows an additional extension id.

## Verify with `doctor`

`ghostlight doctor` is read-only and diagnoses the whole chain: browser registered, client
registered, IPC endpoint accepting, extension connected. A healthy run exits 0. Anything wrong
prints as a specific, actionable finding rather than a generic failure. `--verbose` adds detail,
and `--fix` is the one mode that changes anything, reaping orphaned sessions and clearing stale
state.

## Uninstall

    ghostlight uninstall

This reverses what `install` wrote: the native-host registration, the client entries (again by
idempotent merge, so a foreign config is left alone), the per-instance relay copy, and the
supervisor. `--dry-run` shows the plan first.

## Troubleshooting

- **Start with `doctor`.** It pinpoints the common failures by name.
- **Extension shows disconnected?** Reload it at `chrome://extensions`. A service worker can be
  evicted; reloading re-establishes the link.
- **Rebuilding on Windows?** Stop the MCP client first. A running client holds the relay executable
  open, and the build cannot overwrite a locked file. This is the most common "my build failed for
  no reason" on Windows, and it has a one-line cause.
- **Ran `ghostlight` and got an error exit?** That is expected. A bare `ghostlight` with no
  subcommand no longer serves anything; the MCP role lives in `ghostlight-relay`, which your client
  launches. Run a real subcommand (`install`, `doctor`, `status`), or let the client drive the
  relay.

## Environment variables

For most installs you set none of these. When you need them:

- `GHOSTLIGHT_DEBUG=1`: observability on (same as `--debug`).
- `GHOSTLIGHT_MANIFEST=file://...`: point the server at a policy manifest (see
  [governance-configuration.md](governance-configuration.md)).
- `GHOSTLIGHT_INSTANCE=<name>`: select a named, isolated instance (advanced; lets two independent
  setups coexist on one machine).
- `GHOSTLIGHT_AUDIT_DIR`, `GHOSTLIGHT_LOG_DIR`: relocate the audit and log directories.
- `GHOSTLIGHT_ENDPOINT` / `GHOSTLIGHT_ENDPOINTS`: pin the IPC endpoint name(s).
