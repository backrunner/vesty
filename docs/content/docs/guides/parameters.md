---
title: Parameters
description: Declare typed parameters, stable IDs, metadata, and host automation.
order: 1
---

## Declare a collection

`#[derive(Params)]` generates indexed handle access without searching strings during processing.

```rust
#[derive(Params)]
struct FilterParams {
    cutoff: FloatParam,
    resonance: FloatParam,
    bypass: BoolParam,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            cutoff: FloatParam::new("cutoff", "Cutoff", 20.0, 20_000.0, 1_000.0)
                .with_unit("Hz"),
            resonance: FloatParam::new("resonance", "Resonance", 0.1, 12.0, 0.7),
            bypass: BoolParam::new("bypass", "Bypass", false).with_bypass(true),
        }
    }
}
```

The string ID is a persistent project contract. Do not rename it after release unless you also provide a migration strategy.

## Resolve handles once

```rust
struct FilterKernel {
    cutoff: ParamHandle,
    resonance: ParamHandle,
}

fn create_kernel(&self, _init: KernelInit) -> FilterKernel {
    FilterKernel {
        cutoff: self.params.resolve_or_invalid("cutoff"),
        resonance: self.params.resolve_or_invalid("resonance"),
    }
}
```

An invalid handle safely returns `None`; it does not panic through host initialization.

## Sample-accurate automation

Use the parameter events carried by `ProcessContext` when the algorithm needs exact offsets. The adapter preserves the previous block value until the first event, orders points by sample offset, and commits the final value after processing.

For a coefficient that can change less frequently, combine event handling with a preallocated smoother instead of recalculating expensive state every sample.

## Generate the manifest

```bash
cargo run -p vesty-cli -- param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json

cargo run -p vesty-cli -- param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json \
  --check
```

Commit the manifest. Packaging copies it into the VST3 bundle, and strict validation checks that it still matches the parameter specifications.
