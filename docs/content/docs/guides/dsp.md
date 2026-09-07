---
title: DSP kernels
description: Prepare, process, and test native DSP within realtime constraints.
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

Dense host blocks are processed in batches of at most 512 events without dropping the remaining events. A kernel may receive multiple calls for one host block; sample offsets are relative to each call's audio range. If more events remain at the same sample, Vesty delivers a zero-frame context before rendering that sample. Consume its events even when `audio().frames() == 0`; only skip audio generation. This applies to both `process` and `process_f64`. The MIDI synth example demonstrates this contract.

Batch storage is preallocated. Oversized blocks require additional scans of the host event lists, so CPU work grows with event volume even though memory stays bounded. This removes the event-count cutoff, not the audio callback's execution deadline.

For note identity, MIDI mappings, payload limits, and a rendering timeline, see [MIDI and event timing](/docs/guides/midi).

## Double precision

The default path uses `f32`. Opt into native `f64` processing only when the algorithm benefits:

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
