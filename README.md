# Vesty

Vesty is a Rust-first framework for building VST3 audio plugins with realtime-safe DSP and system WebView user interfaces.

The project keeps plugin audio code native and deterministic while letting editor UIs use ordinary web tooling. The MVP target is VST3, with `wry` used directly for system WebView embedding. Vesty does not use Tauri.

## Status

Vesty is an alpha framework skeleton. The workspace currently includes:

- Rust plugin traits, process contexts, audio buffer APIs, host/profile metadata, and plugin state types.
- Typed parameters, normalized/plain conversion, smoothing, handles, schema export, and stable VST3 parameter IDs.
- Realtime queues, fixed event lists, allocation guards, and audio-thread safety checks.
- VST3 adapter code, binding baseline utilities, packaging helpers, validators, and release-evidence tooling.
- A typed JSBridge, bridge state store, subscriptions, diagnostics, logs, meters, and framework-agnostic UI runtime traits.
- A CLI named `vesty` for scaffolding, building, packaging, validation, protocol export, manifest generation, evidence collection, and release checks.
- npm packages under the `@vesty/*` scope for framework-agnostic, React, Vue, and Svelte WebView UI integration.
- Example plugins for gain, MIDI synth, and a web UI parameter demo.

Release readiness still requires external evidence beyond unit tests, including real DAW smoke tests, platform WebView checks, Steinberg validator output, signing verification, CI artifacts, and macOS notarization/stapling logs.

## Repository Layout

- `crates/vesty`: facade crate and public prelude.
- `crates/vesty-core`: plugin traits, audio buffers, events, state, UI descriptors, and host profiles.
- `crates/vesty-params`: typed parameters, conversion, smoothing, handles, schema export, and stable VST3 IDs.
- `crates/vesty-rt`: realtime-safe queues, fixed event lists, and allocation guards.
- `crates/vesty-vst3`: host-facing VST3 adapter.
- `crates/vesty-vst3-sys`: VST3 binding baseline, SDK probes, ABI metadata, and generated-binding planning helpers.
- `crates/vesty-ipc`: JSBridge packets and TypeScript/JSON schema exports.
- `crates/vesty-bridge`: bridge state store, routing, subscriptions, backpressure, gestures, meters, logs, and diagnostics.
- `crates/vesty-ui`: host-agnostic UI runtime traits.
- `crates/vesty-ui-wry`: system WebView runtime built on `wry`.
- `crates/vesty-build`: `vesty.toml`, UI asset manifests, VST3 bundle packaging, and static validation.
- `crates/vesty-cli`: the `vesty` command-line tool.
- `crates/vesty-macros`: derive macros for plugin ergonomics.
- `packages/plugin-ui`: framework-agnostic JavaScript bridge SDK.
- `packages/react`, `packages/vue`, `packages/svelte`: thin adapters over `@vesty/plugin-ui`.
- `examples/gain`, `examples/midi-synth`, `examples/web-ui-param-demo`: sample plugins and UI flows.
- `.agents`: deeper project research, architecture notes, implementation status, and completion audit references.

## Minimal Plugin Shape

```rust
use vesty::prelude::*;

#[derive(Params)]
struct MyParams {
    gain: FloatParam,
}

impl Default for MyParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
        }
    }
}

#[derive(Default)]
struct MyPlugin {
    params: MyParams,
}

struct MyKernel {
    gain: ParamHandle,
}

impl Plugin for MyPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "My Plugin",
        vendor: "My Company",
        url: "https://github.com/orchiliao/vesty",
        email: "",
        version: "0.1.0",
        class_id: *b"VESTYEXAMPLE0001",
        kind: PluginKind::AudioEffect,
    };

    type Params = MyParams;
    type Kernel = MyKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        MyKernel {
            gain: self.params.resolve_or_invalid("gain"),
        }
    }
}

impl AudioKernel for MyKernel {
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        let normalized = context.param_normalized(self.gain).unwrap_or(0.833_333);
        let gain_db = -60.0 + normalized * 72.0;
        let gain = 10.0_f32.powf(gain_db as f32 / 20.0);
        let channels = context
            .audio()
            .input_channels()
            .min(context.audio().output_channels());
        let audio = context.audio_mut();

        for channel in 0..channels {
            audio.copy_input_to_output(channel, gain);
        }

        ProcessResult::Continue
    }
}

vesty::export_vst3!(MyPlugin);
```

## Web UI Model

Vesty Web UIs communicate with Rust through a typed JSBridge:

- JS to Rust: `window.ipc.postMessage(JSON.stringify(packet))`.
- Rust to JS: UI-thread script evaluation with batched delivery.
- State is snapshot + revision + typed commands.
- Parameters are host/controller authoritative.
- Meters and analyzers are latest-wins streams.
- Reliable lifecycle/state events use explicit subscriptions.

Use `@vesty/plugin-ui` directly, or one of the React, Vue, and Svelte adapters.

## Realtime Rules

The audio `process` path must not allocate, lock, perform JSON work, touch WebView APIs, format logs, or block. Use the provided atomics, parameter handles, fixed event lists, SPSC queues, and allocation guards to keep realtime boundaries visible.

The default DSP path is f32. Plugins that need true double-precision host processing can set `AudioKernel::SUPPORTS_F64 = true` and implement `process_f64(&mut ProcessContext64)`.

## Local Checks

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo run -p vesty-cli -- param-manifest --specs examples/gain/params.specs.json --out examples/gain/vesty-parameters.json --check
npm run build --prefix examples/web-ui-param-demo/ui
cargo run -p vesty-cli -- smoke-host --out target/smoke-host/smoke-host.json --check
```

`vesty smoke-host` is a local headless framework self-check for example configs, parameter sidecars, Web UI assets, and optional bridge/meter traces. It does not replace real DAW, platform WebView, validator, signing, notarization, or CI evidence.

## Development Notes

Agent-facing project rules live in `AGENTS.md`. Additional architecture and audit notes live under `.agents/`.

Commits should use the project convention:

```text
xxx(comp): desc
```

Examples:

- `feat(cli): add release evidence import`
- `fix(vst3): preserve controller parameter state`
- `docs(project): update contributor guidance`

## License

Vesty is licensed under either of:

- Apache License, Version 2.0, in `LICENSE-APACHE`
- MIT license, in `LICENSE-MIT`

at your option.
