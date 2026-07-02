# Stage 2 browser tests

Deferred live-browser verification for stage-2 governance. The unattended executor CANNOT drive a real
browser, so every check that needs one is written here instead of run. A human runs these against a
live browser after the code lands (as in release-1). Accumulate entries as tasks land; do not delete
them.

## Format

One entry per check:

```
## <task-id>-<n>: <one-line purpose>
Changed: <what code changed and why a browser is needed to verify it>
Steps: <exact, ordered steps a human runs (tools, URLs, inputs)>
Expect: <the precise observable result that means PASS>
```

Keep steps concrete and self-contained (name the tool, the URL, the manifest/config used). Prefer
checks that are unambiguous to eyeball. Note when a check depends on a specific manifest or config
posture (all-open vs a restrictive manifest vs observe/shadow mode).

## Checks

(none yet; appended as governance tasks land)
