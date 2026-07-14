---
title: Realtime safety
description: Keep process() deterministic, bounded, and non-blocking.
order: 2
---

Realtime safety is a design constraint, not a late optimization. The host may call `process()` under a strict deadline hundreds of times per second.

## Never do this in process()

- Allocate or grow a collection.
- Lock a mutex or wait on a condition variable.
- Perform file, network, JSON, or WebView work.
- Format strings or emit ordinary logs.
- Call an API with unknown blocking behavior.

## Use bounded primitives

Vesty provides fixed event lists, preallocated sample conversion scratch, atomic parameter values, SPSC queues, realtime meter producers, and an allocation guard used by tests.

Resolve parameter IDs before processing:

```rust
fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
    MyKernel {
        cutoff: self.params.resolve_or_invalid("cutoff"),
    }
}
```

Read the handle in constant time:

```rust
let cutoff = context.param_normalized(self.cutoff).unwrap_or(0.5);
```

Do not call `specs()` or search strings from the audio callback. `ParamCollection::get_normalized_by_handle()` and `set_normalized_by_handle()` are the realtime path.

## Communicate without waiting

For UI meters, publish a small numeric frame to the realtime producer. The UI thread drains it later. For diagnostics, use fixed realtime log records instead of formatting a message in the callback.

## Failure behavior

If the kernel panics or a requested block exceeds preallocated capacity, Vesty clears outputs and marks silence instead of processing partial or unbounded work. This protects the host, but it is not a substitute for testing.

## Tests to keep

- Run processing under `NoAllocGuard`.
- Cover the configured maximum block size and one block above it.
- Test automation before, at, and after the first point in a block.
- Exercise suspend, resume, reset, and sample-rate changes.
- Verify faulted processing produces silence.

