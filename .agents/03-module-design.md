# 03. 模块设计

## Workspace 结构

建议第一版 workspace:

```text
crates/
  vesty/
  vesty-core/
  vesty-params/
  vesty-rt/
  vesty-vst3/
  vesty-vst3-sys/
  vesty-ui/
  vesty-ui-wry/
  vesty-ipc/
  vesty-build/
  vesty-cli/
  vesty-macros/
examples/
  gain/
  midi-synth/
  web-ui/
xtask/
```

## crate 职责

### `vesty`

用户入口 facade crate:

- re-export 核心 trait、参数类型、宏。
- feature 开关: `vst3`、`webview-wry`、`serde-state`。
- 提供 `vesty::export_vst3!(MyPlugin)`。

### `vesty-core`

插件核心抽象:

- `Plugin` trait: metadata、参数、bus layout、创建 kernel 和 UI descriptor。
- fixed `latency_samples()` / `tail_samples()` hook。
- `AudioKernel` trait: 实时 DSP。
- `ProcessContext`、`AudioBuffer`、`Event`、`ParamAutomationPoint`、`Transport`、`ProcessMode`。
- `ProcessContext::param_automation(handle)` / `latest_param_automation(handle)` 提供按 `ParamHandle` 过滤的当前 block automation view，保持 realtime path 无分配。
- `ProcessContext::process_mode()` 暴露 host 当前 block 的 `Realtime` / `Prefetch` / `Offline` 模式；默认构造为 `Realtime`，VST3 adapter 会从 `ProcessData.processMode` 映射。
- 不依赖 wry，不依赖 VST3 unsafe bindings。

### `vesty-params`

参数系统:

- `ParamId`、`ParamSpec`、`ParamValue`。
- `FloatParam`、`BoolParam`、`ChoiceParam` typed 参数。
- normalized/value conversion；choice 参数会 snap 到最近的离散 index，并在 bridge/VST3 host 文本格式化中使用 label。
- smoothing。
- stable ID validation。
- `ParamRegistry` 和 `ParamSnapshot`。

### `vesty-rt`

实时安全原语:

- SPSC 队列 wrapper。
- triple buffer wrapper。
- fixed-capacity event list。
- allocation guard 测试工具。
- non-blocking logger。

### `vesty-vst3`

VST3 适配层:

- VST3 factory exports。
- Processor/controller COM wrappers。
- Bus、parameter、event、state、latency、IPlugView 适配。
- unsafe 代码边界。
- host quirk table。

### `vesty-vst3-sys`

VST3 binding source 层:

- 固定 Steinberg VST3 SDK baseline 和 upstream `vst3` crate baseline。
- 当前 backend 使用 upstream `vst3` crate。
- 预留 `generated-headers` backend，用于后续从官方 SDK headers 生成/维护缺失 API bindings。
- 提供 SDK header input manifest，用于锁定后续 generated-headers 生成所需的官方 `pluginterfaces` header 输入、size、sha256、baseline 和 missing headers。
- 提供 generated-bindings readiness plan，用于审计 SDK header manifest、`.rs` output module path、active backend baseline 和 reserved binding emitter 状态；该 plan 必须保持 `bindingsGenerated = false`，直到真正 emitter 完成。
- 提供 generated-bindings symbol surface，用于审计后续 emitter 预期覆盖的 VST3 interface/type/constant 名称与 header 来源；该 surface 也必须保持 `bindingsGenerated = false`，只锁定审计面，不解析 C++ AST、不验证 ABI、不生成 Rust bindings。
- 提供 deterministic metadata-only scaffold emitter，用于把 planned `.rs` output module、header inputs 和 baseline 先固定成可 `--check` 的 Rust module；它不包含 Steinberg VST3 COM/API bindings，也不会把 `bindingsGenerated` 置为 true。
- 不向插件开发者暴露 unsafe VST3 细节；安全封装仍由 `vesty-vst3` 负责。

### `vesty-ui`

UI 抽象层:

- `EditorRuntime` trait。
- `UiDescriptor`、`UiBridge`、`UiCommand`、`UiEvent`。
- dev/release asset resolution。
- 与具体 WebView 后端解耦。

### `vesty-ui-wry`

wry 后端:

- child WebView 创建。
- custom protocol。
- IPC handler。
- JS bridge injection。
- resize/focus/devtools。
- 平台 parent handle wrapper。

### `vesty-ipc`

Rust/JS schema:

- JSON envelope。
- request/response IDs。
- typed commands/events。
- generated TypeScript definitions。
- generated JSON Schema for bridge packets and key payloads。
- `vesty export-types --out ...` 从 `vesty-ipc`/`vesty-params` 的 Rust 类型源生成 `typescript/` 和 `json-schema/` 产物，避免 Web UI SDK 长期维护手写协议类型分叉。

### `vesty-build`

构建与资源:

- 读取 `vesty.toml`。
- 运行 UI build command。
- 生成 asset manifest。
- 生成 `moduleinfo.json`、Info.plist 模板。
- 组装 VST3 bundle。

### `vesty-cli`

开发者命令:

- `vesty new`
- `vesty dev`
- `vesty build`
- `vesty package`
- `vesty validate`
- `vesty doctor`

### `vesty-macros`

过程宏:

- 当前已实现 `#[derive(Params)]`: 从 `FloatParam` / `BoolParam` / `ChoiceParam` 字段生成 `ParamCollection`，支持 `#[param(skip)]` 忽略非参数字段，支持 `#[param(id = "...")]` 覆盖导出的稳定参数 ID，并支持 BoolParam `#[param(bypass)]` 标记 VST3 bypass flag。
- `vesty-params::validate_param_specs()` 在运行时校验导出的参数 schema，覆盖空/重复 ID、control characters、空 name、非法 float range、非法 default normalized、空/非法 choice label，以及 read-only 参数不能 automatable。VST3 controller 创建路径会使用该 validator 拒绝无效 schema。
- 后续增强项: `#[vesty::plugin]`、更完整的 compile-time ID/schema validation、TypeScript param schema。

## 用户 API 草案

```rust
use vesty::prelude::*;

#[derive(Params)]
pub struct GainParams {
    pub gain: FloatParam,
    pub bypass: BoolParam,
    pub mode: ChoiceParam,
}

impl Default for GainParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
            bypass: BoolParam::bypass("bypass", "Bypass", false),
            mode: ChoiceParam::new("mode", "Mode", ["Clean", "Drive", "Fuzz"], 0),
        }
    }
}

#[derive(Default)]
pub struct GainPlugin {
    params: GainParams,
}

impl Plugin for GainPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Vesty Gain",
        vendor: "Vesty",
        url: "https://github.com/backrunner/vesty",
        email: "",
        version: "0.1.0",
        class_id: *b"0123456789ABCDEF",
        kind: PluginKind::AudioEffect,
    };

    type Params = GainParams;
    type Kernel = GainKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _context: KernelInit) -> Self::Kernel {
        GainKernel
    }

    fn ui(&self) -> Option<UiDescriptor> {
        Some(
            UiDescriptor::web_assets("ui")
                .with_dev_url("http://localhost:5173")
                .with_size(900, 560)
                .with_min_size(640, 420)
                .with_resizable(true),
        )
    }

    fn host_changes_for_param(&self, id: &str, old: f64, new: f64) -> HostChangeFlags {
        if id == "mode" && (old - new).abs() > f64::EPSILON {
            HostChangeFlags::LATENCY
        } else {
            HostChangeFlags::NONE
        }
    }
}

pub struct GainKernel;

impl AudioKernel for GainKernel {
    fn process(&mut self, ctx: &mut ProcessContext<'_>) -> ProcessResult {
        let normalized = ctx.params().get_normalized("gain").unwrap_or(0.833_333);
        let gain_db = -60.0 + normalized * 72.0;
        let gain = 10.0_f32.powf(gain_db as f32 / 20.0);
        let channels = ctx.audio().input_channels().min(ctx.audio().output_channels());
        for channel in 0..channels {
            ctx.audio_mut().copy_input_to_output(channel, gain);
        }
        ProcessResult::Continue
    }
}

vesty::export_vst3!(GainPlugin);
```

## 内部 process path

```text
VST3 process callback
  -> panic_guard(process)
  -> translate ProcessData pointers to borrowed AudioBuffer
  -> drain parameter changes into FixedEventList
  -> update ParamSnapshot/automation view
  -> call AudioKernel::process
  -> write output events/audio
  -> push meters to RT queue if enabled
  -> return tresult
```

## 状态格式

使用 versioned binary state:

```text
magic: "VESTY_STATE"
schema_version: u32
plugin_version: semver
params: stable param id -> normalized value
custom_state: optional serde payload
ui_state: optional JSON/binary payload
checksum: u32/xxhash
```

当前实现:

- VST3 state 使用 `VESTY_STATE_V1\n` magic + JSON payload。
- payload 包含 `version`、`params`，以及可选 `custom`。
- 当前 VST3 state 版本常量为 `1`；读入 state 会先进入 `migrate_vst3_state()`，v1 原样接受，future/unsupported version 明确拒绝并返回 host error，不会静默套用到 plugin。
- `Plugin::save_custom_state()` / `Plugin::load_custom_state()` 默认 no-op。
- `PluginState` trait 提供 typed serde helper；开发者可在 `Plugin` impl 中调用 `save_plugin_state(self)` / `load_plugin_state(self, value)` 接入 VST3 state。

原则:

- 参数 ID 永远比字段顺序更重要。
- 删除参数要保留 migration。
- state restore 不直接写 audio kernel，可先进入 control snapshot，再在安全点同步。

## 依赖方向

```text
vesty -> vesty-core, vesty-params, vesty-macros
vesty-vst3 -> vesty-core, vesty-params, vesty-rt, vesty-ui
vesty-ui-wry -> vesty-ui, vesty-ipc, wry
vesty-cli -> vesty-build
vesty-build -> vesty-core metadata schema
```

避免依赖:

- `vesty-core` 不依赖 `vesty-vst3`。
- `vesty-core` 不依赖 `vesty-ui-wry`。
- `vesty-rt` 不依赖 `std::sync::Mutex` 作为音频路径抽象。
