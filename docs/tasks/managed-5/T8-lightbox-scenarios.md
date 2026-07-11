# T8: Lightbox scenarios -- passport-freshness + sidecar-propagation (ADR-0056 D5)

## Goal
Two named, runnable proofs of the Phase 5 surfaces, through the REAL code with injected paths.

## Preconditions (verify, else STOP)
- T2..T7 DONE per LEDGER.
- `crates/lightbox/src/scenarios.rs` has `pub fn registry() -> Vec<Scenario>` and support helpers
  `TempRoot, BundleServer, sign, manifest, write_bootstrap` in `crates/lightbox/src/support.rs`.

## Required behavior
Add to the registry (names EXACT), implemented in scenarios.rs following the existing style:
1. `("sidecar-propagation", sidecar_propagation)`:
   - TempRoot + `GovernancePaths::under`; BundleServer serving a signed seq-5 bundle; bootstrap.
   - `managed::activate(...)` once -> read the sidecar via
     `governance::managed::status::{sidecar_path, read_sidecar}`; ensure freshness=="fresh",
     seq==Some(5).
   - `server.set_bundle(sign(seed, 6, ...))`; activate again -> sidecar seq==Some(6).
   - Drop the server; activate again -> sidecar freshness=="last_known_good",
     stale_reason==Some("source_unreachable"), seq stays Some(6). This IS the admin's
     "did it propagate?" artifact, end to end.
2. `("passport-freshness", passport_freshness)`:
   - Local-path bundle signed with a presentation: org_name "Acme Security", one contact
     value "security@acme.example" (build `bundle::sign_bundle(seed, None, 3, manifest, Some(p))` --
     note sign_bundle takes the presentation param; construct
     `Presentation{org_name:Some(..),rationale:None,contacts:vec![Contact{kind:"email".into(),
     value:"security@acme.example".into(),label:None}]}`).
   - activate -> read sidecar -> `ghostlight_core::governance::explain::managed_passport(&status)`;
     ensure the returned string contains `Governed by: Acme Security.` and `Policy version 3,` and
     the sacred-domains line, and `security@acme.example`.

## Verification (literal)
- `cargo run -q -p ghostlight-lightbox -- run --all` -> ALL scenarios `ok` (now 9).
- `cargo clippy -p ghostlight-lightbox --all-targets -- -D warnings` -> clean.
- Global verification per BOOTSTRAP.

## Out of scope
- No changes under crates/core (if a needed item is not public, STOP -- record BLOCKED; making an
  item `pub` is sanctioned ONLY if it is in the T2/T5 public lists already and was mistakenly left
  private, and then is a one-word change noted as a deviation).

## Commit message (pinned)
`feat(lightbox): T8 passport-freshness + sidecar-propagation scenarios (ADR-0056 D5)`
