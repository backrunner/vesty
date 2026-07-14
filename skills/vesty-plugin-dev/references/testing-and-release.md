# Testing, packaging, and release reference

Read this reference for test, validation, packaging, CI, signing, notarization, or release-readiness work.

## Local implementation gate

Run from the affected plugin or workspace:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For UI work, also run the repository's typecheck, unit tests, and production build. For framework protocol work, run `vesty export-types --out target/vesty-protocol --check`.

Kernel tests should cover nominal output, parameter extremes, bypass, zero frames, channel mismatches, automation sample offsets, configured maximum block size, reset/lifecycle behavior, and bounded failure. Instruments also need note-on/off ordering, overlapping notes, velocity, expression/pitch events, and voice-capacity behavior.

## Parameter manifest gate

Commit `params.specs.json` and the generated `vesty-parameters.json`. Run:

```bash
vesty param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json \
  --check
```

Do not regenerate merely to hide an accidental ID change. Review compatibility first.

## Package and static validation

Build a release `cdylib`, then pass the platform binary to `vesty package`. Use `.dylib` on macOS, `.dll` on Windows x64, and `.so` on Linux x64.

```bash
vesty package \
  --config vesty.toml \
  --platform macos \
  --binary target/release/libmy_plugin.dylib \
  --out target/vesty

vesty validate target/vesty/MyPlugin.vst3 \
  --static-only \
  --strict \
  --format json \
  --report target/vesty/MyPlugin.static.json
```

Strict static validation checks layout, metadata agreement, binary exports, parameter manifest, and Web UI asset hashes. It is not the Steinberg validator and does not instantiate the plugin in a DAW.

## External release evidence

A public release requires real artifacts for every claimed platform:

- Steinberg validator report and raw log;
- DAW scan, instantiate, UI, automation, save/restore, and offline-render smoke results;
- system WebView smoke result for the platform;
- CI run URL and downloadable build/test artifacts;
- platform signing verification;
- Apple notarization acceptance and stapling where applicable.

Use `vesty release-check` to organize and audit evidence. Pending templates and fabricated markers are not evidence. A locally loadable bundle is not automatically release-ready.

## Handoff language

Report each category separately:

- **Passed locally:** name exact commands and results.
- **Not run:** explain missing tool, host, platform, or credential.
- **External evidence pending:** name the required artifact.
- **Release-ready:** use only when every declared release gate is supported by real evidence.
