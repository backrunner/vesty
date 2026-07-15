---
title: 快速开始
description: 使用 Rust 构建并验证你的第一个 Vesty 音频效果器。
order: 1
---

Vesty 使用 Rust 运行 DSP，同时允许编辑器采用常见的 Web 框架。本指南将带你创建一个规模最小但功能完整的增益效果器，并指出后续开发路径。

## 环境要求

- Rust 1.95 或更新版本。
- 用于手动测试的 VST3 宿主。
- 只有使用 Web UI 时才需要 Node.js 24 或更新版本。
- 编译 `wry` 后端时，需要当前平台对应的 WebView 开发库。

## 添加 Vesty

创建一个同时输出 `rlib` 和 `cdylib` 的 Rust 库：

```toml title="Cargo.toml"
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["rlib", "cdylib"]

[dependencies]
vesty = "0.1.0"
```

Vesty 目前仍处于 alpha 阶段。在本仓库中使用尚未发布的 API 时，请采用路径依赖：

```toml
vesty = { path = "../../crates/vesty" }
```

## 实现增益效果器

```rust title="src/lib.rs"
use vesty::prelude::*;

#[derive(Params)]
struct GainParams {
    gain: FloatParam,
}

impl Default for GainParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0)
                .with_unit("dB"),
        }
    }
}

#[derive(Default)]
struct GainPlugin {
    params: GainParams,
}

struct GainKernel {
    gain: ParamHandle,
}

impl Plugin for GainPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "My Gain",
        vendor: "My Company",
        url: "https://example.invalid",
        email: "",
        version: "0.1.0",
        class_id: *b"MYGAINPLUGIN0001",
        kind: PluginKind::AudioEffect,
    };

    type Params = GainParams;
    type Kernel = GainKernel;

    fn params(&self) -> &Self::Params { &self.params }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        GainKernel { gain: self.params.resolve_or_invalid("gain") }
    }
}

impl AudioKernel for GainKernel {
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        let normalized = context.param_normalized(self.gain).unwrap_or(0.833_333);
        let gain_db = -60.0 + normalized * 72.0;
        let gain = 10.0_f32.powf(gain_db as f32 / 20.0);
        let channels = context.audio().input_channels()
            .min(context.audio().output_channels());
        let audio = context.audio_mut();

        for channel in 0..channels {
            audio.copy_input_to_output(channel, gain);
        }

        ProcessResult::Continue
    }
}

vesty::export_vst3!(GainPlugin);
```

## 验证工作区

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

仓库中的 `examples/gain`、`examples/midi-synth` 和 `examples/web-ui-param-demo` 都提供了可运行的参考实现。

## 下一步

- 通过[架构](/docs/zh/concepts/architecture)理解原生层与 WebView 的关系。
- 在实现 DSP 前阅读[实时安全](/docs/zh/concepts/realtime-safety)规则。
- 使用[参数指南](/docs/zh/guides/parameters)添加宿主可见参数。
- 使用[Web UI 指南](/docs/zh/guides/web-ui)构建编辑器。
