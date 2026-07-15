# Vesty

Vesty is a Rust-first framework for building VST3 audio plugins with realtime-safe DSP and system WebView user interfaces.

Audio processing stays native and deterministic. Plugin editors can use ordinary JavaScript, React, Vue, or Svelte without adding Tauri to the plugin runtime.

> Vesty is alpha software. The core framework and local validation tools are implemented, but release readiness still requires real DAW, platform WebView, Steinberg validator, signing, and notarization evidence.

## What It Provides

- Rust traits and process contexts for audio effects and instruments.
- Typed parameters with stable VST3 IDs and realtime-safe handles.
- Fixed-capacity events, lock-free queues, meters, and diagnostics.
- VST3 factory, processor, controller, state, automation, and editor integration.
- A typed JSBridge with generated TypeScript protocol definitions.
- Direct system WebView embedding through `wry`.
- A `vesty` CLI for scaffolding, building, packaging, validation, and release checks.
- Gain, MIDI synth, and Web UI example plugins.
- A multilingual Svedocs documentation site and companion AI development skill.

## Quick Start

Requirements:

- Rust 1.95 or newer
- Node.js 24 or newer for the UI packages
- Platform WebView development libraries when enabling the `wry` backend

Install the prebuilt CLI from the latest GitHub Release:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/orchiliao/vesty/main/scripts/install.sh | sh
vesty --version
vesty doctor
```

Then create a plugin from an embedded starter:

```bash
vesty templates
vesty new my-plugin --template gain
cd my-plugin
cargo test
```

Windows users can run `irm https://raw.githubusercontent.com/orchiliao/vesty/main/scripts/install.ps1 | iex` in PowerShell. Source checkout instructions remain available for contributors and unreleased development.

The complete English and Simplified Chinese guides live in [`docs/`](docs/). Start with the [complete plugin tutorial](docs/content/docs/guides/complete-plugin.md) for the path from scaffold to validated VST3 bundle.

AI-assisted workflows can use the repository-distributed [`vesty-plugin-dev`](skills/vesty-plugin-dev/SKILL.md) skill. Its instructions preserve realtime boundaries and distinguish local checks from external release evidence.

## Minimal Plugin

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

The audio `process` path must not allocate, lock, block, perform JSON work, call WebView APIs, or format logs.

## Repository Layout

- `crates/`: Rust framework, VST3 adapter, WebView runtime, build support, macros, and CLI.
- `packages/`: `@vesty/plugin-ui` plus React, Vue, and Svelte adapters.
- `examples/`: example VST3 plugins and Web UI assets.
- `docs/`: multilingual Svedocs site, tutorials, guides, and references.
- `skills/`: installable AI development workflows for Vesty projects.
- `.agents/`: architecture research, implementation notes, and completion audits.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

These checks validate the repository locally. They do not replace real host and platform release evidence.

## Contributing

Project-specific development rules are in `AGENTS.md`. Commit messages use:

```text
xxx(comp): desc
```

Keep commits focused and use lowercase types and scopes, for example `fix(vst3): preserve sample-accurate automation`.

## License

Vesty is licensed under the [Apache License 2.0](LICENSE-APACHE).
