# 04. VST3 适配设计

## 绑定策略

第一阶段:

- 使用 `vst3` crate 的 generated bindings、COM pointer 和 interface helper。
- 在 `vesty-vst3` 内部创建安全 wrapper，外部不暴露 unsafe VST3 类型。
- `vesty-vst3-sys` 已作为 binding source 层存在，当前记录 Steinberg SDK / upstream `vst3` crate baseline，并通过 `upstream-vst3` backend 使用 `vst3 0.3.0`。
- 对缺失或行为不稳定的 API，`vesty-vst3-sys` 预留 `generated-headers` backend，并提供 `probe_sdk_headers()` / `probe_sdk_headers_from_env()` 检查官方 SDK checkout 是否包含生成绑定所需的关键 `pluginterfaces` headers。`vesty doctor` 会报告 `VESTY_VST3_SDK_DIR` 的 probe 结果。
- `vesty-vst3-sys` 还提供 `SdkHeaderInputManifest`、`sdk_header_input_manifest()` 和 `check_sdk_header_input_manifest()`；CLI 暴露 `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out target/vst3-sdk-headers.json` 生成 deterministic header 输入快照，并可用 `--check` 防止官方 SDK checkout 漂移。manifest 记录 baseline、generator、version hint、required headers、size、sha256 和 missing headers，并拒绝 symlink/non-file header input。
- `vesty-vst3-sys` 同时提供 `GeneratedBindingsPlan` 和 `generated_bindings_plan(root, bindings_module)`；CLI 暴露 `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --bindings-module target/vst3-sdk/generated.rs --out generated-bindings-plan.json`，可生成/复验 generated-bindings readiness report。该 report 复用 header manifest，检查 SDK input 完整性、`.rs` output module path、active backend baseline 和 reserved binding emitter 状态。
- `vesty-vst3-sys` 同时提供 `GeneratedBindingsSurface` 和 `generated_bindings_surface(root)`；CLI 暴露 `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out generated-bindings-surface.json`，可生成/复验 generated-bindings symbol surface report。该 report 复用 header manifest，锁定未来 generated-headers emitter 预期覆盖的 interface/type/constant 名称、header 来源和用途说明，并对每个 locked header 做 identifier-token 级别检查；缺失符号会写入 `missingSymbols`，对应 symbol 的 `symbolPresent = false`，并把 surface 标为 `blocked`。
- `vesty-vst3-sys` 还提供 `generated_bindings_scaffold(root, bindings_module)`；CLI 暴露 `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated.rs`，生成 deterministic metadata-only Rust module，并可用 `--check` 复验输出漂移。该 scaffold 只记录 header inputs、baseline 和 `BINDINGS_GENERATED = false`，不包含 Steinberg VST3 COM/API bindings。
- `vesty-vst3-sys` 还提供 `generated_bindings_abi_seed(root, bindings_module)`；CLI 暴露 `vesty vst3-sdk emit-abi-seed --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi-seed.rs`，生成 deterministic ABI seed Rust module，并可用 `--check` 复验输出漂移。该 seed 只固定基础 VST3 ABI aliases/constants 和 metadata，例如 `TResult`、`ParamID`、`ParamValue`、`TChar`、`TUID`、result constants 与 platform type strings，同时保持 `BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`。
- `vesty-vst3-sys` 还提供 `generated_bindings_abi(root, bindings_module)`；CLI 暴露 `vesty vst3-sdk emit-abi --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi.rs`，生成 deterministic foundational ABI layout Rust module，并可用 `--check` 复验输出漂移。该 module 固定基础 aliases/constants 与少量 `#[repr(C)]` layouts，例如 `TUID`、`FUnknownVTable`、`FUnknown`、`ViewRect`、`ProgramListInfo`、`UnitInfo`、`NoteExpressionValueDescription`、`NoteExpressionTypeInfo`、`PhysicalUIMap` 和 `PhysicalUIMapList`，同时输出 `ABI_LAYOUT_RECORDS` size/alignment 指纹与 `ABI_FIELD_OFFSETS` 关键字段 offset 指纹，并保持 `ABI_LAYOUT_GENERATED = true`、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`。
- `vesty-vst3-sys` 还提供 `generated_bindings_interface_skeleton(root, bindings_module)`；CLI 暴露 `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-interface-skeleton.rs`，生成 deterministic interface/vtable skeleton Rust module，并可用 `--check` 复验输出漂移。该 module 固定 interface placeholder、vtable skeleton、method-surface/slot-order/signature-intent、vtable slot seed、callback type alias seed、vtable callback field layout seed、vtable field offset fingerprint、upstream `vst3 0.3.0` IID words、per-interface `*_IID` constants、`InterfaceId` records、`QueryInterfaceEntry` planned dispatch records、`iid_from_words()` byte-order helper、当前 `vesty-vst3` adapter 的 `COM_OBJECT_INTERFACES` object-to-interface exposure plan、`COM_OBJECT_IDENTITY_PLANS` object FUnknown identity plan、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` per-object queryInterface dispatch plan、`FACTORY_EXPORT_PLAN` factory export plan、`FACTORY_CLASS_PLANS` processor/controller class plan、`MODULE_EXPORT_PLANS` platform module export plan、`BINARY_EXPORT_SYMBOL_PLANS` per-platform binary export symbol plan 和 `BINARY_EXPORT_INSPECTION_TOOL_PLANS` per-platform inspection tool plan，并保持 `INTERFACE_SKELETON_GENERATED = true`、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`。
- 真正 bindgen/com-scrape 生成完整 Rust bindings 仍是后续阶段；当前 manifest、binding-plan、binding-surface、scaffold、ABI seed、ABI layout 和 interface skeleton 都是生成层的输入锁定/准备度/符号面/输出落点/基础 ABI/interface/COM identity/dispatch/exposure-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan 审计证据，不代表 Vesty 已拥有完整 SDK 3.8 bindings。`GeneratedBindingsPlan.bindingsGenerated` 和 `GeneratedBindingsSurface.bindingsGenerated` 必须保持 `false`，scaffold 必须保持 `BINDINGS_GENERATED = false`，ABI seed、ABI layout 和 interface skeleton 必须保持 `FULL_COM_BINDINGS_GENERATED = false`，直到真正 emitter 和覆盖度验证完成；surface 只做文本 identifier-token 存在性审计，不解析 C++ AST、不生成 Rust bindings，ABI layout 只覆盖少量基础、program/unit 和 Note Expression `repr(C)` 类型及 Rust 侧 size/alignment/field-offset 指纹，interface skeleton 只记录未来 emitter 的接口/vtable/IID/queryInterface dispatch plan、Vesty COM object interface exposure plan、object FUnknown identity plan、per-object queryInterface dispatch plan、factory export plan、processor/controller factory class plan、`export_vst3!` module export plan、future binary inspection expected-name/tool plan 和 required-symbol helper seed 审计 metadata，不生成 callable `queryInterface`、generated factory exports、generated module exports、binary inspection tooling、factory glue 或 Steinberg method implementations。

第二阶段:

- 对比官方 VST3 SDK 3.8.x headers 和 `vst3` crate 覆盖度。
- 如需要，使用 bindgen/com-scrape 重新生成 bindings。
- 生成 API 版本锁定在 workspace，避免上游 crate 小版本导致 ABI 行为变化。

## Factory 和 exports

需要实现:

- `GetPluginFactory`。
- macOS `bundleEntry` / `bundleExit`。
- Windows `InitDll` / `ExitDll` 可选。
- plugin factory metadata。
- processor class 和 controller class 注册。

Factory COM boundary 会拒绝 null `PFactoryInfo` / `PClassInfo` / class id / interface id / instance output pointer；`createInstance()` 在可写 output pointer 上会先置空，失败路径不会把陈旧实例指针留给 host。

Vesty 插件默认一个 Rust library 输出一个插件，也保留一个 binary 导出多个插件 class 的能力。

## Processor 映射

VST3 processor 负责:

- `initialize` / `terminate`。
- `getControllerClassId`，null output pointer 返回 `kInvalidArgument`，避免 host/validator 探测坏指针时穿过 COM 边界崩溃。
- bus 信息和 arrangement。
- `setupProcessing`。
  - 保存 host sample rate / max block size。
  - 首次创建 kernel 时传入 `KernelInit`，随后调用 `AudioKernel::prepare(PrepareContext)`。
  - kernel 已存在时再次收到 `setupProcessing`，不重建 kernel，但会用新的 sample rate / max block size 重新调用 `prepare()`。
- `setActive`。
- `setProcessing`。
- `process`。
- latency: `getLatencySamples()` 读取 `Plugin::latency_samples()`。
- tail: `getTailSamples()` 读取 `Plugin::tail_samples()`。
- state get/set。

Vesty processor wrapper 持有:

- `KernelCell`: 仅 audio thread 使用的 DSP kernel。
- `ParamRuntime`: 参数原子镜像和 automation event list。
- `RtQueues`: meter/event/log queue endpoints。
- `StateMirror`: control thread 传来的 immutable state snapshot。
- `FaultState`: panic 或 fatal error 后的静音/fault 状态。

### process event ordering

`vesty-vst3` 在 `process()` 中先收集 VST3 `IParameterChanges` 和 `IEventList`，再把合并后的固定容量 `FixedEventList` 按 `sample_offset` 做稳定排序，最后交给 `AudioKernel`。

- 排序使用固定 slice 上的插入排序，不创建新的 `Vec`，不扩容。
- 同一个 sample offset 的事件保留收集顺序；同 offset 的同参数自动化点由 `ParamAutomationSegments` 继续按“最后值生效”处理。
- `ParamAutomationSegments` 假设输入事件已经是 sample-order；非 VST3 adapter 调用方如果自行构造 `ProcessContext`，也应遵守这个约定。
- `ProcessData.processMode` 会映射到 `ProcessContext::process_mode()`，覆盖 `Realtime`、`Prefetch` 和 `Offline`，供 kernel 在 offline render 或 prefetch pass 中选择不同质量/缓存策略。
- `IComponent::setIoMode()` 会接受并记录 VST3 标准 `kSimple`、`kAdvanced` 和 `kOfflineProcessing` mode，未知 mode 返回 `kInvalidArgument` 且不污染现有状态；实际每个 block 的 realtime/offline/prefetch DSP 选择仍以 `ProcessData.processMode` 为准。
- `setupProcessing()` 会拒绝 null pointer、unsupported sample size、非有限或非正 sample rate、非正或异常大的 `maxSamplesPerBlock`，并返回 `kInvalidArgument`；无效 setup 不会创建 kernel、不会调用 `prepare()`，也不会为 sample64 scratch 预分配异常大小。
- `IAudioProcessor::process()` 在 sample-size 检查后立即进入 `NoAllocGuard`，因此事件收集、排序、transport mirror、buffer/context 组装和 developer kernel 都被实时分配检测覆盖。
- `IAudioProcessor::setProcessing(false)` 会把 processor 标记为显式非处理状态；如果异常 host 随后仍调用 `process()`，wrapper 会在 `NoAllocGuard` 内清零当前 host output buffers、设置 silence flags、返回 `kResultOk`，且不会进入 developer kernel。默认初始状态仍为 processing active，避免要求所有兼容 host 必须先调用 `setProcessing(true)`。
- kernel 创建和 `prepare()` 发生在 `setupProcessing()` / `setActive(true)` 非实时生命周期；如果异常 host 在缺 kernel 时调用 `process()`，wrapper 会清零输出、设置 silence flags，并推固定结构 `HostWarning` RT log，不在实时区兜底分配。
- `process()` 在构造 host input bus slice 前会校验 `numInputs` 不为负、不超过插件声明 input bus 数量，并要求非零 input count 搭配非 null input pointer；异常输入 bus shape 会清零输出、设置 silence flags、返回 `kResultOk`，且不会进入 developer kernel。
- `process()` 在构造 per-bus channel pointer slice 前会先约束 `AudioBusBuffers::numChannels`: main input/sidechain input 只在 `1..=2` 范围内被转换为 DSP input slice；超出范围的 input bus 会被视为空输入以保持 host 兼容。output bus 还会按插件声明 layout 或 effect main bus mono/stereo 规则做严格 channel-count 校验，异常 output shape 会拒绝本次 output layout、返回 `kResultOk`，且不会进入 developer kernel。
- `process()` 会在进入事件收集和 developer kernel 前校验 block size: `numSamples < 0` 时设置 silence flags、返回 `kResultOk`，且不会进入 developer kernel。超过 sample64 scratch capacity 的默认 f64 fallback 仍由 sample64 路径清零输出并静音；native f64 opt-in kernel 可处理 host 提供的大 block。

### Sidechain MVP

当前 VST3 adapter 支持 effect 插件声明一个 optional sidechain input bus:

- `Plugin::sidechain_inputs()` 默认返回 `0`，老插件保持源码兼容；effect 插件返回 `1` 时，`IComponent::getBusCount(kAudio, kInput)` 从 `1` 变为 `2`。
- audio input bus `0` 是 main bus，`BusTypes::kMain` 且 default active；audio input bus `1` 是 `Sidechain`，`BusTypes::kAux` 且非 default active。
- sidechain arrangement 目前只接受 mono/stereo，默认 stereo；`setBusArrangements()` 对 sidechain effect 接受 `numIns = 1` 或 `2`，`numIns = 2` 时会校验并保存 sidechain arrangement。
- `ProcessData.inputs[0]` 始终映射到 `AudioBuffers` main input；`ProcessData.inputs[1]` 只在插件声明 sidechain 时映射到 `ProcessContext::sidechain()` / `ProcessContext64::sidechain()`，不会混入 main input。
- `kSample64` fallback path 会在 `setupProcessing()` 预分配 sidechain f32 scratch；`process()` 中只做 f64->f32 拷贝，不扩容、不分配。
- instrument 插件不会暴露 audio sidechain input；`vesty-build` 也会拒绝 `[plugin].sidechain = true` 搭配 `[plugin].kind = "instrument"`。

## Controller 映射

VST3 edit controller 负责:

- 参数数量、参数信息、normalized conversion。
- beginEdit/performEdit/endEdit。
- state get/set。
- units/program list metadata。
- editor view 创建。
- processor/controller message 通信。
- 内部 `IAttributeList` wrapper 支持 `int` / `float` / `string` / `binary` attribute set/get，用于非实时 processor/controller message path。字符串按 VST3 `TChar` UTF-16 nul-terminated buffer 处理，`getString` 使用 byte-size capacity 并在截断时强制 nul-terminate；binary getter 返回内部存储指针，生命周期到 attribute list 被 drop 或同 key 被更新为止。

Vesty controller wrapper 持有:

- `ParamRegistry`: 参数 spec 和 stable ID 映射。
- `ControllerState`: 非实时状态和 UI state。
- `EditorFactory`: 创建 `IPlugView` wrapper。
- `HostContext`: host interfaces 的安全薄封装。

### Program List Metadata / Apply / Program Data MVP

当前 controller 已实现 `IUnitInfo` 的静态 metadata + opt-in program apply 路径，并暴露 controller-side `IProgramListData` program data helper:

- `Plugin::program_lists()` 默认返回空 slice；开发者可返回静态 `ProgramList` / `Program` 描述。
- `Plugin::apply_program(list_id, program_index)` 默认返回 `Ok(false)`，保持 metadata-only 兼容；插件 opt-in 返回 `Ok(true)` 时表示已在 controller 非实时路径应用该 program。
- `Plugin::program_data_supported(list_id)` 默认返回 `false`；插件 opt-in 后，`IProgramListData::programDataSupported()` 对有效 list 返回 `kResultTrue`，未 opt-in 返回 `kResultFalse`。
- `Plugin::save_program_data(list_id, program_index)` / `load_program_data(list_id, program_index, data)` 默认不处理；插件 opt-in 后，adapter 会在 controller 非实时路径把 JSON payload 包进 `VESTY_PROGRAM_DATA_V1\n` envelope，并通过 `IBStream` 读写。
- 空 program list 会被过滤，不暴露给 host。
- `getUnitCount()` 固定返回一个 root unit；`getUnitInfo(0)` 返回 root unit、插件名，以及第一个可见 program list id。无 program list 时返回 `kNoProgramListId`。
- `getProgramListCount()`、`getProgramListInfo()` 和 `getProgramName()` 允许 host 查询 list metadata 和 program name。
- `Plugin::program_attributes(list_id, program_index)` 默认返回空 slice；插件可 opt-in 返回静态 `ProgramAttribute`，adapter 通过 `getProgramInfo()` 暴露有效 attribute，并过滤空 id / NUL-containing id/value。
- `Plugin::program_pitch_names(list_id, program_index)` 默认返回空 slice；插件可 opt-in 返回静态 `ProgramPitchName`，adapter 通过 `hasProgramPitchNames()` / `getProgramPitchName()` 暴露有效 MIDI pitch name，并过滤越界 pitch、空 name 和 NUL-containing name。
- 参数 spec 可用 `.as_program_change()` 标记为 host 可见的 program-change 参数；VST3 adapter 会在 `ParameterInfo.flags` 上设置 `kIsProgramChange`，并保持 `kCanAutomate` 由 `automatable` 独立控制。controller/control-thread 上的 `setParamNormalized()` 和内部 edit relay 会把这类参数的 plain value 解释为第一个可见 program list 的 program index；如果 index 有效且插件的 `apply_program()` 返回 `Ok(true)`，adapter 会应用 program、同步该 program-change 参数自身的 normalized value，并以 program delta 通知 host/UI。无可见 program list、index 越界或 `Ok(false)` 时保持旧行为，回退为普通参数写入。audio `process()` 中来自 host 的 program-change 参数 automation 不会套用 program/state；它按普通 sample-accurate 参数事件进入 `ProcessContext::events()`，并只更新 atomic 参数快照。
- `getUnitByBus()` 把当前 MVP 的 main audio/event bus 和 optional sidechain bus 映射到 root unit。
- `setUnitProgramData(data == null)` 会校验 list/root unit id 和 program index；有效 program 会调用 `apply_program()`，`Ok(true)` 返回 `kResultOk`，`Ok(false)` 返回 `kNotImplemented`，`Err(_)` 返回 `kResultFalse`，无效输入返回 `kInvalidArgument`。
- `setUnitProgramData(data != null)` 与 `IProgramListData::setProgramData()` 共用 program data envelope parser；magic/version/list/program mismatch 返回 `kInvalidArgument`，插件 load error 返回 `kResultFalse`。
- opt-in apply/program-data load 成功后，adapter 会 diff 当前参数值，向 host 发 `kParamValuesChanged`，合并 `host_changes_for_param()` 声明的 restart flags，并把参数变化以 `ParamChangeSource::Program` 排队给 Web UI bridge。

这意味着当前能力适合“让 host 看到 factory/program 名称、静态 program attributes / pitch names、program-change 参数 metadata，并让插件在 controller/control thread 把静态 program 或 per-program JSON data 应用到参数/state”。`examples/midi-synth` 已提供一个具体示例: `program` choice 参数标记为 `.as_program_change()`，暴露 factory program list/attributes/pitch names，并通过 `apply_program()` / `save_program_data()` / `load_program_data()` 在 controller 非实时路径切换和保存 program level。它仍不能替代完整跨 DAW host preset/program 验收: audio `process()` 内的 sample-accurate program-change automation 被有意限定为普通参数 automation，不会自动套用 program/state；真实 DAW program workflow evidence 仍缺。program apply/data load 继续限定在 controller 非实时路径。

## 参数

Vesty 参数 ID:

- 对用户暴露字符串 ID。
- VST3 adapter 会从字符串 ID 派生稳定正数 31-bit VST3 `ParamID`，并在 controller/processor 两侧用同一映射表做 host ID <-> 本地参数 index 转换。
- 参数重排不会改变 DAW 看到的 `ParamID`；host automation 进入 `process()` 时会先用稳定 `ParamID` 查回本地 `ParamHandle`。
- 如果参数 schema 无效，或两个参数字符串 ID 派生出相同 VST3 `ParamID`，VST3 factory 创建 processor/controller 时都会拒绝该插件实例，避免出现 processor 可加载但 controller/host automation 参数映射不可靠的半初始化状态。
- `vesty-params::stable_vst3_param_id()` 是 VST3 adapter 和打包 metadata 共用的唯一算法源；算法名为 `vesty.vst3.param.fnv1a31-positive.v2`。算法保留 FNV-1a 输入命名空间，但清除最高位并避免 0，防止 Steinberg validator 或 host 把 high-bit `ParamID` 解释成负数 invalid ID。
- 打包器支持可选 `Contents/Resources/parameters.manifest.json` 侧车文件，记录字符串 ID、VST3 数值 ID 和完整 `ParamSpec` 显示 metadata；该文件只在 `[package].parameter_manifest` 指向显式 JSON 时复制并校验。`vesty param-manifest --specs params.specs.json --out vesty-parameters.json` 可从显式 `ParamSpec` JSON 生成该 sidecar；当前不会从已编译二进制自动 introspect 参数 schema。
- 编译期检测重复 ID。

normalized 转换:

```text
plain value <-> distribution <-> normalized 0.0..=1.0 <-> VST3 ParamValue
```

自动化规则:

- UI 改参数必须包在 beginEdit/endEdit 中。
- host 回放自动化以 process data 为准。
- controller 的 `setParamNormalized` 只更新 controller/UI 侧显示，不直接绕过 host 写 DSP；program-change 参数是例外的 controller-side host/program 语义，它只在非实时 controller/control thread 尝试调用 opt-in `apply_program()`，不会在 audio `process()` 中自动套用 program/state。
- meter 等 read-only 参数不设 `kCanAutomate`。
- adapter 会在 `setParamNormalized` 和内部 begin/perform/end edit helper 上拒绝 `read_only` 参数写入，避免 host/UI 绕过 metadata 约束。
- bypass 参数设置 VST3 bypass flag，effect 插件默认提供。
- VST3 controller 的参数 parse 会把 host `String128` 输入限制在 128 个 UTF-16 单元内读取；非 NUL 结尾输入不会触发无界扫描。
- 参数可以通过 `ParamSpec::with_midi_mapping(controller, channel)`、`.with_midi_cc(controller)` 和 `.with_channel_midi_cc(controller, channel)` 声明 host MIDI mapping；`FloatParam` / `BoolParam` / `ChoiceParam` 也提供同名 builder。
- `ParamSpec.midi_mappings` 会导出到 JSBridge wire schema 的 `midiMappings` 字段，并由 `validate_param_specs()` 校验 controller 范围、channel 范围和同一参数内的重复 mapping。
- VST3 controller 实现 `IMidiMapping::getMidiControllerAssignment()`，当前仅对 main event input bus `0` 返回 opt-in mapping；只映射 `automatable && !read_only` 参数。channel 为 `None` 时匹配所有 MIDI channel，`Some(0..=15)` 时只匹配指定 channel。

## MIDI/Event

Vesty 事件类型:

```rust
pub enum Event {
    NoteOn { sample_offset: u32, channel: u16, key: u8, velocity: f32, note_id: i32 },
    NoteOff { sample_offset: u32, channel: u16, key: u8, velocity: f32, note_id: i32 },
    PolyPressure { sample_offset: u32, channel: u16, key: u8, pressure: f32, note_id: i32 },
    MidiCc { sample_offset: u32, channel: u16, controller: u16, value: f32 },
    PitchBend { sample_offset: u32, channel: u16, value: f32 },
    ChannelPressure { sample_offset: u32, channel: u16, pressure: f32 },
    SysEx { sample_offset: u32, data_len: u16, data: [u8; MAX_SYSEX_BYTES], truncated: bool },
    NoteExpressionValue { sample_offset: u32, type_id: u32, note_id: i32, value: f64 },
    NoteExpressionInt { sample_offset: u32, type_id: u32, note_id: i32, value: u64 },
    NoteExpressionText { sample_offset: u32, type_id: u32, note_id: i32, text_len: u8, text: [u16; MAX_NOTE_EXPRESSION_TEXT_UNITS] },
    Param { sample_offset: u32, handle: ParamHandle, id_hash: u32, normalized: f64 },
}
```

MVP:

- NoteOn/NoteOff/PolyPressure。
- VST3 legacy MIDI CC event 映射为 `MidiCc`，并将 VST3 `kPitchBend` / `kAfterTouch` 映射为 `PitchBend` / `ChannelPressure`。
- VST3 `kDataEvent` 且 data type 为 `kMidiSysEx` 时映射为 `SysEx`；host bytes 会复制到固定 `[u8; MAX_SYSEX_BYTES]` buffer，当前上限为 256 bytes，超长或 null bytes + non-zero size 会设置 `truncated = true`，不把 host 裸指针暴露给 DSP。
- VST3 `kNoteExpressionValueEvent` / `kNoteExpressionIntValueEvent` / `kNoteExpressionTextEvent` 分别映射为 `NoteExpressionValue`、`NoteExpressionInt` 和 `NoteExpressionText`，保留 `type_id`、`note_id` 和 value/text payload；text payload 复制到固定 UTF-16 buffer，长度上限为 `MAX_NOTE_EXPRESSION_TEXT_UNITS`，不在 realtime path 分配内存。`vesty_core::note_expression` 提供 VOLUME/PAN/TUNING/VIBRATO/EXPRESSION/BRIGHTNESS/TEXT/PHONEME 等标准 type id 常量。
- `Plugin::note_expression_value_types()` 可 opt-in 暴露静态 `NoteExpressionValueType` metadata；VST3 controller 实现 `INoteExpressionController`，只对 instrument event input bus `0` / channel `-1..=15` 返回有效的 expression type info，并提供 conservative normalized string/value conversion；host `String128` 输入同样被限制在 128 个 UTF-16 单元内读取。
- `Plugin::note_expression_physical_ui_mappings()` 可 opt-in 暴露静态 `NoteExpressionPhysicalUiMapping` metadata；VST3 controller 实现 `INoteExpressionPhysicalUIMapping`，只返回指向已声明有效 expression type 的 X/Y/Pressure physical mapping。
- MIDI CC 到参数的 host mapping 已通过 `IMidiMapping` 暴露给 host。
- Pitch bend/channel pressure 可作为参数 mapping；使用 `vesty_params::midi::PITCH_BEND` / `CHANNEL_PRESSURE` 等常量声明。

- `examples/midi-synth` 已展示 developer-facing DSP 消费方式: 固定 SysEx `[F0, 7D, level, F7]` 更新 kernel 内部 level override，`note_expression::BRIGHTNESS` / `TUNING` 更新当前 active note 的音色/音高偏移；该示例单测直接构造 `ProcessContext` 验证事件消费，不写 controller state、不走 JSON、不在 realtime path 分配。

后续:

- Note Expression 自定义 expression editor workflow 和真实 DAW SysEx/expression evidence。
- MIDI 2.0 mapping。

## Bus 和 layout

MVP 支持:

- audio effect: mono->mono、stereo->stereo、mono->stereo。
- audio effect optional sidechain: 最多一个 mono/stereo aux input bus，独立暴露到 `ProcessContext::sidechain()`；如果 host 在 process 阶段没有提供 sidechain bus，或提供 trailing empty inactive sidechain bus，DSP 会看到空 sidechain input slice，main input/output 仍正常处理。
- instrument: event input + one or more mono/stereo output buses declared through `Plugin::output_buses()`；bus 0 是 main/default active，后续 bus 是 aux/non-default-active。
- `IComponent::getRoutingInfo()` 对当前路由模型返回保守 main route: effect 的 main audio input bus `0` 映射到 main audio output bus `0` / all channels，instrument 的 main event input bus `0` 映射到 main audio output bus `0` / all channels。非法 media type、bus index、channel 或 null pointer 返回 `kInvalidArgument`。
- `IComponent::activateBus()` 会验证 media type、direction 和 bus index，只接受当前声明存在的 audio input/output 或 instrument event input bus；不存在的 bus 返回 `kInvalidArgument`。adapter 会记录 host 激活状态，默认 main input/output 和 instrument event input active，sidechain/aux output inactive。
- `setBusArrangements()` 仍按完整声明 bus list 协商，防止 host 和插件对 layout 的理解漂移；`process()` 阶段则以 host 实际传入的 `ProcessData.outputs` 为准，允许 output bus 前缀和 trailing empty inactive aux bus。这样未激活 aux output 不会导致 main bus 整块处理被跳过，同时已提供的 aux/instrument output bus 仍按声明声道严格校验。
- `IAudioProcessor::canProcessSampleSize()` 支持 VST3 `kSample32` 和 `kSample64`。
  - 默认开发者-facing DSP API 仍是 f32 `AudioBuffers` / `ProcessContext`；VST3 adapter 在 `setupProcessing()` 非实时阶段预分配 f32 scratch，64-bit host block 在实时 `process()` 中做 f64->f32、调用现有 `AudioKernel::process()`，再 f32->f64 回写，不在实时路径扩容。
  - 需要原生 double-precision DSP 的插件可显式设置 `AudioKernel::SUPPORTS_F64 = true` 并实现 `AudioKernel::process_f64(&mut ProcessContext64)`；此时 `kSample64` 直接把 host f64 buffers 暴露为 `AudioBuffers64`，不走 f32 scratch fallback。
  - fallback scratch capacity 不足或缺少 setup lifecycle 时静音返回；native f64 路径不依赖 scratch capacity，但仍要求 developer `process_f64()` 遵守 realtime path 规则。
- silence flags 基础处理。
  - kernel 返回 `ProcessResult::Silence` 或 panic/fault fallback 时清零输出，并把每个输出通道对应的 VST3 `AudioBusBuffers::silenceFlags` bit 置位。
  - kernel 正常 `Continue` 时清掉旧的 output silence flags，避免 host 看到 stale silence state。
- host 请求 unsupported bus arrangement 时返回 false，并报告当前支持 arrangement。

后续:

- sidechain real-host smoke、multi-sidechain 和 activation-aware processing policy。
- multi-output instrument real-host routing smoke。
- surround/immersive formats。
- activation state 与真实 DAW bus routing/offline render 行为的兼容性矩阵。

## Editor View

VST3 editor 适配要点:

- `IPlugView::attached(parent, platform_type)` 中把 parent 转成平台 handle。
  - null `platform_type` 会被视为 unsupported platform；supported platform 但 null native parent handle 会返回 `kResultFalse`，不会创建 editor runtime。
- `IPlugView::removed` 释放 WebView。
- `IPlugView::onSize` 更新 child bounds。
- `IPlugView::getSize` 返回当前 logical/physical 尺寸。
- `IPlugView::canResize` 根据 `UiDescriptor` 判断。
- `IPlugView::checkSizeConstraint` 应用 min/max/aspect constraints。

平台 parent:

- Windows: HWND。
- macOS: NSView。
- Linux: X11 window id 或 GTK container 路线。

## State

VST3 state 分两层:

- processor state: DSP 参数、kernel state、latency 相关状态。
- controller state: UI 状态、参数显示、用户配置。

Vesty 对开发者暴露:

```rust
trait PluginState {
    type State: Serialize + DeserializeOwned + 'static;
    fn save_state(&self) -> Self::State;
    fn load_state(&self, state: Self::State) -> Result<(), StateError>;
}
```

内部:

- 使用 `VESTY_STATE_V1\n` magic + JSON payload。
- payload 包含 `version`、`params` 和可选 `custom`。
- 旧 payload 缺少 `custom` 时按 `None` 处理。
- 支持 migration。
- state restore 在 control thread 完成解析，audio thread 只接收已验证 snapshot。

## Latency / Tail

当前实现:

- `Plugin::latency_samples()` 默认返回 0，插件可覆盖为固定 latency。
- `Plugin::tail_samples()` 默认返回 0，插件可覆盖为固定 tail。
- VST3 `IAudioProcessor::getLatencySamples()` / `getTailSamples()` 直接读取上述 hook。
- `Plugin::host_changes_for_param(id, old, new)` 可返回 `HostChangeFlags::LATENCY`。
- controller/control thread 上的 `IEditController::setParamNormalized()`、内部 param gesture relay 和 wry UI gesture relay 会把 `HostChangeFlags::LATENCY` 映射为 VST3 `RestartFlags_::kLatencyChanged`，通过 `IComponentHandler::restartComponent()` 通知 host。

后续增强:

- 在更多真实 DAW 中验证运行中 latency 变化后的 restart/setup 刷新行为。
- 如果后续 AU/CLAP wrapper 落地，复用 `HostChangeFlags` 做 wrapper-specific host notification。

## Validator 策略

`vesty validate` 默认寻找 Steinberg validator:

- 环境变量 `VST3_VALIDATOR`。
- SDK 常见 build output。
- 用户传入 `--validator`。

检查:

- factory metadata。
- processor/controller 创建和生命周期。
- 参数 ID 稳定性。
- bus arrangement。
- state restore。
- UI view attach/detach smoke test。
