# Ghostlight guides

Task-oriented guides. Each one owns its topic; the persona walkthroughs point back to the
mechanics guides instead of repeating them, so nothing here drifts out of sync with another page.

| If you want to...                          | Read                                                       |
| ------------------------------------------ | ---------------------------------------------------------- |
| Install Ghostlight and verify it works     | [installation.md](installation.md)                         |
| Get going fast as a solo developer         | [solo-developer.md](solo-developer.md)                     |
| Write and apply a governance policy        | [governance-configuration.md](governance-configuration.md) |
| Roll governance out across an organization | [compliance-team.md](compliance-team.md)                   |
| Send the audit trail to your SIEM          | [siem-integration.md](siem-integration.md)                 |
| Enter or check a license key (paid tier)   | [licensing.md](licensing.md)                               |

## Reference

The authoritative sources, generated from the binary or published as specs, so a guide never has
to go stale repeating them:

- `ghostlight config docs`: every configuration key and its meaning.
- `ghostlight config schema`: JSON Schema for the user config file.
- [open-spec/](../../open-spec/): the RAWX capability model, vendor-neutral.
- [examples/](../../examples/): ready-to-adapt policy manifests.
- [../SPEC.md](../SPEC.md): the authoritative design specification.
