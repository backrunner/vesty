---
title: Packaging and validation
description: Build a platform VST3 bundle and inspect its static contracts.
order: 2
---

## Configure the package

```toml title="vesty.toml"
[plugin]
name = "My Gain"
vendor = "My Company"
version = "0.1.0"
kind = "Fx"
class_id = "4d594741-494e-3030-3030-303030303031"

[package]
bundle_id = "com.example.my-gain"
category = "Fx"
parameter_manifest = "vesty-parameters.json"
```

Class IDs, bundle IDs, and parameter IDs are persistent compatibility contracts.

## Build and package on macOS

```bash
cargo build -p my-plugin --release

vesty package \
  --config vesty.toml \
  --platform macos \
  --binary target/release/libmy_plugin.dylib \
  --out target/package
```

Platform values also include `windows-x64` and `linux-x64`; use the matching `.dll` or `.so` binary.

## Run strict static validation

```bash
vesty validate target/package/MyGain.vst3 \
  --static-only \
  --strict \
  --format json \
  --report target/package/MyGain.validate.json
```

Static validation checks bundle layout, metadata agreement, binary exports, parameter manifests, and Web UI asset hashes. Strict mode fails if the platform's export inspection tool cannot provide positive evidence.

## Run the Steinberg validator

```bash
vesty validate target/package/MyGain.vst3 \
  --strict \
  --validator /path/to/validator \
  --report target/validator/MyGain.validate.json \
  --validator-log target/validator/MyGain.log
```

Keep validator and static reports for every supported release platform.

