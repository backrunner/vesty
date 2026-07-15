---
title: 打包与验证
description: 为目标平台构建 VST3 插件包，并检查其中的静态契约。
order: 2
---

## 配置插件包

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

Class ID、Bundle ID 和参数 ID 都是需要长期保持稳定的兼容性契约。

## 在 macOS 构建和打包

```bash
cargo build -p my-plugin --release

vesty package \
  --config vesty.toml \
  --platform macos \
  --binary target/release/libmy_plugin.dylib \
  --out target/package
```

可用的平台值还包括 `windows-x64` 和 `linux-x64`；请分别传入对应的 `.dll` 或 `.so` 文件。

## 运行严格静态验证

```bash
vesty validate target/package/MyGain.vst3 \
  --static-only \
  --strict \
  --format json \
  --report target/package/MyGain.validate.json
```

静态验证会检查插件包结构、元数据一致性、二进制导出符号、参数清单和 Web UI 资源哈希。在严格模式下，平台导出检查工具必须提供明确的通过证据，否则验证会失败。

## 运行 Steinberg validator

```bash
vesty validate target/package/MyGain.vst3 \
  --strict \
  --validator /path/to/validator \
  --report target/validator/MyGain.validate.json \
  --validator-log target/validator/MyGain.log
```

应为每个受支持的发布平台保存 validator 报告和静态验证报告。
