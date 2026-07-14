---
title: Architecture
description: Follow audio, parameters, state, and UI messages through Vesty.
order: 1
---

## Runtime shape

```text
DAW / VST3 host
  ├─ factory + controller ── parameter edits, state, editor lifecycle
  └─ audio processor ─────── automation + events + audio buffers
             │
             ▼
        AudioKernel
             │ fixed-capacity telemetry queues
             ▼
      BridgeRuntime ─────── system WebView ─────── @vesty/plugin-ui
```

The processor and controller are separate VST3 objects. Vesty keeps their shared responsibilities explicit rather than hiding them behind a global mutable singleton.

## Plugin and kernel

`Plugin` owns long-lived metadata and parameter storage. `AudioKernel` owns mutable DSP state. The adapter creates and prepares the kernel outside the audio callback, then calls `process()` for each host block.

Lifecycle hooks are deliberately small:

```rust
pub trait AudioKernel: Send + 'static {
    fn prepare(&mut self, context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
}
```

Use `prepare` for sample-rate and maximum-block-size setup, `reset` after host processing configuration changes, and `suspend` / `resume` for transport-independent processing pauses.

## Parameters and automation

At block entry, the adapter reads host parameter queues into a preallocated event list. Events are sorted by sample offset and rendered by the kernel. The final value is written to the atomic parameter store only after the block finishes, so samples before the first automation point still observe the previous block value.

## Web editor boundary

The UI runtime uses direct `wry` embedding and a versioned JSON protocol. It never calls WebView APIs from the audio thread. Meters and logs move through bounded queues and use latest-wins behavior where dropping stale data is preferable to blocking.

## Panic boundaries

Every host-facing COM callback is wrapped by an ABI panic boundary. A panic becomes the appropriate VST3 fallback result rather than unwinding through foreign code. During audio processing, faults additionally produce silence and a diagnostic event.

