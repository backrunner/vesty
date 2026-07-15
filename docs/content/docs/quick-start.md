---
title: Get started
description: Build, test, and validate your first Vesty audio effect.
order: 0
---

Vesty keeps DSP in Rust while letting you build the editor with familiar web frameworks. This guide creates the smallest useful audio effect, verifies it, and points you to the next stage of development.

## Requirements

- Rust 1.95 or newer.
- A VST3 host for manual testing.
- Node.js 24 or newer only when the plugin has a Web UI.
- Platform WebView development libraries when compiling the `wry` backend.

## Install Vesty

Install the prebuilt CLI from GitHub Releases. The installer selects the archive for your platform, verifies its SHA-256 checksum, and writes `vesty` to `~/.local/bin` by default.

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/backrunner/vesty/main/scripts/install.sh | sh
```

On Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/backrunner/vesty/main/scripts/install.ps1 | iex
```

The installers use the latest stable GitHub Release. To install an alpha, beta, or other specific version, set `VESTY_VERSION` to the v-prefixed tag listed on the Releases page. You can also download an archive and `SHA256SUMS` manually from the same release.

Verify the CLI and inspect the local toolchain:

```bash
vesty --version
vesty doctor
```

If the shell cannot find `vesty`, add `~/.local/bin` to `PATH` and open a new terminal.

## Create a plugin

List the built-in starters, then scaffold the headless gain effect used in this guide:

```bash
vesty templates
vesty new my-plugin --template gain
cd my-plugin
```

The CLI writes the plugin metadata, parameter manifest, `vesty.toml`, and a Rust dependency pinned to the framework version that matches the CLI. Use `--template web-ui-param-demo`, `--template vue-ui-param-demo`, or `--template svelte-ui-param-demo` when you want a Web editor from the start.

## Inspect the gain effect

The generated `src/lib.rs` follows this minimal structure:

```rust title="src/lib.rs (abridged)"
use vesty::prelude::*;

#[derive(Params)]
struct GainParams {
    gain: FloatParam,
}

impl Default for GainParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0)
                .with_unit("dB"),
        }
    }
}

#[derive(Default)]
struct GainPlugin {
    params: GainParams,
}

struct GainKernel {
    gain: ParamHandle,
}

impl Plugin for GainPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "My Gain",
        vendor: "My Company",
        url: "https://example.invalid",
        email: "",
        version: "0.1.0",
        class_id: *b"MYGAINPLUGIN0001",
        kind: PluginKind::AudioEffect,
    };

    type Params = GainParams;
    type Kernel = GainKernel;

    fn params(&self) -> &Self::Params { &self.params }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        GainKernel { gain: self.params.resolve_or_invalid("gain") }
    }
}

impl AudioKernel for GainKernel {
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        let normalized = context.param_normalized(self.gain).unwrap_or(0.833_333);
        let gain_db = -60.0 + normalized * 72.0;
        let gain = 10.0_f32.powf(gain_db as f32 / 20.0);
        let channels = context.audio().input_channels()
            .min(context.audio().output_channels());
        let audio = context.audio_mut();

        for channel in 0..channels {
            audio.copy_input_to_output(channel, gain);
        }

        ProcessResult::Continue
    }
}

vesty::export_vst3!(GainPlugin);
```

## Verify the workspace

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The repository includes working examples under `examples/gain`, `examples/midi-synth`, and `examples/web-ui-param-demo`.

## Continue

- Learn how the native and WebView halves fit together in [Architecture](/docs/concepts/architecture).
- Read the non-negotiable [Realtime safety](/docs/concepts/realtime-safety) rules.
- Add a parameter with the [Parameters guide](/docs/guides/parameters).
- Build an editor with the [Web UI guide](/docs/guides/web-ui).
