---
title: 打包与验证
description: 构建平台 VST3 bundle，并检查静态契约。
order: 2
---

## 配置 package

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

Class ID、bundle ID 和参数 ID 都是持久兼容性契约。

## 在 macOS 构建和打包

```bash
cargo build -p my-plugin --release

vesty package \
  --config vesty.toml \
  --platform macos \
  --binary target/release/libmy_plugin.dylib \
  --out target/package
```

平台值还包括 `windows-x64` 和 `linux-x64`，分别使用匹配的 `.dll` 或 `.so`。

## 运行 strict static validation

```bash
vesty validate target/package/MyGain.vst3 \
  --static-only \
  --strict \
  --format json \
  --report target/package/MyGain.validate.json
```

静态验证检查 bundle 结构、元数据一致性、binary export、参数 manifest 和 Web UI asset hash。Strict 模式要求平台导出检查工具提供正向证据。

## 运行 Steinberg validator

```bash
vesty validate target/package/MyGain.vst3 \
  --strict \
  --validator /path/to/validator \
  --report target/validator/MyGain.validate.json \
  --validator-log target/validator/MyGain.log
```

每个支持的发布平台都要保存 validator 和 static report。

