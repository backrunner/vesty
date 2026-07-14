---
title: 构建一个完整插件
description: 从脚手架开始，完成一个可在 DAW 中测试的立体声 VST3 效果器。
order: 1
---

本教程会构建 **Signal Gain**：一个带 sample-accurate 增益自动化、bypass、可独立测试的 DSP kernel、可选 WebView 编辑器和可重复打包验证流程的立体声 VST3 效果器。它的算法有意保持简单，但会经过均衡器、压缩器或合成器同样需要面对的工程边界。

完成后，你会得到一个可以加载的 `.vst3` bundle。通过本地验证意味着它适合进入开发测试；只有补齐[发布证据](/docs/zh/tooling/release-evidence)矩阵后，才能把它当作可公开发布的构建。

## 1. 准备工具链

你需要 Rust 1.95 或更高版本。只有开发 Web UI 时才需要 Node.js 24 或更高版本。启用 `wry` backend 时还需要平台 WebView 开发库；最后的 smoke test 则需要一个 VST3 宿主。

Vesty 仍处于 alpha 阶段，当前最可靠的方式是从 Vesty checkout 开发：

```bash
git clone https://github.com/orchiliao/vesty.git
cd vesty
cargo build -p vesty-cli
cargo run -p vesty-cli -- doctor
cd ..
```

后文中的 `vesty` 命令都可以从相邻的 Vesty workspace 通过 `cargo run --manifest-path ../vesty/Cargo.toml -p vesty-cli -- <command>` 执行。如果已经安装 CLI，直接使用 `vesty <command>` 即可。

## 2. 创建效果器工程

先查看维护中的模板，再创建 headless gain 工程：

```bash
./vesty/target/debug/vesty templates
./vesty/target/debug/vesty new signal-gain \
  --template gain \
  --vesty-path "$PWD/vesty/crates/vesty"
cd signal-gain
```

生成的工程包含四个关键文件：

```text
signal-gain/
├── Cargo.toml
├── params.specs.json
├── src/lib.rs
└── vesty.toml
```

`Cargo.toml` 需要同时生成供测试使用的普通 Rust library，以及供宿主加载的动态库：

```toml title="Cargo.toml"
[lib]
crate-type = ["rlib", "cdylib"]

[dependencies]
vesty = { path = "../vesty/crates/vesty" }
```

如果你的目录结构不同，请相应调整路径。当 Vesty 发布了适合产品使用的版本后，应固定一个明确的兼容版本，不要让正式构建跟随移动中的 Git branch。

## 3. 确立不可随意变更的身份

宿主会把 VST3 class ID 和参数 ID 写入用户工程。可以把它们理解为数据库主键：显示名称可以变化，ID 不应该变化。

在 `vesty.toml` 中填写产品身份：

```toml title="vesty.toml"
[plugin]
name = "Signal Gain"
vendor = "Signal Works"
version = "0.1.0"
kind = "Fx"
class_id = "5349474e-414c-4741-494e-303030303031"

[package]
bundle_id = "audio.signalworks.signal-gain"
category = "Fx"
parameter_manifest = "vesty-parameters.json"
```

上面的 UUID 文本与 Rust 中的 16 bytes 表示同一个 class ID。每个 plugin class 都必须使用唯一值；不要把示例 ID 带入发布产品。

在 `src/lib.rs` 中使用一致的元数据：

```rust
const INFO: PluginInfo = PluginInfo {
    name: "Signal Gain",
    vendor: "Signal Works",
    url: "https://example.com/signal-gain",
    email: "support@example.com",
    version: "0.1.0",
    class_id: *b"SIGNALGAIN000001",
    kind: PluginKind::AudioEffect,
};
```

打包过程会验证两处表示是否一致。若不一致，应该直接失败，而不是在 runtime 猜测哪一份元数据才是正确的。

## 4. 声明宿主可见参数

用类型化参数表示增益和 bypass。Rust 字段名只是实现细节；字符串 ID 才是持久的宿主契约。

```rust title="src/lib.rs"
use vesty::prelude::*;

#[derive(Params)]
pub struct SignalParams {
    pub gain: FloatParam,
    pub bypass: BoolParam,
}

impl Default for SignalParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0)
                .with_unit("dB"),
            bypass: BoolParam::bypass("bypass", "Bypass", false),
        }
    }
}
```

宿主在 `0.0..1.0` 的 normalized 范围内传输参数。`FloatParam` 负责 normalized 值与 `-60..12 dB` 之间的转换；下文的 kernel 会执行相同的线性映射，因为 audio callback 只读取 realtime-safe handle 对应的 normalized 值。

让 `params.specs.json` 与声明保持一致：

```json title="params.specs.json"
{
  "version": 1,
  "parameters": [
    {
      "id": "gain",
      "name": "Gain",
      "kind": { "float": { "min": -60.0, "max": 12.0 } },
      "defaultNormalized": 0.8333333333333334,
      "unit": "dB",
      "stepCount": null,
      "flags": {
        "automatable": true,
        "bypass": false,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    },
    {
      "id": "bypass",
      "name": "Bypass",
      "kind": "bool",
      "defaultNormalized": 0.0,
      "unit": null,
      "stepCount": 1,
      "flags": {
        "automatable": true,
        "bypass": true,
        "readOnly": false,
        "programChange": false
      },
      "midiMappings": []
    }
  ]
}
```

生成 bundle 中要携带的 manifest，并把它提交到 Git：

```bash
cargo run --manifest-path ../vesty/Cargo.toml -p vesty-cli -- \
  param-manifest --specs params.specs.json --out vesty-parameters.json
```

每次有意修改参数规格后都要重新生成。CI 中应添加 `--check`，让漂移直接失败，而不是静默重写文件。

## 5. 分离宿主状态与实时状态

Plugin 对象持有面向宿主的参数；kernel 只持有 audio callback 所需的状态。在创建 kernel 时一次性解析字符串 ID：

```rust
#[derive(Default)]
pub struct SignalGain {
    params: SignalParams,
}

pub struct SignalKernel {
    gain: ParamHandle,
    bypass: ParamHandle,
}

impl Plugin for SignalGain {
    const INFO: PluginInfo = PluginInfo {
        name: "Signal Gain",
        vendor: "Signal Works",
        url: "https://example.com/signal-gain",
        email: "support@example.com",
        version: "0.1.0",
        class_id: *b"SIGNALGAIN000001",
        kind: PluginKind::AudioEffect,
    };

    type Params = SignalParams;
    type Kernel = SignalKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        SignalKernel {
            gain: self.params.resolve_or_invalid("gain"),
            bypass: self.params.resolve_or_invalid("bypass"),
        }
    }
}
```

不要把 `String`、UI 对象、文件句柄、mutex 或可能动态增长的 collection 放进 hot path。Delay line、lookup table 和 scratch storage 应在 `create_kernel()` 或 `prepare()` 中预先分配，然后才进入 `process()`。

## 6. 连续处理宿主自动化

宿主可能在一个 audio block 内放入多个增益点。每个 block 只读取一次参数会把 automation 降为 block-accurate。`ParamAutomationSegments` 会把已经排序的事件转换为有界区间：

```rust
impl AudioKernel for SignalKernel {
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        let bypass = context.param_normalized(self.bypass).unwrap_or(0.0) >= 0.5;
        let initial_gain = context.param_normalized(self.gain).unwrap_or(0.833_333);
        let frames = context.audio().frames().min(u32::MAX as usize) as u32;
        let channels = context
            .audio()
            .input_channels()
            .min(context.audio().output_channels());
        let (audio, events) = context.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.gain, initial_gain, frames) {
            let gain_db = -60.0 + segment.normalized * 72.0;
            let amplitude = if bypass {
                1.0
            } else {
                10.0_f32.powf(gain_db as f32 / 20.0)
            };

            for channel in 0..channels {
                audio.copy_input_to_output_range(
                    channel,
                    segment.start_sample as usize,
                    segment.end_sample as usize,
                    amplitude,
                );
            }
        }

        ProcessResult::Continue
    }
}

vesty::export_vst3!(SignalGain);
```

这个 callback 只执行有界运算并写入借用的宿主 buffer。它不会分配、加锁、输出普通日志、解析 JSON、访问磁盘或调用 WebView。正式的增益插件还可以对突变做 smoothing 以避免 click；smoother 状态应保存在 `SignalKernel` 中，并以零分配方式更新。

当输入和输出 channel 数不同，循环只处理相互匹配的 channel。更复杂的 bus layout 必须显式清空所有没有写入的输出。

## 7. 不依赖 DAW 测试 kernel

在插件旁保留一个小而确定的 DSP 测试。默认 `0 dB` 应当逐样本复制两个 channel：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gain_is_stereo_passthrough() {
        let plugin = SignalGain::default();
        let mut kernel = plugin.create_kernel(KernelInit {
            sample_rate: 48_000.0,
            max_block_size: 4,
        });
        let left = [1.0_f32, -0.5, 0.25, 0.0];
        let right = [-0.25_f32, 0.5, -1.0, 0.75];
        let inputs: [&[f32]; 2] = [&left, &right];
        let mut left_out = [0.0_f32; 4];
        let mut right_out = [0.0_f32; 4];

        {
            let mut outputs: [&mut [f32]; 2] = [&mut left_out, &mut right_out];
            let audio = AudioBuffers::new(&inputs, &mut outputs);
            let mut context = ProcessContext::new(
                audio,
                plugin.params(),
                &[],
                Transport::default(),
            );
            assert_eq!(kernel.process(&mut context), ProcessResult::Continue);
        }

        assert_eq!(left_out, left);
        assert_eq!(right_out, right);
    }
}
```

继续补充最小和最大增益、bypass、零帧、输入输出 channel 数不一致、sample `0` automation，以及最后一帧 automation。对于有状态 DSP，还要覆盖 `prepare()`、`reset()`、suspend/resume、采样率变化和配置允许的最大 block size。

运行本地质量门：

```bash
cargo fmt --all --check
cargo test
cargo clippy --all-targets -- -D warnings
```

VST3 adapter 会在宿主调用的处理路径外层进入 Vesty 的 no-allocation guard。它可以捕获受 guard 监测的 allocator 调用；对于锁、syscall 和耗时未知的第三方函数，仍然必须进行代码审查。

## 8. DSP 稳定后添加 Web UI

你也可以一开始就选择 UI 模板：

```bash
cd ..
./vesty/target/debug/vesty new signal-gain-ui \
  --template web-ui-param-demo \
  --vesty-path "$PWD/vesty/crates/vesty" \
  --plugin-ui-path "$PWD/vesty/packages/plugin-ui"
cd signal-gain-ui
```

或者给现有插件添加下面的 descriptor，并创建 `ui/` 应用：

```rust
fn ui(&self) -> Option<UiDescriptor> {
    Some(
        UiDescriptor::web_assets("ui")
            .with_dev_url("http://localhost:5173")
            .with_size(900, 560)
            .with_min_size(640, 420)
            .with_resizable(true),
    )
}
```

UI 必须使用 `ready.paramValues` 初始化控制项，然后用 `beginParamEdit`、`performParamEdit` 和 `endParamEdit` 包住一次用户 gesture。把 `param.changed` 当作宿主确认后的事实来源。完整流程见 [Web UI](/docs/zh/guides/web-ui)。

开发时，在两个 terminal 中分别运行 UI server 和 plugin watcher：

```bash
npm install --prefix ui
npm run dev --prefix ui

cargo run --manifest-path ../vesty/Cargo.toml -p vesty-cli -- \
  dev --config vesty.toml
```

Web UI 路径中的任何工作都不能从 `process()` 调用；JSON 和 WebView 操作必须留在 controller/UI 一侧。

## 9. 构建并打包 bundle

先构建 release 动态库：

```bash
cargo build --release
```

根据平台传入对应的 binary：

```bash
# macOS
cargo run --manifest-path ../vesty/Cargo.toml -p vesty-cli -- package \
  --config vesty.toml \
  --platform macos \
  --binary target/release/libsignal_gain.dylib \
  --out target/vesty

# Windows x64: target/release/signal_gain.dll
# Linux x64:   target/release/libsignal_gain.so
```

如果工程带 UI，打包过程会运行配置中的 build command，并且只复制生成的 `dist` 资源。不要把 `node_modules` 或开发服务器 URL 当作 release 内容打进 bundle。

## 10. 打开 DAW 前先验证

首先运行严格静态验证：

```bash
cargo run --manifest-path ../vesty/Cargo.toml -p vesty-cli -- validate \
  target/vesty/SignalGain.vst3 \
  --static-only \
  --strict \
  --format json \
  --report target/vesty/signal-gain.static.json
```

它会检查 bundle 结构、导出符号、元数据一致性、参数 manifest 和 UI asset hash。之后再运行 Steinberg validator，并保留 JSON report 与原始日志：

```bash
cargo run --manifest-path ../vesty/Cargo.toml -p vesty-cli -- validate \
  target/vesty/SignalGain.vst3 \
  --strict \
  --validator /absolute/path/to/validator \
  --report target/vesty/signal-gain.validator.json \
  --validator-log target/vesty/signal-gain.validator.log
```

## 11. 完成宿主 smoke test

把 bundle 安装到平台的 VST3 开发目录，重新扫描插件，并在至少一个受支持 DAW 中逐项验证：

1. 插件可以被扫描并实例化，宿主没有报告错误。
2. `0 dB` 时立体声音频正确通过，gain 和 bypass 输出符合预期。
3. 录制的 automation 跟随时间线，并同步更新 editor。
4. 保存、关闭并重新打开 DAW 后，参数值正确恢复。
5. Editor 可以打开、缩放和 reload，并能反映宿主侧修改。
6. Offline render 与 realtime render 在算法预期误差范围内一致。
7. 反复 activate/deactivate、修改采样率和 buffer size 后仍然稳定。

在你声明支持的每个平台上重复 validator、宿主、WebView 和签名检查，并把报告作为 release artifact 保存。Vesty 可以组织和校验这些证据，但不能代替真实宿主和真实平台测试。

## 下一步

- 阅读 [DSP kernel](/docs/zh/guides/dsp)，继续实现 prepared state、事件、meter、sidechain 与 f64 processing。
- 阅读[状态与生命周期](/docs/zh/guides/state-and-lifecycle)，实现自定义非参数状态和迁移。
- 在编写 instrument 前研究 `examples/midi-synth`。
- 使用 [AI 辅助开发](/docs/zh/tooling/ai-development)安装配套 skill，让 AI agent 同样遵守实时和发布约束。
