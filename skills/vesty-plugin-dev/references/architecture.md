# Vesty architecture reference

Use this reference when a task changes ownership, lifecycle, state, buses, event handling, or the VST3 adapter boundary.

## Layer responsibilities

| Layer | Owns | Must not own |
| --- | --- | --- |
| Plugin | metadata, typed parameters, UI descriptor, program/state hooks | host buffer references |
| AudioKernel | prepared DSP state, parameter handles, bounded voice/event state | WebView, JSON, files, mutex-protected services |
| VST3 adapter | ABI entry points, host/controller translation, event ordering, buffer views | product DSP policy |
| UI controller | current host-authoritative state, gestures, subscriptions | audio-thread processing |
| Web editor | rendering and user interaction through typed JSBridge | direct kernel access |
| CLI/build | scaffold, manifest generation, packaging, validation, evidence organization | claims about unperformed host tests |

The host calls the exported VST3 factory. Vesty constructs the `Plugin`, exposes its parameters through the controller, creates an `AudioKernel` for processing, and borrows host buffers into `ProcessContext` for one callback.

## Core Rust contract

```rust
pub trait Plugin: Send + Sync + 'static {
    const INFO: PluginInfo;
    type Params: ParamCollection + Send + Sync + 'static;
    type Kernel: AudioKernel;

    fn params(&self) -> &Self::Params;
    fn create_kernel(&self, init: KernelInit) -> Self::Kernel;
}

pub trait AudioKernel: Send + 'static {
    const SUPPORTS_F64: bool = false;
    fn prepare(&mut self, context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
}
```

Use optional `Plugin` hooks only for capabilities the product implements: UI, latency, tail, sidechain inputs, output buses, program lists, custom state, MIDI mapping, and note expression.

## Effects and instruments

Effects read input channels and write output channels. Limit processing to matched channels unless the plugin declares a deliberate remapping, and clear remaining outputs.

Instruments normally clear output buffers first, consume note/events by sample offset, and synthesize into declared output buses. Keep voice storage fixed-capacity or preallocated. `InstrumentKernel` is an alias of `AudioKernel`; it has the same realtime rules.

## Lifecycle

- Construct and resolve stable handles in `create_kernel()`.
- Use `prepare()` for sample-rate-dependent coefficients and maximum-block-size storage.
- Use `reset()` to clear signal history without changing persistent host parameters.
- Use `suspend()` and `resume()` for host activation changes without doing work in the callback.
- Never retain `ProcessContext` buffer or event references after `process()` returns.

## Identity and state

Keep these stable after release:

- `PluginInfo.class_id` and `[plugin].class_id`
- `[package].bundle_id`
- parameter string IDs and their generated VST3 numeric IDs
- program list and program IDs
- serialized state schema keys

Parameter state is host-authoritative. Use `PluginState` hooks only for additional state that is not represented by parameters. Version custom state and provide explicit migration for schema changes.
