---
title: Build a complete plugin
description: Take a stereo gain effect from a scaffold to a DAW-ready VST3 bundle.
order: 1
---

This tutorial builds **Signal Gain**, a stereo VST3 effect with sample-accurate gain automation, bypass, a testable DSP kernel, an optional WebView editor, and reproducible package validation. It is deliberately small enough to understand in one sitting, but it crosses the same boundaries as a larger equalizer, compressor, or synthesizer.

At the end, you will have a loadable `.vst3` bundle. A bundle that passes local validation is suitable for development testing; it is not ready for public release until the [release evidence](/docs/tooling/release-evidence) matrix is complete.

## 1. Prepare the toolchain

You need Rust 1.95 or newer. Node.js 24 or newer is required only for a Web UI. You also need the platform WebView development libraries when enabling the `wry` backend and a VST3 host for the final smoke test.

Install the prebuilt CLI as described in [Get started](/docs/quick-start), then verify the executable and local environment:

```bash
vesty --version
vesty doctor
```

The CLI embeds its maintained project templates and pins generated Rust and npm dependencies to the matching Vesty release. A source checkout is only required when contributing to Vesty itself or testing an unreleased framework revision.

## 2. Scaffold the effect

List the maintained starters, then create the headless gain template:

```bash
vesty templates
vesty new signal-gain --template gain
cd signal-gain
```

The generated project has four important files:

```text
signal-gain/
├── Cargo.toml
├── params.specs.json
├── src/lib.rs
└── vesty.toml
```

`Cargo.toml` must produce both an ordinary Rust library for tests and a dynamic library for the host:

```toml title="Cargo.toml"
[lib]
crate-type = ["rlib", "cdylib"]

[dependencies]
vesty = "=0.1.0"
```

The generated dependency version matches `vesty --version`. Keep this exact pin unless you intentionally upgrade the CLI and framework together.

## 3. Establish permanent identities

A host saves the VST3 class ID and parameter IDs in user projects. Treat them like database primary keys: labels may change, IDs should not.

Update `vesty.toml` with your product identity:

```toml title="vesty.toml"
[plugin]
name = "Signal Gain"
vendor = "Signal Works"
version = "0.1.0"
kind = "Fx"
class_id = "5349474e-414c-4741-494e-303030303031"

[package]
bundle_id = "audio.signalworks.signal-gain"
category = "Fx"
parameter_manifest = "vesty-parameters.json"
```

The textual UUID above encodes the same 16 bytes used by Rust. Give every plugin class a unique value and never copy an example class ID into a released product.

Use matching metadata in `src/lib.rs`:

```rust
const INFO: PluginInfo = PluginInfo {
    name: "Signal Gain",
    vendor: "Signal Works",
    url: "https://example.com/signal-gain",
    email: "support@example.com",
    version: "0.1.0",
    class_id: *b"SIGNALGAIN000001",
    kind: PluginKind::AudioEffect,
};
```

Packaging validates these two representations. A mismatch is an error, not metadata that should be guessed at runtime.

## 4. Declare host-visible parameters

Use typed parameters for gain and bypass. The Rust field names are private implementation details; the string IDs are the persistent host contract.

```rust title="src/lib.rs"
use vesty::prelude::*;

#[derive(Params)]
pub struct SignalParams {
    pub gain: FloatParam,
    pub bypass: BoolParam,
}

impl Default for SignalParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0)
                .with_unit("dB"),
            bypass: BoolParam::bypass("bypass", "Bypass", false),
        }
    }
}
```

The host transports normalized values from `0.0` to `1.0`. `FloatParam` owns the conversion between that normalized domain and `-60..12 dB`; the kernel below performs the same linear mapping because audio processing only receives the realtime-safe normalized handle value.

Keep `params.specs.json` synchronized with those declarations:

```json title="params.specs.json"
{
  "version": 1,
  "parameters": [
    {
      "id": "gain",
      "name": "Gain",
      "kind": { "float": { "min": -60.0, "max": 12.0 } },
      "defaultNormalized": 0.8333333333333334,
      "unit": "dB",
      "stepCount": null,
      "flags": {
        "automatable": true,
        "bypass": false,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    },
    {
      "id": "bypass",
      "name": "Bypass",
      "kind": "bool",
      "defaultNormalized": 0.0,
      "unit": null,
      "stepCount": 1,
      "flags": {
        "automatable": true,
        "bypass": true,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    }
  ]
}
```

Generate and commit the bundle manifest:

```bash
vesty param-manifest --specs params.specs.json --out vesty-parameters.json
```

After any intentional specification change, regenerate it. In CI, add `--check` so drift fails rather than silently rewriting the file.

## 5. Separate plugin state from realtime state

The plugin object owns host-facing parameters. The kernel owns only values needed by the audio callback. Resolve string IDs once when the kernel is created:

```rust
#[derive(Default)]
pub struct SignalGain {
    params: SignalParams,
}

pub struct SignalKernel {
    gain: ParamHandle,
    bypass: ParamHandle,
}

impl Plugin for SignalGain {
    const INFO: PluginInfo = PluginInfo {
        name: "Signal Gain",
        vendor: "Signal Works",
        url: "https://example.com/signal-gain",
        email: "support@example.com",
        version: "0.1.0",
        class_id: *b"SIGNALGAIN000001",
        kind: PluginKind::AudioEffect,
    };

    type Params = SignalParams;
    type Kernel = SignalKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        SignalKernel {
            gain: self.params.resolve_or_invalid("gain"),
            bypass: self.params.resolve_or_invalid("bypass"),
        }
    }
}
```

Do not store `String`, UI objects, file handles, mutexes, or dynamically growing collections in the hot path. Allocate delay lines, lookup tables, and scratch storage in `create_kernel()` or `prepare()`, before `process()` starts.

## 6. Process automation without discontinuities

Hosts may place several gain points inside one audio block. Reading one value for the whole block would make automation block-accurate rather than sample-accurate. `ParamAutomationSegments` turns the sorted event list into bounded ranges:

```rust
impl AudioKernel for SignalKernel {
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        let bypass = context.param_normalized(self.bypass).unwrap_or(0.0) >= 0.5;
        let initial_gain = context.param_normalized(self.gain).unwrap_or(0.833_333);
        let frames = context.audio().frames().min(u32::MAX as usize) as u32;
        let channels = context
            .audio()
            .input_channels()
            .min(context.audio().output_channels());
        let (audio, events) = context.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.gain, initial_gain, frames) {
            let gain_db = -60.0 + segment.normalized * 72.0;
            let amplitude = if bypass {
                1.0
            } else {
                10.0_f32.powf(gain_db as f32 / 20.0)
            };

            for channel in 0..channels {
                audio.copy_input_to_output_range(
                    channel,
                    segment.start_sample as usize,
                    segment.end_sample as usize,
                    amplitude,
                );
            }
        }

        ProcessResult::Continue
    }
}

vesty::export_vst3!(SignalGain);
```

This callback performs bounded arithmetic and writes borrowed host buffers. It does not allocate, lock, log, parse JSON, access disk, or call the WebView. A production gain plugin may also smooth sharp steps to avoid clicks; keep the smoother state in `SignalKernel` and update it without allocation.

If the input and output channel counts differ, the loop handles only matched channels. A more complex bus layout must explicitly clear every output it does not write.

## 7. Test the kernel without a DAW

Keep a small deterministic DSP test next to the plugin. The default `0 dB` setting should copy both channels exactly:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gain_is_stereo_passthrough() {
        let plugin = SignalGain::default();
        let mut kernel = plugin.create_kernel(KernelInit {
            sample_rate: 48_000.0,
            max_block_size: 4,
        });
        let left = [1.0_f32, -0.5, 0.25, 0.0];
        let right = [-0.25_f32, 0.5, -1.0, 0.75];
        let inputs: [&[f32]; 2] = [&left, &right];
        let mut left_out = [0.0_f32; 4];
        let mut right_out = [0.0_f32; 4];

        {
            let mut outputs: [&mut [f32]; 2] = [&mut left_out, &mut right_out];
            let audio = AudioBuffers::new(&inputs, &mut outputs);
            let mut context = ProcessContext::new(
                audio,
                plugin.params(),
                &[],
                Transport::default(),
            );
            assert_eq!(kernel.process(&mut context), ProcessResult::Continue);
        }

        assert_eq!(left_out, left);
        assert_eq!(right_out, right);
    }
}
```

Add cases for the minimum and maximum gain, bypass, zero frames, unmatched channel counts, automation at sample `0`, and an automation point at the last frame. For stateful DSP, also cover `prepare()`, `reset()`, suspend/resume, sample-rate changes, and the configured maximum block size.

Run the local quality gate:

```bash
cargo fmt --all --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The VST3 adapter itself enters Vesty's no-allocation guard around host processing. That catches guarded allocator use; code review is still required for locks, syscalls, and third-party functions with unknown timing.

## 8. Add a Web UI when the DSP is stable

You can start with a UI template instead:

```bash
cd ..
vesty new signal-gain-ui --template web-ui-param-demo
cd signal-gain-ui
```

Or add this descriptor to the existing plugin and create a `ui/` app:

```rust
fn ui(&self) -> Option<UiDescriptor> {
    Some(
        UiDescriptor::web_assets("ui")
            .with_dev_url("http://localhost:5173")
            .with_size(900, 560)
            .with_min_size(640, 420)
            .with_resizable(true),
    )
}
```

The UI must initialize controls from `ready.paramValues`, then wrap each user gesture in `beginParamEdit`, `performParamEdit`, and `endParamEdit`. Treat `param.changed` as the confirmed source of truth. See [Web UI](/docs/guides/web-ui) for the full bridge flow.

During development, run the UI server and plugin watcher in separate terminals:

```bash
npm install --prefix ui
npm run dev --prefix ui

vesty dev --config vesty.toml
```

Nothing in the Web UI path may be called from `process()`; JSON and WebView work stay on the controller/UI side of the boundary.

## 9. Build and package the bundle

Build the release dynamic library:

```bash
cargo build --release
```

Package it with the platform-specific binary name:

```bash
# macOS
vesty package \
  --config vesty.toml \
  --platform macos \
  --binary target/release/libsignal_gain.dylib \
  --out target/vesty

# Windows x64: target/release/signal_gain.dll
# Linux x64:   target/release/libsignal_gain.so
```

If the project contains a UI, packaging runs its configured build command and copies only the generated `dist` assets. Never package `node_modules` or a development server URL as release content.

## 10. Validate before opening a DAW

First run strict static validation:

```bash
vesty validate \
  target/vesty/SignalGain.vst3 \
  --static-only \
  --strict \
  --format json \
  --report target/vesty/signal-gain.static.json
```

This checks the bundle layout, exported symbols, metadata agreement, parameter manifest, and UI asset hashes. Then run Steinberg's validator and keep both its report and raw log:

```bash
vesty validate \
  target/vesty/SignalGain.vst3 \
  --strict \
  --validator /absolute/path/to/validator \
  --report target/vesty/signal-gain.validator.json \
  --validator-log target/vesty/signal-gain.validator.log
```

## 11. Complete the host smoke test

Install the bundle into your platform's VST3 development directory, rescan plugins, and verify all of the following in at least one supported DAW:

1. The plugin scans and instantiates without host errors.
2. Stereo audio passes at `0 dB`; gain and bypass produce the expected output.
3. Recorded automation follows the timeline and updates the editor.
4. Saving, closing, and reopening the DAW restores parameter values.
5. The editor opens, resizes, reloads, and reflects host-side changes.
6. Offline render matches realtime render within the algorithm's expected tolerance.
7. Repeated activate/deactivate, sample-rate changes, and buffer-size changes remain stable.

Repeat the validator, host, WebView, and signing checks on every platform you claim to support. Preserve the reports as release artifacts. Vesty can organize and verify that evidence, but it cannot manufacture proof that a real host or platform was tested.

## Where to go next

- Use [DSP kernels](/docs/guides/dsp) for prepared state, events, meters, sidechains, and f64 processing.
- Use [State and lifecycle](/docs/guides/state-and-lifecycle) for custom non-parameter state and migration.
- Study `examples/midi-synth` before writing an instrument.
- Use [AI-assisted development](/docs/tooling/ai-development) to install the companion skill and keep an AI agent inside the same realtime and release constraints.
