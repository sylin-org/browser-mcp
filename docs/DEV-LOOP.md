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

## 2. Install the dev instance (once)

```
ghostlight --instance dev install --no-supervisor --debug --extension-id <your-unpacked-id>
```

`--no-supervisor` skips registering the OS auto-start service. That matters: an auto-started dev
service would relaunch itself from `target/debug` and hold the exe lock during your next rebuild.
With it off, you run the service yourself in a terminal (next step). Then load the unpacked
extension at chrome://extensions and restart your editor so it picks up the new MCP registration.

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
