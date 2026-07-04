Ghostlight ${VERSION} -- governed browser automation for AI agents.

A single Rust binary plus a thin Chromium extension that gives an AI agent controlled
access to your real, authenticated browser session, with an opt-in governance layer
(capability grants, sacred domains, audit). See the README and docs/guides/ for the
full walkthrough.

## Install

1. Download the archive for your platform below and extract it; put the `ghostlight`
   binary on your PATH.
2. Load the extension: in Chrome open `chrome://extensions`, enable Developer mode, and
   "Load unpacked" the `extension/` directory (from the source tree, or the
   `ghostlight-extension-${VERSION}.zip` below). The pinned extension id is
   `cjcmhepmagomefjggkcohdbfemacojoa`.
3. Register the native host and your MCP client:
   `ghostlight install --extension-id cjcmhepmagomefjggkcohdbfemacojoa`
4. Verify the whole chain: `ghostlight doctor`.

End-to-end verified on Windows; the binary also builds and passes the test suite on
macOS and Linux (live browser verification on those platforms is on the roadmap).

## Verify

Every archive carries a signed build-provenance attestation (GitHub Artifact
Attestations / Sigstore). Prove an artifact was built by this repo's release workflow,
not a mirror or a tampered copy:

```
gh attestation verify <archive> --repo sylin-org/ghostlight
```

The SHA-256 checksums are below (the attestation is the stronger, signed check).

## Downloads

| Platform | Architecture | Download |
|---|---|---|
| Windows | x86_64 | `ghostlight-${VERSION}-x86_64-pc-windows-msvc.zip` |
| macOS | Apple Silicon | `ghostlight-${VERSION}-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `ghostlight-${VERSION}-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `ghostlight-${VERSION}-x86_64-unknown-linux-gnu.tar.gz` |
| Extension | any | `ghostlight-extension-${VERSION}.zip` |

## Checksums (SHA-256)

```
${CHECKSUMS}
```
