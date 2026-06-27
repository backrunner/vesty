# 08. 开发文档草案

## 快速开始

```bash
cargo install vesty-cli
vesty templates
vesty new my-gain --template web-ui-param-demo
cd my-gain
vesty dev
```

在 DAW 里重新扫描 VST3 插件目录，加载 `My Gain`。开发模式下 UI 从前端 dev server 加载，Rust DSP 修改后需要重新构建插件并让 host 重新加载实例。

`vesty templates` 会列出内置 starter gallery，例如 `gain`、`midi-synth`、`web-ui-param-demo`、`vanilla-ui-param-demo`、`vue-ui-param-demo`、`svelte-ui-param-demo` 和 `web-ui-instrument`。`vesty new <name> --template <id>` 会把 starter 的默认 `[plugin].kind` 和 UI 模板写入项目；显式 `--kind` 或 `--ui` 会覆盖 template 默认值，方便从某个 starter 出发再裁剪。

`--ui` 支持:

- `react`: Vite + React + `@vesty/plugin-ui`。
- `vue`: Vite + Vue + `@vesty/plugin-ui`。
- `svelte`: Vite + Svelte + `@vesty/plugin-ui`。
- `vanilla`: Vite + TypeScript + `@vesty/plugin-ui`。
- `none`: 只生成 Rust 插件，无 Web UI。

`vesty new` 会同时生成 `README.md`、不可发布到 crates.io 的 `Cargo.toml`（`publish = false`）、`params.specs.json`、`vesty-parameters.json` 以及 `[package]` 元数据: 默认 `bundle_id = "dev.vesty.<crate-name>"`，effect 使用 `category = "Fx"`，instrument 使用 `category = "Instrument"`，并写入 `parameter_manifest = "vesty-parameters.json"`。有 UI 的模板还会在 `vesty.toml` 和 Rust `UiDescriptor` 里写入同一组默认 editor 尺寸: 900x560，最小 640x420，可 resize；`ui/package.json` 默认带 `"private": true`，因为它是随 `.vst3` 打包的 UI asset app，不是 npm library package。正式发布前应改成自己的反向域名 bundle id 和目标 VST3 category。

`vesty.toml` 的 `[plugin].kind` 是 Vesty 框架语义，只接受 effect/instrument 这组 MVP 类型及其 `fx`、`audio-effect`、`audio_effect` 同义写法。VST3 的细分类，例如 `Fx|Analyzer`，应写到 `[package].category`。

`vesty.toml` 是严格 schema。未知 table 或字段会让 `vesty build/package/dev` 在读取配置时失败；effect 可以写 `[plugin].sidechain = true` 来声明一个可选 stereo sidechain input bus，instrument 写该字段会失败。Instrument 多输出当前通过 Rust `Plugin::output_buses()` runtime API 声明，尚未提供 `[bus]` 配置 DSL；不要提前写 `[bus]`、`[ui].experimental_wayland` 或 `[package].installer` 这类未来 scope 字段。等框架支持对应配置/Wayland embedding/installer 流程时，再由 schema 和迁移文档明确接入。

`[package].bundle_id` 用于 macOS `CFBundleIdentifier`，建议使用自己的反向域名，例如 `com.example.my-gain`。Vesty 会拒绝空段、空格、斜杠、下划线等不适合发布 bundle metadata 的写法；未填写时会从 bundle executable 生成 `dev.vesty.<name>` fallback。

在 Vesty 仓库本地开发脚手架时，可以用本地 path dependency 让新项目直接编译当前源码:

```bash
vesty new my-local-gain --ui vanilla --vesty-path /path/to/vesty/crates/vesty
cargo check --manifest-path my-local-gain/Cargo.toml
```

如果同时要验证本地 JS bridge package，可以再传 `--plugin-ui-path`:

```bash
vesty new my-local-ui --ui vanilla --vesty-path /path/to/vesty/crates/vesty --plugin-ui-path /path/to/vesty/packages/plugin-ui
cd my-local-ui/ui
npm install
npm run build
npm run typecheck
```

## 写一个 effect

开发者主要实现三件事:

1. 参数。
2. plugin metadata。
3. DSP kernel。

```rust
use vesty::prelude::*;

#[derive(Params)]
pub struct Params {
    gain: FloatParam,
    bypass: BoolParam,
    mode: ChoiceParam,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
            bypass: BoolParam::bypass("bypass", "Bypass", false),
            mode: ChoiceParam::new("mode", "Mode", ["Clean", "Drive", "Fuzz"], 0),
        }
    }
}

#[derive(Default)]
pub struct Plugin {
    params: Params,
}

impl vesty::Plugin for Plugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Gain",
        vendor: "Example",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: *b"0123456789ABCDEF",
        kind: PluginKind::AudioEffect,
    };

    type Params = Params;
    type Kernel = Kernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Kernel {
        Kernel {
            gain: self.params.resolve_or_invalid("gain"),
            bypass: self.params.resolve_or_invalid("bypass"),
        }
    }

    fn latency_samples(&self) -> u32 {
        128
    }

    fn tail_samples(&self) -> u32 {
        0
    }
}

pub struct Kernel {
    gain: ParamHandle,
    bypass: ParamHandle,
}

impl AudioKernel for Kernel {
    fn process(&mut self, ctx: &mut ProcessContext<'_>) -> ProcessResult {
        let _mode = ctx.process_mode(); // Realtime, Prefetch or Offline.
        let bypass = ctx.param_normalized(self.bypass).unwrap_or(0.0) >= 0.5;
        let initial_gain = ctx.param_normalized(self.gain).unwrap_or(0.833_333);
        let frames = ctx.audio().frames().min(u32::MAX as usize) as u32;
        let channels = ctx.audio().input_channels().min(ctx.audio().output_channels());
        let (audio, events) = ctx.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.gain, initial_gain, frames) {
            let gain_db = -60.0 + segment.normalized * 72.0;
            let gain = if bypass {
                1.0
            } else {
                10.0_f32.powf(gain_db as f32 / 20.0)
            };
            for channel in 0..channels {
                audio.copy_input_to_output_range(
                    channel,
                    segment.start_sample as usize,
                    segment.end_sample as usize,
                    gain,
                );
            }
        }
        ProcessResult::Continue
    }
}

vesty::export_vst3!(Plugin);
```

## Web UI descriptor

有 Web UI 的插件在 `Plugin::ui()` 返回 `UiDescriptor`。Release 模式从打包后的 `ui` assets 目录加载；开发模式可用 `dev_url` 指向前端 dev server。尺寸可以用 builder 链式设置:

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

这些值会进入 VST3 `IPlugView::getSize()` / `checkSizeConstraint()` / `onSize()` 路径；host 仍是最终 editor window owner。

## 读取 automation

`param_normalized(handle)` 读取当前参数镜像值；host 在当前 block 内送来的 sample-accurate automation 会同时出现在 `param_automation(handle)` 中。只需要最终值时可用 `latest_param_automation(handle)`；需要逐样本准确写 audio 时，优先使用 `ParamAutomationSegments` 把 block 切成固定范围。

```rust
impl AudioKernel for Kernel {
    fn process(&mut self, ctx: &mut ProcessContext<'_>) -> ProcessResult {
        let initial = ctx.param_normalized(self.gain).unwrap_or(0.5);
        let frames = ctx.audio().frames().min(u32::MAX as usize) as u32;
        let (audio, events) = ctx.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.gain, initial, frames) {
            let gain = segment.normalized as f32;
            audio.copy_input_to_output_range(
                0,
                segment.start_sample as usize,
                segment.end_sample as usize,
                gain,
            );
        }
        ProcessResult::Continue
    }
}
```

`ParamAutomationSegments` 只保存 slice/index/cursor，不分配内存。`audio_mut_and_events()` 用于把 audio 写入权限和事件 slice 拆开，避免在持有 automation iterator 时再次借用整个 `ProcessContext`。

VST3 adapter 会在调用 `AudioKernel::process()` 前，把参数自动化和 MIDI 事件按 `sample_offset` 稳定排序；同一个 sample offset 的事件保留收集顺序。`ParamAutomationSegments` 按 sample-order 消费事件，同 offset 的同参数点以最后一次值为准，超出当前 block 的点会夹到 block 末尾且不生成越界音频区间。

## 原生 f64 DSP

默认情况下，Vesty 的开发者 DSP API 使用 f32 `AudioBuffers` / `ProcessContext`。当 VST3 host 以 `kSample64` 调用插件时，adapter 会走预分配 scratch fallback: host f64 buffer -> f32 kernel -> host f64 buffer。这个默认路径兼容现有插件，且不会在 realtime `process()` 中扩容。

需要 double-precision DSP 的插件可以显式 opt in:

```rust
impl AudioKernel for Kernel {
    const SUPPORTS_F64: bool = true;

    fn process(&mut self, ctx: &mut ProcessContext<'_>) -> ProcessResult {
        // 仍然实现 f32 路径，供 kSample32 host block 使用。
        ctx.audio_mut().clear_outputs();
        ProcessResult::Continue
    }

    fn process_f64(&mut self, ctx: &mut ProcessContext64<'_>) -> ProcessResult {
        let channels = ctx
            .audio()
            .input_channels()
            .min(ctx.audio().output_channels());
        let frames = ctx.audio().frames();

        for channel in 0..channels {
            for frame in 0..frames {
                let sample = ctx
                    .audio()
                    .input_channel(channel)
                    .and_then(|input| input.get(frame))
                    .copied()
                    .unwrap_or(0.0);
                ctx.audio_mut().set_output_sample(channel, frame, sample);
            }
        }

        ProcessResult::Continue
    }
}
```

`ProcessContext64` 提供和 `ProcessContext` 对齐的参数、事件、transport、process mode、automation segment 和 meter API；差异是 audio buffer 为 `AudioBuffers64`，sample 类型为 `f64`。`process_f64()` 仍运行在 realtime path 中，不能 allocation、lock、JSON、WebView、日志格式化或 blocking wait。

## 写一个 instrument

instrument 插件:

- 声明 event input。
- 处理需要的 MIDI/event 输入。当前 VST3 adapter 会把 NoteOn/NoteOff/PolyPressure、legacy MIDI CC、PitchBend、ChannelPressure、SysEx data event 和 Note Expression value/int/text event 转成 `vesty_core::Event`；简单 synth 可以只匹配 NoteOn/NoteOff。
- 输出 audio。

默认 instrument 只有一个 stereo output bus。需要多输出时，在插件类型上返回静态 `AudioOutputBus` 列表:

```rust
static OUTPUT_BUSES: &[AudioOutputBus] = &[
    AudioOutputBus::stereo("Main"),
    AudioOutputBus::stereo("Aux 1"),
];

impl Plugin for SynthPlugin {
    // INFO / Params / Kernel / params() / create_kernel() 略。

    fn output_buses(&self) -> &'static [AudioOutputBus] {
        OUTPUT_BUSES
    }
}
```

`ProcessContext::audio()` 看到的是按 output bus 顺序展平后的 channel list。上面的例子中 output channel `0/1` 是 Main L/R，`2/3` 是 Aux 1 L/R。当前支持最多 4 个 mono/stereo output bus、最多 8 个输出通道；真实 DAW 中的多输出 routing 仍需要单独 smoke evidence。

```rust
impl AudioKernel for SynthKernel {
    fn process(&mut self, ctx: &mut ProcessContext<'_>) -> ProcessResult {
        for event in ctx.events() {
            match event {
                Event::NoteOn { key, velocity, .. } if *velocity > 0.0 => {
                    self.active_key = Some(*key);
                }
                Event::NoteOff { key, .. } if self.active_key == Some(*key) => {
                    self.active_key = None;
                }
                _ => {}
            }
        }

        ctx.audio_mut().clear_outputs();
        let Some(key) = self.active_key else {
            return ProcessResult::Continue;
        };
        let frames = ctx.audio().frames();
        let outputs = ctx.audio().output_channels();
        for frame in 0..frames {
            let sample = self.render_sample(key);
            for channel in 0..outputs {
                ctx.audio_mut().set_output_sample(channel, frame, sample);
            }
        }
        ProcessResult::Continue
    }
}
```

如果需要 per-note expression，可以匹配 `Event::NoteExpressionValue { type_id, note_id, value, .. }`、`Event::NoteExpressionInt { .. }` 或 `Event::NoteExpressionText { text_len, text, .. }`。标准 type id 常量在 `vesty::prelude::note_expression` 下，例如 `note_expression::BRIGHTNESS`、`note_expression::TUNING` 和 `note_expression::TEXT`。text event 的 payload 是固定 UTF-16 buffer，读取时只看 `text[..text_len as usize]`。

如果需要 SysEx，可以匹配 `Event::SysEx { data_len, data, truncated, .. }`。payload 是固定 byte buffer，读取时只看 `data[..data_len as usize]`；`truncated = true` 表示 host 提供的数据超过 `MAX_SYSEX_BYTES` 或声明了非零长度但 bytes 指针为空。

`examples/midi-synth` 展示了这两个事件族的最小实时安全写法: 固定格式 SysEx `[F0, 7D, level, F7]` 只更新 kernel 内部 level override，`note_expression::BRIGHTNESS` 和 `note_expression::TUNING` 只更新当前 active note 的 brightness/tuning 字段；这些状态都留在 audio kernel 内，不写 controller state、不做 JSON、不分配内存。

插件还可以 opt in 暴露 Note Expression value metadata:

```rust
static EXPRESSIONS: &[NoteExpressionValueType] = &[
    NoteExpressionValueType::new(
        note_expression::BRIGHTNESS,
        "Brightness",
        "Bright",
    )
    .with_range(0.0, 1.0, 0.5)
    .with_flags(NoteExpressionValueFlags::ABSOLUTE),
];

fn note_expression_value_types(&self) -> &'static [NoteExpressionValueType] {
    EXPRESSIONS
}
```

VST3 controller 会通过 `INoteExpressionController` 对 instrument event input bus `0` 暴露这些有效 value metadata。当前已支持 value/int/text event translation、静态 value metadata 和静态 physical UI mapping metadata；自定义 expression editor workflow 和真实 DAW expression workflow 仍属于后续能力。

插件也可以 opt in 暴露 Note Expression physical UI mapping metadata:

```rust
static PHYSICAL_UI: &[NoteExpressionPhysicalUiMapping] = &[
    NoteExpressionPhysicalUiMapping::new(
        physical_ui::PRESSURE,
        note_expression::BRIGHTNESS,
    ),
];

fn note_expression_physical_ui_mappings(&self) -> &'static [NoteExpressionPhysicalUiMapping] {
    PHYSICAL_UI
}
```

当前 `physical_ui` 支持 `X_MOVEMENT`、`Y_MOVEMENT` 和 `PRESSURE`。VST3 controller 会通过 `INoteExpressionPhysicalUIMapping` 暴露这些静态 mapping，并过滤无效 physical UI type 或未声明的 expression type。真实 DAW expression workflow、自定义 expression editor 和 host-specific 行为仍需要外部 smoke evidence。

## 参数 ID 规则

- `id` 一旦发布不能改。
- 不要复用已删除参数 ID。
- VST3 host 看到的是由字符串 `id` 派生的稳定正数 VST3 `ParamID`；重排 Rust 字段或 `specs()` 顺序不会改变已发布参数的 host ID。
- 稳定 host ID 算法由 `vesty-params::stable_vst3_param_id()` 提供，算法名为 `vesty.vst3.param.fnv1a31-positive.v2`；不要在插件或构建脚本里复制一份私有 hash。该算法会把 FNV-1a 结果收敛到正数 31-bit 空间，避免 host/validator 把 high-bit ID 解释成负数 invalid ID。
- 插件代码、UI 和 state 仍使用字符串 ID 或 `ParamHandle`；不要把 VST3 numeric `ParamID` 写进业务状态或 JS 协议。
- 对用户可见参数设置 `automatable`。
- meter/read-only 参数不要设置 `automatable`。
- 当前 `#[derive(Params)]` 从 `FloatParam` / `BoolParam` / `ChoiceParam` 字段收集参数；默认参数 ID、名称、range、unit、bypass、choice values 等由 `FloatParam::new()`、`BoolParam::new()`、`BoolParam::bypass()`、`ChoiceParam::new()` 等 constructor 提供。
- `FloatParam` / `BoolParam` / `ChoiceParam` 都支持 `.with_automatable(false)` 和 `.as_read_only()`；`as_read_only()` 会同时把 `readOnly = true` 且 `automatable = false` 写入导出的 `ParamSpec`。
- `BoolParam::bypass(...)` 和 `BoolParam::new(...).as_bypass()` 都会设置 VST3 bypass flag。
- `ParamSpec`、`FloatParam`、`BoolParam` 和 `ChoiceParam` 都支持 `.with_midi_cc(controller)`、`.with_channel_midi_cc(controller, channel)` 和 `.with_midi_mapping(controller, Option<channel>)`，用于声明 host 可见的 `IMidiMapping`。例如 `.with_midi_cc(7)` 映射 MIDI volume CC 到所有 channel，`.with_channel_midi_cc(vesty_params::midi::PITCH_BEND, 0)` 映射 pitch bend 到 MIDI channel 1。
- 可用 `#[param(id = "stable-id")]` 覆盖 derive 导出的参数 ID；覆盖后的 ID 会用于 spec、resolve、get/set 和 handle 映射，适合让 Rust 字段名或内部 constructor ID 和发布后的 host 参数 ID 解耦。
- 可用 `#[param(bypass)]` 在 `BoolParam` 字段上标记 VST3 bypass flag；这等价于在导出的 spec 上设置 bypass，不改变 realtime value 存储。
- `ChoiceParam::new(id, name, values, default_index)` 的 `default_index` 会 clamp 到可用范围；运行时设置 normalized 值时会 snap 到最近的离散选项；bridge 和 VST3 host 文本格式化会优先显示/解析 choice label。
- 非参数字段使用 `#[param(skip)]`。
- `#[param(skip)]` 不能和 `id` / `bypass` 混用；`#[param(bypass)]` 只能用于 `BoolParam` 字段。
- Vesty 会在参数 schema 进入 VST3 controller 前校验 `ParamSpec`: ID 不能为空或重复，ID/name/unit/choice label 不能包含 control characters，float range 必须有限且 `min < max`，`defaultNormalized` 必须在 `0.0..=1.0`，read-only 参数不能同时标记 automatable，MIDI controller 必须在 `0..=140` 且 channel 必须在 `0..=15`。

## Program Lists And Apply

如果插件有一组固定出厂 program 名称，可以在 `Plugin::program_lists()` 返回静态描述。VST3 host 可通过 `IUnitInfo` 查询 root unit、program list 和 program name。需要响应 host program selection 时，再覆盖 `apply_program()`；默认实现返回 `Ok(false)`，保持 metadata-only 行为。

```rust
use vesty::prelude::*;

static PROGRAMS: &[Program] = &[
    Program::new("Init"),
    Program::new("Bright Lead"),
    Program::new("Soft Pad"),
];

static PROGRAM_LISTS: &[ProgramList] = &[
    ProgramList::new(1, "Factory Programs", PROGRAMS),
];

static BRIGHT_LEAD_ATTRIBUTES: &[ProgramAttribute] = &[
    ProgramAttribute::new("category", "Lead"),
    ProgramAttribute::new("mood", "Bright"),
];

static BRIGHT_LEAD_PITCH_NAMES: &[ProgramPitchName] = &[
    ProgramPitchName::new(60, "C4 Lead"),
    ProgramPitchName::new(64, "E4 Lead"),
];

impl vesty::Plugin for Plugin {
    // ...

    fn program_lists(&self) -> &'static [ProgramList] {
        PROGRAM_LISTS
    }

    fn program_attributes(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramAttribute] {
        if list_id == 1 && program_index == 1 {
            BRIGHT_LEAD_ATTRIBUTES
        } else {
            &[]
        }
    }

    fn program_pitch_names(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramPitchName] {
        if list_id == 1 && program_index == 1 {
            BRIGHT_LEAD_PITCH_NAMES
        } else {
            &[]
        }
    }

    fn apply_program(&self, list_id: u32, program_index: usize) -> Result<bool, StateError> {
        if list_id != 1 {
            return Ok(false);
        }

        let gain = match program_index {
            0 => 0.5,
            1 => 0.8,
            2 => 0.3,
            _ => return Ok(false),
        };
        self.params()
            .set_normalized("gain", gain)
            .map_err(|error| StateError::custom(error.to_string()))?;
        Ok(true)
    }

    fn program_data_supported(&self, list_id: u32) -> bool {
        list_id == 1
    }

    fn save_program_data(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> Result<Option<serde_json::Value>, StateError> {
        if list_id != 1 || program_index >= PROGRAMS.len() {
            return Ok(None);
        }

        Ok(Some(serde_json::json!({
            "gain": self.params().get_normalized("gain").unwrap_or(0.5),
        })))
    }

    fn load_program_data(
        &self,
        list_id: u32,
        program_index: usize,
        data: serde_json::Value,
    ) -> Result<bool, StateError> {
        if list_id != 1 || program_index >= PROGRAMS.len() {
            return Ok(false);
        }

        let gain = data
            .get("gain")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| StateError::Deserialize("missing gain program data".into()))?;
        self.params()
            .set_normalized("gain", gain)
            .map_err(|error| StateError::custom(error.to_string()))?;
        Ok(true)
    }
}
```

`program_attributes()` 和 `program_pitch_names()` 运行在 controller 非实时查询路径，adapter 会过滤无效静态 metadata: 空 attribute id、NUL-containing strings、越界 MIDI pitch、空 pitch name 都不会暴露给 host。`apply_program()`、`save_program_data()` 和 `load_program_data()` 同样运行在 controller 非实时路径，不能在 audio `process()` 中调用。

VST3 adapter 会把 `save_program_data()` 返回的 JSON payload 包进 `VESTY_PROGRAM_DATA_V1\n` envelope，经 `IProgramListData::getProgramData()` 写入 host 提供的 `IBStream`；`IProgramListData::setProgramData()` 和 `IUnitInfo::setUnitProgramData(data != null)` 会校验 magic、version、list id 和 program index 后调用 `load_program_data()`。成功应用 program 或 program data 后，adapter 会 diff 参数值、通知 host `kParamValuesChanged`，并把变化以 `source = "program"` 排队给 Web UI。需要让 host 识别某个 choice/int-like 参数为 program-change 参数时，可在参数 builder 或 spec 上调用 `.as_program_change()`；这会映射到 VST3 `ParameterInfo::kIsProgramChange` metadata，并让 controller/control-thread 上的 `setParamNormalized()` / edit relay 尝试把 plain value 解释为第一个可见 program list 的 index。只有 index 有效且 `apply_program()` 返回 `Ok(true)` 时才应用 program 并同步该参数自身；无 program list、越界或 `Ok(false)` 会回退为普通参数写入。audio thread 仍不会自动调用 `apply_program()` 或 `load_program_data()`；host 在 `process()` 中送来的 program-change 参数 automation 会作为普通 sample-accurate 参数事件进入 DSP，并更新 atomic 参数快照。`examples/midi-synth` 展示了这套 flow 的最小完整写法，包括 host-visible `program` 参数、program list metadata、program attributes/pitch names、per-program JSON data roundtrip、固定 SysEx level override 和 Note Expression brightness/tuning。真实 DAW program/SysEx/expression workflow evidence 仍是后续外部验收项。

## Host 变更通知

如果某个参数改变会影响宿主可见能力，例如 latency、IO 或参数显示，插件可以覆盖 `host_changes_for_param()`。VST3 adapter 会在 controller/control thread 上把 `HostChangeFlags::LATENCY` 映射到 `restartComponent(kLatencyChanged)`；不要在 audio `process()` 中触发这类通知。

```rust
fn host_changes_for_param(&self, id: &str, old: f64, new: f64) -> HostChangeFlags {
    if id == "quality" && (old - new).abs() > f64::EPSILON {
        HostChangeFlags::LATENCY
    } else {
        HostChangeFlags::NONE
    }
}
```

## UI 开发

前端可以使用任意 bundler。只需要调用:

```ts
import { createBridge } from "@vesty/plugin-ui";

const bridge = createBridge();
const hello = await bridge.ready();
const snapshot = await bridge.getSnapshot();

bridge.beginParamEdit("gain");
bridge.performParamEdit("gain", 0.72);
bridge.endParamEdit("gain");

bridge.subscribe("param.changed", (event) => {
  // update local UI state from host/plugin changes
});
```

对于 slider:

- pointerdown: `beginParamEdit`。
- pointermove/input: `performParamEdit`。
- pointerup/cancel/lostpointercapture: `endParamEdit`。
- 建议在 pointerdown 时调用 `setPointerCapture(pointerId)`，并用本地 `editing` guard 防止 `pointerup` 和 `lostpointercapture` 重复发送 `endParamEdit`。`vesty new` 生成的 vanilla/React/Vue/Svelte 模板已经内置这个模式。

对于 toggle:

- click: begin + perform + end。

## 状态保存

只保存稳定、可迁移的状态:

```rust
#[derive(Serialize, Deserialize)]
struct MyState {
    oversampling: u8,
    theme: String,
}

impl PluginState for Plugin {
    type State = MyState;

    fn save_state(&self) -> Self::State {
        MyState {
            oversampling: self.oversampling.load(Ordering::Relaxed),
            theme: self.theme.lock().unwrap().clone(),
        }
    }

    fn load_state(&self, state: Self::State) -> Result<(), StateError> {
        self.oversampling.store(state.oversampling.min(4), Ordering::Relaxed);
        *self.theme.lock().map_err(|_| StateError::custom("theme lock poisoned"))? = state.theme;
        Ok(())
    }
}

impl vesty::Plugin for Plugin {
    // ...

    fn save_custom_state(&self) -> Result<Option<serde_json::Value>, StateError> {
        save_plugin_state(self).map(Some)
    }

    fn load_custom_state(&self, state: Option<serde_json::Value>) -> Result<(), StateError> {
        if let Some(state) = state {
            load_plugin_state(self, state)?;
        }
        Ok(())
    }
}
```

参数值由框架保存；`PluginState` 只用于额外的可迁移配置。当前 VST3 state 格式会保存 `version = 1`、`params` 和可选 `custom` JSON payload。框架读入 VST3 state 时会先走版本迁移入口；当前只接受 v1，future/unsupported version 会被拒绝，不会静默写入参数或 custom state。

## 实时安全清单

在 `process` 中不要:

- 创建 `Vec`、`String`、`Box`。
- `lock()`。
- 访问 Web UI。
- 读写文件。
- `println!`。
- `unwrap()` 不可信数据。

推荐:

- 在 `prepare` 里分配。
- 使用固定容量容器。
- 在 `create_kernel()` 中用 `params.resolve_or_invalid("id")` 解析参数 handle；正常参数走快速 handle 路径，参数 ID 写错时会得到一个 invalid handle，后续 `param_normalized()` 返回 `None` 并让 DSP fallback 到默认值，而不是在 host 初始化回调中 panic。
- 用参数 handle 代替字符串查找。
- 队列满时丢弃 UI/meter 数据。

## 调试

```bash
vesty doctor
vesty export-types --out target/vesty-protocol
vesty export-types --out target/vesty-protocol --check
vesty daw-matrix --write-template --evidence-root target/daw-evidence --format markdown
vesty daw-matrix --write-report --evidence-root target/daw-evidence --host bitwig --platform "macOS arm64 / Bitwig smoke" --scan "VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3" --load "load=true" --ui "ui=true" --ui-host-param "ui_host_param=true" --meter-stream "meter_flush sent=3" --automation "automation=true" --buffer-sample-rate-change "buffer_sample_rate_change=true" --save-restore "save_restore=true" --offline-render "offline_render=true" --format json
vesty daw-matrix --evidence-root target/daw-evidence --format json --strict
vesty platform-smoke --write-template --dir target/release-evidence/platform-smoke
vesty platform-smoke --write-report --dir target/release-evidence/platform-smoke --platform macos --system-webview "WebKit.framework loaded" --vst3-validator "Steinberg validator passed 47 tests, 0 failed" --vst3-example-scan "VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3" --webview-attach "webview_attach=true" --webview-resize "webview_resize=true width=640 height=420" --asset-protocol "asset_protocol=true assets.manifest.json served" --jsbridge-roundtrip "jsbridge_roundtrip=true readyAck reply" --meter-stream "meter_flush sent=3"
vesty platform-smoke --check --dir target/release-evidence/platform-smoke --strict
vesty host-quirks --format markdown
vesty host-quirks --host bitwig --format json
mkdir -p target/smoke-host
npm run build --prefix examples/web-ui-param-demo/ui
vesty smoke-host --out target/smoke-host/smoke-host.json
vesty smoke-host --out target/smoke-host/smoke-host.json --check
printf '%s\n' '{"type":"param.begin","result":0}' '{"type":"param.perform","result":0}' '{"type":"param.end","result":0}' 'result=0' > target/smoke-host/bridge-trace.log
printf '%s\n' 'meter_flush sent=1' > target/smoke-host/meter.log
vesty smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --strict
vesty release-check --format markdown
vesty release-check --write-evidence-template target/release-evidence --format markdown
vesty release-evidence collect-local --dir target/release-evidence --protocol-snapshot target/vesty-protocol
vesty release-evidence collect-local --dir target/release-evidence --protocol-snapshot target/vesty-protocol --crate-package
vesty release-evidence collect-local --dir target/release-evidence --protocol-snapshot target/vesty-protocol --vst3-sdk-dir /path/to/VST_SDK --vst3-sdk-bindings-module target/vst3-sdk/generated.rs
vesty crate-package --out target/crate-package/crate-package.json
vesty crate-package --check --out target/crate-package/crate-package.json
vesty release-check --format json --strict
vesty release-check --format json --strict \
  --evidence-root target/daw-evidence \
  --release-evidence-dir target/release-evidence \
  --protocol-snapshot target/vesty-protocol \
  --report target/release-evidence/release-check.json \
  --require-release-artifacts
vesty param-manifest --specs params.specs.json --out vesty-parameters.json
vesty param-manifest --specs params.specs.json --out vesty-parameters.json --check
vesty build --debug
vesty build --debug --no-ui
vesty dev --config vesty.toml --no-ui --install-dev --vst3-dir target/dev-vst3
vesty dev --config vesty.toml --no-ui --install-dev --binary target/debug/libmy_plugin.dylib --vst3-dir target/dev-vst3
vesty package --config vesty.toml --platform macos --binary target/release/libmy_plugin.dylib --install-dev
vesty package --config vesty.toml --platform macos --binary target/release/libmy_plugin.dylib --install-dev --vst3-dir target/dev-vst3 --install-mode copy
vesty notarize target/vesty/MyPlugin.vst3 --keychain-profile VestyNotary
vesty validate target/vesty/MyPlugin.vst3
vesty validate target/vesty/MyPlugin.vst3 --static-only
vesty validate target/vesty/MyPlugin.vst3 --format json
vesty validate target/vesty/MyPlugin.vst3 --strict --format json --report target/validate.json --validator-log target/validator.log
vesty doctor --format json
vesty vst3-sdk manifest --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/vst3-sdk-headers.json
vesty vst3-sdk binding-plan --sdk-dir /path/to/VST_SDK --bindings-module target/vst3-sdk/generated.rs --out target/vst3-sdk/generated-bindings-plan.json
vesty vst3-sdk binding-surface --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-bindings-surface.json
vesty vst3-sdk binding-surface --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-bindings-surface.json --check
vesty vst3-sdk emit-scaffold --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated.rs
vesty vst3-sdk emit-scaffold --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated.rs --check
vesty vst3-sdk emit-abi-seed --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-abi-seed.rs
vesty vst3-sdk emit-abi-seed --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-abi-seed.rs --check
vesty vst3-sdk emit-abi --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-abi.rs
vesty vst3-sdk emit-abi --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-abi.rs --check
vesty vst3-sdk emit-interface-skeleton --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-interface-skeleton.rs
vesty vst3-sdk emit-interface-skeleton --sdk-dir /path/to/VST_SDK --out target/vst3-sdk/generated-interface-skeleton.rs --check
RUST_LOG=vesty=debug vesty dev
```

Web UI:

- debug 构建默认开启 devtools。
- release 构建默认关闭。
- 如果 WebView 创建失败，查看 `vesty doctor` 的 runtime 检查。
- `vesty dev --install-dev` 会在启动/跳过 UI dev server 前打包并安装 debug `.vst3`，适合 DAW 反复 rescan。
- 不传 `--binary` 时，CLI 会用 Cargo metadata 匹配当前项目 `Cargo.toml` 的 `cdylib` target 并推断 debug/release cdylib 路径；`--binary <path>` 仍可作为显式 override。

本地 headless self-check:

- `vesty smoke-host` 检查 Vesty workspace 根 manifest、三个 MVP examples 的 `vesty.toml`、`params.specs.json` 到 `vesty-parameters.json` 的 drift，以及 Web UI demo 已构建 asset manifest。
- `--bridge-trace` 和 `--meter-log` 可传入本地桥接/meter marker；`--strict` 会把缺失可选 marker 也视为失败，适合 CI 诊断。
- `--out` 写 JSON report，`--check --out` 复验 report 与当前 workspace 状态是否一致。
- 该命令不加载 `.dylib` / `.dll` / `.so`，不替代真实 DAW、platform WebView、Steinberg validator、签名或 notarization evidence。

协议类型:

- `vesty export-types --out target/vesty-protocol` 会生成 `typescript/protocol/*.ts` 和 `json-schema/*.schema.json`。
- 这些类型来自 Rust `vesty-ipc`/`vesty-params` 源类型，适合给 Web UI SDK、schema drift 检查和框架适配包复用。
- `vesty export-types --out target/vesty-protocol --check` 不会修改 `--out`，而是把临时导出结果与该目录逐文件比较；有 drift 时返回非零。
- `vesty release-check --format json` 的 `protocol snapshot` check 在 drift 时会把 missing/changed/extra 的相对路径摘要写进 `value`，并在 `hint` 中给出当前 snapshot 路径对应的 `vesty export-types --out ...` 命令。
- 最终发布门禁必须先运行 `vesty export-types --out target/vesty-protocol --check`，再在 `vesty release-check --strict --require-release-artifacts` 中传入 `--protocol-snapshot target/vesty-protocol`；最终 gate 不允许 `--skip-protocol`。

DAW evidence:

- `vesty daw-matrix --write-template` 会创建 REAPER/Cubase/Bitwig/Ableton/Studio One 的 evidence 目录模板。
- 每个 evidence 目录的 `README.md` 会包含对应 host profile、required smoke checks、platform 范围和 quirk/mitigation 表，方便手工 smoke 前逐项核对。
- 模板不会覆盖已有日志或已有 README，`pending` 值不会被矩阵当作 pass。
- 真实 smoke 采集完成后可用 `vesty daw-matrix --write-report --host <host>` 写入标准 evidence 文件；CLI 要求显式 marker，写入前会拒绝 pending/false/占位值、`manual platform pending`、zero meter、明显负面 marker（failed/not found/unavailable/crashed 等）和无法被 matrix parser 识别的模糊 marker，避免留下半套无效 evidence。`--platform` 还必须能映射到该 host profile 声明支持的平台，例如 Bitwig 可用 `Linux X11`，但 Wayland 或 Ableton Linux 这类不在 profile 内的组合会被拒绝。写入后仍会复用 matrix parser 验证该 host 行完整。REAPER 仍支持专用 cache/render/param-watch evidence，同时也接受这套通用 marker。
- 手工编辑或导入的 `platform.txt` 在读取时也会用同一 host profile 规则复验；如果把 Bitwig 改成 `Linux Wayland`、只写泛 `Linux`，或把 Ableton Live 改成 `Linux X11`，其它 smoke marker 即使都为 pass，Markdown `Platform` 列也会显示 `(unsupported)`，并且 `vesty daw-matrix --strict` / `vesty release-check` 仍会把该行标为缺失 `platform` evidence。
- `release-check` 的 JSON report 自身也会自校验 DAW 平台状态；手工把 `daw_matrix[].platform_supported` 改成 `true`，但 `platform` 仍是该 host 不支持的文本，或把受支持平台标成 `false`，都会在 report shape validation 阶段失败。
- 如果 marker log 明确写了 `host=...`、`daw=...`、`daw_host=...`、`host_profile=...` 或 `profile=...`，读取端会要求它匹配当前 host evidence 目录；把 `host=Ableton Live` 的日志放进 `bitwig/` 不会被当作 Bitwig pass evidence。普通没有显式 host 字段的既有日志仍保持兼容。
- 缺失的 DAW 行仍会显示为 missing，但 `Evidence` 列会指向应补日志的 evidence 目录。
- `vesty daw-matrix --strict` 会在任意 DAW 或 smoke 项缺失时返回非零；发布前可用它防止误把 incomplete matrix 当成 pass。
- `vesty host-quirks` 会输出内置 host profile/quirk registry，帮助准备每个 DAW 的 smoke 项和注意事项；它不是兼容性通过证据。
- `vesty release-check` 会聚合 DAW matrix、host profile 覆盖和 protocol snapshot drift check；`--strict` 会在报告打印后对缺失 evidence 返回非零。
- `release-check` 的 `daw_matrix` JSON 行必须精确对应当前内置 release host profile 集合，并使用 canonical host name: `REAPER`、`Cubase/Nuendo`、`Bitwig Studio`、`Ableton Live`、`Studio One`。缺行、重复行、未知 host、别名 host（例如 `reaper`）或额外第三方 host 行都会被拒绝；`host profiles` invariant check 也会要求 value 精确列出这五个 profile。第三方 DAW smoke 可以另行记录，但不算当前 MVP release gate 的五 host matrix。
- `release-check` JSON 中的 `host profiles`、`daw matrix` 和每个 `daw smoke:<host>` check 必须与 `daw_matrix` 明细重新计算出的 status/value 完全一致。手工把明细改成 pass 但保留 failed summary，或把某个 DAW 明细缺项却把 `daw smoke:<host>` 改成 ok，都会在 report shape validation 阶段失败。
- `vesty platform-smoke --write-template --dir <dir>` 会创建 macOS、Windows x64 和 Linux X11 的 system WebView/VST3 editor smoke 模板；pending JSON 模板不会被当作通过证据，也不会把普通 optional release-check 从 `skipped` 变成 `failed`。真实 report 必须覆盖 system WebView、Steinberg validator、示例插件扫描、WebView attach/resize、asset protocol、JSBridge roundtrip 和非零 meter stream；Linux Wayland 仍是 experimental，不进入首版 release gate。采集完真实 smoke 后推荐用 `vesty platform-smoke --write-report --platform ...` 把显式 marker 写成规范 JSON；CLI 写入前会拒绝 pending/false/占位值、zero meter、`system_webview=true` / `vst3_validator=true` 这类泛 marker。system WebView evidence 必须按平台识别: macOS 为 `WebKit.framework` / `WKWebView`，Windows x64 为 `WebView2`，Linux X11 为 `WebKitGTK` + active `X11`，且不能同时写 Wayland/fallback/not-X11 这类否定或实验路径；validator evidence 必须识别 Steinberg/VST3 validator，并带 passed tests / 0 failed 摘要。
- `vesty release-check --require-release-artifacts` 会把外部 release evidence 也作为硬门禁: `--release-evidence-dir`，或分项传入 `--ci-run-url`、`--ci-doctor-dir`、`--ci-release-check-dir`、`--platform-smoke-dir`、`--publish-plan-report`、`--crate-package-report`、`--npm-pack-report`、`--validate-report`、`--static-validate-report`、`--signed-bundle-evidence`、`--notarization-log`。crate package readiness 在普通本地检查中缺失会保持 `skipped`，但在 `--require-release-artifacts` 下必须存在并严格有效；generated-headers 审计证据缺失时保持 `skipped`，存在时严格校验。generated-headers 审计证据可通过 `--vst3-sdk-manifest`、`--vst3-sdk-binding-plan` 和 `--vst3-sdk-binding-surface` 分项传入，其中 binding-surface 还要求 `missingSymbols = []` 且所有 required symbol 都有 `symbolPresent = true`。`generated.rs` scaffold、`generated-abi-seed.rs` ABI seed、`generated-abi.rs` ABI layout 和 `generated-interface-skeleton.rs` interface skeleton 只作为 `collect-local` / `import-ci` 的 drift/audit 留档，不作为 release-check pass evidence。不加 `--require-release-artifacts` 时，外部 evidence 缺失只会显示为 `skipped`，方便本地开发检查 DAW/protocol 部分。加上该参数时，protocol snapshot 也不能被跳过；`--skip-protocol` 只适合本地临时检查或 per-OS CI snapshot。
- `vesty daw-matrix --evidence-root <dir>` 和 `vesty release-check --evidence-root <dir>` 会按 `<dir>/reaper`、`<dir>/cubase`、`<dir>/bitwig`、`<dir>/ableton`、`<dir>/studio-one` 读取 DAW smoke evidence，适合把外部 DAW 证据打包成一个目录。
- `vesty release-check --write-evidence-template <dir>` 会生成 release artifact 证据模板；模板不会覆盖已有日志，`pending` validator validate、static validate、signing、notary 值不会被 release gate 当作 pass。模板会创建 `ci-doctor/README.md` 指引下载 `doctor-Linux.json`、`doctor-macOS.json` 和 `doctor-Windows.json`，创建 `ci-release-checks/README.md` 指引下载 `release-check-Linux.json`、`release-check-macOS.json` 和 `release-check-Windows.json`，并说明可选的 `release-action-plan-Linux.json`、`release-action-plan-macOS.json`、`release-action-plan-Windows.json` sidecar 只用于人工采证追踪，创建 `platform-smoke/README.md` 指引运行 `vesty platform-smoke --write-template --dir <dir>/platform-smoke`，并创建 `vst3-sdk/README.md` 说明 `vst3-sdk-headers.json`、`generated-bindings-plan.json`、`generated-bindings-surface.json`、`generated.rs` scaffold、`generated-abi-seed.rs` ABI seed、`generated-abi.rs` ABI layout 和 `generated-interface-skeleton.rs` interface/vtable skeleton 的生成/复验命令；这些 README 本身不算 evidence。
- `vesty release-check --report <path>` 会额外保存完整 JSON report，适合 CI artifact 和 release note 留档。
- `vesty release-check --plan <path>` 会额外保存机器可读 release action plan。它根据当前 report 把 failed/skipped checks 转成待办项，包含 priority、evidence path 和建议命令，适合真实 DAW、平台、CI、签名和 notarization 采证时逐项核对；它不是 pass evidence，也不会改变 `release-check` 的通过/失败结果。GitHub Actions 的 per-OS release-check artifact 会同时包含 `release-action-plan-<OS>.json` sidecar；`--ci-release-check-dir` 只采纳 `release-check-*.json`，不会把 action plan 当作通过证据。Action plan sidecar 会校验顶层 `protocol_snapshot`、`evidence_root`、`release_evidence_dir` 和 action `evidence_path` 的词法路径安全性，拒绝 `..` parent-directory component，并会校验已知 action 的 `evidence_path` 与这些顶层路径推导出的标准路径一致，避免 checklist 路径与建议命令互相矛盾。
- `vesty release-evidence collect-local` 写出的 `local-collect-report.json` 会校验本地 evidence 明细和顶层字段自洽。除 `protocol snapshot` item 可指向独立 `target/vesty-protocol` 外，其它 item path 必须位于 `evidence_dir` 下；顶层 `protocol_snapshot` 必须精确匹配唯一一条 ok 的 `protocol snapshot` item，缺少顶层字段时也不能出现该 item。
- GitHub Actions 中的 `vesty doctor --format json` 会自动写入可选 `ci_run_url`；当 release evidence 同时提供 `ci-run-url.txt` / `--ci-run-url` 时，`release-check` 会确认新版 doctor artifacts 来自同一个 repo/run id。没有 `ci_run_url` 的旧 doctor JSON 仍可用于本地兼容检查。
- GitHub Actions 的 `vesty-smoke-host` artifact 只记录 `smoke-host` 本地诊断 report、bridge marker 和 meter marker；`release-evidence import-ci` 不会把它导入为 release pass evidence。

Bundle validation:

- `vesty validate <bundle.vst3>` 先跑 Vesty 静态 bundle/resource check，再调用 Steinberg validator。
- `vesty package` 会读取 `[package].signing`；macOS 使用该 identity 调用 `codesign` 签 `.vst3` bundle，Windows 使用 `signtool.exe` 签 platform binary，Linux 保持发行渠道外部签名策略。
- `vesty package` 可读取可选 `[package].parameter_manifest = "path/to/vesty-parameters.json"`；该文件必须是 `vesty-build::ParameterManifest` JSON，打包后会成为 `Contents/Resources/parameters.manifest.json`。它用于发布包审计字符串参数 ID 与 VST3 数值 `ParamID` 的对应关系；推荐用 `vesty param-manifest --specs params.specs.json --out vesty-parameters.json --check` 在 CI 中防止参数 sidecar 漂移。CLI 不会从已编译插件自动生成或 introspect 这个 sidecar。
- `vesty package --install-dev` 会在打包后把 `.vst3` copy 或 symlink 到本机 VST3 dev 目录；`--vst3-dir` 可覆盖目标路径，默认 `--install-mode copy`。
- `vesty package` 和 `vesty dev --install-dev` 不传 `--platform` 时会按当前 OS 推断打包平台；交叉打包时显式传 `--platform macos|windows-x64|linux-x64`。
- `vesty notarize <bundle.vst3> --keychain-profile <profile>` 会在 macOS 上创建 notary zip、调用 `xcrun notarytool submit --wait`，并默认执行 `xcrun stapler staple`；Apple ID 模式可用 `--apple-id --team-id --password`。
- `vesty validate <bundle.vst3> --static-only` 只跑静态 check，可用于没有 validator 的 CI；它会检查平台 binary 魔数和架构，macOS 需为 64-bit Mach-O/fat Mach-O，Windows x64 需为 PE/MZ 且 COFF machine 为 x86_64，Linux x64 需为 64-bit ELF 且 `e_machine = x86_64`。静态检查还会尽量用 `nm` / `llvm-nm` / `llvm-objdump` / `dumpbin` 检查 VST3 导出符号；工具成功运行但缺少 `GetPluginFactory` 或平台 entry/exit symbol 会使 static validation 失败，工具缺失或无法解析会记录为 skipped。加 `--strict` 时，每个可识别平台 binary 都必须有匹配的 `ok` `static_check.binary_exports`；缺失/skipped 会在 report 写出后返回非零，适合 release package CI。
- `vesty validate <bundle.vst3> --format json` 会输出机器可读报告: `static_check.status`、`static_check.binaries`、`static_check.binary_exports`、`static_check.parameter_manifest`、`static_check.asset_manifest`、`validator.status`、`validator.exit_code`、`validator.tests_passed`、`validator.tests_failed` 和 validator stdout/stderr。
- `vesty validate <bundle.vst3> --report <path> --validator-log <path>` 会分别保存 JSON report 和 validator 原始日志；静态检查失败时也会先保存 report。
- `vesty release-check --release-evidence-dir <dir>` 会按 `--write-evidence-template` 的目录约定自动发现 CI URL、CI doctor artifacts、CI per-OS release-check artifacts、platform smoke artifacts、validator/static validate reports、publish-plan/crate-package/npm-pack reports、`vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json`、`vst3-sdk/generated-bindings-surface.json`、签名日志和 notary log；`ci-run-url.txt` 只接受裸 GitHub Actions run URL，或 `ci_run_url=` / `ci-run-url=` key，忽略其它 `key=value` 说明行和 `pending`；显式传入的单项参数仍优先。`crate-package/crate-package.json` 在普通本地检查中缺失会保持 `skipped`，但在 `--require-release-artifacts` 下必须存在；存在时必须显示 leaf crates 为 `packaged`、内部依赖 crates 为 `deferred`。`platform-smoke/` 只有存在非 pending report 或无法解析的 JSON 时才会自动启用，单独生成的 pending 模板不会把 optional release-check 从 `skipped` 变成 `failed`。VST3 SDK manifest/plan/surface 只有 JSON 文件存在时才启用，缺失保持 `skipped`；存在时必须完整有效，且 plan/surface 必须保持 `bindingsGenerated = false`。`vst3-sdk/generated.rs` scaffold、`vst3-sdk/generated-abi-seed.rs` ABI seed、`vst3-sdk/generated-abi.rs` ABI layout 和 `vst3-sdk/generated-interface-skeleton.rs` interface skeleton 可作为 `import-ci` 规范化后的审计文件留档，但 release-check 不把它们当作 pass evidence。
- `vesty release-check --validate-report <path>` 会读取该 JSON report，要求静态 bundle check 为 `ok` 且 Steinberg validator 为 `passed`；`--static-only` 生成的 skipped validator report 不会被当作 release 通过证据。
- `vesty release-check --validate-report <path>` 在 Vesty framework release 的 `--require-release-artifacts` 下还会要求 `VestyGain.vst3`、`VestyWebUIDemo.vst3`、`VestyMIDISynth.vst3` 在 `linux-x64`、`macos`、`windows-x64` 上都有 Steinberg validator-passed report；这些示例/platform report 还必须包含指向对应 bundle `Contents/Resources/parameters.manifest.json` 的 `static_check.parameter_manifest`，证明 `[package].parameter_manifest` 已进入 packaged `.vst3`。`VestyWebUIDemo.vst3` 额外要求指向 `Contents/Resources/assets.manifest.json` 的 `static_check.asset_manifest` 和非零 `asset_count`。最终 strict gate 还要求每个示例/platform report 含匹配平台且 required/found symbols 完整的 `ok` `static_check.binary_exports`；`skipped` 只能作为诊断记录。生成 release validator report 时应使用 `vesty validate --strict --report <path>`，让导出符号工具缺失或 skipped evidence 在采证阶段直接失败。
- `vesty crate-package --out <path>` 会为 publishable Rust crates 生成 package readiness report: 当前无内部 workspace 依赖的 crate 会运行真实 `cargo package -p <crate> --allow-dirty --no-verify` 并标记为 `packaged`，仍依赖其它 Vesty workspace crate 的 package 会标记为 `deferred`。`vesty crate-package --check --out <path>` 只复验已有 report。`vesty release-check --crate-package-report <path>` 和 `--release-evidence-dir` 的 `crate-package/crate-package.json` 会把它作为可选预发布 evidence；它不表示已完成 `cargo publish`。
- `vesty npm-pack --out <path>` 会运行 npm workspace dry-run pack，写入规范 JSON，并立即复用 release gate 校验四个 JS package 的发布边界；`vesty npm-pack --check --out <path>` 只复验已有 report。`vesty release-check --npm-pack-report <path>` 要求 `@vesty/plugin-ui`、`@vesty/react`、`@vesty/vue` 和 `@vesty/svelte` 全部存在，且 packed files 只包含 `dist/**` 和 `package.json`；`--release-evidence-dir` 会自动发现 `npm-pack/npm-pack.json` 或根目录 `npm-pack.json`。
- `vesty release-check --platform-smoke-dir <dir>` 会读取 `vesty platform-smoke --format json` 风格的 macOS、Windows x64、Linux X11 report；`--require-release-artifacts` 会要求三平台都存在且真实通过，Linux Wayland report 会被拒绝；手写 report 也会复用平台特异 WebView 和 Steinberg/VST3 validator passed/0 failed evidence 校验。
- `vesty release-check --static-validate-report <path>` 会读取 `vesty validate --static-only --report` JSON，只证明 CI packaging smoke 的 bundle 结构、平台 binary 魔数/架构，以及可观察到的导出符号状态；它不会替代 `--validate-report` 的 Steinberg validator passed 证据。Vesty framework release 的 `--require-release-artifacts` 会额外要求 `VestyGain.vst3`、`VestyWebUIDemo.vst3`、`VestyMIDISynth.vst3` 在 `linux-x64`、`macos`、`windows-x64` 上都有 static validate report；这些示例 static reports 同样必须带指向对应 bundle `Contents/Resources/parameters.manifest.json` 的 `static_check.parameter_manifest`，Web UI 示例还必须带指向 `Contents/Resources/assets.manifest.json` 的 UI asset manifest evidence，并且最终 strict gate 要求匹配平台的 `static_check.binary_exports` 为完整 `ok` evidence。生成 release package evidence 时推荐运行 `vesty validate --static-only --strict --report <path>`，让 skipped binary export evidence 在 CI 阶段失败而不是等到最终聚合门禁。
- `vesty release-check --release-evidence-dir <dir>` 会额外扫描目录内其它 `*.json` validate reports；validator-passed reports 自动进入 release validate evidence，static-only/skipped reports 自动进入 CI static validate smoke，方便直接使用 CI package artifact 解压目录。目录中的 macOS `.vst3` 只有在 `Contents/_CodeSignature/CodeResources` 是可解析 plist，且包含 `files` 或 `files2` dictionary 条目时才会被当作 signed bundle evidence；占位文本文件或字符串值不会通过。
- `vesty doctor --format json` 会输出 `os`、可选 `ci_run_url` 和 `checks[]`，每个 check 带 `name`、`status`、`value` 和可选 `hint`，适合 CI 预检；其中包含 toolchain、VST3 binding baseline、VST3 SDK headers probe、WebView、validator、release signing/notarization 前置工具和 DAW install 检查。
- `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out target/vst3-sdk-headers.json` 会生成 deterministic SDK header input manifest，记录 Vesty generated-headers 后备路径所需的官方 `pluginterfaces` headers、size、sha256、baseline、generator、version hint 和 missing headers；`--check --out <path>` 会复验已有 manifest 并在 SDK checkout 漂移时失败。该 manifest 是后续 bindgen/com-scrape 的输入锁定证据，不表示完整 VST3 SDK bindings 已生成。
- `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --bindings-module target/vst3-sdk/generated.rs --out target/vst3-sdk/generated-bindings-plan.json` 会生成 generated-bindings readiness report；`--check --out <path>` 会复验 SDK headers、output module path、active backend baseline、reserved binding emitter check 和 next steps 是否漂移。该 plan 必须报告 `bindingsGenerated = false`，是 readiness evidence，不是完整 SDK 3.8 Rust bindings 已生成的声明。
- `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-bindings-surface.json` 会生成 generated-bindings symbol surface report；`--check --out <path>` 会复验 SDK headers、required interface/type/constant surface、identifier-token 存在性、active backend baseline 和 audit notes 是否漂移。当前 locked inputs 覆盖 program/unit (`ivstunits.h`) 和 Note Expression (`ivstnoteexpression.h`) headers/symbols。该 surface 必须报告 `bindingsGenerated = false`、`missingSymbols = []`，且每个 required symbol 都必须有 `symbolPresent = true`；它只锁定 future generated-headers emitter 的预期覆盖面，不解析 C++ AST、不验证 ABI、不生成 Rust bindings。
- `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated.rs` 会生成 metadata-only Rust module，并可用 `--check` 逐字节复验漂移。该 scaffold 只记录 header inputs、baseline、active backend 和 `BINDINGS_GENERATED = false`；它不包含 Steinberg VST3 COM/API bindings，也不是 release-check pass evidence。
- `vesty vst3-sdk emit-abi-seed --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi-seed.rs` 会生成 ABI seed Rust module，并可用 `--check` 逐字节复验漂移。该 seed 只记录基础 VST3 ABI aliases/constants、header inputs、baseline、active backend、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`；它不包含 Steinberg VST3 COM/API bindings，也不是 release-check pass evidence。
- `vesty vst3-sdk emit-abi --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi.rs` 会生成 foundational ABI layout Rust module，并可用 `--check` 逐字节复验漂移。该 module 记录基础 VST3 ABI aliases/constants、header inputs、baseline、active backend、`#[repr(C)]` `TUID` / `FUnknownVTable` / `FUnknown` / `ViewRect`，以及 program/unit 和 Note Expression 的基础数据结构 `ProgramListInfo`、`UnitInfo`、`NoteExpressionValueDescription`、`NoteExpressionTypeInfo`、`PhysicalUIMap`、`PhysicalUIMapList`；同时输出 `ABI_LAYOUT_RECORDS` size/alignment 指纹与 `ABI_FIELD_OFFSETS` 关键字段 offset 指纹，并保持 `ABI_LAYOUT_GENERATED = true`、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`。它不包含 Steinberg VST3 COM/API bindings，不等同完整 ABI 验证，也不是 release-check pass evidence。
- `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-interface-skeleton.rs` 会生成 interface/vtable skeleton Rust module，并可用 `--check` 逐字节复验漂移。该 module 记录基础 VST3 ABI aliases/constants、header inputs、baseline、active backend、发现到的 VST3 interface `#[repr(C)]` placeholder / vtable skeleton 和 deterministic method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata，例如 realtime `IAudioProcessor::process`、program/unit 与 Note Expression controller method names/signatures、upstream `vst3 0.3.0` IID words、per-interface `*_IID` constants、`InterfaceId` records、`QueryInterfaceEntry` planned dispatch records、`iid_from_words()` byte-order helper、`QUERY_INTERFACE_IID_LOOKUP_SCOPE`、`interface_id_for_iid()` / `query_interface_entry_by_interface()` / `query_interface_entry_for_iid()` / `com_object_query_interface_dispatch_by_interface()` / `com_object_query_interface_dispatch_for_iid()` 纯查找 helper、当前 adapter 的 `VestyProcessor` / `VestyController` / `VestyPlugView` / `VestyFactory` 等 object-to-interface exposure records、object FUnknown identity records、per-object queryInterface dispatch records、`VestyFactory` class count、processor/controller class index/category/CID derivation 和 `createInstance` dispatch/error policy、`export_vst3!` 的 `GetPluginFactory`、Windows `InitDll`/`ExitDll`、macOS `bundleEntry`/`bundleExit`/compat aliases、Linux `ModuleEntry`/`ModuleExit` entry symbol plan，以及 Windows PE/COFF、macOS Mach-O、Linux ELF 后续静态导出符号检查所需的 expected symbol/tool spelling plan、inspection tool order 和 required-symbol helper seed；`vesty-vst3-sys` 同时暴露 `binary_export_symbol_plans()`、`binary_export_inspection_tools()`、`required_binary_export_tool_symbols()`、`first_missing_binary_export_symbol()` 和 `binary_export_required_symbols_present()`，当前 `vesty-build` / CLI validate/release gates 复用这份 single source of truth。slot 只表示接口内审计顺序，signature 只表示方法签名意图，queryInterface entries 和 lookup helper 只表示 future interface dispatch lookup seed，`COM_OBJECT_INTERFACES` 只表示当前 Vesty adapter 暴露计划，`COM_OBJECT_IDENTITY_PLANS` / `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` 只表示 object identity/dispatch 审计计划，`FACTORY_EXPORT_PLAN` / `FACTORY_CLASS_PLANS` 只表示当前 factory/class export 审计计划，`MODULE_EXPORT_PLANS` 只表示当前 platform module export 审计计划，`BINARY_EXPORT_SYMBOL_PLANS` / `BINARY_EXPORT_INSPECTION_TOOL_PLANS` 只表示 future binary inspection 预期表。它保持 `INTERFACE_SKELETON_GENERATED = true`、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`，不包含 callable `queryInterface` glue、generated factory exports、generated module exports、binary inspection tooling、factory glue、Steinberg method implementations 或完整 COM/API bindings，也不是 release-check pass evidence。
- `vesty release-evidence collect-local --crate-package` 会在本地 evidence 目录额外写入并复验 `crate-package/crate-package.json`；默认不跑，因为它会执行真实 `cargo package` smoke。`vesty release-evidence collect-local --vst3-sdk-dir <official-vst3sdk>` 会在本地 evidence 目录额外写入 `vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json`、`vst3-sdk/generated-bindings-surface.json`、metadata-only `vst3-sdk/generated.rs` scaffold、ABI seed `vst3-sdk/generated-abi-seed.rs`、ABI layout `vst3-sdk/generated-abi.rs` 和 interface skeleton `vst3-sdk/generated-interface-skeleton.rs`，并立即复用 release-check/scaffold/ABI/interface validators 校验；默认不传该参数时不会从环境或 CI 聚合 job 隐式生成 SDK evidence。
- `vesty release-evidence import-ci` 会在 CI artifact 中识别有效的 per-OS release action plan sidecar、crate package report 和 VST3 SDK manifest/plan/surface/scaffold/ABI seed/ABI layout/interface skeleton；有效 action plan sidecar 会复制到 `ci-release-checks/release-action-plan-<OS>.json`，无效 sidecar 会记录为 failed 且不会复制，它们只做 checklist/audit 留档，不作为 release-check pass evidence。`import-ci-report.json` item 会校验 status/source/path 语义: `ok` / `imported` 必须带 output path，`failed` 不能带 output path，`skipped` 只有 destination already exists 时才能带 output path；完整 report 还会校验 artifact `source` 必须位于 report 声明的 CI artifact source root 下，output `path` 必须位于 report 声明的 release evidence dir 下，显式 `ci run url` 文件除外。crate package report 会复制到 `crate-package/crate-package.json`，VST3 SDK artifacts 会分别复制到 `vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json`、`vst3-sdk/generated-bindings-surface.json`、`vst3-sdk/generated.rs`、`vst3-sdk/generated-abi-seed.rs`、`vst3-sdk/generated-abi.rs` 和 `vst3-sdk/generated-interface-skeleton.rs`。VST3 SDK 校验要求 JSON evidence 完整且 `bindingsGenerated = false`，surface 不能包含 `missingSymbols` 或 `symbolPresent = false`，scaffold marker、baseline、complete header metadata 和 `BINDINGS_GENERATED = false`，ABI seed marker、基础 alias/constant surface 和 `FULL_COM_BINDINGS_GENERATED = false`，ABI layout marker、基础 layout surface、program/unit 与 Note Expression foundational struct surface、`ABI_LAYOUT_RECORDS` / `ABI_FIELD_OFFSETS` 指纹、`ABI_LAYOUT_GENERATED = true` 和 `FULL_COM_BINDINGS_GENERATED = false`，interface skeleton marker、interface/vtable skeleton surface、method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata、`INTERFACE_IDS` / `QUERY_INTERFACE_ENTRIES`、`QUERY_INTERFACE_IID_LOOKUP_SCOPE`、`interface_id_for_iid()`、`query_interface_entry_by_interface()`、`query_interface_entry_for_iid()`、`com_object_query_interface_dispatch_by_interface()`、`com_object_query_interface_dispatch_for_iid()`、`COM_OBJECT_INTERFACES`、`COM_OBJECT_INTERFACE_SCOPE`、`COM_OBJECT_IDENTITY_PLANS`、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`、`FACTORY_EXPORT_PLAN`、`FACTORY_CLASS_PLANS`、`MODULE_EXPORT_PLANS`、`BINARY_EXPORT_SYMBOL_PLANS`、`BINARY_EXPORT_INSPECTION_TOOL_PLANS`、`BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED`、`BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`、`binary_export_symbol_plan_by_platform_and_symbol()`、`binary_export_inspection_tools()`、`required_binary_export_symbol_count()`、`first_missing_binary_export_symbol()`、`binary_export_required_symbols_present()`、per-interface `*_IID` constants、`iid_from_words()`、`INTERFACE_SKELETON_GENERATED = true` 和 `FULL_COM_BINDINGS_GENERATED = false`。这些文件只做 readiness/surface/drift/interface/ABI/COM identity/dispatch/exposure-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan 审计留档，不改变 generated bindings 尚未完成的事实，也不替代真实 binary inspection evidence。
- `vesty release-evidence import-ci` 对 platform smoke artifact 也做路径/报告平台一致性检查。路径中明确出现 `macOS`、`Windows` 或 `Linux-X11` token 时，report 的 `platform` 必须一致；不一致会写入 failed item，且不会复制到 `release-evidence/platform-smoke/<platform>.json`。如果路径同时出现多个平台 token，例如 `macos-windows` 或 `linux-x11-windows`，该 artifact 会被视为 ambiguous evidence 并拒绝导入。Linux 路径只有同时包含 `linux` 和 `x11` token 才会被当作 Linux X11 线索。
- `vesty release-evidence import-ci` 对 VST3 validator/static validate artifact 也做明确平台路径一致性检查。文件名或单独父目录表达 `macos`、`windows-x64` 或 `linux-x64` 时，report 的 `static_check.binaries` 必须包含该平台，否则会写入 failed item 并跳过复制。文件名同时表达多个平台也会失败，避免歧义 artifact 被静默规范化。像 `linux-vst3-static-validate/` 这种 CI job 目录名不会被当成强平台信号，所以一个下载目录里包含多平台矩阵 report 仍可正常导入。
- `vesty release-evidence import-ci` 和最终 `vesty release-check` 也会校验签名 evidence 的路径平台一致性。文件名或单独父目录明确表达 `macOS` 或 `Windows` 时，内容必须分别证明 macOS `codesign` 或 Windows `signtool`；例如 `Windows/signing.log` 不能携带 `codesign=pass`，`signing-macos-windows.log` 这种歧义文件名也会失败。macOS signed `.vst3` bundle 如果被放在 `Windows/` 目录下，同样不会作为有效 signed bundle evidence 导入。
- `vesty release-evidence import-ci` 和最终 `vesty release-check` 会把 notarization/stapler evidence 视为 macOS-only。路径文件名或单独父目录明确表达 `Windows` 或 `Linux` 时，即使日志包含 accepted notarytool 与 stapler success marker，也会失败并留下诊断；文件名或父目录同时表达多个平台也会失败，避免 `notary-macos-windows.log` 这类歧义 artifact 被静默归档；`macOS/notary.log`、`darwin/stapler.log` 等 macOS 路径可正常作为 notarization evidence。
- DAW install checks 只表示常见路径存在，不能替代 `daw-matrix` evidence；signing checks 只表示工具存在或 release-channel policy，需要真实签名/公证/安装包 artifact 作为发布证据。
- `vesty release-check --ci-doctor-dir <dir>` 会递归读取 CI 上传/下载的 doctor JSON。它要求 Linux、macOS 和 Windows 三平台都存在，OS 可从文件名或父目录路径推断，因此既支持 `doctor-Linux.json` / `doctor-macOS.json` / `doctor-Windows.json`，也支持 `Linux/doctor.json` / `macOS/doctor.json` / `Windows/doctor.json` 这类下载结构。每个平台 artifact 必须包含 `vst3 SDK headers` check；该 check 可为 `ok` 或 `skipped`，后者表示仍使用 upstream `vst3` backend 且未启用 generated headers。新版 doctor JSON 如果带 `os` 字段，`release-check` 会校验 report OS 和 artifact path 推断 OS 一致，防止 artifact 下载/重命名错位；旧版无 `os` report 仍按 legacy artifact 读取。`--release-evidence-dir` 只有在 `ci-doctor/` 中已有 JSON doctor artifacts 时才自动启用该目录，空模板 README 不会被当成 evidence。`--static-validate-report` 接受 CI static-only packaging smoke JSON 并检查 Vesty 示例覆盖矩阵；`--validate-report` 接受 `vesty validate --strict --report` 生成且 validator passed 的 JSON；`--signed-bundle-evidence` 接受明确通过的 codesign/signtool 日志，或带可解析 `CodeResources` plist 且含 file dictionary 的 macOS signed `.vst3` bundle；`--notarization-log` 接受 notarytool accepted/stapler 日志。
- `vesty release-check --ci-release-check-dir <dir>` 会递归读取 CI 上传/下载的大小写不敏感 `release-check*.json`。它要求 Linux、macOS 和 Windows 三平台都存在，OS 可从文件名或父目录路径推断，因此既支持 `release-check-Linux.json` / `release-check-macOS.json` / `release-check-Windows.json`，也支持 `Linux/release-check.json` / `macOS/release-check.json` / `Windows/release-check.json` 这类下载结构。它确认 host profile coverage、protocol snapshot skip/ok、VST3 binding baseline 等本地 invariant 通过；这些 invariant 不只看 status，还会拒绝重复 check name、伪造 host profile coverage 数量、非 `--skip-protocol` 形态的 protocol skip、缺少当前 Steinberg SDK baseline / upstream `vst3` crate baseline / binding backend 的 binding baseline 值，以及顶层 status 与内部 check status 不自洽的 report。当同时提供 `--ci-run-url` 或 `ci-run-url.txt` 时，它还会要求每个 snapshot 的 `ci_run_url` 来自同一个 GitHub repo/run id。同目录可保留 `release-action-plan-*.json` sidecar，但收集器只采纳 `release-check*.json`，不会把 checklist 当成 per-OS report 或通过证据。DAW smoke、validator、signing 和 notarization 这类外部 evidence 缺口可以继续由对应 release gate 报告，不会让 per-OS snapshot gate 误判为本地 CI 失败。这里允许的 protocol skip 只适用于这些 per-OS snapshots；最终 consolidated release-check 仍必须检查 `--protocol-snapshot`。
- `vesty release-check --platform-smoke-dir <dir>` 会递归读取平台 smoke JSON。report 内的 `platform` 字段仍是权威平台；如果 artifact path 同时带有平台 token，例如 `macOS/platform-smoke.json`、`Windows/platform-smoke.json` 或 `Linux-X11/platform-smoke.json`，路径平台必须与 report platform 一致。路径若同时带有多个平台 token 会被拒绝，避免一个 artifact 用命名混淆污染多个平台覆盖。Linux 路径必须同时包含 `linux` 和 `x11` token 才会被当作 Linux X11 evidence，避免 Wayland 或泛 Linux 文件夹误满足最终 X11 gate。
