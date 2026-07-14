---
name: vesty-plugin-dev
description: Build, modify, debug, test, package, and review Rust VST3 plugins made with the Vesty framework, including typed parameters, realtime-safe AudioKernel DSP, instruments and MIDI events, system WebView editors, JSBridge integrations, parameter manifests, CLI workflows, bundle validation, and release evidence. Use when working in a Vesty plugin repository or when a user asks an AI agent to create or change a Vesty effect, instrument, DSP algorithm, automation flow, Web UI, or VST3 package.
---

# Vesty Plugin Development

Build Vesty plugins without weakening the realtime boundary or claiming release confidence that has not been proven in a real host.

## Start with repository context

1. Read the nearest `AGENTS.md`, `Cargo.toml`, `vesty.toml`, parameter specs, and existing plugin entry point.
2. Run `vesty templates` and `vesty --help` when choosing a scaffold or CLI workflow. In a Vesty source checkout, use `cargo run -p vesty-cli -- <command>`.
3. Classify the change as effect or instrument, headless or Web UI, and local implementation or release work.
4. Preserve existing class IDs, bundle IDs, parameter string IDs, and generated manifests unless the user explicitly requests a compatibility change.

Read [references/architecture.md](references/architecture.md) before changing layer ownership, plugin lifecycle, buses, state, or events. Read [references/realtime-safety.md](references/realtime-safety.md) before writing or reviewing any `AudioKernel` code. Read [references/web-ui.md](references/web-ui.md) for editors and JSBridge work. Read [references/testing-and-release.md](references/testing-and-release.md) for packaging, validation, CI, or release requests.

## Choose the smallest maintained starting point

- Use `gain` for a headless audio effect.
- Use `midi-synth` for a headless instrument.
- Use `web-ui-param-demo`, `vanilla-ui-param-demo`, `vue-ui-param-demo`, or `svelte-ui-param-demo` for an effect editor.
- Use `web-ui-instrument` for an instrument editor.

Prefer `vesty new <name> --template <id>` over manually recreating project boilerplate. Pass `--vesty-path` and `--plugin-ui-path` when developing against an unpublished local checkout.

## Implement in dependency order

1. Define stable plugin metadata in both Rust `PluginInfo` and `vesty.toml`.
2. Define typed parameters with `FloatParam`, `BoolParam`, or `ChoiceParam` and `#[derive(Params)]`.
3. Resolve `ParamHandle` values once in `create_kernel()`.
4. Put prepared, bounded DSP state in the kernel. Allocate in construction or `prepare()`, never in `process()`.
5. Handle parameter automation with `ParamAutomationSegments` when sample offsets affect output.
6. Process borrowed input/output buffers and explicitly clear outputs that are not written.
7. Add deterministic kernel tests before adding an editor.
8. Add Web UI and JSBridge behavior only after the native parameter flow is correct.
9. Regenerate and check `vesty-parameters.json` whenever parameter specifications intentionally change.
10. Package, statically validate, run the official validator, and collect real host evidence in that order.

## Protect compatibility

- Treat VST3 class IDs, package bundle IDs, parameter IDs, program IDs, and bridge protocol fields as persistent contracts.
- Keep Rust parameter declarations, `params.specs.json`, generated `vesty-parameters.json`, and UI parameter IDs synchronized.
- Never rename `vesti` identifiers back into the repository. Use `vesty`, `Vesty`, or `VESTY` according to context.
- Do not add Tauri. Vesty uses direct `wry` system WebView embedding.

## Verify every change

Run the smallest meaningful tests while iterating, then use the bundled verifier from the plugin root:

```bash
skills/vesty-plugin-dev/scripts/verify.sh .
```

Set `VESTY_MANIFEST=/path/to/vesty/Cargo.toml` when the CLI is not installed and the plugin is outside the framework workspace. Inspect every skipped check; a skipped manifest or UI check is not a pass.

For a Vesty framework checkout, also run the workspace contract checks described in its `AGENTS.md`. Do not fabricate DAW, validator, signing, WebView, CI, or notarization evidence. Report external gates as pending until the corresponding real artifact exists.

## Finish with an evidence-based handoff

State what changed, which checks passed, which checks were skipped or failed, and which external host/platform gates remain. Distinguish a loadable local bundle from a release-ready plugin.
