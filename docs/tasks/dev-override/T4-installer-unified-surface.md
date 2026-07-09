# T4 -- the unified install surface (ADR-0048 D5/D6)

## Goal

The default install's host manifest allows BOTH shipped extension ids (Web Store + pinned
unpacked-dev), `--extension-id` becomes an optional EXTRA origin (the `MissingExtensionId` error
is retired), and `--instance dev install` thins to pinned MCP-client entries only (no host, no
copy, no supervisor) -- dev browser traffic rides the unified default host. Dev UNINSTALL keeps
full legacy cleanup, unchanged. Normative: ADR-0048 D5/D6. Oracles: PINS.md P4.

## Files this task owns (touch nothing else)

- `crates/core/src/install/native_host.rs`
- `crates/core/src/install/mod.rs`
- `crates/transport/src/error.rs` (variant removal only)
- `src/main.rs` (ONE help-comment line -- the batch's sanctioned exception)
- `tests/install_instance.rs` (the dev-plan test moves with the design; PINS P4)
- `docs/tasks/dev-override/LEDGER.md` (ledger commit)

## Verified tree facts (as of dev @ 3928a74 -- re-read before editing)

- native_host.rs: `HostManifest { path, allowed_origins }`; `resolve(current_exe, extension_id:
  Option<&str>)` does `let id = extension_id.ok_or(Error::MissingExtensionId)?;` and builds
  `allowed_origins: vec![origin_for(id)]`; `origin_for`, `validate_extension_id`,
  `normalize_exe_path` exist. Tests `host_manifest_json_has_type_stdio_and_exact_origin`
  (asserts len == 1) and `missing_extension_id_is_an_error` exist.
- error.rs: the `MissingExtensionId` variant with its doc + `#[error(...)]` attr; used ONLY by
  native_host.rs (`grep -rn "MissingExtensionId" crates/ src/` = the variant + the two
  native_host.rs sites).
- mod.rs: `fn plan_install(opts, ctx)` opens with the launcher/manifest resolution and the
  windows/unix host blocks, then the MCP-clients section; `run_install` has the
  `if opts.no_supervisor { ... } else { supervisor::apply_steps(...) }` arm; `plan_uninstall`
  handles per-instance copy removal (UNTOUCHED by this task).
- src/main.rs InstallArgs: the help comment
  `/// Unpacked-dev extension id (32 chars, a-p). Required until a build-time key ships.`
- tests/install_instance.rs drives `install --dry-run` as a subprocess and contains
  `dev_install_plan_copies_a_named_binary_and_suffixes_the_whole_stack`, which asserts the
  PRE-0048 dev plan (per-instance copy + suffixed host + suffixed supervisor) -- it MUST be
  replaced by the two pinned tests or `cargo test --workspace` goes red after this task.
- The two shipped ids (pinned in P4): store `lejccfmoeogmhemakeknjjdhkfkgncdl`, dev
  `cjcmhepmagomefjggkcohdbfemacojoa` (both documented in-tree: README.md, docs/legal/
  STORE_LISTING.md, ADR-0016).

## STOP preconditions

- STOP if `STORE_EXTENSION_ID` or `DEV_EXTENSION_ID` already exist anywhere.
- STOP if `MissingExtensionId` has callers OUTSIDE native_host.rs.
- STOP if `plan_install` no longer matches the shape above (anchor drift).
- STOP if T1 is not landed (`DEV_INSTANCE` must exist in transport::instance).

## Changes (transcribe from PINS P4)

1. native_host.rs: the two id consts; the pinned `resolve` rewrite; the pinned test updates
   (`host_manifest_json_has_type_stdio_and_exact_origin` origins block;
   `missing_extension_id_is_an_error` DELETED,
   `resolve_without_an_id_allows_the_two_shipped_extensions` ADDED).
2. error.rs: delete the `MissingExtensionId` variant (doc + attr + variant lines).
3. mod.rs: the `plan_install` -> `plan_install_for` split with the pinned `dev_thin` wrap; the
   pinned `run_install` supervisor middle arm; the pinned
   `plan_install_for_the_dev_instance_is_client_entries_only` test.
4. src/main.rs: the ONE pinned help-comment line.
5. tests/install_instance.rs: the pinned module-doc sentence; DELETE
   `dev_install_plan_copies_a_named_binary_and_suffixes_the_whole_stack`; ADD the two pinned
   tests (`dev_install_plan_is_thin_client_entries_only`,
   `a_named_non_dev_instance_still_plans_the_full_stack`); the other two tests stay
   byte-identical.

## Verification (all green, in this order)

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test -p ghostlight-core --no-fail-fast
cargo test --workspace --no-fail-fast
cargo check --target x86_64-unknown-linux-gnu --workspace --all-targets
```

Do NOT run `ghostlight install` (not even --dry-run) as verification; the unit tests are the
gate.

## Out of scope (fences)

- NO change to `plan_uninstall`, `plan_client_uninstall`, or any uninstall path (dev uninstall
  keeps full legacy cleanup BY KEEPING TODAY'S CODE).
- NO change to `clients.rs` (the default entry already writes `args: []`, which T1 made
  unpinned; nothing to do).
- NO change to `instance_launcher` itself (it is simply not CALLED on the dev-thin path).
- NO change to supervisor registration steps for non-dev instances.
- src/main.rs: ONLY the one pinned line; nothing else in that file.

## Commit

Stage exactly the five named source files. Pinned message (PINS P4):

```
feat(install): one browser surface -- both shipped extension ids allowed, dev install thinned (ADR-0048 D5/D6)
```

Then update LEDGER.md and commit as `docs(dev-override): ledger T4`.
