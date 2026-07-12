# ADR-0061: Extension-owned browser identity and server-assigned tab slots

- Status: Accepted (design; implementation pending)
- Date: 2026-07-12
- Amends: ADR-0058 (per-browser identity and focus routing)

## Context

ADR-0058 gave each connected browser an identity so the Hub can route to it and composite tab ids
carry their owning browser. That identity is the browser's OS pid, which the browser-role relay
derives from `proc::parent()` and sends in the hello as `browserPid`; sessions are keyed by it, and
the composite tab id is `browserPid * 2^32 + native_tab_id`.

Live testing the `ghostlight demo` tour surfaced a real, reproducible failure: `navigate` returned
"the browser that owns this tab is no longer connected", and the minted tab id decoded to
**browser-pid 0**. Root cause, traced in `hub/outbound/browser.rs`:

1. **`browserPid` can be 0.** `attach()` reads `hello.browserPid ... unwrap_or(0)`. When the relay's
   `proc::parent()` cannot resolve the browser process, the session is keyed `0`.
2. **`resolve_target(None)` (used by `tabs_create`, which has no input tab id) falls back to
   `sessions.keys().min()`** when the focus chain has no live entry -- so a lingering pid-0 session
   (a hard-killed relay that never cleanly detached) is the smallest key and gets picked. The new
   tab is minted under pid 0 even though the live browser is a different pid; `navigate` then decodes
   pid 0, routes to the dead session, and fails.

The deeper problem is that identity is **guessed by the relay from an OS artifact** that is indirect,
not guaranteed non-zero, and not the most stable choice. A spot-fix (fall back to `relayPid` when
`browserPid` is 0) removes the immediate symptom but leaves identity sourced from process metadata.
This ADR does the root fix instead.

A related question was raised: should the tab id be a string (`"{pid}:{tabid}"`) rather than a
composite number? Rejected: `tabId` is `"type": "number"` in the trained tool schemas (a frozen
ADR-0034 D7 field, and the type the per-call input validator enforces), so a string would break the
contract and gamble on Claude's round-trip behavior. The design below keeps the number tab id.

## Decision

**Browser identity belongs to the extension, not the relay.** The extension is the one entity that
persists across every relay reconnect and service-worker relaunch, so it owns the identity:

1. **Extension mints a persistent browser id.** On first run the extension generates a UUID and
   stores it in `chrome.storage.local` under `ghostlight_browser_id` (`local`, not `session`, so it
   survives service-worker death). It reads it back on every startup and includes it in every
   native-messaging hello. Always present, never 0, unique per browser profile, stable across relay
   reconnects AND SW relaunches -- strictly more than `browserPid` or `relayPid` gives.
2. **The hello carries `browserId`** (`handshake::browser_hello_bytes`), alongside the existing
   fields. `proc::parent()` stays ONLY for the browser-role parent-death watchdog (what it is
   actually good at); it is no longer the identity.
3. **The service keys browser sessions by `browserId`.** A reconnect from the same browser (same
   UUID) cleanly REPLACES its session -- the exact ADR-0058 semantics, now hung on a stable,
   reliable, never-zero identity.
4. **The service assigns each browser a small, stable numeric `slot`** (1, 2, 3, ...; never 0),
   mapped from its UUID for the lifetime of the service. The composite tab id stays exactly as
   ADR-0058 designed -- `slot * 2^32 + native_tab_id` -- but `slot` replaces the guessed pid.
   Decoding routes `slot -> UUID -> session`. Because slots are dense, non-zero, and always map to a
   live browser, the `pid=0` and `min()-picks-a-corpse` failures are impossible by construction.

This is the synthesis of both threads: the tab id STAYS a `number` (Claude-safe, no D7 change, no
string round-trip gamble), but its high bits become a reliable server-assigned slot rather than an
unreliable pid; and identity moves to where it belongs. No lookup table for tabs (only a small
slot<->UUID map), no schema change.

Prior art: an application-minted persistent instance id is the standard device/client-identity
pattern (browser fingerprint-free device ids, mobile install ids, WebSocket client tokens) -- the
canonical way to get identity that survives transport churn without relying on process metadata.

## Consequences

- Fixes the pid-0 routing failure at the root; a freshly created tab always carries the pid... slot
  of the actual live browser.
- Identity survives relay reconnects and service-worker relaunches, which `browserPid` did not
  reliably do.
- The number tab id and the composite arithmetic are preserved; Claude stays on trained rails.
- Dead-session hygiene still matters (a slot must be freed when its browser disconnects), but a
  stale slot can no longer be silently minted onto a new tab, because slots map to UUIDs and a new
  tab is minted under the resolving live session's slot.
- Slots are per-service-lifetime (a service restart re-assigns them); outstanding tab ids from
  before a restart are stale, which is already true today (a restart re-groups tabs).

## Implementation plan (three layers)

1. **Extension (`extension/service-worker.js`, maybe `extension/lib/`):** a small module that reads
   `ghostlight_browser_id` from `chrome.storage.local`, generating + persisting a `crypto.randomUUID()`
   if absent, and exposes it to the hello-send path. Mirror the injected-dependency, unit-testable
   shape of `lib/debug.js` / `lib/grouping.js` (a `createBrowserIdentity(storage)` factory), with a
   `tests/extension/*.test.js`.
2. **Transport (`crates/transport/src/handshake.rs`):** `browser_hello_bytes` gains a `browser_id:
   &str` (UUID) field; `ROLE_BROWSER` hello carries `browserId`. Keep `browserPid`/`browserCreated`
   for the watchdog only, or drop from identity use.
3. **Core (`crates/core/src/hub/outbound/browser.rs` + `crates/core/src/constants.rs`):**
   - A `BrowserRegistry`: `browser_id (String) -> slot (u32, monotonic from 1)`, plus the reverse
     for decode. `attach()` looks up/inserts by `browserId`, assigns a slot, keys `sessions` by slot.
   - `tab_id::{encode,decode}` unchanged (already `slot * 2^32 + native`); callers pass `slot`.
   - `resolve_target(None)` picks the most-recently-focused LIVE slot; drop the `min()` fallback in
     favor of "any live slot, focus-ordered" (still deterministic, never a corpse). Evict a slot's
     session on relay disconnect (tighten the detach path).
   - `note_focus`, `focus_chain`, `encode_tab_ids`, `merge_tab_id` all key on slot.
   - `ghostlight doctor`'s browser list shows slot + (optionally) a short id prefix.
- Tests: a reconnect from the same `browserId` replaces (not duplicates) the session; two distinct
  `browserId`s get distinct non-zero slots; a `tabs_create` mint + `navigate` round trip routes to
  the same live browser; `browserId` absent/blank is rejected at attach (fail closed, no pid-0
  fallback path remaining).
