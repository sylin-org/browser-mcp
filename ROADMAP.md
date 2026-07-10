# Roadmap

Ghostlight ships today as a governed browser-automation MCP server for Chromium, verified
end-to-end on Windows. This page is what we are working toward next. Nothing here changes the
Continuity Promise or the trained tool surface.

## Near term

- **Chrome Web Store listing.** Install the extension without developer mode.
- **Live browser verification on macOS and Linux.** Both already build and pass the full test
  suite in CI; this brings end-to-end browser coverage on par with Windows.
- **`http` audit destination.** Stream audit records to an HTTP collector alongside the existing
  file, stderr, and syslog sinks.
- **Offline license keys for organizations.** For the one case that needs a subscription (see
  [PRICING.md](PRICING.md)), with no phone-home.
- **`managed://` policy distribution.** Deliver policy through MDM and Group Policy for fleets.

## Direction

More adapters will follow on the same governance spine. The browser is the first surface, not
the last. The durable asset is the [RAWX capability model](open-spec/rawx-capability-model.md);
the mechanisms around it will change.

Have a request? [GitHub Discussions](../../discussions) is the place, and every request gets a
disposition with reasoning.
