# Realtime safety reference

Read this reference before writing, changing, or reviewing `AudioKernel::process()` or `process_f64()`.

## Hard rules

Inside the audio callback, do not:

- allocate, free, resize, clone owned data, or grow a collection;
- acquire a mutex/RwLock, wait, sleep, block, or join;
- access files, networks, environment variables, subprocesses, or WebViews;
- parse or serialize JSON/TOML, format strings, or emit ordinary logs;
- call a third-party API unless its realtime behavior is known and bounded;
- search parameters by string or rebuild parameter metadata;
- panic on malformed host input.

Allowed work is bounded arithmetic, reads/writes over borrowed buffers, fixed-capacity event/state updates, atomics with appropriate ordering, and non-blocking bounded queue pushes that may fail cleanly.

## Parameter pattern

Resolve handles outside processing:

```rust
fn create_kernel(&self, _init: KernelInit) -> MyKernel {
    MyKernel {
        cutoff: self.params.resolve_or_invalid("cutoff"),
    }
}
```

Read normalized values in constant time:

```rust
let cutoff = context.param_normalized(self.cutoff).unwrap_or(0.5);
```

When points inside the block matter, iterate `ParamAutomationSegments::new(events, handle, initial, frames)`. Preserve the previous value before the first event and apply the final value after its sample offset. Use a preallocated smoother for coefficients that must transition continuously.

## Buffers and events

- Clamp work to `context.audio().frames()` and declared/prepared capacity.
- Handle zero frames and missing channels.
- Clear every output that the algorithm does not write.
- Consume note, parameter, SysEx, and expression events using their sample offsets.
- Keep SysEx and voice state within Vesty's fixed limits and handle truncation explicitly.
- Return `ProcessResult::Silence` only when every output sample is known to be silent.

## Preparation and failure

Allocate delay lines, FFT plans, tables, oversampling state, scratch buffers, and voices during construction or `prepare()`. Reuse them in every callback. If host input exceeds prepared capacity, fail boundedly and clear output rather than resizing.

Vesty wraps adapter processing in `vesty_rt::NoAllocGuard`. Tests should prove the guard is active where relevant, but it does not detect every realtime hazard. Review locks, syscalls, destructor behavior, and third-party code separately.

## Review checklist

- No owned temporary collection is constructed per block.
- No hidden allocation from `format!`, `to_string`, iterator collection, error construction, or logging.
- All loops have a host-buffer or fixed-capacity upper bound.
- Parameter handles are resolved before processing.
- Buffer aliasing and input/output count differences are handled.
- Automation at sample 0, within the block, and on the last frame is tested.
- Reset, sample-rate changes, max block size, silence, and fault behavior are tested.
