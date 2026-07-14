---
title: Plugin API
description: A compact reference for the Rust authoring surface.
order: 1
---

## Plugin

```rust
pub trait Plugin: Send + Sync + 'static {
    const INFO: PluginInfo;
    type Params: ParamCollection;
    type Kernel: AudioKernel;

    fn params(&self) -> &Self::Params;
    fn create_kernel(&self, init: KernelInit) -> Self::Kernel;
}
```

Optional hooks expose UI descriptors, buses, programs, state, MIDI mappings, note expression, latency, and tail length. Implement only the capabilities the plugin actually supports.

## AudioKernel

```rust
pub trait AudioKernel: Send + 'static {
    const SUPPORTS_F64: bool = false;
    fn prepare(&mut self, context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
    fn process_f64(&mut self, context: &mut ProcessContext64<'_>) -> ProcessResult;
}
```

`InstrumentKernel` is an alias of `AudioKernel`; effects and instruments use the same realtime contract.

## Process contexts

`ProcessContext` and `ProcessContext64` expose:

- Input, sidechain, and output audio buffers.
- Sorted events with sample offsets.
- Transport state and process mode.
- Constant-time parameter handle reads.
- Realtime meter and diagnostic producers.

The context borrows host buffers for one callback. Do not retain references after `process()` returns.

## Parameters

`FloatParam`, `BoolParam`, and choice parameters produce `ParamSpec` metadata. `#[derive(Params)]` implements `ParamCollection`, including allocation-free handle access. Use `#[param(skip)]`, explicit IDs, and bypass flags only where their semantics are stable.

## Export

```rust
vesty::export_vst3!(MyPlugin);
```

The macro exports the platform factory and module entry points with panic guards. A plugin type must implement `Default` because the host factory creates instances without application-owned constructor arguments.

