---
title: DSP kernels
description: Prepare, process, and test bounded native audio work.
order: 2
---

## Prepare outside the callback

Use `prepare()` to configure coefficients and preallocate storage from the host's sample rate and maximum block size.

```rust
impl AudioKernel for DelayKernel {
    fn prepare(&mut self, context: PrepareContext) {
        self.sample_rate = context.sample_rate;
        self.delay_line.resize(context.max_block_size * 8, 0.0);
        self.write = 0;
    }

    fn reset(&mut self) {
        self.delay_line.fill(0.0);
        self.write = 0;
    }

    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        // Only bounded reads, writes, arithmetic, and preallocated state here.
        ProcessResult::Continue
    }
}
```

The resize belongs in `prepare`, never in `process`.

## Buffer access

`ProcessContext` exposes non-owning input and output channels for the current block. Handle hosts that provide different input/output channel counts and clear outputs that you do not write.

Return `ProcessResult::Silence` when the entire output is known to be silent. Otherwise return `Continue`.

## Events and transport

The context carries sorted parameter and note events plus a transport snapshot. Instruments should consume NoteOn, NoteOff, pressure, pitch bend, and expression events by sample offset. Effects can inspect tempo and project position without querying the host from the callback.

## Double precision

The default path is f32. Opt into native f64 processing only when the algorithm benefits:

```rust
impl AudioKernel for MasteringKernel {
    const SUPPORTS_F64: bool = true;

    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        // f32 host path
        ProcessResult::Continue
    }

    fn process_f64(&mut self, context: &mut ProcessContext64<'_>) -> ProcessResult {
        // native f64 host path
        ProcessResult::Continue
    }
}
```

Without the opt-in, Vesty uses preallocated f64↔f32 scratch conversion for hosts requesting 64-bit buffers.

## Testing

Test the kernel independently of a DAW, then run the adapter suites for automation, buses, events, silence flags, and capacity limits. A host smoke test is still required before release.

