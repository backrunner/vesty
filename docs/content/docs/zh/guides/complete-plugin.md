---
title: 构建一个完整插件
description: 从脚手架开始，构建并验证一个可在 DAW 中测试的立体声 VST3 效果器。
order: 1
---

本教程将构建一个名为 **Signal Gain** 的立体声 VST3 效果器。它支持采样级精确的增益自动化与旁路，包含可独立测试的 DSP 内核和可选的 WebView 编辑器，并提供可重复执行的打包验证流程。算法本身有意保持简单，但开发过程会经过均衡器、压缩器或合成器同样需要面对的工程边界。

完成后，你会得到一个可由宿主加载的 `.vst3` 插件包。通过本地验证只说明它适合进入开发测试；只有补齐[发布证据](/docs/zh/tooling/release-evidence)矩阵后，才能把它视为可公开发布的版本。

## 1. 准备工具链

你需要 Rust 1.95 或更高版本。只有开发 Web UI 时才需要 Node.js 24 或更高版本。启用 `wry` 后端还需要当前平台的 WebView 开发库；最后的冒烟测试则需要一个 VST3 宿主。

按照[快速开始](/docs/zh/quick-start)安装预编译 CLI，然后验证可执行文件和本机环境：

```bash
vesty --version
vesty doctor
```

CLI 内置了维护中的工程模板，并把生成的 Rust 与 npm 依赖固定到匹配的 Vesty release。只有参与 Vesty 框架开发或测试尚未发布的版本时，才需要源码 checkout。

## 2. 创建效果器工程

先查看仍在维护的模板，再创建一个不带 UI 的增益插件工程：

```bash
vesty templates
vesty new signal-gain --template gain
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

`Cargo.toml` 需要同时生成供测试使用的普通 Rust 库，以及供宿主加载的动态库：

```toml title="Cargo.toml"
[lib]
crate-type = ["rlib", "cdylib"]

[dependencies]
vesty = "=0.1.0"
```

生成的依赖版本与 `vesty --version` 保持一致。除非明确同时升级 CLI 和框架，否则应保留这个精确版本。

## 3. 确立不可随意变更的身份

宿主会把 VST3 Class ID 和参数 ID 写入用户工程。可以把它们理解为数据库主键：显示名称可以调整，ID 一旦发布就不应改变。

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

上面的 UUID 文本与 Rust 中的 16 字节数组表示同一个 Class ID。每个插件类都必须使用唯一值；不要把示例 ID 带入正式产品。

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

打包过程会验证这两处表示是否一致。如果不一致，构建应直接失败，而不是等到运行时再猜测哪一份元数据才正确。

## 4. 声明宿主可见参数

使用强类型参数表示增益和旁路。Rust 字段名只是实现细节；字符串 ID 才是需要长期保持稳定的宿主契约。

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

宿主使用 `0.0..1.0` 范围内的归一化值传输参数。`FloatParam` 负责在归一化值和 `-60..12 dB` 之间转换；下文的音频内核会执行相同的线性映射，因为音频回调只能读取实时安全参数句柄对应的归一化值。

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

生成插件包需要携带的参数清单，并将它提交到 Git：

```bash
vesty param-manifest --specs params.specs.json --out vesty-parameters.json
```

每次有意修改参数规格后都要重新生成。CI 中应添加 `--check`，让漂移直接失败，而不是静默重写文件。

## 5. 分离宿主状态与实时状态

`Plugin` 对象持有面向宿主的参数；音频内核只持有音频回调需要的状态。在创建内核时，一次性把字符串 ID 解析成参数句柄：

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

不要把 `String`、UI 对象、文件句柄、互斥锁或可能动态增长的集合放进实时处理路径。延迟线、查找表和临时存储空间应在 `create_kernel()` 或 `prepare()` 中预先分配，之后才进入 `process()`。

## 6. 连续处理宿主自动化

宿主可能在一个音频块内放入多个增益自动化点。如果整块只读取一次参数，自动化精度就会退化到块级。`ParamAutomationSegments` 会把已排序的事件转换为一组有界区间：

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

这个音频回调只执行有界运算，并写入从宿主借用的缓冲区。它不会分配内存、加锁、输出普通日志、解析 JSON、访问磁盘或调用 WebView。正式的增益插件还可以平滑突变以避免爆音；平滑器状态应保存在 `SignalKernel` 中，并以不分配内存的方式更新。

当输入和输出声道数不同时，循环只处理相互匹配的声道。对于更复杂的总线布局，必须显式清空所有没有写入的输出声道。

## 7. 不依赖 DAW 测试音频内核

在插件工程中保留一个小型、结果确定的 DSP 测试。默认的 `0 dB` 设置应当逐采样复制两个声道：

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

继续补充最小和最大增益、旁路、零帧、输入输出声道数不一致、采样点 `0` 的自动化事件，以及位于最后一帧的自动化事件。对于有状态 DSP，还要覆盖 `prepare()`、`reset()`、暂停与恢复、采样率变化和配置允许的最大块大小。

运行本地质量检查：

```bash
cargo fmt --all --check
cargo test
cargo clippy --all-targets -- -D warnings
```

VST3 适配层会在宿主调用的处理路径外启用 Vesty 的内存分配检测器。它可以捕获受检测器监控的分配器调用；对于锁、系统调用和耗时未知的第三方函数，仍然必须进行代码审查。

## 8. DSP 稳定后添加 Web UI

你也可以一开始就选择 UI 模板：

```bash
cd ..
vesty new signal-gain-ui --template web-ui-param-demo
cd signal-gain-ui
```

也可以给现有插件添加下面的 UI 描述，并创建 `ui/` 应用：

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

UI 必须使用 `ready.paramValues` 初始化控件，并用 `beginParamEdit`、`performParamEdit` 和 `endParamEdit` 包裹每一次用户编辑手势。只有 `param.changed` 表示宿主已经确认的状态。完整流程见 [Web UI](/docs/zh/guides/web-ui)。

开发时，在两个终端中分别运行 UI 开发服务器和插件监听进程：

```bash
npm install --prefix ui
npm run dev --prefix ui

vesty dev --config vesty.toml
```

Web UI 路径中的任何工作都不能由 `process()` 调用；JSON 与 WebView 操作必须留在控制器和 UI 一侧。

## 9. 构建并打包插件

先构建发布模式的动态库：

```bash
cargo build --release
```

根据目标平台传入对应的二进制文件：

```bash
# macOS
vesty package \
  --config vesty.toml \
  --platform macos \
  --binary target/release/libsignal_gain.dylib \
  --out target/vesty

# Windows x64: target/release/signal_gain.dll
# Linux x64:   target/release/libsignal_gain.so
```

如果工程包含 UI，打包过程会运行配置中的构建命令，并且只复制生成的 `dist` 资源。不要把 `node_modules` 或开发服务器 URL 作为发布内容放进插件包。

## 10. 打开 DAW 前先验证

首先运行严格静态验证：

```bash
vesty validate \
  target/vesty/SignalGain.vst3 \
  --static-only \
  --strict \
  --format json \
  --report target/vesty/signal-gain.static.json
```

这一步会检查插件包结构、导出符号、元数据一致性、参数清单和 UI 资源哈希。之后再运行 Steinberg validator，并保留 JSON 报告与原始日志：

```bash
vesty validate \
  target/vesty/SignalGain.vst3 \
  --strict \
  --validator /absolute/path/to/validator \
  --report target/vesty/signal-gain.validator.json \
  --validator-log target/vesty/signal-gain.validator.log
```

## 11. 完成宿主冒烟测试

把插件包安装到当前平台的 VST3 开发目录，重新扫描插件，并在至少一个受支持的 DAW 中逐项验证：

1. 插件可以被扫描并实例化，宿主没有报告错误。
2. `0 dB` 时立体声音频能够正确通过，增益和旁路输出符合预期。
3. 录制的自动化数据跟随时间线，并同步更新编辑器。
4. 保存、关闭并重新打开 DAW 后，参数值正确恢复。
5. 编辑器可以打开、缩放和重新加载，并能反映宿主侧的修改。
6. 离线渲染与实时渲染在算法允许的误差范围内一致。
7. 反复激活、停用，以及修改采样率和缓冲区大小后，插件仍然稳定。

在声明支持的每个平台上重复运行 Steinberg validator，并完成宿主、WebView 和签名检查，再把所有报告保存为发布证据。Vesty 可以组织和校验这些证据，但不能替代真实宿主和真实平台测试。

## 下一步

- 阅读 [DSP 内核](/docs/zh/guides/dsp)，继续了解预分配状态、事件、电平数据、侧链与 `f64` 处理。
- 阅读[状态与生命周期](/docs/zh/guides/state-and-lifecycle)，实现自定义非参数状态和迁移。
- 在编写乐器插件前研究 `examples/midi-synth`。
- 使用 [AI 辅助开发](/docs/zh/tooling/ai-development)安装配套 Skill，让 AI 助手同样遵守实时与发布约束。
