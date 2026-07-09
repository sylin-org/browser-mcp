# The Ghostlight dev loop

Ghostlight ships as three executables (ADR-0046): `ghostlight` (the CLI + the persistent
service), `ghostlight-adapter-agent` (the pass-through your MCP client launches), and
`ghostlight-adapter-browser` (the pass-through Chrome launches). Only the service carries the
churny code; both adapters are thin, resilient pipes. That split is what makes the dev loop
frictionless: you rebuild and restart the service while the adapters keep your editor and browser
connected.

Use a named instance (here `dev`) so your work never touches the default install.

## 1. Build

```
cargo build -p ghostlight
```

Build ONLY the `ghostlight` package. It does not relink the two adapter binaries, so a running
`ghostlight-adapter-agent` (launched by your editor) is never locked, and the rebuild always
succeeds even while an editor session is live.

## 2. Install (once)

```
ghostlight install --debug --no-supervisor
```

Since ADR-0048 the plain DEFAULT install is all the dev loop needs: it registers ONE browser
native host (whose manifest already allows the unpacked-dev extension id -- no --extension-id)
and ONE unpinned MCP-client entry (`ghostlight`). An unpinned client resolves at connect time and
PREFERS a live dev instance, so the moment your terminal service (next step) is up, unpinned
clients and the browser route to it; when it is down, they fall back to a default service if one
exists. `--no-supervisor` matters when installing FROM target/debug: an auto-started default
service would hold the exe lock during rebuilds. Then load the unpacked extension at
chrome://extensions and restart your editor once so it picks up the registration.

Optional pin: `ghostlight --instance dev install --debug` additionally registers a PINNED
`ghostlight-dev` client entry (client entries only since ADR-0048 D6 -- no second native host, no
supervisor). Pin a client when you want it bound to dev even while a default service is running
(dev-or-nothing, e.g. mid-rebuild).

## 3. Run the service in a terminal

```
ghostlight --debug --instance dev service --keep-warm
```

`--keep-warm` disables the idle-grace shutdown, so the terminal service stays up between actions
instead of exiting after a quiet window. Note the flag placement: `--debug` is a root-level flag
and must come BEFORE the `service` subcommand (`--instance` and `--keep-warm` are accepted in
either position).

## 4. The edit loop

Edit code, then in the service terminal:

```
Ctrl-C            # stop the running service (releases the exe lock)
cargo build -p ghostlight
ghostlight --instance dev service --keep-warm --debug   # rerun
```

You do NOT restart your editor or the browser. The agent adapter reconnects to the fresh service
within its patient reconnect window (up to 120s; ADR-0045), replays the MCP handshake, and your
next tool call is served by the new code. A rebuild that takes a minute or two is invisible to the
MCP client.
