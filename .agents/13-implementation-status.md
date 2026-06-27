# Vesty 当前实现状态

更新时间: 2026-06-13

## 当前阶段

Vesty 已进入 alpha skeleton 状态: workspace、核心 API、VST3 adapter、wry UI runtime、JSBridge、CLI、打包和三个示例插件都已落地。当前代码可以在本机完成 Rust/TypeScript 编译、单元测试、feature check、headless `vesty smoke-host` self-check、macOS `.vst3` 打包 smoke，并通过 Steinberg VST3 validator。

还不能宣称达到完整发布验收: 真实 DAW smoke matrix 尚未完成。因此“多 DAW 兼容性”仍是外部验证项。

## 已实现

- Cargo workspace: `vesty` facade、`vesty-core`、`vesty-params`、`vesty-rt`、`vesty-vst3`、`vesty-vst3-sys`、`vesty-ipc`、`vesty-bridge`、`vesty-ui`、`vesty-ui-wry`、`vesty-build`、`vesty-cli`、`vesty-macros`。
- JS package: `packages/plugin-ui`，提供 framework-agnostic bridge API。
  - package build 产物为 `packages/plugin-ui/dist/index.js` 和 `packages/plugin-ui/dist/index.d.ts`。
  - `createSnapshotStore(bridge)` 提供 framework-agnostic snapshot external store，已被 React/Vue/Svelte 薄适配复用。
- JS framework adapters:
  - `packages/react` 发布名 `@vesty/react`，提供 `VestyBridgeProvider`、`useVestyBridge()`、`useVestySnapshotStore()`、`useVestySnapshot()` 和 `useVestyParamEdit()`。
  - `packages/vue` 发布名 `@vesty/vue`，提供 `useVestySnapshot()` 和 `useVestyParamEdit()` composables。
  - `packages/svelte` 发布名 `@vesty/svelte`，提供 `vestySnapshotStore()` 和 `vestyParamEdit()` stores/helpers。
  - 三个 adapter 只依赖 `@vesty/plugin-ui` bridge/store/param gesture API，不引入额外 native runtime。
- Examples: `examples/gain`、`examples/midi-synth`、`examples/web-ui-param-demo`。
  - `gain` 是 headless effect 示例。
  - `midi-synth` 是 headless instrument 示例，同时展示 VST3 program list metadata、host-visible program-change 参数、program attributes/pitch names、controller-side program data JSON roundtrip、固定 SysEx level override 和 Note Expression brightness/tuning DSP 消费路径。
  - `web-ui-param-demo` 是 wry/Web UI + JSBridge 示例，`ui/src`、`ui/scripts/build.mjs` 和 `ui/scripts/dev.mjs` 可重建 `ui/dist`，与 `vesty.toml` 的 `build = "npm run build"` 对齐。
- Host profile/quirk registry:
  - `vesty-core` 提供 `HostProfile`、`HostQuirk`、`HostQuirkArea`、`HostQuirkSeverity`、`host_profiles()`、`find_host_profile()` 和 `RELEASE_SMOKE_CHECKS`。
  - 内置 REAPER、Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One 五个 release smoke 目标 profile。
  - profile 只描述验证计划和注意事项，不表示对应 DAW 已通过兼容性验收。
- VST3 binding source 层:
  - `vesty-vst3-sys` 固定 Steinberg VST3 SDK baseline `v3.8.0_build_66` 和 upstream `vst3` crate baseline `0.3.0`。
  - 当前 backend 仍是 upstream `vst3` crate；`generated-headers` backend 已预留，用于后续从官方 SDK headers 生成缺失 bindings。
  - `vesty-vst3-sys` 已提供 generated headers 前置 probe: `REQUIRED_GENERATED_HEADER_INPUTS`、`probe_sdk_headers()` 和 `probe_sdk_headers_from_env()` 会检查官方 SDK checkout 中关键 `pluginterfaces` headers 是否齐全。当前 locked inputs 包含 program/unit (`pluginterfaces/vst/ivstunits.h`) 和 Note Expression (`pluginterfaces/vst/ivstnoteexpression.h`) headers，以覆盖后续 VST3 SDK backend 需要审计的 unit/program list 与 expression controller surface。
  - `vesty-vst3-sys` 已提供 generated headers 输入锁定 manifest: `SdkHeaderInputManifest`、`sdk_header_input_manifest()` 和 `check_sdk_header_input_manifest()` 记录 required headers、size、sha256、baseline、generator、version hint 和 missing headers；CLI 可用 `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out <json>` 生成并用 `--check` 复验。
  - `vesty-vst3-sys` 已提供 generated-bindings readiness report: `GeneratedBindingsPlan` 和 `generated_bindings_plan()` 复用 header manifest，并检查 output module `.rs` 路径、active backend baseline 和 reserved binding emitter 状态；CLI 可用 `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --bindings-module target/vst3-sdk/generated.rs --out <json>` 生成并用 `--check` 复验。该 report 必须保持 `bindingsGenerated = false`，不宣称完整 SDK 3.8 bindings 已生成。
  - `vesty-vst3-sys` 已提供 generated-bindings symbol surface report: `GeneratedBindingsSurface` 和 `generated_bindings_surface()` 复用 header manifest，并检查 required symbol/header surface、identifier-token 存在性、active backend baseline 和 audit notes；CLI 可用 `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out <json>` 生成并用 `--check` 复验。该 report 必须保持 `bindingsGenerated = false`、`missingSymbols = []` 且所有 required symbol `symbolPresent = true`；它只锁定 future emitter 的文本 token 审计面，不解析 C++ AST、不验证 ABI、不生成 Rust bindings。
  - `vesty-vst3-sys` 已提供 deterministic metadata-only scaffold emitter: `GeneratedBindingsScaffold` 和 `generated_bindings_scaffold()` 会在 SDK header inputs 和 `.rs` output module path 都 ready 时生成 `generated.rs` scaffold；CLI 可用 `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated.rs` 生成并用 `--check` 逐字节复验。该 scaffold 只记录 generator、header inputs、baseline、active backend 和 `BINDINGS_GENERATED = false`，不包含 Steinberg VST3 COM/API bindings。
  - `vesty-vst3-sys` 已提供 deterministic ABI seed emitter: `GeneratedBindingsAbiSeed` 和 `generated_bindings_abi_seed()` 会在 SDK header inputs、binding plan 和 symbol surface 都 ready 时生成 `generated-abi-seed.rs`；CLI 可用 `vesty vst3-sdk emit-abi-seed --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi-seed.rs` 生成并用 `--check` 逐字节复验。该 seed 只记录基础 VST3 ABI aliases/constants、generator、header inputs、baseline、active backend、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`，不包含 Steinberg VST3 COM/API bindings。
  - `vesty-vst3-sys` 已提供 deterministic foundational ABI layout emitter: `GeneratedBindingsAbi` 和 `generated_bindings_abi()` 会在 SDK header inputs、binding plan 和 symbol surface 都 ready 时生成 `generated-abi.rs`；CLI 可用 `vesty vst3-sdk emit-abi --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi.rs` 生成并用 `--check` 逐字节复验。该 module 记录基础 VST3 ABI aliases/constants 和少量 `#[repr(C)]` layout，例如 `TUID`、`FUnknownVTable`、`FUnknown`、`ViewRect`、`ProgramListInfo`、`UnitInfo`、`NoteExpressionValueDescription`、`NoteExpressionTypeInfo`、`PhysicalUIMap` 和 `PhysicalUIMapList`，同时输出 `ABI_LAYOUT_RECORDS` size/alignment 指纹与 `ABI_FIELD_OFFSETS` 关键字段 offset 指纹；它保持 `ABI_LAYOUT_GENERATED = true`、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`，不包含 Steinberg VST3 COM/API bindings，也不等同完整 ABI 验证。
  - `vesty-vst3-sys` 已提供 deterministic interface/vtable skeleton emitter: `GeneratedBindingsInterfaceSkeleton` 和 `generated_bindings_interface_skeleton()` 会在 SDK header inputs、binding plan 和 symbol surface 都 ready 时生成 `generated-interface-skeleton.rs`；CLI 可用 `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-interface-skeleton.rs` 生成并用 `--check` 逐字节复验。该 module 只记录基础 ABI aliases/constants、generator、header inputs、baseline、active backend、interface placeholder、vtable skeleton 和 deterministic method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata，包括 upstream `vst3 0.3.0` IID words、per-interface `*_IID` constants、`InterfaceId` records、`QueryInterfaceEntry` planned dispatch records、`iid_from_words()` byte-order helper、`QUERY_INTERFACE_IID_LOOKUP_SCOPE`、`interface_id_for_iid()` / `query_interface_entry_by_interface()` / `query_interface_entry_for_iid()` / `com_object_query_interface_dispatch_by_interface()` / `com_object_query_interface_dispatch_for_iid()` 纯查找 helper、当前 Vesty adapter object-to-interface exposure records、object FUnknown identity records、per-object queryInterface dispatch records、`VestyFactory` export plan、processor/controller class plans、`export_vst3!` platform module export plans、per-platform expected binary export symbol/tool plans 和 `binary_export_symbol_plan_by_platform_and_symbol()` / `binary_export_inspection_tools()` / `required_binary_export_symbol_count()` / `first_missing_binary_export_symbol()` / `binary_export_required_symbols_present()` 纯 required-symbol helpers；它保持 `INTERFACE_SKELETON_GENERATED = true`、`BINDINGS_GENERATED = false`、`FULL_COM_BINDINGS_GENERATED = false` 和 `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`，不包含 callable `queryInterface` glue、generated factory exports、generated module exports、binary inspection tooling、factory glue、Steinberg method implementations 或完整 VST3 COM/API bindings。
  - `vesty-vst3::binding_baseline()` 对外报告当前 binding baseline；`vesty doctor` 已输出同等 baseline，并且 CI doctor artifact gate 会要求该 check 存在。
- `#[derive(Params)]`:
  - `vesty-macros` 已生成 `ParamCollection` impl。
  - 当前支持 `FloatParam` / `BoolParam` / `ChoiceParam` 字段，并支持 `#[param(skip)]` 忽略非参数字段。
  - `vesty::prelude::*` 已导出 `Params` derive。
  - 三个示例插件和 `vesty new` 模板已切换为 derive-first 参数定义。
- `PluginState`:
  - `vesty-core` 提供 `StateError`、`PluginState`、`save_plugin_state()`、`load_plugin_state()`。
  - `Plugin` trait 提供默认 no-op 的 `save_custom_state()` / `load_custom_state()` hook。
  - `vesty-vst3` 的 getState/setState 已接入 custom JSON payload。
- Latency/tail:
  - `Plugin` trait 提供默认 0 的 `latency_samples()` / `tail_samples()` hook。
  - `vesty-vst3` 的 `IAudioProcessor::getLatencySamples()` / `getTailSamples()` 已读取对应 hook。
  - `Plugin::host_changes_for_param()` 可返回 `HostChangeFlags::LATENCY`；VST3 controller/control thread 的参数 set/gesture path 会映射为 `restartComponent(kLatencyChanged)`。
- VST3 exports: `export_vst3!` 生成 macOS `bundleEntry`/`bundleExit`、兼容 `BundleEntry`/`BundleExit`、`GetPluginFactory` 等平台入口。
- VST3 adapter:
  - factory class metadata；generated SDK audit skeleton 也锁定 factory export/class plan metadata。
  - processor/controller split。
  - mono/stereo MVP bus arrangement negotiation: effect 支持 mono->mono、mono->stereo、stereo->stereo；instrument 支持 event input + stereo output；multi-output instrument MVP 已接入 `Plugin::output_buses()`，VST3 可暴露 main + aux output buses，sample32/native sample64/scratch sample64 process path 会以固定容量栈结构展平多 bus 输出且不在实时区分配。
  - parameter registry、format/parse、state get/set。
  - read-only 参数在 VST3 metadata 中不暴露 `kCanAutomate`，并且 adapter 的 `setParamNormalized` 与内部 begin/perform/end edit helper 会拒绝写入；bypass 参数暴露 VST3 `kIsBypass`。
  - VST3 state `params + custom + bridge` payload；`custom` 保留给插件开发者，`bridge` 独立保存 JSBridge `PluginSnapshot` 的 config/UI state；旧 state 缺 `custom` 或 `bridge` 时向后兼容。
  - sample-accurate automation 转换。
  - NoteOn/NoteOff/PolyPressure、legacy MIDI CC、PitchBend 和 ChannelPressure 转换。
  - transport mirror。
  - output silence flags: `ProcessResult::Silence` / faulted silence 会清零输出并设置 VST3 output channel silence bits；正常 `Continue` 会清除 stale silence flags。
  - processor/controller `IConnectionPoint` connect/disconnect/notify，以及用于 telemetry 绑定的内部 message。
  - controller 暴露 `IUnitInfo` program MVP: root unit、静态 `Plugin::program_lists()`、program list count/info/name、valid bus -> root unit 映射，以及 opt-in `Plugin::apply_program()` 支持；有效 host program selection 会在插件返回 `Ok(true)` 时应用到参数/state 并通知 host/UI，默认 `Ok(false)` 仍保持 metadata-only；`kIsProgramChange` 参数在 controller/control-thread 的 `setParamNormalized()` 和 edit relay 中会把 plain value 映射为第一个可见 program list 的 index，成功应用后参数变化以 `source = "program"` 进入 Web UI。
  - panic guard/faulted silence。
  - `IPlugView` 基础尺寸和约束。
- UI descriptor API:
  - `UiDescriptor::web_assets("ui")` 提供 release assets 默认描述。
  - `with_dev_url()`、`with_size()`、`with_min_size()` 和 `with_resizable()` 提供链式 editor URL/尺寸/resize 约束配置，避免开发者为了改尺寸直接写完整 struct literal。
- wry UI runtime:
  - macOS NSView、Windows HWND、Linux X11 parent handle。
  - attach/detach/resize。
  - dev URL 和 release asset custom protocol。
  - release asset custom protocol 必须加载 `assets.manifest.json`；加载失败时 editor attach 返回 runtime unavailable。运行时加载会先校验 entry、files、path、mime、sha256 和重复 path，其中 manifest MIME 必须能作为 HTTP `Content-Type` header value；加载成功后只服务 manifest 内 path，并继续做 canonical root 检查，同时校验 manifest `size`/`sha256`。
  - release asset mode 默认只允许 bundle asset URL 导航，拒绝 remote navigation、download 和 `window.open`；release IPC handler 只接受 bundle asset URL；HTML 响应带默认 CSP 和 `X-Content-Type-Options: nosniff`。
  - release asset custom protocol response 构造不依赖 `expect()`；异常 asset、篡改 asset 或非法 manifest metadata fail closed，不让 response builder panic 穿过 WebView request callback。
  - native IPC callback 使用 `catch_unwind` 包住 bridge handler；handler panic 时不穿过 WebView/host UI callback，parseable request 会收到 retryable `internal_error`。
  - bridge bootstrap injection。
- JSBridge:
  - packet envelope。
  - hello/ready。
  - `bridge.hello` typed payload: JS SDK 和 wry bootstrap 会发送 `supportedProtocolVersions`、JS package/bootstrap version 和 page URL；Rust bridge runtime 校验 payload schema，并要求版本列表包含当前协议版本 1。
  - `ready()` 幂等: `@vesty/plugin-ui` 和 wry bootstrap 在同一个 bridge instance 内会复用并发 ready promise；成功后缓存 ready payload，后续 `ready()` 不再重复发送 `bridge.hello` / `bridge.readyAck`，失败时清空 promise 以允许重试。
  - `bridge.ready` capabilities: ready payload 暴露 `paramGestures`、`paramFormatParse`、`stateConfig`、`subscriptions`、`meterStream` 和 `reliableEvents`，供 Web UI feature gating。
  - `@vesty/plugin-ui` 现在把 Rust IPC 生成的协议 TypeScript snapshot 打包到 `dist/protocol`，并通过 `@vesty/plugin-ui/protocol` 子路径暴露；顶层 SDK 继续 re-export 常用协议类型，React/Vue/Svelte 薄适配仍复用顶层类型。
  - snapshot、state config、persistent UI state。
  - `state.setConfig` 使用 `baseRevision` 做 config revision 冲突检测，冲突时返回 `state_conflict` 并附最新 snapshot。
  - `state.setUiState` 使用 `baseRevision` 做 UI revision 冲突检测，成功后整块替换 `PluginSnapshot.uiState`，冲突时返回 `state_conflict` 并附最新 snapshot。
  - `state.setConfig` / `state.setUiState` 成功提交后会向订阅了 `state.changed` 的 UI 发送完整 `PluginSnapshot`，让 `createSnapshotStore()` 和 React/Vue/Svelte adapter store 不需要额外 `snapshot.get` 就能同步；VST3 controller `getState()` 会把最新 bridge snapshot 写入独立 `bridge` 字段，重新 `setState()` 后新的 editor handshake 会恢复该 snapshot；若 UI 已经打开，controller `setState()` / `setComponentState()` 会通过带 generation 的共享 snapshot 同步到 active wry bridge runtime，并在下一次 IPC / `event.flush` 向订阅 UI 推送 `state.changed`。
  - param begin/perform/end；`beginParamEdit(id, gestureId?)`、`performParamEdit(id, normalized, gestureId?)`、`endParamEdit(id, gestureId?)` 和便捷 `setParam(id, normalized, gestureId?)` 都可携带同一个 optional gesture token。
  - read-only 参数的 `param.begin` / `param.perform` / `param.end` 会返回 `permission_denied`，但 `param.format` / `param.parse` 仍允许用于只读 meter/analyzer 显示。
  - `param.changed` event: wry bridge relay 的 `param.perform` 被 host `IComponentHandler::performEdit()` 接受后，会向订阅了 `param.changed` 的 UI 回推 confirmed value、plain/display、source、gestureId 和 revision；host/controller 侧 `setParamNormalized()` 与 state restore 也会进入 controller pending queue，并由 `event.flush` 以 `source = "host"` / `source = "state"` 推给订阅 UI。
  - param format/parse。
  - subscription/backpressure 基础模型。
  - reliable event 只向已订阅 topic 推送。
  - meter latest-wins 队列: 未订阅时丢弃，高频同 topic 只保留最新帧，flush 时通过 `BridgeLane::Meter` 批量发送。
  - `MeterFrame` -> bridge payload 转换: UI/control 线程可把 RT meter frame 推入 latest-wins 队列。
  - `BridgeRuntime::drain_param_gestures()` typed gesture 队列。
  - IPC message size limit: `vesty-ipc` 对原始 JSON 设置 256 KiB 绝对上限；`vesty-bridge` 对 state lane 使用 256 KiB，对其它 JS -> Rust lane 使用 64 KiB，超限 request 返回 retryable `backpressure`。
  - Bridge control queue backpressure: subscription table 限制 256 个 topic、topic 128 bytes；pending param gesture 队列限制 1024 条；同一参数在 begin/end gesture 边界内的连续 `param.perform` 使用 latest-wins coalescing；超限返回 retryable `backpressure`；`param.end` 满队列时会优先丢弃旧 perform 并通过 diagnostics 暴露 `droppedParamGestures` 累计计数。
  - JS SDK 与 wry bootstrap 的内部 request 入口会校验 packet type: 必须是 string、非空、最长 128 UTF-8 bytes、不能包含控制字符；无效 generic `request(type, payload)` 返回 non-retryable `validation_error`，不会创建 pending request 或发送 IPC。
  - wry bootstrap 暴露的 fallback `window.__VESTY_INTERNAL__.request(type, lane, payload)` 也会校验 lane 必须是已知 `BridgeLane` 枚举值；非法 lane 返回 non-retryable `validation_error`，不会创建 pending request、启动 timeout 或发送 `postMessage`。
  - wry bootstrap 暴露的 fallback `window.__VESTY_INTERNAL__.setSession(value)` 使用同一套 session guard，要求非空、最长 128 UTF-8 bytes 且无控制字符，避免页面脚本把内部 session 改成畸形值后污染后续 request。
  - Rust native `BridgeRuntime` 也会权威校验 packet type: 直接伪造 IPC 的空/超长/控制字符 type 会返回 non-retryable `validation_error`，错误回包使用 `bridge.invalidType.error` 避免回显畸形 type；可识别当前 session/id 但无法反序列化成完整 packet 的 request 会返回 non-retryable `parse_error`，VST3/wry endpoint 会把这些错误 packet 回传 WebView。
  - Param gesture metadata validation: 可选 `gestureId` 必须非空、最长 128 bytes、不能包含控制字符；JS SDK、wry bootstrap fallback 和 React/Vue/Svelte param edit helpers 都已支持在 begin/perform/end/set 阶段传入 `gestureId`。
  - Native bridge state/param payload shape validation: `state.setConfig` / `state.setUiState` 会区分缺字段、类型错误和内容非法；`baseRevision` 必须是非负整数，config key 必须是合法 string，`value` 字段必须存在且允许 JSON null。`param.begin` / `param.perform` / `param.end` / `param.format` / `param.parse` 会校验 param id、normalized finite number、optional gestureId 和 parse text；无效 payload 不会污染 pending gesture queue 或 state snapshot。
  - `state.setConfig` config schema/backpressure: key 必须非空、最长 128 bytes、不能包含控制字符；config 表最多 256 个 entry，表满新增 key 返回 retryable `backpressure`，更新已有 key 仍允许。
  - `state.setUiState` 使用 state lane 的 256 KiB message limit，整块替换 persistent UI state；UI revision 过旧返回 retryable `state_conflict`。
- Realtime meter primitives:
  - `vesty-core::MeterFrame` 固定最多 8 channel，保存 `id_hash`、`sample_offset`、peak 和 RMS，不需要堆分配。
  - `vesty-core::MeterSink` 和 `ProcessContext::emit_meter()` / `emit_output_meter()`。
  - `vesty-params::ParamHandle`、`ParamCollection::resolve()`、`get_normalized_by_handle()` / `set_normalized_by_handle()` 和 `ProcessContext::param_normalized()` 已接入；推荐在 `create_kernel()` 中解析 handle，audio `process()` 内用 handle 读参数。
  - `#[derive(Params)]` 生成的 handle access path 直接按字段 index match，不通过 `specs()` 分配。
  - VST3 sample-accurate automation 转换出的 `Event::Param` 现在携带 `ParamHandle` 和 `id_hash`，developer kernel 可直接按 handle 匹配 automation event。
  - VST3 `kNoteExpressionValueEvent` 会转换为 `Event::NoteExpressionValue`，并在同一个固定事件列表中按 `sample_offset` 稳定排序；`vesty_core::note_expression` 提供标准 type id 常量。
  - `Plugin::note_expression_value_types()` 可 opt-in 暴露静态 Note Expression value metadata；VST3 controller 实现 `INoteExpressionController`，fake COM 测试覆盖 count/info/string/value。
  - `Plugin::note_expression_physical_ui_mappings()` 可 opt-in 暴露静态 Note Expression physical UI mapping metadata；VST3 controller 实现 `INoteExpressionPhysicalUIMapping`，fake COM 测试覆盖 mapping query、capacity truncation 和 invalid pointer/bus/channel。
  - `examples/midi-synth` 已 opt in 声明 `BRIGHTNESS` / `TUNING` value metadata 和 pressure/X movement physical UI mapping，并在 audio kernel 中用固定字段实时消费 brightness/tuning，不写 controller state、不做 JSON。
  - `examples/midi-synth` 已展示固定格式 SysEx `[F0, 7D, level, F7]` 的实时安全消费路径，用 kernel 内部 level override 影响合成输出。
  - `vesty-vst3` 在 `process()` 中把参数自动化和 MIDI 事件合并到固定容量 event list 后，按 `sample_offset` 做零分配稳定排序；`ParamAutomationSegments` 可假设事件流为 sample-order，同 offset 保留收集顺序。
  - `ProcessContext::param_automation(handle)` 和 `latest_param_automation(handle)` 已接入；开发者可在 realtime path 中按 `ParamHandle` 过滤当前 block 的 automation point，不需要字符串查找或分配。
  - `ParamAutomationSegments`、`ParamAutomationSegment`、`ProcessContext::param_automation_segments()` 和 `audio_mut_and_events()` 已接入；developer kernel 可把当前 block 切成 sample-accurate 参数区间，同时保持零分配和可变 audio 写入。
  - `ProcessMode` 和 `ProcessContext::process_mode()` 已接入；默认手工构造为 `Realtime`，VST3 adapter 从 `ProcessData.processMode` 映射 `Realtime` / `Prefetch` / `Offline`，让 kernel 可识别 offline render pass。
  - `AudioBuffers::copy_input_to_output_range()` 已接入，示例和模板可按 automation segment 复制指定 sample 范围。
  - `examples/gain`、`examples/midi-synth`、`examples/web-ui-param-demo` 以及 `vesty new` effect/instrument 模板已改为使用 `ParamAutomationSegments` 做 sample-accurate gain/level/mix。
  - `vesty-rt::meter_spsc()` 基于 `rtrb`，`RtMeterProducer` 实现 `MeterSink`，队列满时返回 false 并丢弃该帧。
  - VST3 processor 创建 RT-safe meter SPSC producer，并在 `IAudioProcessor::process` 中作为 `MeterSink` 注入 `ProcessContext`；audio thread 不做 JSON、WebView、锁或 host message 分配。
  - VST3 factory 持有 telemetry registry；processor/controller 通过 `IConnectionPoint` 上的内部 `IMessage` 绑定 telemetry id，让 controller 获取对应 `RtMeterConsumer`。
  - wry bootstrap 与 `@vesty/plugin-ui` 在订阅 `meter.*`、`param.changed`、`diagnostics.fault` 或 `log.rt` topic 时启动约 60 Hz 的 `event.flush` async event pump；UI/control thread drain SPSC、pending host param changes、fault/log snapshot 后通过 bridge batch 回推 JS。
  - `examples/web-ui-param-demo` 的 DSP kernel 调用 `emit_output_meter()`，打包 UI 订阅 `meter.main` 并显示 peak。
  - `vesty-rt::log_spsc()` 提供非阻塞 RT log queue；`RtLogEvent` 使用固定 enum/code/数字 payload，不在 audio thread 做字符串格式化，队列满时 `try_push` 返回 error 由调用方丢弃/计数。
  - VST3+wry 首版真实接线: WebView IPC `param.begin/perform/end` 进入 Rust bridge，并 relay 到 VST3 `IComponentHandler`。
  - VST3 embedded wry runtime 通过 UI-thread `evaluate_script(deliverBatch(...))` 回推 bridge responses/events。
- Build/package:
  - `vesty.toml`。
  - asset manifest。
  - symlink asset rejection。
  - macOS/Windows/Linux bundle path mapping。
  - macOS/Windows/Linux bundle structure package。
  - `[package].category` 已写入 `moduleinfo.json` class category；缺失时按 `[plugin].kind` 映射为 VST3 category `Fx` 或 `Instrument`。
  - `read_config()` 和 `package_vst3()` 已校验 `[plugin].name`、`vendor`、`version`、`kind` 非空，`[plugin].kind` 属于 effect/fx/audio-effect/audio_effect/instrument 支持集合，`[plugin].class_id` 合法，`[package].bundle_id` 如果存在必须非空且符合 conservative reverse-DNS shape，`[package].signing` 如果存在也必须非空；`[package].category` 为空时仍按既有语义从 `[plugin].kind` 映射。
  - `[plugin].class_id` 已在 package/validate 中校验为 16-byte hex UUID/FUID；打包写入 `moduleinfo.json` 时规范化为小写 UUID。
  - `[package].signing` 已接入 `vesty package`: macOS 使用 `codesign` 签 `.vst3` bundle，Windows 使用 `signtool.exe` 签 platform binary，Linux 返回 release-channel 外部签名提示。
  - `vesty package --install-dev` 已接入: 打包后可把 `.vst3` copy/symlink 到默认用户 VST3 目录或 `--vst3-dir` 指定目录；默认 `--install-mode copy` 并覆盖同名旧 dev bundle。
  - `vesty package` 和 `vesty dev --install-dev` 的 `--platform` 已改为可选；不传时按当前 OS 推断 macOS / Windows x64 / Linux x64，交叉打包仍可显式指定。
  - `vesty notarize` 已接入 macOS notarization workflow command planning: `ditto` zip、`xcrun notarytool submit`、可选 `xcrun stapler staple`。
  - `vesty validate` 静态 bundle/resource validation: `.vst3` 结构、moduleinfo、platform binary、macOS plist/pkginfo 内容、UI manifest entry/files/path/duplicate/mime/sha256 格式/size/sha256。
  - `read_config()`、`read_parameter_specs()`、`read_parameter_manifest()`、`package_vst3()` 和 `validate_vst3_bundle()` 对配置、参数 sidecar、打包 binary、bundle metadata、platform binary dirs/binaries、macOS `Info.plist` / `PkgInfo`、packaged parameter manifest 与 Web UI assets 执行 no-follow metadata 检查，symlinked inputs/artifacts 会失败而不是被跟随。
  - `validate_vst3_bundle()` 会校验 `moduleinfo.json` 顶层 `name`、`vendor`、`plugin_version` 非空，每个 class 的 `name`、`category` 非空，并校验 class id 格式。
  - macOS `Info.plist` 会被解析校验: dictionary root、`CFBundlePackageType = BNDL`、非空 `CFBundleExecutable` 且指向 `Contents/MacOS/<executable>` 文件，并且必须匹配 `moduleinfo.json` 推导出的 macOS binary name；`CFBundleName` 必须匹配 `moduleinfo.json` name；`CFBundleShortVersionString` 和 `CFBundleVersion` 必须匹配 `moduleinfo.json` plugin_version；`CFBundleIdentifier` 非空且符合 conservative reverse-DNS shape；`PkgInfo` 必须精确为 `BNDL????`。
  - platform binary 会按 `moduleinfo.name` 计算规范路径并校验: macOS `Contents/MacOS/<name>`、Windows `Contents/x86_64-win/<name>.vst3`、Linux `Contents/x86_64-linux/<name>.so`。
- CLI:
  - `vesty new`
  - `vesty templates` 列出内置 starter gallery；支持 text/json 输出。
  - `vesty new --template gain|midi-synth|web-ui-param-demo|vanilla-ui-param-demo|vue-ui-param-demo|svelte-ui-param-demo|web-ui-instrument` 使用 gallery 默认 kind/UI，并允许显式 `--kind` / `--ui` 覆盖。
  - `vesty new --ui react|vue|svelte|vanilla|none`
  - `vesty new --vesty-path /path/to/crates/vesty`
  - `vesty new --plugin-ui-path /path/to/packages/plugin-ui`
  - `vesty dev --config ... [--release] [--no-ui] [--ui-command ...]`
  - `vesty dev --install-dev [--platform ...] [--out ...] [--vst3-dir ...] [--install-mode copy|symlink]`
  - `vesty dev --install-dev --binary <cdylib>` 仍可作为显式 cdylib override。
  - `vesty build`
  - `vesty build --debug|--release`
  - `vesty build --no-ui`
  - `vesty build` 已对 UI 目录缺失、`[ui].build` 为空/失败、`[ui].dist` 缺失和 manifest 生成失败提供带目录/命令上下文的错误；对应 CLI 单元测试覆盖 missing UI dir、missing dist 和 shell cwd validation。
  - `vesty package`
  - `vesty notarize <bundle.vst3> --keychain-profile ...` 或 Apple ID credentials
  - `vesty validate [--static-only] [--format text|json] [--report path] [--validator-log path]`
  - `vesty daw-matrix --format markdown|json [--write-template] [--strict]`
  - `vesty host-quirks [--host alias] [--format markdown|json]`
  - `vesty release-check [--format markdown|json] [--strict] [--protocol-snapshot path] [--skip-protocol]`
  - `vesty release-check --release-evidence-dir <dir>` 自动发现 evidence 时只采纳内容有效的 release/static validate JSON；模板 pending JSON 保持 optional check skipped，显式传入的 pending/invalid report 仍严格失败。
  - `vesty release-check --require-release-artifacts` 的 signed bundle gate 会要求签名证据同时覆盖 macOS codesign 和 Windows signtool；泛用 `signed=true` / `signature=ok` 会被拒绝，因为无法证明 macOS 或 Windows 平台签名验证；同一日志中的 invalid signature、非零 signtool error count 或 `SignTool Error` 会覆盖正向 marker 并使证据失败。
  - `vesty release-check --require-release-artifacts` 的 notarization gate 会要求 notarization log 同时包含 accepted notarytool 输出和 stapler success；泛用 `notarization=pass` / `notary=ok` 会被拒绝，因为无法证明 notarytool accepted status；同一日志中的 rejected/invalid notary status 或 stapler failure 会覆盖正向 marker 并使证据失败。
  - `vesty smoke-host [--workspace <path>] [--bridge-trace <path>] [--meter-log <path>] [--out <path>] [--check] [--strict]`
  - `vesty export-types --out target/vesty-protocol [--check]`
  - `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out <json> [--check]`
  - `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --bindings-module target/vst3-sdk/generated.rs --out <json> [--check]`
  - `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out <json> [--check]`
  - `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated.rs [--check]`
  - `vesty vst3-sdk emit-abi-seed --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi-seed.rs [--check]`
  - `vesty vst3-sdk emit-abi --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi.rs [--check]`
  - `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-interface-skeleton.rs [--check]`
  - `vesty doctor [--format text|json]`
- CI:
  - `.github/workflows/ci.yml`
  - Rust matrix: Ubuntu/macOS/Windows。
  - JS SDK typecheck/test/build。
  - protocol export artifact。
  - 每个 Rust matrix OS 生成并上传 `vesty doctor --format json` artifact，覆盖 toolchain、WebView、validator、signing/notarization 预检和 DAW install hints。
  - 可选 `vst3-sdk` job 在 `VESTY_VST3_SDK_DIR` 存在时生成并复验 `vst3-sdk-headers.json`、`generated-bindings-plan.json`、`generated-bindings-surface.json`、metadata-only `generated.rs` scaffold、ABI seed `generated-abi-seed.rs`、ABI layout `generated-abi.rs` 与 interface skeleton `generated-interface-skeleton.rs`，并上传到 `vesty-vst3-sdk-headers` artifact；未设置时上传 skip note。
  - headless smoke host job 构建 `examples/web-ui-param-demo/ui`，写入本地 JSBridge/meter marker，运行 `vesty smoke-host --strict` 和 `--check --strict`，上传 `vesty-smoke-host` 诊断 artifact。该 artifact 只用于本地框架自检和 CI drift 定位，不进入 release evidence gate。
  - scaffold local path smoke。
  - Linux/macOS/Windows example package/static validate artifact；static validate JSON 通过 `vesty validate --report` 写入 artifact。

## 最近补强

- 新增真实 COM 边界测试:
  - fake `IBStream` 验证 `IComponent::getState/setState`、`IEditController::getState/setState`、`setComponentState`。
  - fake `IParameterChanges` / `IParamValueQueue` 验证 sample-accurate automation 进入 `vesty_core::Event::Param`。
  - fake `IEventList` 验证 VST3 NoteOn/NoteOff/PolyPressure、legacy MIDI CC、PitchBend 和 ChannelPressure 进入 `vesty_core::Event`。
  - fake VST3 `ProcessContext` 验证 tempo/playing/sample position mirror。
  - fake VST3 factory boundary 测试验证 `getFactoryInfo()` / `getClassInfo()` null output pointer 返回 `kInvalidArgument`，`createInstance()` 拒绝 null class id/interface id/output pointer，并在失败路径清空可写 output pointer，避免 host 拿到 stale instance。
  - fake VST3 factory 参数 schema 测试验证 processor 和 controller 创建都会先校验 `ParamSpec` schema 与稳定 VST3 `ParamID` registry；重复/非法参数 schema 返回 `kResultFalse`，输出指针保持 null，避免 host 拿到 processor/controller 半初始化组合。
  - fake VST3 `setupProcessing()` matrix 验证首次 process 会用 host sample rate / max block size 创建 kernel 并调用 `AudioKernel::prepare()`，后续 sample rate / block size 变化会重新调用 `prepare()`。
  - fake VST3 `IComponent::getControllerClassId()` 验证 null output pointer 返回 `kInvalidArgument`，正常 output pointer 写入 controller CID。
  - fake VST3 controller parameter callback 测试验证 `getParameterInfo()`、`getParamStringByValue()`、`getParamValueByString()` 会拒绝 null host pointers 和负参数 index，不会因 host/validator 坏参数探测触发空指针读写。
  - fake VST3 bounded `String128` parse 测试验证 controller 参数解析和 Note Expression 解析不会依赖 host 输入 NUL 结尾；非 NUL 结尾但 128 单元内有效的输入会在固定边界内解析。
  - fake VST3 `IPlugView` boundary 测试验证 null `platform_type` 不被支持，supported platform 搭配 null native parent handle 时 `attached()` 返回 `kResultFalse`，不会误报 editor attach 成功。
  - fake VST3 `IComponent::setIoMode()` 验证 `kSimple`、`kAdvanced`、`kOfflineProcessing` 三种标准 mode 会被接受并记录，未知 mode 返回 `kInvalidArgument` 且保持上一有效状态；block 级 DSP mode 仍由 `ProcessData.processMode` 映射。
  - fake VST3 process input bus shape 测试验证 host `numInputs` 超过插件声明 input bus 数量时，adapter 不构造越界 input slice、不进入 developer kernel，并清零 output + 设置 silence flags。
  - fake VST3 process channel-count 测试验证 host `AudioBusBuffers::numChannels` 超过固定支持上限或声明 output layout 时，adapter 不按异常 host 声称数量构造 raw pointer slice；oversized input bus 会降级为空输入并保持 DSP 兼容，oversized output bus 会拒绝本次 layout 且不进入 developer kernel。
  - fake VST3 process block-size 测试验证 `numSamples < 0` 时，adapter 不进入事件收集或 developer kernel，并设置 output silence flags。
  - fake VST3 bus arrangement 测试验证 effect 的 mono->mono、mono->stereo、stereo->stereo negotiation，拒绝 unsupported stereo->mono/surround，并验证 instrument 的 event input + stereo output；`IComponent::activateBus()` 现在会校验 media type、direction 和 bus index，并记录 main input/output、sidechain、instrument event input 与 aux output 的 active state；multi-output instrument 测试覆盖两个 stereo output bus 的 main/aux metadata、arrangement 校验、sample32 process 写入 main 与 aux bus，以及 realtime allocation guard 0 allocation。
  - fake VST3 `AudioBusBuffers::silenceFlags` 测试验证 `ProcessResult::Silence` 会设置 output channel silence bits 并清零输出，正常 `Continue` 会清理 stale silence flags。
  - fake VST3 factory/query 验证 processor 和 controller 暴露 `IConnectionPoint`，并可双向 connect、notify、disconnect。
  - `IAudioProcessor::process` 在 sample-size 检查后激活 `NoAllocGuard` 实时区，覆盖 VST3 event collection、sample-order sort、transport mirror、buffer/context 组装和 developer kernel，并由 guard-aware 测试 allocator 验证热路径 0 allocation。
  - kernel 创建和 `prepare()` 提前发生在 `setupProcessing()` / `setActive(true)`；缺 kernel 的异常 `process()` 调用会清零输出、设置 silence flags，并推固定 `HostWarning` RT log，不在实时区兜底创建 kernel。
  - `IAudioProcessor::process` 中 developer kernel panic 会 fault 当前实例、清空当前输出，并在后续 block 保持 faulted silence，不再重进 panic kernel。
  - `vesty-vst3::FaultState::report()` 暴露 `FaultReport { faulted, fault_count }`；第一次 panic/fault transition 递增 count，后续 faulted fallback 不重复递增。
  - VST3 COM 边界的 telemetry bind message、factory instance、plugin factory 和 plug view 创建失败时返回 `kResultFalse` 或 null pointer，不再通过 `unwrap()` panic 穿过 host callback。
- `vesty doctor` 增加:
  - rustc/cargo/node/npm 检查。
  - VST3 binding baseline 检查: Steinberg SDK baseline、upstream `vst3` crate baseline 和当前 binding backend。
  - VST3 SDK headers probe: `VESTY_VST3_SDK_DIR` 未设置时显示 skipped；设置后会检查 generated headers 后备路径所需的关键 `pluginterfaces` headers，缺失则报告 missing。
  - `VST3_VALIDATOR`、PATH、常见 VST3 SDK 路径 validator discovery。
  - workspace `target/steinberg/...` validator discovery。
  - macOS WebKit.framework、Windows WebView2、Linux WebKitGTK/X11 检查。
  - release signing/notarization 前置工具检查: macOS `codesign` / `xcrun notarytool`、Windows SDK `signtool.exe`、Linux release-channel policy 提示。
  - 常见 DAW 安装路径检查: REAPER、Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One；该检查只提示安装存在，不替代 smoke evidence。
  - `--format json` 机器可读输出，每个 check 包含 `name/status/value/hint`。
- `vesty validate` 增加:
  - `--format json` 机器可读输出。
  - 静态 bundle check 失败时仍先输出 JSON report 再返回非零。
  - `--report <path>` 可把完整 JSON report 写入文件，静态检查失败也会写。
  - `--validator-log <path>` 可把 Steinberg validator stdout/stderr 写入文件，便于 CI artifact 留档。
  - validator stdout/stderr/exit code 捕获。
  - Steinberg `Result: ... tests passed, ... tests failed` 解析为 `tests_passed` / `tests_failed`。
  - `vesty dev` 从占位提示升级为:
  - 按 config 所在目录运行 `cargo build`。
  - 有 UI 时启动 `npm run dev` 或用户传入的 `--ui-command`。
  - 打印 `dev_url`，支持 `--no-ui`。
  - `--install-dev` 时复用 package/install pipeline，在启动 UI dev server 前打包并安装 debug `.vst3`。
  - `vesty build --no-ui` 已接入，可在存在 `[ui]` 配置时跳过 Web UI build/dist manifest，只构建 Rust 插件。
- JSBridge/VST3 UI 接线补强:
  - `vesty-bridge` 将 param gesture 记录为 typed `ParamGesture`，可被 host adapter drain。
  - `vesty-bridge` 新增 `queue_latest_meter()`、`flush_latest_meters()`，用订阅表和 latest-wins 语义保护 UI meter/analyzer 等高频流。
  - `vesty-bridge` 的 reliable `emit_event()` 已通过订阅表过滤，未订阅 topic 不会发送给 WebView。
  - `vesty-bridge` 为 `state.setConfig` 增加 stale revision rejection，覆盖成功提交、缺少 `baseRevision` 和冲突返回三个单元测试。
  - `vesty-bridge` 为 `state.setUiState` 增加 stale UI revision rejection，覆盖成功提交、缺少 `baseRevision` 和冲突返回三个单元测试。
  - `vesty-bridge` 为 `state.setConfig` / `state.setUiState` 增加订阅式 `state.changed` 完整 snapshot event，覆盖 config 和 persistent UI state 写入后的事件顺序与 payload。
  - `vesty-bridge` 为 `state.setConfig` 增加 config key/entry limits，覆盖 invalid key、config 表满背压和表满更新已有 key 的单元测试。
  - `vesty-ui-wry` 注入 bootstrap 的 `subscribe()` 现在会发送 `subscription.add/remove` IPC；同 topic 多 handler 时只在首个 handler 注册和最后一个 handler 移除时通知 Rust。
  - `packages/plugin-ui` 与 wry bootstrap 的 `subscribe(topic, handler)` 会在本地校验 handler 必须是 function；无效 handler 返回 non-retryable `validation_error`，不会写入 listener 表或发送 `subscription.add`。
  - `vesty-ui-wry` 注入 bootstrap 在存在 `meter.*`、`param.changed`、`diagnostics.fault` 或 `log.rt` 订阅时以约 60 Hz 发送 `event.flush`，无异步事件订阅时停止 pump。
  - `vesty-ui-wry` 和 `packages/plugin-ui` 提供 `setConfig(key, value, baseRevision)` 与 `setUiState(value, baseRevision)` typed API。
  - `packages/plugin-ui` 与 wry bootstrap 对齐订阅和 async event pump 语义，并在 `postMessage` 前登记 Promise pending，避免同步/快速 response 丢失。
  - `packages/plugin-ui` 与 wry bootstrap 已实现 request timeout：默认 5000ms；TS SDK 可通过 `createBridge(..., { timeoutMs })` 配置，超时和同步 `postMessage` 失败都会释放 pending/timer；`@vesty/plugin-ui` 会在初始化时校验 host 必须能注册 unload listener、`initialSession` 必须是非空/长度受限/无控制字符字符串、`options` 必须是 object、`timeoutMs` 必须是 finite number，无效输入返回 non-retryable `validation_error`，不会注册 unload listener、创建 `__VESTY_INTERNAL__` 或发送 IPC。
  - `packages/plugin-ui` 与 wry bootstrap 已实现 UI unload cleanup：`pagehide` / `beforeunload` 会停止 async event pump，清空 JS 侧 topic listeners，并 reject 所有 pending request，降低 WebView reload/close 时的 Promise、handler 和 interval 残留风险。
  - `packages/plugin-ui` 与 wry bootstrap 已实现 subscription listener 异常隔离：单个 handler 抛错只记录 console error，不会阻断同 topic 其它 handler 或同批 `deliverBatch` 后续 packet。
  - `packages/plugin-ui` 的 `createSnapshotStore()` 已实现 runtime input 校验与 snapshot listener 异常隔离：`options` 必须是 object，`topic` 复用 subscription topic 校验，`refreshOnEvent` 必须是 boolean，`subscribe(listener)` 会先校验 listener 必须是 function，`select(selector)` 会先校验 selector 必须是 function；无效输入返回 non-retryable `validation_error` 且不会触发底层 `subscription.add`。单个 listener 抛错只记录 console error，不会阻断其它 listener，也不会让 `refresh()` 因 UI 回调异常而 reject；React/Vue/Svelte adapter 共享该行为。
  - `packages/plugin-ui` 与 wry bootstrap 已实现 editor session adoption：`bridge.hello` 返回 ready payload 后会先校验 `editorSessionId` 必须是非空、最长 128 UTF-8 bytes 且无控制字符的字符串，再采纳该 session；畸形 ready session 返回 non-retryable `validation_error`，不会采纳 session 或发送 `bridge.readyAck`。`vesty-bridge` 从 `pending` handshake 切换到 editor session，旧 session 后续消息返回 `permission_denied`，UI 侧入站分发也会丢弃 session 不匹配的 stale packet。
  - `packages/plugin-ui` 与 wry bootstrap 已实现 typed `bridge.hello` payload；`vesty-bridge` 会拒绝缺失/无效 hello payload 或不支持当前协议版本的 UI。
  - `packages/plugin-ui` 与 wry bootstrap 已实现 ready response protocol guard：`bridge.hello` response 的 `protocolVersion` 必须为 `1`；不兼容时 JS `ready()` 以 `unsupported_version` 拒绝，不采纳 editor session，也不发送 `bridge.readyAck`。
  - `packages/plugin-ui` 与 wry bootstrap 已实现 `bridge.readyAck`：ready payload 后用 editor session ack；`vesty-bridge` 要求 readyAck 必须发生在成功 `bridge.hello` 之后，并校验 ack payload 的 `protocolVersion` 存在、类型为非负整数且等于当前协议版本，提前 ack、缺失/类型错误或不支持版本都不会把 runtime 标记为 ready，校验通过后记录 `ready_acknowledged()` 并返回 `bridge.readyAck.response`。
  - `vesty-ipc`、`vesty-bridge`、`vesty-vst3` 和 `packages/plugin-ui` 已接入 `BridgeCapabilities`；ready response 测试覆盖 capabilities 字段。
  - `BridgeCapabilities` 增加 `diagnostics`；`vesty-ipc` 导出 `BridgeDiagnosticsSnapshot` 和 `PluginFaultReport` 的 TypeScript/JSON Schema。
  - `vesty-ipc` 导出 `RtLogRecord` / `RtLogLevel` / `RtLogKind` / `RtLogQueue`；`@vesty/plugin-ui` 同步提供这些 TS 类型。
  - `vesty-bridge` 支持 `diagnostics.get`，返回 ready ack、订阅、待处理 param/meter、`droppedParamGestures` 和 fault report 快照。
  - `vesty-bridge` 支持订阅式 `diagnostics.fault` 事件，仍由订阅表过滤，未订阅不推送。
  - `vesty-bridge` 支持订阅式 `log.rt` 事件，payload 为结构化 `RtLogRecord`，走 `BridgeLane::Log`。
  - `vesty-vst3` telemetry channel 从 meter consumer 扩展为 meter + RT log + shared `FaultState`；processor panic guard 只做原子 fault 标记和固定 `RtLogEvent::Faulted` nonblocking push，controller/UI 线程读取并经 JSBridge 暴露。
  - `packages/plugin-ui` 和 wry bootstrap 新增 `getDiagnostics()` API。
  - `packages/plugin-ui` 增加显式 `rootDir = "src"`，兼容最新 TypeScript emit；`dist/index.js` 和 `dist/index.d.ts` 已生成。
  - `vesty-ipc`/`vesty-params` 接入 `ts-rs` derive，`vesty export-types` 可从 Rust protocol source 输出 TypeScript 类型和 JSON Schema；生成类型覆盖 bridge packet、ready/hello/capabilities、snapshot、param changed event 和 ParamSpec/ParamKind/ParamFlags。
  - 参数协议 schema 已统一为 JS-friendly camelCase: `defaultNormalized`、`stepCount`、`readOnly`、`programChange`，`ParamKind` tag 为 `"float"` / `"bool"` / `"choice"`；Rust 侧 `ParamSpec` / `ParamFlags` / `ParamKind` 仍可反序列化旧的 snake_case 字段和 Rust variant 名称，便于 alpha 期间迁移。
  - `vesty export-types --check` 会导出到临时目录，并与 `--out` snapshot 做逐文件 byte comparison；missing/changed/extra 文件会返回非零，作为 schema/protocol drift gate。
  - `vesty-vst3` controller 保存 host `IComponentHandler` 并提供 begin/perform/end relay。
  - `vesty-vst3` 的 wry view attach 时安装 IPC handler，真实 WebView slider 事件可进入 Rust 并通知 host；host 接受 perform 后会通过 `param.changed` 回推 UI confirmed value。
  - `vesty-vst3` controller 维护非实时 pending host param change queue；订阅前的 latest-wins host 参数变化不会被丢弃，订阅 `param.changed` 后或下一次 `event.flush` 会回推 `source = "host"`，state restore 标记为 `source = "state"`，controller-side program apply/program data load 标记为 `source = "program"`。
  - `vesty-ui-wry` 的 bridge handler 可返回 `BridgePacket` batch，并在 UI callback 中用 `evaluate_script` 回推给 JS Promise。
  - `VESTY_BRIDGE_TRACE=/path/to/log` 可在 UI 线程记录 IPC/relay smoke，不进入 audio realtime path。
- RT guard 测试扩展:
  - `processor_process_does_not_allocate_inside_rt_guard_under_automation_and_midi` 覆盖 automation queue + MIDI events + audio block。
  - `processor_prepare_tracks_sample_rate_and_block_size_matrix` 覆盖 32/64/128/1024 samples 和 44.1/48/96/192 kHz，并确认进入 kernel 时 `NoAllocGuard` 处于 active。
- Realtime meter 测试:
  - `vesty-core` 验证 output peak/RMS 计算、无 sink 时丢帧。
  - `vesty-rt` 验证 meter producer 可作为 `MeterSink` 推入 SPSC，队列满时不阻塞。
  - `vesty-bridge` 验证 active channel meter payload 进入 `BridgeLane::Meter` latest-wins flush。
  - `vesty-vst3` 验证 processor/controller 经 `IConnectionPoint` 绑定后，developer kernel 产生的 `MeterFrame` 可从 controller 侧 drain，证明 RT SPSC telemetry 链路已接通。
  - `vesty-ui-wry` 验证 bootstrap 包含 async `event.flush` pump。
- UI asset protocol 测试:
- `vesty-ui-wry` 验证 custom protocol 使用 manifest allowlist，拒绝 manifest 外文件和路径穿越，校验 manifest `size`/`sha256` 并拒绝篡改内容；缺失、无效 JSON、缺失 entry、unsafe path、重复 path、bad sha256 或非法 MIME manifest 都会 fail closed。
- `vesty-ui-wry` 和 `vesty-build` 验证 asset manifest MIME 不含控制字符或前后空白，并能安全进入 HTTP `Content-Type` header；custom protocol response 使用显式 fallback 组装，避免 release WebView asset request callback 中因 response builder error panic。
- `vesty-ui-wry` 验证 release navigation 和 release IPC 只允许 bundle asset URL，并验证 HTML asset response 带默认 CSP / `nosniff`。
- `vesty-ui-wry` 验证 native IPC handler panic 会被边界捕获，并在原始 IPC 可解析时生成 `internal_error` bridge error packet；无法解析的 panic 消息不回包，避免崩溃恢复路径再触发解析/序列化风险。
- DAW matrix 脚手架:
  - `vesty daw-matrix` 根据 evidence log 汇总 REAPER、Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One 的 scan/load/UI/UI->Host/meter stream/automation/buffer-sample-rate change/save-restore/offline render，并把未采集 DAW 明确标为 missing。
  - CLI 支持 `--reaper-evidence`、`--cubase-evidence`、`--bitwig-evidence`、`--ableton-evidence`、`--studio-one-evidence`，默认目录分别是 `target/reaper-smoke`、`target/cubase-smoke`、`target/bitwig-smoke`、`target/ableton-smoke`、`target/studio-one-smoke`。
  - CLI 支持 `--evidence-root <dir>`，按 `<dir>/reaper`、`<dir>/cubase`、`<dir>/bitwig`、`<dir>/ableton`、`<dir>/studio-one` 统一读取/写入 DAW evidence；`release-check` 使用同一约定，便于外部 host smoke 证据打包。
  - CLI 支持 `--write-template` 生成 evidence 目录和 pending 日志模板；模板不会覆盖已有日志，也不会被判定为 pass。
  - `--write-template` 生成的每个 host README 会从 `vesty-core` host profile registry 内联 required smoke checks、platform、notes 和 quirk/mitigation 表；已有 README 不会被覆盖，避免破坏手工采集笔记。
  - CLI 支持 `--write-report --host <host>` 从真实 smoke 后的显式 marker 写入标准 evidence 文件；写入前拒绝 pending/false/占位值、zero meter 和无法被 matrix parser 识别的模糊 marker，避免留下半套无效 evidence。写入后仍会复用 DAW matrix parser 验证该 host 行完整。REAPER 的专用 evidence 解析仍保留，同时也接受这套通用 marker。
  - 缺失的非 REAPER host 行会保持所有 smoke check 为 missing，但 `Evidence` 字段会指向对应 evidence 目录，便于 `release-check` hint 直接指导补采路径。
  - CLI 支持 `--strict` release gate: 先打印 markdown/json matrix，再在任意 required check 非 `true` 时返回非零。
  - 非 REAPER DAW evidence 目录支持 `platform.txt`、`scan-smoke.log`、`load-smoke.log`、`ui-smoke.log`、`ui-host-smoke.log`、`meter-stream.log`、`automation-smoke.log`、`buffer-sample-rate.log`、`restore-smoke.log`、`offline-render.log`，每项可用 `scan=true`、`load=pass`、`ui_ok=true`、`ui_host_param=true`、`automation=true`、`buffer_sample_rate_change=true`、`save_restore=pass`、`offline_render=ok` 等明确 marker 证明。
  - Offline render evidence 还接受 `render_file=/absolute/path.wav`、`render_file = "rendered.wav"` 和 `render_file='rendered.wav'`；作为文件证据时目标必须存在且非空，相对路径按当前 host evidence 目录解析且不允许 `..` 父目录跳转，显式 `offline_render=true|pass|ok` marker 仍然可用。
  - UI->Host 判定同时接受 bridge trace relay 证据和 REAPER host-side `param-watch.log` normalized 参数移动证据，避免后续 hello trace 覆盖 param relay trace 后误报 missing。
  - Meter stream 判定接受 `target/reaper-smoke/meter-stream.log` 中的 `meter.main` 非零 peak/rms 记录，或 bridge trace 中的 `meter_flush sent=N` / `BridgeLane::Meter` packet evidence。
- Cross-platform packaging 测试:
  - `vesty-build` 单元测试覆盖 macOS、Windows x86_64、Linux x86_64 的 VST3 binary relative path。
  - `package_vst3()` 的 macOS/Windows/Linux bundle 结构均有文件系统 fixture 覆盖。
  - `package_vst3()` merged bundle fixture 覆盖同一个 `.vst3` 目录连续写入 macOS、Windows x86_64、Linux x86_64 后，三个 platform binary 都被 `validate_vst3_bundle()` 保留并收集。
  - 测试覆盖空 plugin metadata、unsupported plugin kind、空/非法 bundle id 被拒绝，以及空 package category 按 plugin kind 映射到 VST3 category。
  - 测试固定 `Contents/Resources/moduleinfo.json`、`Contents/Resources/ui`、`assets.manifest.json` 生成。
  - 测试确认 Windows/Linux bundle 不生成 macOS-only 的 `Info.plist` 和 `PkgInfo`。
  - `validate_vst3_bundle()` 覆盖 packaged bundle、缺失 platform binary、misnamed Windows/Linux binary、moduleinfo/macOS binary name mismatch、tampered UI asset manifest entry、malformed UI manifest metadata、tampered macOS executable plist、invalid package type plist、bad/missing PkgInfo，`vesty validate` 会在调用 Steinberg validator 前打印静态检查结果；`--static-only` 可跳过外部 validator。
- Params derive 补强:
  - `vesty-macros::Params` 不再是空 derive，已实现 named struct 参数字段收集。
  - derive 通过 `proc-macro-crate` 解析 facade 路径，普通插件 crate 默认生成 `::vesty::params::ParamCollection`，facade crate 内部测试生成 `crate::params::ParamCollection`。
  - `vesty` facade 增加 derive 单元测试，覆盖 `FloatParam`、`BoolParam`、`ChoiceParam`、`#[param(skip)]`、unknown param error。
  - `vesty-params` 增加 `ChoiceParam`，提供 `ParamSpec::choice()`、离散 normalized/index conversion、默认 index clamp、nearest-index snap、空 values 安全行为，以及 choice label format/parse helper。
  - `vesty-bridge` 和 `vesty-vst3` 的 param format/parse 已使用共享 helper；choice 参数在 JSBridge 和 VST3 host 文本中显示/解析 label。
  - `vesty-cli` 模板测试确认新项目不再生成手写 `impl ParamCollection` 样板。
- UI template 补强:
  - `vesty new --ui react|vue|svelte|vanilla|none` 已分流生成对应 UI 模板。
  - React/Vue/Svelte 模板生成 Vite config、框架入口组件，并通过 `@vesty/react`、`@vesty/vue`、`@vesty/svelte` 薄适配调用 param gesture helper；底层 bridge 仍来自 framework-agnostic `@vesty/plugin-ui`。
  - `vesty new --plugin-ui-path ...` 会自动推断 sibling `packages/react`、`packages/vue`、`packages/svelte`，生成本地 `file:` adapter dependency，便于当前 workspace smoke；未提供本地路径时使用发布依赖。
  - React 模板使用 `tsc --noEmit` 并声明 `@types/react` / `@types/react-dom`；Vue 模板使用 `vue-tsc --noEmit`；Svelte 模板使用 `svelte-check --tsconfig ./tsconfig.json`。
  - 本机 smoke 将 `@vesty/plugin-ui` 临时指向 workspace package 后，vanilla/React/Vue/Svelte 四个模板均完成 `npm install` + `npm run build` + `npm run typecheck`，并且生成的 Rust 插件均可 `cargo check`。
  - `vesty new --vesty-path ...` 可把生成项目依赖指向当前 workspace 的 `crates/vesty`，生成 Cargo.toml 默认带空 `[workspace]`，便于在任意父 workspace 下作为独立插件项目 `cargo check`。
  - `vesty new --plugin-ui-path ...` 可把生成 UI 的 `@vesty/plugin-ui` 依赖写成本地 `file:` dependency，免去 smoke 时手工 patch `package.json`。
  - `vesty new` 生成的 `vesty.toml` 已包含 `[package]` 默认元数据: effect 使用 `bundle_id = "dev.vesty.<crate-name>"` / `category = "Fx"`，instrument 使用 `category = "Instrument"`。
- 新增 GitHub Actions CI workflow:
  - `rust` job 跑 `cargo fmt --all --check`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、VST3 bindings/wry feature tests 和对应 feature clippy gates。
  - `js` job 跑 `npm install`、`npm run typecheck`、`npm test`、`npm run build`。
  - `protocol` job 跑 `vesty export-types`、deterministic `--check`、exported TypeScript strict check，并上传 `vesty-protocol` artifact。
  - `publish-plan` job 跑 `vesty publish-plan --out ...`、`vesty publish-plan --check --out ...` 和 `vesty release-order`，并上传 `vesty-publish-plan` artifact。
  - `scaffold` job 用 `none`、`vanilla`、`react`、`vue`、`svelte` 矩阵和本地 `--vesty-path` / `--plugin-ui-path` 生成项目；`none` 跑 generated Rust check，有 UI 的模板额外跑 `npm install`、`npm run build` 和 `npm run typecheck`。
  - `package` job 在 Ubuntu/macOS/Windows release build 三个 examples，按 linux-x64/macos/windows-x64 package `.vst3`，跑 static JSON validate，并上传 packaged bundle/validate JSON artifact。
- Plugin custom state 补强:
  - `vesty-core` 单元测试覆盖 typed `PluginState` helper -> JSON -> typed restore。
  - `vesty-vst3` state roundtrip 测试覆盖 params、custom state 和 bridge snapshot。
  - fake `IBStream` COM 边界测试使用带 `custom` 的 `VESTY_STATE_V1` payload，经 component `setState/getState` 后确认 custom payload 保留；wry/controller 测试覆盖 JSBridge state 写入 -> controller `getState()` -> 新 controller `setState()` -> editor ready snapshot 恢复。
- Latency/tail 补强:
  - `vesty-vst3` fake COM 测试创建 `IAudioProcessor` 并验证 `getLatencySamples()` / `getTailSamples()` 返回插件声明值。
  - `vesty-vst3` fake `IComponentHandler` 测试验证 latency-affecting 参数变化会调用 `restartComponent(RestartFlags_::kLatencyChanged)`，普通参数变化不会误触发。

本次 2026-06-08 针对 React/Vue/Svelte scaffold adapter 模板通过:

```bash
cargo test -p vesty-cli ui_package_template
cargo test -p vesty-cli ui_templates_emit_framework_specific_files
# 在 /tmp/vesty-adapter-smoke.u5oxcP 下分别生成 react/vue/svelte 项目:
cargo run --quiet --manifest-path /Users/orchiliao/Projects/vesty/Cargo.toml -p vesty-cli -- new vesty-<ui>-smoke --ui <ui> --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
cd /tmp/vesty-adapter-smoke.u5oxcP/vesty-<ui>-smoke/ui && npm install --no-audit --no-fund && npm run build && npm run typecheck
cd /tmp/vesty-adapter-smoke.u5oxcP/vesty-<ui>-smoke && cargo check
```

新增行为:

- React 模板依赖并使用 `@vesty/react` 的 `VestyBridgeProvider` / `useVestyParamEdit()`。
- Vue 模板依赖并使用 `@vesty/vue` 的 `useVestyParamEdit()`。
- Svelte 模板依赖并使用 `@vesty/svelte` 的 `vestyParamEdit()`。
- 当 `--plugin-ui-path` 指向当前 workspace 的 `packages/plugin-ui` 时，CLI 会自动把 sibling adapter package 写成本地 `file:` dependency；三种 UI 模板均已真实 npm build/typecheck，并且生成的 Rust 插件均已 `cargo check`。

## 已验证

本机通过:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vesty-vst3 --features vst3-bindings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo build -p vesty-example-gain --release
nm -gU target/release/libvesty_example_gain.dylib | rg 'GetPluginFactory|[Bb]undleEntry|[Bb]undleExit'
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-smoke
cargo build -p vesty-example-web-ui-param-demo --release
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-web-smoke
cargo build -p vesty-example-midi-synth --release
cargo run -p vesty-cli -- package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/vesty-synth-smoke
cmake -G Xcode -S target/steinberg/vst3sdk -B target/steinberg/vst3sdk-build-xcode -DSMTG_CREATE_PLUGIN_LINK=0
cmake --build target/steinberg/vst3sdk-build-xcode --config Release --target validator
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-smoke/VestyGain.vst3
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-web-smoke/VestyWebUIDemo.vst3
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-synth-smoke/VestyMIDISynth.vst3
cargo run -p vesty-cli -- validate target/vesty-smoke/VestyGain.vst3
cargo run -p vesty-cli -- validate target/vesty-web-smoke/VestyWebUIDemo.vst3
cargo run -p vesty-cli -- validate target/vesty-synth-smoke/VestyMIDISynth.vst3
cargo run -p vesty-cli -- doctor
cargo run -p vesty-cli -- daw-matrix --format markdown
```

最近一次 2026-06-08 针对 JSBridge 订阅、meter latest-wins、VST3 RT SPSC telemetry、`IConnectionPoint` 和 DAW matrix 判定补强后重新通过:

```bash
cargo test -p vesty-bridge
cargo test -p vesty-ui-wry
cargo test -p vesty-cli
cargo test -p vesty-core
cargo test -p vesty-rt
cargo test -p vesty-rt log_queue_uses_fixed_events_and_drops_on_overflow
cargo test -p vesty-vst3 panic_guard_faults
cargo test -p vesty-vst3 --features vst3-bindings
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-smoke
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-web-smoke
cargo run -p vesty-cli -- package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/vesty-synth-smoke
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-smoke/VestyGain.vst3
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-web-smoke/VestyWebUIDemo.vst3
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-synth-smoke/VestyMIDISynth.vst3
cargo run -p vesty-cli -- validate target/vesty-smoke/VestyGain.vst3
cargo run -p vesty-cli -- validate target/vesty-web-smoke/VestyWebUIDemo.vst3
cargo run -p vesty-cli -- validate target/vesty-synth-smoke/VestyMIDISynth.vst3
cargo run -p vesty-cli -- daw-matrix --format markdown
cargo run -p vesty-cli -- daw-matrix --format json
cargo run -p vesty-cli -- daw-matrix --help
cargo test -p vesty-build
cargo clippy -p vesty-build --all-targets -- -D warnings
cargo test -p vesty -p vesty-macros -p vesty-cli -p vesty-example-gain -p vesty-example-midi-synth -p vesty-example-web-ui-param-demo
cargo test -p vesty-core -p vesty-vst3 --features vst3-bindings
cargo test -p vesty-vst3 --features vst3-bindings processor_reports_latency_and_tail_samples
```

本次 2026-06-08 针对 `ChoiceParam` 和 `#[derive(Params)]` 扩展重新通过:

```bash
cargo fmt --all --check
cargo test -p vesty-params -p vesty -p vesty-macros
cargo test -p vesty-params -p vesty-bridge -p vesty-vst3 --features vst3-bindings
cargo test -p vesty-vst3 --features vst3-bindings processor_prepare_tracks_sample_rate_and_block_size_matrix
cargo test -p vesty-vst3 --features vst3-bindings processor_negotiates_mvp_effect_bus_arrangements
cargo test -p vesty-vst3 --features vst3-bindings processor_negotiates_instrument_event_input_and_stereo_output
cargo test -p vesty-vst3 --features vst3-bindings processor_updates_output_silence_flags
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
cargo test -p vesty-ui-wry --features wry-backend asset_protocol_uses_manifest_allowlist
```

本次 2026-06-08 针对 JSBridge request timeout、pending/timer 清理和 `@vesty/plugin-ui` dist 重新通过:

```bash
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json
cargo fmt --all --check
cargo test -p vesty-ui-wry
cargo test -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
node --input-type=module -e 'import { createBridge } from "./packages/plugin-ui/dist/index.js"; const listeners = new Map(); const posted = []; const host = { ipc: { postMessage(message) { posted.push(message); } }, addEventListener(name, handler) { listeners.set(name, handler); } }; const bridge = createBridge(host, "test-session", { timeoutMs: 0 }); const pending = bridge.request("demo.request").then(() => ({ ok: false, reason: "resolved unexpectedly" }), (error) => ({ ok: true, error })); listeners.get("pagehide")(); const result = await pending; if (!result.ok) throw new Error(result.reason); if (result.error?.code !== "internal_error") throw new Error(`wrong code: ${result.error?.code}`); if (result.error?.retryable !== true) throw new Error("expected retryable unload error"); if (posted.length !== 1) throw new Error(`expected one posted message, got ${posted.length}`); console.log("plugin-ui unload cleanup rejects pending request");'
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
```

本次 2026-06-08 针对 JSBridge request packet sequencing 和 `@vesty/plugin-ui` Node 测试脚手架通过:

```bash
npm test
cargo test -p vesty-ui-wry
cargo check -p vesty-ui-wry --features wry-backend
npm run typecheck
npm run build
cargo fmt --all --check
```

新增行为:

- `@vesty/plugin-ui` 与 wry bootstrap 现在对每个 JS request 使用同一个递增值生成 `id` 后缀和 `seq`；首包为 `id=js-1, seq=1`，readyAck 为 `id=js-2, seq=2`。
- `packages/plugin-ui/tests/bridge.test.mjs` 固定验证普通 request sequencing、`bridge.hello` typed payload、editor session adoption 和 `bridge.readyAck` sequencing；root `npm test` 会跑所有 workspace package test。

本次 2026-06-08 针对 GitHub Actions JS SDK test gate 通过:

```bash
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
npm test
npm run typecheck
npm run build
cargo fmt --all --check
```

新增行为:

- `.github/workflows/ci.yml` 的 `js sdk` job 已在 `npm run typecheck` 和 `npm run build` 之间加入 `npm test`，确保 `@vesty/plugin-ui` 的 bridge sequencing Node 测试进入 CI。

本次 2026-06-08 针对 `HostChangeFlags` 和 VST3 dynamic latency restart notification 重新通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3 --features vst3-bindings controller_notifies_host_when_latency_affecting_param_changes
cargo test -p vesty -p vesty-core
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-smoke
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-web-smoke
cargo run -p vesty-cli -- package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/vesty-synth-smoke
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-smoke/VestyGain.vst3
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-web-smoke/VestyWebUIDemo.vst3
target/steinberg/vst3sdk-build-xcode/bin/Release/validator target/vesty-synth-smoke/VestyMIDISynth.vst3
cargo run -p vesty-cli -- validate target/vesty-smoke/VestyGain.vst3
cargo run -p vesty-cli -- validate target/vesty-web-smoke/VestyWebUIDemo.vst3
cargo run -p vesty-cli -- validate target/vesty-synth-smoke/VestyMIDISynth.vst3
```

本次 2026-06-08 针对 JSBridge UI unload cleanup、async event pump 停止和 pending request reject 重新通过:

```bash
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json
cargo fmt --all --check
cargo test -p vesty-ui-wry
cargo test -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
```

本次 2026-06-08 针对 IPC message size limit 和 lane-specific backpressure 重新通过:

```bash
cargo fmt --all --check
cargo test -p vesty-ipc
cargo test -p vesty-bridge size
cargo test --workspace
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
```

本次 2026-06-08 针对 editor session handshake、`pending` -> `editorSessionId` promotion 和 stale session rejection 重新通过:

```bash
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json
cargo fmt --all --check
cargo test -p vesty-bridge hello_promotes_pending_session_to_editor_session
cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions
npm test
npm run typecheck
node --input-type=module -e 'import { createBridge } from "./packages/plugin-ui/dist/index.js"; const sent = []; const host = { ipc: { postMessage(message) { const packet = JSON.parse(message); sent.push(packet); if (packet.type === "bridge.hello") queueMicrotask(() => host.__VESTY_INTERNAL__.deliver({ v: 1, session: packet.session, seq: 1, lane: packet.lane, kind: "response", type: "bridge.hello.response", replyTo: packet.id, payload: { protocolVersion: 1, editorSessionId: "editor-js-test" } })); if (packet.type === "bridge.readyAck") queueMicrotask(() => host.__VESTY_INTERNAL__.deliver({ v: 1, session: packet.session, seq: 2, lane: packet.lane, kind: "response", type: "bridge.readyAck.response", replyTo: packet.id, payload: { ready: true } })); } }, addEventListener() {} }; const bridge = createBridge(host, "pending", { timeoutMs: 0 }); await bridge.ready(); void bridge.request("after.ready", {}).catch(() => {}); if (sent[0].session !== "pending") throw new Error(`hello used wrong session: ${sent[0].session}`); if (sent[1].type !== "bridge.readyAck" || sent[1].session !== "editor-js-test") throw new Error("readyAck did not use editor session"); if (sent[2].session !== "editor-js-test") throw new Error(`next request used wrong session: ${sent[2].session}`); console.log("plugin-ui adopts editor session and sends readyAck");'
cargo test --workspace
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
```

新增行为:

- `@vesty/plugin-ui` 的 `deliver` 会先比较入站 packet session；stale response 不会 resolve/reject 当前 pending request，stale event 不会触发 subscriber。
- wry bootstrap 内置 `window.__VESTY_INTERNAL__.deliver` 也使用相同 session 过滤，保持无框架 UI 和 SDK UI 行为一致。

本次 2026-06-08 针对 typed `bridge.hello` payload 和 protocol version negotiation 重新通过:

```bash
cargo fmt --all --check
cargo test -p vesty-ipc validates_hello_payload_shape
cargo test -p vesty-bridge hello
cargo test -p vesty-ui-wry bootstrap_script_registers_host_subscriptions
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json
node --input-type=module -e 'import { createBridge } from "./packages/plugin-ui/dist/index.js"; const sent = []; const host = { location: { href: "vesty://assets/index.html" }, ipc: { postMessage(message) { const packet = JSON.parse(message); sent.push(packet); if (packet.type === "bridge.hello") queueMicrotask(() => host.__VESTY_INTERNAL__.deliver({ v: 1, session: packet.session, seq: 1, lane: packet.lane, kind: "response", type: "bridge.hello.response", replyTo: packet.id, payload: { protocolVersion: 1, editorSessionId: "editor-js-test" } })); if (packet.type === "bridge.readyAck") queueMicrotask(() => host.__VESTY_INTERNAL__.deliver({ v: 1, session: packet.session, seq: 2, lane: packet.lane, kind: "response", type: "bridge.readyAck.response", replyTo: packet.id, payload: { ready: true } })); } }, addEventListener() {} }; const bridge = createBridge(host, "pending", { timeoutMs: 0 }); await bridge.ready(); if (sent[0].type !== "bridge.hello") throw new Error("missing hello"); const payload = sent[0].payload; if (!Array.isArray(payload?.supportedProtocolVersions) || !payload.supportedProtocolVersions.includes(1)) throw new Error("hello payload missing protocol v1"); if (payload.jsPackageVersion !== "0.1.0") throw new Error(`wrong js package version: ${payload.jsPackageVersion}`); if (payload.pageUrl !== "vesty://assets/index.html") throw new Error(`wrong pageUrl: ${payload.pageUrl}`); if (sent[1].type !== "bridge.readyAck" || sent[1].session !== "editor-js-test") throw new Error("readyAck did not use editor session"); void bridge.request("after.ready", {}).catch(() => {}); if (sent[2].session !== "editor-js-test") throw new Error(`next request used wrong session: ${sent[2].session}`); console.log("plugin-ui ready sends typed hello payload, adopts editor session and sends readyAck");'
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
```

本次 2026-06-08 针对 JS SDK/wry bootstrap ready protocol guard 通过:

```bash
cargo fmt --all --check
npm test --prefix packages/plugin-ui
cargo test -p vesty-ui-wry
cargo check -p vesty-ui-wry --features wry-backend
```

新增行为:

- `@vesty/plugin-ui` 的 `ready()` 现在会拒绝缺失或非 `1` 的 `protocolVersion`，返回 `unsupported_version` 且 `retryable = false`。
- JS SDK 在不兼容 hello response 下不会采纳 `editorSessionId`，也不会发送 `bridge.readyAck`；测试覆盖 `protocolVersion: 2` 时 posted packet 只包含 `bridge.hello`。
- wry 注入 bootstrap 与 JS SDK 保持相同行为，内置 `assertCompatibleReadyPayload()` 和 `unsupportedProtocolError()`。

本次 2026-06-08 针对 `bridge.ready` capabilities schema 重新通过:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge hello_returns_ready_payload
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-ui-wry --features wry-backend
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-smoke
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-web-smoke
cargo run -p vesty-cli -- package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/vesty-synth-smoke
cargo run -p vesty-cli -- validate target/vesty-smoke/VestyGain.vst3
cargo run -p vesty-cli -- validate target/vesty-web-smoke/VestyWebUIDemo.vst3
cargo run -p vesty-cli -- validate target/vesty-synth-smoke/VestyMIDISynth.vst3
```

本次 2026-06-08 针对 bridge subscription/param gesture queue backpressure 重新通过:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge subscription
cargo test -p vesty-bridge pending_param_gesture_queue_full_returns_backpressure
cargo test -p vesty-bridge param_
cargo test --workspace
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
```

本次补强确认 `param.perform` 在同一参数的 begin/end gesture 边界内会 latest-wins coalesce，连续 perform 不会无限增长 pending gesture queue；`param.end` 仍会在满队列时优先丢弃旧 perform 并收尾。

本次 2026-06-08 针对 `state.setConfig` config schema/entry backpressure 重新通过，并抽查 crates.io 主依赖仍与 workspace baseline 一致:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge state
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo search wry --limit 1
cargo search vst3 --limit 1
cargo search clap --limit 1
cargo search ts-rs --limit 1
cargo search raw-window-handle --limit 1
cargo search rtrb --limit 1
cargo search serde --limit 1
cargo search serde_json --limit 1
```

本次 2026-06-08 针对 persistent UI state 写入面补齐通过:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge state_set_ui_state
cargo test -p vesty-ui-wry bootstrap_script_registers_host_subscriptions
npm run typecheck
npm run build
npm test
```

新增行为:

- `state.setUiState` 是 typed state command，payload 为 `{ baseRevision, value }`，其中 `baseRevision` 对应当前 `uiRevision`。
- 成功提交会整块替换 `PluginSnapshot.uiState`，并递增 `revision` / `uiRevision`。
- stale `uiRevision` 返回 retryable `state_conflict`，error details 带最新 snapshot。
- `@vesty/plugin-ui` 与 wry bootstrap 都暴露 `setUiState(value, baseRevision)`。

本次 2026-06-08 针对 `state.changed` snapshot event 闭环通过:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge state
cargo test -p vesty-bridge
cargo clippy -p vesty-bridge --all-targets -- -D warnings
```

新增行为:

- `state.setConfig` / `state.setUiState` 成功后先返回 request response，再按订阅表发送 `state.changed` event。
- `state.changed` payload 是完整 `PluginSnapshot`，可被 `@vesty/plugin-ui` 的 `createSnapshotStore()` 直接发布。
- 未订阅 `state.changed` 时仍只发送 request response，保持 reliable event 订阅过滤语义。

本次 2026-06-08 针对 JSBridge config/UI state 的 VST3 controller 持久化通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' controller_wry_bridge_state_roundtrips_through_vst3_state
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo clippy -p vesty-vst3 --all-targets --features 'vst3-bindings wry-ui' -- -D warnings
```

新增行为:

- VST3 state payload 新增独立 `bridge` JSON 字段，保存 JSBridge `PluginSnapshot`，不占用开发者 `custom` state。
- wry bridge runtime 每次处理 IPC batch 后把当前 snapshot 写回 controller 共享槽。
- controller `getState()` 写入 `bridge` snapshot；新 controller `setState()` 恢复后，新的 editor `bridge.hello` ready payload 会携带恢复后的 config/UI state 和 revision。

本次 2026-06-08 针对 `param.changed` UI confirmed event 闭环通过:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge param_changed_events_use_subscription_filter_and_revision
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' controller_wry_bridge_emits_param_changed_after_ui_perform
npm run typecheck
npm run build
npm test
```

新增行为:

- `vesty-bridge::emit_param_changed()` 使用订阅表过滤 `param.changed`，并生成 `ParamChangedEvent` payload。
- 每次 param changed event 会递增 bridge snapshot 的 `revision` / `paramsRevision`。
- VST3 wry bridge 的 UI `param.perform` relay 在 host `performEdit()` 返回 OK 后，会回推 `param.changed`，payload 包含 `source = "ui"` 和原始 `gestureId`，用于前端 echo suppression。
- `@vesty/plugin-ui` 同步导出 `ParamChangedEvent` 和 `ParamChangeSource` TypeScript 类型。

本次 2026-06-08 针对 host/controller -> Web UI `param.changed` 闭环通过:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge event_flush_request_is_supported
cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' controller_wry_bridge_emits_host_param_changes_on_event_flush
npm test
```

新增行为:

- `vesty-bridge` 支持轻量 `event.flush` request，供 UI/control 线程 drain 异步事件源。
- `vesty-vst3` controller 在非实时路径 coalesce host/controller 参数变化；订阅 `param.changed` 前的 latest-wins 值会保留到订阅建立后，订阅后的 host 参数变化会在下一次 `event.flush` 回推。
- `@vesty/plugin-ui` 与 wry bootstrap 的 pump 从 meter-only 扩展到 async event pump，覆盖 `meter.*`、`param.changed`、`diagnostics.fault` 和 `log.rt`。

本次 2026-06-08 针对 Rust IPC -> TypeScript/JSON Schema protocol export 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-ipc exports_protocol_types_and_json_schema
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-smoke
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-smoke --check
npx -y -p typescript@latest tsc --strict --noEmit --target ES2022 --module ESNext --moduleResolution Bundler $(find target/vesty-protocol-smoke/typescript -name '*.ts' | sort)
```

本次 2026-06-08 针对 protocol export drift gate 通过:

```bash
cargo test -p vesty-cli protocol_export_check_detects_snapshot_drift
rm -rf target/vesty-protocol-check-smoke
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-check-smoke
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-check-smoke --check
```

本次 2026-06-08 针对 DAW matrix evidence template 通过:

```bash
cargo test -p vesty-cli daw_evidence_templates_do_not_count_as_pass_or_overwrite_logs
cargo run -p vesty-cli -- daw-matrix --write-template --reaper-evidence target/daw-template-smoke/reaper --cubase-evidence target/daw-template-smoke/cubase --bitwig-evidence target/daw-template-smoke/bitwig --ableton-evidence target/daw-template-smoke/ableton --studio-one-evidence target/daw-template-smoke/studio-one --format markdown
```

本次 2026-06-08 针对 DAW matrix strict release gate 通过:

```bash
cargo test -p vesty-cli daw_matrix_complete_requires_every_smoke_check_to_pass
cargo run -p vesty-cli -- daw-matrix --help
cargo run -p vesty-cli -- daw-matrix --format json --strict > target/daw-matrix-strict-smoke.json
python3 -m json.tool target/daw-matrix-strict-smoke.json >/dev/null
```

当前本机只有 REAPER evidence，因此 `--strict` smoke 预期返回非零，错误为 `DAW matrix is incomplete; collect all host smoke evidence before release`。这证明 release gate 不会把 missing Cubase/Bitwig/Ableton/Studio One 当成 pass。

本次 2026-06-08 针对 `vesty validate` 静态 bundle/resource check 通过:

```bash
cargo test -p vesty-build validates_packaged_vst3_bundle
cargo test -p vesty-build validation
cargo build -p vesty-example-gain --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-validate-smoke
cargo run -p vesty-cli -- validate target/vesty-validate-smoke/VestyGain.vst3
cargo build -p vesty-example-web-ui-param-demo --release
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-web-validate-smoke
cargo run -p vesty-cli -- validate target/vesty-web-validate-smoke/VestyWebUIDemo.vst3
cargo run -p vesty-cli -- validate target/vesty-web-validate-smoke/VestyWebUIDemo.vst3 --static-only
```

本次 2026-06-08 针对 `vesty validate` / `vesty doctor` JSON report 通过:

```bash
cargo test -p vesty-cli output_format_accepts_text_and_json_aliases
cargo test -p vesty-cli validate_report_serializes_static_and_validator_status
cargo test -p vesty-cli validator_summary_extracts_passed_and_failed_counts
cargo test -p vesty-cli doctor_report_includes_toolchain_webview_and_validator_checks
cargo test -p vesty-cli command_presence_check_uses_candidate_paths_and_missing_hint
cargo run -p vesty-cli -- validate target/vesty-validate-smoke/VestyGain.vst3 --format json
cargo run -p vesty-cli -- validate target/vesty-web-validate-smoke/VestyWebUIDemo.vst3 --static-only --format json
cargo run -p vesty-cli -- doctor --format json
```

`vesty doctor` report 现在包含 release signing/notarization 前置工具检查；这些 checks 只证明工具可发现，不替代真实 signed/notarized artifact。

本次 2026-06-08 针对 `[package].signing` package command 接线通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli bundle_signing_command_maps_platforms
cargo test -p vesty-cli
```

`vesty package` 现在会在 `[package].signing` 非空时执行平台签名命令；测试覆盖 macOS `codesign`、Windows `signtool.exe` 和 Linux 外部签名提示。真实证书签名、公证和安装包签名仍需要发布流水线 artifact 证明。

本次 2026-06-08 针对 `[package].category` moduleinfo 接线通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build moduleinfo_uses_package_category_when_present
cargo test -p vesty-build
```

`vesty package` 现在会把 `[package].category` 写入 `Contents/Resources/moduleinfo.json` 的 class category，缺失或空值时按 `[plugin].kind` 映射为 `Fx` 或 `Instrument`。

本次 2026-06-08 针对 `vesty notarize` command planning 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli notarization_credentials_reject_missing_or_mixed_modes
cargo test -p vesty-cli notarization_plan_builds_archive_submit_and_staple_commands
cargo test -p vesty-cli notarization_plan_supports_apple_id_without_wait_or_staple
cargo test -p vesty-cli
```

`vesty notarize` 现在可规划 macOS notarization workflow: `ditto` 生成 notary zip、`xcrun notarytool submit` 支持 keychain profile 或 Apple ID credentials，默认 wait 后 stapler；`--no-wait` 必须配合 `--no-staple`。真实 Apple notarization 仍需要有效凭据和发布流水线 artifact 证明。

本次 2026-06-08 针对 `ParamHandle` 实时参数读取 API 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-params param_handles_resolve_and_access_values
cargo test -p vesty params_derive_implements_param_collection
cargo test -p vesty-core copies_audio_with_gain
cargo test -p vesty-cli new_project_templates_use_params_derive
cargo test -p vesty-vst3 --features vst3-bindings processor_translates_automation_midi_and_transport
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

三个 example 和 `vesty new` Rust 模板已改为在 `create_kernel()` 中 resolve `ParamHandle`，audio `process()` 中使用 `ProcessContext::param_normalized()`，避免示例鼓励 process-time 字符串查找。VST3 automation event 也携带同一套 `ParamHandle`。

本次 2026-06-08 针对 JSBridge `bridge.readyAck` handshake 通过:

```bash
cargo test -p vesty-bridge ready_ack_marks_runtime_ready
cargo test -p vesty-bridge hello_promotes_pending_session_to_editor_session
cargo test -p vesty-ui-wry renders_batch_delivery_once
cargo test -p vesty-ui-wry bootstrap_script_registers_host_subscriptions
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json
node --input-type=module -e 'import { createBridge } from "./packages/plugin-ui/dist/index.js"; const sent = []; const host = { location: { href: "vesty://assets/index.html" }, ipc: { postMessage(message) { const packet = JSON.parse(message); sent.push(packet); if (packet.type === "bridge.hello") queueMicrotask(() => host.__VESTY_INTERNAL__.deliver({ v: 1, session: packet.session, seq: 1, lane: packet.lane, kind: "response", type: "bridge.hello.response", replyTo: packet.id, payload: { protocolVersion: 1, editorSessionId: "editor-js-test" } })); if (packet.type === "bridge.readyAck") queueMicrotask(() => host.__VESTY_INTERNAL__.deliver({ v: 1, session: packet.session, seq: 2, lane: packet.lane, kind: "response", type: "bridge.readyAck.response", replyTo: packet.id, payload: { ready: true } })); } }, addEventListener() {} }; const bridge = createBridge(host, "pending", { timeoutMs: 0 }); await bridge.ready(); if (sent[0]?.type !== "bridge.hello") throw new Error("missing hello"); if (sent[1]?.type !== "bridge.readyAck") throw new Error(`missing readyAck: ${sent[1]?.type}`); if (sent[1].session !== "editor-js-test") throw new Error(`readyAck used wrong session: ${sent[1].session}`); void bridge.request("after.ready", {}).catch(() => {}); if (sent[2]?.session !== "editor-js-test") throw new Error(`next request used wrong session: ${sent[2]?.session}`); console.log("plugin-ui ready sends readyAck and keeps editor session");'
```

本次 2026-06-08 针对 React/Vue/Svelte UI templates 通过:

```bash
cargo test -p vesty-cli ui_templates_emit_framework_specific_files
rm -rf target/vesty-template-smoke
mkdir -p target/vesty-template-smoke
cd target/vesty-template-smoke
cargo run -p vesty-cli -- new demo-react --ui react
cargo run -p vesty-cli -- new demo-vue --ui vue
cargo run -p vesty-cli -- new demo-svelte --ui svelte
cd /Users/orchiliao/Projects/vesty
for app in demo-react demo-vue demo-svelte; do npm pkg set 'dependencies.@vesty/plugin-ui=file:../../../../packages/plugin-ui' --prefix target/vesty-template-smoke/$app/ui >/dev/null; npm install --prefix target/vesty-template-smoke/$app/ui; npm run build --prefix target/vesty-template-smoke/$app/ui; done
```

本次 2026-06-08 针对 vanilla/React/Vue/Svelte UI templates 最新依赖 typecheck 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
rm -rf target/scaffold-ui-smoke
mkdir -p target/scaffold-ui-smoke
cd target/scaffold-ui-smoke
cargo run -p vesty-cli -- new demo-vanilla --ui vanilla --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
cargo run -p vesty-cli -- new demo-react --ui react --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
cargo run -p vesty-cli -- new demo-vue --ui vue --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
cargo run -p vesty-cli -- new demo-svelte --ui svelte --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
cd /Users/orchiliao/Projects/vesty
for project in demo-vanilla demo-react demo-vue demo-svelte; do npm install --prefix target/scaffold-ui-smoke/$project/ui; npm run build --prefix target/scaffold-ui-smoke/$project/ui; npm run typecheck --prefix target/scaffold-ui-smoke/$project/ui; cargo check --manifest-path target/scaffold-ui-smoke/$project/Cargo.toml; done
```

新增行为:

- React scaffold devDependencies 增加 `@types/react` 和 `@types/react-dom`，保持最新 React + strict TypeScript 可检查。
- Vue scaffold 的 `npm run typecheck` 改为 `vue-tsc --noEmit`，能正确检查 `.vue` 单文件组件。
- Svelte scaffold 的 `npm run typecheck` 改为 `svelte-check --tsconfig ./tsconfig.json`，能正确检查 `.svelte` 组件。
- CLI 单元测试 `ui_templates_emit_framework_specific_files` 固定验证上述 typecheck 脚本和 devDependencies，防止模板回退成只 build 不 typecheck。

本次 2026-06-08 针对 GitHub Actions scaffold matrix 补强并通过:

```bash
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'
cargo fmt --all --check
rm -rf target/ci-scaffold-local
mkdir -p target/ci-scaffold-local
cd target/ci-scaffold-local
for UI_TEMPLATE in none vanilla react vue svelte; do cargo run -p vesty-cli -- new "demo-${UI_TEMPLATE}" --ui "${UI_TEMPLATE}" --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui; cargo check --manifest-path "demo-${UI_TEMPLATE}/Cargo.toml"; if [ "${UI_TEMPLATE}" != "none" ]; then npm install --prefix "demo-${UI_TEMPLATE}/ui"; npm run build --prefix "demo-${UI_TEMPLATE}/ui"; npm run typecheck --prefix "demo-${UI_TEMPLATE}/ui"; fi; done
```

新增行为:

- `.github/workflows/ci.yml` 的 `scaffold smoke` job 改为 `matrix.ui = none, vanilla, react, vue, svelte`。
- 每个 matrix entry 都用本地 workspace path dependency 生成独立插件并跑 `cargo check`。
- UI 模板 entry 会额外跑 `npm install`、`npm run build`、`npm run typecheck`，让 CI 能直接防住 React/Vue/Svelte latest 依赖或类型工具链漂移。

本次 2026-06-08 针对 `vesty new` package metadata 补齐并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli vesty_toml_template_includes_package_metadata
rm -rf target/package-metadata-scaffold-smoke
mkdir -p target/package-metadata-scaffold-smoke
cd target/package-metadata-scaffold-smoke
cargo run -p vesty-cli -- new my-gain --kind effect --ui none --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty
cargo run -p vesty-cli -- new my-synth --kind instrument --ui none --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty
cd /Users/orchiliao/Projects/vesty
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

新增行为:

- `vesty new` 生成的 `vesty.toml` 现在包含 `[package]`，不再只生成 `[plugin]` 和可选 `[ui]`。
- effect scaffold 默认写入 `bundle_id = "dev.vesty.<crate-name>"` 和 `category = "Fx"`。
- instrument scaffold 默认写入同样的 bundle id 规则和 `category = "Instrument"`。
- CLI 单元测试 `vesty_toml_template_includes_package_metadata` 固定验证这些默认值；本机 smoke 已确认 `my-gain/vesty.toml` 和 `my-synth/vesty.toml` 生成结果。

本次 2026-06-08 针对 generated Rust scaffold local path compile 通过:

```bash
cargo test -p vesty-cli cargo_template_can_use_local_vesty_path
rm -rf target/vesty-rust-template-smoke
mkdir -p target/vesty-rust-template-smoke
cd target/vesty-rust-template-smoke
cargo run -p vesty-cli -- new demo-effect --ui none --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty
cargo check --manifest-path demo-effect/Cargo.toml
cargo run -p vesty-cli -- new demo-ui-effect --ui vanilla --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty
cargo check --manifest-path demo-ui-effect/Cargo.toml
```

本次 2026-06-08 针对 generated scaffold local Rust + local `@vesty/plugin-ui` path 通过:

```bash
cargo test -p vesty-cli ui_package_template_can_use_local_plugin_ui_path
rm -rf target/vesty-new-local-ui-smoke
mkdir -p target/vesty-new-local-ui-smoke
cd target/vesty-new-local-ui-smoke
cargo run -p vesty-cli -- new demo-local-ui --ui vanilla --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
node -e 'const fs=require("fs"); const pkg=JSON.parse(fs.readFileSync("demo-local-ui/ui/package.json","utf8")); if(pkg.dependencies["@vesty/plugin-ui"]!=="file:/Users/orchiliao/Projects/vesty/packages/plugin-ui") process.exit(1);'
cargo check --manifest-path demo-local-ui/Cargo.toml
npm install --prefix demo-local-ui/ui
npm run build --prefix demo-local-ui/ui
```

本次 2026-06-08 针对 GitHub Actions CI workflow 通过本机静态解析和对应命令集回归:

```bash
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo check -p vesty-ui-wry --features wry-backend
cargo run -p vesty-cli -- validate target/vesty-validate-smoke/VestyGain.vst3 --format json
cargo run -p vesty-cli -- doctor --format json
```

CI workflow 自身仍需在 GitHub runner 上首次执行后补充实际 run URL；商业 DAW smoke 和 Windows/Linux host smoke 不由该 workflow 伪造。

本次 2026-06-08 针对 JSBridge diagnostics、VST3 fault telemetry、RT log bridge、`vesty validate --report/--validator-log` 和 `vesty doctor` DAW install checks 通过:

```bash
cargo test -p vesty-cli
cargo test -p vesty-ipc -p vesty-bridge -p vesty-ui-wry
cargo test -p vesty-vst3 --features vst3-bindings
cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
npm run typecheck --workspace packages/plugin-ui
npm run build --workspace packages/plugin-ui
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-rtlog-smoke
```

新增协议面:

- JS API: `getDiagnostics()`。
- bridge request: `diagnostics.get`。
- subscribable topic: `diagnostics.fault`。
- subscribable topic: `log.rt`。
- exported protocol types: `BridgeDiagnosticsSnapshot`、`PluginFaultReport`、`RtLogRecord`。

本次 2026-06-08 针对 host profile/quirk registry 和 `vesty host-quirks` 通过:

```bash
cargo test -p vesty-core -p vesty-cli
cargo run -p vesty-cli -- host-quirks --host bitwig --format json
cargo run -p vesty-cli -- host-quirks --format markdown
```

该 registry 只用于准备 release smoke 和标记 host 注意事项；商业 DAW 真实通过状态仍以 `vesty daw-matrix` evidence 为准。

本次 2026-06-08 针对 DAW evidence host-specific README 模板通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli host_evidence_readme_includes_profile_checks_and_quirks
cargo test -p vesty-cli daw_evidence_templates_do_not_count_as_pass_or_overwrite_logs
cargo test -p vesty-cli generic_daw_evidence_defaults_to_missing_when_dir_absent
cargo test -p vesty-cli
cargo run -p vesty-cli -- release-check --skip-protocol --format json
```

`vesty daw-matrix --write-template` 现在会在每个 host evidence 目录生成包含 required smoke checks、platform、notes 和 quirk/mitigation 表的 README；已有 README 和日志仍不会被覆盖，pending 模板值仍不会被判定为 pass。缺失 host 的 release-check hint 会指向默认 evidence 目录，例如 `target/cubase-smoke`，而不是不可操作的占位描述。

本次 2026-06-08 针对 `vesty release-check` 聚合 release gate 通过:

```bash
cargo test -p vesty-cli release_check
cargo run -p vesty-cli -- release-check --skip-protocol --format json > target/release-check-smoke.json
python3 -m json.tool target/release-check-smoke.json >/dev/null
cargo run -p vesty-cli -- release-check --skip-protocol --format markdown > target/release-check-smoke.md
cargo run -p vesty-cli -- release-check --skip-protocol --format json --strict > target/release-check-strict-smoke.json
python3 -m json.tool target/release-check-strict-smoke.json >/dev/null
```

当前本机 release-check status 预期为 `failed`，因为只有 REAPER evidence 完整，Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One 仍缺真实 smoke。`--strict` 预期返回非零，用于防止 incomplete release。

本次 2026-06-08 针对 release UI asset manifest integrity 重新通过:

```bash
cargo fmt --all --check
cargo test -p vesty-ui-wry --features wry-backend asset_protocol_uses_manifest_allowlist
cargo test -p vesty-ui-wry --features wry-backend release_navigation_only_allows_bundle_asset_urls
cargo test -p vesty-ui-wry --features wry-backend release_ipc_only_allows_bundle_asset_urls
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npx -y -p typescript@latest tsc -p packages/plugin-ui/tsconfig.json --noEmit
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-smoke
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-web-smoke
cargo run -p vesty-cli -- package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/vesty-synth-smoke
cargo run -p vesty-cli -- validate target/vesty-smoke/VestyGain.vst3
cargo run -p vesty-cli -- validate target/vesty-web-smoke/VestyWebUIDemo.vst3
cargo run -p vesty-cli -- validate target/vesty-synth-smoke/VestyMIDISynth.vst3
```

本次 2026-06-08 针对 release UI asset manifest fail-closed 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-ui-wry --features wry-backend
cargo check -p vesty-ui-wry --features wry-backend
cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings
```

新增行为:

- `vesty-ui-wry` release asset mode 不再在缺失/无效 `assets.manifest.json` 时 fallback 到 root-canonical 文件服务。
- release `attach()` 必须成功加载 manifest；缺失 manifest 返回 `NotFound`，损坏 JSON 返回 `InvalidData` 并映射为 `UiError::RuntimeUnavailable`。
- manifest 运行时加载会 fail-fast 校验 entry、非空 files、安全 path、重复 path、空 mime 和 64 位 hex sha256。
- custom protocol 仍只服务 manifest allowlist，继续校验 canonical root、`size` 和 `sha256`；篡改或 manifest 外请求返回 404。

本次 2026-06-08 针对 `vesty-build` 静态 UI manifest metadata validation 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build
cargo clippy -p vesty-build --all-targets -- -D warnings
```

新增行为:

- `validate_vst3_bundle()` 现在会在静态检查阶段拒绝 unsafe `manifest.entry`、重复 asset path、空 mime 和非 64 位 hex sha256。
- asset sha256 比对改为大小写不敏感，但仍要求 manifest sha256 是合法 hex digest。
- `validation_rejects_malformed_asset_manifest_metadata` fixture 覆盖 missing entry、unsafe path、duplicate path 和 malformed sha。

`vesty doctor` 当前输出结论:

- Rust toolchain: found。
- Node/npm: found。
- System WebView: macOS WebKit.framework found。
- VST3 validator: found at `target/steinberg/vst3sdk-build-xcode/bin/Release/validator`。

Validator 结果:

- `VestyGain.vst3`: 47 tests passed, 0 tests failed。
- `VestyWebUIDemo.vst3`: 47 tests passed, 0 tests failed。
- `VestyMIDISynth.vst3`: 47 tests passed, 0 tests failed。

## DAW smoke

REAPER 7.73/macOS-arm64 已完成一轮本机 smoke:

- Scan: `~/Library/Application Support/REAPER/reaper-vstplugins_arm64.ini` 中发现三个插件:
  - `Vesty Gain`
  - `Vesty Web UI Demo`
  - `Vesty MIDI Synth`
- Load: `target/reaper-smoke/load-smoke.log` 记录三个插件均可插入工程。
- Save/restore: `target/reaper-smoke/restore-smoke.log` 记录重开工程后 3 条 track 和 3 个 VST3 FX 均恢复。
- UI: `target/reaper-smoke/ui-smoke.log` 记录 `TrackFX_Show` 成功打开 `Vesty Web UI Demo`，Computer Use 可访问性树确认 WebView URL 为 `vesty://assets/index.html`，不是 localhost dev server。
- UI screenshot: `/tmp/reaper-ui-smoke-after-fix.png`，可见打包 UI 内的 `Mix` slider 和当前值。
- Rust->JS bridge response: `examples/web-ui-param-demo/ui/dist/index.js` 在加载时调用 `bridge.ready()`；REAPER 可访问性树和 `/tmp/reaper-ui-ready-response.png` 显示 `ready:Vesty Web UI Demo`，证明 `bridge.hello` response 已经从 Rust 回推到 JS Promise。
- Meter stream: 使用 `VESTY_UI_DEV=0 VESTY_BRIDGE_TRACE=target/reaper-smoke/bridge-trace-meter.log` 启动 REAPER 加载 `target/reaper-smoke/vesty-reaper-load-smoke.rpp`，Web UI 自动订阅 `meter.main` 并连续发送 `meter.flush`；`target/reaper-smoke/meter-stream.log` 记录多次 `meter_flush sent=1`，证明 UI 端 60 Hz meter stream 已经从 RT SPSC 经 controller/bridge 到达 WebView。
- UI->Host bridge: `target/reaper-smoke/bridge-trace.log` 记录真实 WebView `window.ipc.postMessage` 发出的 `param.begin`、多次 `param.perform`、`param.end`，Rust bridge 全部 relay 到 VST3 `IComponentHandler` 且 result=0。
- Host param watch: `target/reaper-smoke/param-watch.log` 在同一 REAPER 进程内用 ReaScript 轮询 `TrackFX_GetParamNormalized`，拖动 Web UI slider 后 host 参数从 `0.5` 跟随到 `0.88899999856949`。
- Host automation/offline render: `target/reaper-smoke/render-smoke.log` 记录 `Vesty MIDI Synth` 插入成功、参数 0 初值 0.75、3 个 automation points、1 个 MIDI note；命令行 `-renderproject` 产出 `/tmp/vesty-reaper-offline-render.wav`。
- Render output: `afinfo` 显示 1.5s、stereo、48 kHz、24-bit PCM，音频数据 432000 bytes；十六进制非零数据计数为 623464，确认不是全零空文件。
- Matrix summary: 旧 `target/daw-matrix.md` / `target/daw-matrix.json` 中 REAPER 行 scan/load/UI/UI->Host/meter stream/automation/save-restore/offline render 均为 pass，其它 DAW 为 missing。2026-06-10 起 strict DAW matrix 还要求 `buffer-sample-rate.log` / `buffer_sample_rate_change=true` 证据；旧 REAPER smoke 需补采 buffer size/sample-rate change 后才能满足新九项 gate。UI->Host pass 当前由 `param-watch.log` 证明 host normalized parameter 从 0.5 移动到 0.88899999856949；meter stream pass 当前由 `target/reaper-smoke/meter-stream.log` 的 `meter_flush sent=1` 证明。

这次 smoke 同时验证了 release UI asset 路径修复: release VST3 bundle 默认加载 `vesty://assets/index.html`，只有 debug build 或显式 `VESTY_UI_DEV=1/true/yes/on` 才使用 `dev_url`。

本机 `/Applications` 当前只发现 `/Applications/REAPER.app`；未发现 Cubase/Nuendo、Bitwig Studio、Ableton Live 或 Studio One，因此这些 host 的 smoke 仍需外部机器或安装后补采证据。

## Release artifact evidence gate

`vesty release-check` 现在除了 DAW matrix、host profile 覆盖、protocol snapshot 和 VST3 binding baseline 之外，还能聚合外部发布证据:

- `--ci-run-url`: 验证 GitHub Actions run URL 形态，要求指向 `https://github.com/.../actions/runs/...`。
- `--ci-doctor-dir`: 递归读取 CI 上传/下载的 doctor JSON artifacts，要求覆盖 `doctor-Linux.json`、`doctor-macOS.json`、`doctor-Windows.json`，并检查每个平台 report 至少包含 toolchain、Node/npm、VST3 binding baseline、VST3 validator、system WebView 和对应 signing/notarization 前置检查。
- `--release-evidence-dir`: 先拒绝 symlinked evidence root，再按 `--write-evidence-template` 的目录约定自动发现 `ci-run-url.txt`、`ci-doctor/`、`validate-report.json`、`static-validate-report.json`、`signing-macos.log`、`signing-windows.log` 和 `notary.log`；同时扫描目录内其它 `*.json` validate reports，并按内容分类为 release validator evidence 或 CI static validate smoke。显式传入的单项参数仍优先。
- `--static-validate-report`: 可重复读取 `vesty validate --static-only --report <path>` 生成的 JSON，要求静态 bundle check 为 `ok`、包含 moduleinfo/binary，并且显式 `static_check.binary_exports` 平台、required symbols、found symbols、status 和 skipped reason 自洽；该 check 只作为 CI packaging smoke，不替代 `--validate-report` 的 Steinberg validator passed 证据。Vesty framework release 的 `--require-release-artifacts` 会额外要求三示例乘三平台 static validate 覆盖，并要求每个示例/platform report 含匹配平台的完整 `ok` `static_check.binary_exports`。
- `--validate-report`: 可重复读取 `vesty validate --report <path>` 生成的 JSON，要求静态 bundle check 为 `ok`、包含 moduleinfo/binary，并且显式 `static_check.binary_exports` 自洽；同时 Steinberg validator status 必须为 `passed`、`exit_code = 0`、`tests_passed > 0`、`tests_failed = 0`；`--static-only` 的 skipped validator report 不算 release 通过证据。Vesty framework release 的 `--require-release-artifacts` 会额外要求 `VestyGain.vst3`、`VestyWebUIDemo.vst3` 和 `VestyMIDISynth.vst3` 在 `linux-x64`、`macos`、`windows-x64` 都有 validator-passed report，并要求每个示例/platform report 含匹配平台的完整 `ok` `static_check.binary_exports`。
- `VestyWebUIDemo.vst3` 的 validator/static example coverage 额外要求 static check report 中包含 UI `asset_manifest` 且 `asset_count > 0`，避免空 UI bundle report 被当成 Web UI 示例覆盖证据。
- `--signed-bundle-evidence`: 可重复传入 codesign/signtool verification log，或 macOS signed `.vst3` bundle；日志必须包含明确通过 marker，避免把普通 bundle 路径误当签名证据。
- `--notarization-log`: 检查 accepted notarytool/stapler 证据。
- `--require-release-artifacts`: 将缺失的 CI/validate/static validate/signing/notarization evidence 从默认 `skipped` 提升为 `failed`，用于真正 release gate；最终 gate 同时要求 protocol snapshot drift check 参与，传入 `--skip-protocol` 会直接失败。
- `--report <path>`: 把完整 release readiness JSON report 写入文件，便于 CI artifact 上传或 release note 留档。
- `--write-evidence-template <dir>`: 生成 release artifact 证据模板，包括 README、CI run URL 占位、`ci-doctor/README.md`、pending validator validate report、static validate report、签名日志和 notary 日志；模板不会覆盖已有文件，pending 值和 CI doctor README 不会被判定为 pass。README 会给出 strict release gate 和 CI static packaging smoke 两组命令。

这些 checks 仍不替代真实 CI run、真实签名、公证和 DAW smoke；它们只是把这些外部证据纳入同一个机器可读 release report。

GitHub Actions Rust matrix 现在会在每个 OS 上传两类 artifact: `doctor-<OS>` 和非严格 `release-check-<OS>`。后者记录当前 readiness 状态，但因为默认没有 DAW/signed/notarized evidence，所以不能替代最终 release gate。

本次 2026-06-08 针对 release artifact evidence gate 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'
cargo run -p vesty-cli -- release-check --skip-protocol --format json
# Historical local smoke only: final --require-release-artifacts now rejects --skip-protocol.
cargo run -p vesty-cli -- release-check --skip-protocol --format markdown --report target/release-check-report-smoke.json > target/release-check-report-smoke.md
python3 -m json.tool target/release-check-report-smoke.json >/dev/null
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

本次 2026-06-08 针对 `vesty release-check --validate-report` evidence gate 通过:

```bash
cargo test -p vesty-cli release_check
cargo test -p vesty-cli release_evidence_templates
```

本次 2026-06-08 针对 CI static validate report 和 VST3 event ordering hardening 通过:

```bash
cargo test -p vesty-cli release_check_accepts_static_validate_reports_as_ci_smoke_only
cargo test -p vesty-cli release_check_rejects_failed_static_validate_reports
cargo test -p vesty-vst3 --features vst3-bindings events_are_sorted_by_sample_offset_stably
cargo test -p vesty-core automation_segments_ignore_other_handles_and_clamp_to_block
cargo test -p vesty-vst3 --features vst3-bindings
cargo test -p vesty-build moduleinfo
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs
cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths
cargo test -p vesty-cli daw_evidence_root_maps_standard_host_directories
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

本次 2026-06-08 针对 realtime param automation helper 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-core filters_param_automation_by_handle
cargo test -p vesty params_derive_implements_param_collection
cargo test -p vesty-cli new_project_templates_use_params_derive
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

本次 2026-06-08 针对 `vesty package --install-dev` 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli dev_install_mode
cargo test -p vesty-cli install_dev
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo build -p vesty-example-gain --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-install-smoke --install-dev --vst3-dir target/dev-vst3-smoke --install-mode copy
test -f target/dev-vst3-smoke/VestyGain.vst3/Contents/Resources/moduleinfo.json
```

本次 2026-06-08 针对 `vesty dev --install-dev` 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli bundle_platform_parser_accepts_release_targets
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo run -p vesty-cli -- dev --config examples/gain/vesty.toml --no-ui --install-dev --binary target/debug/libvesty_example_gain.dylib --platform macos --out target/vesty-dev-install-smoke --vst3-dir target/dev-vst3-dev-smoke --install-mode copy
test -f target/dev-vst3-dev-smoke/VestyGain.vst3/Contents/Resources/moduleinfo.json
```

本次 2026-06-08 针对 `vesty dev --install-dev` 自动 cdylib discovery 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo run -p vesty-cli -- dev --config examples/gain/vesty.toml --install-dev --no-ui --out target/dev-autobinary-smoke --vst3-dir target/dev-autobinary-install --install-mode copy
cargo run -p vesty-cli -- validate target/dev-autobinary-smoke/VestyGain.vst3 --static-only --format json --report target/dev-autobinary-smoke/VestyGain.static-validate.json
```

新增行为:

- `vesty dev --install-dev` 不传 `--binary` 时，会用 Cargo metadata 匹配当前项目 `Cargo.toml` 的 `cdylib` target 并推断 debug/release cdylib 路径。
- workspace member 场景下，Cargo metadata 可能没有 `root_package`；CLI 现在优先按当前 manifest path 选 package，再 fallback 到 `root_package` 或唯一 `workspace_default_members`。
- `--binary <path>` 仍然是显式 override；metadata 不能识别 plugin package 时会要求开发者传入 `--binary`。
- 本次 smoke 在 Vesty workspace 的 `examples/gain` 中成功推断 `target/debug/libvesty_example_gain.dylib`，生成 `target/dev-autobinary-smoke/VestyGain.vst3`，copy 到 `target/dev-autobinary-install/VestyGain.vst3`，并通过静态 validate: `static_check.status = ok`，macOS binary 为 `Contents/MacOS/VestyGain`。

本次 2026-06-08 针对 `vesty build --debug` 和 gain headless cleanup 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli build_profile_defaults_to_release_and_accepts_debug
cargo run -p vesty-cli -- build --help
cargo run -p vesty-cli -- build --config examples/gain/vesty.toml --debug
cargo run -p vesty-cli -- build --config examples/midi-synth/vesty.toml --debug
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

本次 2026-06-08 针对 package/dev current-platform default 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli bundle_platform_parser_accepts_release_targets
cargo run -p vesty-cli -- package --help
cargo run -p vesty-cli -- dev --help
cargo build -p vesty-example-gain --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --binary target/release/libvesty_example_gain.dylib --out target/vesty-current-platform-smoke --install-dev --vst3-dir target/dev-vst3-current-platform-smoke --install-mode copy
test -f target/dev-vst3-current-platform-smoke/VestyGain.vst3/Contents/Resources/moduleinfo.json
cargo run -p vesty-cli -- dev --config examples/gain/vesty.toml --no-ui --install-dev --binary target/debug/libvesty_example_gain.dylib --out target/vesty-dev-current-platform-smoke --vst3-dir target/dev-vst3-dev-current-platform-smoke --install-mode copy
test -f target/dev-vst3-dev-current-platform-smoke/VestyGain.vst3/Contents/Resources/moduleinfo.json
```

本次 2026-06-08 针对 `vesty build --no-ui` 通过:

```bash
cargo fmt --all --check
cargo run -p vesty-cli -- build --help
cargo run -p vesty-cli -- build --config examples/web-ui-param-demo/vesty.toml --debug --no-ui
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

本次 2026-06-08 针对 class id/FUID validation 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build class_id
cargo test -p vesty-build validates_packaged_vst3_bundle
cargo clippy -p vesty-build --all-targets -- -D warnings
cargo build -p vesty-example-gain --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --binary target/release/libvesty_example_gain.dylib --out target/vesty-class-id-smoke
python3 - <<'PY'
import json
p='target/vesty-class-id-smoke/VestyGain.vst3/Contents/Resources/moduleinfo.json'
with open(p) as f: data=json.load(f)
assert data['classes'][0]['cid'] == '56455354-4947-4149-4e30-303030303031'
PY
```

本次 2026-06-08 针对 VST3 realtime guard 扩围、缺 kernel 生命周期异常处理和 web-ui 示例 validator smoke 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3 --features vst3-bindings processor_process_does_not_allocate_inside_rt_guard_under_automation_and_midi
cargo test -p vesty-vst3 --features vst3-bindings processor_prepare_tracks_sample_rate_and_block_size_matrix
cargo test -p vesty-vst3 --features vst3-bindings processor_process_without_setup_silences_without_creating_kernel
cargo test -p vesty-vst3 --features vst3-bindings
cargo clippy -p vesty-vst3 --features vst3-bindings --all-targets -- -D warnings
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npm run typecheck
npm run build
cargo run -p vesty-cli -- export-types --out target/vesty-protocol
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo run -p vesty-cli -- doctor --format json
cargo build -p vesty-example-web-ui-param-demo --release
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/package-smoke --platform macos
cargo run -p vesty-cli -- validate target/package-smoke/VestyWebUIDemo.vst3 --static-only --format json --report target/package-smoke/static-validate-web-ui-demo.json
cargo run -p vesty-cli -- validate target/package-smoke/VestyWebUIDemo.vst3 --format json --report target/package-smoke/validate-web-ui-demo.json --validator-log target/package-smoke/validator-web-ui-demo.log
```

新增行为:

- `IAudioProcessor::process()` 的 `NoAllocGuard` 现在覆盖 VST3 event collection、sample-order sort、transport mirror、buffer/context 组装和 developer kernel。
- `setupProcessing()` / `setActive(true)` 是 kernel 创建和 `prepare()` 的非实时生命周期；异常 host 若在缺 kernel 时调用 `process()`，wrapper 清零输出、设置 silence flags，并推固定 `HostWarning` RT log，不在实时区创建 kernel。
- 本机 macOS web-ui 示例 bundle 已通过 Steinberg validator: 47 tests passed, 0 failed；报告位于 `target/package-smoke/validate-web-ui-demo.json`，validator log 位于 `target/package-smoke/validator-web-ui-demo.log`。

本次 2026-06-08 针对 DAW evidence root 模板创建和 release evidence README 示例通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli daw_evidence_root_templates_create_standard_host_directories
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs
rm -rf target/daw-template-root-smoke && cargo run -p vesty-cli -- daw-matrix --write-template --evidence-root target/daw-template-root-smoke --format json > target/daw-template-root-smoke-output.json
test -f target/daw-template-root-smoke/reaper/README.md
test -f target/daw-template-root-smoke/cubase/scan-smoke.log
test -f target/daw-template-root-smoke/studio-one/buffer-sample-rate.log
test -f target/daw-template-root-smoke/studio-one/offline-render.log
cargo test -p vesty-cli daw_evidence
cargo test -p vesty-cli release_evidence
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

新增覆盖:

- `vesty daw-matrix --write-template --evidence-root <dir>` 的标准 host 子目录创建行为已由单元测试和真实 CLI smoke 覆盖；五个 host 共创建 50 个模板文件，重复执行不会覆盖已有 evidence。
- `release-check --write-evidence-template` 生成的 README 推荐同时使用 `target/daw-evidence` 和 `target/release-evidence`，并在 strict release gate 中传入同一个 `--evidence-root`。

本次 2026-06-08 针对 release evidence `ci-doctor/` 模板和自动发现语义通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli release_evidence
cargo test -p vesty-cli release_check_reports_missing_host_evidence
rm -rf target/release-evidence-template-smoke && cargo run -p vesty-cli -- release-check --write-evidence-template target/release-evidence-template-smoke --format json > target/release-evidence-template-smoke-report.json
test -f target/release-evidence-template-smoke/ci-doctor/README.md
test -f target/release-evidence-template-smoke/validate-report.json
cargo run -p vesty-cli -- release-check --release-evidence-dir target/release-evidence-template-smoke --format json > target/release-evidence-template-smoke-applied.json
python3 - <<'PY'
import json
with open('target/release-evidence-template-smoke-applied.json') as f:
    data=json.load(f)
check = next(c for c in data['checks'] if c['name'] == 'ci doctor artifacts')
assert check['status'] == 'skipped', check
PY
```

新增行为:

- `release-check --write-evidence-template <dir>` 现在会创建 `ci-doctor/README.md`，指引放置 `doctor-Linux.json`、`doctor-macOS.json` 和 `doctor-Windows.json`。
- `--release-evidence-dir <dir>` 只有在 `ci-doctor/` 中存在 JSON doctor artifacts 时才自动启用该目录；仅有模板 README 时 `ci doctor artifacts` 仍保持 `skipped`，不会把本地非严格检查误报为 failed。

本次 2026-06-08 针对 release evidence dir validate report 自动分类通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli release_evidence_dir_discovers_validate_reports_by_content
cargo test -p vesty-cli release_evidence
cargo run -p vesty-cli -- release-check --release-evidence-dir target/package-smoke --skip-protocol --format json > target/release-evidence-discovery-smoke.json
python3 - <<'PY'
import json
with open('target/release-evidence-discovery-smoke.json') as f:
    data=json.load(f)
checks={c['name']: c for c in data['checks']}
assert checks['vst3 validate reports']['status'] == 'ok', checks['vst3 validate reports']
assert checks['vst3 static validate reports']['status'] == 'ok', checks['vst3 static validate reports']
PY
```

新增行为:

- `--release-evidence-dir <dir>` 会扫描目录内其它 `*.json`，只处理能解析成 `ValidateReport` 的文件。
- validator-passed report 自动归入 `validate_reports`；static-only/skipped report 自动归入 `static_validate_reports`；其它 JSON，例如 release-check report 或 doctor artifact，不会被误分类。
- CI package artifact 目录中的 `VestyGain.validate.json` / `VestyWebUIDemo.validate.json` / `VestyMIDISynth.validate.json` 这类文件名无需手动改名即可被聚合为 static validate smoke。

本次 2026-06-08 针对 realtime automation segment deterministic fuzz-style 覆盖通过:

```bash
cargo fmt --all --check
cargo test -p vesty-core automation_segments
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增覆盖:

- `automation_segments_cover_block_for_deterministic_point_patterns` 覆盖 block size 0/1/2/4/8/16/32，空事件、block 起点事件、同 offset 多点、其它 handle 干扰和越界 point。
- 测试按 sample 生成期望时间线，验证 `ParamAutomationSegments` 输出连续覆盖整个 block、segment 不为空、不倒退、不越界，并保持同 offset 最后值生效。

本次 2026-06-08 在 VST3 no-allocation 测试锁修复后重新跑完整本地验证通过:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npm test
npm run typecheck
npm run build
cargo fmt --all --check
```

确认结果:

- `cargo test --workspace` 覆盖全部 Rust crate、examples 和 doc-tests，VST3 并发测试没有再出现共享 no-allocation 状态污染。
- `npm test` 覆盖 `@vesty/plugin-ui` 的 Node bridge sequencing 测试，并重新构建 package dist。
- `npm run typecheck`、`npm run build`、Rust clippy、feature-gated checks 和 format check 均通过。

本次 2026-06-08 针对 `@vesty/plugin-ui` subscription reference counting 和 async event pump 行为补充 Node 测试并通过:

```bash
npm test
npm run typecheck
npm run build
```

新增覆盖:

- 同一 topic 多个 handler 只发送一次 `subscription.add`，移除第一个 handler 不会提前 `subscription.remove`，最后一个 handler 移除时才退订。
- `meter.*` 与 `param.changed` 订阅会启动约 60 Hz `event.flush` async event pump；同 topic 多 handler 不会启动多个 interval，最后一个 async event handler 移除时停止 pump。

本次 2026-06-08 针对 wry bootstrap bridge 生命周期不变量补强测试并通过:

```bash
cargo test -p vesty-ui-wry
cargo test -p vesty-ui-wry --features wry-backend
cargo clippy -p vesty-ui-wry --features wry-backend --all-targets -- -D warnings
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增覆盖:

- `bootstrap_script_registers_host_subscriptions` 现在固定验证 bootstrap 脚本包含首个 handler 才 `subscription.add`、最后一个 handler 才 `subscription.remove` 的引用计数逻辑。
- 同一测试固定验证 `meter.*` topic 检测、`meterPump` 单例启动和 `clearInterval(meterPump)` 停止逻辑，避免 wry 注入脚本与 `@vesty/plugin-ui` SDK 行为漂移。

本次 2026-06-08 针对 release artifact evidence 目录自动发现补强并通过:

```bash
cargo fmt --all
cargo test -p vesty-cli release_evidence
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo run -p vesty-cli -- release-check --write-evidence-template target/release-evidence-nested-smoke --format json
cargo run -p vesty-cli -- release-check --release-evidence-dir target/release-evidence-nested-smoke --skip-protocol --format json
```

新增行为:

- `release-check --release-evidence-dir <dir>` 会递归扫描 `.log` / `.txt` 文件，按内容自动发现 codesign/signtool 签名验证日志和 accepted notarization/stapler 日志。
- 同一目录下带可解析 `Contents/_CodeSignature/CodeResources` plist，且 plist 含 `files` 或 `files2` dictionary 条目的 macOS `.vst3` bundle 会被自动归入 signed bundle evidence。
- 顶层模板 `signing-macos.log`、`signing-windows.log` 和 `notary.log` 只有内容验证通过时才自动导入；pending 模板不会污染后续真实 nested artifact。

本次 2026-06-08 针对 GitHub Actions release evidence artifact flow 补强:

```bash
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'
```

新增行为:

- `.github/workflows/ci.yml` 新增 `release evidence snapshot` job，依赖 `rust`、`protocol` 和 `package` jobs。
- Rust matrix 的 per-OS `release-check-*` artifact 生成命令会传入当前 GitHub Actions run URL，让 snapshot JSON 携带 `ci_run_url` provenance。
- `release evidence snapshot` job 下载同一 workflow 的 `doctor-*`、`release-check-*`、`vesty-protocol` 和 `*-vst3-static-validate` artifacts，写入 `ci-run-url.txt`，并运行非严格 consolidated `vesty release-check --release-evidence-dir ... --protocol-snapshot ...`。
- CI 会上传 `release-evidence-consolidated` artifact，作为真实 GitHub Actions run 的机器可读 release readiness 快照；它仍不替代 DAW smoke、Steinberg validator passed reports 或 signed/notarized artifact。

本次 2026-06-08 针对 GitHub Actions package smoke 扩展:

```bash
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'
npm run build --prefix examples/web-ui-param-demo/ui
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/ci-package-matrix-macos-smoke
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/ci-package-matrix-macos-smoke
cargo run -p vesty-cli -- package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/ci-package-matrix-macos-smoke
cargo run -p vesty-cli -- validate target/ci-package-matrix-macos-smoke/VestyGain.vst3 --static-only --format json --report target/ci-package-matrix-macos-smoke/VestyGain.validate.json
cargo run -p vesty-cli -- validate target/ci-package-matrix-macos-smoke/VestyWebUIDemo.vst3 --static-only --format json --report target/ci-package-matrix-macos-smoke/VestyWebUIDemo.validate.json
cargo run -p vesty-cli -- validate target/ci-package-matrix-macos-smoke/VestyMIDISynth.vst3 --static-only --format json --report target/ci-package-matrix-macos-smoke/VestyMIDISynth.validate.json
```

新增行为:

- `package smoke` 从 macOS-only job 扩展为 Ubuntu/macOS/Windows matrix，分别使用 `linux-x64`、`macos` 和 `windows-x64` 打包平台。
- package matrix 现在先使用 Node 24 运行 `npm run build --prefix examples/web-ui-param-demo/ui`，不再依赖预先存在的 `ui/dist` 来证明 Web UI 资源被打入 bundle。
- 三个平台都会 release build `gain`、`web-ui-param-demo` 和 `midi-synth` examples，调用 `vesty package` 生成 `.vst3`，再运行 `vesty validate --static-only --report`。
- CI 会分别上传 `linux-vst3-static-validate`、`macos-vst3-static-validate` 和 `windows-vst3-static-validate` artifacts；`release evidence snapshot` job 会递归下载并聚合这些 package/static validate reports。

本次 2026-06-08 针对 `web-ui-param-demo` UI source/build 补齐并通过:

```bash
npm run build --prefix examples/web-ui-param-demo/ui
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'
cargo run -p vesty-cli -- build --config examples/web-ui-param-demo/vesty.toml --debug
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/debug/libvesty_example_web_ui_param_demo.dylib --out target/web-ui-source-smoke
cargo run -p vesty-cli -- validate target/web-ui-source-smoke/VestyWebUIDemo.vst3 --static-only --format json --report target/web-ui-source-smoke/VestyWebUIDemo.static-validate.json
```

新增行为:

- `examples/web-ui-param-demo/ui` 新增 `package.json`、`src/index.html`、`src/index.js`、`scripts/build.mjs` 和 `scripts/dev.mjs`。
- build 脚本只依赖 Node 标准库，重建 `dist/index.html` 和 `dist/index.js`；dev 脚本在 `127.0.0.1:5173` 服务 `dist`，和 `vesty.toml` 的 `dev_url` 对齐。
- `vesty build --config examples/web-ui-param-demo/vesty.toml --debug` 已验证会执行 UI build、生成 asset manifest 并构建 Rust example。
- 新生成的 Web UI demo bundle 已通过 `vesty validate --static-only`，静态检查 `ok`，asset manifest 包含 2 个 UI asset。

本次 2026-06-08 针对 DAW evidence 模板可操作性补强并通过:

```bash
cargo fmt --all
cargo test -p vesty-cli evidence
cargo run -p vesty-cli -- daw-matrix --write-template --evidence-root target/daw-template-marker-smoke --format json
rg -n "Accepted Pass Markers|scan=true|ui_host_param=true|meter_flush sent=1|vesty daw-matrix --evidence-root target/daw-evidence --strict" target/daw-template-marker-smoke/bitwig/README.md
```

新增行为:

- `vesty daw-matrix --write-template` 生成的每个 host README 现在包含 accepted pass marker 样例，直接对应 parser 支持的 `scan=true`、`load=true`、`ui=true`、`ui_host_param=true`、`meter_flush sent=1`、`automation=true`、`buffer_sample_rate_change=true`、`save_restore=true`、`offline_render=true` 和 `render_file=/absolute/path/to/rendered.wav`。
- README 同时包含 `vesty daw-matrix --evidence-root target/daw-evidence --format markdown` 和 `--strict` 验证命令，降低外部 DAW smoke 证据采集后 marker 填错的概率。

本次 2026-06-08 针对 offline render 文件证据解析补强并通过:

```bash
cargo fmt --all
cargo test -p vesty-cli render_file_evidence
cargo test -p vesty-cli generic_daw_evidence
cargo test -p vesty-cli reaper_evidence_accepts_explicit_offline_render_marker
cargo test -p vesty-cli generic_daw_evidence_accepts_relative_render_file_marker
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
npm test
npm run typecheck
```

新增行为:

- `offline-render.log` / `render-smoke.log` 中的 `render_file` 现在接受等号两侧空格以及单引号/双引号包裹的路径。
- `render_file` 文件证据必须指向存在且非空的文件；空文件不会被判定为 offline render pass。
- 相对 `render_file` 会按当前 host evidence 目录解析；`../...` 父目录跳转会被拒绝，避免 evidence 目录外的旧文件误报通过。
- REAPER `render-smoke.log` 现在和 generic DAW `offline-render.log` 一样接受显式 `offline_render=true|pass|ok`、`render=true|pass|ok`、`render_ok=true|pass` marker，用于 DAW 没有稳定 render path 输出的场景。

本次 2026-06-08 针对 validate/static validate report platform coverage 补强并通过:

```bash
cargo fmt --all
cargo test -p vesty-cli static_validate_reports
cargo run -p vesty-cli -- release-check --skip-protocol --static-validate-report target/ci-package-matrix-macos-smoke/VestyGain.validate.json --static-validate-report target/ci-package-matrix-macos-smoke/VestyWebUIDemo.validate.json --static-validate-report target/ci-package-matrix-macos-smoke/VestyMIDISynth.validate.json --format json --report target/ci-package-matrix-macos-smoke/static-validate-platform-coverage-release-check.json
```

新增行为:

- `vst3 validate reports` 和 `vst3 static validate reports` release-check 项会从 `static_check.binaries` 推断并显示 `platforms: ...`。
- 当前识别 macOS `Contents/MacOS`、Windows x64 `Contents/x86_64-win` 和 Linux x64 `Contents/x86_64-linux` bundle binary paths；CI 三平台 static validate artifacts 聚合后会在 consolidated release report 中显式暴露 platform coverage。
- `vst3 example validator coverage` release-check 项会进一步检查 validator-passed reports 中的 Vesty 示例插件覆盖。普通本地检查允许部分覆盖并给出 missing hint；`--require-release-artifacts` 会要求 `VestyGain.vst3`、`VestyWebUIDemo.vst3` 和 `VestyMIDISynth.vst3` 乘以 `linux-x64`、`macos`、`windows-x64` 的完整 3x3 validator 覆盖；每个示例/platform report 还必须包含指向对应 bundle `Contents/Resources/parameters.manifest.json` 的 `static_check.parameter_manifest`，证明参数 sidecar 已进入 `.vst3` bundle，并包含匹配平台的 `ok` `static_check.binary_exports`，证明 VST3 factory/entry exports 已被真实工具观察到。
- `ci example static validate coverage` release-check 项会进一步检查 CI static validate reports 中的 Vesty 示例插件矩阵。普通 per-OS package smoke 要求当前平台的 `VestyGain.vst3`、`VestyWebUIDemo.vst3` 和 `VestyMIDISynth.vst3` 都存在；`--require-release-artifacts` 会要求三示例乘 `linux-x64`、`macos`、`windows-x64` 完整 3x3 覆盖；三示例 static reports 同样必须包含指向对应 bundle `Contents/Resources/parameters.manifest.json` 的 `static_check.parameter_manifest` 和匹配平台的 `ok` `static_check.binary_exports`。
- `VestyWebUIDemo.vst3` 覆盖要求 UI asset manifest evidence 指向 `Contents/Resources/assets.manifest.json`；新增测试会拒绝 `asset_manifest = null`、`asset_count = 0` 或非 bundle 标准路径的 Web UI 示例 report。

本次 2026-06-08 针对 Web UI example asset evidence release gate 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

新增行为:

- `example_validate_coverage_release_check()` 和 `example_static_validate_coverage_release_check()` 对 `VestyWebUIDemo.vst3` 要求 UI asset evidence。
- 正常 Web UI example fixture 会写入 `asset_manifest` 和 `asset_count = 2`；`example_coverage_rejects_web_ui_report_without_asset_evidence` 覆盖缺失 asset evidence 的 validator/static report 都会失败。

本次 2026-06-08 针对 Vesty example parameter sidecar release evidence gate 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli
cargo test -p vesty-build
cargo clippy -p vesty-cli -p vesty-build --all-targets -- -D warnings
cargo build -p vesty-example-gain -p vesty-example-midi-synth -p vesty-example-web-ui-param-demo --release
cargo run -p vesty-cli -- param-manifest --specs examples/gain/params.specs.json --out examples/gain/vesty-parameters.json --check
cargo run -p vesty-cli -- param-manifest --specs examples/midi-synth/params.specs.json --out examples/midi-synth/vesty-parameters.json --check
cargo run -p vesty-cli -- param-manifest --specs examples/web-ui-param-demo/params.specs.json --out examples/web-ui-param-demo/vesty-parameters.json --check
target/debug/vesty package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-paramid-v2-smoke/gain
target/debug/vesty package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-paramid-v2-smoke/web
target/debug/vesty package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/vesty-paramid-v2-smoke/midi
target/debug/vesty validate target/vesty-paramid-v2-smoke/gain/VestyGain.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json
target/debug/vesty validate target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json
target/debug/vesty validate target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json
target/debug/vesty validate target/vesty-paramid-v2-smoke/gain/VestyGain.vst3 --format json --report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json --validator-log target/vesty-paramid-v2-smoke/gain/VestyGain.validator.log
target/debug/vesty validate target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.vst3 --format json --report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json --validator-log target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validator.log
target/debug/vesty validate target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.vst3 --format json --report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json --validator-log target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validator.log
cargo run -p vesty-cli -- release-check --skip-protocol --format json --validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json --validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json --validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json --static-validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json --static-validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json --static-validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json --report target/vesty-paramid-v2-smoke/release-check-validator-static.json
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml
```

新增行为:

- `example_validate_coverage_release_check()` 和 `example_static_validate_coverage_release_check()` 对三个内置示例都要求 `static_check.parameter_manifest`，防止 CI/validator evidence 只证明 binary 存在却漏掉参数 ID sidecar。
- 新增 `example_coverage_rejects_example_report_without_parameter_manifest_evidence`，覆盖 validator-passed report 和 static-only report 缺参数 sidecar 时都会失败。
- 新增 `example_coverage_rejects_example_report_with_non_bundle_parameter_manifest_path`、`example_coverage_rejects_example_report_with_suffix_spoofed_parameter_manifest_path`、`example_coverage_rejects_web_ui_report_with_non_bundle_asset_manifest_path` 和 `example_coverage_rejects_web_ui_report_with_suffix_spoofed_asset_manifest_path`，要求 sidecar/asset evidence 按路径组件匹配对应 `.vst3/Contents/Resources/...` 标准路径，拒绝裸文件名、错 bundle 路径和后缀伪装路径。
- 新增 `non_example_static_validate_report_does_not_require_parameter_manifest`，保持第三方/非内置示例 bundle 的静态 validate smoke 不被 Vesty framework release 专用门禁误伤。
- `.github/workflows/ci.yml` 的 package smoke 在 build/package 前先运行三个 example 的 `vesty param-manifest --check`，让参数 specs/sidecar drift 在 CI 早期失败。
- 本机 macOS ParamID v2 package/static/validator smoke 显示 `VestyGain.vst3`、`VestyMIDISynth.vst3` 和 `VestyWebUIDemo.vst3` 均包含 `Contents/Resources/parameters.manifest.json`；Web UI 示例同时包含 `assets.manifest.json` 且 `asset_count = 2`。`target/vesty-paramid-v2-smoke/release-check-validator-static.json` 中 `vst3 validate reports`、`vst3 example validator coverage`、`vst3 static validate reports` 和 `ci example static validate coverage` 在非最终本地检查下均为 `ok`，覆盖平台为 `macos`；最终 `--require-release-artifacts` gate 仍需要 Linux/Windows validator reports。

本次 2026-06-08 针对 release evidence template 的 sidecar 指引补强:

```bash
cargo fmt --all --check
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths -- --nocapture
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

新增行为:

- `vesty release-check --write-evidence-template` 生成的 release artifact README 现在明确说明三示例 validator/static reports 必须包含指向对应 bundle `Contents/Resources/parameters.manifest.json` 的 `static_check.parameter_manifest`。
- 模板 README 同时说明 `VestyWebUIDemo.vst3` 需要指向 `Contents/Resources/assets.manifest.json` 的 UI asset manifest evidence；这与 release-check 的实际门禁一致。
- `release_evidence_templates_do_not_count_as_pass_or_overwrite_logs` 新增断言，避免模板文案后续再次和参数 sidecar / UI asset evidence gate 脱节。

本次 2026-06-08 针对 Steinberg validator report hardening 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli
cargo test -p vesty-cli release_check_rejects_validator_passed_report_without_exit_or_test_counts
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

新增行为:

- `validate_release_validate_report()` 不再只接受 `validator.status = passed`，还要求 `exit_code = 0`、`tests_passed > 0` 和 `tests_failed = 0`。
- `release_check_rejects_validator_passed_report_without_exit_or_test_counts` 覆盖伪造 passed report 缺少 exit code 或测试计数字段时会失败。

本次 2026-06-08 针对 release artifact evidence README 可操作性补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli release_evidence
cargo run -p vesty-cli -- release-check --write-evidence-template target/release-evidence-marker-smoke --format json
rg -n "Accepted release artifact markers|codesign=pass|signtool=pass|Number of errors: 0|notarytool=pass|status: Accepted" target/release-evidence-marker-smoke/README.md
```

新增行为:

- `release-check --write-evidence-template` 生成的 release artifact README 现在包含 accepted marker 示例，覆盖 codesign、signtool、signtool summary `Number of errors: 0`、notarization、notarytool、stapled、`status: Accepted` 和 stapler validate output；后续 2026-06-13 已进一步移除泛用 signing/signature marker 的签名证据语义。
- 这些示例直接对应 `validate_signing_evidence()`、`validate_notarization_evidence()` 和 `explicit_truthy_marker()` 支持的内容，减少外部签名/公证日志留证后无法被 gate 识别的概率。

本次 2026-06-08 针对 release signing/notarization evidence parser hardening 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli signing_evidence
cargo test -p vesty-cli notarization_evidence
cargo test -p vesty-cli explicit_truthy_marker
```

新增行为:

- `explicit_truthy_marker()` 从子串 contains 收紧为精确 `key=value` / `key: value` 解析，仍接受 `codesign: pass`、`scan_ok=pass` 等模板 marker；后续 2026-06-13 已进一步要求签名证据必须能归属到 macOS codesign 或 Windows signtool，`signed=true` / `signature=ok` 不再作为签名证据通过。
- `unsigned=true`、`note: signed=true after verification`、`notarization=pending`、`stapled: false` 这类子串、说明或 false/pending 值不再会通过 release evidence gate。
- `validate_notarization_evidence()` 不再因为任意 `stapled:` 行通过，仍接受 notarytool `status: Accepted` 和 `The staple and validate action worked!`。
- `signing_evidence_platforms_from_text()` / `notarization_evidence_from_text()` 现在会拒绝同一日志中的矛盾失败证据: invalid signature、`SignTool Error`、非零 signtool error count、rejected/invalid notary status、notarytool/stapler failure 都不能被 `codesign=pass`、`signtool=pass`、`status: Accepted` 或 `stapled=true` 覆盖。
- release artifact README 已说明轻量 marker 必须是独立、精确的 key/value 证据行。

本次 2026-06-08 针对 CI run URL evidence parser hardening 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli ci_run_url_requires_exact_github_actions_run_shape
```

新增行为:

- `ci run url` release-check 不再只检查 GitHub 前缀和 `/actions/runs/` 子串。
- `--ci-run-url` / `ci-run-url.txt` 现在要求精确 `https://github.com/<org>/<repo>/actions/runs/<numeric-id>`，允许 trailing slash、`/attempts/<n>` 和 query/fragment；`release-check --release-evidence-dir` 同时收到显式 `--ci-run-url` 和目录内 `ci-run-url.txt` 时，会拒绝 repo/run id 不一致的组合，避免命令行 provenance 静默覆盖目录 provenance；`ci-run-url.txt` symlink 现在会被拒绝，避免 evidence 目录指向外部可替换 provenance 文件。
- 普通 Actions 页面、缺失 run id、非数字 run id、job URL、带空白的 URL 和非 HTTPS URL 都会失败。

本次 2026-06-08 针对 CI doctor artifact status gate hardening 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli ci_doctor_artifacts
```

新增行为:

- `ci doctor artifacts` release-check 不再只看 required check 名称是否存在。
- Linux/macOS/Windows 的 toolchain、Node/npm、VST3 binding baseline、validator、system WebView 和平台签名/公证 preflight check 现在必须有可接受状态；常规 required check 要求 `ok`。
- Linux `signing: linux release policy` 允许 `unknown`，匹配当前 doctor 输出的“无标准 VST3 bundle 签名工具，按发行渠道处理”语义。
- 缺失 check 会报告 `<OS>/<check> missing`，失败状态会报告 `<OS>/<check> status <status>`。

本次 2026-06-08 针对 Steinberg validator summary parser hardening 通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli validator_summary_extracts_passed_and_failed_counts
```

新增行为:

- `validator_test_summary()` 不再依赖严格大小写的单行 `Result:` 格式。
- 解析器现在支持 canonical `Result: 47 tests passed, 0 tests failed`、带时间戳/前缀的 result 行、大小写变化、`Tests passed: 47` / `Tests failed: 0` 拆行，以及单数 `1 test passed`。
- 仍然要求 passed 和 failed 两个计数都出现；只有 `tests passed` 的摘要不会产生 release-valid 的计数字段。

本次 2026-06-08 针对 `@vesty/plugin-ui` 参数 helper API 测试覆盖通过:

```bash
npm test
```

新增覆盖:

- `setParam(id, normalized, gestureId?)` 必须按 `param.begin` -> `param.perform` -> `param.end` 顺序发送 param lane request，并在传入 gesture token 时贯穿三段 payload。
- 如果 `param.begin` 返回 error，`setParam()` 会 reject，并且不会继续发送 perform/end。
- `formatParam()` 和 `parseParam()` 的 payload 与 response resolve 行为有 JS SDK 测试约束，避免 Web UI 侧最常用的参数显示/解析 API 在后续重构中漂移。
- `createBridge(host, initialSession, options)` 覆盖 invalid host/session/options runtime validation，无效 host、`initialSession` 或 `timeoutMs` 不会注册 unload listener、创建 `__VESTY_INTERNAL__` 或发送 IPC。
- `createSnapshotStore(bridge)` 覆盖 initial snapshot、`refresh()` -> `snapshot.get`、完整 `state.changed` snapshot 发布、custom topic + `refreshOnEvent: false`、options/listener/selector runtime validation、`select()`、listener 异常隔离和最后一个 listener 移除时的 `subscription.remove`。

本次 2026-06-08 针对 React/Vue/Svelte 薄适配包落地通过:

```bash
npm install
npm run typecheck
npm run build
npm test
```

新增行为:

- `@vesty/react` 提供 bridge context、snapshot store hook、snapshot hook 和 param edit hook。
- `@vesty/vue` 提供 snapshot composable 和 param edit composable。
- `@vesty/svelte` 提供 snapshot readable store 和 param edit helper。
- 三个包的 `dist/index.js` / `dist/index.d.ts` 均由 TypeScript build 生成，并通过 workspace export smoke test。

本次 2026-06-08 针对 `setParam()` 参数手势失败收尾补强通过:

```bash
npm test
cargo test -p vesty-ui-wry bootstrap_script_registers_host_subscriptions
```

新增行为:

- `@vesty/plugin-ui` 与 wry bootstrap 的 `setParam()` 在 `param.begin` 成功后会保证尝试发送 `param.end`。
- 如果 `param.perform` 失败，`setParam()` 仍会尽力发送 `param.end`，最终优先 reject 原始 perform error，降低 host 侧参数 edit gesture 悬空风险。
- 如果 `param.begin` 失败，则不会发送 perform/end，保持未知参数或权限错误的快速失败语义。

本次 2026-06-08 针对 Rust bridge `param.end` backpressure 优先级补强通过:

```bash
cargo test -p vesty-bridge param_end_is_prioritized_when_gesture_queue_is_full
cargo test -p vesty-bridge pending_param_gesture_queue_full_returns_backpressure
cargo test -p vesty-bridge param_gesture_id_shape_is_validated
```

新增行为:

- pending param gesture 队列满时，`param.begin` / `param.perform` 仍返回 retryable `backpressure`。
- `param.end` 作为收尾信号获得优先级；队列满时会丢弃一个旧 pending perform gesture 并接收 end，尽量避免 host 侧参数 edit gesture 悬空，且 diagnostics 的 `droppedParamGestures` 会累计被丢弃的 pending gesture。
- 新测试确认满队列下 end 得到 response、pending count 保持容量上限、最后一个 drained gesture 是 `ParamGesturePhase::End`，并确认 `droppedParamGestures = 1`。
- `gestureId` 仍是可选字段；如果提供，Rust bridge 会拒绝空字符串、超过 128 bytes 或包含控制字符的值。

本次 2026-06-08 针对 `vesty-vst3-sys` binding source 层补齐并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3-sys
cargo test -p vesty-vst3-sys --all-features
cargo check -p vesty-vst3-sys --all-features
cargo test -p vesty-vst3 binding_baseline_reports_reserved_sys_layer
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo check -p vesty-ui-wry --features wry-backend
npm test
npm run typecheck
```

新增行为:

- workspace 新增 `crates/vesty-vst3-sys`，固定 Steinberg VST3 SDK baseline `v3.8.0_build_66` 和 upstream `vst3` crate baseline `0.3.0`。
- `vesty-vst3-sys` 当前默认 backend 是 `upstream-vst3`，并预留 `generated-headers` feature/backend；该 feature 当前只表示生成层入口存在，不伪造完整 headers bindings 已生成。
- `vesty-vst3` 依赖并 re-export `vesty_vst3_sys` 为 `vesty_vst3::sys`，同时提供 `binding_baseline()` 作为 doctor/release diagnostics 可复用的查询入口。

本次 2026-06-08 针对 `vesty doctor` VST3 binding baseline 接入并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli doctor_report_includes_toolchain_webview_and_validator_checks
cargo test -p vesty-cli release_check_accepts_ci_signing_and_notarization_evidence
cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths
cargo test -p vesty-cli
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo run -p vesty-cli -- doctor --format json > target/doctor-binding-baseline.json
python3 -m json.tool target/doctor-binding-baseline.json >/dev/null
rg -n 'vst3 binding baseline|v3.8.0_build_66|upstream vst3 crate 0.3.0' target/doctor-binding-baseline.json
```

新增行为:

- `vesty doctor` JSON/text report 现在包含 `vst3 binding baseline` check，输出 Steinberg SDK baseline、upstream `vst3` crate baseline 和当前 backend。
- `ci doctor artifacts` release-check gate 现在要求每个平台 doctor artifact 包含 `vst3 binding baseline`，防止 CI 证据缺少 binding source/version 记录。

本次 2026-06-08 针对 `vesty release-check` VST3 binding baseline 接入并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli release_check_passes_with_complete_matrix_and_protocol_snapshot
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs
cargo run -p vesty-cli -- release-check --skip-protocol --format json --report target/release-check-binding-baseline.json > target/release-check-binding-baseline.stdout.json
python3 -m json.tool target/release-check-binding-baseline.json >/dev/null
rg -n 'vst3 binding baseline|v3.8.0_build_66|upstream vst3 crate 0.3.0' target/release-check-binding-baseline.json
```

新增行为:

- `vesty release-check` report 现在包含 `vst3 binding baseline` check，和 `vesty doctor` 使用同一份 baseline formatting。
- `release-check --write-evidence-template` 生成的 README/`ci-doctor/README.md` 会提示真实 doctor artifact 应包含 `vst3 binding baseline`。

本次 2026-06-08 针对三示例本机 macOS Steinberg validator evidence 通过:

```bash
target/debug/vesty validate target/vesty-paramid-v2-smoke/gain/VestyGain.vst3 --format json --report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json --validator-log target/vesty-paramid-v2-smoke/gain/VestyGain.validator.log
target/debug/vesty validate target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.vst3 --format json --report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json --validator-log target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validator.log
target/debug/vesty validate target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.vst3 --format json --report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json --validator-log target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validator.log
cargo run -p vesty-cli -- release-check --skip-protocol --format json --validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json --validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json --validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json --report target/vesty-paramid-v2-smoke/release-check-validator-coverage.json
```

新增 evidence:

- `VestyGain.vst3`、`VestyWebUIDemo.vst3`、`VestyMIDISynth.vst3` 的本机 macOS validator report 均为 static check `ok`、Steinberg validator `passed`，结果均为 47 tests passed / 0 failed。
- `release-check` 已把三份 macOS `--validate-report` 聚合为 `vst3 validate reports: ok` 和非最终本地 `vst3 example validator coverage: ok`。
- 该 ParamID v2 evidence 证明本机 macOS 三个示例都能通过 Steinberg validator；它仍不替代真实 GitHub Actions artifact、Windows/Linux validator、商业 DAW smoke、签名或 notarization evidence。更早的本机 validator smoke 已被 `target/vesty-paramid-v2-smoke` 取代。

本次 2026-06-08 针对 active Web UI 的 VST3 state restore 同步并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo clippy -p vesty-bridge --all-targets -- -D warnings
cargo clippy -p vesty-vst3 --all-targets --features 'vst3-bindings wry-ui' -- -D warnings
```

新增行为:

- `vesty-bridge` 新增 `BridgeRuntime::restore_snapshot_from_host()`，可在 host/controller 恢复持久 state 后替换 runtime snapshot，并向订阅了 `state.changed` 的 UI 推送完整 `PluginSnapshot`。
- `vesty-vst3` 的 wry bridge endpoint 现在使用带 generation 的共享 bridge snapshot。active WebView runtime 会在每次 IPC / `event.flush` 前吸收 controller `setState()` / `setComponentState()` 恢复的 bridge snapshot；runtime 写回 snapshot 时如果发现 controller 已有更新 generation，会跳过写回，避免覆盖宿主恢复状态。
- 新增测试覆盖纯 bridge host snapshot restore，以及 VST3 controller 在 UI 已打开、已订阅 `state.changed` 时，通过 `event.flush` 将恢复后的 config/UI state 推给现有 UI runtime，并确认后续 `snapshot.get` 返回同一份恢复状态。

本次 2026-06-08 针对 `@vesty/plugin-ui` protocol package exports 并通过:

```bash
cargo run --quiet -p vesty-cli -- export-types --out target/vesty-plugin-ui-protocol
npm run --workspace @vesty/plugin-ui typecheck
npm run --workspace @vesty/plugin-ui test
npm test
node --input-type=module -e 'const mod = await import("@vesty/plugin-ui/protocol"); if (typeof mod !== "object") throw new Error("protocol subpath import failed"); console.log("protocol subpath runtime import ok");'
mkdir -p target/typecheck-smoke
printf '%s\n' 'import type { BridgeReadyPayload, PluginSnapshot } from "@vesty/plugin-ui/protocol";' 'const snapshot: PluginSnapshot = { revision: 1, paramsRevision: 0, configRevision: 1, uiRevision: 0, config: { theme: "dark" }, uiState: null };' 'const ready: BridgeReadyPayload = { protocolVersion: 1, instanceId: "i", editorSessionId: "e", devMode: true, pluginName: "P", vendor: "V", capabilities: { paramGestures: true, paramFormatParse: true, stateConfig: true, subscriptions: true, meterStream: true, reliableEvents: true, diagnostics: true }, params: [], snapshot };' 'void ready;' > target/typecheck-smoke/plugin-ui-protocol-import.ts
npx tsc --strict --noEmit --target ES2022 --module ES2022 --moduleResolution Bundler target/typecheck-smoke/plugin-ui-protocol-import.ts
npm pack --workspace @vesty/plugin-ui --dry-run --json
rm -rf target/vesty-plugin-ui-protocol-check
cargo run --quiet -p vesty-cli -- export-types --out target/vesty-plugin-ui-protocol-check
for f in $(find target/vesty-plugin-ui-protocol-check/typescript -type f | sort); do rel=${f#target/vesty-plugin-ui-protocol-check/typescript/}; diff -q "$f" "packages/plugin-ui/src/$rel" >/dev/null || { echo "protocol drift: $rel"; exit 1; }; done
cargo test -p vesty-ipc exports_protocol_types_and_json_schema
cargo test -p vesty-cli protocol
```

新增行为:

- `packages/plugin-ui/src/protocol` 与 `src/serde_json` 现在保存由 `vesty-ipc::export_protocol_bindings()` 生成的 TypeScript 协议类型，包括 `BridgePacket`、`BridgeReadyPayload`、`BridgeHelloPayload`、`BridgeDiagnosticsSnapshot`、`PluginSnapshot`、`ParamChangedEvent`、`RtLogRecord` 和 `ParamSpec`。
- `@vesty/plugin-ui` package manifest 新增 `exports["./protocol"]` 和 `exports["./protocol/*"]`，发布包只包含 `dist`，让 UI 开发者可以从 `@vesty/plugin-ui/protocol` 引入精确协议类型。

本次 2026-06-09 针对 JSBridge subscription topic 校验:

```bash
cargo fmt --all --check
cargo check -p vesty-bridge -j1
cargo test -p vesty-bridge subscription -- --nocapture
cargo test -p vesty-bridge bridge_runtime -- --nocapture
npm --workspace @vesty/plugin-ui run typecheck
npm --workspace @vesty/plugin-ui test
npm test
cargo check -p vesty-ui-wry --features wry-backend -j1
cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture
```

新增行为:

- `vesty-bridge` 的 `subscription.add` / `subscription.remove` 共用 `validate_subscription_topic()`，除原有长度上限外，现在拒绝空 topic 和 ASCII/control 字符。
- 无效 topic 会返回 `validation_error`，不会写入 subscription table，也不会影响 meter/latest event/filter 状态。
- `@vesty/plugin-ui` 的 `subscribe()` 现在在本地 listener 表 mutation 和 `window.ipc.postMessage` 前同步拒绝非字符串 topic、空 topic、超过 128 UTF-8 bytes 的 topic 和控制字符 topic；测试覆盖无效 topic 不会发送 `subscription.add`。
- `vesty-ui-wry` 注入 bootstrap 的 fallback `subscribe()` 使用同一套 topic 预校验，避免不加载 SDK 的 Web UI 绕过前端 guard；bootstrap 文本测试固定 `assertSubscriptionTopic`、`TextEncoder` byte-length 和 `validation_error` 行为存在。
- `vesty-ui-wry` 注入 bootstrap 的 fallback `ready()` 现在和 `@vesty/plugin-ui` 一样校验 ready payload shape，覆盖 protocol version、metadata、capabilities、snapshot revisions、ParamSpec schema 和 MIDI mapping schema；畸形 ready payload 会以 `validation_error` 拒绝并允许 retry。
- `@vesty/plugin-ui` 和 `vesty-ui-wry` 注入 bootstrap 的 Promise 型 state/param API 现在会在发送 IPC 前做轻量预校验: config key、baseRevision、param id、normalized finite number、optional gestureId、param parse text、undefined config/state value。校验失败返回 rejected Promise，错误为 non-retryable `validation_error`；SDK 测试覆盖畸形 state/param command 不会触发 `window.ipc.postMessage`。
- SDK 顶层 `BridgePacket<T>` 继承生成的 `ProtocolBridgePacket` 并只放宽 `payload` 泛型；`BridgeError`、`PluginSnapshot`、`BridgeReadyPayload` 等常用类型改为直接引用/重导出 Rust 生成类型。
- 新增 `protocol-files.test.mjs`，确认 `npm run build` 后 `dist/index.d.ts`、`dist/protocol/index.d.ts` 和关键协议 declaration files 已输出。
- 新增 `vesty-cli` 单元测试 `plugin_ui_protocol_sources_match_generated_export`，每次从 Rust 源临时导出 TypeScript 协议类型，并逐文件比较 `packages/plugin-ui/src/protocol` / `src/serde_json`，把此前手动 shell drift check 固化为仓库测试。

本次 2026-06-08 针对 vanilla UI scaffold ready handshake 并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli ui_templates_emit_framework_specific_files
cargo test -p vesty-cli ui_package_template_can_use_local_plugin_ui_path
cargo test -p vesty-cli ui_package_template_infers_local_framework_adapter_paths
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo test -p vesty-cli
tmp=$(mktemp -d /tmp/vesty-vanilla-smoke.XXXXXX)
cd "$tmp"
cargo run --quiet --manifest-path /Users/orchiliao/Projects/vesty/Cargo.toml -p vesty-cli -- new vesty-vanilla-smoke --ui vanilla --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
cd "$tmp/vesty-vanilla-smoke/ui"
npm install --no-audit --no-fund
npm run build
npm run typecheck
cd "$tmp/vesty-vanilla-smoke"
cargo check
```

新增行为:

- vanilla UI 模板的 `main()` 现在先调用 `bridge.ready()`，完成 `bridge.hello` / `bridge.readyAck` 和 editor session 切换，再使用 `ready.snapshot`。此前模板直接 `snapshot.get`，不会完成 ready handshake。
- `ui_templates_emit_framework_specific_files` 现在覆盖 vanilla 输出，要求包含 `bridge.ready()`、`ready.snapshot`，并拒绝重新出现未 ready 的 `bridge.getSnapshot` 初始化模式。
- 真实 `vesty new --ui vanilla` scaffold smoke 已验证 UI `npm install`、Vite build、TypeScript typecheck 和 Rust `cargo check` 均通过。

本次 2026-06-08 针对 JSBridge ready handshake 幂等并通过:

```bash
npm run --workspace @vesty/plugin-ui test
npm test
npm run typecheck
cargo fmt --all --check
cargo test -p vesty-ui-wry
cargo clippy -p vesty-ui-wry --all-targets -- -D warnings
```

新增行为:

- `@vesty/plugin-ui` 的 `createBridge()` 现在缓存 `readyPromise` 和 ready payload。并发 `ready()` 只发送一次 `bridge.hello`，readyAck 成功后后续 `ready()` 直接返回同一个 payload；失败时清空 promise，允许重试。
- wry bootstrap 注入的 `window.__VESTY__.ready()` 使用同样的 ready promise / payload cache，保持内置全局 bridge 与 npm SDK 行为一致。
- `packages/plugin-ui/tests/bridge.test.mjs` 新增并发 ready 测试，确认第二次并发调用不追加 packet，ready 完成后的第三次调用也不重新发送 hello；同时覆盖 unsupported protocol 和 hello timeout 后清空 ready promise、允许下一次 ready 重试成功。
- `vesty-ui-wry` bootstrap string test 新增 ready cache 断言。

本次 2026-06-08 针对 ParamSpec JSBridge schema ergonomics 并通过:

```bash
cargo test -p vesty-params
cargo test -p vesty-ipc exports_protocol_types_and_json_schema
cargo test -p vesty-cli plugin_ui_protocol_sources_match_generated_export
npm run --workspace @vesty/plugin-ui test
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
```

新增行为:

- `ParamSpec` / `ParamFlags` / `ParamKind` 的 JSON/TypeScript wire schema 统一为 camelCase 和小写 tag: `defaultNormalized`、`stepCount`、`readOnly`、`programChange`、`"float"` / `"bool"` / `"choice"`。
- Rust `Deserialize` 使用内部 wire helper 保留旧 schema alias: `default_normalized`、`step_count`、`read_only`、`program_change`、`Float` / `Bool` / `Choice` 仍可读入；公开类型上不放 `serde(alias)`，避免 `ts-rs` warning。
- `packages/plugin-ui/src/protocol` 和 `dist/protocol` 已重新生成；JS declaration test 和 `vesty-ipc` export test 都会断言参数协议不再输出 snake_case。

本次 2026-06-08 针对 wry DevTools policy 显式化并通过:

```bash
cargo test -p vesty-ui-wry
cargo test -p vesty-ui-wry --features wry-backend
```

新增行为:

- `vesty-ui-wry` 创建 child WebView 时显式调用 `with_devtools(use_devtools())`，不再只依赖 wry 默认值。
- `use_devtools()` 默认保持 debug 构建开启、release 构建关闭；`VESTY_UI_DEVTOOLS=1|true|yes|on` 可显式开启，其它值会显式关闭。
- 新增 `devtools_policy_defaults_to_debug_and_allows_env_override` 单元测试，覆盖 env truthy parsing、默认策略和 override。
- CI 的 wry backend gate 已从 `cargo check -p vesty-ui-wry --features wry-backend` 提升为 `cargo test -p vesty-ui-wry --features wry-backend`，确保 feature-only UI runtime 测试在 GitHub Actions 中执行。

本次 2026-06-08 针对 VST3 editor attach 失败诊断并通过:

```bash
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' editor_attach_failure_is_traced
```

新增行为:

- `IPlugView::attached()` 在 unsupported platform、unsupported parent 和 wry runtime attach error 时通过 `VESTY_BRIDGE_TRACE` 写入 `editor_attach_*` marker，同时保持向 host 返回 `kResultFalse`。
- 新增 `editor_attach_failure_is_traced` 测试，使用 fake editor view 验证 unsupported platform attach 失败会留下 trace marker。
- CI 的 VST3+wry feature gate 已从 `cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"` 提升为 `cargo test -p vesty-vst3 --features "vst3-bindings wry-ui"`，确保 feature-only VST3/UI tests 在 GitHub Actions 中执行。

本次 2026-06-08 针对 VST3 state version migration gate 并通过:

```bash
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' vst3_state_migration_accepts_v1_and_rejects_future_versions
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' component_rejects_unsupported_state_version
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' controller_rejects_invalid_custom_state_without_param_partial_restore
```

新增行为:

- `bindings_impl` 新增 `VST3_STATE_VERSION = 1` 和 `migrate_vst3_state()`，写出 state 时使用版本常量，读入/应用 state 前统一进入迁移入口。
- 当前仅 v1 state 会被接受；future/unsupported version 会返回 `StateError::Deserialize`，COM `IComponent::setState()` 会拒绝该 stream 并返回 host error，不会静默套用参数或 custom state。
- 新增内部迁移单元测试和 fake COM `component_rejects_unsupported_state_version` 测试，覆盖 v1 接受、future version 拒绝，以及 VST3 host 边界行为。
- `apply_state()` 现在先完成 state version migration 和 custom state load，成功后才写参数；新增 fake COM `controller_rejects_invalid_custom_state_without_param_partial_restore` 测试，确认 invalid custom state 失败不会把参数半恢复到 plugin。

本次 2026-06-08 针对 release UI asset manifest URL-safe path hardening 并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build asset_manifest
cargo test -p vesty-ui-wry --features wry-backend asset
cargo clippy -p vesty-build --all-targets -- -D warnings
cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
```

新增行为:

- `vesty-build::AssetManifest::from_dir()` 生成 manifest 时会拒绝 dist 中含 URL 解析歧义的资源路径，例如 `%2e%2e` 这类 percent-encoded traversal 形态。
- `vesty-build` 静态 bundle validation 和 `vesty-ui-wry` runtime manifest loader 使用匹配的 path safety 规则: manifest path 必须是相对 URL-safe path，不能包含反斜杠、空段、`.`/`..` 段、ASCII control、`%`、`?`、`#` 或 `:`。
- `vesty-ui-wry` release custom protocol 请求路径也用同一规则 fail closed；`/assets/%2e%2e/app.js` 和 `/assets//app.js` 这类请求不会被折叠或解码成其它 manifest key，而是直接返回 404。
- `vesty-ui-wry` release custom protocol 在打开 manifest asset 前使用 `symlink_metadata()` 拒绝 symlink 和非 regular file；即使手工篡改 bundle manifest 指向 symlink，运行期也会返回 404。

本次 2026-06-09 针对 release WebView asset URL allowlist hardening 并通过:

```bash
cargo fmt --all --check
cargo check -p vesty-ui-wry --features wry-backend -j1
cargo test -p vesty-ui-wry --features wry-backend release_ -- --nocapture
cargo test -p vesty-ui-wry --features wry-backend asset -- --nocapture
```

新增行为:

- `vesty-ui-wry` release navigation / IPC allowlist 从字符串前缀检查收紧为 URL 组件检查。该轮曾兼容 `vesty://assets/...`、`http://vesty.assets/...`、`https://vesty.assets/...`；2026-06-12 已进一步收紧为仅允许 `vesty://assets/...` custom protocol，HTTP(S) shim origin 不再允许。
- allowlist 会拒绝空白包裹、ASCII control、反斜杠、userinfo、端口、非 asset host、query、fragment、percent-encoded traversal 和空路径段，避免形似 bundle asset URL 的 remote/malformed URL 进入 release WebView bridge。

本次 2026-06-08 针对 VST3 generated headers backend 前置 probe 并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3-sys
cargo test -p vesty-vst3-sys --no-default-features
cargo test -p vesty-vst3-sys --no-default-features --features generated-headers
cargo test -p vesty-cli doctor
cargo clippy -p vesty-vst3-sys --all-targets --all-features -- -D warnings
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo run --quiet -p vesty-cli -- doctor --format json > target/doctor-sdk-headers-smoke.json
python3 -m json.tool target/doctor-sdk-headers-smoke.json >/dev/null
rg -n 'vst3 SDK headers|VESTY_VST3_SDK_DIR|vst3 binding baseline' target/doctor-sdk-headers-smoke.json
```

新增行为:

- `vesty-vst3-sys` 新增 `REQUIRED_GENERATED_HEADER_INPUTS`、`SdkHeaderProbe`、`probe_sdk_headers()` 和 `probe_sdk_headers_from_env()`，用于检查官方 SDK checkout 是否包含后续生成 Vesty-owned VST3 bindings 所需的关键 `pluginterfaces` headers。
- `vesty doctor` 新增 `vst3 SDK headers` check；未设置 `VESTY_VST3_SDK_DIR` 时返回 `skipped` 且说明当前 upstream `vst3` crate backend 仍可用，设置后会报告 missing/ok header 覆盖。
- `vesty-vst3-sys` 的 baseline 测试现在覆盖 default upstream backend、`--no-default-features` metadata-only backend 和 `--features generated-headers` reserved backend，避免 feature matrix 下 backend 状态被旧断言误判。

本次 2026-06-09 针对 VST3 generated headers 输入 manifest 并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3-sys
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo check -p vesty-cli -j1
cargo test -p vesty-cli
```

新增行为:

- `vesty-vst3-sys` 新增 `SdkHeaderInputManifest`、`SdkHeaderInput`、`sdk_header_input_manifest()` 和 `check_sdk_header_input_manifest()`；manifest deterministic 记录 Steinberg SDK baseline、upstream `vst3` crate baseline、generator、version hint、required `pluginterfaces` headers、header size、sha256 和 missing headers。
- `probe_sdk_headers()`、manifest 生成、generated-bindings surface header 读取和 SDK version hint 读取都会用 no-follow regular-file 检查；required header 是 symlink 时 probe 视为 missing，manifest/surface 直接拒绝，避免后续 bindgen/com-scrape 读取不可审计或路径可替换的 SDK 输入。
- CLI 新增 `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out <path>`，可生成 SDK header input manifest；`--check --out <path>` 会复验已有 manifest，并在 header 内容、缺失项、baseline 或 generator 漂移时失败。未显式传 `--sdk-dir` 时会读取 `VESTY_VST3_SDK_DIR`。
- 该 manifest 是后续 generated-headers backend 的输入锁定/审计证据；它不表示完整 VST3 SDK 3.8 bindings 已经生成。

本次 2026-06-08 针对 `ProcessMode` public API/facade 导出和全量本地回归验证通过:

```bash
cargo fmt --all --check
cargo test -p vesty prelude_exports_process_mode -- --nocapture
cargo test -p vesty-core process_context_process_mode_defaults_and_overrides -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings processor_translates_automation_midi_and_transport -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo test -p vesty-vst3 --features vst3-bindings
cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo check -p vesty-ui-wry --features wry-backend
```

确认结果:

- `vesty_core::ProcessMode` 已经通过 `vesty::prelude::*` 导出，facade 单元测试 `prelude_exports_process_mode` 固定该 public API。
- `ProcessContext` 默认 `Realtime`，并可通过 `with_process_mode(ProcessMode::Offline)` 覆盖；VST3 adapter 会把 `ProcessData.processMode` 映射到 `Realtime` / `Prefetch` / `Offline`。
- VST3 fake process 测试已断言 `kOffline` 进入 developer kernel 后可通过 `ProcessContext::process_mode()` 读取为 `ProcessMode::Offline`。
- Rust workspace tests、workspace clippy、JS workspace typecheck/test、protocol snapshot drift check、`vst3-bindings` tests、VST3+wry feature tests/clippy 和 wry backend feature tests/clippy 均通过。

本次 2026-06-08 针对 `web-ui-param-demo` sample-accurate automation 示例一致性通过:

```bash
cargo fmt --all --check
cargo check -p vesty-example-web-ui-param-demo
cargo test -p vesty-core automation_segments -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `examples/web-ui-param-demo` 的 `mix` 参数处理已从 block 内 latest point 读取改为 `ParamAutomationSegments` 分段处理。
- Web UI 示例现在和 `gain`、`midi-synth` 以及 `vesty new` effect/instrument 模板一样，展示 sample-accurate、零分配的参数 automation 处理模式。
- meter emission 保持在处理后从输出 buffer 生成 `meter.main` 数据，UI/bridge 订阅链路不变。

本次 2026-06-08 针对 CI doctor artifact gate 的 VST3 SDK headers probe 覆盖并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli ci_doctor
cargo test -p vesty-cli release_check_accepts_ci_signing_and_notarization_evidence
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

新增行为:

- `release-check --ci-doctor-dir` 现在要求 `doctor-Linux.json`、`doctor-macOS.json` 和 `doctor-Windows.json` 中都包含 `vst3 SDK headers` check。
- 该 check 接受 `ok` 或 `skipped`: `ok` 表示 `VESTY_VST3_SDK_DIR` 指向的 SDK headers 齐全，`skipped` 表示默认 upstream `vst3` backend 仍可用且未启用 generated headers。
- 新增 `ci_doctor_artifacts_require_sdk_headers_check`，确认旧 doctor artifact 缺少该 check 时 release gate 会失败。
- `.agents/09-crash-safety-and-testing.md` 已修正 alpha release gate 表述: 当前本机 `release-check --strict` 不应被描述为已通过完整 DAW evidence，Cubase/Nuendo、Bitwig、Ableton Live 和 Studio One 仍缺真实 smoke。

本次 2026-06-08 针对 Web UI 参数手势收尾可靠性并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli ui_templates_emit_framework_specific_files
cargo clippy -p vesty-cli --all-targets -- -D warnings
npm run build --prefix examples/web-ui-param-demo/ui
tmp=$(mktemp -d /tmp/vesty-gesture-template-smoke.XXXXXX)
cd "$tmp"
for UI_TEMPLATE in vanilla react vue svelte; do
  cargo run --quiet --manifest-path /Users/orchiliao/Projects/vesty/Cargo.toml -p vesty-cli -- new "demo-${UI_TEMPLATE}" --ui "${UI_TEMPLATE}" --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
  npm install --prefix "demo-${UI_TEMPLATE}/ui" --no-audit --no-fund
  npm run build --prefix "demo-${UI_TEMPLATE}/ui"
  npm run typecheck --prefix "demo-${UI_TEMPLATE}/ui"
  cargo check --manifest-path "demo-${UI_TEMPLATE}/Cargo.toml"
done
```

新增行为:

- `vesty new --ui vanilla|react|vue|svelte` 生成的参数 slider 现在使用本地 `editing` guard，并在 pointerdown 时调用 `setPointerCapture(pointerId)`。
- 模板会在 `pointerup`、`pointercancel` 和 `lostpointercapture` 时调用同一个 guarded `endParamEdit` 收尾，防止 WebView 失焦、拖拽离开控件或 pointer cancel 造成 VST3 host beginEdit/endEdit 悬挂。
- `examples/web-ui-param-demo/ui/src/index.js` 和 `ui/dist/index.js` 已同步采用相同收尾模式，release demo bundle 的 UI asset 也随之更新。
- `ui_templates_emit_framework_specific_files` 已增加 pointer capture / cancel cleanup 断言，防止脚手架回退到只有 `pointerup` 的手势收尾。

本次 2026-06-08 针对 protocol snapshot release gate 诊断并通过:

```bash
cargo run --quiet -p vesty-cli -- export-types --out target/vesty-protocol
cargo run --quiet -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo fmt --all --check
cargo test -p vesty-cli protocol_ -- --nocapture
cargo clippy -p vesty-cli --all-targets -- -D warnings
cargo run --quiet -p vesty-cli -- release-check --strict --require-release-artifacts --format json
```

新增行为:

- 本地 `target/vesty-protocol` 已重新导出并通过 deterministic `export-types --check`，当前 protocol snapshot 项在 release-check 中为 `ok`。
- `check_protocol_export()` 的 drift error 现在包含 missing/changed/extra 的相对路径摘要，例如 `typescript/protocol/BridgePacket.ts`；stdout/stderr 被 CI 拆分时，JSON/markdown report 本身也能定位 drift 文件。
- `release-check` 的 protocol snapshot failure hint 现在使用实际 `--protocol-snapshot` 路径生成 `vesty export-types --out ...` 命令。
- `cargo run --quiet -p vesty-cli -- release-check --strict --require-release-artifacts --format json` 仍按预期失败，因为 Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One、CI run/artifacts、validator reports、签名和 notarization 证据尚未采集；这些外部证据未被伪造。

本次 2026-06-08 针对 CI doctor artifact 与 GitHub Actions run 绑定硬化并通过:

```bash
cargo fmt --all
cargo test -p vesty-cli ci_doctor -- --nocapture
cargo test -p vesty-cli doctor_report_includes_toolchain_webview_and_validator_checks -- --nocapture
```

新增行为:

- `vesty doctor --format json` 在 GitHub Actions 环境中会从 `GITHUB_SERVER_URL`、`GITHUB_REPOSITORY`、`GITHUB_RUN_ID` 和可选 `GITHUB_RUN_ATTEMPT` 生成 `ci_run_url`。
- `release-check` 在同时拥有 release `--ci-run-url` / `ci-run-url.txt` 和新版 doctor artifact `ci_run_url` 时，会确认 doctor artifacts 来自同一个 GitHub repo 和 run id；attempt 不必完全相同，允许同一 run 的不同 attempt artifact 被显式收集。
- 没有 `ci_run_url` 的旧 doctor JSON 仍然兼容，但不能提供 run 绑定证明；新增测试覆盖旧 artifact 兼容、run id mismatch 拒绝和 SDK headers check 继续生效。

本次 2026-06-08 针对 macOS 三示例 package/static validate smoke 并通过:

```bash
npm run build --prefix examples/web-ui-param-demo/ui
cargo build -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/vesty-paramid-v2-smoke/gain
cargo run -p vesty-cli -- package --config examples/web-ui-param-demo/vesty.toml --platform macos --binary target/release/libvesty_example_web_ui_param_demo.dylib --out target/vesty-paramid-v2-smoke/web
cargo run -p vesty-cli -- package --config examples/midi-synth/vesty.toml --platform macos --binary target/release/libvesty_example_midi_synth.dylib --out target/vesty-paramid-v2-smoke/midi
cargo run -p vesty-cli -- validate target/vesty-paramid-v2-smoke/gain/VestyGain.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json
cargo run -p vesty-cli -- validate target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json
cargo run -p vesty-cli -- validate target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json
cargo run -p vesty-cli -- release-check --skip-protocol --format json \
  --static-validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json \
  --report target/vesty-paramid-v2-smoke/static-release-check.json
```

结果:

- `VestyGain.vst3`、`VestyWebUIDemo.vst3`、`VestyMIDISynth.vst3` 均完成 macOS bundle packaging 和 `--static-only` validation。
- `VestyWebUIDemo.static-validate.json` 报告 `asset_count = 2`，确认 Web UI asset manifest 被打包进 release bundle。
- `target/vesty-paramid-v2-smoke/static-release-check.json` 的 `vst3 static validate reports` 和 `ci example static validate coverage` 均为 `ok`，覆盖平台为 `macos`。这仍只是本机 static packaging smoke，不替代 Steinberg validator passed report、Windows/Linux CI artifact 或真实 DAW smoke。

本次 2026-06-08 针对 macOS 三示例 Steinberg validator evidence 并通过:

```bash
cargo run -p vesty-cli -- validate target/vesty-paramid-v2-smoke/gain/VestyGain.vst3 --format json --report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json --validator-log target/vesty-paramid-v2-smoke/gain/VestyGain.validator.log
cargo run -p vesty-cli -- validate target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.vst3 --format json --report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json --validator-log target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validator.log
cargo run -p vesty-cli -- validate target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.vst3 --format json --report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json --validator-log target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validator.log
cargo run -p vesty-cli -- release-check --skip-protocol --format json \
  --validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json \
  --validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json \
  --validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json \
  --static-validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json \
  --report target/vesty-paramid-v2-smoke/release-check-validator-static.json
```

结果:

- 三个示例的 `validator.status = passed`、`exit_code = 0`、`tests_passed = 47`、`tests_failed = 0`。
- `target/vesty-paramid-v2-smoke/release-check-validator-static.json` 的 `vst3 validate reports`、`vst3 example validator coverage`、`vst3 static validate reports` 和 `ci example static validate coverage` 在非最终本地检查下均为 `ok`。
- 加上 `--strict --require-release-artifacts` 并只传入这些本机 macOS validator/static reports 后，最终 validator coverage 仍会因缺少 Linux/Windows 三示例 validator reports 而失败；此外 Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One 真实 DAW evidence、CI run/doctor artifacts、Linux/Windows static validate matrix、signed bundle evidence 和 notarization log 也仍缺失。

本次 2026-06-08 针对 static bundle binary format/architecture gate 并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build -- --nocapture
cargo clippy -p vesty-build --all-targets -- -D warnings
cargo run --quiet -p vesty-cli -- validate target/vesty-paramid-v2-smoke/gain/VestyGain.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json
cargo run --quiet -p vesty-cli -- validate target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json
cargo run --quiet -p vesty-cli -- validate target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.vst3 --static-only --format json --report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json
```

新增行为:

- `package_vst3()` 会在复制 binary 前校验目标平台文件魔数和架构: macOS 接受 64-bit Mach-O/fat Mach-O，Windows x64 接受 PE/MZ 且 COFF machine 为 x86_64，Linux x64 接受 64-bit ELF 且 `e_machine = x86_64`。
- `validate_vst3_bundle()` 会对 bundle 内所有识别到的平台 binary 做同样校验，防止把错误平台产物或文本占位文件作为 `--static-only` release evidence。
- `vesty-build` 新增 `package_rejects_binary_format_mismatch`、`validation_rejects_wrong_platform_binary_format`、`validation_rejects_non_x64_windows_binary` 和 `validation_rejects_non_x64_linux_binary`，同时 fixture 改为写入最小可解析平台 header。真实本机 macOS 三示例 bundle 的 binary magic 为 `cffaedfe`，已通过新 static validate。

本次 2026-06-08 针对 macOS signed bundle evidence hardening 并通过:

```bash
cargo fmt --all
cargo test -p vesty-cli signing_evidence -- --nocapture
cargo test -p vesty-cli release_evidence_dir_discovers_nested_signing_and_notary_evidence -- --nocapture
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
```

负向 release gate sanity check 也已运行，命令按预期返回非零并写出缺口报告:

```bash
cargo run --quiet -p vesty-cli -- release-check --strict --require-release-artifacts --format json --report target/release-check-current-strict.json
```

新增行为:

- `release-check` 不再因为 macOS `.vst3` bundle 里存在文本占位版 `Contents/_CodeSignature/CodeResources` 就接受 signed bundle evidence。
- `CodeResources` 必须是可解析 plist，root 必须是 dictionary，并且包含 `files` 或 `files2` dictionary 条目。
- 新增 `signing_evidence_rejects_code_resources_without_file_dictionary` 回归测试，防止合法 plist 但 `files2` 为字符串的占位证据通过。
- codesign/signtool 验证日志仍是发布流水线中更强的签名证据；macOS bundle 目录证据只作为已签名 bundle 结构的自动发现补充。
- 当前 `release-check --strict --require-release-artifacts` 仍按预期失败，缺口是 Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One DAW smoke、CI run/doctor artifacts、validator reports、CI static validate matrix、signed bundle evidence 和 notarization log；这些外部证据未被伪造。

本次 2026-06-08 针对 GitHub Actions artifact action baseline 复核并更新:

```bash
git ls-remote --tags https://github.com/actions/checkout.git
git ls-remote --tags https://github.com/actions/setup-node.git
git ls-remote --tags https://github.com/actions/upload-artifact.git
git ls-remote --tags https://github.com/actions/download-artifact.git
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml
```

新增行为:

- 官方 tags 显示 `actions/checkout@v6`、`actions/setup-node@v6`、`actions/upload-artifact@v7` 和 `actions/download-artifact@v8` 均存在。
- `.github/workflows/ci.yml` 的 release evidence snapshot 下载步骤从 `actions/download-artifact@v7` 升级到 `@v8`，让 artifact 聚合脚手架跟上当前可用 action baseline。
- `.github/workflows/ci.yml` 已通过 `actionlint v1.7.12` 语义检查。

本次 2026-06-08 针对 GitHub Actions publish-plan artifact 接入:

```bash
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml
cargo run -p vesty-cli -- publish-plan --out target/publish-plan/publish-plan.json
cargo run -p vesty-cli -- publish-plan --check --out target/publish-plan/publish-plan.json
cargo run -p vesty-cli -- release-order
```

新增行为:

- `.github/workflows/ci.yml` 新增 `publish plan` job，在 Ubuntu runner 用 `vesty publish-plan --out target/publish-plan/publish-plan.json` 生成 report，并用 `--check --out` 复验，同时生成 `target/publish-plan/release-order.txt`。
- CI 会上传 `vesty-publish-plan` artifact；`release-evidence` job 会下载该 artifact 到 `target/release-evidence/publish-plan`，让 consolidated evidence 目录同时包含 crate registry 发布顺序留档。
- 这只记录 dependency-safe 发布顺序，不执行 `cargo publish`，也不替代 API/semver review 或 crates.io/npm registry credentials。

本次 2026-06-08 针对 `vesty release-check` publish-plan evidence gate:

```bash
cargo test -p vesty-cli publish_plan_release_check -- --nocapture
cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths -- --nocapture
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
cargo test -p vesty-cli release_check_accepts_ci_signing_and_notarization_evidence -- --nocapture
cargo test -p vesty-cli release_check_requires_release_artifacts_when_requested -- --nocapture
cargo run -p vesty-cli -- publish-plan --out target/publish-plan-smoke.json
cargo run -p vesty-cli -- publish-plan --check --out target/publish-plan-smoke.json
cargo run -p vesty-cli -- release-check --skip-protocol --publish-plan-report target/publish-plan-smoke.json --format json --report target/release-check-publish-plan-smoke.json
python3 -m json.tool target/release-check-publish-plan-smoke.json >/dev/null
```

新增行为:

- `vesty release-check` 新增 `--publish-plan-report <path>`，读取 `vesty publish-plan --out <path>` 输出并校验 package order 连续、name/order 唯一、level 非零、内部依赖引用存在，且 dependency order/level 均早于 dependent。
- `--release-evidence-dir` 会自动采纳 `publish-plan/publish-plan.json` 或根目录 `publish-plan.json` 中有效的 publish plan evidence；模板只创建 `publish-plan/README.md`，不会生成 pending JSON 伪证据。
- `--require-release-artifacts` 现在会要求 crate publish plan evidence 存在并有效；缺失时 `crate publish plan` check 为 failed，普通非严格检查仍为 skipped。

本次 2026-06-08 针对 release evidence `ci-run-url.txt` 解析硬化:

```bash
cargo test -p vesty-cli ci_run_url -- --nocapture
cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths -- --nocapture
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml
```

新增行为:

- `ci-run-url.txt` 现在只接受裸 URL、`ci_run_url=<url>` 或 `ci-run-url=<url>`。
- 其它 `key=value` 说明行会被忽略，避免 release evidence README/注释里的 URL 被误采纳。
- `pending` 比较改为大小写不敏感，模板里的 `PENDING` 不会污染 release gate。

本次 2026-06-08 针对 realtime automation segment seeded fuzz 补强:

```bash
cargo test -p vesty-core automation_segments_cover_block_for_seeded_fuzz_patterns -- --nocapture
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo test -p vesty-vst3 --features vst3-bindings
cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo check -p vesty-ui-wry --features wry-backend
```

新增行为:

- `vesty-core` 新增 deterministic seeded fuzz 风格单元测试，覆盖 `ParamAutomationSegments` 在同 offset、其它 param handle、非参数 MIDI event 和越界 automation point 混入时的行为。
- 测试逐 sample 验证 segment value，确保 segment 有序、不为空、不重叠，并完整覆盖当前 audio block。

本次 2026-06-08 针对 offline render process mode API 补强:

```bash
cargo test -p vesty-core process_context_process_mode_defaults_and_overrides -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings processor_translates_automation_midi_and_transport -- --nocapture
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo test -p vesty-vst3 --features vst3-bindings
cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo check -p vesty-ui-wry --features wry-backend
```

新增行为:

- `vesty-core::ProcessMode` 提供 `Realtime`、`Prefetch` 和 `Offline` 三种 block mode。
- `ProcessContext::new()` 默认 `Realtime`；`with_process_mode()` 可由 host adapter 注入模式，`process_mode()` 供 `AudioKernel` 读取。
- `vesty-vst3` 将 VST3 `ProcessData.processMode` 映射到 core `ProcessMode`，fake COM 测试覆盖 offline block 进入 developer kernel。

本次 2026-06-08 针对 VST3 editor view resize/remove 生命周期补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3 --features vst3-bindings controller_creates_editor_view -- --nocapture
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui" controller_creates_editor_view -- --nocapture
cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings wry-ui" -- -D warnings
```

新增覆盖:

- fake `IPlugView` 测试现在覆盖 `canResize()`、`getSize(null)`、`checkSizeConstraint(null)` 和 `onSize(null)` 的返回值，防止 host 传入异常指针时越过 COM 边界 panic。
- `onSize()` 会 clamp 到 descriptor 的 `min_width` / `min_height`，并更新后续 `getSize()` 返回值；大尺寸 resize 会保持 host 传入的 rect。
- `removed()` 可重复调用并返回 `kResultOk`，固定 editor close/reopen 路径的幂等 detach 行为；真实 host/WebView attach 仍需 DAW smoke 证明。

本次 2026-06-08 针对 VST3 fake-host editor open/close/resize stress 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3 --features vst3-bindings editor_open_close_resize_fake_host_stress -- --nocapture
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui" editor_open_close_resize_fake_host_stress -- --nocapture
```

新增覆盖:

- `editor_open_close_resize_fake_host_stress` 使用 fake VST3 host 循环 128 次创建 controller 和 `IPlugView`，覆盖 unknown view name rejection、初始 size、`canResize()`、`checkSizeConstraint()` min clamp、`onSize()` 后 `getSize()` 同步，以及 `removed()` 重复调用。
- 同一测试在 `vst3-bindings` 和 `vst3-bindings wry-ui` feature 组合下通过，固定带 bridge endpoint 的 view 创建和未 attach runtime 的幂等 detach 路径。
- 这不是真实 host/WebView 证据；Windows WebView2、Linux WebKitGTK/X11、以及 Cubase/Nuendo、REAPER、Bitwig、Ableton Live、Studio One 中的 editor open/close/resize stress 仍需采集。
- 顺手清理 `IPlugView::removed()` 在未启用 `wry-ui` 时的 `unused_unsafe` warning，保持严格 lint 输出干净；`onSize()` 的 wry runtime resize 行为保持不变。

本次 2026-06-08 针对 `UiDescriptor` public builder API 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-core ui_descriptor_builders_override_editor_geometry -- --nocapture
cargo test -p vesty -- --nocapture
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui" controller_creates_editor_view -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `UiDescriptor::with_size(width, height)`、`with_min_size(min_width, min_height)` 和 `with_resizable(bool)` 已接入，和既有 `with_dev_url()` 一起形成完整链式 Web UI editor descriptor 配置。
- `vesty-core` 单元测试固定默认 assets dir、dev URL、初始尺寸、最小尺寸和 resizable 覆盖行为。
- `.agents/03-module-design.md` 与 `.agents/08-developer-guide.md` 已更新为链式 descriptor 示例；VST3 fake `IPlugView` resize 约束测试继续通过。

本次 2026-06-08 针对 `vesty new` UI descriptor/template size 同步通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli new_project_templates_use_params_derive -- --nocapture
cargo clippy -p vesty-cli --all-targets -- -D warnings
rm -rf target/ui-descriptor-template-smoke
mkdir -p target/ui-descriptor-template-smoke
cd target/ui-descriptor-template-smoke
cargo run --quiet --manifest-path /Users/orchiliao/Projects/vesty/Cargo.toml -p vesty-cli -- new descriptor-demo --ui vanilla --vesty-path /Users/orchiliao/Projects/vesty/crates/vesty --plugin-ui-path /Users/orchiliao/Projects/vesty/packages/plugin-ui
cargo check --manifest-path descriptor-demo/Cargo.toml
rg -n "with_size|with_min_size|with_resizable|width = 900|min_width = 640" descriptor-demo/src/lib.rs descriptor-demo/vesty.toml
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- 有 UI 的 effect/instrument Rust 模板现在会生成 `UiDescriptor::web_assets("ui").with_dev_url(...).with_size(900, 560).with_min_size(640, 420).with_resizable(true)`。
- `vesty.toml` 的默认 `[ui] width/height/min_width/min_height` 与生成的 Rust `UiDescriptor` 默认值显式保持一致，不再只是依赖 `UiDescriptor::web_assets()` 当前默认值碰巧相同。
- 真实 `vesty new --ui vanilla` scaffold smoke 已确认生成项目可 `cargo check`，且 `vesty.toml` 和 `src/lib.rs` 都包含对应尺寸约束。

本次 2026-06-08 针对 `[ui]` config validation 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build package_rejects_invalid_ui_config -- --nocapture
cargo test -p vesty-build validates_packaged_vst3_bundle -- --nocapture
cargo test -p vesty-build
cargo clippy -p vesty-build --all-targets -- -D warnings
```

新增行为:

- `vesty-build::validate_config()` 现在校验 `[ui].dir` 非空，并要求可选 `dev_url/build/dist` 出现时非空。
- `[ui].width/height` 与 `[ui].min_width/min_height` 必须成对出现且大于 0；如果 initial size 和 min size 同时存在，min 不能超过 initial。
- `package_rejects_invalid_ui_config` 覆盖空字段、缺失成对尺寸、0 尺寸、`min_width > width` 和 `min_height > height`，防止无效 editor geometry 进入 package/static validate 流程。

本次 2026-06-08 针对 `[package].signing` config validation 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build package_rejects_empty_signing_identity_when_present -- --nocapture
cargo test -p vesty-build package_rejects_invalid_ui_config -- --nocapture
cargo test -p vesty-build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

新增行为:

- `[package].signing` 如果出现在 `vesty.toml` 中必须是非空字符串；未填写仍表示 `vesty package` 不执行内置签名步骤。
- 这样可以防止 release 配置里写了空 signing identity 时被 CLI 静默当作 unsigned package 处理。
- `.agents/14-completion-audit.md` 新增原始计划到当前证据的完成度审计，明确区分 alpha skeleton 已实现项和仍需真实 DAW/CI/签名/公证证据的 release blockers。

本次 2026-06-08 针对 `[plugin].kind` config/template validation 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build kind -- --nocapture
cargo test -p vesty-cli kind -- --nocapture
cargo test -p vesty-cli vesty_toml_template_includes_package_metadata -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `vesty-build::validate_config()` 现在只接受 `effect`、`fx`、`audio-effect`、`audio_effect` 或 `instrument` 作为 `[plugin].kind`，防止任意字符串进入 package metadata。
- `[package].category` 为空或缺失时不再把原始 kind 字符串直接写入 `moduleinfo.json`，而是按框架 kind 映射为 VST3 category `Fx` 或 `Instrument`。
- `vesty new --kind` 会使用同一组支持值，并把 effect 同义词规范化成生成项目中的 `kind = "effect"`；未知 kind 会直接报错，不生成后续 package 才失败的项目。

本次 2026-06-08 针对 `[package].bundle_id` config validation 和 macOS fallback 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build bundle_id -- --nocapture
cargo test -p vesty-build validates_packaged_vst3_bundle -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `[package].bundle_id` 现在必须符合 conservative reverse-DNS shape: 至少包含一个 `.`，每段非空，只允许 ASCII 字母、数字和 `-`，且段首/段尾不能是 `-`。
- macOS `Info.plist` fallback bundle id 现在会从 executable 名称规整生成 `dev.vesty.<name>`，例如 `My_Plugin` 会得到 `dev.vesty.my-plugin`，避免 fallback 自己写出不合格 `CFBundleIdentifier`。

本次 2026-06-08 针对 macOS static validator bundle id shape 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build macos_validation_rejects_invalid_bundle_identifier_plist -- --nocapture
cargo test -p vesty-build validates_packaged_vst3_bundle -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `validate_vst3_bundle()` 现在对现有 macOS `.vst3` bundle 的 `Info.plist CFBundleIdentifier` 也执行同一套 conservative reverse-DNS shape 校验。
- 手工篡改或外部工具生成的 invalid `CFBundleIdentifier` 会在静态检查阶段返回 `InvalidBundle`，不再只要求非空。

本次 2026-06-08 针对 macOS static validator executable/moduleinfo 一致性补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build executable_plist -- --nocapture
cargo test -p vesty-build validates_packaged_vst3_bundle -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `validate_vst3_bundle()` 现在要求 `Info.plist CFBundleExecutable` 指向的文件不只是存在，还必须与 `moduleinfo.json` name 推导出的 macOS binary name 一致。
- 如果 plist 指向缺失 binary，仍保持 `MissingFile`；如果 plist 指向额外存在但不匹配的 binary，则返回 `InvalidBundle`。

本次 2026-06-08 针对 macOS static validator bundle name/moduleinfo 一致性补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build bundle_name -- --nocapture
cargo test -p vesty-build validates_packaged_vst3_bundle -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `validate_vst3_bundle()` 现在要求 `Info.plist CFBundleName` 与 `moduleinfo.json` 的 plugin name 一致。
- 手工篡改 `CFBundleName` 或外部打包工具写出不一致 display name 时，静态检查会返回 `InvalidBundle`。

本次 2026-06-08 针对 macOS static validator version/moduleinfo 一致性补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build version_plist -- --nocapture
cargo test -p vesty-build validates_packaged_vst3_bundle -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `validate_vst3_bundle()` 现在要求 `Info.plist CFBundleShortVersionString` 与 `CFBundleVersion` 都匹配 `moduleinfo.json` 的 `plugin_version`。
- 手工篡改 plist version 或外部打包工具写出不一致版本时，静态检查会返回 `InvalidBundle`。

本次 2026-06-08 针对 config/moduleinfo metadata control character validation 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-build control -- --nocapture
cargo test -p vesty-build validates_packaged_vst3_bundle -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

新增行为:

- `vesty-build::validate_config()` 现在会拒绝 `[plugin].name/vendor/version/kind`、`[package].bundle_id/category/signing` 和 `[ui].dir/dev_url/build/dist` 中的 control characters。
- `[package].category` 仍允许空字符串或纯空白以启用 `[plugin].kind` -> `Fx` / `Instrument` fallback，但不允许换行等不可见控制字符进入 `moduleinfo.json` category。
- `validate_vst3_bundle()` 现在会拒绝 `moduleinfo.json` 顶层 `name`、`vendor`、`plugin_version` 以及 class `name/category` 中的 control characters，防止外部打包或手工篡改产物污染 host scan metadata。

本次 2026-06-08 针对 crates.io/npm package metadata 和根文档补强:

```bash
npm install --package-lock-only
cargo fmt --all --check
cargo test -p vesty-cli workspace_packages_have_release_metadata -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm test
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
npm pack --workspaces --dry-run --json
cargo metadata --no-deps --format-version 1
cargo package -p vesty-params --allow-dirty --no-verify
cargo package -p vesty-build --allow-dirty --no-verify
cargo package -p vesty-macros --allow-dirty --no-verify
cargo package -p vesty-vst3-sys --allow-dirty --no-verify
cargo test -p vesty-cli publish_plan -- --nocapture
cargo run -p vesty-cli -- publish-plan --out target/publish-plan-smoke.json
cargo run -p vesty-cli -- publish-plan --check --out target/publish-plan-smoke.json
! rg -n "https://example.com/vesty|dev@example.com" examples
```

新增行为/证据:

- 新增根 `README.md`、`LICENSE-MIT` 和 `LICENSE-APACHE`，与 workspace license `MIT OR Apache-2.0` 对齐。
- 根 `README.md` 的最小插件示例已使用当前真实 API: `FloatParam::new(...)` 参数构造、`Plugin::params()`、`create_kernel(&self, KernelInit)`、`ProcessContext<'_>` 和 per-channel `copy_input_to_output()`；不再展示旧的 `#[param(...)]` 字段属性或旧 `create_kernel(params: ...)` 形态。
- Rust workspace metadata 已从 `https://example.com/vesty` 占位 repository 改为真实 repository/homepage 字段，并补齐 authors、keywords 和 crates.io categories。
- 所有 `crates/*` manifest 现在都有 description、workspace authors/categories/homepage/keywords 和 `readme = "../../README.md"`；`examples/*` 已显式 `publish = false`。
- 三个 example plugin 的 `PluginInfo` 已移除 `https://example.com/vesty` / `dev@example.com` 占位值，统一使用项目 URL 和空 email；`workspace_packages_have_release_metadata` 会防止示例 metadata 回退。
- `packages/plugin-ui`、`packages/react`、`packages/vue`、`packages/svelte` 已补 description/license/repository/homepage/keywords；React/Vue/Svelte 适配包也补齐 `exports` 和 `files = ["dist"]`。
- 新增 `workspace_packages_have_release_metadata` 单元测试，防止 README 示例 API 漂移、占位 repository、缺 readme/license、示例可发布或 JS package metadata 退化。
- `npm pack --workspaces --dry-run --json` 通过，确认四个 JS package 的发布包边界只包含 `dist` 和 `package.json`。
- `cargo package -p vesty-params|vesty-build|vesty-macros|vesty-vst3-sys --allow-dirty --no-verify` 通过，验证无内部未发布依赖的叶子 crate 已可打包。`cargo package -p vesty` 仍会因为 `vesty-bridge` 等内部 crates 尚未发布到 crates.io 而失败；这属于多 crate workspace 的发布顺序问题，不是 metadata/readme/license 缺失。
- 新增 `vesty publish-plan`，alias 为 `vesty release-order`。命令从 Cargo metadata 生成 publishable workspace crates 的 dependency-safe 顺序，跳过 `publish = false` 的 examples，并在可发布 crate 依赖 private workspace package 时返回非零。`--out <path>` 会写入规范 JSON 并立即复用 release gate validator；`--check --out <path>` 只复验已有 report。
- 当前 `publish-plan --format json` 输出的 crate 顺序为: `vesty-params`、`vesty-macros`、`vesty-vst3-sys`、`vesty-build`、`vesty-core`、`vesty-ipc`、`vesty-rt`、`vesty-ui`、`vesty-cli`、`vesty-bridge`、`vesty-ui-wry`、`vesty-vst3`、`vesty`。

本次 2026-06-08 针对 `vesty new` scaffold README 和 crate publish safety 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli template -- --nocapture
cargo test -p vesty-cli ui_templates_emit_framework_specific_files -- --nocapture
cargo run -p vesty-cli -- new target/vesty-new-smoke.<tmp>/readme-smoke --ui none --vesty-path crates/vesty
```

新增行为:

- `vesty new` 现在会生成项目级 `README.md`，说明布局、headless/Web UI 差异、UI build、`vesty build/package/validate` 和 `vesty doctor` 起步命令。
- 生成的 `Cargo.toml` 现在带 `description = "<Plugin> VST3 plugin"` 和 `publish = false`，避免插件 cdylib 项目被误当 crates.io library 发布。
- 新增 `project_readme_template_documents_generated_project_flow` 覆盖 React/UI 与 headless/instrument 两类 README 分支；`cargo_template_can_use_local_vesty_path` 同时覆盖 `publish = false` 和 description。
- 真实 `vesty new ... --ui none --vesty-path crates/vesty` smoke 已确认生成项目含 `README.md`、`Cargo.toml publish = false`、headless 说明和 `vesty doctor` 提示。

本次 2026-06-08 针对 `vesty new` UI package publish safety 补强:

- 有 UI 的脚手架模板现在会在生成的 `ui/package.json` 写入 `"private": true`，明确该子工程是插件 UI asset app，而不是 npm library package。
- `ui_package_template_can_use_local_plugin_ui_path` 和 `ui_templates_emit_framework_specific_files` 已覆盖默认发布依赖、本地 `file:` 依赖以及 React/Vue/Svelte 模板均保持 `private: true`。
- 真实 `vesty new ... --ui vanilla --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui` smoke 已确认生成的 `ui/package.json` 同时包含 `private: true` 和本地 `@vesty/plugin-ui` file dependency。
- `.agents/03-module-design.md` 中的 public API 示例已移除旧 `example.com`/`dev@example.com` 占位 contact metadata，保持与 README 和 examples 的当前风格一致。
- `vesty-ui-wry` 已开启 `#![deny(clippy::undocumented_unsafe_blocks)]`，并为平台 parent handle、WebView IPC 回推指针和测试环境变量 unsafe block 补齐 `SAFETY:` 注释；`cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings` 通过。
- `vesty-vst3` crate root 和 `src/bindings_impl.rs` 已显式开启 `#![deny(unsafe_op_in_unsafe_fn)]` 与 `#![deny(clippy::undocumented_unsafe_blocks)]`；`cargo fix --workspace --allow-dirty --allow-no-vcs --all-targets` 迁移了 2024 unsafe-op blocks，随后为 production COM helper、fake COM tests 和 raw callback tests 补齐 `SAFETY:` 注释。`cargo clippy --workspace --all-targets -- -D warnings` 和 `cargo test --workspace` 通过。

本次 2026-06-08 针对 CI doctor artifact OS evidence hardening 补强并通过:

```bash
cargo fmt --all --check
cargo test -p vesty-cli ci_doctor_artifacts -- --nocapture
cargo test -p vesty-cli doctor_report_includes_toolchain_webview_and_validator_checks -- --nocapture
```

新增行为:

- `vesty doctor --format json` 现在输出 `os` 字段，值为 `Linux`、`macOS`、`Windows` 或 `unknown`，用于标记该 doctor report 的真实生成平台。
- `release-check --ci-doctor-dir` 读取 `doctor-Linux.json`、`doctor-macOS.json`、`doctor-Windows.json` 时，如果 report 内存在 `os` 字段，会校验它和文件名推断平台一致；错名或错放 artifact 会让 `ci doctor artifacts` check 失败，并指出具体 JSON 路径。
- 旧版无 `os` 字段的 doctor artifact 仍保持 legacy 兼容，不会因为 schema 升级立刻失效；但新版 CI artifact 会有更强的防错能力。
- 新增 `ci_doctor_artifacts_reject_os_label_mismatch_when_present` 与 `ci_doctor_artifacts_allow_legacy_reports_without_os_label` 回归测试，防止后续 release evidence gate 误把跨 OS 错放报告当成完整平台覆盖。

本次 2026-06-08 针对 npm package release evidence gate 补强:

```bash
cargo run -p vesty-cli -- npm-pack --out target/npm-pack-smoke/npm-pack.json
cargo run -p vesty-cli -- npm-pack --check --out target/npm-pack-smoke/npm-pack.json
cargo test -p vesty-cli npm_pack_release_check -- --nocapture
cargo test -p vesty-cli release_check_accepts_ci_signing_and_notarization_evidence -- --nocapture
cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths -- --nocapture
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
```

新增行为:

- `release-check` 新增 `--npm-pack-report <path>`，读取 `npm pack --workspaces --dry-run --json` 输出，验证 `@vesty/plugin-ui`、`@vesty/react`、`@vesty/vue` 和 `@vesty/svelte` 四个 package 全部存在。
- `vesty npm-pack --out <path>` 现在会运行 npm workspace dry-run pack，写入规范 JSON，并立即复用同一套 release-check validator；`vesty npm-pack --check --out <path>` 可复验已有 artifact。
- npm pack report gate 会验证每个 package 有非空 version、`.tgz` filename、`package.json`、`dist/**` 文件，并拒绝 `src/**`、`tests/**`、绝对路径、`..` 或其它非发布边界文件进入 packed tarball。
- `--release-evidence-dir` 会自动发现 `npm-pack/npm-pack.json` 或根目录 `npm-pack.json`；`--require-release-artifacts` 会要求该 evidence 存在并有效。
- `release-check --write-evidence-template` 现在创建 `npm-pack/README.md`，说明 dry-run 命令和严格 gate 语义；README 只是占位，不会被当作 npm pack evidence。
- GitHub Actions `js sdk` job 会通过 `vesty npm-pack --out target/npm-pack/npm-pack.json` 生成并复验 `vesty-npm-pack` artifact；`release evidence snapshot` job 会下载到 `target/release-evidence/npm-pack`，让 consolidated release report 自动纳入 JS package publish boundary evidence。它仍不执行 `npm publish`，registry credentials 和实际发布仍属于外部 release 步骤。

本次 2026-06-08 针对 CI per-OS release-check artifact gate 补强:

```bash
cargo fmt --all --check
cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
cargo test -p vesty-cli release_check_requires_release_artifacts_when_requested -- --nocapture
cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths -- --nocapture
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
cargo clippy -p vesty-cli --all-targets -- -D warnings
```

新增行为:

- `release-check` 新增 `--ci-release-check-dir <dir>`，递归读取 `release-check-Linux.json`、`release-check-macOS.json` 和 `release-check-Windows.json`。
- 该 gate 要求三平台 snapshot 都存在且 JSON 可解析，并确认 host profile coverage、protocol snapshot skip/ok、VST3 binding baseline 等本地 invariant 通过；当 release evidence 同时提供 `ci-run-url.txt` / `--ci-run-url` 时，会校验每个 snapshot 的 `ci_run_url` 来自同一个 GitHub repo/run id。DAW smoke、validator、signing、notarization 等外部证据项允许继续由各自 gate 报告失败，避免把“尚未采集外部证据”误判成 CI runner 本地检查坏掉。这里允许的 protocol snapshot skip 只适用于 per-OS snapshot；最终 consolidated `--strict --require-release-artifacts` gate 必须检查 `--protocol-snapshot`，且会拒绝 `--skip-protocol`。
- CI per-OS release-check snapshot 还会检查 report 顶层 `status` 与内部 check status 自洽，拒绝未知 check status、重复 check name、伪造 host profile coverage 数量、非 `--skip-protocol` 形态的 protocol skip，以及缺少当前 Steinberg SDK baseline / upstream `vst3` crate baseline / binding backend 的 VST3 binding baseline 值；这防止手写/损坏的 release-check JSON 只靠少量 invariant 字段通过导入。
- `--release-evidence-dir` 会自动发现 `ci-release-checks/` 中的 JSON artifacts；空模板 README 不会被当作 evidence。
- `--require-release-artifacts` 现在会要求 CI per-OS release-check snapshots 存在并有效，缺失时 `ci release-check artifacts` check 为 failed，普通非严格检查仍为 skipped。
- `release-check --write-evidence-template` 现在创建 `ci-release-checks/README.md`，说明三个 per-OS snapshot 的文件名和边界语义；README 只是占位，不会被当作 CI release-check evidence。
- release evidence / platform smoke pending JSON 模板生成现在以 fallible serialization 传播错误，不再依赖 `expect()`；UI scaffold 的 `package.json` string literal helper 也改为 `serde_json::Value::String(...).to_string()`，保留 JSON 转义且不 panic。

本次 2026-06-11 针对 CI per-OS release-check artifact invariant 自洽性补强:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current.json
```

新增/调整:

- `validate_ci_release_check_report()` 现在拒绝重复 check name，防止一个 snapshot 通过追加第二个同名 `ok` invariant 混淆 gate。
- `host profiles` invariant 的 `value` 必须证明当前 `vesty_core::host_profiles()` 数量和 release host profile coverage，不能只把 status 手写为 `ok`。
- `protocol snapshot` invariant 如果为 `skipped`，`value` 必须明确是 `--skip-protocol`；该宽限只保留给 per-OS CI snapshot，最终 consolidated strict release gate 仍拒绝 `--skip-protocol`。
- `vst3 binding baseline` invariant 的 `value` 必须包含当前 Steinberg SDK baseline、upstream `vst3` crate baseline 和 active binding backend；只写 `Steinberg SDK v3.8.0_build_66` 这类不完整文本会被拒绝。
- `ci_release_check_artifacts_reject_duplicate_or_forged_invariant_checks` 覆盖重复 check、伪造 host coverage、伪造 binding baseline 和模糊 protocol skip；测试夹具更新为当前真实 `daw matrix` check 名，同时 gate 兼容旧 `daw smoke matrix` 名称。
- 本地验证通过: `ci_release_check_artifacts` 6 passed、`import_ci` 2 passed、`release_evidence` 10 passed、`release_check` 33 passed、workspace Rust tests 470 passed、workspace clippy no issues、JS workspace tests passed。strict `release-check` 仍按预期失败在真实 DAW/platform/CI/validator/signing/notarization evidence 缺失；本地 invariant 中 host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok。

本次 2026-06-08 针对 `@vesty/plugin-ui` reload/close stress 补强:

```bash
npm --workspace @vesty/plugin-ui test
npm --workspace @vesty/plugin-ui run typecheck
cargo test -p vesty-ui-wry bootstrap_script_registers_host_subscriptions -- --nocapture
cargo clippy -p vesty-ui-wry --all-targets -- -D warnings
```

新增行为:

- `createBridge()` 和 wry 注入 bootstrap 的 `pagehide` / `beforeunload` cleanup 现在除了停止 async event pump、拒绝 pending request、清理 ready cache 之外，还会清空 JS 侧 topic listeners。
- 新增 32 次循环的 Node stress 测试，覆盖每次创建 bridge、发送 pending command、订阅 `meter.main` 启动 event pump、手动触发 `event.flush`、`pagehide` cleanup、pending request reject、interval clear、unload 后 unsubscribe 不再发送 remove request，以及同 session 迟到 event/response 不触发旧 handler。
- 该测试补强了 `.agents/12-jsbridge-design.md` 中 WebView reload/close recovery 的本地自动化证据；真实 DAW/WebView close/reopen stress 仍属于 release evidence。

本次 2026-06-08 针对 `vesty.toml` strict schema 补强:

```bash
cargo fmt --all --check
cargo test -p vesty-build read_config -- --nocapture
cargo test -p vesty-build
cargo clippy -p vesty-build --all-targets -- -D warnings
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

新增行为:

- `VestyConfig`、`PluginConfig`、`UiConfig` 和 `PackageConfig` 反序列化现在使用 `deny_unknown_fields`，未知顶层 table 或未知字段会在 `read_config()` 阶段失败。
- 新增 `read_config_rejects_unknown_future_scope_fields_and_tables`，当时固定拒绝 `[plugin].sidechain`、顶层 `[bus]`、`[ui].experimental_wayland` 和 `[package].installer`，避免未实现的 sidechain、多输出 bus、Wayland embedding 或 installer DSL 被静默忽略。2026-06-09 sidechain MVP 落地后，`[plugin].sidechain` 已迁出未知字段拒绝列表，effect 可用，instrument 仍被配置校验拒绝。
- 新增 `read_config_accepts_current_schema_for_examples`，确认 `examples/gain`、`examples/midi-synth` 和 `examples/web-ui-param-demo` 的现有合法 `vesty.toml` 仍能解析。
- `.agents/07-build-packaging.md` 和 `.agents/08-developer-guide.md` 已同步说明严格 schema 语义，未来 scope 必须等实现落地后再扩展配置。

本次 2026-06-08 针对 `#[derive(Params)]` 字段属性补强:

```bash
cargo fmt --all --check
cargo test -p vesty params_derive -- --nocapture
cargo test -p vesty-macros
```

新增行为:

- `vesty-macros` 的 `#[derive(Params)]` 现在支持 `#[param(id = "stable-id")]`，derive 生成的 `ParamCollection` 会用覆盖后的 ID 导出 `ParamSpec`，并用同一 ID 进行 `resolve()`、`get_normalized()`、`set_normalized()` 和 handle 映射。
- `#[param(bypass)]` 现在可用于 `BoolParam` 字段，导出的 `ParamSpec.flags.bypass` 会被置为 true；非 `BoolParam` 字段会编译时报错。
- `#[param(skip)]` 保持用于非参数字段，并禁止和 `id` / `bypass` 混用，避免无效属性被静默忽略。
- 新增 facade crate 测试 `params_derive_supports_id_and_bypass_attributes`，确认覆盖 ID 不再解析内部 constructor ID，且 bypass flag 由 derive 属性导出；`vesty-macros` 自身也新增 AST 级负例测试，覆盖非 BoolParam 使用 bypass、skip/id 混用和重复 id 属性。
- `.agents/08-developer-guide.md` 已从“后续宏增强”更新为已实现 API 说明。

本次 2026-06-08 针对参数 flag builder 补强:

```bash
cargo fmt --all
cargo test -p vesty-params param_flag -- --nocapture
cargo clippy -p vesty-params --all-targets -- -D warnings
```

新增行为:

- `ParamSpec` 新增 `.with_automatable(bool)` 和 `.as_read_only()` builder；`as_read_only()` 会同时设置 `read_only = true` 和 `automatable = false`，符合 meter/read-only 参数不暴露 host automation 的约定。
- `FloatParam`、`BoolParam` 和 `ChoiceParam` 新增 `.with_automatable(bool)` 与 `.as_read_only()`，开发者不再需要手写 `ParamCollection` 或私有字段改动才能声明 meter/analyzer/read-only 参数。
- `BoolParam` 新增 `.as_bypass()` builder，`BoolParam::bypass(...)` 复用该路径；constructor 风格和 builder 风格都能设置 VST3 bypass flag。
- 新增 `param_flag_builders_mark_bypass_read_only_and_non_automatable` 单测覆盖 ParamSpec 和三类参数 builder 的 flag 语义。
- `.agents/08-developer-guide.md` 已更新参数 ID/flag 规则，明确这些 builder 是已实现 API。

本次 2026-06-08 针对参数 schema validation 补强:

```bash
cargo fmt --all --check
cargo test -p vesty-params validates_param_specs -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' factory_rejects_controller_with_invalid_param_schema -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' controller_maps_bypass_and_read_only_parameter_flags -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' factory_creates_component_and_controller_instances -- --nocapture
```

新增行为:

- `vesty-params` 新增 `ParamSpecError`、`validate_param_specs()` 和 `ParamRegistry::try_new()`，统一校验 host/Bridge 侧会消费的参数 schema。
- validator 覆盖空 ID、重复 ID、ID/name/unit 中的 control characters、空 name、非法 default normalized、非法 float range、空/非法 choice label，以及 `read_only && automatable` 的冲突组合。
- `VestyProcessor::try_with_telemetry_registry()` 和 `VestyController::try_with_telemetry_registry()` 都会先校验 `plugin.params().specs()` 与稳定 VST3 `ParamID` registry；VST3 factory 创建 processor 或 controller 时如遇无效 schema 返回 `kResultFalse`，避免加载一个参数映射不可靠或半初始化的插件实例。
- `controller_maps_bypass_and_read_only_parameter_flags` 已改用 `.as_read_only()` 合法 builder 语义；fake COM 测试 `factory_rejects_processor_and_controller_with_invalid_param_schema` 覆盖重复参数 ID 被拒绝且 output pointer 保持 null。
- `.agents/03-module-design.md` 和 `.agents/08-developer-guide.md` 已同步记录参数 schema validator 和 VST3 controller gate。

本次 2026-06-08 针对 JSBridge ready 参数 schema gate 补强:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge schema -- --nocapture
cargo test -p vesty-bridge
cargo clippy -p vesty-bridge --all-targets -- -D warnings
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' factory_rejects_controller_with_invalid_param_schema -- --nocapture
cargo clippy -p vesty-vst3 --features 'vst3-bindings wry-ui' --all-targets -- -D warnings
npm --workspace @vesty/plugin-ui test
```

新增行为:

- `BridgeRuntime::new()` / `BridgeRuntime::try_new()` 会在建立 ready snapshot store 和参数 ID map 前调用 `validate_param_specs()` 并返回 `ParamSpecError`，拒绝重复/非法参数 schema，不再通过公共 constructor panic。
- 新增 `bridge_runtime_try_new_rejects_invalid_param_schema`，覆盖 duplicate ID 和 empty ID ready payload 被 `vesty-bridge` 拒绝。
- wry/VST3 bridge endpoint 改为使用 `try_new()`；正常情况下无效 schema 已在 VST3 controller 创建前被挡住，如果未来路径绕过 controller gate，IPC 会返回 `validation_error`，不会在 WebView attach 路径 panic。
- `@vesty/plugin-ui` 的 `ready()` 新增 ready payload shape 校验，覆盖 protocol metadata、capabilities、snapshot revision 字段和 `ParamSpec` schema；畸形 payload 会以 non-retryable `validation_error` 拒绝，清空 ready promise 后允许重试。
- `.agents/12-jsbridge-design.md` 已同步说明 ready payload 参数 schema 在 Rust runtime 建表前校验，并记录 JS SDK 的运行时校验边界。

本次 2026-06-08 针对基础 MIDI event 覆盖补强:

```bash
cargo fmt --all --check
cargo test -p vesty-core event_sample_offset -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' processor_translates_automation_midi_and_transport -- --nocapture
cargo test -p vesty-core
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui'
cargo clippy -p vesty-vst3 --features 'vst3-bindings wry-ui' --all-targets -- -D warnings
```

新增行为:

- `vesty_core::Event` 新增 `PolyPressure`、`MidiCc`、`PitchBend` 和 `ChannelPressure`，并提供 `Event::sample_offset()`，避免新增事件时遗漏排序/automation helper 的 offset 处理。
- `vesty-vst3` 的 input event collector 现在会转换 VST3 `kPolyPressureEvent` 和 `kLegacyMIDICCOutEvent`；legacy `kPitchBend` 会合成 14-bit bipolar `PitchBend`，legacy `kAfterTouch` 会映射为 `ChannelPressure`，其它 controller 映射为 normalized `MidiCc`。
- fake processor 测试 `processor_translates_automation_midi_and_transport` 已扩展为同一 block 内混合 automation、NoteOn/Off、PolyPressure、CC、PitchBend 和 ChannelPressure，确认 sample-order sort 和 developer kernel 可见事件序列。
- `.agents/04-vst3-adapter.md` 已同步当前 MIDI/Event shape；当时 SysEx、Note Expression 和 MIDI 2.0 均保留在后续 scope。后续补强已落地 SysEx data event translation、Note Expression value/int/text event translation、opt-in `INoteExpressionController` value metadata 和 opt-in `INoteExpressionPhysicalUIMapping` static mapping metadata；MIDI 2.0、真实 SysEx workflow 和真实 expression workflow 仍在后续/外部验证 scope。

本次 2026-06-08 针对参数 MIDI mapping / `IMidiMapping` 补强:

```bash
cargo fmt --all --check
cargo test -p vesty-params param_midi -- --nocapture
cargo test -p vesty-params validates_param_specs -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' midi -- --nocapture
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
npm --workspace @vesty/plugin-ui test
```

新增行为:

- `ParamSpec` 新增 `midi_mappings`，TypeScript/JSON wire shape 为 `midiMappings: ParamMidiMapping[]`；`vesty-ipc::export_protocol_bindings()` 会导出 `ParamMidiMapping.ts`。
- `vesty_params::midi` 暴露 VST3 MIDI controller 常量，包括 `CHANNEL_PRESSURE`、`PITCH_BEND`、`PROGRAM_CHANGE` 和 system controller `132..=140`，并以 `MAX_MIDI_CONTROLLER = 140` 作为 schema 上限。
- `ParamSpec`、`FloatParam`、`BoolParam` 和 `ChoiceParam` 新增 `.with_midi_mapping()`、`.with_midi_cc()` 和 `.with_channel_midi_cc()` builder；`validate_param_specs()` 会拒绝非法 controller、非法 channel 和同一参数内重复 mapping。
- `VestyController` 现在实现 `IMidiMapping`，host 查询 main bus `0` 时会按 `midi_mappings` 返回第一个 opt-in、automatable、非 read-only 参数的 VST3 `ParamID`；channel-specific mapping 只匹配对应 MIDI channel，read-only 参数不会被返回。
- fake COM 测试 `controller_exposes_opt_in_midi_mapping` 覆盖全 channel CC、指定 channel CC、pitch bend、read-only mapping、非 main bus、负 controller/channel 和 null out pointer。
- `@vesty/plugin-ui` 的 ready payload 校验现在要求每个 `ParamSpec` 都包含 `midiMappings`，并校验 controller/channel 范围和同一参数内重复 mapping；无效 payload 会以 `validation_error` 拒绝并允许 retry。
- `.agents/04-vst3-adapter.md`、`.agents/08-developer-guide.md` 和 `.agents/12-jsbridge-design.md` 已同步记录参数 MIDI mapping、wire schema 和 JS ready 校验。

本次 2026-06-08 针对 VST3 stable `ParamID` 补强:

```bash
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' -- --nocapture
```

新增行为:

- `vesty-vst3` 新增 `stable_vst3_param_id()` 和内部 `Vst3ParamIds` registry，从字符串参数 ID 派生稳定正数 31-bit VST3 `ParamID`。
- controller 的 `getParameterInfo()`、format/parse、normalized get/set、begin/perform/end edit 和 `IMidiMapping` 都使用稳定 host `ParamID`；不再把参数数组下标暴露给 host。
- processor 的 `IParameterChanges` 收集会先用 host `ParamID` 查回本地参数 index，再生成 `ParamHandle` 和 `Event::Param`；`Event::Param.id_hash` 现在携带稳定 host `ParamID`。
- wry bridge 的 UI param gesture relay 会从字符串 ID 查到稳定 host `ParamID` 后调用 `IComponentHandler`，避免 UI path 和 host path 使用不同 ID。
- 新增 `stable_vst3_param_ids_are_derived_from_string_ids` 和 `stable_vst3_param_id_registry_rejects_collisions`，并更新 fake COM tests，覆盖 state roundtrip、automation、MIDI mapping、UI gesture relay 和 latency restart notification 均走稳定 `ParamID`。
- `.agents/04-vst3-adapter.md` 和 `.agents/08-developer-guide.md` 已同步参数 ID 规则。

本次 2026-06-08 针对可审计参数 manifest 侧车补强:

```bash
cargo test -p vesty-params stable_vst3_param_ids -- --nocapture
cargo test -p vesty-build parameter_manifest -- --nocapture
cargo test -p vesty-build
cargo test -p vesty-cli param_manifest -- --nocapture
cargo test -p vesty-cli create_project_generates_parameter_sidecar_files -- --nocapture
cargo test -p vesty-cli
cargo run -p vesty-cli -- param-manifest --specs examples/gain/params.specs.json --out examples/gain/vesty-parameters.json --check
cargo run -p vesty-cli -- param-manifest --specs examples/midi-synth/params.specs.json --out examples/midi-synth/vesty-parameters.json --check
cargo run -p vesty-cli -- param-manifest --specs examples/web-ui-param-demo/params.specs.json --out examples/web-ui-param-demo/vesty-parameters.json --check
cargo clippy -p vesty-build -p vesty-cli --all-targets -- -D warnings
cargo fmt --all --check
cargo test -p vesty-cli validate_report -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' stable_vst3_param_ids -- --nocapture
```

新增行为:

- `vesty-params` 现在公开 `VST3_PARAM_ID_ALGORITHM = "vesty.vst3.param.fnv1a31-positive.v2"` 和 `stable_vst3_param_id()`；`vesty-vst3` 的 controller/processor ID registry 改为复用该 shared helper，不再保留本地副本。v1 的 full 32-bit FNV-1a ID 会让 `mix`、`level` 等高位 ID 在 Steinberg validator 中被解释为负数并触发 `Invalid Id!!!`，因此 v2 保留 FNV-1a namespace 后把结果 mask 到正数 31-bit 空间，`0` 回退为 `1`。
- `vesty-build` 新增 `ParameterManifest` / `ParameterManifestEntry` / `ParameterSpecDocument` / `parameter_manifest_from_specs_json()` / `read_parameter_manifest()` / `validate_parameter_manifest()`，可从显式 `ParamSpec` JSON 生成包含字符串 ID、稳定 VST3 `ParamID` 和完整 `ParamSpec` metadata 的 sidecar；当前 sidecar 明确保留 `ParamFlags.programChange`。
- `vesty-cli` 新增 `param-manifest` / `parameter-manifest` 命令；支持 `--specs <params.specs.json>`、`--out <vesty-parameters.json>`、`--check` 和 `--format text|json`，不会加载已编译插件二进制。
- `vesty new` 现在默认生成 `params.specs.json`、`vesty-parameters.json`，并在 `vesty.toml` 写入 `parameter_manifest = "vesty-parameters.json"`；三个 example 也已接入同名 sidecar。新生成的 `params.specs.json` 和 `vesty-parameters.json` 都显式写入 `flags.programChange`，旧的缺省输入仍可反序列化为 `false`。
- `[package].parameter_manifest` 作为可选严格 schema 字段接入；配置后 `package_vst3()` 会读取该 JSON，校验算法名、参数 schema、`id == spec.id`、`vst3ParamId` 重新计算值和 duplicate host ID，再写入 `Contents/Resources/parameters.manifest.json`。
- `validate_vst3_bundle()` 会在 bundle 包含 `parameters.manifest.json` 时重新校验该 sidecar；`vesty validate --format json` 的 `static_check` 现在带可选 `parameter_manifest` 字段，并通过 `serde(default)` 兼容旧 validate report。
- 当前仍不会从已编译 `.dylib` / `.dll` / `.so` 自动 introspect 参数列表；生成该 sidecar 需要显式 JSON 输入，以避免 CLI 加载执行任意插件代码。

## 依赖基线

2026-06-08 通过 crates.io sparse index 和 Steinberg `vst3sdk` tag 重新核对，关键依赖仍匹配当前计划基线:

- `wry 0.55.1`
- `vst3 0.3.0`
- Steinberg VST3 SDK `v3.8.0_build_66`
- `raw-window-handle 0.6.2`
- `rtrb 0.3.4`
- `serde 1.0.228`
- `serde_json 1.0.150`
- `ts-rs 12.0.1`
- `clap 4.6.1`
- `notify 9.0.0-rc.4` 仍只作为 CLI/dev watcher 候选，不进入 runtime/audio path。
- workspace 中的 `toml 1.1.2`、`sha2 0.11.0` 也与 crates.io 当前搜索结果一致。`toml` crate 在 registry 中展示为 `1.1.2+spec-1.1.0`；Cargo manifest 保持推荐 requirement `1.1.2`，latest gate 单独比对 registry 展示版本。
- `syn 2.0.117`、`quote 1.0.45`、`proc-macro2 1.0.106`、`proc-macro-crate 3.5.0` 用于 `vesty-macros`，2026-06-08 已通过 crates.io 搜索核对。

2026-06-09 通过 `cargo search`、npm workspace outdated 和本地 baseline gate 重新核对:

- `wry 0.55.1`、`vst3 0.3.0`、`raw-window-handle 0.6.2`、`rtrb 0.3.4`、`serde 1.0.228`、`serde_json 1.0.150`、`ts-rs 12.0.1`、`clap 4.6.1` 仍为 crates.io 当前返回版本。
- workspace baseline 同步覆盖 `schemars 1.2.1`、`toml 1.1.2`、`sha2 0.11.0`、`tempfile 3.27.0`、`thiserror 2.0.18`。
- `@vesty/plugin-ui`、`@vesty/react`、`@vesty/vue`、`@vesty/svelte` 的 TypeScript devDependency 已升级到 `^6.0.3`，`package-lock.json` installed `typescript` 为 `6.0.3`。
- 新增 `vesty dependency-baseline`，离线校验当前 workspace 的 Cargo baseline、VST3 SDK/binding baseline、JS TypeScript range、React/Vue/Svelte adapter devDependency range 和 lockfile installed version；CI 新增 `dependency-baseline` job 并上传 `vesty-dependency-baseline` artifact。该命令防止仓库版本漂移，但不替代 release 前联网 registry/npm/Steinberg 最新性复核。

本次 2026-06-09 针对 final protocol snapshot release gate 硬化:

```bash
cargo fmt --all --check
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
cargo test -p vesty-cli protocol_release_check -- --nocapture
cargo test -p vesty-cli release_check_requires_release_artifacts_when_requested -- --nocapture
cargo test -p vesty-cli
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo run -p vesty-cli -- publish-plan --out target/publish-plan-smoke.json
cargo run -p vesty-cli -- publish-plan --check --out target/publish-plan-smoke.json
cargo run -p vesty-cli -- npm-pack --out target/npm-pack-smoke.json
cargo run -p vesty-cli -- npm-pack --check --out target/npm-pack-smoke.json
cargo run -p vesty-cli -- release-check --strict --require-release-artifacts --skip-protocol --format json \
  --validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json \
  --validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json \
  --validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json \
  --static-validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json \
  --report target/vesty-paramid-v2-smoke/release-check-strict-skip-protocol.json
cargo run -p vesty-cli -- release-check --strict --require-release-artifacts --format json \
  --protocol-snapshot target/vesty-protocol \
  --publish-plan-report target/publish-plan-smoke.json \
  --npm-pack-report target/npm-pack-smoke.json \
  --validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.validate.json \
  --validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.validate.json \
  --validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.validate.json \
  --static-validate-report target/vesty-paramid-v2-smoke/gain/VestyGain.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/web/VestyWebUIDemo.static.json \
  --static-validate-report target/vesty-paramid-v2-smoke/midi/VestyMIDISynth.static.json \
  --report target/vesty-paramid-v2-smoke/release-check-strict-local-artifacts.json
```

新增行为:

- `protocol_release_check(protocol_snapshot, skip_protocol, required)` 在 `required = true` 且传入 `--skip-protocol` 时返回 failed，并提示运行 `vesty export-types --out <snapshot> --check`；最终 `--strict --require-release-artifacts` gate 不能再跳过 protocol snapshot。
- per-OS CI `release-check-Linux/macOS/Windows.json` snapshot 仍允许使用 `--skip-protocol`，因为 protocol artifact 由单独 job 生成；这些 snapshot 只证明各 runner 的本地 invariant 没坏，不能替代最终 consolidated release gate。
- `release-check --write-evidence-template` 生成的 README 已同步最终命令: 先运行 `vesty export-types --out target/vesty-protocol --check`，再给 final gate 传 `--protocol-snapshot target/vesty-protocol`，并明确禁止 `--skip-protocol`。
- 带本地 `target/vesty-protocol`、`target/publish-plan-smoke.json`、`target/npm-pack-smoke.json` 和 macOS 三示例 validator/static reports 的 strict local-artifacts report 中，protocol snapshot、crate publish plan、npm package dry-run、VST3 validate/static reports 可为 `ok`；最终 `Vesty example validator coverage` 仍应因缺少 Linux/Windows 三示例 validator reports 而失败，整体也仍应因真实 GitHub Actions artifacts、Linux/Windows static matrix、DAW smoke、签名和 notarization evidence 缺失而失败。

本次 2026-06-09 针对三示例/三平台 Steinberg validator coverage gate 收紧:

- `example_validate_coverage_release_check(paths, require_all_platforms)` 在普通本地检查中仍允许部分示例/platform validator coverage，用于开发者本机 smoke；在 `--require-release-artifacts` 下改为要求 `VestyGain.vst3`、`VestyWebUIDemo.vst3`、`VestyMIDISynth.vst3` 乘以 `linux-x64`、`macos`、`windows-x64` 的完整 3x3 Steinberg validator-passed matrix。
- 最终 gate 缺失项现在按 `bundle@platform` 报告，例如 `VestyGain.vst3@linux-x64`、`VestyGain.vst3@windows-x64`；本机 macOS 三示例 validator reports 只能让非最终本地 coverage 为 `ok`，不能再满足最终 release validator coverage。
- release evidence README、`.agents/07-build-packaging.md`、`.agents/08-developer-guide.md`、`.agents/09-crash-safety-and-testing.md` 和 `.agents/14-completion-audit.md` 已同步 3x3 validator matrix 口径；最终 `--require-release-artifacts` 同时要求 protocol snapshot，不允许 `--skip-protocol`。

本次 2026-06-09 针对 platform smoke release evidence gate:

```bash
cargo fmt --all --check
cargo test -p vesty-cli platform_smoke -- --nocapture
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
cargo test -p vesty-cli release_check -- --nocapture
cargo test -p vesty-cli
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

新增行为:

- `vesty platform-smoke` 可生成/检查 macOS、Windows x64 和 Linux X11 的 system WebView/VST3 editor smoke evidence；report 必须覆盖 system WebView、Steinberg validator、示例插件扫描、WebView attach/resize、asset protocol、JSBridge roundtrip 和非零 meter stream。
- `vesty platform-smoke --write-report` 会从显式 evidence marker 写入规范 `macos.json` / `windows-x64.json` / `linux-x11.json`，并在写入前复用同一套 validator；pending、false、`replace with real ...` 占位值、Linux Wayland 和 zero meter evidence 都会被拒绝，避免手工改模板时把占位内容误当成真实 smoke。
- `vesty daw-matrix --write-report` 会从显式 DAW smoke marker 写入 host evidence 目录并立即验证；该命令只规范化真实外部 smoke 后的记录方式，不会生成 Cubase/Bitwig/Ableton/Studio One 的通过证据。
- `vesty release-check` 新增 `--platform-smoke-dir <dir>`，在普通本地检查中允许局部平台覆盖并给出 missing hint；在 `--require-release-artifacts` 下要求 macOS、Windows x64、Linux X11 三份真实 report，Linux Wayland report 会被拒绝。
- `--release-evidence-dir` 会自动发现 `platform-smoke/`，但只有存在非 pending report 或无法解析 JSON 时才会启用；单独运行 `vesty platform-smoke --write-template --dir target/release-evidence/platform-smoke` 生成的 pending JSON 模板不会被当作 pass evidence，也不会把 optional release-check 从 `skipped` 变成 `failed`。
- `release-check --write-evidence-template` 创建 `platform-smoke/README.md` 作为采集说明；最终 strict local-artifacts report 现在应包含 `platform smoke artifacts: failed | required evidence missing`，直到真实 macOS/Windows/Linux X11 evidence 被补齐。
- `.agents/07-build-packaging.md`、`.agents/08-developer-guide.md`、`.agents/09-crash-safety-and-testing.md` 和 `.agents/14-completion-audit.md` 已同步 platform smoke gate；Linux Wayland 继续标记为 experimental。

本次 2026-06-09 针对 DAW/npm release evidence 写入 helper:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli daw_matrix -- --nocapture
cargo test -p vesty-cli npm_pack -- --nocapture
cargo test -p vesty-cli
cargo run -p vesty-cli -- export-types --out target/vesty-protocol
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo run -p vesty-cli -- daw-matrix --evidence-root target/daw-write-report-smoke --write-report --host bitwig --platform "macOS arm64 / Bitwig smoke" --scan "VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3" --load "load=true" --ui "ui=true" --ui-host-param "ui_host_param=true" --meter-stream "meter_flush sent=3" --automation "automation=true" --buffer-sample-rate-change "buffer_sample_rate_change=true" --save-restore "save_restore=true" --offline-render "offline_render=true" --format json
cargo run -p vesty-cli -- daw-matrix --evidence-root target/daw-write-report-reaper-smoke --write-report --host reaper --platform "macOS arm64 / REAPER marker smoke" --scan "VestyGain.vst3 VestyWebUIDemo.vst3 VestyMIDISynth.vst3" --load "load=true" --ui "ui=true" --ui-host-param "ui_host_param=true" --meter-stream "meter_flush sent=3" --automation "automation=true" --buffer-sample-rate-change "buffer_sample_rate_change=true" --save-restore "save_restore=true" --offline-render "offline_render=true" --format json
cargo run -p vesty-cli -- npm-pack --out target/npm-pack-cli-smoke/npm-pack.json
cargo run -p vesty-cli -- npm-pack --check --out target/npm-pack-cli-smoke/npm-pack.json
cargo run -p vesty-cli -- release-check --format json --npm-pack-report target/npm-pack-cli-smoke/npm-pack.json --protocol-snapshot target/vesty-protocol
cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence --protocol-snapshot target/vesty-protocol > target/release-check-strict-post-daw-write-report.json
```

新增行为:

- `vesty daw-matrix --write-report --host <host>` 写入真实 smoke 后的显式 marker，并在写入前拒绝 pending/false/占位值、zero meter 和无法被 matrix parser 识别的模糊 marker，避免留下半套无效 evidence。写入后仍会复用 DAW matrix parser 验证该 host 行完整；REAPER 保留专用 cache/render/param-watch evidence，同时也接受通用 marker fallback。
- `vesty npm-pack --out <path>` 会运行 npm workspace dry-run pack，写入规范 JSON，并立即复用 release-check 的 JS package boundary validator；`vesty npm-pack --check --out <path>` 可复验已有 artifact。CI `js sdk` job 现在通过该 helper 生成并复验 `vesty-npm-pack` artifact。
- `release-check --write-evidence-template` 的 README 和 `.agents` 文档已改为推荐 `vesty npm-pack --out ...`，而不是手工重定向裸 `npm pack`。
- 当前 strict release gate 仍按预期失败，失败项包括真实 DAW matrix、CI run URL/doctor/release-check artifacts、macOS/Windows/Linux X11 platform smoke、三示例/三平台 validator/static validate 覆盖、签名和 notarization evidence，证明新 helper 没有把外部证据缺口误放行。

本次 2026-06-09 针对本地 release evidence collect helper:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli collect_local_release_evidence -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli
rm -rf target/release-evidence-local-smoke target/vesty-protocol-local-smoke
cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke
cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke --vst3-sdk-dir /path/to/VST_SDK --vst3-sdk-bindings-module target/vst3-sdk/generated.rs
cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke > target/release-check-strict-post-collect-local.json
```

新增行为:

- 新增 `vesty release-evidence collect-local`，默认会创建 release evidence 模板、导出并检查 protocol snapshot、生成 `publish-plan/publish-plan.json`、生成 `npm-pack/npm-pack.json`，并写入 `local-collect-report.json`。
- `collect-local` 可显式传入 `--vst3-sdk-dir <official-vst3sdk>` 生成并复验 `vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json`、`vst3-sdk/generated-bindings-surface.json`、metadata-only `vst3-sdk/generated.rs` scaffold、ABI seed `vst3-sdk/generated-abi-seed.rs`、ABI layout `vst3-sdk/generated-abi.rs` 和 interface skeleton `vst3-sdk/generated-interface-skeleton.rs`；`--vst3-sdk-bindings-module <path>` 用于锁定 plan/scaffold 里的 `.rs` output module path。该 plan/surface 仍必须保持 `bindingsGenerated = false`，scaffold 仍必须保持 `BINDINGS_GENERATED = false`，ABI seed、ABI layout 和 interface skeleton 仍必须保持 `FULL_COM_BINDINGS_GENERATED = false`，它们只是 generated-headers backend 的 readiness/surface/audit evidence。
- `collect-local` 的输出明确说明它只采集本地真实可运行命令的 evidence 和显式请求的 VST3 SDK 审计文件；DAW matrix、macOS/Windows/Linux X11 platform smoke、Steinberg validator-passed reports、真实 GitHub Actions provenance、codesign/signtool 和 notarization/stapler evidence 仍必须由外部真实流程采集。
- `.github/workflows/ci.yml` 的 `release evidence snapshot` job 现在先运行 `vesty release-evidence collect-local --no-protocol --no-publish-plan --no-npm-pack` 初始化模板和 `local-collect-report.json`，再下载 protocol/publish-plan/npm-pack/package/doctor/release-check artifacts；默认不传 `--vst3-sdk-dir`，因此不会在聚合 job 中隐式生成 SDK evidence。这避免聚合 job 覆盖已下载 artifact、重复执行 npm pack 或误把本机 SDK checkout 当成下载 artifact。
- `release-check --release-evidence-dir target/release-evidence-local-smoke --require-release-artifacts` 会自动采纳 `collect-local` 生成的 protocol、crate publish plan 和 npm pack report，这些项为 `ok`；整体仍因真实 DAW、CI、平台、validator 3x3、签名和 notarization evidence 缺失而失败，符合 release gate 边界。

本次 2026-06-09 针对 signing/notarization release evidence collection helper:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli signing_evidence -- --nocapture
cargo test -p vesty-cli signing_verification -- --nocapture
cargo test -p vesty-cli notarization -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli
rm -rf target/release-evidence-local-smoke target/vesty-protocol-local-smoke
cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke
cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke > target/release-check-strict-post-collect-local.json
```

新增行为:

- 新增 `vesty release-evidence collect-signing <bundle.vst3> --platform macos|windows-x64 --dir <dir>`。该 helper 会运行真实 `codesign --verify --deep --strict --verbose=2` 或 `signtool verify /pa /v`，捕获 stdout/stderr，并且只有输出被签名 evidence parser 接受后才写入 `signing-macos.log` 或 `signing-windows.log`。
- 新增 `vesty release-evidence collect-notarization --notary-log <log> --stapler-log <log> --dir <dir>`。该 helper 会合并真实 notarytool/stapler 日志，并且只有同时证明 accepted notarytool result 与 stapler success 后才写入 `notary.log`。
- `collect-signing` 明确拒绝 Linux bundle signing，因为 Linux 首版仍是 release-channel policy，需要发行包/渠道签名证据，不能由 VST3 bundle 内部签名 helper 伪造。
- release evidence 模板 README、README.md 和 `.agents/07-build-packaging.md` 已同步这些 helper；文档仍明确 `collect-local` 不生成签名/公证 evidence，真实 DAW、平台、validator、CI、签名和 notarization evidence 仍需外部流程采集。
- 当前 strict release gate 仍按预期失败，失败项仍包括真实 DAW matrix、CI run/artifacts、平台 smoke、三示例/三平台 validator/static 覆盖、签名和 notarization evidence；这些 helper 只规范化真实日志，不放宽 release gate。

本次 2026-06-09 针对 CI artifact release evidence import helper:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli import_ci -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli
rm -rf target/release-evidence-local-smoke target/vesty-protocol-local-smoke target/release-check-strict-post-collect-local.json
cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke
cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke > target/release-check-strict-post-collect-local.json
```

新增行为:

- 新增 `vesty release-evidence import-ci --source <downloaded-artifacts-dir> --dir <release-evidence-dir>`，用于把已经下载到本地的 GitHub Actions artifacts 规范化到 release evidence 目录。它按内容识别并验证 protocol snapshot、doctor artifacts、per-OS release-check snapshots、publish-plan、npm-pack、validate/static validate reports、platform smoke、签名日志、已签名 macOS `.vst3` bundle 和 accepted + stapled notarization logs。
- `import-ci` 默认保留已有文件，只有显式 `--overwrite` 才会替换；每次导入都会写入 `import-ci-report.json`，列出 imported/skipped/failed artifacts。该 report 是审计元数据，不是 pass evidence。
- `import-ci` 现在拒绝 `--source` 和 `--dir` 相同或互相嵌套，避免下载 artifact staging 目录扫描/覆盖自身规范化输出；`--source` 必须是已存在的真实目录且不能是 symlink，既有 `--dir` 也不能是 symlink，缺失的 output dir 只会通过真实父目录创建，symlinked output parent 会被拒绝；如果同时传入 `--ci-run-url` 和 `--ci-run-url-file`，两者都必须有效且匹配同一个 GitHub repo/run id，同一 run 的不同 attempt 允许通过；`release-check --release-evidence-dir` 对显式 `--ci-run-url` 和目录内 `ci-run-url.txt` 执行同样的一致性校验；`ci-run-url.txt` symlink 会被拒绝；文件 overwrite 删除既有目标时也会把目标 symlink 当成 symlink 本身 unlink，不会跟随删除外部目录；导入写入/复制前还会拒绝 symlinked destination parent，避免标准 evidence 子目录被替换到 release evidence bundle 外部。
- CI `release-evidence` job 现在把同一 workflow 的 artifacts 下载到 `target/downloaded-artifacts` staging 目录，再调用 `vesty release-evidence import-ci` 内容验证并规范化到 `target/release-evidence`；随后 consolidated `release-check` 只读取规范化后的 evidence 目录。
- `import-ci` 不合成 GitHub Actions、DAW、platform smoke、validator-passed、签名或 notarization 通过证据；缺少真实外部 artifact 时，最终 `--strict --require-release-artifacts` gate 仍必须失败。

本次 2026-06-09 针对 VST3 SDK header manifest、generated-bindings plan 与 generated-bindings surface release evidence / CI 聚合并通过:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli import_ci -- --nocapture
```

新增行为:

- `vesty release-check` 新增 `--vst3-sdk-manifest <path>`，会严格验证 `vesty vst3-sdk manifest` 生成的 SDK header input manifest，包括 manifest version/generator、Steinberg SDK baseline、upstream `vst3` crate baseline、required header set、duplicates/unexpected headers、`missingHeaders` complement、`complete`、非零 size 和 lowercase SHA-256 shape。
- `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --bindings-module target/vst3-sdk/generated.rs --out <path>` 可生成 generated-bindings readiness report；`--check` 会复验 SDK headers、output module path、active backend baseline、reserved binding emitter check 和 next steps 是否漂移。
- `vesty release-check` 新增 `--vst3-sdk-binding-plan <path>`，会严格验证 `vesty vst3-sdk binding-plan` 生成的 JSON: plan version/generator、`bindingsGenerated = false`、`status = ready-for-binding-generator`、无 blockers、SDK/crate baseline、active backend、`.rs` module path、embedded header manifest 完整性、reserved binding emitter check 和 next steps。blocked、incomplete 或声称已经 generated 的 plan 都会失败。
- `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out <path>` 可生成 generated-bindings symbol surface report；`--check` 会复验 SDK headers、required symbol/header surface、active backend baseline 和 audit notes 是否漂移。
- `vesty release-check` 新增 `--vst3-sdk-binding-surface <path>`，会严格验证 `vesty vst3-sdk binding-surface` 生成的 JSON: surface version/generator、`bindingsGenerated = false`、`status = ready-for-binding-emitter`、无 blockers、SDK/crate baseline、active backend、embedded header manifest 完整性、required header set、required symbols、symbol/header mapping 和 audit notes。blocked、incomplete 或声称已经 generated 的 surface 都会失败。
- SDK header manifest 缺失时保持 `skipped`，即使传入 `--require-release-artifacts` 也不会变成 required；这是因为当前 active backend 仍是 upstream `vst3` crate。只要显式传入或被 evidence 目录自动发现，invalid/incomplete manifest 就会让 release-check failed。
- SDK generated-bindings plan 缺失时同样保持 `skipped`，即使传入 `--require-release-artifacts` 也不会变成 required；只要显式传入或被 evidence 目录自动发现，无效 plan 就会让 release-check failed。
- SDK generated-bindings surface 缺失时同样保持 `skipped`，即使传入 `--require-release-artifacts` 也不会变成 required；只要显式传入或被 evidence 目录自动发现，无效 surface 就会让 release-check failed。
- `--release-evidence-dir <dir>` 现在会自动发现 `vst3-sdk/vst3-sdk-headers.json` / `vst3-sdk/generated-bindings-plan.json` / `vst3-sdk/generated-bindings-surface.json`，也兼容根目录 `vst3-sdk-headers.json` / `generated-bindings-plan.json` / `generated-bindings-surface.json`。`vst3-sdk/README.md` 模板不会被当作 evidence，也不会把 optional check 从 `skipped` 变成 `failed`。
- `release-check --write-evidence-template` 现在创建 `vst3-sdk/README.md`，说明如何用 `vesty vst3-sdk manifest`、`vesty vst3-sdk binding-plan` 和 `vesty vst3-sdk binding-surface` 生成/复验这些可选审计证据。
- `vesty release-evidence import-ci` 现在按内容识别 `SdkHeaderInputManifest`、`GeneratedBindingsPlan` 和 `GeneratedBindingsSurface`，只有完整有效 manifest / ready plan / ready surface 才会分别复制到 `vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json` 与 `vst3-sdk/generated-bindings-surface.json`；invalid/incomplete/blocked artifact 会记录为 failed 且不复制。
- `.github/workflows/ci.yml` 的可选 `vst3-sdk` job 在 `VESTY_VST3_SDK_DIR` 存在时生成并 `--check` 复验 `target/vst3-sdk/vst3-sdk-headers.json`、`target/vst3-sdk/generated-bindings-plan.json`、`target/vst3-sdk/generated-bindings-surface.json`、`target/vst3-sdk/generated.rs` metadata scaffold、`target/vst3-sdk/generated-abi-seed.rs` ABI seed、`target/vst3-sdk/generated-abi.rs` ABI layout 和 `target/vst3-sdk/generated-interface-skeleton.rs` interface skeleton；未设置时上传 README skip note。`release-evidence` job 会下载 `vesty-vst3-sdk-headers` artifact 并交给 `import-ci` 内容验证 manifest/plan/surface/scaffold/ABI seed/ABI layout/interface skeleton。
- 该功能只是把 official SDK header 输入、generated-bindings readiness plan、symbol surface、metadata-only generated module scaffold、ABI aliases/constants seed、少量 foundational ABI layout 和 interface/vtable skeleton/method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata 纳入 CI drift 检查；它不表示完整 SDK 3.8 generated bindings 已经生成，也不替代 DAW、validator、platform smoke、签名或 notarization 外部证据。

本次 2026-06-09 针对 VST3 SDK generated-bindings metadata scaffold / ABI seed / ABI layout / interface skeleton:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo check -p vesty-cli -j1
```

新增行为:

- `vesty-vst3-sys::generated_bindings_scaffold(root, bindings_module)` 会复用 `GeneratedBindingsPlan`，只有在 SDK header inputs 完整且 output module path 是 `.rs` 时才生成 deterministic Rust module。
- `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated.rs` 会写入 metadata-only `generated.rs`，并支持 `--check` 逐字节复验 drift。该 module 包含 header manifest metadata、baseline、active backend 和 `BINDINGS_GENERATED = false`；它不包含 Steinberg VST3 COM/API bindings。
- `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-interface-skeleton.rs` 会写入 interface/vtable skeleton module，并支持 `--check` 逐字节复验 drift。该 module 包含 header manifest metadata、baseline、active backend、基础 ABI aliases/constants、interface placeholder/vtable skeleton、per-interface method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata 和 `INTERFACE_SKELETON_GENERATED = true`，同时保持 `BINDINGS_GENERATED = false`、`FULL_COM_BINDINGS_GENERATED = false` 与 `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`；slot 只表示接口内审计顺序，signature 只表示方法签名意图，vtable slot seed、callback type alias seed、field layout seed、offset fingerprint、IID records、queryInterface planned dispatch entries、`QUERY_INTERFACE_IID_LOOKUP_SCOPE` 与纯查找 helper、`COM_OBJECT_INTERFACES` records、`COM_OBJECT_IDENTITY_PLANS` records、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` records、`FACTORY_EXPORT_PLAN` / `FACTORY_CLASS_PLANS` records、`MODULE_EXPORT_PLANS` records、`BINARY_EXPORT_SYMBOL_PLANS` / `BINARY_EXPORT_INSPECTION_TOOL_PLANS` records、binary export required-symbol helpers 和 `binary_export_inspection_tools()` 只固定 future emitter 的 local slot、field name、callback type alias name、签名意图、repr(C) callback field layout、Rust 侧 `offset_of!` 指纹、upstream IID words、future interface dispatch lookup seed、当前 Vesty adapter object-to-interface exposure plan、object FUnknown identity plan、per-object dispatch plan、current factory/class export plan、current `export_vst3!` platform entry symbol plan 和 future binary inspection expected symbol/tool spelling + required-symbol 判定，它不包含 callable `queryInterface` glue、generated factory exports、generated module exports、binary inspection tooling、factory glue、Steinberg method implementations 或完整 VST3 COM/API bindings。
- `.github/workflows/ci.yml` 的 optional `vst3-sdk` job 现在在 manifest、binding-plan 和 binding-surface 之外，也生成并复验 metadata scaffold、ABI seed、ABI layout 和 interface skeleton；未配置 SDK checkout 时仍上传 skip note，不让 CI/release gate 强依赖本机 SDK。
- `release-evidence import-ci` 现在会把有效 generated-bindings surface 规范化到 `vst3-sdk/generated-bindings-surface.json`，把有效 metadata scaffold 规范化到 `vst3-sdk/generated.rs`，把有效 ABI seed 规范化到 `vst3-sdk/generated-abi-seed.rs`，把有效 ABI layout 规范化到 `vst3-sdk/generated-abi.rs`，并把有效 interface skeleton 规范化到 `vst3-sdk/generated-interface-skeleton.rs`；surface、scaffold、ABI seed、ABI layout 和 interface skeleton/method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata 现在都会在显式传入或被 `--release-evidence-dir` 标准路径发现时作为 optional strict audit evidence 进入 `release-check`；这些文件仍只证明 CI drift/audit metadata 有效，不证明完整 SDK 3.8 bindings、callable COM glue 或 final release readiness。

本次 2026-06-10 针对 VST3 SDK generated-header audit 的 Unit/Program 与 Note Expression surface 补强:

```bash
cargo fmt --all --check
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo test -p vesty-cli import_ci -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test --workspace -j1
npm test
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --check --strict
cargo run -p vesty-cli -- release-check --format json --strict --report target/release-check-generated-sdk-interface-iid-query-plan-seed.json
```

新增行为:

- `REQUIRED_GENERATED_HEADER_INPUTS` 现在显式包含 `pluginterfaces/vst/ivstunits.h` 和 `pluginterfaces/vst/ivstnoteexpression.h`，让后续 generated-header backend 的 input manifest 覆盖 program/unit 与 Note Expression controller headers。
- `GeneratedBindingsSurface` 的 required symbol surface 现在覆盖 `IUnitInfo`、`IProgramListData`、`UnitInfo`、`ProgramListInfo`、`INoteExpressionController`、`INoteExpressionPhysicalUIMapping`、`NoteExpressionTypeInfo` 和 `PhysicalUIMap`；surface 仍必须保持 `bindingsGenerated = false`，并只做 identifier-token 审计。
- `generated-abi.rs` 的 foundational ABI layout audit 现在也固定 program/unit 与 Note Expression 基础数据结构: `ProgramListInfo`、`UnitInfo`、`NoteExpressionValueDescription`、`NoteExpressionTypeInfo`、`PhysicalUIMap` 和 `PhysicalUIMapList`，并新增 `String128`、`UnitID`、`ProgramListID`、`NoteExpressionTypeID`、`NoteExpressionValue`、`PhysicalUITypeID` aliases 与 root/no-program-list constants；该 module 还输出 `ABI_LAYOUT_RECORDS` size/alignment 指纹和 `ABI_FIELD_OFFSETS` 关键字段 offset 指纹，CLI/import validator 会拒绝缺少这些指纹的 ABI layout artifact。
- `generated-interface-skeleton.rs` 现在额外输出 `InterfaceMethod` metadata、global `INTERFACE_METHODS` 以及 per-interface method arrays，例如 realtime `IAudioProcessor::process`、`IUnitInfo::getProgramListInfo`、`IProgramListData::getProgramData`、`INoteExpressionController::getNoteExpressionInfo` 和 `INoteExpressionPhysicalUIMapping::getPhysicalUIMapping`。
- `vesty-cli` 的 interface skeleton validator 和 `import-ci` 路径会要求这些 method-surface/slot-order/signature-intent metadata 存在；后续 vtable slot seed、callback type alias seed、vtable field layout seed 与 vtable field offset fingerprint 补强还会要求 `InterfaceVTableSlot` / `INTERFACE_VTABLE_SLOT_SCOPE`、`InterfaceCallbackType` / `INTERFACE_CALLBACK_TYPE_SCOPE`、`INTERFACE_VTABLE_FIELD_SCOPE` 和 `INTERFACE_VTABLE_FIELD_OFFSET_SCOPE` 存在。2026-06-13 之后，validator 还要求 `global_slot` 和 `INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE` 存在，用来固定包含 FUnknown base slots 的 COM vtable absolute slot seed。但它们仍不是 callable method implementations，不代表完整 COM bindings 或 SDK 3.8 Rust bindings 已生成。

本次 2026-06-10 针对 VST3 SDK interface skeleton slot/signature 审计补强:

- `generated-interface-skeleton.rs` 的 `InterfaceMethod` 现在包含 `slot: usize` 和 `signature: &'static str`，并输出 `INTERFACE_METHOD_SLOT_SCOPE = "per-interface-order-audit"` 与 `INTERFACE_METHOD_SIGNATURE_SCOPE = "signature-intent-audit"` marker。
- per-interface method arrays 与 global `INTERFACE_METHODS` 都写入接口内 slot order；例如 `IAudioProcessor::process` 固定为 slot 6，`IUnitInfo::getProgramListInfo`、`IProgramListData::getProgramData`、`INoteExpressionController::getNoteExpressionInfo`、`INoteExpressionPhysicalUIMapping::getPhysicalUIMapping` 都带签名意图字符串。
- `vesty-cli` interface skeleton validator 现在要求 slot/order/signature marker 和关键方法签名存在，旧的“只有 method name/purpose/realtime” skeleton artifact 会被拒绝。
- 该补强仍是 audit metadata，不生成 callable Steinberg method implementations，不改变 `BINDINGS_GENERATED = false` / `FULL_COM_BINDINGS_GENERATED = false` 边界。

本次 2026-06-10 针对 VST3 SDK interface skeleton vtable slot seed 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `InterfaceVTableSlot`、global `INTERFACE_VTABLE_SLOTS`、per-interface `*_VTABLE_SLOTS`、`INTERFACE_VTABLE_SLOT_SCOPE = "per-interface-local-vtable-seed-audit"` 和 `INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE = "com-vtable-global-slot-seed-audit"`。
- 每条 vtable slot seed 固定 local slot、包含 FUnknown base slots 的 global slot、interface、method、field、callback type 和签名意图，例如 `IAudioProcessor::process` 固定 local slot 6 / global slot 9、field `process`、callback type `IAudioProcessorProcess`。
- `vesty-cli` interface skeleton validator 现在要求 global/per-interface vtable slot seed arrays、scope marker 和关键方法 callback type 存在，进一步拒绝只含方法名/签名但没有 future vtable seed 的 skeleton artifact。
- 该补强仍不生成 callable Steinberg COM/API method implementations，也不把 `FULL_COM_BINDINGS_GENERATED` 改为 true。
- 本地验证结果: `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings`、`cargo test -p vesty-cli vst3_sdk`、`cargo test -p vesty-cli import_ci`、`cargo test -p vesty-cli release_evidence`、`cargo test --workspace -j1` 和 `npm test` 均通过；workspace Rust 测试为 419 passed。`smoke-host --check --strict` 通过，覆盖三示例 config、参数 sidecar、Web UI assets、JSBridge trace 和 meter marker。后续 ABI layout 扩展又复验了 `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings`、`cargo test -p vesty-cli vst3_sdk`、`cargo test -p vesty-cli import_ci` 和 `cargo test -p vesty-cli release_evidence`；再后续 size/alignment/field-offset 指纹扩展复验了 `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings` 和 `cargo test -p vesty-cli vst3_sdk`。`release-check --format json --strict` 仍按预期失败，原因是 REAPER/Cubase/Bitwig/Ableton/Studio One 真实 DAW smoke、平台 smoke、validator 矩阵、CI、签名和 notarization evidence 尚未提供；protocol snapshot 与 VST3 binding baseline 为 ok。

本次 2026-06-10 针对 VST3 SDK interface skeleton callback type alias seed 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `InterfaceCallbackType`、global `INTERFACE_CALLBACK_TYPES`、per-interface `*_CALLBACK_TYPES`、`INTERFACE_CALLBACK_TYPE_SCOPE = "callback-type-alias-seed-audit"` 和 deterministic callback type aliases。
- callback type alias seed 会为未来 emitter 固定名称与签名意图，例如 `pub type IAudioProcessorProcess = unsafe extern "system" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult;`；这些 alias 使用 skeleton 内的基础 alias / opaque placeholders 保持 module 可编译。
- `vesty-cli` interface skeleton validator 现在要求 callback type alias seed marker、global/per-interface callback type arrays 和关键 callback type aliases 存在，进一步拒绝只有 vtable slot metadata、但没有 future callback alias seed 的 skeleton artifact。
- 该补强仍不把 callback alias 挂进 vtable struct，不生成 callable Steinberg COM/API method implementations，也不把 `FULL_COM_BINDINGS_GENERATED` 改为 true。
- 本地验证结果: `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings`、`cargo test -p vesty-cli vst3_sdk`、`cargo test -p vesty-cli import_ci`、`cargo test -p vesty-cli release_evidence`、`cargo test --workspace -j1` 和 `npm test` 均通过；workspace Rust 测试为 419 passed。`smoke-host --check --strict` 通过，覆盖三示例 config、参数 sidecar、Web UI assets、JSBridge trace 和 meter marker。后续 strict release-check 最新报告改为 `target/release-check-generated-sdk-vtable-field-offset-seed.json`，并仍按预期因为 REAPER/Cubase/Bitwig/Ableton/Studio One 真实 DAW smoke、三平台 smoke、validator 矩阵、CI、签名和 notarization evidence 尚未提供而失败；protocol snapshot 与 VST3 binding baseline 为 ok。

本次 2026-06-10 针对 VST3 SDK interface skeleton vtable callback field layout seed 审计补强:

- `generated-interface-skeleton.rs` 的非 `FUnknown` `*VTable` 现在包含 deterministic callback field layout，例如 `IAudioProcessorVTable { unknown, setBusArrangements, getBusArrangement, canProcessSampleSize, getLatencySamples, setupProcessing, setProcessing, process, getTailSamples }`。
- 新增 `INTERFACE_VTABLE_FIELD_COUNT` 和 `INTERFACE_VTABLE_FIELD_SCOPE = "repr-c-vtable-callback-field-layout-seed-audit"` marker；CLI/import validator 要求关键字段如 `pub process: IAudioProcessorProcess,`、`pub getProgramListInfo: IUnitInfoGetProgramListInfo,`、`pub getProgramData: IProgramListDataGetProgramData,` 和 `pub getNoteExpressionInfo: INoteExpressionControllerGetNoteExpressionInfo,` 存在。
- 该补强只固定 `repr(C)` vtable callback field layout seed；仍不生成任何 callback implementation、factory/queryInterface glue、完整 COM/API bindings 或完整 ABI 验证，也不把 `FULL_COM_BINDINGS_GENERATED` 改为 true。
- 本地验证结果: `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings`、`cargo test -p vesty-cli vst3_sdk`、`cargo test -p vesty-cli import_ci`、`cargo test -p vesty-cli release_evidence`、`cargo test --workspace -j1` 和 `npm test` 均通过；workspace Rust 测试为 419 passed。`smoke-host --check --strict` 通过，覆盖三示例 config、参数 sidecar、Web UI assets、JSBridge trace 和 meter marker。后续 strict release-check 最新报告改为 `target/release-check-generated-sdk-vtable-field-offset-seed.json`，并仍按预期因为 REAPER/Cubase/Bitwig/Ableton/Studio One 真实 DAW smoke、三平台 smoke、validator 矩阵、CI、签名和 notarization evidence 尚未提供而失败；protocol snapshot 与 VST3 binding baseline 为 ok。

本次 2026-06-10 针对 VST3 SDK interface skeleton vtable field offset fingerprint 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `InterfaceVTableFieldOffset`、global `INTERFACE_VTABLE_FIELD_OFFSETS`、per-interface `*_VTABLE_FIELD_OFFSETS`、`INTERFACE_VTABLE_FIELD_OFFSET_COUNT` 和 `INTERFACE_VTABLE_FIELD_OFFSET_SCOPE = "repr-c-vtable-callback-field-offset-fingerprint-audit"`。
- offset fingerprint 使用 Rust `std::mem::offset_of!` 固定 skeleton 侧 `repr(C)` vtable callback field offset，例如 `IAudioProcessorVTable::process`、`IUnitInfoVTable::getProgramListInfo`、`IProgramListDataVTable::getProgramData` 和 `INoteExpressionControllerVTable::getNoteExpressionInfo`。
- `vesty-cli` interface skeleton validator 和 `import-ci` 路径现在要求 field offset fingerprint marker、global/per-interface offset arrays 和关键 offset 表达式存在，进一步拒绝只有 field layout、但没有 offset 指纹的 skeleton artifact。
- 该补强仍不是完整 ABI 验证: 它只对 Vesty 生成的 Rust skeleton layout 做 deterministic fingerprint，不解析 C++ AST、不验证官方 SDK 二进制 ABI、不生成 callable factory/queryInterface glue 或 Steinberg method implementations，也不把 `FULL_COM_BINDINGS_GENERATED` 改为 true。
- 最新验证目标改为 `target/release-check-generated-sdk-interface-iid-query-plan-seed.json`；strict release-check 仍预期因为真实 DAW/platform/validator/CI/signing/notarization evidence 缺失而失败。

本次 2026-06-10 针对 VST3 SDK interface skeleton IID / queryInterface dispatch plan 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `InterfaceId`、`QueryInterfaceEntry`、global `INTERFACE_IDS`、global `QUERY_INTERFACE_ENTRIES`、`INTERFACE_ID_COUNT`、`QUERY_INTERFACE_ENTRY_COUNT`、`INTERFACE_ID_SCOPE = "upstream-vst3-interface-iid-audit"` 和 `QUERY_INTERFACE_ENTRY_SCOPE = "query-interface-dispatch-plan-audit"`。
- 每个 locked interface 都有 per-interface `*_IID` constant；IID words 来自 upstream `vst3 0.3.0` generated bindings，并通过 `iid_from_words()` 复刻 upstream `vst3::support::uid` 的 Windows / non-Windows byte ordering。
- `QueryInterfaceEntry` 只固定 future emitter 的 dispatch plan: interface name、IID const name、是否继承 `FUnknown` 和 `implementation = "planned-dispatch-entry-no-callable-glue"`；它不是 callable `queryInterface` 实现，也不生成 factory glue。
- `vesty-cli` interface skeleton validator 现在要求 `InterfaceId` / `QueryInterfaceEntry` / `ComObjectInterface` structs、scope/count constants、关键 `*_IID` constants、`INTERFACE_IDS` / `QUERY_INTERFACE_ENTRIES` / `COM_OBJECT_INTERFACES` records 和 `iid_from_words()` 存在，旧的缺 COM identity 或 exposure-plan metadata 的 skeleton artifact 会被拒绝。
- 该补强仍保持 `BINDINGS_GENERATED = false` 与 `FULL_COM_BINDINGS_GENERATED = false`；完整 SDK 3.8 generated bindings emitter、callable COM glue 和真实 ABI 覆盖验证仍是后续工作。
- 本地验证结果: `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings`、`cargo test -p vesty-cli vst3_sdk`、`cargo test -p vesty-cli import_ci`、`cargo test -p vesty-cli release_evidence`、`cargo test --workspace -j1` 和 `npm test` 均通过；workspace Rust 测试为 419 passed。`smoke-host --check --strict` 通过，覆盖三示例 config、参数 sidecar、Web UI assets、JSBridge trace 和 meter marker。最新 strict release-check 报告为 `target/release-check-generated-sdk-com-object-interface-plan-seed.json`，并仍按预期因为 REAPER/Cubase/Bitwig/Ableton/Studio One 真实 DAW smoke、三平台 smoke、validator 矩阵、CI、签名和 notarization evidence 尚未提供而失败；protocol snapshot 与 VST3 binding baseline 为 ok。

本次 2026-06-10 针对 VST3 SDK interface skeleton COM object interface exposure plan 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `ComObjectInterface`、`COM_OBJECTS`、per-object `VESTYPROCESSOR_INTERFACES` / `VESTYCONTROLLER_INTERFACES` / `VESTYPLUGVIEW_INTERFACES` / `VESTYFACTORY_INTERFACES` 等数组、global `COM_OBJECT_INTERFACES`、`COM_OBJECT_COUNT`、`COM_OBJECT_INTERFACE_COUNT` 和 `COM_OBJECT_INTERFACE_SCOPE = "vesty-com-object-interface-exposure-plan-audit"`。
- 该 exposure plan 锁定当前 `vesty-vst3` adapter 的 `Class::Interfaces` 关系: `VestyFactory -> IPluginFactory`，`VestyProcessor -> IComponent / IAudioProcessor / IProcessContextRequirements / IConnectionPoint`，`VestyController -> IEditController / IConnectionPoint / IMidiMapping / IUnitInfo / IProgramListData / INoteExpressionController / INoteExpressionPhysicalUIMapping`，`VestyPlugView -> IPlugView`，`VestyMessage -> IMessage`，`VestyAttributeList -> IAttributeList`。
- `vesty-cli` interface skeleton validator 现在要求 `ComObjectInterface` struct、scope/count constants、per-object/global arrays 和关键 object/interface records 存在；旧的只含 IID/queryInterface dispatch plan、但缺 object exposure plan 的 skeleton artifact 会被拒绝。
- 该补强仍只是 audit metadata: `COM_OBJECT_INTERFACES` 不生成 callable `queryInterface` 分派、不生成 factory glue、不替代 `vst3` crate 的当前 COM wrapper，也不表示完整 SDK 3.8 generated bindings 已完成。
- 本地验证结果: `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings`、`cargo test -p vesty-cli vst3_sdk`、`cargo test -p vesty-cli import_ci`、`cargo test -p vesty-cli release_evidence`、`cargo test --workspace -j1` 和 `npm test` 均通过；workspace Rust 测试为 419 passed。`smoke-host --check --strict` 通过。`release-check --format json --strict --report target/release-check-generated-sdk-com-object-interface-plan-seed.json` 仍按预期失败在真实 DAW/platform/validator/CI/signing/notarization evidence 缺失，host profiles、protocol snapshot 和 VST3 binding baseline 为 ok。

本次 2026-06-10 针对 VST3 SDK interface skeleton COM object identity / per-object queryInterface dispatch plan 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `ComObjectIdentityPlan`、`ComObjectQueryInterfaceDispatchEntry`、per-object `VESTYPROCESSOR_IDENTITY_PLAN` / `VESTYPROCESSOR_QUERY_INTERFACE_DISPATCH` 等常量、global `COM_OBJECT_IDENTITY_PLANS`、global `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`、`COM_OBJECT_IDENTITY_PLAN_COUNT`、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRY_COUNT`、`COM_OBJECT_IDENTITY_PLAN_SCOPE = "vesty-com-object-funknown-identity-plan-audit"` 和 `COM_OBJECT_QUERY_INTERFACE_DISPATCH_SCOPE = "vesty-com-object-query-interface-dispatch-plan-audit"`。
- identity plan 锁定每个 current Vesty COM object 的 root interface、FUnknown identity policy、success addRef policy、unknown IID fallback `kNoInterface` 和 null out pointer fallback `kInvalidArgument`；dispatch entries 额外为每个 object 固定 `FUnknown` 以及当前 exposed interfaces 的 per-object queryInterface 计划。
- `vesty-cli` interface skeleton validator 现在要求 `ComObjectIdentityPlan` / `ComObjectQueryInterfaceDispatchEntry` structs、scope/count constants、global/per-object arrays、`COM_OBJECT_IDENTITY_PLANS`、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` 和关键 object dispatch records 存在；旧的只含 object-to-interface exposure plan、但缺 object identity/per-object dispatch metadata 的 skeleton artifact 会被拒绝。
- 该补强仍只是 audit metadata: 它不生成 callable `queryInterface`、不实现 refcount glue、不生成 factory glue、不替代 upstream `vst3` crate 当前 COM wrapper，也不表示完整 SDK 3.8 generated bindings 已完成。
- 最新 strict release-check 仍预期因为真实 DAW/platform/validator/CI/signing/notarization evidence 缺失而失败；新增 metadata 只推进 generated-headers backend 的 deterministic audit surface。

本次 2026-06-10 针对 VST3 SDK interface skeleton pure IID / dispatch lookup seed 补强:

- `generated-interface-skeleton.rs` 现在在 IID 与 per-object dispatch metadata 之外，额外输出 `QUERY_INTERFACE_IID_LOOKUP_SCOPE = "pure-iid-dispatch-lookup-seed-audit"` 以及 `interface_id_for_iid()`、`query_interface_entry_by_interface()`、`query_interface_entry_for_iid()`、`com_object_query_interface_dispatch_by_interface()`、`com_object_query_interface_dispatch_for_iid()` 纯查找 helper。
- 这些 helper 只在已生成的 `INTERFACE_IDS`、`QUERY_INTERFACE_ENTRIES` 和 `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` 表上做 deterministic lookup；测试会把生成的 skeleton 编译成临时模块，再编译并运行一个小程序，验证 `IAUDIOPROCESSOR_IID` 可解析到 `IAudioProcessor`、`VestyProcessor` 可解析到对应 per-object dispatch，`VestyPlugView` 对该 IID 返回 `None`，未知 `TUID::ZERO` 返回 `None`。
- `vesty-cli` interface skeleton validator / `import-ci` 路径现在要求 lookup scope marker 和这组 helper 存在；旧的只有 IID table、但缺 pure lookup helper 的 skeleton artifact 会被拒绝。
- 该补强仍不是 callable COM glue: helper 不写 output object pointer、不调用 `addRef()`、不实现 `queryInterface()` callback、不替代 upstream `vst3` crate 当前 COM wrapper，也不表示完整 SDK 3.8 generated bindings 已完成。

本次 2026-06-10 针对 VST3 SDK interface skeleton factory export / class plan 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `FactoryExportPlan`、`FactoryClassPlan`、`FACTORY_EXPORT_PLAN`、`VESTYPROCESSOR_FACTORY_CLASS_PLAN`、`VESTYCONTROLLER_FACTORY_CLASS_PLAN`、global `FACTORY_CLASS_PLANS`、`FACTORY_EXPORT_PLAN_COUNT`、`FACTORY_CLASS_PLAN_COUNT`、`FACTORY_EXPORT_PLAN_SCOPE = "vesty-factory-export-plan-audit"` 和 `FACTORY_CLASS_PLAN_SCOPE = "vesty-factory-class-plan-audit"`。
- factory export plan 锁定当前 `VestyFactory` 暴露 `IPluginFactory`、`countClasses() = 2` 和 `getFactoryInfo()` metadata 来源；factory class plans 锁定 processor/controller 的 class index、category、`PluginInfo::name` 名称来源、processor CID = `PluginInfo::class_id`、controller CID = `PluginInfo::class_id[15].wrapping_add(1)`、`kManyInstances` cardinality、createInstance target object/root interface、unknown CID `kInvalidArgument`、construction failure `kResultFalse` 和 requested IID delegated-to-instance-queryInterface 策略。
- `vesty-cli` interface skeleton validator 现在要求 factory export/class plan structs、scope/count constants、global/per-class arrays、`FACTORY_EXPORT_PLAN`、`FACTORY_CLASS_PLANS` 和 processor/controller 关键 records 存在；旧的只含 object identity/per-object dispatch metadata、但缺 factory class/export plan 的 skeleton artifact 会被拒绝。
- 该补强仍只是 audit metadata: 它不生成 callable factory exports、不生成 factory glue、不替代 upstream `vst3` crate 当前 `VestyFactory` wrapper，也不表示完整 SDK 3.8 generated bindings 已完成。
- 最新 strict release-check 仍预期因为真实 DAW/platform/validator/CI/signing/notarization evidence 缺失而失败；新增 metadata 只推进 generated-headers backend 的 deterministic audit surface。

本次 2026-06-10 针对 VST3 SDK interface skeleton module export plan 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `ModuleExportPlan`、`MODULE_EXPORT_PLANS`、per-symbol `GETPLUGINFACTORY_MODULE_EXPORT_PLAN` / `WINDOWS_INITDLL_MODULE_EXPORT_PLAN` / `WINDOWS_EXITDLL_MODULE_EXPORT_PLAN` / `MACOS_BUNDLEENTRY_MODULE_EXPORT_PLAN` / `MACOS_BUNDLEEXIT_MODULE_EXPORT_PLAN` / `MACOS_BUNDLEENTRY_COMPAT_MODULE_EXPORT_PLAN` / `MACOS_BUNDLEEXIT_COMPAT_MODULE_EXPORT_PLAN` / `LINUX_MODULEENTRY_MODULE_EXPORT_PLAN` / `LINUX_MODULEEXIT_MODULE_EXPORT_PLAN`、`MODULE_EXPORT_PLAN_COUNT = 9` 和 `MODULE_EXPORT_PLAN_SCOPE = "vesty-module-export-plan-audit"`。
- module export plan 锁定当前 `export_vst3!` 的跨平台入口符号: 全平台 `GetPluginFactory`、Windows `InitDll` / `ExitDll`、macOS `bundleEntry` / `bundleExit` 与兼容 `BundleEntry` / `BundleExit`、Linux `ModuleEntry` / `ModuleExit`，以及每个入口的签名意图、用途、当前实现策略和返回策略。
- `vesty-cli` interface skeleton validator 现在要求 module export plan struct、scope/count constants、per-symbol constants、global `MODULE_EXPORT_PLANS` 和关键 records 存在；旧的只含 factory class/export plan、但缺 platform module export plan 的 skeleton artifact 会被拒绝。
- 该补强仍只是 audit metadata: 它不生成 callable module exports、不替代 `export_vst3!` 宏当前真实导出入口、不生成 factory glue 或 Steinberg method implementations，也不表示完整 SDK 3.8 generated bindings 已完成。
- 最新 strict release-check 仍预期因为真实 DAW/platform/validator/CI/signing/notarization evidence 缺失而失败；新增 metadata 只推进 generated-headers backend 的 deterministic audit surface。

本次 2026-06-10 针对 VST3 SDK interface skeleton binary export symbol plan 审计补强:

- `generated-interface-skeleton.rs` 现在额外输出 `BinaryExportSymbolPlan`、`BINARY_EXPORT_SYMBOL_PLANS`、per-symbol/per-platform constants、`BINARY_EXPORT_SYMBOL_PLAN_COUNT = 11` 和 `BINARY_EXPORT_SYMBOL_PLAN_SCOPE = "vesty-binary-export-symbol-plan-audit"`。
- binary export symbol plan 把当前 `export_vst3!` 入口拆成平台二进制导出预期: Windows x64 PE/COFF `GetPluginFactory` / `InitDll` / `ExitDll`，macOS Mach-O `_GetPluginFactory` / `_bundleEntry` / `_bundleExit` / `_BundleEntry` / `_BundleExit`，Linux x64 ELF `GetPluginFactory` / `ModuleEntry` / `ModuleExit`，并记录建议 inspection tool。
- `vesty-cli` interface skeleton validator 现在要求 binary export symbol plan struct、scope/count constants、global `BINARY_EXPORT_SYMBOL_PLANS` 和 Windows/macOS/Linux 代表性 records 存在；后续补强还要求 `BINARY_EXPORT_INSPECTION_TOOL_PLANS`、inspection tool count/scope 和 `binary_export_inspection_tools()` 存在。旧的只含 platform module export plan、但缺 future binary symbol inspection expected-name/tool plan 的 skeleton artifact 会被拒绝。
- 该补强仍只是 audit metadata: 它不运行 `nm`、`dumpbin` 或 `llvm-objdump`，不验证任何已构建 `.vst3` binary，也不替代后续真实 package/static/validator/platform evidence。
- 最新 strict release-check 仍预期因为真实 DAW/platform/validator/CI/signing/notarization evidence 缺失而失败；新增 metadata 只推进 generated-headers backend 的 deterministic audit surface。
- 本地验证结果: `cargo fmt --all --check`、`cargo test -p vesty-vst3-sys generated_bindings -- --nocapture`、`cargo test -p vesty-cli vst3_sdk -- --nocapture`、`cargo test -p vesty-cli import_ci -- --nocapture`、`cargo test -p vesty-cli release_evidence -- --nocapture`、`cargo check -p vesty-cli -j1`、`npm test` 和 `cargo test --workspace -j1` 均通过；workspace Rust 测试为 419 passed。`release-check --format json --strict --report target/release-check-generated-sdk-binary-export-symbol-plan-seed.json` 仍按预期失败在真实 DAW smoke、platform smoke、validator matrix、CI、签名和 notarization evidence 缺失，host profiles、protocol snapshot 和 VST3 binding baseline 为 ok。

本次 2026-06-10 针对 packaged binary export static evidence:

- `vesty-build::validate_vst3_bundle()` 现在会为每个 platform binary 生成 `BinaryExportCheck`，并透传到 CLI `vesty validate` JSON 的 `static_check.binary_exports`。
- macOS 检查预期 `_GetPluginFactory`、`_bundleEntry`、`_bundleExit`、`_BundleEntry`、`_BundleExit`；Windows x64 检查 `GetPluginFactory`、`InitDll`、`ExitDll`；Linux x64 检查 `GetPluginFactory`、`ModuleEntry`、`ModuleExit`。检查工具按平台尝试 `nm` / `llvm-nm` / `llvm-objdump` / `dumpbin`。
- 如果导出符号工具成功解析 binary 且缺少 required symbol，static bundle validation 会失败；如果工具缺失或无法解析当前格式，report 记录 `status = "skipped"` 和错误说明，不伪造 pass。
- `vesty-cli` release evidence parser 兼容旧 validate JSON 缺失 `static_check.binary_exports`，但显式 binary export check 必须平台/符号/status 自洽: unknown platform、binary path/platform mismatch、不完整 required symbol list、`ok` 但缺 found symbol、`ok` 仍列出 missing symbol、`skipped` 缺 reason 或未知 status 都会失败。最终 `--require-release-artifacts` 的 Vesty 三示例乘三平台 validator/static matrix 现在还要求每个示例/platform report 包含匹配平台的完整 `ok` `static_check.binary_exports`；`skipped` 只保留为诊断，不算最终导出符号证据。
- 本机实包 smoke: `cargo build -p vesty-example-gain --release` 后用 `vesty package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/binary-export-smoke` 生成 `VestyGain.vst3`，再运行 `vesty validate target/binary-export-smoke/VestyGain.vst3 --static-only --format json --report target/binary-export-smoke/gain-static-validate.json`。报告中的 `static_check.binary_exports[0]` 为 `status = "ok"`、`tool = "nm -gU"`，found symbols 包含 `_GetPluginFactory`、`_bundleEntry`、`_bundleExit`、`_BundleEntry` 和 `_BundleExit`。
- 本次又补跑本机 macOS 三示例静态包 smoke: `VestyGain.vst3`、`VestyMIDISynth.vst3` 和 `VestyWebUIDemo.vst3` 均已从 release dylib 打包到 `target/macos-example-static-smoke/` 并通过 `vesty validate --static-only --format json`；三份报告的 `static_check.binary_exports[0]` 均为 `status = "ok"`、`tool = "nm -gU"`，Web UI 示例报告还包含 `asset_manifest` 和 `asset_count = 2`。
- 本地验证结果: `cargo fmt --all --check`、`cargo test -p vesty-build -- --nocapture`、`cargo test -p vesty-cli validate_report_ -- --nocapture`、`cargo test -p vesty-cli release_evidence -- --nocapture`、`cargo test -p vesty-cli`、`cargo test --workspace -j1` 和 `npm test` 均通过；workspace Rust 测试最新为 427 passed。`release-check --format json --strict --report target/release-check-binary-export-schema-hardening.json` 仍按预期失败在真实 DAW/platform/validator/CI/signing/notarization evidence 缺失，host profiles、protocol snapshot 和 VST3 binding baseline 为 ok。

本次 2026-06-09 针对 `release-evidence import-ci` 导入 metadata scaffold / ABI seed / ABI layout / interface skeleton:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli import_ci -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
```

新增行为:

- `import-ci` 会递归扫描已下载 artifact 中的 `.rs` 文件，只对含 VST3 SDK generated-bindings scaffold/ABI/interface marker，或 VST3 SDK artifact 目录里的 `generated*.rs` 候选文件做校验，避免普通 Rust 源码产生报告噪音。
- 有效 scaffold 会复制到 `release-evidence/vst3-sdk/generated.rs`，有效 interface skeleton 会复制到 `release-evidence/vst3-sdk/generated-interface-skeleton.rs`，并在 `import-ci-report.json` 中记录对应 SDK artifact = imported。
- 校验要求 generator marker、`STATUS = "metadata-scaffold"`、plan generator、`PLAN_STATUS = "ready-for-binding-generator"`、SDK/crate baselines、complete header metadata、全部 required header inputs，以及 `BINDINGS_GENERATED = false`；声称 `BINDINGS_GENERATED = true` 或缺 marker 的候选 artifact 会记录 failed 且不复制。
- 当时该 scaffold、ABI seed、ABI layout 和 interface skeleton 仅由 collect/import 留档；后续 2026-06-11 已加入 `ReleaseEvidenceOptions` 和 `release-check` 可选 strict gates，manifest/plan/surface 与四个 `.rs` audit artifact 现在都遵循“缺失 skipped，存在 strict”的 generated-headers 审计语义。

本次 2026-06-09 针对 `collect-local --vst3-sdk-dir` 生成 metadata scaffold / ABI seed / ABI layout / interface skeleton:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
```

新增行为:

- `collect-local --vst3-sdk-dir <official-vst3sdk>` 在显式 opt-in 时除 header manifest、generated-bindings plan 和 generated-bindings surface 外，还会写入 `vst3-sdk/generated.rs` metadata scaffold、`vst3-sdk/generated-abi-seed.rs` ABI seed、`vst3-sdk/generated-abi.rs` ABI layout 和 `vst3-sdk/generated-interface-skeleton.rs` interface skeleton。
- 写入后复用 scaffold / ABI seed / ABI layout / interface skeleton validator 检查 marker、baseline、required headers、`BINDINGS_GENERATED = false`、`ABI_LAYOUT_GENERATED = true`、`INTERFACE_SKELETON_GENERATED = true` 和 `FULL_COM_BINDINGS_GENERATED = false`；若 SDK header 缺失或 output module path 不是 `.rs`，采集会失败而不是写入半真 evidence。
- 默认 `collect-local` 和 CI release evidence 聚合 job 仍不从环境变量隐式生成 SDK evidence，避免把本机 SDK checkout 混入下载 artifact 聚合。

本次 2026-06-09 针对 `collect-local --vst3-sdk-dir` 显式 SDK 审计采集:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli collect_local_release_evidence -- --nocapture
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli
```

新增行为:

- `vesty release-evidence collect-local --vst3-sdk-dir <official-vst3sdk>` 会把官方 SDK checkout 的 header input manifest 写到 `vst3-sdk/vst3-sdk-headers.json`，把 generated-bindings readiness plan 写到 `vst3-sdk/generated-bindings-plan.json`，把 generated-bindings symbol surface 写到 `vst3-sdk/generated-bindings-surface.json`，把 metadata-only scaffold 写到 `vst3-sdk/generated.rs`，把 ABI seed 写到 `vst3-sdk/generated-abi-seed.rs`，把 ABI layout 写到 `vst3-sdk/generated-abi.rs`，并把 interface skeleton 写到 `vst3-sdk/generated-interface-skeleton.rs`；写入后立即复用 release-check/scaffold/ABI/interface validators 校验这些文件。
- `--vst3-sdk-bindings-module <path>` 默认为 `target/vst3-sdk/generated.rs`，用于锁定 future generated bindings `.rs` module path；路径必须保持 `.rs`，否则 binding-plan/scaffold validator 会拒绝。ABI seed/layout 使用各自 `generated-abi*.rs` artifact path，也必须保持 `.rs`。
- 该入口是显式 opt-in；默认 `collect-local` 和 CI release evidence 聚合 job 不从环境变量隐式生成 SDK evidence，避免把本机 checkout 混入下载 artifact 聚合。
- 这些文件仍是 optional generated-headers audit evidence；manifest/plan/surface 和四个 `.rs` artifact 缺失在 `--require-release-artifacts` 下保持 `skipped`，存在则严格验证。`generated-bindings-plan.json` 和 `generated-bindings-surface.json` 必须保持 `bindingsGenerated = false`，`generated.rs` 必须保持 `BINDINGS_GENERATED = false`，`generated-abi-seed.rs`、`generated-abi.rs` 和 `generated-interface-skeleton.rs` 必须保持 `FULL_COM_BINDINGS_GENERATED = false`，不代表完整 SDK 3.8 bindings 已生成。

本次 2026-06-09 针对 Rust native JSBridge state/param payload validation 补强:

```bash
cargo fmt --all --check
cargo check -p vesty-bridge -j1
cargo test -p vesty-bridge state_ -- --nocapture
cargo test -p vesty-bridge param_ -- --nocapture
cargo test -p vesty-bridge subscription -- --nocapture
cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture
npm --workspace @vesty/plugin-ui run typecheck
npm --workspace @vesty/plugin-ui test
```

新增行为:

- `BridgeRuntime` 新增 shared payload helpers，并将 `state.setConfig` / `state.setUiState`、`param.begin` / `param.perform` / `param.end` / `param.format` / `param.parse` 全部收口到 native authoritative schema parsing。
- State command 会区分 missing `baseRevision`、wrong-type `baseRevision`、missing/wrong-type config key、missing value；`baseRevision` 必须是非负整数，`value: null` 被保留为合法 JSON 写入。
- Param command 会区分 missing/wrong-type/empty/control-character parameter id、missing/wrong-type normalized、missing/wrong-type parse text、wrong-type/empty/too-long/control-character optional `gestureId`；无效 payload 不会写入 pending gesture queue。
- `@vesty/plugin-ui` 和 `vesty-ui-wry` bootstrap 仍保留前端预校验，但 Rust bridge runtime 现在在 native IPC 边界用同一语义作为最终裁决源。

本次 2026-06-09 针对 Rust native JSBridge `bridge.readyAck` protocol/order guard 补强:

```bash
cargo fmt --all --check
cargo check -p vesty-bridge -j1
cargo test -p vesty-bridge ready_ack -- --nocapture
cargo test -p vesty-bridge hello -- --nocapture
cargo test -p vesty-bridge subscription -- --nocapture
cargo test -p vesty-bridge -- --nocapture
cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture
npm --workspace @vesty/plugin-ui run typecheck
npm --workspace @vesty/plugin-ui test
```

新增行为:

- `BridgeRuntime` 现在要求 `bridge.readyAck` 必须发生在成功 `bridge.hello` 之后；提前 ack 返回 non-retryable `permission_denied`，不会把 runtime 标记为 ready。
- 通过顺序 gate 后，runtime 继续校验 `bridge.readyAck` payload 的 `protocolVersion` 字段: 缺失或 wrong type 返回 non-retryable `validation_error`，不支持版本返回 `unsupported_version`。
- 无效 readyAck 不会把 `ready_acknowledged()` 置为 true，避免旧 UI、畸形 bootstrap 或测试 harness 绕过 JS ready response guard 后让 native runtime 进入半 ready 状态；旧 diagnostics 测试也已改为真实 hello -> readyAck 流程。
- `@vesty/plugin-ui` 与 `vesty-ui-wry` 仍从 ready payload 派生并发送 `protocolVersion`，正常 hello/readyAck 流程由 Rust、wry bootstrap 和 JS SDK 回归测试覆盖。

本次 2026-06-09 针对 JSBridge param begin `gestureId` 贯穿补强:

```bash
cargo fmt --all --check
cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture
npm --workspace @vesty/plugin-ui test
npm --workspace @vesty/plugin-ui run typecheck
npm --workspace @vesty/react test
npm --workspace @vesty/vue test
npm --workspace @vesty/svelte test
npm test
npm run typecheck
```

新增行为:

- `@vesty/plugin-ui` 的 `beginParamEdit(id, gestureId?)` 现在与 `performParamEdit()` / `endParamEdit()` 一样支持 optional gesture token，并在发送前复用 `gestureId` 非空、最长 128 bytes、无控制字符的预校验。
- `vesty-ui-wry` 注入 bootstrap 的 fallback `beginParamEdit(id, gestureId?)` 同步支持并校验 gesture token，发送 `param.begin` payload `{ id, gestureId }`。
- `setParam(id, normalized, gestureId?)` 会把同一个 optional gesture token 贯穿 `param.begin` -> `param.perform` -> `param.end`，并在发送前复用同一套 gesture token 预校验。
- `@vesty/react`、`@vesty/vue` 和 `@vesty/svelte` 的 param edit helper API 已对齐为 `begin(gestureId?)` 与 `set(normalized, gestureId?)`；Vue/Svelte adapter 测试覆盖 begin/perform/set/end 都会把同一个 token 转发到底层 bridge。
- `@vesty/plugin-ui` 测试覆盖 `beginParamEdit("gain", "drag-1")` 发出 `param.begin` 且 payload 带 `gestureId`，并覆盖 begin 阶段非法 gesture token 不会触发 IPC。

本次 2026-06-09 针对 JSBridge generic `request(type, payload)` type validation 补强:

```bash
npm --workspace @vesty/plugin-ui test
npm --workspace @vesty/plugin-ui run typecheck
cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture
cargo fmt --all --check
npm test
npm run typecheck
cargo test -p vesty-ui-wry --features wry-backend -- --nocapture
```

新增行为:

- `@vesty/plugin-ui` 新增 shared `assertRequestType()`，所有 JS SDK request 在进入 `post()` 前校验 packet type；必须是 string、非空、最长 128 UTF-8 bytes、不能包含控制字符。
- `vesty-ui-wry` 注入 bootstrap 的 `window.__VESTY_INTERNAL__.request(type, lane, payload)` 使用同一套 type 校验，保护不加载 npm SDK、直接使用 fallback `window.__VESTY__` 的页面。
- 无效 generic `bridge.request(type, payload)` 会抛出 non-retryable `validation_error`，不会创建 pending request、启动 timer 或调用 `window.ipc.postMessage`；后续合法 request 仍可正常发送和 resolve。
- Node 回归测试覆盖 non-string、empty、ASCII/UTF-8 超长和 control-character request type；wry bootstrap 文本测试固定 `MAX_BRIDGE_PACKET_TYPE_BYTES`、`assertRequestType(type)` 和对应错误语义存在。

本次 2026-06-09 针对 Rust native JSBridge packet type / parse error 兜底补强:

```bash
cargo test -p vesty-ipc -- --nocapture
cargo test -p vesty-bridge -- --nocapture
cargo test -p vesty-ui-wry --features wry-backend -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' -- --nocapture
cargo fmt --all --check
```

新增行为:

- `vesty-ipc` 新增共享 `MAX_BRIDGE_PACKET_TYPE_BYTES = 128` 和 `validate_packet_type()`，并让 `BridgePacket::response_to()` / `error_to()` 在原始 packet type 畸形时使用安全的 `bridge.invalidType.response|error`，避免把控制字符或超长 type 原样拼入回包。
- `BridgeRuntime::handle_packet()` 在 protocol version 和 session 通过后、dispatch 之前执行 packet type 权威校验；直接伪造 IPC 的 empty、超长或 control-character `type` 会收到 non-retryable `validation_error`，不会进入 hello/state/param/subscription handler。
- `BridgeRuntime::receive_json()` 对当前 session、`kind = "request"`、且带合法 string `id` 的可恢复反序列化失败 request 返回 non-retryable `parse_error`，回包 type 为 `bridge.parseError.error`；session 不匹配、非 request 或 id 畸形的 parse error 仍不回包。
- `vesty-vst3` 的 wry bridge endpoint 测试覆盖 adapter 层会把 native `parse_error` 和 `validation_error` packet 返回 WebView，证明错误没有在 VST3/wry 接线里被吞掉。

本次 2026-06-09 针对最新依赖基线 drift gate / CI artifact:

```bash
cargo search wry --limit 1
cargo search vst3 --limit 1
cargo search raw-window-handle --limit 1
cargo search rtrb --limit 1
cargo search serde --limit 1
cargo search serde_json --limit 1
cargo search ts-rs --limit 1
cargo search clap --limit 1
npm outdated --workspaces --long || true
npm install --workspace @vesty/plugin-ui --workspace @vesty/react --workspace @vesty/vue --workspace @vesty/svelte typescript@latest --save-dev
npm test
npm run typecheck
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli dependency_baseline -- --nocapture
cargo run -p vesty-cli -- dependency-baseline --out target/dependency-baseline/dependency-baseline.json
cargo run -p vesty-cli -- dependency-baseline --check --out target/dependency-baseline/dependency-baseline.json
cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline/dependency-baseline-latest.json --format text
cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline/dependency-baseline-latest.json --format text
```

新增行为:

- `@vesty/plugin-ui`、`@vesty/react`、`@vesty/vue`、`@vesty/svelte` 的 `typescript` devDependency 已升级为 `^6.0.3`，`package-lock.json` 中 `node_modules/typescript` 为 `6.0.3`。
- `vesty-cli` 新增 `vesty dependency-baseline`，生成 deterministic JSON report，离线校验全部当前外部 Cargo workspace dependencies、VST3 SDK/binding baseline、四个 JS package 的 TypeScript range、React/Vue/Svelte adapter devDependency range，以及 lockfile installed versions。
- `vesty dependency-baseline --check --out <report>` 会复验已有 report 与当前 workspace 是否一致；report drift、missing dependency 或版本不匹配都会非零退出。
- `vesty dependency-baseline --latest --out <report>` 会显式联网查询 crates.io / npm registry latest 并把 registry latest checks 加入 report；Rust latest 查询现在 `cargo search` 优先、`cargo info` fallback，本次真实运行中 `cargo search ts-rs` 返回 crates.io 500，但 fallback 到 `cargo info ts-rs` 后仍确认 `ts-rs 12.0.1`。本次真实运行全绿，覆盖所有当前外部 workspace Rust dependencies，并确认 npm `typescript 6.0.3`、`react 19.2.7`、`@types/react 19.2.17`、`vue 3.5.38` 和 `svelte 5.56.3`。`toml` registry latest 以 `1.1.2+spec-1.1.0` 校验，Cargo manifest 仍保留 `1.1.2` 以避免 SemVer metadata warning。
- `.github/workflows/ci.yml` 新增 `dependency-baseline` job，运行离线生成/check 和显式 `--latest` 生成/check，并上传 `vesty-dependency-baseline` artifact；`release evidence snapshot` job 依赖该 job，从 CI 层面防止已复核依赖基线漂移，并为 final release gate 留存 crates.io/npm latest evidence。
- 默认 gate 不联网判定“最新”；`--latest` 是显式联网复核。Steinberg SDK 最新性仍需 release 前按官方 tag/change history 单独复核。

本次 2026-06-09 针对 VST3 message `IAttributeList` 支持补强:

```bash
cargo test -p vesty-vst3 --features vst3-bindings attribute_list -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' attribute_list -- --nocapture
cargo fmt --all --check
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui' -j1
```

新增行为:

- `vesty-vst3` 的内部 `VestyAttributeList` 从只支持 `int` 扩展为支持 VST3 `int` / `float` / `string` / `binary` attribute set/get，供非实时 processor/controller `IMessage` 通道使用。
- `setString` 存储 nul-terminated `TChar` UTF-16 buffer；`getString` 按 VST3 的 byte-size capacity 转换为 `TChar` capacity，buffer 足够时完整复制，buffer 过小时复制可容纳前缀、强制最后一个 code unit 为 nul，并返回 `kResultFalse`。
- `setBinary` 支持 size 0 的空 binary；非空 binary 如传入 null data 会返回 `kInvalidArgument`。`getBinary` 返回 attribute list 内部存储指针和 size，空 binary 返回 null data + size 0。
- 单元测试覆盖 int/float/string/binary roundtrip、string truncation nul-termination、empty binary、null id/output pointer、missing attribute 和 invalid binary data pointer。

本次 2026-06-09 针对 VST3 main bus routing info 补强:

```bash
cargo test -p vesty-vst3 --features vst3-bindings processor_negotiates -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' processor_negotiates -- --nocapture
cargo fmt --all --check
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui' -j1
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' -- --nocapture
```

新增行为:

- `IComponent::getRoutingInfo()` 不再对所有调用返回 `kNotImplemented`；在当前单 main bus 模型下，audio effect 的 main audio input bus `0` / valid channel 映射到 main audio output bus `0` / all channels。
- Instrument plugin 的 main event input bus `0` / channel `-1..=15` 映射到 main audio output bus `0` / all channels，便于 host 查询 event input 与 audio output 关系。
- 无效 media type、bus index、channel、null input pointer 和 null output pointer 会返回 `kInvalidArgument`。
- 现有 effect/instrument bus negotiation fake COM tests 已扩展覆盖 routing success/failure；完整 `vesty-vst3` + `vst3-bindings wry-ui` 测试仍通过。

本次 2026-06-09 针对 VST3 `Sample64` process path 补强:

```bash
cargo test -p vesty-vst3 --features vst3-bindings sample64 -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' sample64 -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings processor_negotiates_mvp_effect_bus_arrangements -- --nocapture
cargo fmt --all --check
cargo check -p vesty-vst3 --features 'vst3-bindings wry-ui' -j1
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' -- --nocapture
```

新增行为:

- `IAudioProcessor::canProcessSampleSize(kSample64)` 现在返回 `kResultOk`，与 `kSample32` 一起声明支持。
- VST3 adapter 保持 developer-facing `vesty-core::AudioBuffers` 为 f32；`setupProcessing()` 会按 host `maxSamplesPerBlock` 在非实时生命周期预分配最多双声道的 f32 scratch。
- `ProcessData.symbolicSampleSize = kSample64` 时，adapter 在 realtime `process()` 中把 host `Sample64` input/output block 转为 f32 scratch，调用现有 `AudioKernel`，再把 f32 output 回写到 host f64 buffer；该路径不改 public DSP trait。
- 如果 host 传入超过预分配 capacity 的 64-bit block，adapter 会清零 host outputs、设置 silence flags 并返回，不在 realtime path 扩容。
- fake COM 测试覆盖 `Sample64` 输出回写、`NoAllocGuard` 下零分配、未调用 `setupProcessing()` 的异常 host 调用，以及 block 超过预分配 capacity 的静音 fallback；完整 `vesty-vst3` + `vst3-bindings wry-ui` 回归为 46 tests passed。

本次 2026-06-09 针对原生 double-precision developer DSP API 补强:

```bash
cargo fmt --all --check
cargo check -p vesty -p vesty-core -p vesty-vst3 --features vesty-vst3/vst3-bindings -j1
cargo test -p vesty-core 64 -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings sample64 -- --nocapture
cargo test -p vesty-vst3 --features 'vst3-bindings wry-ui' sample64 -- --nocapture
cargo test --workspace -j1
```

新增行为:

- `vesty-core` 新增 `AudioBuffers64` 和 `ProcessContext64`，与 f32 `AudioBuffers` / `ProcessContext` 对齐，支持 channel/frame 查询、copy range、set sample、clear outputs、参数读取、automation iterator/segments、transport、process mode 和 meter emission。
- `AudioKernel` 新增 opt-in hook: `const SUPPORTS_F64: bool = false` 和默认 `process_f64(&mut ProcessContext64) -> ProcessResult::Silence`，保持现有 f32 插件源码兼容。
- `vesty-vst3` 的 `kSample64` dispatch 现在按 `P::Kernel::SUPPORTS_F64` 分流: 默认继续走预分配 f32 scratch fallback；opt-in f64 kernel 直接以 host f64 buffers 构造 `AudioBuffers64` 并调用 `process_f64()`。
- fake COM 测试覆盖 native f64 kernel 不进入 f32 fallback、`NoAllocGuard` active、realtime path 0 allocation、直接写回 host f64 outputs，并用超过 scratch `maxSamplesPerBlock` 的 block 证明 native f64 路径不依赖 scratch capacity。

本次 2026-06-09 针对 dependency latest release evidence gate:

```bash
cargo test -p vesty-cli dependency_baseline -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli import_ci -- --nocapture
cargo test -p vesty-cli release_check_requires_release_artifacts_when_requested -- --nocapture
cargo fmt --all --check
```

新增行为:

- `release-check --dependency-baseline-report <path>` 和 `--release-evidence-dir <dir>` 会严格校验 `vesty dependency-baseline --latest` 生成的 latest report；该 report 必须包含 `cargo workspace external dependency baseline coverage`、所有当前外部 workspace Rust dependency 的 crates.io latest checks，以及 npm `typescript`、`react`、`@types/react`、`vue`、`svelte` latest checks。
- `--require-release-artifacts` 现在要求 `dependency latest baseline` evidence；普通离线 `dependency-baseline.json` 会因为缺少 registry latest checks 被拒绝，不会被当作 release latest evidence。
- `vesty release-evidence collect-local --dependency-baseline-latest` 是显式 opt-in，会联网生成 `dependency-baseline/dependency-baseline-latest.json` 并立即复验；默认 `collect-local` 仍不联网。
- `vesty release-evidence import-ci` 会从下载的 `vesty-dependency-baseline` artifact 中导入有效 `dependency-baseline-latest.json` 到 `release-evidence/dependency-baseline/dependency-baseline-latest.json`；无 latest checks 的离线 report、或缺少 workspace 外部依赖覆盖检查的 latest report，只会记录为 failed import item。
- `.github/workflows/ci.yml` 的 `dependency-baseline` job 现在同时生成/check 离线 report 和 `--latest` report，`release-evidence` job 会下载 `vesty-dependency-baseline` artifact 并交给 `import-ci` 规范化。

本次 2026-06-09 针对 VST3 SDK generated-bindings surface symbol token audit 补强:

```bash
cargo test -p vesty-vst3-sys generated_bindings_surface -- --nocapture
cargo fmt --all
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo test -p vesty-cli import_ci -- --nocapture
```

新增行为:

- `GeneratedBindingsSurface` schema 升级到 version 2，新增 `missingSymbols` 顶层列表和每个 `GeneratedBindingsSurfaceSymbol.symbolPresent` 布尔值。
- `generated_bindings_surface()` 不再只检查 header 是否存在；它会读取 locked header manifest 中的 header 文本，并用 identifier-token 边界检查 required interface/type/constant 名称是否真的出现在对应 header 中。缺失 token 会让 surface 进入 `blocked`，并写入 `missingSymbols` 与 blockers。
- `generated_bindings_surface_differences()`、metadata-only `generated.rs` scaffold 和测试 fake SDK header fixture 已同步新 schema；fixture 会写入各 header 对应的 expected symbol token。
- `vesty vst3-sdk binding-surface` 的文本报告现在显示 missing symbol 数量；`release-check` / `release-evidence import-ci` 会拒绝 `missingSymbols` 非空、`symbolPresent = false` 或二者不一致的 surface evidence。
- 该检查仍是文本 identifier-token 层面的审计，不解析 C++ AST；`generated-abi.rs` 只固定少量 foundational `repr(C)` layout 及 Rust 侧 size/alignment/field-offset 指纹，不等同完整 ABI 验证，也不生成完整 Rust bindings。

本次 2026-06-09 针对 crate package readiness evidence gate:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli crate_package -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli import_ci -- --nocapture
cargo test -p vesty-cli release_check -- --nocapture
cargo test -p vesty-cli publish_plan -- --nocapture
cargo test -p vesty-cli -j1
rm -rf target/crate-package-smoke
cargo run -p vesty-cli -- crate-package --out target/crate-package-smoke/crate-package.json
cargo run -p vesty-cli -- crate-package --check --out target/crate-package-smoke/crate-package.json
rm -rf target/release-evidence-crate-package-smoke target/vesty-protocol-crate-package-smoke
cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-crate-package-smoke --protocol-snapshot target/vesty-protocol-crate-package-smoke --crate-package
```

新增行为:

- 新增 `vesty crate-package`，从 workspace publish plan 生成 crate package readiness report；无内部 workspace 依赖的 crates 会运行真实 `cargo package -p <crate> --allow-dirty --no-verify` 并标记为 `packaged`，仍依赖内部 workspace crates 的 package 会标记为 `deferred`。
- 当前真实 smoke 结果为 13 个 publishable crates: `vesty-params`、`vesty-macros`、`vesty-vst3-sys` 已 package；其余 10 个因内部依赖尚未发布而 deferred。
- `vesty crate-package --check --out <report>` 会复验已有 report，要求 leaf crates 为 `packaged`、internal-dependent crates 为 `deferred`、dependency order 早于 dependent、无重复 package name/order，且整体 `status = ok`。
- `release-check --crate-package-report <path>` 和 `--release-evidence-dir` 中的 `crate-package/crate-package.json` 会把该 report 作为 readiness evidence；普通本地检查缺失时保持 `skipped`，`--require-release-artifacts` 下缺失会失败，存在则严格校验。它不表示已经执行 `cargo publish`。
- `vesty release-evidence collect-local --crate-package` 会在本地 evidence 目录额外写入并复验 `crate-package/crate-package.json`；默认不跑，因为它会执行真实 `cargo package` smoke。
- `vesty release-evidence import-ci` 会识别并内容验证 `vesty-crate-package` artifact，只有有效 report 才复制到 `crate-package/crate-package.json`；无效 report 会记录为 failed 且不落盘。
- `.github/workflows/ci.yml` 的 `publish-plan` job 现在同时生成/check `target/crate-package/crate-package.json` 并上传 `vesty-crate-package` artifact；`release-evidence` job 会下载该 artifact 并交给 `import-ci` 规范化。
- `.agents/07-build-packaging.md`、`.agents/08-developer-guide.md` 和 `.agents/14-completion-audit.md` 已同步命令、CI artifact、release evidence 自动发现路径和可选 readiness gate 语义。

本次 2026-06-09 针对 headless smoke host / 本地框架自检:

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-cli smoke_host -- --nocapture
npm run build --prefix examples/web-ui-param-demo/ui
mkdir -p target/smoke-host
printf '%s\n' '{"type":"param.begin","result":0}' '{"type":"param.perform","result":0}' '{"type":"param.end","result":0}' 'result=0' > target/smoke-host/bridge-trace.log
printf '%s\n' 'meter_flush sent=1' > target/smoke-host/meter.log
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --strict
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --check --strict
```

新增行为:

- 新增 `vesty smoke-host`，生成 `version = 1`、`generator = "vesty-cli.smoke-host.v1"` 的本地 headless self-check report。
- 检查 workspace `Cargo.toml`、`gain` / `midi-synth` / `web-ui-param-demo` 三个 MVP examples 的 `vesty.toml`、`params.specs.json` 到 `vesty-parameters.json` drift，以及 Web UI demo 的 `ui/dist` asset manifest。
- `--bridge-trace` 接受 readyAck/reply roundtrip 或 param begin/perform/end marker；`--meter-log` 接受非零 meter stream marker；未传时 report 为 `partial`，`--strict` 会返回非零。
- `--out` 写出 JSON report；`--check --out` 复验当前 workspace 状态与已有 report 完全一致。
- `.github/workflows/ci.yml` 新增 `headless smoke host` job，构建 Web UI demo assets、生成本地 bridge/meter marker、运行 `vesty smoke-host --strict` 和 `--check --strict`，上传 `vesty-smoke-host` artifact。
- 该工具不加载插件 `.dylib` / `.dll` / `.so`，不执行二进制 metadata introspection，也不替代真实 DAW、platform WebView、Steinberg validator、签名或 notarization evidence；CI `vesty-smoke-host` artifact 只用于诊断，不进入 `release-evidence import-ci` 或 final release gate。

本次 2026-06-09 针对 VST3 optional sidechain MVP:

```bash
cargo fmt --all
cargo check -p vesty-vst3 -j1
cargo test -p vesty-vst3 --features vst3-bindings sidechain -- --nocapture
cargo test -p vesty-core sidechain -- --nocapture
cargo test -p vesty-build sidechain -- --nocapture
cargo check -p vesty-core -p vesty-vst3 -p vesty-build -p vesty-cli -j1
```

新增行为:

- `vesty-core` 新增 `SidechainBuffers` / `SidechainBuffers64` 与 `ProcessContext::sidechain()` / `ProcessContext64::sidechain()`，`Plugin::sidechain_inputs()` 默认返回 `0`，老插件源码兼容。
- `vesty-vst3` 对 effect 插件支持一个 optional audio sidechain input bus: main input bus `0` 保持 `kMain` + default active，sidechain input bus `1` 为 `kAux` + 非 default active；instrument 不暴露 audio sidechain。
- `setBusArrangements()` 对 sidechain effect 接受 `numIns = 1` 或 `2`，sidechain arrangement 只接受 mono/stereo；`getBusArrangement()` 和 `getRoutingInfo()` 已覆盖 sidechain bus index `1`。
- `process()` 中 `ProcessData.inputs[0]` 映射到 main `AudioBuffers`，`ProcessData.inputs[1]` 只在插件声明 sidechain 时映射到 `ProcessContext::sidechain()`，不会混入主输入。
- `kSample64` fallback path 已将 sidechain f64 input 拷贝到 `setupProcessing()` 预分配的 f32 sidechain scratch 后再调用 f32 kernel；实时路径不做扩容。
- `vesty-build` 支持 `[plugin].sidechain = true|false`，并拒绝 instrument + sidechain。`moduleinfo.json` 不写 sidechain，sidechain bus metadata 由 VST3 runtime bus info 提供。
- fake COM 测试覆盖 sidechain bus count/info/arrangement/routing、`kSample32` sidechain process path 和默认 `kSample64` scratch fallback sidechain process path。
- 该能力仍缺真实 DAW sidechain routing smoke、Steinberg validator sidechain-specific report，以及 Windows/Linux host evidence；不能据此声明 release-ready sidechain。

本次 2026-06-09 针对 VST3 program list metadata + opt-in apply MVP:

```bash
cargo fmt --all
cargo test -p vesty-core program_list -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings program_list -- --nocapture
cargo check -p vesty-core -p vesty-vst3 -p vesty -p vesty-cli -j1
```

新增行为:

- `vesty-core` 新增 `Program` / `ProgramList` / `ProgramAttribute` / `ProgramPitchName` 静态描述类型，`Plugin::program_lists()`、`program_attributes()` 和 `program_pitch_names()` 默认返回空 slice；`Plugin::apply_program(list_id, program_index)` 默认返回 `Ok(false)`，保持老插件源码兼容。
- `vesty` facade prelude 导出 `Program` / `ProgramList` / `ProgramAttribute` / `ProgramPitchName`，开发者可直接 `use vesty::prelude::*` 编写 program list。
- `vesty-vst3` controller 新增 `IUnitInfo` interface，暴露一个 root unit，并将第一个非空 program list 绑定到 root unit；空 list 不暴露给 host。
- `getProgramListCount()` / `getProgramListInfo()` / `getProgramName()` 支持 host 查询静态 program list metadata；invalid list id、program index 或 null out pointer 返回 `kInvalidArgument`。
- `getProgramInfo()` 暴露开发者 opt-in 的静态 program attributes，并过滤空 id、NUL-containing id/value；missing attribute 返回 `kResultFalse`，invalid list/program/null pointer 返回 `kInvalidArgument`。
- `hasProgramPitchNames()` / `getProgramPitchName()` 暴露开发者 opt-in 的静态 MIDI pitch names，并过滤越界 pitch、空 name 和 NUL-containing name；missing pitch 返回 `kResultFalse`，invalid list/program/pitch/null pointer 返回 `kInvalidArgument`。
- `getUnitByBus()` 将当前 MVP 支持的 audio output、effect main/sidechain audio input、instrument event input 映射到 root unit。
- `setUnitProgramData()` 会验证 list/root unit id 与 program index；有效 program 会调用 `apply_program()`，`Ok(true)` 返回 `kResultOk`，`Ok(false)` 返回 `kNotImplemented`，`Err(_)` 返回 `kResultFalse`，无效输入返回 `kInvalidArgument`。
- opt-in apply/program-data load 成功后，adapter 会 diff controller 参数值、合并 `HostChangeFlags::PARAM_VALUES` 与 `host_changes_for_param()`、调用 host `restartComponent()`，并把变化以 `source = "program"` 排队给 Web UI bridge；program-change 参数成功选择 program 时也使用同一 program source，不会被 controller host-change queue 覆盖成 `source = "host"`。
- fake COM 测试覆盖 root unit metadata、program list info/name、selected/select unit、program attributes、pitch names、无效 metadata 过滤、bus-to-unit mapping、同一 COM controller 实例上的 list id/root unit program apply、program data stream、program-change 参数 `setParamNormalized()` / edit relay selection、audio `process()` 内 program-change 参数 automation 的 realtime-safe 普通参数事件语义、参数变化、`kParamValuesChanged` host 通知和 Web UI `source = "program"` 回推。
- 当前能力不是完整跨 host preset/program 验收；已提供 program-change 参数 metadata、controller/control-thread program-change 参数 selection、audio-thread program-change 参数 automation 普通参数语义、controller-side program data envelope/helper，以及 `examples/midi-synth` 的 concrete program/preset workflow 示例。真实 DAW program workflow evidence 仍未提供。

本次 2026-06-10 针对 `examples/midi-synth` program/preset workflow 示例:

```bash
cargo run -p vesty-cli -- param-manifest --specs examples/midi-synth/params.specs.json --out examples/midi-synth/vesty-parameters.json
cargo fmt --all
cargo test -p vesty-example-midi-synth -- --nocapture
cargo run -p vesty-cli -- param-manifest --specs examples/midi-synth/params.specs.json --out examples/midi-synth/vesty-parameters.json --check
cargo test -p vesty-vst3 --features vst3-bindings program -- --nocapture
cargo test -p vesty-cli smoke_host -- --nocapture
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --strict
cargo fmt --all --check
cargo check -p vesty-example-midi-synth -j1
```

新增行为:

- `examples/midi-synth` 新增 `program` `ChoiceParam`，使用 `.as_program_change()` 暴露 VST3 `kIsProgramChange` metadata，并用 MIDI Program Change controller mapping 标记 host-facing program selector。
- `midi-synth` 新增静态 `ProgramList` / `Program` / `ProgramAttribute` / `ProgramPitchName` metadata，host 可查询 `Init`、`Bright Lead`、`Soft Bass` 三个 factory programs、分类/character attributes 和常用 pitch names。
- `midi-synth` 实现 `apply_program()`、`program_data_supported()`、`save_program_data()` 和 `load_program_data()`，在 controller/control-thread 路径把 program selection 和 per-program JSON data 应用到 atomic 参数 `level` / `program`；audio `process()` 仍只按 `ParamHandle` 读取参数快照。
- `midi-synth` 新增示例单测，覆盖 program metadata、program-change 参数 flag、program apply、program data save/load 和 mismatched program data rejection。
- `examples/midi-synth/params.specs.json` 和 `vesty-parameters.json` 已同步为 2 个参数；当前 workspace `vesty smoke-host --strict` 报告 `midi-synth parameter sidecar: ok - 2 parameter(s)`。
- 该示例证明 framework-level program/preset flow 的开发形态，但仍不等同真实 DAW program workflow 通过；后者继续保留在外部 smoke evidence 中。

本次 2026-06-10 针对 `examples/midi-synth` SysEx / Note Expression DSP 示例:

```bash
cargo fmt --all --check
cargo test -p vesty-example-midi-synth -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings sysex -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings note_expression -- --nocapture
cargo test --workspace -j1
npm test
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --check --strict
```

新增行为:

- `examples/midi-synth` 新增 `note_expression::BRIGHTNESS` / `TUNING` value metadata 和 pressure/X movement physical UI mapping metadata，作为开发者 opt-in Note Expression 的 concrete instrument 示例。
- `SynthKernel` 新增 `brightness`、`tuning_cents` 和 `sysex_level_override` realtime state；`process()` 在事件循环里消费固定格式 SysEx `[F0, 7D, level, F7]`、`NoteExpressionValue(BRIGHTNESS)` 和 `NoteExpressionValue(TUNING)`，只更新 kernel 内部字段，不写 controller state、不做 JSON、不分配内存。
- 新增示例单测 `consumes_sysex_and_note_expression_without_touching_controller_state` 和 `ignores_unmatched_note_expression_and_invalid_sysex`，直接构造 `AudioBuffers` / `ProcessContext` 验证 DSP 消费路径、无效 SysEx 截断防御、unmatched note expression 忽略和 controller 参数状态不被事件路径污染。
- 该示例补齐 developer-facing SysEx / Note Expression 编写形态；真实 DAW SysEx 和 expression workflow evidence 仍未提供，不能据此声明跨 host 通过。

本次 2026-06-10 针对 VST3 program-change 参数 automation realtime-safe 语义:

- 新增 fake COM 测试 `processor_treats_program_change_automation_as_realtime_safe_param_event`，构造 `.as_program_change()` 参数的 host automation queue，验证 processor `process()` 在 `NoAllocGuard` 内把点位转换为 `CoreEvent::Param`，更新最终 normalized 参数值，并保持 `Plugin::apply_program()` / `Plugin::load_program_data()` 调用次数为 0。
- 该行为将 VST3 `kIsProgramChange` 的 controller-side program selection 与 audio-thread sample-accurate automation 明确分开: program apply/data load 仍只允许在 controller/control thread 执行，audio thread 不套用 program/state。

```bash
cargo fmt --all --check
cargo test -p vesty-vst3 --features vst3-bindings program_change -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings processor_translates_automation_midi_and_transport -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings processor_process_does_not_allocate_inside_rt_guard_under_automation_and_midi -- --nocapture
cargo test --workspace -j1
npm test
```

本次 2026-06-10 针对 CLI template gallery 和生成项目 smoke:

```bash
cargo test -p vesty-cli project_template -- --nocapture
cargo test -p vesty-cli create_project_accepts_template_gallery_defaults_and_overrides -- --nocapture
cargo test -p vesty-cli every_builtin_project_template_generates_expected_files -- --nocapture
cargo test -p vesty-cli create_project_uses_leaf_directory_as_project_name -- --nocapture
cargo run -p vesty-cli -- templates
cargo run -p vesty-cli -- templates --format json
cargo test -p vesty-cli -- --nocapture
rm -rf target/template-gallery-smoke
mkdir -p target/template-gallery-smoke
cargo run -p vesty-cli -- new target/template-gallery-smoke/template-gain --template gain --vesty-path crates/vesty
cargo check --manifest-path target/template-gallery-smoke/template-gain/Cargo.toml
cargo run -p vesty-cli -- new target/template-gallery-smoke/template-synth --template midi-synth --vesty-path crates/vesty
cargo check --manifest-path target/template-gallery-smoke/template-synth/Cargo.toml
cargo run -p vesty-cli -- new target/template-gallery-smoke/template-web --template web-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
cargo check --manifest-path target/template-gallery-smoke/template-web/Cargo.toml
npm install --prefix target/template-gallery-smoke/template-web/ui
npm run build --prefix target/template-gallery-smoke/template-web/ui
npm run typecheck --prefix target/template-gallery-smoke/template-web/ui
cargo run -p vesty-cli -- new target/template-gallery-smoke/template-vanilla --template vanilla-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
cargo run -p vesty-cli -- new target/template-gallery-smoke/template-vue --template vue-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
cargo run -p vesty-cli -- new target/template-gallery-smoke/template-svelte --template svelte-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
cargo run -p vesty-cli -- new target/template-gallery-smoke/template-web-instrument --template web-ui-instrument --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
cargo check --manifest-path target/template-gallery-smoke/template-vanilla/Cargo.toml
cargo check --manifest-path target/template-gallery-smoke/template-vue/Cargo.toml
cargo check --manifest-path target/template-gallery-smoke/template-svelte/Cargo.toml
cargo check --manifest-path target/template-gallery-smoke/template-web-instrument/Cargo.toml
npm install --prefix target/template-gallery-smoke/template-vanilla/ui
npm run build --prefix target/template-gallery-smoke/template-vanilla/ui
npm run typecheck --prefix target/template-gallery-smoke/template-vanilla/ui
npm install --prefix target/template-gallery-smoke/template-vue/ui
npm run build --prefix target/template-gallery-smoke/template-vue/ui
npm run typecheck --prefix target/template-gallery-smoke/template-vue/ui
npm install --prefix target/template-gallery-smoke/template-svelte/ui
npm run build --prefix target/template-gallery-smoke/template-svelte/ui
npm run typecheck --prefix target/template-gallery-smoke/template-svelte/ui
npm install --prefix target/template-gallery-smoke/template-web-instrument/ui
npm run build --prefix target/template-gallery-smoke/template-web-instrument/ui
npm run typecheck --prefix target/template-gallery-smoke/template-web-instrument/ui
rm -rf target/template-path-name-smoke
cargo run -p vesty-cli -- new target/template-path-name-smoke/my-gain --template gain --vesty-path crates/vesty
cargo check --manifest-path target/template-path-name-smoke/my-gain/Cargo.toml
```

新增/复核行为:

- `vesty templates` 提供 first-party starter gallery 的 text/json 输出，当前包含 `gain`、`web-ui-param-demo`、`vanilla-ui-param-demo`、`vue-ui-param-demo`、`svelte-ui-param-demo`、`midi-synth` 和 `web-ui-instrument`。
- `vesty new --template <id>` 已支持由 starter 提供默认 `kind/ui`，并允许显式 `--kind` / `--ui` 覆盖 starter 默认值；无 `--template` 时保持 `effect + react` 默认行为。
- 新增 CLI 单测 `every_builtin_project_template_generates_expected_files`，遍历全部内置 starter，确保每个模板至少生成 Cargo/Vesty config、README、Rust source、参数 specs/manifest，并按 UI 类型生成或省略 UI 文件。
- `vesty new path/to/my-plugin` 现在只用路径最后一级目录名作为项目/plugin/crate 名称；目标路径仍决定落盘位置，避免把 `path/to/` 写入 VST3 plugin display name、Rust type name、bundle id 或 README。新增 `create_project_uses_leaf_directory_as_project_name` 覆盖该行为。
- Web UI 模板现在跟随 plugin kind 绑定 starter 主参数: effect UI 继续使用 `gain`，instrument UI 使用 `volume`。`web-ui-instrument` 生成的 React UI 会调用 `useVestyParamEdit("volume")`，不再把 gesture 发给不存在的 `gain` 参数；全量 starter 测试会检查 UI 源码包含对应主参数并拒绝 instrument UI 中残留的 `gain` bridge 调用。
- Web UI 模板现在用 `bridge.ready()` 返回的 `BridgeReadyPayload.params[].defaultNormalized` 初始化 starter slider，并订阅 `param.changed` 同步 host/controller/UI 确认后的参数值；`PluginSnapshot` 只用于 config/ui state revision，不作为当前参数值容器。
- 本机 smoke 已用当前 workspace path 生成全部 7 个内置 starter 临时项目，并分别通过独立 `cargo check`。
- 5 个带 Web UI 的 starter (`web-ui-param-demo`、`vanilla-ui-param-demo`、`vue-ui-param-demo`、`svelte-ui-param-demo`、`web-ui-instrument`) 均通过生成项目内的 `npm install`、`npm run build` 和 `npm run typecheck`，验证 `@vesty/plugin-ui` 本地 file dependency、React/Vue/Svelte adapter file dependency、Vite build、TypeScript/template checker 配置、effect/instrument UI 参数绑定、ready default 初始化和 `param.changed` 订阅可用。
- 该 smoke 证明内置模板的开发者起步路径可用；第三方模板 registry、远程模板下载、模板签名/缓存和外部 DAW 加载仍是后续工作，不作为当前 release evidence。

追加 2026-06-10 针对 Web UI starter ready 参数默认值 + `param.changed` 同步复跑:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ui_templates_emit_framework_specific_files -- --nocapture
rtk cargo test -p vesty-cli every_builtin_project_template_generates_expected_files -- --nocapture
rm -rf target/template-ready-param-smoke
rtk cargo run -p vesty-cli -- new target/template-ready-param-smoke/react-effect --template web-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
rtk cargo run -p vesty-cli -- new target/template-ready-param-smoke/vanilla-effect --template vanilla-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
rtk cargo run -p vesty-cli -- new target/template-ready-param-smoke/vue-effect --template vue-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
rtk cargo run -p vesty-cli -- new target/template-ready-param-smoke/svelte-effect --template svelte-ui-param-demo --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
rtk cargo run -p vesty-cli -- new target/template-ready-param-smoke/web-instrument --template web-ui-instrument --vesty-path crates/vesty --plugin-ui-path packages/plugin-ui
rtk cargo check --manifest-path target/template-ready-param-smoke/react-effect/Cargo.toml
rtk cargo check --manifest-path target/template-ready-param-smoke/vanilla-effect/Cargo.toml
rtk cargo check --manifest-path target/template-ready-param-smoke/vue-effect/Cargo.toml
rtk cargo check --manifest-path target/template-ready-param-smoke/svelte-effect/Cargo.toml
rtk cargo check --manifest-path target/template-ready-param-smoke/web-instrument/Cargo.toml
rtk npm install --prefix target/template-ready-param-smoke/react-effect/ui
rtk npm run build --prefix target/template-ready-param-smoke/react-effect/ui
rtk npm run typecheck --prefix target/template-ready-param-smoke/react-effect/ui
rtk npm install --prefix target/template-ready-param-smoke/vanilla-effect/ui
rtk npm run build --prefix target/template-ready-param-smoke/vanilla-effect/ui
rtk npm run typecheck --prefix target/template-ready-param-smoke/vanilla-effect/ui
rtk npm install --prefix target/template-ready-param-smoke/vue-effect/ui
rtk npm run build --prefix target/template-ready-param-smoke/vue-effect/ui
rtk npm run typecheck --prefix target/template-ready-param-smoke/vue-effect/ui
rtk npm install --prefix target/template-ready-param-smoke/svelte-effect/ui
rtk npm run build --prefix target/template-ready-param-smoke/svelte-effect/ui
rtk npm run typecheck --prefix target/template-ready-param-smoke/svelte-effect/ui
rtk npm install --prefix target/template-ready-param-smoke/web-instrument/ui
rtk npm run build --prefix target/template-ready-param-smoke/web-instrument/ui
rtk npm run typecheck --prefix target/template-ready-param-smoke/web-instrument/ui
rtk cargo test --workspace -j1
rtk npm test
rtk cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --check --strict
rtk cargo run -p vesty-cli -- release-check --format json --strict --report target/release-check-template-gallery.json
```

复跑结果:

- `ui_templates_emit_framework_specific_files` 现在固定要求 vanilla/React/Vue/Svelte 模板导入 `BridgeReadyPayload` / `ParamChangedEvent`、读取 `ready.params[].defaultNormalized`、订阅 `param.changed`、按 `PARAM_ID` 过滤事件，并继续保留 pointer capture/cancel cleanup。
- 5 个 Web UI starter 的 `cargo check`、`npm install`、`npm run build` 和 `npm run typecheck` 全部通过；`web-ui-instrument` 生成的 UI 使用 `volume`，effect 模板使用 `gain`。
- `cargo test --workspace -j1` 通过 419 个 Rust tests，`npm test` 通过，`smoke-host --check --strict` 通过。
- `release-check --format json --strict` 继续按预期失败，失败项仅为真实 DAW matrix / platform / signing / notarization 等外部 evidence 缺失，不是模板或本地框架回归。

本次 2026-06-09 针对 VST3 Note Expression value/int/text event / metadata MVP:

```bash
cargo fmt --all --check
cargo test -p vesty-core event_sample_offset -- --nocapture
cargo test -p vesty-core note_expression -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings processor_translates_automation_midi_and_transport -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings note_expression -- --nocapture
cargo check -p vesty-core -p vesty-vst3 -p vesty -p vesty-cli -j1
cargo test --workspace -j1
npm test
```

新增行为:

- `vesty-core::Event` 新增 `NoteExpressionValue { sample_offset, type_id, note_id, value }`、`NoteExpressionInt { sample_offset, type_id, note_id, value }` 和 `NoteExpressionText { sample_offset, type_id, note_id, text_len, text }`，`Event::sample_offset()` 覆盖这些 variants，避免 event sort/automation helper 漏掉新事件类型。
- `vesty_core::note_expression` 新增 VST3 standard type id 常量: `VOLUME`、`PAN`、`TUNING`、`VIBRATO`、`EXPRESSION`、`BRIGHTNESS`、`TEXT`、`PHONEME`、`CUSTOM_START`、`CUSTOM_END` 和 `INVALID`。
- `MAX_NOTE_EXPRESSION_TEXT_UNITS = 64` 固定了 realtime path 中 Note Expression text payload 的 UTF-16 code unit 上限；VST3 adapter 将 host 指针内容复制进固定 `[u16; 64]` buffer，并用 `text_len` 表示实际可读长度。
- `vesty-core` 新增 `NoteExpressionValueType` / `NoteExpressionValueFlags` 静态 metadata 描述，`Plugin::note_expression_value_types()` 默认返回空 slice，现有插件不会自动声明 Note Expression 支持。
- `vesty-vst3` 在 `collect_input_events()` 中识别 VST3 `kNoteExpressionValueEvent`、`kNoteExpressionIntValueEvent` 和 `kNoteExpressionTextEvent`，把 `typeId`、`noteId` 和 value/text payload 转成 core event；non-finite normalized value 会收敛为 `0.0`。
- fake COM 测试把 Note Expression value/int/text events 混入参数自动化、NoteOn/Off、PolyPressure、MIDI CC、PitchBend 和 ChannelPressure，验证同 offset 下保持收集顺序并进入 developer `ProcessContext::events()`。
- `vesty-vst3` controller 新增 `INoteExpressionController` interface；仅 instrument 的 event input bus `0`、channel `-1..=15` 可查询开发者 opt-in 的有效 value expression metadata。`getNoteExpressionStringByValue()` / `getNoteExpressionValueByString()` 提供 conservative normalized 数字格式化/解析。
- `vesty-vst3` controller 新增 `INoteExpressionPhysicalUIMapping` interface；仅 instrument event input bus `0`、channel `-1..=15` 可查询开发者 opt-in 的有效 static physical UI mapping metadata，且 mapping 必须指向已声明的有效 expression type。
- fake COM 测试覆盖 `INoteExpressionController` count、info、invalid bus/channel/index、value->string、string->value 和 invalid type id，也覆盖 `INoteExpressionPhysicalUIMapping` 正常查询、host 容量截断、invalid bus/channel/list/null map。
- 当前仍不实现自定义 expression editor workflow 或真实 DAW expression evidence。

本次 2026-06-09 针对 VST3 SysEx data event MVP:

```bash
cargo fmt --all --check
cargo test -p vesty-core event_sample_offset -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings sysex -- --nocapture
cargo test -p vesty-vst3 --features vst3-bindings processor_translates_automation_midi_and_transport -- --nocapture
```

新增行为:

- `vesty-core::Event` 新增 `SysEx { sample_offset, data_len, data, truncated }`，`Event::sample_offset()` 覆盖该 variant。
- `MAX_SYSEX_BYTES = 256` 固定 realtime path 中 SysEx payload 的 byte 上限；VST3 adapter 将 host `DataEvent.bytes` 复制进固定 `[u8; 256]` buffer，开发者只读取 `data[..data_len as usize]`。
- `vesty-vst3` 在 `collect_input_events()` 中识别 VST3 `kDataEvent` 且 data type 为 `kMidiSysEx` 的事件；其它 VST3 data event 仍忽略。
- 超过固定 buffer 的 SysEx payload 会被截断并设置 `truncated = true`；host 声明非零 size 但 bytes 为空时也返回空 payload + truncated，避免把 host 裸指针传给 DSP。
- fake processor 测试把 SysEx 混入参数自动化、NoteOn/Off、PolyPressure、MIDI CC、PitchBend、ChannelPressure 和 Note Expression events，验证 stable sample-order sort 和 developer `ProcessContext::events()` 可见 payload。
- 当前仍未完成真实 DAW SysEx workflow evidence，也不代表 MIDI 2.0 mapping/UMP 已实现。

本次 2026-06-09 针对 JSBridge ready capabilities native gate:

```bash
cargo fmt --all --check
cargo test -p vesty-bridge capabilities -- --nocapture
cargo test -p vesty-bridge subscription -- --nocapture
cargo test -p vesty-bridge -j1
```

新增行为:

- `BridgeReadyPayload.capabilities` 现在由 Rust `BridgeRuntime` 在 native dispatch 前强制执行，不再只是给 UI feature gating 的描述字段。
- 关闭 `paramGestures` 会拒绝 `param.begin` / `param.perform` / `param.end`；关闭 `paramFormatParse` 会拒绝 `param.format` / `param.parse`；关闭 `stateConfig` 会拒绝 `state.setConfig` / `state.setUiState`；关闭 `subscriptions` 会拒绝 `subscription.add` / `subscription.remove`；关闭 `meterStream` 会拒绝 `meter.flush`，并禁止 `meter.*` topic 订阅和 latest-wins meter queue；关闭 `diagnostics` 会拒绝 `diagnostics.get`、`diagnostics.fault` / `log.rt` topic 和 RT log emission；关闭 `reliableEvents` 会拒绝除 meter/diagnostics/log 之外的 reliable topic 订阅，并抑制对应 native event emission。
- 被 capability gate 拒绝的 request 返回 non-retryable `unsupported_type`，且不会污染 subscription table、pending param gesture queue、state snapshot 或 meter queue。
- 新增 `disabled_bridge_capabilities_reject_request_classes`、`disabled_topic_capabilities_reject_matching_subscriptions` 和 `disabled_topic_capabilities_suppress_native_events_and_meter_queue` 单测，覆盖 request class、topic-specific add/remove、native event emission 和 meter queue suppression。
- 当前 VST3/wry endpoint 仍默认发布 `BridgeCapabilities::v1_default()`，所以现有 examples 行为不变；该 gate 是为后续 host/profile degraded mode 或精简 runtime 提供后端强约束。

本次 2026-06-10 针对完整计划落地状态和本机可验证 gate 复核:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -j1
npm test
cargo run -p vesty-cli -- --help
cargo run -p vesty-cli -- doctor --format json
cargo run -p vesty-cli -- release-check --format json --strict
cargo check --workspace -j1
rm -rf target/vesty-protocol-audit
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-audit
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-audit --check
```

复核结果:

- `cargo test --workspace -j1` 通过，当前覆盖 430 个 Rust unit/doc tests，包括 `vesty` facade、bridge/build/cli/core/ipc/macros/params/rt/ui-wry/vst3/vst3-sys 和三个 examples。
- `cargo clippy --workspace --all-targets -- -D warnings` 通过，当前 README 中列出的 workspace clippy gate 可作为真实本地质量门禁。
- `npm test` 通过，覆盖 `@vesty/plugin-ui` bridge/protocol 测试以及 React/Svelte/Vue adapter export tests。
- `cargo check --workspace -j1` 通过，确认清理并重建 `target/debug` 后 workspace 仍可完整编译。
- `vesty doctor --format json` 在当前 macOS 环境中报告 rustc/cargo/node/npm、VST3 binding baseline、macOS WebKit、codesign/notarytool 和 REAPER install detection 为 ok；Steinberg validator 未在当前 PATH/`VST3_VALIDATOR` 中发现，Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One 未安装。
- `vesty release-check --format json --strict` 按预期失败，失败点是缺真实 DAW smoke evidence；REAPER 只检测到安装和 scan，仍缺 load/UI/UI->Host/meter stream/automation/buffer-sample-rate change/save-restore/offline render 当前 strict evidence，其它四个 DAW 全部 smoke evidence 缺失。该失败证明 release gate 没有把安装检测或模板当作发布通过证据。
- `vesty export-types` 重新生成到 `target/vesty-protocol-audit` 后，`--check` 通过，证明 Rust IPC/params schema 到 TypeScript/JSON Schema snapshot 的导出链路可复验。
- 复核过程中一度因本机磁盘仅剩约 145 MiB 导致 Cargo 无法写 `target/debug`；已只清理可再生的 `target/debug` 构建产物，保留 release evidence/report 目录，之后 workspace check 和 protocol export/check 均恢复通过。

本次 2026-06-10 针对 workspace clippy quality gate:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings wry-ui" -- -D warnings
cargo test -p vesty-ui-wry --features wry-backend
cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings
cargo test --workspace -j1
npm test
```

新增/调整:

- `NoteExpressionValueType::new()` 从长参数构造器收敛为 `new(type_id, title, short_title)`，并通过 `with_units()`、`with_range()`、`with_step_count()`、`with_flags()` 组合 optional metadata，避免 developer API 继续暴露 clippy `too_many_arguments` 形状。
- `examples/midi-synth` 和 `.agents/08-developer-guide.md` 的 Note Expression metadata 示例已迁移到链式 API。
- 修复 `vesty-bridge` capability gate 测试和 param parse 路径中的 clippy findings，修复 `vesty-cli` workspace dependency coverage/template/import-ci 测试中的 mechanical findings。
- `vesty-vst3` f64 scratch path 改为等价 iterator loop，并为 VST3 host raw pointer helper 补齐 clippy 要求的 `SAFETY:` 注释；去掉无效 raw pointer cast。
- GitHub Actions `rust` job 现在在 feature tests 旁边显式运行 `cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings wry-ui" -- -D warnings` 和 `cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings`，防止 feature-only VST3/UI backend code 绕过 lint。
- 验证结果: `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test -p vesty-vst3 --features "vst3-bindings wry-ui"`、`cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings wry-ui" -- -D warnings`、`cargo test -p vesty-ui-wry --features wry-backend`、`cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings`、`cargo test --workspace -j1` 和 `npm test` 全部通过。

本次 2026-06-10 针对本地 CI/release subchain 聚合:

```bash
rtk npm run typecheck
rtk npm run build
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol-ci-local
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol-ci-local --check
rtk zsh -c 'npx tsc --strict --noEmit --target ES2022 --module ESNext --moduleResolution Bundler $(find target/vesty-protocol-ci-local/typescript -name "*.ts" | sort)'
rtk cargo run -p vesty-cli -- publish-plan --out target/publish-plan-ci-local/publish-plan.json
rtk cargo run -p vesty-cli -- publish-plan --check --out target/publish-plan-ci-local/publish-plan.json
rtk cargo run -p vesty-cli -- npm-pack --out target/npm-pack-ci-local/npm-pack.json
rtk cargo run -p vesty-cli -- npm-pack --check --out target/npm-pack-ci-local/npm-pack.json
rtk cargo run -p vesty-cli -- dependency-baseline --out target/dependency-baseline-ci-local/dependency-baseline.json
rtk cargo run -p vesty-cli -- dependency-baseline --check --out target/dependency-baseline-ci-local/dependency-baseline.json
rtk cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline-ci-local/dependency-baseline-latest.json --format text
rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-ci-local/dependency-baseline-latest.json --format text
rtk cargo run -p vesty-cli -- crate-package --out target/crate-package-ci-local/crate-package.json
rtk cargo run -p vesty-cli -- crate-package --check --out target/crate-package-ci-local/crate-package.json
rtk cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-ci-local --protocol-snapshot target/vesty-protocol-ci-local --crate-package --dependency-baseline-latest --format json
rtk cargo run -p vesty-cli -- release-check --format json --strict --protocol-snapshot target/vesty-protocol-ci-local --publish-plan-report target/publish-plan-ci-local/publish-plan.json --crate-package-report target/crate-package-ci-local/crate-package.json --npm-pack-report target/npm-pack-ci-local/npm-pack.json --dependency-baseline-report target/dependency-baseline-ci-local/dependency-baseline-latest.json --report target/release-check-local-ci-subchain.json
rtk cargo run -p vesty-cli -- release-check --format json --strict --release-evidence-dir target/release-evidence-ci-local --protocol-snapshot target/vesty-protocol-ci-local --report target/release-check-local-ci-evidence-dir.json
```

复核结果:

- JS SDK/adapters 的 `typecheck`、`build` 和 `npm test` 均通过；导出的 protocol TypeScript snapshot 也通过独立 `tsc --strict --noEmit`。
- `export-types --check`、`publish-plan --check`、`npm-pack --check`、`dependency-baseline --check`、`dependency-baseline --latest --check` 和 `crate-package --check` 全部通过。
- dependency latest report 当前包含 44 个 baseline checks 和 29 个 crates.io/npm registry latest checks，并包含 `cargo workspace external dependency baseline coverage`，因此 release gate 不会接受缺 workspace 覆盖的 latest 报告。`DependencyBaselineReport` parser 还会复验 top-level/check status、expected/actual 自一致性和 unknown status，避免手写 `ok` latest evidence 绕过 registry drift。
- `release-evidence collect-local` 成功生成 `target/release-evidence-ci-local`: protocol snapshot 为 22 个 TypeScript files / 7 个 JSON schema files；publish plan 为 13 个 publishable crates / 3 个 private skipped；crate package readiness 为 3 个 packageable now / 10 个 deferred；npm pack 为 4 packages / 58 files；dependency latest baseline 为 44 baseline checks / 29 latest registry checks。
- 直接传入本地 reports 的 strict `release-check` 和通过 `--release-evidence-dir target/release-evidence-ci-local` 自动发现的 strict `release-check` 都按预期失败，但本地可验证项均为 ok: protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline。失败/跳过项仍是 DAW matrix、platform smoke、三示例/三平台 validator/static validate、真实 GitHub Actions artifacts、签名和 notarization 等外部 evidence。

本次 2026-06-10 针对 release action plan 采证辅助:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --release-evidence-dir target/release-evidence-ci-local --protocol-snapshot target/vesty-protocol-ci-local --plan target/release-action-plan-local-ci.json --report target/release-check-local-ci-with-plan.json
```

新增/调整:

- `vesty release-check` 新增 `--plan <path>`，会从当前 `ReleaseCheckReport` 派生机器可读 `ReleaseActionPlan`，列出所有 failed/skipped checks、priority、当前 value/hint、evidence path 和建议采证命令。
- `--plan` 不改变 release-check pass/fail，不会把 skipped/pending 当作 pass evidence；它只是把真实 DAW、platform smoke、CI artifacts、validator/static validate、签名、notarization 和可选 VST3 SDK audit 的下一步采证动作写成 JSON。
- `.github/workflows/ci.yml` 的 per-OS release readiness step 现在同时上传 `release-check-<OS>.json` 和 `release-action-plan-<OS>.json`；后者用于下载 CI artifacts 后查看剩余采证动作。`--ci-release-check-dir` 的收集器只采纳 `release-check*.json`，会忽略同目录 action-plan sidecar，避免把 checklist 当成 per-OS report。
- 新增测试验证 action plan 中每个 DAW `--host` 参数都能被 `write_daw_smoke_report()` 接受，并验证 CI release-check artifact parser 在同目录存在 `release-action-plan-*.json` 时仍只读取三份 OS report。
- `release-evidence import-ci` 只会保存完整的 per-OS action plan sidecar；sidecar action 必须有合法 status/priority、自洽 summary、非空且无控制字符的 check/value/hint/evidence path/command，并且每个 action 至少包含一条建议命令。`status = "failed"` 的 sidecar 还必须包含至少一个 action，且至少一个 action 处于 failed/skipped，避免把空 checklist 或全 `ok` checklist 伪装成失败态采证清单。损坏 checklist 会被记录为 failed，不会写入 `ci-release-checks/`。
- `release-evidence` 模板 README、`.agents/07-build-packaging.md`、`.agents/08-developer-guide.md` 和 `README.md` 已说明 `--plan` 的用途和边界。
- 本机实跑写出 `target/release-action-plan-local-ci.json`，当前 summary 为 7 ok、6 failed、13 skipped、19 actions；strict `release-check` 仍按预期失败在真实外部 evidence 缺失上，但 action plan 文件已落盘并给出可执行采证清单。

本次 2026-06-10 继续收口 `release-evidence import-ci` 对 action plan sidecar 的处理:

- `release-evidence import-ci` 现在会识别并内容验证 `ReleaseActionPlan` JSON，合法的 per-OS sidecar 会复制到 `ci-release-checks/release-action-plan-<OS>.json`，作为下载 CI artifacts 后的采证清单留档。
- 无效 sidecar 会在 `import-ci-report.json` 中记录为 failed 且不会落盘；校验会拒绝 version/status/summary 计数、failed/skipped 数量、action status/priority 组合、空 check/value、空 failed plan 或 failed plan 中没有 failed action 的不一致 plan。
- 这些 sidecar 仍然只是 checklist/audit metadata；`--ci-release-check-dir` 只读取 `release-check*.json`，不会把 `release-action-plan-*.json` 当作 per-OS release-check report，也不会影响 release-check pass/fail。

本次 2026-06-10 针对 multi-output instrument MVP:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-core audio_output -- --nocapture
rtk cargo test -p vesty-vst3 multi_output --features "vst3-bindings" -- --nocapture
rtk cargo test -p vesty-vst3 bus --features "vst3-bindings" -- --nocapture
rtk cargo test -p vesty-vst3 --features "vst3-bindings" -- --test-threads=1
rtk cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings" -- -D warnings
```

新增/调整:

- `vesty-core` 新增 `AudioOutputBus`、`MAX_AUDIO_OUTPUT_BUSES`、`MAX_AUDIO_OUTPUT_CHANNELS` 和 `Plugin::output_buses()`；默认仍是单 stereo `"Output"`，保持现有插件兼容。
- `vesty-vst3` 现在按 `Plugin::output_buses()` 暴露 output bus: bus 0 为 main/default active，后续 bus 为 aux/non-default-active；instrument 可声明多个 mono/stereo output bus。
- VST3 sample32、native sample64 和 sample64 f32 scratch fallback path 都会把多个 output bus 展平成 fixed-capacity channel list，不在 realtime process path 分配；scratch fallback 仍只使用 `setupProcessing()` 预分配容量，容量不足时清零并设置 silence flags。
- `process()` 阶段允许 host 只传入声明 output bus 的前缀，或为 trailing inactive aux output 传入 `numChannels = 0`；主 output 仍必须有效，已传入的 aux/instrument output bus 仍按声明声道严格校验。这避免未激活 aux bus 的宿主形态让 main bus 整块处理被跳过。
- Effect 主 output 保持旧的 mono/stereo host buffer 容错和 mono->mono、mono->stereo、stereo->stereo negotiation；aux/instrument output bus 按声明声道严格校验。
- fake COM 测试新增双 stereo output instrument，覆盖 main/aux bus metadata、arrangement 校验、invalid output count/mono aux 拒绝、`IUnitInfo::getUnitByBus()` 对 aux output bus 的 root-unit 映射、sample32 process、sample64 f32 scratch fallback、native sample64 `process_f64()` 写入 main/aux 四个通道、host 只传 main output bus、host 传 empty inactive aux output bus，以及 realtime allocation guard 0 allocation。
- `IPluginFactory` boundary 现在会拒绝 null output/class/interface pointers；`createInstance()` 会先清空可写 output pointer，保证失败路径不把 stale instance 留给 host。
- `IComponent::getControllerClassId()` 现在会拒绝 null output pointer，避免 validator 或异常 host 的 COM boundary probe 触发空指针写入；正常调用仍写入 controller CID。
- `IEditController` 参数 metadata/format/parse callbacks 现在会拒绝 null host pointers 和负参数 index；invalid parse input 不会改写 caller 的 normalized output。
- 参数 parse 和 Note Expression parse 现在把 host `String128` 输入限制在 128 个 UTF-16 单元内读取，避免异常 host 提供非 NUL 结尾字符串时越界扫描。
- `IPlugView::attached()` 现在会拒绝 supported platform 下的 null native parent handle；`isPlatformTypeSupported(nullptr)` 返回 `kResultFalse`。
- `process()` 现在在构造 input bus slice 前校验 `numInputs` 不为负、不超过插件声明 input bus 数量，并要求非零 input count 搭配非 null pointer；异常 host input shape 会静音输出且不进入 developer kernel。
- `process()` 现在在构造 per-bus channel pointer slice 前校验 `AudioBusBuffers::numChannels`；main/sidechain input 超出固定上限时 DSP 会看到空输入，output bus 声道数不符合 effect mono/stereo 或插件声明 output layout 时会拒绝本次 layout 且不进入 developer kernel。
- `process()` 现在会拒绝负 `numSamples`；异常负 block size 不进入事件收集或 developer kernel，并设置 silence flags。超过 sample64 scratch capacity 的默认 f64 fallback 仍由 sample64 path 静音处理，native f64 opt-in path 保持可处理 host 大 block 的现有行为。
- Optional sidechain process path 现在也覆盖 host 只传 main input bus、以及 host 传 trailing empty inactive sidechain input bus 两种形态；DSP 会看到空 `ProcessContext::sidechain()`，main input/output 仍正常处理，且 `NoAllocGuard` 下 0 allocation。
- `IComponent::setIoMode()` 现在校验并记录标准 VST3 IO mode: `kSimple`、`kAdvanced`、`kOfflineProcessing` 返回 `kResultOk`，未知 mode 返回 `kInvalidArgument` 且不改变上一有效值；`process()` 的 block 级 realtime/offline/prefetch 行为仍由 `ProcessData.processMode` 驱动。
- `IAudioProcessor::setProcessing(false)` 现在会记录显式非处理状态；异常 host 后续调用 `process()` 时会在 `NoAllocGuard` 下清零输出、设置 silence flags、返回 `kResultOk`，且不会进入 developer kernel。`setProcessing(true)` 会恢复正常处理；默认初始状态保持 active 以兼容未显式调用该 lifecycle 的 host。
- `IAudioProcessor::setupProcessing()` 现在会拒绝 null pointer、unsupported sample size、非有限/非正 sample rate、非正或异常大的 `maxSamplesPerBlock`，返回 `kInvalidArgument`；fake COM 测试验证这些无效 setup 不会创建 kernel、不会调用 `prepare()`，也不会为 sample64 scratch 预分配异常大小。
- 真实 DAW 中的多输出 instrument routing、bus activation、save/restore、offline render 和 host automation 仍需外部 smoke evidence。

本次 2026-06-10 针对参数句柄初始化防崩溃补强:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-params param_handle -- --nocapture
rtk cargo test -p vesty-cli template -- --nocapture
rtk cargo check -p vesty-example-gain -p vesty-example-web-ui-param-demo -p vesty-example-midi-synth
```

新增/调整:

- `vesty-params::ParamHandle` 新增 `INVALID_INDEX`、`invalid()` 和 `is_invalid()`；`ParamCollection` 新增 `resolve_or_invalid(id)`。未知参数 ID 会得到 invalid handle，`get_normalized_by_handle()` 返回 `None`，`set_normalized_by_handle()` 返回 `ParamError::Unknown("handle:<usize::MAX>")`，不会在 kernel 初始化阶段 panic。
- 三个内置 examples、`vesty new` effect/instrument Rust 模板、README 最小示例和开发文档均改用 `resolve_or_invalid()`，避免示例继续鼓励在 host-facing `create_kernel()` 中用 `expect("param exists")`。
- 新增 `invalid_param_handle_is_safe_fallback` 单测，证明未知参数 handle fallback 不会越界读取或写入；模板测试和三个示例 `cargo check` 已通过。
- 这不是隐藏参数 schema 错误: VST3 adapter 和 JSBridge runtime 仍会在 processor/controller/ready store 建立前校验完整 `ParamSpec` schema；该 fallback 只降低开发者在 kernel 里写错字符串 ID 时的 host 初始化 panic 风险。

本次 2026-06-11 针对 VST3 validate release evidence 自洽性补强:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli validate_report -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli example_coverage -- --nocapture
rtk cargo test -p vesty-cli static_validate -- --nocapture
```

新增/调整:

- `vesty-cli` 的 `validate_release_validate_report()` 和 `validate_static_validate_report()` 现在先复用 shared self-consistency validator 校验 `static_check` 和 `validator` 字段组合。
- `static_check.status = "ok"` 必须有非空 moduleinfo、至少一个 platform binary、没有 stale `error`，并继续校验显式 `static_check.binary_exports`；`status = "failed"` 必须有 error，且不能同时携带 moduleinfo/binaries/binary_exports/parameter manifest/asset manifest/asset_count 等 positive evidence；未知 static status 会被拒绝。
- `validator.status = "passed"` 必须有非空 validator path、`exit_code = 0`、`tests_passed > 0`、`tests_failed = 0`，且不能携带 stale `reason` 或 `error`；`skipped` / `not_run` / `not_found` 必须有 reason 且不能携带 path/exit_code/test counts/stdout/stderr/error；未知 validator status 会被拒绝。
- ok static report 的 moduleinfo、binaries、binary export checks、parameter manifest 和 asset manifest 现在都必须归属 `ValidateReport.bundle` 声明的同一个 `.vst3` bundle；绝对路径和 `./Bundle.vst3/...` 相对路径被接受，但跨 bundle、suffix spoof、未列入 `static_check.binaries` 的 export check、asset manifest 与 `asset_count` 不一致都会被拒绝。
- 新增 `validate_report_rejects_inconsistent_static_check_fields`、`validate_report_rejects_inconsistent_validator_passed_fields`、`validate_report_rejects_skipped_validator_with_run_fields`、`validate_report_rejects_unknown_static_or_validator_status`、`validate_report_rejects_paths_from_other_bundle`、`validate_report_accepts_dot_prefixed_bundle_paths`、`validate_report_rejects_mismatched_binary_export_paths` 和 `validate_report_rejects_manifest_paths_from_other_bundle_or_bad_asset_count`，证明手写或篡改的 validate JSON 不能通过 release evidence import/discovery/gate。
- 该改动不改变真实外部 evidence 要求: strict release gate 仍需真实 DAW matrix、macOS/Windows/Linux platform smoke、三示例/三平台 Steinberg validator/static validate、GitHub Actions artifacts、签名和 notarization evidence。

本次 2026-06-11 针对 platform smoke release evidence marker 收紧:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
```

新增/调整:

- `vesty platform-smoke` 的 `system_webview` evidence 现在按平台校验: macOS 必须包含 `WebKit.framework` 或 `WKWebView`，Windows x64 必须包含 `WebView2`，Linux X11 必须同时包含 `WebKitGTK` 和 `X11`；`system_webview=true` / `webview=true` 这类泛 marker 会被拒绝。
- `vst3_validator` evidence 现在必须识别 Steinberg/VST3 validator 输出，并包含 passed tests 与 0 failed 语义；单独的 `passed`、`vst3_validator=true`、非 Steinberg/VST3 checker、0 passed 或非零 failed 都会被拒绝。
- release action plan 的 `platform smoke artifacts` 建议命令改为 macOS、Windows x64、Linux X11 三条平台特异示例，并新增回归断言，防止后续退回泛 true marker。
- `platform_smoke_requires_platform_specific_webview_evidence`、`platform_smoke_requires_validator_identity_and_zero_fail_summary` 和 `platform_smoke_accepts_alternate_system_webview_and_validator_markers` 覆盖了拒绝和接受路径；`.agents/07-build-packaging.md`、`.agents/08-developer-guide.md`、`.agents/14-completion-audit.md` 与 README 已同步该证据口径。

本次 2026-06-11 针对 DAW matrix write-report evidence marker 收紧:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli daw_matrix -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current.json
```

新增/调整:

- `vesty daw-matrix --write-report` 的 `--platform` 现在会拒绝模板占位 `manual platform pending`，避免把未采集平台/host 版本的 evidence 写成通过目录。
- DAW smoke marker 写入前现在会拒绝明显负面语义，如 `failed`、`not found`、`not installed`、`unavailable`、`timeout`、`crashed` 等；`render_file=...` 路径行被排除在该负面词扫描之外，避免误伤真实渲染文件名。
- 既有 pending/false/占位值、zero meter 和无法被 DAW matrix parser 识别的模糊 marker 拒绝逻辑保持不变；写入后仍复用 parser 验证 host 行完整。
- `daw_matrix_write_report_rejects_pending_or_zero_meter` 增加 pending platform 和负面 scan evidence 覆盖；`.agents/07-build-packaging.md` 与 `.agents/08-developer-guide.md` 已同步该证据口径。
- 本地验证通过: `daw_matrix` 4 passed、`release_check` 33 passed、workspace Rust tests 470 passed、workspace clippy no issues、JS workspace tests passed。strict `release-check` 仍按预期失败在真实 DAW/platform/CI/validator/signing/notarization evidence 缺失；本地 invariant 中 host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok。

本次 2026-06-11 针对 release evidence 目录固定路径 symlink 收紧:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk cargo test -p vesty-cli symlink -- --nocapture
```

新增/调整:

- `release-check --release-evidence-dir` 的模板标准路径自动发现现在通过 `symlink_metadata()` 逐段检查固定文件和中间目录，拒绝 symlinked `publish-plan/`、`crate-package/`、`npm-pack/`、`dependency-baseline/`、`vst3-sdk/`、`validate-report.json`、`static-validate-report.json`、`signing-macos.log`、`signing-windows.log` 和 `notary.log` 等标准槽位。
- `ci-doctor/`、`ci-release-checks/` 和 `platform-smoke/` 的目录自动发现不再吞掉递归扫描中的 symlink 错误；模板目录缺失或没有 JSON 仍保持 skipped 语义。
- 新增 `release_evidence_dir_rejects_standard_file_symlink` 和 `release_evidence_dir_rejects_standard_dir_symlink`，证明标准 evidence 文件和中间目录不能指向 evidence bundle 外部的可替换文件。
- 该改动只收紧 `--release-evidence-dir` 的自动发现边界；显式传入的单项 evidence path 仍由对应 parser/validator 校验。它不生成外部 pass evidence，strict release gate 仍需真实 DAW、platform、CI、validator、签名和 notarization artifact。

本次 2026-06-11 针对 release evidence root 和 import-ci root symlink 继续硬化:

- `release-check --release-evidence-dir <dir>` 现在会拒绝 `<dir>` 本身是 symlink 的情况，避免整包 release evidence root 被替换到外部目录。
- `vesty release-evidence import-ci --source <dir>` 要求 source root 是已存在的真实目录且不是 symlink，避免 artifact staging 根目录在导入前被替换。
- `vesty release-evidence import-ci --dir <dir>` 如果目标目录已存在，会拒绝 symlink；如果目标目录缺失，只会通过真实父目录创建标准 release evidence 目录，避免缺失 output root 经 symlinked parent 落到外部。
- `import_copy_file()`、`import_write_text_file()` 和 `import_copy_dir_contents()` 现在在导入写入/复制前还会逐段拒绝 symlinked destination parent。
- 新增 `release_evidence_dir_rejects_root_symlink`、`import_ci_release_evidence_rejects_symlink_source_root`、`import_ci_release_evidence_rejects_existing_symlink_output_dir`、`import_ci_release_evidence_rejects_symlink_output_parent` 和 `import_ci_writers_reject_symlink_output_parents`，覆盖上述 root-level、output-parent 与 destination-parent 信任边界。

本次 2026-06-11 继续收紧显式递归 artifact root:

- `--ci-doctor-dir`、`--ci-release-check-dir` 和 `--platform-smoke-dir` 现在在读取前统一拒绝 symlinked file/directory root；单文件 evidence 仍被允许，但文件本身也不能是 symlink。
- 新增 `ci_doctor_artifacts_reject_symlink_root`、`ci_release_check_artifacts_reject_symlink_root` 和 `platform_smoke_release_check_rejects_symlink_root`，证明显式传入的递归 evidence 目录不会跟随到外部可替换树。

本次 2026-06-11 继续收紧显式 release evidence file/log:

- `--validate-report`、`--static-validate-report`、`--publish-plan-report`、`--crate-package-report`、`--npm-pack-report`、`--dependency-baseline-report`、`--vst3-sdk-manifest`、`--vst3-sdk-binding-plan`、`--vst3-sdk-binding-surface`、`--signed-bundle-evidence` 和 `--notarization-log` 等显式 file/log evidence 在读取前都会先拒绝 symlink，避免 release-check 跟随到 evidence bundle 外部可替换文件。
- `vesty release-evidence collect-notarization --notary-log <path> --stapler-log <path>` 的输入日志也要求是真实文件而不是 symlink；`collect-signing` / signing evidence parser 对显式签名 log 文件同样拒绝 symlink，`.vst3` bundle evidence root 仍按真实 file/directory root 校验。
- 新增 `release_check_rejects_symlink_validate_report`、`signing_evidence_rejects_symlink_log` 和 `collect_notarization_release_evidence_rejects_symlink_input_logs`，覆盖显式 validate report、签名 log 与 notarization 输入 log 的 no-follow 行为。
- 该改动只收紧本地 release evidence 信任边界，不生成新的外部 pass evidence；strict release gate 仍需真实 DAW、platform、CI、validator、签名和 notarization artifact。

本次 2026-06-11 继续收紧 CLI report/log 输出文件:

- 通用 `write_text_file()` 现在会在写入前拒绝既有 symlink 输出文件，并逐段拒绝用户可替换的 symlink 输出父目录；root-level 系统前缀 symlink（例如 macOS `/var` 或 `/tmp`）保持允许，避免误伤系统临时目录。
- `platform-smoke --write-report` 从裸 `fs::write()` 切换到同一 no-follow 输出 helper；`validate --report`、`validate --validator-log`、`release-check --report`、`release-check --plan`、`smoke-host --out`、`publish-plan --out`、`crate-package --out`、`npm-pack --out`、`dependency-baseline --out` 和 DAW smoke marker 写入均继承同一行为。
- 新增 `report_writers_reject_symlink_output_files`、`validator_log_writer_rejects_symlink_output_file`、`platform_smoke_write_report_rejects_symlink_output_file`、`release_check_report_writer_rejects_symlink_output_file` 和 `release_check_report_writer_rejects_symlink_output_parent`，同时断言外部目标内容未被覆盖。
- 该改动只防止本机采证输出被 symlink 劫持；不改变任何 release pass 条件。

本次 2026-06-11 继续收紧 evidence template 初始化:

- 通用 `write_template_file()` 现在用 `symlink_metadata()` 检查既有模板文件槽位，新增 `create_template_dir()` 对模板根目录、DAW host 目录、release evidence 标准子目录和创建路径中的已存在祖先目录执行同样的 no-follow 目录检查；普通文件/目录保持“不覆盖”语义，既有 symlink 文件、目录槽位或深层输出路径中的 symlink ancestor 会直接报错，避免 `daw-matrix --write-template`、`platform-smoke --write-template`、`release-check --write-evidence-template` 和 `release-evidence collect-local` 初始化 pending 模板时跟随到 evidence bundle 外部目标。
- 新增 `daw_evidence_templates_reject_symlink_slots`、`daw_evidence_templates_reject_symlink_host_dirs`、`platform_smoke_templates_reject_symlink_root_dir`、`release_evidence_templates_reject_symlink_slots`、`release_evidence_templates_reject_symlink_standard_dirs` 和 `evidence_template_dirs_reject_nested_symlink_output_parent`，覆盖 DAW smoke 模板、platform smoke 模板和 release evidence 模板的 no-follow 行为，并断言外部 symlink 目标内容未被覆盖。
- 该改动只收紧模板初始化的本地文件/目录边界；pending 模板仍不算任何 release pass evidence，strict release gate 仍需真实 DAW、platform、CI、validator、签名和 notarization artifact。

本次 2026-06-11 继续收紧 release Web UI asset runtime:

- `vesty-ui-wry` release asset manifest loader 现在先用 `symlink_metadata()` 验证 UI asset root 是真实目录且不是 symlink；`assets.manifest.json` 候选文件也通过 no-follow metadata 检查，既有 symlink manifest 会直接以 permission denied 失败，而不是跟随到 bundle 外部 manifest。
- custom protocol 已有的 manifest path allowlist、URL-safe path、manifest-listed asset symlink 拒绝、canonical root、size 和 sha256 校验保持不变；本次补强把信任边界前移到 manifest/root 加载阶段。
- 新增 `asset_manifest_rejects_symlinked_manifest_file` 和 `asset_manifest_rejects_symlinked_asset_root`，与既有 `asset_protocol_rejects_symlinked_manifest_assets` 共同覆盖 root、manifest 和 manifest-listed asset 三层 no-follow 行为。
- 该改动只收紧 release Web UI 本地 asset loading，不替代 Windows WebView2、Linux WebKitGTK/X11 或真实 DAW editor attach/resize evidence。

本次 2026-06-11 继续收紧 `vesty-build` UI asset package/validate:

- `AssetManifest::from_dir()`、`copy_dir_recursive()` 和 `package_vst3()` 现在都会用 no-follow directory helper 检查 UI dist root；`ui/dist` 本身是 symlink 时会返回 `BuildError::SymlinkAsset`，不会跟随到项目目录外部生成/复制 release assets。
- `validate_vst3_bundle()` 的 Web UI 静态检查现在用 no-follow presence/file/directory helper 检查 `Contents/Resources/ui` 和 `Contents/Resources/assets.manifest.json`；packaged UI root、manifest 文件或 manifest-listed asset 是 symlink 时都会失败。
- 新增 `asset_manifest_rejects_symlinked_root`、`package_rejects_symlinked_ui_dist_root`、`validate_rejects_symlinked_ui_asset_manifest_file` 和 `validate_rejects_symlinked_ui_asset_root`，与既有 `asset_manifest_rejects_symlinks` / `package_rejects_symlinked_ui_assets` 一起覆盖 build、package 和 static validate 三条路径。
- 该改动让 build/package/static validation 与 `vesty-ui-wry` release runtime 的 asset root / manifest / file no-follow 语义一致；仍不替代真实 WebView2/WebKitGTK 或 DAW editor evidence。

本次 2026-06-11 继续收紧 `vesty-build` config/binary/bundle metadata no-follow 边界:

- `read_config()`、`read_parameter_specs()` 和 `read_parameter_manifest()` 现在统一先用 no-follow file helper 检查输入，再读取 `vesty.toml`、参数 specs 或参数 manifest；symlinked config/sidecar 不会被跟随。
- `package_vst3()` 现在在校验 binary format 和复制 plugin binary 前拒绝 symlinked binary input；配置了 `[package].parameter_manifest` 时，source sidecar 同样必须是真实文件。
- `validate_vst3_bundle()` 现在拒绝 symlinked `.vst3` bundle root、`Contents`、`Resources`、`moduleinfo.json`、`parameters.manifest.json`、platform binary directory、macOS `Info.plist` 和 `PkgInfo`；platform binary format 读取也走同一 no-follow file helper。
- 新增 `read_config_rejects_symlinked_file`、`read_parameter_specs_rejects_symlinked_file`、`read_parameter_manifest_rejects_symlinked_file`、`package_rejects_symlinked_binary_input`、`package_rejects_symlinked_configured_parameter_manifest`、`validate_rejects_symlinked_bundle_root`、`validate_rejects_symlinked_moduleinfo_file`、`validate_rejects_symlinked_packaged_parameter_manifest`、`validate_rejects_symlinked_macos_metadata_files` 和 `validate_rejects_symlinked_platform_binary_dir`。
- 本地已通过 `rtk cargo test -p vesty-build symlink -- --nocapture` 和 `rtk cargo test -p vesty-build`；该改动只收紧本机构建/静态校验路径，仍不替代真实 DAW、平台 WebView、validator、CI、签名或 notarization evidence。

本次 2026-06-11 继续收紧 `vesty param-manifest --specs` 输入边界:

- `vesty-cli` 的 `run_param_manifest()` 现在通过 CLI no-follow file helper 读取 `--specs`，拒绝 symlinked parameter specs 输入；`--check --out` 读取既有 manifest 时继续走 `vesty-build::read_parameter_manifest()` 的 no-follow 校验。
- 新增 `param_manifest_rejects_symlinked_specs_input`，并已通过 `rtk cargo test -p vesty-cli param_manifest -- --nocapture` 与 `rtk cargo test -p vesty-cli symlink -- --nocapture`。
- 该改动只收紧本地参数 sidecar 生成/检查入口，不改变 release pass 条件。

本次 2026-06-11 继续收紧 `vesty-cli` diagnostic/dependency no-follow 输入边界:

- CLI 通用 `read_toml_file()` / `read_json_file()` 现在通过 `read_text_file_no_symlink()` 读取 TOML/JSON 输入，覆盖 dependency baseline workspace/package manifests、package lock 和后续复用这些 helper 的 report readers；symlinked TOML/JSON input 会直接失败。
- `vesty smoke-host --check --out` 读取既有 report 时改为 no-follow file helper，`--bridge-trace`、`--meter-log` 和 example `params.specs.json` 读取同样拒绝 symlinked input。这样本地 headless diagnostic 不能通过 symlinked report/trace/log/specs 跟随到 workspace 外部的可替换文件。
- 新增 `toml_and_json_read_helpers_reject_symlink_inputs` 和 `smoke_host_rejects_symlinked_report_bridge_trace_meter_log_and_parameter_specs`，分别覆盖通用 TOML/JSON reader、smoke-host report、bridge trace、meter log 和 example parameter specs。
- 该改动只收紧本地 CLI diagnostic/dependency evidence 读取边界；`vesty smoke-host` 仍是 diagnostic artifact，不替代真实 DAW、platform WebView、validator、CI、签名或 notarization evidence。

本次 2026-06-11 继续收紧 `vesty daw-matrix` evidence 读取边界:

- 新增 parent/leaf directory no-follow helper，并接入 DAW evidence host dir 创建与读取；`--evidence-root` 本身是 symlink 时，`daw-matrix --write-template`、`daw-matrix --write-report` 和 matrix 读取都不会跟随到外部 evidence tree。
- `read_optional()` 现在通过 no-follow file helper 读取 DAW evidence marker；`platform.txt`、`scan-smoke.log`、`load-smoke.log`、`ui-smoke.log`、`ui-host-smoke.log`、`meter-stream.log`、`automation-smoke.log`、`buffer-sample-rate.log`、`restore-smoke.log`、`offline-render.log` 和 REAPER 兼容 marker 如果是 symlink，会被当作不可用 evidence，而不是跟随到外部目标。
- `render_file_exists_and_nonempty()` 现在复用 no-follow file helper 并检查 metadata length；`render_file=...` 指向 symlinked rendered WAV 时不会被判定为 offline render pass。
- 新增 `daw_evidence_templates_reject_symlink_evidence_root`、`generic_daw_evidence_ignores_symlinked_evidence_root`、`daw_matrix_write_report_rejects_symlinked_evidence_root`、`generic_daw_evidence_ignores_symlinked_marker_files` 和 `render_file_evidence_rejects_symlinked_render_targets`，分别覆盖 symlinked DAW evidence root、write-report root、marker/platform fallback 和 symlinked render target。
- 该改动只收紧本地/外部 DAW evidence 的读取信任边界；仍不合成任何 DAW pass evidence，也不替代真实 host smoke。

本次 2026-06-11 继续收紧 JSBridge Rust -> JS delivery 边界:

- `@vesty/plugin-ui` 的 `__VESTY_INTERNAL__.deliver()` / `deliverBatch()` 现在会先做入站 packet/batch shape 校验: batch 必须是 array，packet 必须是 object、`v = 1`、合法 session/type、JavaScript safe-integer 范围内的非负 seq、已知 lane/kind；Rust -> JS packet 不能携带 `id`，response/error 必须有合法 `replyTo`，event/ack 不能混入 `replyTo` 或 `error`，malformed error payload 会被拒绝。
- wry bootstrap 内嵌 fallback bridge 使用同一 fail-closed 语义；畸形 packet 或非数组 batch 会被静默丢弃，不触发 subscription listener、不错误 resolve/reject pending request，也不从 dispatcher 抛异常。
- 新增 JS SDK 测试覆盖 malformed inbound event batch、malformed response/error 不结算 pending request、后续合法 response 仍可完成；wry bootstrap string test 固定 `validInboundPacket()` / `validBridgeError()` / `deliverBatch` array guard。
- 该改动不改变协议 wire format，也不替代 native `BridgeRuntime` 权威校验；它只是让 WebView 全局 dispatcher 面对旧脚本或畸形 evaluate_script 输入时更稳。

本次 2026-06-11 继续收紧 wry bootstrap internal session/request 边界:

- `vesty-ui-wry` 注入 bootstrap 的 `window.__VESTY_INTERNAL__.setSession(value)` 现在复用 session validator，拒绝 non-string、空字符串、超过 128 UTF-8 bytes 或包含控制字符的 session value。
- `window.__VESTY_INTERNAL__.request(type, lane, payload)` 现在除了校验 request type，也会校验 lane 必须是已知 `BridgeLane`；非法 lane 不会创建 pending request、启动 timeout 或调用 `window.ipc.postMessage`。
- wry bootstrap string test 固定 `BRIDGE_REQUEST_LANES`、`assertRequestLane(lane)`、`assertBridgeSessionValue(value, "session", validationError)` 和相关 validation error 文案存在；本地已通过 `rtk cargo fmt --all --check` 与 `rtk cargo test -p vesty-ui-wry bootstrap_script -- --nocapture`。
- 该改动只保护 fallback/global internal JS 入口，不改变公开 `@vesty/plugin-ui` API，也不替代 native `BridgeRuntime` 对实际 IPC packet 的权威校验。

本次 2026-06-11 最终本地验证:

- `rtk cargo fmt --all --check`: passed。
- `rtk cargo test -p vesty-cli smoke_host -- --nocapture`: 4 passed。
- `rtk cargo test -p vesty-cli dependency_baseline -- --nocapture`: 6 passed。
- `rtk cargo test -p vesty-cli daw -- --nocapture`: 18 passed。
- `rtk cargo test -p vesty-cli symlink -- --nocapture`: 33 passed。
- `rtk cargo test -p vesty-cli release_check -- --nocapture`: 37 passed。
- `rtk cargo test -p vesty-ui-wry bootstrap_script -- --nocapture`: 1 passed。
- `rtk cargo test --workspace -j1`: 531 passed。
- `rtk cargo clippy --workspace --all-targets -- -D warnings`: no issues。
- `rtk npm test`: passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current.json`: 按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败/跳过项仍是 DAW matrix、platform smoke、validator/static validate matrix、CI artifacts、签名和 notarization 等真实外部 evidence 缺失。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json`: 按预期 failed；`vst3 static validate reports` 在 required 模式下明确为 failed，不再是 skipped。

本次 2026-06-11 继续收紧 VST3 SDK generated-header input probe:

- `probe_sdk_headers()` 不再用 `Path::is_file()` 跟随 symlink；required SDK header 是 symlink 时会报告 missing，而不是 present。
- `generated_bindings_surface()` 重新读取 manifest-listed header 时复用 no-follow file helper，SDK version hint 读取也先拒绝 symlink/non-file。
- 新增 `sdk_header_probe_treats_symlink_inputs_as_missing`，并通过 `rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture` 与 `rtk cargo test -p vesty-vst3-sys sdk_header -- --nocapture`。
- 该改动只收紧 generated-headers backend 的输入审计一致性；仍保持 `BINDINGS_GENERATED = false` / `FULL_COM_BINDINGS_GENERATED = false`，不表示完整 SDK 3.8 bindings 已生成。

本次 2026-06-11 继续收紧 VST3 SDK audit/check 与 `import-ci` artifact 读取边界:

- `vesty vst3-sdk manifest --check`、`binding-plan --check`、`binding-surface --check`、`emit-scaffold --check`、`emit-abi-seed --check`、`emit-abi --check` 和 `emit-interface-skeleton --check` 现在通过 CLI no-follow file helper 读取 `--out` 复验输入；symlinked audit/check 文件会直接失败，不会跟随到可替换外部目标。
- `release-evidence import-ci` 的 JSON artifact 识别入口和 Rust artifact 识别入口现在也先拒绝 symlinked source files，再解析 release-check、doctor、validate、VST3 SDK manifest/plan/surface/scaffold/ABI/interface skeleton 等 artifact。
- 新增 `vst3_sdk_check_commands_reject_symlink_outputs` 与 `import_ci_artifact_readers_reject_symlink_files`，并通过 `rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 和 `rtk cargo fmt --all --check`。
- 该改动只收紧 generated-headers audit/check 与 CI artifact staging 的文件读取信任边界；它不合成 CI pass evidence，也不改变 `generated.rs` / ABI / interface skeleton 仍为 drift/audit 留档而非完整 bindings 的事实。

本次 2026-06-11 继续收紧 `release-check --require-release-artifacts` 的 static validate 门禁表达:

- `vst3 static validate reports` 汇总项现在和 validator、CI static coverage 一样接收 required 语义；在 `--require-release-artifacts` 下没有任何 `--static-validate-report` / evidence-dir static validate report 时会明确 `failed: required evidence missing`，不再保持 `skipped`。
- 这不会把本地 static validate 诊断报告当作发布通过证据；完整发布仍由 `ci example static validate coverage` 要求三示例 x macOS/Windows x64/Linux x64 的 package/static validate matrix，并要求 matching bundle path、parameter sidecar、asset manifest 和 binary export evidence 自洽。
- 新增/更新 release-check 回归断言，覆盖 require 模式缺失 static validate 汇总失败，以及完整 release evidence fixture 中 static validate 汇总仍为 ok；本地已通过 `rtk cargo test -p vesty-cli release_check -- --nocapture`。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 按预期失败，并且 `vst3 static validate reports` 现在明确失败；其它失败仍是真实 DAW/platform/CI/validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 release action plan 命令漂移保护:

- `release_action_plan_vesty_commands_parse_with_current_cli` 会从 `build_release_action_plan()` 生成的所有 `vesty ...` 建议命令中提取 argv，并用当前 `Cli::try_parse_from()` 做解析级回归；测试不执行外部命令，只验证 action plan 中的子命令和 flag 名称仍存在。
- 该测试覆盖 DAW matrix、platform smoke、validate/static validate、publish-plan、crate-package、npm-pack、dependency-baseline、VST3 SDK audit、collect-signing 和 collect-notarization 等建议路径，防止 `--notary-log` / `--stapler-log` 这类采证 flag 再次与真实 CLI 漂移。
- 同步修正文档中旧的 notarytool log flag 写法为实际支持的 `--notary-log`。
- 本地已通过 `rtk cargo fmt --all --check` 与 `rtk cargo test -p vesty-cli release_action_plan -- --nocapture`。

本次 2026-06-11 继续收紧外部 release action plan sidecar 导入校验:

- `validate_release_action_plan_sidecar()` 现在也会解析 sidecar 中所有以 `vesty ` 开头的建议命令；如果命令已经不符合当前 CLI 子命令/flag 形状，会拒绝该 sidecar，`release-evidence import-ci` 会记录 failed 且不会复制到 `ci-release-checks/`。
- command parser 只做轻量 shell-like 拆分，支持双引号和 `#` 注释，不执行命令；命令会先 trim 再识别 `vesty`，所以前导空格不能绕过解析，未闭合双引号也会被拒绝，避免导入半截不可执行采证命令。
- 新增 `release_action_plan_sidecar_rejects_stale_vesty_commands`，覆盖前导空格 + 旧 notarization flag、裸 `vesty` 命令和未闭合引号；本地已通过 `rtk cargo test -p vesty-cli release_action_plan -- --nocapture` 与 `rtk cargo test -p vesty-cli import_ci -- --nocapture`。
- 该改动只提高外部 checklist 的可用性和防漂移能力；action plan sidecar 仍不是 release pass evidence，`--ci-release-check-dir` 仍只采纳 `release-check*.json`。

本次 2026-06-11 继续收紧 release action plan failed summary 自洽性:

- `validate_release_action_plan_sidecar()` 现在要求 `status = "failed"` 的 sidecar 至少包含一个真正的 failed action；只有 skipped action 的 action plan 不应标记为 failed，因为 release-check 顶层状态在只有 skipped optional checks 时应为 ok。
- `release_action_plan_sidecar_rejects_failed_empty_plan` 增加 skipped-only failed plan 覆盖，避免外部 CI checklist 用 `failed` 顶层状态包裹纯 optional skipped 项来制造错误的失败摘要。
- 本地已通过 `rtk cargo test -p vesty-cli release_action_plan -- --nocapture` 与 `rtk cargo test -p vesty-cli import_ci -- --nocapture`。

本次 2026-06-11 继续收紧 release action plan 顶层路径字段校验:

- `validate_release_action_plan_sidecar()` 现在会校验 `protocol_snapshot`、`evidence_root` 和 `release_evidence_dir` 顶层字段，拒绝空白或包含控制字符的路径文本，和 action 内 `evidence_path`/command 文本使用同一信任边界。
- `release_action_plan_sidecar_rejects_incomplete_actions` 增加空 `protocol_snapshot`、带控制字符的 `evidence_root` 和带控制字符的 `release_evidence_dir` 覆盖，避免外部 sidecar 导入后携带不可复制或污染终端/日志的顶层路径 metadata。
- `validate_release_action_plan_sidecar()` 现在还会拒绝重复的 `action.check`，避免外部 sidecar 用同名 checklist action 覆盖或混淆 release evidence 后续处理。
- `release_action_plan_sidecar_rejects_incomplete_actions` 增加 duplicate action check 覆盖；重复项会作为 malformed sidecar 被拒绝，不会被 `release-evidence import-ci` 复制进 `ci-release-checks/`。
- 本地已通过 `rtk cargo test -p vesty-cli release_action_plan -- --nocapture` 与 `rtk cargo test -p vesty-cli import_ci -- --nocapture`。

本次 2026-06-11 duplicate action check 收口后的完整复验:

- `release_check_accepts_ci_signing_and_notarization_evidence` 的完成态断言现在会打印 failed check 名称和值，方便后续定位 release gate fixture 失败。
- `rtk cargo fmt --all --check`: passed。
- `rtk cargo test -p vesty-cli release_action_plan -- --nocapture`: 6 passed。
- `rtk cargo test -p vesty-cli import_ci -- --nocapture`: 9 passed。
- `rtk cargo test --workspace -j1`: 531 passed。
- `rtk cargo clippy --workspace --all-targets -- -D warnings`: no issues。
- `rtk npm test`: passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json`: 按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW matrix、platform smoke、validator/static validate matrix、CI artifacts、签名和 notarization evidence 缺失。

本次 2026-06-11 继续收紧 release action plan `vesty` 命令识别边界:

- `validate_release_action_command_syntax()` 现在通过首 token 判断 `vesty` 建议命令；`vesty` 后跟任意 Unicode whitespace 或命令结束都会进入当前 `Cli::try_parse_from()` 校验，不再只识别普通空格 `vesty `。
- `release_action_plan_sidecar_rejects_stale_vesty_commands` 增加 `vesty\u{00a0}...` 覆盖，防止外部 sidecar 用 non-breaking space 之类的非控制 whitespace 绕过 stale flag 检查。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 531 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 release action plan sidecar 文本字段边界:

- `validate_release_action_text()` 现在除了拒绝空白和控制字符，也会拒绝常见不可见/双向 Unicode format 字符，包括 zero-width、bidi override/isolate、word joiner 和 BOM 码位，避免外部 action-plan sidecar 在 check/value/hint/evidence path/command 等 metadata 中污染日志或制造展示混淆。
- `release_action_plan_sidecar_rejects_incomplete_actions` 增加 `hint` 中 U+202E 的覆盖；这类 sidecar 会作为 malformed checklist 被拒绝，不会由 `release-evidence import-ci` 复制进 `ci-release-checks/`。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 531 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 release action plan 写出闭环:

- `write_release_action_plan()` 现在在写入 `--plan` JSON 前调用同一套 `validate_release_action_plan_sidecar()`；如果内部生成的 action plan 与 `release-evidence import-ci` 接受的 sidecar 契约漂移，CLI 会返回 invalid data error，坏 checklist 不会落盘。
- `release_action_plan_writer_rejects_invalid_plan` 覆盖写出前校验: 缺少 suggested commands 的 plan 会被拒绝，目标 `release-action-plan.json` 不会创建。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 532 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 release action plan summary 自洽性:

- `validate_release_action_plan_sidecar()` 现在要求 `summary.action_count == summary.failed + summary.skipped`，因为 action plan 的 `actions` 列表只应包含 pending action，也就是 failed/skipped checks。
- `validate_release_action_plan_sidecar()` 现在还会拒绝 `ok + failed + skipped == 0` 的空 summary，避免空的外部 sidecar 被当作有意义的 checklist metadata 保存。
- `release_action_plan_sidecar_rejects_incomplete_actions` 增加 pending count mismatch 覆盖，`release_action_plan_sidecar_rejects_failed_empty_plan` 增加 empty-ok summary 覆盖。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 532 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 release action plan summary sanity bound:

- `validate_release_action_plan_sidecar()` 现在对 `summary.ok + summary.failed + summary.skipped` 增加宽松上限 `128`；当前 release-check 实际只有几十项，该上限给后续 gate 增长留空间，同时拒绝明显伪造或损坏的超大 summary 计数。
- `release_action_plan_sidecar_rejects_incomplete_actions` 增加 absurd summary 覆盖，`summary.ok` 被填成超过上限时会作为 malformed sidecar 被拒绝，不会被 `release-evidence import-ci` 保存为 checklist metadata。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 532 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 release action plan sidecar 资源边界:

- `validate_release_action_text()` 现在限制单个 checklist 文本字段最大 8 KiB，覆盖 check/value/hint/evidence path/command 和顶层路径 metadata，避免外部 sidecar 用超长文本污染日志或拖慢导入审计。
- `validate_release_action_item()` 现在限制每个 action 最多 16 条 suggested commands；当前生成器远低于该数量，限制只用于拒绝畸形或滥用的外部 checklist。
- `release_action_plan_sidecar_rejects_incomplete_actions` 增加超长 value 和过多 commands 覆盖；这类 sidecar 会作为 malformed checklist 被拒绝，不会由 `release-evidence import-ci` 保存到 `ci-release-checks/`。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 532 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 CI release-check artifact 文本 metadata 边界:

- `validate_ci_release_check_report()` 现在会校验每个 `ReleaseCheckItem` 的 `name`、`value` 和 optional `hint`，复用 release evidence 文本边界: 非空、无控制字符、无 unsafe invisible/bidirectional Unicode format 字符，并且单字段最大 8 KiB。
- `ci_release_check_artifacts_reject_inconsistent_or_unknown_statuses` 增加带换行的 forged check value 和超长 hint 覆盖；畸形 CI release-check artifact 会被标记为 failed，不会参与 release evidence pass。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 532 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 CI release-check artifact 顶层 `ci_run_url` metadata:

- `validate_ci_release_check_report()` 现在如果看到 report 顶层 `ci_run_url`，会先复用 release evidence 文本边界校验，再要求它是合法 GitHub Actions run URL；即使最终汇总命令没有显式传入 `--ci-run-url`，单个 artifact 自身也不能携带畸形来源 URL。
- `ci_release_check_artifacts_reject_inconsistent_or_unknown_statuses` 增加非数字 run id 和带控制字符 run URL 覆盖；畸形来源 metadata 会让对应 CI release-check artifact failed。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 532 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 CI release-check artifact checks 结构边界:

- `validate_ci_release_check_report()` 现在拒绝空的 `checks` 列表，并限制单个 CI release-check report 最多 128 个 checks；当前真实 report 只有几十项，该上限给后续 gate 增长留空间，同时拒绝损坏或滥用的超大 artifact。
- `ci_release_check_artifacts_reject_inconsistent_or_unknown_statuses` 增加 empty checks 和 too-many checks 覆盖；畸形 CI snapshot 会被标记为 failed，不会参与 release evidence pass。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 532 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 platform smoke artifact shape 边界:

- `validate_platform_smoke_report_shape()` 现在会在真实 platform smoke report 验证前统一校验 `platform`、optional `os`、optional `host` 以及每个 check 的 `name/status/value/hint`，复用 release evidence 文本边界: 非空、无控制字符、无 unsafe invisible/bidirectional Unicode format 字符，并且单字段最大 8 KiB。
- platform smoke report 现在必须包含至少一个 check，且最多 32 个 checks；每个 check name 归一化后必须稳定且不能重复。`platform_smoke_release_check()` 会在跳过 pending template 前先跑这层 shape 校验，因此畸形 pending JSON 不会绕过 artifact 结构审计。
- `platform_smoke_rejects_malformed_report_shape` 覆盖重复 `system_webview` check、`host` 控制字符、hint 中 U+202E 和超出 check 上限；畸形 platform smoke artifact 会被标记为 failed，不会参与 release evidence pass。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli platform_smoke -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 533 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/publish/npm/signing/notarization evidence 缺失。

本次 2026-06-11 刷新 dependency latest baseline:

- online `vesty dependency-baseline --latest` 发现 npm registry latest `vue` 已从 `3.5.35` 前进到 `3.5.37`，后续本次复验又发现 `vue` 已前进到 `3.5.38`；`package-lock.json` / `node_modules` 已更新到 Vue `3.5.38`，`packages/vue/package.json` 仍保持 devDependency range `"latest"`。
- `vesty-cli` 内置 `REQUIRED_JS_LATEST_BASELINE_DEPENDENCIES` 和对应 dependency-baseline 单测已同步 Vue `3.5.38`，`.agents/11-latest-deps-feasibility.md` 也同步当前 React/Vue/Svelte latest 记录。
- 本地已通过 `rtk cargo test -p vesty-cli dependency_baseline -- --nocapture`、`rtk cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline-current/dependency-baseline-latest.json --format text`、`rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-current/dependency-baseline-latest.json --format text`、`rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 533 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/publish/npm/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 CI doctor artifact shape 边界:

- 新增 `validate_doctor_report()`，会在 `release-check --ci-doctor-dir` 汇总和 `release-evidence import-ci` 单文件导入时统一校验 doctor artifact。顶层 `os`、顶层 `ci_run_url`、每个 check 的 `name/status/value/hint` 复用 release evidence 文本边界: 非空、无控制字符、无 unsafe invisible/bidirectional Unicode format 字符，并且单字段最大 8 KiB。
- 顶层 `ci_run_url` 一旦存在就必须是合法 GitHub Actions run URL；不再只在传入 expected run URL 时才发现 artifact 自身的畸形来源 metadata。legacy artifact 仍允许缺少 `ci_run_url`，以兼容旧 CI doctor reports。
- doctor artifact 现在必须包含至少一个 check，最多 128 个 checks；check status 必须是 `ok`、`missing`、`skipped`、`unknown` 或 `unsupported`；重复 check name 会被拒绝。`ci_doctor_artifacts_release_check()` 现在还会拒绝同一目录内重复 OS reports，避免递归 artifact 目录中同一 runner 证据被重复计数。
- `ci_doctor_artifacts_reject_malformed_report_shape` 覆盖非法 `ci_run_url`、重复 check、`os` 控制字符、hint 中 U+202E、未知 status 和超出 check 上限；`ci_doctor_artifacts_reject_duplicate_os_reports` 覆盖重复 Linux report；畸形 doctor artifact 会被标记为 failed，不会参与 release evidence pass。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_doctor -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 535 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW/platform/CI/validator/static validator/publish/npm/signing/notarization evidence 缺失。

本次 2026-06-11 继续收紧 VST3 validate report artifact shape 边界:

- `validate_validate_report_shape()` 现在统一校验 `vesty validate --report` 产物的顶层 bundle、static check、binary export check 和 validator check 文本字段。普通 metadata 字段限制为非空、无控制字符、无 unsafe invisible/bidirectional Unicode format 字符、单字段最大 8 KiB；validator stdout/stderr 允许 tab/newline 作为日志文本，但拒绝其它控制字符、unsafe Unicode format 字符，并限制为最大 256 KiB。
- static bundle report 现在限制最多 64 个 binaries、64 个 binary export checks、每类 required/found/missing symbols 最多 64 项，并拒绝重复 binary path、重复 binary/platform export check 和重复 symbol 项。`write_validate_report()`、`validate_release_validate_report()` 和 `validate_static_validate_report()` 都会先走这层 shape 校验，因此畸形 validate artifact 不会被 CI import、release evidence auto-discovery 或 release-check 计入。
- `validate_report_rejects_malformed_shape_fields` 覆盖 bundle 控制字符、重复 binary、重复 required symbol、过多 binaries、validator stdout 允许 tab/newline、stdout 拒绝 NUL、stderr 超限和 reason 中 U+202E；畸形 validate artifact 会被标记为 failed，不会参与 release evidence pass。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli validate_report -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 536 passed。
- `release-check --strict --require-release-artifacts` 仍按预期 failed；本地 invariant checks 可通过，但真实 DAW matrix、platform smoke、CI artifacts、完整 validator/static matrix、publish/npm evidence、签名和 notarization evidence 仍缺失。

本次 2026-06-11 继续收紧 smoke-host report artifact shape 边界:

- `validate_smoke_host_report()` 现在先执行 `validate_smoke_host_report_shape()`，统一校验 generator、workspace、status、external evidence note 以及每个 check 的 name/status/value/hint。外部 report 字段复用 release evidence 文本边界: 非空、无控制字符、无 unsafe invisible/bidirectional Unicode format 字符、单字段最大 8 KiB。
- smoke-host report 现在必须至少包含一个 check，最多 64 个 checks；check name 归一化后必须稳定且不能重复。已有版本、generator、status 和 status-from-checks 语义校验保留。
- 生成端的 `smoke_host_ok/skipped/failed` 会把多行诊断、控制字符和 unsafe Unicode format 字符清洗为单行 bounded 文本，避免本地诊断错误字符串让 CLI 自己产出畸形 report。
- `smoke_host_report_rejects_malformed_shape_fields` 覆盖重复 check、workspace 控制字符、hint 中 U+202E、超出 check 上限，以及生成端多行/unsafe diagnostic 清洗。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli smoke_host -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 537 passed。

本次 2026-06-11 继续收紧 publish/crate/npm package report artifact shape 边界:

- `publish-plan` 现在在 `write_publish_plan_report()` 和 `validate_publish_plan()` 入口执行 `validate_publish_plan_shape()`；publish packages 至少 1 个、最多 128 个，skipped private 最多 128 个，单个 package 的 internal dependencies 最多 128 个，并拒绝空白/控制字符/unsafe Unicode format 字符/超长文本、重复 skipped private 和重复 dependency。
- `crate-package` 现在在 `write_crate_package_report()` 和 `validate_crate_package_report()` 入口执行 `validate_crate_package_report_shape()`；packages 至少 1 个、最多 128 个，单个 package internal dependencies 最多 128 个，并统一校验 generator/status/name/version/manifest/status/reason/dependency 文本边界和重复 dependency。`cargo package` stdout/stderr 摘要会清洗成单行 bounded 文本，避免失败 reason 携带多行控制字符或 bidi/zero-width 字符。
- `npm-pack` 现在在 `validate_npm_pack_entries()` 入口执行 `validate_npm_pack_entries_shape()`；packages 至少 1 个、最多 16 个，单包最多 512 个 packed files，总 packed files 最多 2048 个，并统一校验 package name/version/filename/path 文本边界和重复 packed path。
- `publish_crate_and_npm_reports_reject_malformed_shape_fields` 覆盖 publish skipped private 中 U+202E、重复 publish dependency、过多 publish packages、crate reason 中 U+202E、重复 crate dependency、过多 crate packages、cargo 输出摘要清洗、npm filename 控制字符、重复 packed path 和过多 packed files。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli publish -- --nocapture`、`rtk cargo test -p vesty-cli crate_package -- --nocapture`、`rtk cargo test -p vesty-cli npm_pack -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 538 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW matrix、platform smoke、CI artifacts、validator/static matrix、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 继续收紧 DAW smoke marker 输入边界:

- `write_daw_smoke_report()` 现在会先校验 `host` metadata，复用 release evidence 文本边界，拒绝空白、控制字符、unsafe Unicode format 字符和超长文本，然后再解析 host profile。
- `required_daw_marker()` 现在通过 `validate_daw_smoke_marker_text()` 校验 marker/log 字段。`platform` 仍是严格单行 metadata；scan/load/ui/ui-host/meter/automation/buffer-save/render 等 marker 允许 tab/newline 作为真实 DAW 日志内容，但限制为最大 256 KiB，并拒绝 NUL/其它控制字符和 unsafe Unicode format 字符。
- 语义 marker 解析不变: pending/false/negative evidence、zero meter、vague automation 和 render_file 安全边界仍按原规则处理；新增边界只防止畸形 marker 文本落盘并污染后续 DAW matrix/release-check。
- `daw_matrix_write_report_rejects_malformed_marker_text` 覆盖 host 控制字符、platform 多行控制字符、scan 中 NUL、ui 中 U+202E 和超长 load marker；`daw_matrix_write_report_accepts_multiline_marker_logs` 覆盖多行/tab 日志 marker 仍可通过。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli daw -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 540 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW matrix、platform smoke、CI artifacts、validator/static matrix、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 继续收紧 signing/notarization evidence 日志边界:

- `signing_evidence_platforms_from_text()` 和 `notarization_evidence_from_text()` 现在先执行 `validate_release_evidence_log_text()`；签名和 notarization 日志必须非空，最大 512 KiB，允许 tab/newline 作为真实工具输出，但拒绝 NUL/其它控制字符和 unsafe Unicode format 字符。
- macOS `.vst3` bundle signing evidence 的 `Contents/_CodeSignature/CodeResources` 现在使用 no-symlink file helper 读取，并限制最大 16 MiB；内部 `CodeResources` symlink 会被明确拒绝，不再只依赖顶层 `.vst3` 路径 no-symlink。
- 语义解析不变: `codesign`/`signtool` positive markers、signtool zero-error summary、notary accepted status、stapler success 以及负向 evidence 优先级都保持原规则；新增边界只防止畸形日志或内部 symlink artifact 被计入。
- `signing_and_notarization_evidence_reject_malformed_log_text` 覆盖签名日志 NUL、签名日志 U+202E、超长签名日志、notary 日志 NUL、notary 日志 U+202E 和超长 notary 日志；`signing_evidence_rejects_symlinked_code_resources` 覆盖内部 `CodeResources` symlink。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli notarization -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 542 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW matrix、platform smoke、CI artifacts、validator/static matrix、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 继续收紧 dependency baseline 与 release evidence audit report 边界:

- `validate_dependency_baseline_report_shape()` 现在在 dependency baseline report 语义校验和写出前运行。report generator/status、每个 check 的 name/kind/path/expected/actual/status 和 optional hint 都复用 release evidence 文本边界；hint 允许 tab/newline 但限制最大 64 KiB，report 最多 256 个 checks，并拒绝重复 `kind:name` check key。
- `local-collect-report.json`、`import-ci-report.json` 和 `collect-signing` / `collect-notarization` 的 JSON stdout report 现在也有 shape gate: 顶层 evidence dir/source/workspace/output/kind/external note 和每个 item 的 name/status/path/source/value 都必须是 bounded、无控制字符、无 unsafe Unicode format 字符的文本；item 总数最多 1024。`collect-local` 与签名/公证采集 report 必须至少有一个 item；`import-ci` 允许空导入 report，但只允许 `ok/imported/skipped/failed` 状态。
- `import_ci_item()` 现在会把外部 artifact parser 的错误诊断清洗为单行 bounded text，避免换行、NUL 或 bidi/zero-width 字符进入 `import-ci-report.json`。签名/公证采集会先验证 JSON audit report shape，再写出对应 evidence log。
- `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 unsafe Unicode note、过多 items、import 诊断清洗、非法 import status、非法 collected kind/status 和空 collected report。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports -- --nocapture`、`rtk cargo test -p vesty-cli collect_local_release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli notarization -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 544 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW matrix、platform smoke、CI artifacts、validator/static matrix、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 继续收紧 VST3 SDK JSON audit artifact shape 边界:

- `validate_vst3_sdk_header_manifest_shape()` 现在在 `vesty vst3-sdk manifest` 写出前、manifest `--check`/release-check/import-ci 内容校验前运行。manifest generator/baseline/version hint/header path/sha256/missing headers 都复用 release evidence 文本边界；header path 必须是相对 normalized path，headers 和 missing headers 各最多 128 项，并拒绝重复 header。
- `validate_vst3_sdk_binding_plan_shape()` 现在在 `vesty vst3-sdk binding-plan` 写出前和内容校验前运行。plan metadata、embedded header manifest、checks、blockers 和 next_steps 都受 bounded/control-safe/unsafe-Unicode-safe 文本边界约束；checks 最多 32 项且 name 不可重复，next_steps 必须非空。
- `validate_vst3_sdk_binding_surface_shape()` 现在在 `vesty vst3-sdk binding-surface` 写出前和内容校验前运行。surface metadata、embedded header manifest、required/missing headers、missing symbols、blockers、notes 和 symbol rows 都受同一文本边界约束；symbols 最多 512 项，notes 必须非空，symbol `(name, kind, header)` 不可重复，symbol header 也必须是相对 normalized path。
- `vst3_sdk_json_artifacts_reject_malformed_shape_fields` 覆盖 unsafe Unicode version hint、重复 manifest header、过多 manifest headers、重复 binding plan check、空 next_steps、重复 surface symbol、空 notes 和不安全 header path。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 545 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，VST3 SDK manifest/plan/surface 仍是 optional skipped，失败项仍是真实 DAW matrix、platform smoke、CI artifacts、validator/static matrix、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 继续收紧 release-check report 写出边界:

- 新增 `validate_release_check_report_shape()`，并让 `write_release_check_report()` 在 `--report` JSON 写出前执行。report 顶层 status/ci_run_url、每个 check 的 name/status/value/hint、check 数量、duplicate check name、status 与 failed checks 自洽性都走 bounded/control-safe/unsafe-Unicode-safe 校验；`ci_run_url` 一旦存在必须是合法 GitHub Actions run URL。
- `validate_ci_release_check_report()` 现在复用通用 shape gate 后再做 CI per-OS snapshot 专属的 invariant 检查，避免 writer 和 import/release evidence 的结构契约分叉。
- `daw_matrix` report rows 现在也有基础 shape gate: rows 数量有宽松上限；每行必须是 object，`host/evidence/platform` 字段若存在必须是安全文本；scan/load/ui/ui_host_param/meter_stream/automation/buffer_sample_rate_change/save_restore/offline_render 等 smoke 字段若存在必须是 boolean；重复 host 被拒绝。
- `release_check_report_writer_rejects_malformed_report_shape` 覆盖重复 check name、unsafe Unicode hint 和非 boolean DAW matrix 字段；失败时 report 文件不会被创建。
- 本地已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 546 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed 并成功写出 report；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW matrix、platform smoke、CI artifacts、validator/static matrix、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 继续收紧 CLI report 打印入口:

- `run_doctor()` 现在会在 text/json 输出前调用 `validate_doctor_report()`；生成的 doctor report 与 CI doctor artifact 接受侧共享同一 shape gate，避免本地命令输出畸形 doctor JSON。
- `print_smoke_host_report()` 现在会在 text/json 输出前调用 `validate_smoke_host_report()`；即使未来绕过 `run_smoke_host()` 直接打印，也不能输出重复 check、控制字符、unsafe Unicode format 字符或 status 不自洽的 smoke-host report。
- VST3 SDK audit 的 `manifest`、`binding-plan` 和 `binding-surface` 打印入口现在也会先执行对应 shape gate；write、check/import/release-check acceptance 和 stdout/text 输出使用同一结构边界。
- local/import/collected release evidence、crate package、dependency baseline、npm pack、publish plan 和 validate report 的打印入口也会先执行已有 shape validator；这把 release evidence JSON 的生成、写出、导入和打印合同收敛到同一组 bounded/control-safe/unsafe-Unicode-safe 规则。
- 新增/扩展测试覆盖: `doctor_report_includes_toolchain_webview_and_validator_checks` 会断言生成 doctor report 通过 shape gate；`smoke_host_report_rejects_malformed_shape_fields` 还会断言打印畸形 smoke-host report 被拒绝。相关聚焦测试已通过 `rtk cargo test -p vesty-cli doctor -- --nocapture`、`rtk cargo test -p vesty-cli smoke_host -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli publish -- --nocapture`、`rtk cargo test -p vesty-cli validate -- --nocapture`、`rtk cargo test -p vesty-cli dependency_baseline -- --nocapture`、`rtk cargo test -p vesty-cli npm_pack -- --nocapture`、`rtk cargo test -p vesty-cli crate_package -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture` 和 `rtk cargo test -p vesty-cli import_ci -- --nocapture`。
- 本地完整复验通过 `rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 546 passed。`rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 仍按预期 failed 并写出 report；host profiles、protocol snapshot、VST3 binding baseline 和 dependency latest baseline 为 ok，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 再次刷新 latest dependency baseline 与本地 release evidence:

- `release-evidence collect-local --crate-package --dependency-baseline-latest` 首次复跑时正确发现 npm registry latest `vue` 已从 `3.5.37` 前进到 `3.5.38`，旧 dependency latest report 被拒绝为 drift。随后通过 npm 更新 lockfile/node_modules，保留 `packages/vue/package.json` devDependency range 为 `"latest"`，并把 CLI 内置 `expected_lock_version` 与 dependency-baseline 单测同步到 `3.5.38`。
- `rtk cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline-current/dependency-baseline-latest.json --format text` 和带 `--check` 的复验均通过；report 仍覆盖 44 个 baseline checks 和 29 个 registry latest checks，其中 npm registry latest `vue` 为 `3.5.38`。
- `rtk cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-current --crate-package --dependency-baseline-latest --format json` 已生成/刷新本地可采集 evidence: protocol snapshot 22 个 TypeScript files / 7 个 JSON schema files，crate publish plan 13 个 publishable crates / 3 个 private skipped，crate package readiness 3 个 packageable now / 10 个 deferred，npm pack 4 packages / 58 files，dependency latest baseline 44 baseline checks / 29 latest registry checks。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json` 仍按预期 failed；本地可证明项现在自动发现并为 ok: protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline。失败项收敛为真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。
- 刷新后本地完整复验通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli dependency_baseline -- --nocapture`、`rtk npm test`、`rtk cargo test --workspace -j1` 和 `rtk cargo clippy --workspace --all-targets -- -D warnings`；workspace Rust tests 为 546 passed。

本次 2026-06-11 收紧 crate package readiness final gate:

- `build_release_check_report()` 现在把 `release_evidence.require_release_artifacts` 传给 `crate_package_release_check()`。因此普通本地 `release-check` 仍可在缺少 crate package readiness 时显示 `skipped`，但 `--require-release-artifacts` 下缺失 `--crate-package-report` 或 `release-evidence-dir/crate-package/crate-package.json` 会明确 failed。
- `release_check_requires_release_artifacts_when_requested` 新增断言，锁定 `crate package readiness` 在 final release artifact 模式下是 required evidence，不再悄悄保持 optional skipped。
- 文档同步: `.agents/07-build-packaging.md`、`.agents/08-developer-guide.md`、`.agents/14-completion-audit.md` 和 CLI evidence README 模板现在都说明 crate package readiness 在普通本地检查中可 skipped，但 final `--require-release-artifacts` 必须存在并有效。
- 验证通过 `rtk cargo test -p vesty-cli release_check_requires_release_artifacts_when_requested -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 为 546 passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json` 现在会把 `crate package readiness` 标为 failed / required evidence missing；`rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json` 仍自动发现本地 `crate-package/crate-package.json` 并把该项标为 ok。两者仍按预期因真实 DAW/CI/platform/validator/signing/notarization 外部证据缺失而 failed。

本次 2026-06-11 补强 release action plan 与 latest dependency gate 复验:

- `release_action_plan_lists_required_and_optional_evidence` 现在断言 `--require-release-artifacts` 下缺失 `crate package readiness` 时，生成的 action plan 会把该项列为 `failed` / `required`，并给出 `release-evidence/crate-package/crate-package.json` evidence path 和 `vesty crate-package --out ...` 建议命令。这样 final gate 与 checklist 输出保持同步，避免门禁加严后行动计划仍漏掉 crate package 证据。
- `rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-current/dependency-baseline-latest.json --format text` 已联网复验当前 crates.io/npm latest baseline，结果为 ok；当前仍匹配 `wry 0.55.1`、`vst3 0.3.0`、`raw-window-handle 0.6.2`、`rtrb 0.3.4`、`serde 1.0.228`、`serde_json 1.0.150`、`ts-rs 12.0.1`、`clap 4.6.1`、`typescript 6.0.3`、`react 19.2.7`、`@types/react 19.2.17`、`vue 3.5.38` 和 `svelte 5.56.3`。
- 本次验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；release action plan 测试为 7 passed，release_check focused 测试为 38 passed，workspace Rust tests 为 546 passed。`rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed，但成功写出 action plan，且其中 `crate package readiness` 为 `failed` / `required`。

本次 2026-06-11 补强 release action plan 默认 evidence path:

- `release_action_evidence_path()` 现在在未显式传入 `--release-evidence-dir` 时使用与建议命令一致的默认目录 `target/release-evidence`，因此 `release-check --plan` 生成的 machine-readable checklist 会为 release evidence 项写出 `evidence_path`，不再只在 commands 中隐含默认位置。
- 新增 `release_action_plan_uses_default_release_evidence_paths`，覆盖默认 `ci-run-url.txt`、`crate-package/crate-package.json`、macOS/Windows signing log 和 `notary.log` evidence path。真实 `target/release-action-plan-current-require-artifacts.json` 也已确认包含这些默认路径。
- 本次验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；release action plan 测试为 8 passed，release_check focused 测试为 38 passed，workspace Rust tests 为 547 passed。`rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed，但 action plan 已写出默认 evidence paths。

本次 2026-06-11 对齐 validator/static validate action plan 与 release evidence 目录约定:

- `release_action_evidence_path()` 现在把 `vst3 validate reports` / `vst3 example validator coverage` 指向 `target/release-evidence/validator`，把 `vst3 static validate reports` / `ci example static validate coverage` 指向 `target/release-evidence/package`。这与 `release-evidence import-ci` 的规范化输出和 `release-check --release-evidence-dir` 的递归自动发现路径一致。
- 对应建议命令也改为写入这些目录: validator passed report 使用 `target/release-evidence/validator/<bundle>.<platform>.validate.json` 和 validator log；static-only report 使用 `target/release-evidence/package/<bundle>.<platform>.static-validate.json`。这比单个占位 `validate-report.json` / `static-validate-report.json` 更贴近最终要求的三示例乘三平台矩阵。
- `release-check --write-evidence-template` 现在也会创建 `validator/` 和 `package/` 目录；模板 README 明确推荐 framework release 矩阵写入 `validator/<bundle>.<platform>.validate.json` 和 `package/<bundle>.<platform>.static-validate.json`，根目录 `validate-report.json` / `static-validate-report.json` 只保留为 legacy/single-plugin pending template。
- `release_action_plan_uses_default_release_evidence_paths` 现在覆盖 validator/package evidence paths 和命令目录；`release_evidence_templates_do_not_count_as_pass_or_overwrite_logs` 也覆盖模板目录和 README 文案。真实 `target/release-action-plan-current-require-artifacts.json` 已确认这些 action 带 `evidence_path` 和对应命令。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；release evidence template 测试为 3 passed，release action plan 测试为 8 passed，release_check focused 测试为 38 passed，workspace Rust tests 为 547 passed。严格 release-check 仍按预期 failed，失败项仍是真实 DAW/platform/CI/validator/signing/notarization 外部证据缺失。

本次 2026-06-11 补强 validator/package evidence 目录 symlink 边界:

- 新增 `release_evidence_dir_rejects_validator_and_package_symlink_dirs`，覆盖 `--release-evidence-dir` 递归扫描时遇到 `validator/` 或 `package/` symlink 目录会失败，不会跟随外部目录读取 validator/static validate JSON，也不会把路径加入 `validate_reports` / `static_validate_reports`。
- 新增 `release_evidence_templates_reject_validator_and_package_symlink_dirs`，覆盖 `release-check --write-evidence-template` 遇到已有 `validator/` 或 `package/` symlink 时会拒绝写出模板，不会在外部目标中创建 README 或后续 evidence 文件。
- 本次验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_dir_rejects_validator_and_package_symlink_dirs -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_templates_reject_validator_and_package_symlink_dirs -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；release evidence 聚合测试为 26 passed，workspace Rust tests 为 549 passed。

本次 2026-06-11 补强 action plan 总览项 evidence path:

- `release_action_evidence_path()` 现在会为 `daw matrix` 写出 evidence root；未显式传入 `--evidence-root` 时默认 `target/daw-evidence`，显式传入时使用该目录。这样 action plan 顶层矩阵项和每个 `daw smoke: <host>` 子项都具备机器可读证据位置。
- `protocol snapshot` action 现在也会带 `evidence_path`，指向本次 release-check 使用的 protocol snapshot 目录；当 final gate 因 `--skip-protocol` 或 snapshot 缺失而失败时，checklist 可以直接定位到需要生成/复验的 protocol artifact。
- `release_action_plan_lists_required_and_optional_evidence` 和 `release_action_plan_uses_default_release_evidence_paths` 已覆盖显式/default `daw matrix` 路径和 protocol snapshot 路径；真实 `target/release-action-plan-current-require-artifacts.json` 已确认 `daw matrix` 带 `evidence_path: target/daw-evidence`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；release action plan 测试为 8 passed，release_check focused 测试为 38 passed，workspace Rust tests 为 549 passed。

本次 2026-06-11 补强 VST3 SDK `.rs` audit artifact release-check 闭环:

- `release-check` 新增 `--vst3-sdk-scaffold`、`--vst3-sdk-abi-seed`、`--vst3-sdk-abi` 和 `--vst3-sdk-interface-skeleton` 可选输入。缺失时仍为 optional skipped；显式传入或被 `--release-evidence-dir` 标准路径发现时，会复用现有 scaffold/ABI/interface skeleton marker validators 校验 `BINDINGS_GENERATED = false`、`FULL_COM_BINDINGS_GENERATED = false`、ABI layout/interface skeleton 生成标记和关键 metadata。
- `release-check --release-evidence-dir` 现在会自动发现 `vst3-sdk/generated.rs`、`vst3-sdk/generated-abi-seed.rs`、`vst3-sdk/generated-abi.rs` 和 `vst3-sdk/generated-interface-skeleton.rs`；标准槽位存在即进入 release-check，坏文件会显示为 failed，而不是被忽略成 skipped。
- `release_action_evidence_path()` / `release_action_commands()` 现在为四个 optional SDK audit checks 给出标准 evidence path 和 `vesty vst3-sdk emit-*` 建议命令。它们仍只表示 deterministic drift/audit metadata，不证明完整 VST3 SDK 3.8 bindings、callable COM glue、factory exports 或 binary export verification 已生成。
- `release_evidence_readme()`、`vst3_sdk_manifest_evidence_readme()` 和 `.agents/07-build-packaging.md` 已同步更新为当前语义: `release-check` 会在这些 SDK `.rs` audit artifacts 存在时严格校验，但它们不是完整 SDK bindings 或 final release readiness proof。
- 新增/更新测试覆盖 `vst3_sdk_generated_rust_artifact_release_checks_are_optional_but_strict_when_present`、`release_evidence_dir_populates_standard_evidence_paths` 和 `release_action_plan_vesty_commands_parse_with_current_cli`。本次验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；`vesty-cli` 全套为 255 passed，workspace Rust tests 为 550 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并成功写出 report/action plan；新增 VST3 SDK scaffold/ABI/interface skeleton checks 以 optional skipped 出现，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、publish/npm evidence、签名和 notarization evidence 缺失。

本次 2026-06-11 补强 release action plan 的本地复验命令:

- `release_action_commands()` 现在对可本地复验的 evidence 同时输出生成命令和 `--check` 复验命令，包括 `vesty publish-plan`、`vesty crate-package`、`vesty npm-pack`、`vesty dependency-baseline --latest`，以及 `vesty vst3-sdk manifest` / `binding-plan` / `binding-surface` / `emit-scaffold` / `emit-abi-seed` / `emit-abi` / `emit-interface-skeleton`。
- `release_action_plan_lists_required_and_optional_evidence` 和 `release_action_plan_vesty_commands_parse_with_current_cli` 已覆盖新增 check 命令；所有以 `vesty` 开头的建议命令继续由当前 Clap 定义解析，避免 checklist flag drift。
- 该改动只提升 action plan 的采证可执行性，不生成或声称任何外部 release pass evidence；strict release-check 仍需要真实 DAW/platform/CI/validator/signing/notarization artifacts。

本次 2026-06-11 继续补强 CLI 输出目录 no-follow 边界:

- `release-evidence collect-local`、`collect-signing` 和 `collect-notarization` 现在在创建输出 evidence dir 前复用 `create_directory_no_parent_or_leaf_symlink()`，拒绝既有 symlink leaf 或 symlink output ancestor；这样本地采证不会通过 `linked-parent/release-evidence` 写入 bundle 外部目录。
- `platform-smoke --write-report` 现在同样拒绝 symlinked output parents，再写入 `macos.json` / `windows-x64.json` / `linux-x11.json`，补齐 template path 与 report path 的 no-follow 行为。
- `vesty new` 现在用 no-follow path existence check 拒绝已有 symlink 项目路径，并在创建 starter 前拒绝 symlinked project output parent；模板脚手架不会把 Rust/UI starter 写入 symlink 指向的外部 workspace。
- 新增 `platform_smoke_write_report_rejects_symlink_output_parent`、`collect_release_evidence_commands_reject_symlink_output_parents` 和 `create_project_rejects_symlink_output_parent`，均断言外部 symlink 目标没有被创建或覆盖。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli symlink_output_parent -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke -- --nocapture`、`rtk cargo test -p vesty-cli collect_release_evidence_commands_reject_symlink_output_parents -- --nocapture`、`rtk cargo test -p vesty-cli create_project_rejects_symlink_output_parent -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；`vesty-cli` 为 262 passed，workspace Rust tests 为 557 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并成功写出 report/action plan；本地可证明项为 ok，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-11 补强 `vesty-build` 打包输出 no-follow 边界:

- `package_vst3()` 现在创建 `*.vst3/Contents/Resources`、platform binary parent 和 UI asset output directory 时使用 no-follow output directory helper，拒绝既有 symlink output ancestor 或 symlink leaf；`--out` 或 bundle root 是 symlink 时不会把 `.vst3` 写入外部目标。
- 重新打包已有 bundle 时，`Contents/Resources/ui` 删除前改用 `symlink_metadata()` 检查；如果 UI output dir 被替换成 symlink，会返回 `BuildError::SymlinkAsset`，不会跟随删除外部目录。
- `copy_dir_recursive()` 的 destination 创建也复用同一 helper，补齐 UI asset copy 输出侧边界。输入侧原有 UI dist root / asset symlink / canonical root 检查保持不变。
- 新增 `package_rejects_symlinked_output_dir` 和 `package_rejects_existing_symlinked_ui_output_dir`，覆盖 symlinked `--out` 与既有 symlinked packaged UI output，并断言外部目标未被创建、删除或覆盖。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-build package_rejects_symlinked_output_dir -- --nocapture`、`rtk cargo test -p vesty-build package_rejects_existing_symlinked_ui_output_dir -- --nocapture`、`rtk cargo test -p vesty-build -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；`vesty-build` 为 73 passed，workspace Rust tests 为 559 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地可证明项为 ok，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-11 继续补强 `vesty-build` 打包输出文件 no-follow 边界:

- `package_vst3()` 写入 platform binary、macOS `Info.plist` / `PkgInfo`、`moduleinfo.json`、`parameters.manifest.json` 和 `assets.manifest.json` 时现在统一使用 no-follow output file/copy helpers。既有输出文件是 symlink 时会返回 `BuildError::SymlinkAsset`，不会跟随覆盖外部目标。
- `write_macos_plist()` 改为先用 `plist::to_writer_xml()` 序列化到内存，再通过 no-follow writer 落盘，避免 `plist::to_file_xml()` 直接跟随既有 symlinked `Info.plist`。
- `copy_dir_recursive()` 复制 UI assets 时也使用 no-follow file copy helper，补齐 manifest-listed asset output 文件槽位的保护。
- 新增 `package_rejects_existing_symlinked_output_files` 覆盖 platform binary、`Info.plist`、`PkgInfo`、`moduleinfo.json` 和 `assets.manifest.json`；新增 `package_rejects_existing_symlinked_parameter_manifest_output` 覆盖配置了 `[package].parameter_manifest` 时的 `parameters.manifest.json` 输出槽位。测试均断言外部 symlink 目标内容未被覆盖。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-build package_rejects_existing_symlinked_output_files -- --nocapture`、`rtk cargo test -p vesty-build package_rejects_existing_symlinked_parameter_manifest_output -- --nocapture`、`rtk cargo test -p vesty-build -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；`vesty-build` 为 75 passed，workspace Rust tests 为 561 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地可证明项为 ok，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-11 补强 `vesty package --install-dev` copy 模式 no-follow 边界:

- `install_dev_bundle()` 现在拒绝 symlinked source `.vst3` bundle，并在创建 `--vst3-dir` 前复用 no-follow output directory helper，避免 copy-mode 安装目录通过 symlink ancestor 指向外部目标。
- `remove_existing_dev_bundle()` 改用 `symlink_metadata()` / NotFound 分支，不再用 `path.exists()` 跟随 symlink；旧 destination 是 symlink 时只删除 symlink 本身，dangling symlink 也能被清理。
- `copy_bundle_dir()` 现在创建 destination directory 和拷贝文件前复用 no-follow helpers；source bundle 内 symlink entry 仍被拒绝。
- 新增 `install_dev_bundle_rejects_symlinked_source_bundle`、`install_dev_bundle_rejects_symlinked_install_dir_parent` 和 `install_dev_bundle_unlinks_existing_destination_symlink_without_following_it`，覆盖 source root、install dir parent 和既有 destination symlink 三条路径，并断言外部目标未被创建、删除或覆盖。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli install_dev_bundle -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；install-dev focused 为 5 passed，`vesty-cli` 为 265 passed，workspace Rust tests 为 564 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地可证明项为 ok，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-11 补强 `collect-signing` 输入 no-follow 边界:

- `infer_signing_bundle_platform()` 和 `signing_verification_command()` 现在先用 `symlink_metadata()` 验证 `.vst3` bundle root 是真实目录而不是 symlink，再做平台推断或签名验证命令构造；symlinked bundle 不会被传给 `codesign` / `signtool`。
- Windows 签名验证的自动 binary 推断现在要求 `Contents/x86_64-win` 是真实目录而不是 symlink；显式 `--binary` 也必须是真实文件而不是 symlink。
- 新增 `signing_verification_rejects_symlinked_bundle_root`、`signing_verification_rejects_symlinked_windows_payload_dir` 和 `signing_verification_rejects_symlinked_explicit_windows_binary`，覆盖 bundle root、payload dir 和显式 binary 三条输入路径。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing_verification -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；signing_verification focused 为 4 passed，signing focused 为 21 passed，`vesty-cli` 为 268 passed，workspace Rust tests 为 567 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地可证明项为 ok（host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report、dependency latest baseline），失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 补强 `collect-signing --binary` bundle 归属边界:

- `signing_verification_command()` 现在对 Windows explicit `--binary` 复用 `require_windows_signing_binary_in_bundle()`，要求该 binary 是真实 `.vst3` 文件，且 canonical path 必须位于目标 bundle 的 `Contents/x86_64-win` 目录下。
- 该检查会拒绝三类误采证路径: bundle 外部 signed binary、非 `.vst3` 文件、以及 `Contents/x86_64-win` 内通过中间 symlink 跳到 bundle 外部的 binary。这样 `collect-signing <bundle.vst3> --platform windows-x64 --binary <path>` 不会把另一个 artifact 的 signtool verify 结果挂到当前 bundle release evidence 上。
- 新增 `signing_verification_rejects_explicit_windows_binary_outside_bundle`、`signing_verification_rejects_explicit_windows_binary_wrong_extension` 和 `signing_verification_rejects_explicit_windows_binary_through_payload_symlink`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing_verification -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；signing_verification focused 为 7 passed，signing focused 为 24 passed，`vesty-cli` 为 271 passed，workspace Rust tests 为 570 passed，clippy 无 warning，JS workspace tests passed。

本次 2026-06-12 补强 `collect-signing --binary` 平台语义:

- macOS 签名采证固定验证整个 `.vst3` bundle；`signing_verification_command(BundlePlatform::Macos, ..., Some(binary), ...)` 现在直接报错，避免 `--binary` 被静默忽略后让采证者误以为某个具体 Mach-O binary 已被单独验证。
- Linux 仍保持 release-channel-specific，不接受 `collect-signing` 伪造 bundle signing evidence；显式传入 binary 也不会改变该错误语义。
- 新增 `signing_verification_rejects_macos_binary_argument` 和 `signing_verification_linux_remains_release_channel_specific_with_binary`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing_verification -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；signing_verification focused 为 9 passed，signing focused 为 26 passed，`vesty-cli` 为 273 passed，workspace Rust tests 为 572 passed，clippy 无 warning，JS workspace tests passed。

本次 2026-06-12 收紧 Windows signing evidence parser:

- `signing_evidence_platforms_from_text()` 不再把 `Successfully signed` 自动计为 Windows signtool verification evidence；该输出只能说明执行过签名动作，不能证明 release 要求的 `signtool verify` 已经通过。
- Windows signing evidence 仍接受显式 `signtool=pass` marker、`Successfully verified`、以及带 `signtool` 上下文且 `Number of errors: 0` 的 verify summary；负向 `SignTool Error` 或非零 error count 仍优先失败。
- 新增 `signing_evidence_rejects_signtool_sign_without_verify`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing_evidence -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；signing_evidence focused 为 13 passed，signing focused 为 27 passed，`vesty-cli` 为 274 passed，workspace Rust tests 为 573 passed，clippy 无 warning，JS workspace tests passed。

本次 2026-06-12 补强 macOS signed bundle evidence 内部路径 no-follow:

- `validate_macos_code_resources()` 现在先要求 `.vst3/Contents` 和 `.vst3/Contents/_CodeSignature` 都是真实目录而不是 symlink，再要求 `CodeResources` 是真实文件；防止 release evidence bundle 中的 macOS `.vst3` 目录通过内部父目录 symlink 指向外部可替换 plist。
- 现有 leaf `CodeResources` symlink 拒绝保持不变，16 MiB size cap 和 plist `files` / `files2` dictionary 校验也保持不变。
- 新增 `signing_evidence_rejects_symlinked_code_signature_directory` 和 `signing_evidence_rejects_symlinked_contents_directory`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing_evidence -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；signing_evidence focused 为 15 passed，signing focused 为 29 passed，`vesty-cli` 为 276 passed，workspace Rust tests 为 575 passed，clippy 无 warning，JS workspace tests passed。

本次 2026-06-12 补强 `collect-signing --tool` no-follow 边界:

- `signing_verification_command()` 现在通过 `signing_verification_program()` 统一选择 macOS `codesign` 和 Windows `signtool` 验证程序；显式 `--tool <path>` 会在命令构造前拒绝 symlinked tool leaf 和 symlinked parent path。
- 默认工具查找语义保持不变: 未传 `--tool` 时仍先查固定候选路径，再回退到 bare `codesign` / `signtool.exe` 命令，避免把 PATH lookup 错当成可被 no-follow 校验的文件路径。
- 新增 `signing_verification_rejects_symlinked_explicit_tool` 和 `signing_verification_rejects_symlinked_explicit_tool_parent`，覆盖显式工具 leaf symlink 与 parent symlink 两条 trust-boundary 输入路径。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing_verification -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；signing_verification focused 为 11 passed，signing focused 为 31 passed，`vesty-cli` 为 278 passed，workspace Rust tests 为 577 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地可证明项为 ok，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 补强 VST3 SDK interface skeleton binary export required-symbol helper:

- `generated_bindings_interface_skeleton_module()` 现在在 `BINARY_EXPORT_SYMBOL_PLANS` 之外额外生成 `BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED = true` 和 `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`，明确区分“可校验 expected symbol set”与“尚未读取/检查真实二进制”。
- 生成模块新增 `binary_export_symbol_plan_by_platform_and_symbol()`、`required_binary_export_symbol_count()`、`first_missing_binary_export_symbol()` 和 `binary_export_required_symbols_present()` 纯 helper，可在后续 generated backend 或 CI tooling 中复用同一份 per-platform required export symbol plan。
- `vesty-cli` 的 interface skeleton validator 已要求这些 marker/helper 存在；旧的仅包含 `BINARY_EXPORT_SYMBOL_PLANS` 但缺少 required-symbol helper 的 `generated-interface-skeleton.rs` 会被 `emit-interface-skeleton --check`、`import-ci` 或 `release-check` 拒绝。
- 该改动仍不生成 callable Steinberg COM/API methods、queryInterface glue、factory/module exports，也不执行 `nm` / `dumpbin` / `llvm-objdump` 二进制 inspection；最终 release gate 仍以真实 `vesty validate` binary export evidence 为准。
- 验证通过 `rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture` 和 `rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`；`vesty-vst3-sys` generated_bindings focused 为 13 passed，`vesty-cli` vst3_sdk focused 为 24 passed。

本次 2026-06-12 将 binary export required-symbol plan 提升为 `vesty-vst3-sys` public API:

- `vesty-vst3-sys` 现在暴露 `BinaryExportSymbolPlan`、`binary_export_symbol_plans()`、`binary_export_symbol_plan()`、`required_binary_export_tool_symbols()`、`required_binary_export_symbol_count()`、`first_missing_binary_export_symbol()` 和 `binary_export_required_symbols_present()`。这些 API 复用同一份 `GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS`，覆盖 Windows x64 `GetPluginFactory` / `InitDll` / `ExitDll`，macOS `_GetPluginFactory` / `_bundleEntry` / `_bundleExit` / `_BundleEntry` / `_BundleExit`，Linux x64 `GetPluginFactory` / `ModuleEntry` / `ModuleExit`。
- `vesty-cli` 的 validate/static/release report 校验现在通过 `vesty_vst3_sys::required_binary_export_tool_symbols()` 获取 expected symbol set，不再维护独立硬编码表，减少 generated SDK audit plan 与真实 validate gate 漂移风险。
- 新增 `binary_export_symbol_plan_public_api_matches_required_symbol_helpers` 和 `validate_report_binary_export_expectations_use_vst3_sys_plan`，分别覆盖 sys public plan/helper 一致性，以及 CLI wrapper 与 sys single source of truth 一致。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-vst3-sys binary_export -- --nocapture`、`rtk cargo test -p vesty-cli validate_report_binary_export_expectations_use_vst3_sys_plan -- --nocapture` 和 `rtk cargo test -p vesty-cli validate_report -- --nocapture`。
- 该改动仍不执行真实 binary inspection，也不替代 `vesty validate --strict` 生成的 `static_check.binary_exports` evidence。

本次 2026-06-12 继续将 `vesty-build` binary export validation 收敛到同一份计划:

- `vesty-build::validate_vst3_bundle()` / `BinaryExportCheck` 使用的 `required_export_symbols()` 现在同样委托给 `vesty_vst3_sys::required_binary_export_tool_symbols(platform_slug)`。打包静态校验、CLI validate/release report gate 和 VST3 SDK interface skeleton audit 不再各自维护 per-platform required export symbol list。
- `vesty-vst3-sys` 现在也暴露 `BinaryExportInspectionToolPlan`、`binary_export_inspection_tool_plans()` 和 `binary_export_inspection_tools(platform)`。`vesty-build` 的真实 export-symbol inspection tool 顺序改为复用该计划，覆盖 macOS `nm -gU` / `llvm-nm -gU`、Windows x64 `llvm-objdump -p` / `dumpbin /exports`、Linux x64 `nm -D --defined-only` / `llvm-nm -D --defined-only`。
- `vesty-cli` 的 validate/static report test fixture 也改为从同一个 public helper 生成 fake `BinaryExportCheck.required_symbols` / `found_symbols`，避免测试数据继续携带一份旧 hard-coded symbol table。
- workspace 级 `vesty-vst3-sys` 依赖默认关闭 features；`vesty-build` 只依赖 metadata/helper API，不会为了静态打包校验拉入上游 `vst3` binding。`vesty-cli` 和 `vesty-vst3` 显式启用 `upstream-vst3`，继续保留当前 active binding baseline。
- 新增 `binary_export_validation_uses_vst3_sys_required_symbol_plan` 和 `binary_export_validation_uses_vst3_sys_inspection_tool_plan`，覆盖 macOS、Windows x64 和 Linux x64 三个平台的 build-layer required symbol list、inspection tool list、`BinaryExportCheck.required_symbols` 与 `vesty-vst3-sys` public helper 一致。

本次 2026-06-12 将 binary export inspection tool plan 纳入 interface skeleton audit artifact:

- `vesty vst3-sdk emit-interface-skeleton` 生成的 `generated-interface-skeleton.rs` 现在包含 `BinaryExportInspectionToolPlan`、`BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT`、`BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE`、`BINARY_EXPORT_INSPECTION_TOOL_PLANS` 和 `binary_export_inspection_tools(platform)`。该 metadata 固定后续 `vesty validate`/CI 可使用的 export-symbol inspection tool 顺序，但仍保持 `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`，不表示真实二进制已经被检查。
- `vesty-cli` 的 interface skeleton validator 已要求这些 marker/helper 存在；旧的只包含 symbol plan 而缺少 inspection tool plan 的 `generated-interface-skeleton.rs` 会被 `emit-interface-skeleton --check`、`import-ci` 或 `release-check` 拒绝。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture` 和 `rtk cargo test -p vesty-cli import_ci -- --nocapture`。

本次 2026-06-12 同步 release evidence 模板测试与 binary export 术语:

- `release_evidence_templates_do_not_count_as_pass_or_overwrite_logs` 不再断言旧的 generated-binary-export wording，改为要求 README 模板明确包含 `pure required-symbol checks`、`binary inspection tooling` 以及 `does not inspect plugin binaries and does not create DAW, platform smoke`，防止模板把 pure helper seed 误写成真实二进制 inspection/pass evidence。
- `.agents/04-vst3-adapter.md` 和 `.agents/08-developer-guide.md` 已同步移除旧 generated-binary-export wording，统一描述为 future binary inspection expected-name plan、required-symbol helper seed 和尚未实现的 binary inspection tooling。
- 验证通过 `rtk cargo fmt --all --check`、`rtk npm test`、`rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1` 和 `rtk cargo clippy --workspace --all-targets -- -D warnings`；focused release_evidence_templates 为 4 passed，`vesty-cli` 为 278 passed，workspace Rust tests 为 577 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；ok 项为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 增加 `vesty validate --strict` binary export evidence gate:

- `vesty validate` 新增 `--strict` flag。默认行为保持兼容: static validation 会把导出符号工具缺失或无法解析记录为 `static_check.binary_exports[].status = "skipped"` 并继续写 report；显式 `--strict` 时，命令会在 report 写出后要求每个可识别平台 binary 都有匹配的 `ok` binary export evidence，缺失/skipped/incomplete found-symbol 证据都会返回非零。
- `strict_static_bundle_check_error()` 复用 validate report 的平台推断、路径 normalization 和 `binary_export_check_proves_platform()` 语义，避免 `./Bundle.vst3/...` 等合法 report path 被误判，也避免 skipped export evidence 被 package CI 当作通过。
- release evidence README 模板、release action plan 建议命令和 static validate 缺失/失败 hint 已更新为推荐 `vesty validate --static-only --strict --report <path>`；`release_action_plan_uses_default_release_evidence_paths` 和 `release_action_plan_vesty_commands_parse_with_current_cli` 会覆盖该建议命令仍可由当前 Clap 解析。
- `.agents/07-build-packaging.md` 和 `.agents/08-developer-guide.md` 已同步说明默认宽松诊断与 `--strict` package/release CI 门禁的差异；`--strict` 只是让本地/CI 更早失败，不替代 Steinberg validator、DAW matrix、platform smoke、签名或 notarization evidence。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli strict_validate -- --nocapture`、`rtk cargo test -p vesty-cli validate_command_accepts_strict_flag -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；`vesty-cli` 为 280 passed，workspace Rust tests 为 579 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 对齐 GitHub Actions package smoke 与 `vesty validate --strict`:

- `.github/workflows/ci.yml` 的 `Static validate packaged bundles` step 现在对 `VestyGain.vst3`、`VestyWebUIDemo.vst3` 和 `VestyMIDISynth.vst3` 三个 package smoke report 都传入 `--static-only --strict`。因此导出符号工具缺失、无法解析或 skipped binary export evidence 会在三平台 package matrix 阶段直接失败，同时仍保留 JSON report 供 artifact 上传和诊断。
- 新增 `ci_package_static_validate_uses_strict_binary_export_gate` 回归测试，直接读取 `.github/workflows/ci.yml`，要求 package static validate step 中存在三条 `cargo run -p vesty-cli -- validate`、三处 `--static-only` 和三处 `--strict`，并确认三示例 bundle/report 路径没有漂移。
- release evidence README 模板和 pending static validate report error/hint 已同步为 `vesty validate --static-only --strict --report`，避免模板继续推荐会产生 skipped export diagnostics 的宽松命令。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_package_static_validate_uses_strict_binary_export_gate -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture`、`rtk ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'`、`rtk go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；`vesty-cli` 为 281 passed，workspace Rust tests 为 580 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地 ok 项不变，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 补齐 release action plan strict static validate 回归覆盖:

- `release_action_plan_uses_default_release_evidence_paths` 现在同时断言 `vst3 static validate reports` 和 `ci example static validate coverage` 两个 action 都指向 `target/release-evidence/package`，且建议命令包含 `vesty validate <bundle.vst3> --static-only --strict`。这把 release action plan、CI package smoke 和 `vesty validate --strict` 的采证语义锁在同一条回归线上。
- 运行中发现一次无效验证命令: `cargo test` 只接受单个 test filter，组合传入多个 filter 会报 `unexpected argument`。随后已分别运行三个 focused tests 并通过；这不是代码失败。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan_uses_default_release_evidence_paths -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan_vesty_commands_parse_with_current_cli -- --nocapture`、`rtk cargo test -p vesty-cli strict_validate_requires_ok_binary_export_evidence -- --nocapture`、`rtk cargo test -p vesty-cli validate_command_accepts_strict_flag -- --nocapture`、`rtk cargo test -p vesty-cli ci_package_static_validate_uses_strict_binary_export_gate -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'` 和 `rtk go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml`；`vesty-cli` 为 281 passed，workspace Rust tests 为 580 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地 ok 项为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 对齐 validator-passed release evidence 与 `vesty validate --strict`:

- `release_action_commands()` 现在对 `vst3 validate reports` 和 `vst3 example validator coverage` 生成 `vesty validate <bundle.vst3> --strict --format json --report ... --validator-log ...`。这样 Steinberg validator-passed JSON 在采集时也要求匹配平台的 `ok` `static_check.binary_exports`，不会等到最终 `release-check --require-release-artifacts` 才发现 export-symbol evidence 是 skipped 或缺失。
- release evidence README 模板、`validate-report.json` pending error、`validate_reports_release_check()` missing/failure hints、`.agents/07-build-packaging.md`、`.agents/08-developer-guide.md` 和 `.agents/09-crash-safety-and-testing.md` 已同步推荐 `vesty validate --strict --report` 作为 release validator evidence 采集命令；普通开发中的 `vesty validate --report` 保存 report 语义仍可用，但不再作为 release 采证推荐路径。
- `release_action_plan_uses_default_release_evidence_paths` 新增 validator action strict 断言，`release_evidence_templates` 新增 README strict validator wording 断言，`release_action_plan_vesty_commands_parse_with_current_cli` 继续保证生成的 `vesty ...` 建议命令可被当前 CLI 解析。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan_uses_default_release_evidence_paths -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan_vesty_commands_parse_with_current_cli -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'` 和 `rtk go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml`；`vesty-cli` 为 281 passed，workspace Rust tests 为 580 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地 ok 项为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 release-check validator/static coverage hint:

- `example_validate_coverage_release_check()` 在缺少 validator-passed matrix 或 matrix 不完整时，hint 现在直接包含 `vesty validate --strict --report <path>`；`example_static_validate_coverage_release_check()` 在缺少 CI static/package matrix 或 matrix 不完整时，hint 现在直接包含 `vesty validate --static-only --strict --report <path>`。最终 `release-check --require-release-artifacts` 输出也会把采证者引向 strict evidence 命令。
- 新增/更新断言覆盖 validator 3x3 缺平台、static 单平台缺 bundle、static 3x3 缺平台三个分支，避免以后 release-check failure hint 回退为宽松命令或模糊说明。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli example_validate_coverage_requires_all_release_platforms -- --nocapture`、`rtk cargo test -p vesty-cli example_static_validate_coverage_rejects_partial_platform_matrix -- --nocapture`、`rtk cargo test -p vesty-cli example_static_validate_coverage_requires_all_release_platforms -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'` 和 `rtk go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml`；`vesty-cli` 为 281 passed，workspace Rust tests 为 580 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；输出中的 `vst3 validate reports`、`vst3 example validator coverage`、`vst3 static validate reports` 和 `ci example static validate coverage` hint 均指向 strict validate/static validate 采证路径。本地 ok 项仍为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline；失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 完整验证与 release action plan 3x3 命令复核:

- 在 validator/static matrix action-plan 命令补齐后，重新跑完整本地门禁: `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'` 和 `rtk go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml` 均通过；`vesty-cli` 为 281 passed，workspace Rust tests 为 580 passed，clippy 无 warning，JS workspace tests passed。
- 重新运行 `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json`；该命令仍按预期以 exit code 1 失败并写出 report/action plan，避免在缺少真实外部 evidence 时误宣称 release-ready。
- 生成的 `target/release-action-plan-current-require-artifacts.json` summary 为 7 ok、16 failed、7 skipped、23 actions。`vst3 validate reports`、`vst3 example validator coverage`、`vst3 static validate reports` 和 `ci example static validate coverage` 四个 action 每个都有 11 条 suggested commands: 1 条通用采证命令、1 条矩阵说明、9 条覆盖 VestyGain/VestyWebUIDemo/VestyMIDISynth × macOS/Windows x64/Linux x64 的具体命令。validator action 使用 `vesty validate ... --strict --format json --report ... --validator-log ...`；static action 使用 `vesty validate ... --static-only --strict --format json --report ...`。
- 当前 report 中本地 ok 项为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline；失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 补强 release action plan 3x3 matrix 回归测试:

- `release_action_plan_uses_default_release_evidence_paths` 现在不只检查 strict 命令存在，还会用 `REQUIRED_EXAMPLE_BUNDLES`、`REQUIRED_EXAMPLE_VALIDATE_PLATFORMS` 和 `REQUIRED_EXAMPLE_STATIC_VALIDATE_PLATFORMS` 逐项断言 validator/static action plan 中每个 VestyGain/VestyWebUIDemo/VestyMIDISynth × macOS/Windows x64/Linux x64 命令精确出现一次。
- validator matrix 断言要求每条 concrete command 包含 `vesty validate <path-to-...> --strict`、对应 `target/release-evidence/validator/<bundle>.<platform>.validate.json`、对应 validator log，并且不带 `--static-only`；static matrix 断言要求每条 concrete command 包含 `--static-only --strict`、对应 `target/release-evidence/package/<bundle>.<platform>.static-validate.json`，并且不带 `--validator-log`。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan_uses_default_release_evidence_paths -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan_vesty_commands_parse_with_current_cli -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture` 和 `rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`；`vesty-cli` 仍为 281 passed，focused action-plan tests passed，clippy 无 warning。

本次 2026-06-12 继续收紧 generic validator action matrix 覆盖:

- `release_action_plan_uses_default_release_evidence_paths` 现在同时对 `vst3 validate reports` 和 `vst3 example validator coverage` 两个 validator action 运行同一套 3x3 exact-once 断言，避免通用 validator 采证 action 与 example coverage action 之间发生命令模板漂移。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan_uses_default_release_evidence_paths -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan_vesty_commands_parse_with_current_cli -- --nocapture` 和 `rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`。

本次 2026-06-12 收紧 action-plan 命令解析测试:

- `release_action_plan_vesty_commands_parse_with_current_cli` 已移除独立测试 parser helper，改为复用生产侧 `split_release_action_command()`。这样 action-plan sidecar 校验和测试命令解析走同一套 quote/comment/unterminated-quote 逻辑，避免测试 helper 与真实 import/write 校验发生漂移。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan_vesty_commands_parse_with_current_cli -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan_uses_default_release_evidence_paths -- --nocapture`、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、CI YAML parse 和 actionlint；`vesty-cli` 为 281 passed，workspace Rust tests 为 580 passed，clippy 无 warning，JS workspace tests passed。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；action-plan summary 仍为 7 ok、16 failed、7 skipped、23 actions，四个 validator/static action 均为 11 条命令。本地 ok 项仍为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline；失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 继续收紧 action-plan 命令识别测试:

- `release_action_plan_vesty_commands_parse_with_current_cli` 现在连“是否是 vesty 命令”的判定也复用生产侧 `release_action_command_starts_with_vesty()`，并在测试循环中先 trim command。这样带前导空白的 `vesty ...` 建议命令会被测试覆盖，测试与 sidecar 校验的命令识别边界保持一致。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan_vesty_commands_parse_with_current_cli -- --nocapture`、`rtk cargo test -p vesty-cli release_action_plan_sidecar_rejects_stale_vesty_commands -- --nocapture` 和 `rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`。

本次 2026-06-12 补强 VST3 optional sidechain Sample64 本地覆盖:

- 新增两个 `vesty-vst3` fake-host 测试: `processor_runs_optional_sidechain_effect_sample64_with_main_input_only` 和 `processor_runs_optional_sidechain_effect_sample64_with_empty_inactive_sidechain_input`。它们覆盖 optional sidechain effect 在 `kSample64` f64->f32 scratch fallback 下，只收到 main input bus、以及收到 trailing empty inactive sidechain bus 时，仍能零分配运行 main path、写出预期输出并清理 output silence flags。
- 这补齐了 sample32 optional sidechain main-only/empty-inactive 测试在 Sample64 scratch fallback 上的同构证据；仍不替代真实 DAW sidechain routing/activation smoke。
- 验证通过 `rtk cargo test -p vesty-vst3 --features vst3-bindings processor_runs_optional_sidechain_effect_sample64 -- --nocapture`、`rtk cargo test -p vesty-vst3 --features vst3-bindings processor_routes_sidechain_bus_through_sample64_scratch_fallback -- --nocapture`、`rtk cargo test -p vesty-vst3 --features vst3-bindings -- --nocapture`、`rtk cargo test -p vesty-vst3 --features "vst3-bindings wry-ui" -- --nocapture`、`rtk cargo clippy -p vesty-vst3 --all-targets --features vst3-bindings -- -D warnings` 和 `rtk cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings wry-ui" -- -D warnings`；`vesty-vst3` feature 测试分别为 72 passed 和 79 passed，clippy 无 warning。

本次 2026-06-12 补强 VST3 native f64 sidechain 本地覆盖:

- 新增 `NativeF64SidechainPlugin` / `NativeF64SidechainKernel` 测试 fixture，以及 `processor_routes_sidechain_bus_through_native_sample64_process` fake-host 测试。该测试覆盖 `AudioKernel::SUPPORTS_F64 = true` 时，`kSample64` process path 不进入 f32 fallback，直接通过 `ProcessContext64` 读取 main input 与 sidechain input，并在 NoAllocGuard 下写出 f64 output。
- 测试断言 native f64 sidechain kernel 进入、f32 fallback 未进入、NoAllocGuard active、实时分配计数为 0、输出样本和 silence flags 正确。这补齐了 Sample64 scratch fallback sidechain 之外的原生双精度 sidechain 组合路径；仍需真实 DAW/平台补采 native f64 sidechain smoke。
- 验证通过 `rtk cargo test -p vesty-vst3 --features vst3-bindings processor_routes_sidechain_bus_through_native_sample64_process -- --nocapture`、`rtk cargo clippy -p vesty-vst3 --all-targets --features vst3-bindings -- -D warnings`、`rtk cargo test -p vesty-vst3 --features vst3-bindings -- --nocapture`、`rtk cargo test -p vesty-vst3 --features "vst3-bindings wry-ui" -- --nocapture` 和 `rtk cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings wry-ui" -- -D warnings`；`vesty-vst3` feature 测试分别为 73 passed 和 80 passed，clippy 无 warning。

本次 2026-06-12 收紧 validate report 路径规范化和重复证据检测:

- `validate_static_bundle_check_shape()` 现在会在原始文本重复检查之外，对 `static_check.binaries` 和 `static_check.binary_exports` 使用同一套 report path key 做重复检测。`Gain.vst3/...`、`./Gain.vst3/...` 和反斜杠写法等价时会被视为同一个 binary/export evidence，避免重复路径变体把 validator/static coverage 看起来抬高。
- `validate_report_paths_self_consistent()` 和 `strict_static_bundle_check_error()` 也复用该规范化 key 来匹配 binary 与 export check。这样正常的 dot-prefixed 或 Windows-style separator report 不会被误伤，同时 export check 不能通过拼写变体绕开 `static_check.binaries` membership。
- 新增 `validate_report_rejects_duplicate_paths_after_normalization` 和 `strict_validate_matches_binary_exports_after_path_normalization`。验证通过 `rtk cargo fmt --all --check`、两条 focused tests、`rtk cargo test -p vesty-cli -- --nocapture` 和 `rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`；`vesty-cli` 当前为 283 passed。

本次 2026-06-12 收紧 example validator/static release matrix 的单平台证据语义:

- `example_validate_coverage_release_check()` 和 `example_static_validate_coverage_release_check()` 现在要求每个 Vesty example coverage report 只能从 `static_check.binaries` 推断出一个 release platform。单份 report 混入 macOS/Windows/Linux 多平台 binary 时会失败，不能把一次 validator/static report 拼成多个平台 coverage。
- coverage gate 还会检查 report 文件名中的平台标签；如果文件名包含 `windows-x64` 但 static binary path 只能推断为 `macos`，该 report 会被拒绝。这样 `target/release-evidence/validator/<bundle>.<platform>.validate.json` 和 `package/<bundle>.<platform>.static-validate.json` 的路径约定与 report 内容保持一致。
- 新增 `example_validator_coverage_rejects_multi_platform_report`、`example_static_coverage_rejects_multi_platform_report` 和 `example_coverage_rejects_report_file_name_platform_mismatch`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli example_validate_coverage -- --nocapture`、`rtk cargo test -p vesty-cli example_static_validate_coverage -- --nocapture`、`rtk cargo test -p vesty-cli release_check_requires_example_validator_reports_for_all_release_platforms -- --nocapture`、`rtk cargo test -p vesty-cli validate_report -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir_discovers_validate_reports_by_content -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture` 和 `rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`；`vesty-cli` 当前为 286 passed，clippy 无 warning。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed 并写出 report/action plan；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 report 文件名平台标签解析:

- `validate_report_file_name_platform()` 不再用去分隔符后的子串匹配识别平台，而是把文件名切为 ASCII token 并匹配完整平台 token 序列，例如 `windows-x64` 必须以相邻 `windows` / `x64` token 出现。这样 `mywindowsx64note` 不会被误判为 Windows evidence 标签，但 `windows-x64` / `windows_x64` 仍会被识别。
- 新增 `example_coverage_platform_file_name_match_uses_tokens_not_substrings`。验证通过 `rtk cargo test -p vesty-cli example_coverage_platform_file_name_match_uses_tokens_not_substrings -- --nocapture`、`rtk cargo test -p vesty-cli example_coverage_rejects_report_file_name_platform_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo fmt --all --check`；`vesty-cli` 当前为 287 passed，clippy 无 warning。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项不变，失败项仍是真实 DAW/CI/platform/validator/static/signing/notarization evidence 缺失。

本次 2026-06-12 收紧 example validator/static matrix 重复报告检测:

- `example_validate_coverage_release_check()` 和 `example_static_validate_coverage_release_check()` 现在会拒绝重复的同一 `bundle@platform` evidence。重复的 `VestyGain.vst3@macos` validator report 或 `VestyWebUIDemo.vst3@windows-x64` static report 不再被 `BTreeSet` 静默去重，而是明确 failed，避免采证目录中同一 evidence 被复制多份造成歧义。
- 新增 `example_validator_coverage_rejects_duplicate_bundle_platform_reports` 和 `example_static_coverage_rejects_duplicate_bundle_platform_reports`。验证通过 `rtk cargo test -p vesty-cli example_validator_coverage_rejects_duplicate_bundle_platform_reports -- --nocapture`、`rtk cargo test -p vesty-cli example_static_coverage_rejects_duplicate_bundle_platform_reports -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo fmt --all --check`；`vesty-cli` 当前为 289 passed，clippy 无 warning。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项不变，失败项仍是真实 DAW/CI/platform/validator/static/signing/notarization evidence 缺失。

本次 2026-06-12 收紧 example validator/static report 文件名 bundle 标签自洽性:

- `example_validate_coverage_release_check()` 和 `example_static_validate_coverage_release_check()` 现在会解析 report 文件名中的已知 example bundle 标签；如果文件名表明 `VestyGain.vst3`，但 JSON `report.bundle` 是 `VestyMIDISynth.vst3`，该 report 会失败，不能通过重命名文件把另一个 bundle 的 validator/static evidence 计入 coverage。
- bundle 标签识别与平台标签一样使用 ASCII token 序列匹配，并额外支持 compact bundle token。`VestyGain` / `Vesty-Gain` / `Vesty_Gain` 会被识别为完整标签，`MyVestyGainNote` 不会被子串误判；同一文件名中出现多个已知 example bundle 标签时也会失败，避免 `VestyGain.VestyMIDISynth.macos.validate.json` 这类歧义 evidence。
- 新增 `example_validator_coverage_rejects_report_file_name_bundle_mismatch`、`example_static_coverage_rejects_report_file_name_bundle_mismatch` 和 `example_coverage_bundle_file_name_match_uses_tokens_not_substrings`，并在 token test 中覆盖 dashed/underscored bundle label。验证通过这三条 focused tests、`rtk cargo test -p vesty-cli -- --nocapture`、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo fmt --all --check`；`vesty-cli` 当前为 292 passed，clippy 无 warning。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 刷新最新依赖 evidence:

- `rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-current/dependency-baseline-latest.json --format text` 通过，当前 Cargo workspace 依赖基线和 crates.io latest checks 均为 ok，包括 `wry 0.55.1`、`vst3 0.3.0`、`raw-window-handle 0.6.2`、`rtrb 0.3.4`、`serde 1.0.228`、`serde_json 1.0.150`、`ts-rs 12.0.1`、`clap 4.6.1`，以及 npm registry latest `typescript 6.0.3`、`react 19.2.7`、`@types/react 19.2.17`、`vue 3.5.38`、`svelte 5.56.3`。
- 刷新的 `target/dependency-baseline-current/dependency-baseline-latest.json` 被 `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 接收为 `dependency latest baseline = ok`。strict gate 仍按预期 failed，失败项仍是真实 DAW/CI/platform/validator/static/signing/notarization evidence 缺失。
- 最新完整本地验证通过 `rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 当前为 594 passed，Clippy 无 warning，JS workspace tests passed。

本次 2026-06-12 收紧 wry release WebView URL allowlist:

- `vesty-ui-wry` 的 release navigation / IPC allowlist 现在只允许 `vesty://assets/...` custom protocol asset URL；`about:blank` 仍只允许用于 navigation 生命周期，IPC 不接受。旧的 `http://vesty.assets/...` / `https://vesty.assets/...` shim origin 已被拒绝，release 模式不再为 bundle assets 暴露 HTTP(S) 同名 origin。
- 更新 `release_navigation_only_allows_bundle_asset_urls` 和 `release_ipc_only_allows_bundle_asset_urls`，覆盖 HTTP(S) shim origin 被拒绝，同时保留大小写不敏感的 `VESTY://ASSETS/` custom protocol。验证通过两个 focused tests、`rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture`、`rtk cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test` 和 `rtk cargo fmt --all --check`。
- `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项包括 protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW/CI/platform/validator/static/signing/notarization evidence 缺失。

本次 2026-06-12 收紧 Rust JSBridge session 权威校验:

- `vesty-ipc` 新增 `MAX_BRIDGE_SESSION_BYTES = 128` 和 `validate_bridge_session()`，与 `@vesty/plugin-ui` / wry bootstrap 的 session guard 对齐: session 必须非空、最长 128 bytes、不能包含控制字符。
- `vesty-bridge::BridgeRuntime::new()/try_new()` 现在会在建立 runtime 前校验初始 session 和 `BridgeReadyPayload.editor_session_id`，并以新的 `BridgeRuntimeCreateError` 区分参数 schema 错误和 session shape 错误。这样 native Rust bridge 入口不再只依赖 JS/wry 侧预校验，底层权威状态机也会拒绝畸形 session。
- 新增 `bridge_runtime_try_new_rejects_invalid_sessions`，覆盖空 session、超长 session、控制字符 session 和畸形 editor session；既有 invalid param schema 测试已迁移到新的 create error。验证通过 `rtk cargo test -p vesty-ipc -- --nocapture`、两条 focused bridge tests、`rtk cargo test -p vesty-bridge -- --nocapture`、`rtk cargo clippy -p vesty-bridge --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`；workspace Rust tests 当前为 595 passed，Clippy 无 warning，JS workspace tests passed。
- strict `release-check --require-release-artifacts` 仍按预期 failed；本地 ok 项仍为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW/CI/platform/validator/static/signing/notarization evidence 缺失。

本次 2026-06-12 收紧 Rust JSBridge request id / replyTo 权威校验:

- `vesty-ipc` 新增 `MAX_BRIDGE_PACKET_ID_BYTES = 128` 和 `validate_bridge_packet_id()`，与 `@vesty/plugin-ui` / wry bootstrap 的 `replyTo` guard 对齐: request id 必须非空、最长 128 bytes、不能包含控制字符。
- `BridgePacket::response_to()` / `error_to()` 现在只会把通过校验的 request id 反射为 `replyTo`；空、超长或包含控制字符的 id 会被清洗为无 `replyTo`，避免错误回包把畸形 correlation id 带回 WebView。
- `BridgeRuntime::handle_packet()` 在 request 分发前统一要求 `id` 存在且通过 `validate_bridge_packet_id()`；缺失、空、超长或包含控制字符的 request id 会返回 non-retryable `validation_error`，不会进入 hello/state/param/subscription handler。可恢复 parse error 路径也复用同一 validator，因此畸形 id 不会收到 parse-error response。
- `BridgeRuntime::handle_packet()` 现在还会在 version/session mismatch 错误处理前校验 inbound session shape；空、超长或包含控制字符的 session 会 fail-closed 丢弃且不回包，避免把畸形 session 反射到 response envelope。合法但 stale 的 session 仍返回 `permission_denied`，保留 editor session 切换后的可诊断行为。
- `BridgeRuntime` 的 JS -> Rust inbound contract 现在只接受 `BridgeKind::Request`；伪造的 response/event/ack/error 包会 fail-closed 丢弃且不回包。request 包也不能携带 `replyTo` 或 `error` 字段，避免 UI 侧把 server-origin reply/error envelope 混入 request path。
- 新增 `validates_bridge_packet_id_and_sanitizes_invalid_reply_id`、`invalid_request_id_returns_validation_error_without_dispatching`、`recoverable_parse_error_rejects_invalid_request_id`、`invalid_inbound_session_is_dropped_without_reflection`、`stale_but_valid_inbound_session_returns_permission_denied`、`inbound_non_request_packets_are_dropped_without_response` 和 `request_reply_to_and_error_fields_are_rejected`。验证通过 focused tests、`rtk cargo test -p vesty-ipc -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture` 和 `rtk cargo clippy -p vesty-bridge --all-targets -- -D warnings`；`vesty-ipc` 为 6 passed，`vesty-bridge` 为 61 passed，Clippy 无 warning。

本次 2026-06-12 收紧 wry native IPC panic fallback envelope 校验:

- `vesty-ui-wry::ipc_handler_panic_response()` 现在复用 `vesty-ipc` 的 session/type/request-id validators，并要求 panic fallback 只对合法 request envelope 生成 retryable `internal_error`。畸形 session、畸形 type、畸形 id、非 request kind、以及混入 `replyTo` / `error` 的 request 都会 fail-closed 不回包，避免 handler panic 恢复路径反射不可信 envelope。
- 新增 `ipc_handler_guard_drops_malformed_panic_response_envelopes`，并保留 `ipc_handler_guard_converts_panic_to_internal_error` 的正常路径覆盖。验证通过 `rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture`、`rtk cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings` 和 `rtk cargo fmt --all --check`；`vesty-ui-wry` wry-backend tests 为 17 passed，Clippy 无 warning。

本次 2026-06-12 收紧 JS SDK / wry bootstrap Rust -> JS inbound envelope 校验:

- `@vesty/plugin-ui` 和 wry bootstrap 的 `validInboundPacket()` 现在拒绝 Rust -> JS packet 携带 `id`，并要求 event/ack 不能携带 `replyTo` 或 `error`。response/error 仍必须携带合法 `replyTo`，error 仍必须携带合法 error payload。
- `deliver()` 现在只把 `event` 派发给 subscription listener；`ack` 作为保留 kind 通过 shape/session 校验后静默忽略，不会被当作普通 event 触发 listener，也不会 settle pending request。
- `packages/plugin-ui/tests/bridge.test.mjs` 增加 malformed inbound coverage，覆盖 event 携带 `id` / `replyTo` / `error`、ack 不派发、response 携带 server `id` 不结算 pending；`vesty-ui-wry` bootstrap script test 增加对应脚本断言。验证通过 `rtk npm --workspace @vesty/plugin-ui test`、`rtk npm test`、`rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture` 和 `rtk cargo fmt --all --check`。

本次 2026-06-12 收紧 JSBridge packet flags 边界:

- `vesty-ipc` 新增 `MAX_BRIDGE_PACKET_FLAGS = 16`、`MAX_BRIDGE_PACKET_FLAG_BYTES = 64` 和 `validate_bridge_packet_flags()`；flags 最多 16 个，每个 flag 必须非空、最长 64 bytes、不能包含控制字符。当前已知使用者是 meter event 的 `latest` flag，未知合法 flag 保留给未来扩展。
- `vesty-bridge::BridgeRuntime` 在 request dispatch 前校验 inbound request flags；畸形 flags 返回 non-retryable `validation_error`，不会进入 hello/state/param/subscription handler。`vesty-ui-wry::ipc_handler_panic_response()` 也复用同一 flags validator，避免 panic fallback 反射畸形 flags。
- `@vesty/plugin-ui` 和 wry bootstrap 的 `validInboundPacket()` 现在校验 Rust -> JS `flags` shape；合法 `["latest"]` event 仍可派发，畸形 flags 会 fail-closed 丢弃。
- 新增 `validates_bridge_packet_flags`、`request_flags_are_validated_before_dispatch` 和 wry panic fallback flags case；JS bridge malformed inbound 测试覆盖 bad flags 和合法 `latest` flag。验证通过 `rtk cargo test -p vesty-ipc -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture`、`rtk cargo clippy -p vesty-bridge --all-targets -- -D warnings` 和 `rtk npm --workspace @vesty/plugin-ui test`；`vesty-ipc` 为 7 passed，`vesty-bridge` 为 62 passed，`vesty-ui-wry` 为 17 passed。

本次 2026-06-12 收紧 JSBridge packet seq 边界:

- `vesty-ipc` 新增 `MAX_BRIDGE_PACKET_SEQ = 9_007_199_254_740_991`、`validate_bridge_packet_seq()` 和 `advance_bridge_packet_seq()`，把 Bridge envelope `seq` 明确限制在 JavaScript safe integer 范围内。这样 TS protocol 仍可用 `number`，但不会在 WebView 侧因为 JSON number 精度丢失而产生不可诊断的排序/关联问题。
- `vesty-bridge::BridgeRuntime` 现在在 request dispatch 前校验 inbound request `seq`，recoverable parse-error fallback 也要求原始 packet `seq` 合法；outbound `next_seq()` 到达 safe-integer 上限后回绕到 `1`。新增 `request_seq_is_validated_before_dispatch`、`recoverable_parse_error_rejects_unsafe_seq` 和 `outbound_seq_wraps_before_js_safe_integer_overflow`。
- `vesty-ui-wry::ipc_handler_panic_response()` 复用同一 seq validator，并用 `advance_bridge_packet_seq()` 生成 panic fallback response seq，避免 `saturating_add(1)` 推出 Web 端不可精确表示的值。wry bootstrap 和 `@vesty/plugin-ui` 都改用 `Number.isSafeInteger(packet.seq)` 入站校验，并在 JS request 发送端到达 `Number.MAX_SAFE_INTEGER` 后回绕到 `1`。
- JS malformed inbound 测试覆盖超过 `Number.MAX_SAFE_INTEGER` 的 event 被丢弃；wry bootstrap script test 固定 `MAX_BRIDGE_PACKET_SEQ`、`Number.isSafeInteger(packet.seq)` 和 `nextSeq()` 语义存在。验证通过 `rtk cargo test -p vesty-ipc -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture`、`rtk npm --workspace packages/plugin-ui test`、`rtk cargo fmt --all --check`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`；workspace Rust tests 当前为 609 passed，clippy 无 warning，JS workspace tests passed。strict `release-check --require-release-artifacts` 仍按预期 failed，失败项仍是真实 DAW/CI/platform/validator/static/signing/notarization evidence 缺失。

本次 2026-06-12 收紧 Rust -> JS error code 边界:

- `@vesty/plugin-ui` 和 wry bootstrap 的 `validBridgeError()` 现在要求 inbound error payload 的 `code` 必须属于 `BridgeErrorCode` 协议枚举: `parse_error`、`unsupported_version`、`unsupported_type`、`validation_error`、`permission_denied`、`timeout`、`backpressure`、`host_rejected`、`plugin_faulted`、`state_conflict` 或 `internal_error`。未知 error code 不再仅因“非空 string”通过入站校验。
- JS malformed inbound 测试新增 unknown error code case，证明畸形 error packet 不会 settle pending request，后续合法 response 仍能 resolve；wry bootstrap script test 固定 `BRIDGE_ERROR_CODES` 和 `BRIDGE_ERROR_CODES.has(value.code)` 存在。
- Focused 验证通过 `rtk npm --workspace packages/plugin-ui test`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture` 和 `rtk cargo test -p vesty-ui-wry --features wry-backend ipc_handler_guard_drops_malformed_panic_response_envelopes -- --nocapture`。

本次 2026-06-12 收紧 Rust -> JS response/error 字段互斥:

- `@vesty/plugin-ui` 和 wry bootstrap 的 `validInboundPacket()` 现在要求 Rust -> JS `response` packet 不能携带 `error` 字段，`error` packet 不能携带 `payload` 字段。正常 response/error 仍按 `replyTo` 结算 pending request；字段污染的 response/error 会 fail-closed 丢弃。
- JS malformed inbound 测试新增 response+error 和 error+payload case，证明畸形 packet 不会 settle pending request，后续合法 response 仍能 resolve；wry bootstrap script test 固定 `packet.kind === "response" && "error" in packet` 和 `packet.kind === "error" && "payload" in packet` guard。
- Focused 验证通过 `rtk npm --workspace packages/plugin-ui test` 和 `rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture`；`vesty-ui-wry` wry-backend tests 当前为 17 passed。

本次 2026-06-12 收紧 Rust -> JS error message 边界:

- `vesty-ipc` 新增 `MAX_BRIDGE_ERROR_MESSAGE_BYTES = 2048` 和 `validate_bridge_error_message()`，规定 Bridge error message 最长 2048 bytes 且不能包含控制字符。空 message 在协议层仍允许，便于极少数只依赖 code/details 的错误；常规 runtime 生成的 message 仍为可读字符串。
- `@vesty/plugin-ui` 和 wry bootstrap 的 `validBridgeError()` 现在镜像同一 message 长度上限，超长 error message 会 fail-closed 丢弃，不会 settle pending request。
- JS malformed inbound 测试新增 2049-byte error message case；wry bootstrap script test 固定 `MAX_BRIDGE_ERROR_MESSAGE_BYTES` 和 `utf8ByteLength(value.message) <= MAX_BRIDGE_ERROR_MESSAGE_BYTES`。Focused 验证通过 `rtk cargo test -p vesty-ipc validates_bridge_error_message -- --nocapture`、`rtk npm --workspace packages/plugin-ui test` 和 `rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture`。

本次 2026-06-12 收紧 JSBridge JSON payload/details 边界:

- `@vesty/plugin-ui` 和 wry bootstrap 新增 `validJsonValue()`、`validPlainDataRecord()`、`hasOwnDataProperty()` 和 `ownDataValue()`。Rust -> JS inbound `payload` 与 `error.details` 现在必须是 JSON-compatible value，并受 max depth 64、array items 65536、object keys 16384、nodes 262144、string bytes 262144 边界限制；`undefined`、function、symbol、`NaN`、`Infinity`、循环引用、非 plain object、symbol key、accessor/getter 属性和非 enumerable 属性都会 fail-closed 丢弃。
- `validInboundPacket()` 和 `validBridgeError()` 现在通过 descriptor-safe 读取字段，不会触发 hostile object getter；`deliver()` / `packetMatchesSession()` 在通过 shape 校验后也复用 descriptor 读取 `kind`、`replyTo`、`error`、`payload`、`type` 和 `session`。畸形 direct `window.__VESTY_INTERNAL__.deliver()` 调用不会抛出、不会触发 listener，也不会错误 settle pending Promise。
- JS -> Rust request payload 现在复用同一 JSON-compatible guard，避免 `JSON.stringify` 静默丢弃 `undefined` / function / symbol 或把 `NaN` / `Infinity` 改写成 `null`。无 payload 仍允许省略 payload 字段；param gesture helpers 现在只在确实提供 `gestureId` 时发送该字段，避免 `{ gestureId: undefined }`。
- JS malformed inbound tests 新增 event payload 与 error.details 的 `undefined`、function、symbol、非有限 number、循环引用和 getter/accessor packet 覆盖；request payload tests 覆盖畸形 payload 被同步 `validation_error` 拒绝、不会发包也不会消耗 `seq`，后续合法 request 仍从 `js-1` 开始。wry bootstrap script test 固定 JSON guard、descriptor-safe helpers 和 outgoing payload guard。
- 验证通过 `rtk cargo fmt --all --check`、`rtk npm --workspace packages/plugin-ui test`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture`、`rtk npm test`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`；workspace Rust tests 当前为 610 passed，clippy 无 warning，JS workspace tests passed。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍包括 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 Rust outbound Bridge error message:

- `BridgeErrorPayload::new()` 现在通过 `sanitize_bridge_error_message()` 保证所有标准 Rust 侧 error envelope message 符合 WebView 入站规则。合法 message 保持不变；控制字符替换为空格；超过 `MAX_BRIDGE_ERROR_MESSAGE_BYTES = 2048` 的 message 会按 UTF-8 char boundary 截断。这样 bridge runtime、VST3 adapter invalid-schema response 和 wry IPC panic fallback 只要使用标准构造器，就不会产出会被 JS 入站 validator 拒绝的 error message。
- `vesty-ipc` 新增 constructor sanitizer 覆盖测试，验证 clean message 保持 exact、控制字符替换、ASCII 超长截断、多字节边界截断，以及 sanitizer 输出均通过 `validate_bridge_error_message()`。`vesty-bridge` 测试 helper `assert_last_error()` 现在也断言实际 runtime 发出的 error message 通过同一 validator，避免 runtime helper 绕过协议不变量。
- 验证通过 `rtk cargo test -p vesty-ipc bridge_error_payload_new_sanitizes_message -- --nocapture`、`rtk cargo test -p vesty-ipc -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture`、`rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`；workspace Rust tests 当前为 611 passed，clippy 无 warning，JS workspace tests passed。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍为 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 Rust outbound Bridge error details:

- `vesty-ipc` 新增 Rust 侧 `MAX_BRIDGE_JSON_DEPTH = 64`、`MAX_BRIDGE_JSON_ARRAY_ITEMS = 65536`、`MAX_BRIDGE_JSON_OBJECT_KEYS = 16384`、`MAX_BRIDGE_JSON_NODES = 262144`、`MAX_BRIDGE_JSON_STRING_BYTES = 262144`，与 JS SDK / wry bootstrap 的 inbound JSON-compatible 边界对齐。`validate_bridge_json_value()` 覆盖 `serde_json::Value` 的深度、节点数、数组长度、object key 数和 string/key byte 长度。
- `BridgeErrorPayload::set_details()` / `with_details()` 现在会验证并净化 details；`BridgePacket::error_to()` 会在最终出站前再次调用 `sanitize_details()`。非法 details 会降级为小 JSON object: `{ "dropped": true, "reason": "..." }`，避免整个 error packet 被 WebView 入站 validator 丢弃。合法 state conflict snapshot details 保持原样。
- `vesty-bridge` 的 `state.setConfig` / `state.setUiState` conflict 路径改用 `set_details()`，并在 conflict tests 中断言实际发送的 details 通过 `validate_bridge_json_value()`。`assert_last_error()` 也会校验 error details，覆盖 runtime helper 的常规错误出口。
- 验证通过 `rtk cargo test -p vesty-ipc validates_and_sanitizes_bridge_json_values -- --nocapture`、`rtk cargo test -p vesty-ipc bridge_packet_error_to_sanitizes_error_details -- --nocapture`、`rtk cargo test -p vesty-ipc -- --nocapture`、`rtk cargo test -p vesty-bridge stale_state_set_config_returns_conflict -- --nocapture`、`rtk cargo test -p vesty-bridge stale_state_set_ui_state_returns_conflict -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture`、`rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`；workspace Rust tests 当前为 613 passed，clippy 无 warning，JS workspace tests passed。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 Rust outbound Bridge response/event payload:

- `BridgePacket::response_to()` 现在会对 response payload 调用 `sanitize_bridge_json_value()`，与 WebView 入站 `payload` JSON-compatible 边界对齐。非法 response payload 会降级为 `{ "dropped": true, "reason": "..." }`，避免 pending request 因 Rust 生成过大 payload 而在 JS 侧被 fail-closed 丢弃。
- `vesty-bridge::emit_event()`、`queue_latest_meter()`、`flush_latest_meters()` 和 `emit_rt_log_event()` 现在会在可靠事件、latest-wins meter 和 RT log 出站前净化 payload。合法 payload 保持原样；非法 payload 会降级为小 JSON object，保证 listener 仍能收到诊断事件。
- 新增 `bridge_packet_response_to_sanitizes_payload`、`outbound_event_payload_is_sanitized_before_send` 和 `latest_meter_payload_is_sanitized_before_flush`；既有 reliable event / meter frame tests 继续证明合法 payload 不受影响。
- 验证通过 `rtk cargo test -p vesty-ipc bridge_packet_response_to_sanitizes_payload -- --nocapture`、`rtk cargo test -p vesty-bridge outbound_event_payload_is_sanitized_before_send -- --nocapture`、`rtk cargo test -p vesty-bridge latest_meter_payload_is_sanitized_before_flush -- --nocapture`、`rtk cargo test -p vesty-ipc -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture`、`rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`；workspace Rust tests 当前为 616 passed，clippy 无 warning，JS workspace tests passed。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 Rust inbound Bridge request payload:

- `vesty-bridge::validate_inbound_request_shape()` 现在会在 request dispatch 前对存在的 `packet.payload` 调用 `validate_bridge_json_value()`。这把 Rust native IPC 入口和 JS SDK / wry bootstrap 的 request payload guard 对齐，避免页面绕过 SDK 直接调用 `window.ipc.postMessage(...)` 时，把超深/超大 JSON payload 送进 hello/state/param/subscription handler。
- 新增 `request_payload_json_bounds_are_validated_before_dispatch`，使用小体积但超过 depth 64 的 JSON payload 验证 runtime 返回 non-retryable `validation_error`、保留合法 `replyTo`、不执行 `bridge.hello` dispatch、也不采纳 ready ack。既有 `request_flags_are_validated_before_dispatch` 继续覆盖同一 dispatch-before-handler 阶段的 flags gate。
- 验证通过 `rtk cargo test -p vesty-bridge request_payload_json_bounds_are_validated_before_dispatch -- --nocapture`、`rtk cargo test -p vesty-bridge request_flags_are_validated_before_dispatch -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture`、`rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`；workspace Rust tests 当前为 617 passed，clippy 无 warning，JS workspace tests passed。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 Rust outbound Bridge event/meter/log topic:

- `vesty-bridge::emit_event()`、`queue_latest_meter()` 和 `emit_rt_log_event()` 现在会在发送或排队前复用 `vesty-ipc::validate_packet_type()` 校验 topic，因为这些 topic 会成为 Rust -> JS packet 的 `type` 字段。无效 topic fail-closed，不发送、不排队、不返回错误给无 request 关联的调用方。
- `flush_latest_meters()` 在 drain `latest_meters` 后再次校验 topic，防止未来内部路径绕过 `queue_latest_meter()` 直接插入无效 topic 时产出会被 JS inbound validator 丢弃的 meter packet。
- 新增 `outbound_event_rejects_invalid_packet_type_topic_before_send`、`rt_log_events_reject_invalid_packet_type_topic_before_send`、`latest_meter_rejects_invalid_packet_type_topic_before_queueing` 和 `latest_meter_flush_skips_invalid_packet_type_topics`。验证已通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-bridge invalid_packet_type -- --nocapture`、`rtk cargo test -p vesty-bridge -- --nocapture` 和 `rtk cargo test --workspace -j1`；workspace Rust tests 当前为 621 passed。
- 完整质量门继续通过: `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning，`rtk npm test` 通过四个 JS workspace，`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 显示 protocol export matches，`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"` 通过。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 DAW smoke evidence parser:

- `vesty daw-matrix` / `release-check` 现在读取手工 DAW evidence marker 文件时，会先拒绝同一文件中的矛盾证据: `pending` / `false` assignment、`failed` / `error` / `timeout` / `crashed` 等负向文本会覆盖 `scan=true`、`meter_flush sent=3`、`offline_render=true` 等正向 marker。此前 `--write-report` 已拒绝这类输入，本次把同样语义扩展到直接手工编辑 evidence 目录的读取路径。
- REAPER scan 判断现在显式 evidence marker 优先；如果 `target/reaper-smoke/scan-smoke.log` 或 `scan.log` 存在，则不再被本机 REAPER scan cache 额外“加分”。只有缺少显式 scan marker 时才 fallback 到本机 scan cache，避免安装/扫描缓存掩盖采证目录中的失败 marker。
- 新增 `generic_daw_evidence_rejects_contradictory_positive_markers` 和 `reaper_evidence_rejects_contradictory_marker_files`，覆盖 generic DAW 和 REAPER 专用收集路径。验证通过 `rtk cargo test -p vesty-cli contradictory_marker -- --nocapture`、`rtk cargo test -p vesty-cli generic_daw_evidence_accepts_explicit_smoke_markers -- --nocapture`、`rtk cargo test -p vesty-cli daw_matrix_write_report_accepts_reaper_generic_markers -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 294 passed。
- 完整质量门继续通过: `rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1` 当前 623 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 platform-smoke evidence parser:

- `vesty platform-smoke` / `release-check` 现在读取平台 smoke JSON 时，会拒绝同一 check value 中的矛盾证据: `pending` / `false` assignment、`failed` / `error` / `timeout` / `crashed` 等负向文本会覆盖 `webview_attach=true`、`meter_flush sent=1`、`jsbridge_roundtrip=true` 等正向 marker。assignment 检测支持分号分隔的同一行 marker，例如 `jsbridge_roundtrip=true; roundtrip=false` 不再能通过。
- `vst3_validator` check 不使用通用 negative-word 过滤，因为合法 Steinberg validator 摘要会包含 `0 failed`；它仍完全由 validator-specific parser 约束，需要 Steinberg/VST3 validator 身份和非零 passed、零 failed 摘要。
- 新增 `platform_smoke_rejects_contradictory_positive_values`，覆盖 WebView attach、meter stream 和 JSBridge roundtrip 三类矛盾值。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli platform_smoke_rejects_contradictory_positive_values -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke -- --nocapture`、`rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 295 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 624 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实平台 smoke evidence。

本次 2026-06-12 收紧 signing/notarization marker parser:

- 通用 explicit marker parser 现在支持分号分隔的同一行 marker fragment。`codesign=pass; codesign=false`、`signtool=pass; signtool=failed`、`status: Accepted; status: Rejected`、`stapled=true; stapled=false` 等 inline 矛盾证据会被 negative evidence 检测捕获，不再只按整行第一个 assignment 判断。
- 这个变更复用同一套 `explicit_truthy_marker_line()` / `explicit_falsy_marker_line()` / `explicit_marker_line_matches()` 解析路径，因此签名、公证、DAW 和 platform smoke 的手工 marker 语义更一致。
- 新增/扩展 `signing_evidence_rejects_contradictory_success_markers` 和 `notarization_evidence_rejects_contradictory_success_markers`，覆盖 inline codesign/signtool/notary/stapler 矛盾 marker。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli signing_evidence_rejects_contradictory_success_markers -- --nocapture`、`rtk cargo test -p vesty-cli notarization_evidence_rejects_contradictory_success_markers -- --nocapture`、`rtk cargo test -p vesty-cli explicit_truthy_marker_rejects_substring_keys_and_false_values -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke -- --nocapture`、`rtk cargo test -p vesty-cli contradictory_marker -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 295 passed。

本次 2026-06-12 收紧 DAW bridge-trace 辅助证据:

- `collect_reaper_evidence()` 和 `collect_generic_daw_evidence()` 读取 `bridge-trace.log` 作为 UI->Host 或 meter stream 辅助证据时，现在也必须先通过 `daw_marker_positive()`。带 `failed` / `error` / `timeout` / `false` / `pending` 的 trace 不再能因为同时包含 `param.begin/perform/end` 或 `meter.main` packet 而把对应 DAW matrix 项置为 pass。
- 新增 `daw_evidence_rejects_contradictory_bridge_trace_markers`，同时覆盖 generic DAW 和 REAPER 专用收集路径。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli daw_evidence_rejects_contradictory_bridge_trace_markers -- --nocapture`、`rtk cargo test -p vesty-cli contradictory_marker -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 296 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 625 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实 DAW/平台/signing evidence。

本次 2026-06-12 收紧 smoke-host 本地诊断证据:

- `vesty smoke-host --bridge-trace` 和 `--meter-log` 读取本地 trace/log 时，现在同样复用 `daw_marker_positive()` / `daw_marker_matches()`。包含 `readyAck/reply`、`param.begin/perform/end` 或 `meter_flush sent=1` 的本地诊断文件，如果同时出现 `failed` / `error` / `timeout` / `false` / `pending` 等负向证据，会把对应 smoke-host check 标记为 failed，而不是误报 ok。
- 这只影响 headless local diagnostic 的准确性；`vesty smoke-host` 仍不替代真实 DAW、platform WebView、Steinberg validator、signing 或 notarization evidence。
- 新增 `smoke_host_rejects_contradictory_bridge_and_meter_evidence`，覆盖 JSBridge trace 和 meter log 两条本地诊断输入。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli smoke_host_rejects_contradictory_bridge_and_meter_evidence -- --nocapture`、`rtk cargo test -p vesty-cli smoke_host -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 297 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 626 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 validator-passed report 日志自洽性:

- `validate_release_validate_report()` 现在会在 `validator.status = "passed"` 时检查 `validator.stdout` / `validator.stderr` 中可解析的 Steinberg summary。如果日志摘要中的 passed/failed 计数与 JSON 字段 `tests_passed` / `tests_failed` 不一致，report 会被拒绝，避免手工修改 JSON 把真实失败的 validator 输出伪装成 pass。
- 这条规则只在 stdout/stderr 存在且能解析出 summary 时触发；没有日志摘要的历史/最小 report 仍由既有 `tests_passed > 0`、`tests_failed = 0`、`exit_code = 0` 和无 error/reason 规则约束。
- 新增 `validate_report_rejects_validator_passed_log_summary_mismatch`，覆盖 stdout `47 passed / 1 failed` 和 stderr `46 passed / 0 failed` 两类与 JSON 计数冲突的 validator-passed report。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli validate_report_rejects_validator_passed_log_summary_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli validate_report_rejects_inconsistent_validator_passed_fields -- --nocapture`、`rtk cargo test -p vesty-cli validator_summary_extracts_passed_and_failed_counts -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 298 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 627 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 platform-smoke VST3 validator 运行失败证据:

- `platform_vst3_validator_evidence_ok()` 继续不使用通用 `failed` 负词过滤，以免误拒合法 `0 failed` 摘要；但现在会专门拒绝 `not found`、`missing`、`unavailable`、`not installed`、`failed to run`、`validator error`、`validator timeout/timed out`、`validator crashed` 等运行失败证据。
- 这样 `Steinberg validator passed 47 tests, 0 failed` 仍可通过；`Steinberg validator passed 47 tests, 0 failed; validator timeout`、`VST3 validator: passed=47 failed=0; validator crashed` 和 `... validator error ...` 会失败。
- 扩展 `platform_smoke_requires_validator_identity_and_zero_fail_summary` 覆盖 validator timeout/crash/error 与合法 `0 failed` 的区分。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli platform_smoke_requires_validator_identity_and_zero_fail_summary -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke_accepts_alternate_system_webview_and_validator_markers -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 298 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 627 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、validator/static coverage、签名和 notarization evidence 缺失。

本次 2026-06-12 收紧 static validate static-only evidence:

- `validate_static_validate_report()` 现在要求 static validate 报告保持真正的 static-only 语义: `validator.status` 只能是 `skipped`、`not_run` 或 `not_found`。带 `passed` 或 `failed` validator run 结果的 report 不能再被 `vst3 static validate reports` 或 `ci example static validate coverage` 当作 CI 静态包检查证据。
- 新增 `static_validate_reports_reject_validator_run_reports`，覆盖 validator-passed 与 validator-failed report 都会被 static-only gate 拒绝；同时修正 example coverage 回归测试，让 validator coverage 继续使用 validator-passed fixture，而 static coverage 使用独立的 `validator.status = "skipped"` fixture，从而仍能验证缺 UI asset manifest 和缺 parameter manifest 的领域错误。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli example_coverage_rejects_web_ui_report_without_asset_evidence -- --nocapture`、`rtk cargo test -p vesty-cli example_coverage_rejects_example_report_without_parameter_manifest_evidence -- --nocapture`、`rtk cargo test -p vesty-cli static_validate_reports_reject_validator_run_reports -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 299 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 628 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 validator-passed report 运行失败日志:

- `validate_release_validate_report()` 现在在接受 `validator.status = "passed"` 前，会检查 `validator.stdout` / `validator.stderr` 是否包含 validator 运行失败证据，例如 `not found`、`missing`、`unavailable`、`not installed`、`failed to run`、`validator error`、`validator timeout/timed out` 或 `validator crashed`。这些负向运行状态会覆盖 JSON 中的 `tests_passed > 0` / `tests_failed = 0` 字段，避免手工编辑 report 把真实 timeout/crash/error 伪装成 pass。
- 该逻辑与 platform-smoke 的 VST3 validator parser 共用 `vst3_validator_has_runtime_failure()`，仍不会用通用 `failed` 负词过滤，因此合法的 `0 failed` 摘要不会被误拒。
- 新增 `validate_report_rejects_validator_passed_runtime_failure_logs`，覆盖 stdout 中的 `validator timeout` 和 stderr 中的 `validator crashed`；原有 `platform_smoke_requires_validator_identity_and_zero_fail_summary` 继续覆盖 platform smoke 路径。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli validate_report_rejects_validator_passed_runtime_failure_logs -- --nocapture`、`rtk cargo test -p vesty-cli validate_report_rejects_validator_passed_log_summary_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke_requires_validator_identity_and_zero_fail_summary -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 300 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 629 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 validator-failed report 自洽性:

- `validate_validator_check_self_consistent()` 现在会拒绝形态上互相矛盾的 failed validator report: 只提供一半 test count 的 report 会失败；`status = "failed"` 但 `exit_code = 0`、`tests_failed = 0` 且没有 timeout/crash/error/not-found 等运行失败证据的 report 也会失败；`tests_failed = 0` 且没有 nonzero exit 或运行失败证据的 report 同样不能作为自洽失败报告。
- 真实运行失败仍被允许表达为 failed report，例如 `error = "validator timeout"` 或 stdout/stderr 中包含同类 runtime-failure marker。这样 import/release-check 可以保留真实失败诊断，同时避免 stale/manual error 字段把“全绿”的 validator 结果伪装成失败或污染分类。
- 新增 `validate_report_rejects_contradictory_validator_failed_fields` 覆盖 exit 0 + zero failed + stale failure marker、partial test counts，以及 timeout 运行失败仍可作为 failed report。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli validate_report_rejects_contradictory_validator_failed_fields -- --nocapture`、`rtk cargo test -p vesty-cli static_validate_reports_reject_validator_run_reports -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 301 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 630 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 import-ci signing/notarization 失败日志分类:

- `release-evidence import-ci` 现在读取文本 artifact 时，如果内容或文件名明确属于 signing/notarization 证据但解析为负向/不完整证据，会在 `import-ci-report.json` 中记录对应 failed item，而不是把它降级成普通 `text artifact skipped`。例如 `codesign=pass` 同时包含 `invalid signature`、`signtool` 日志包含 `Number of errors: 1`、notarytool `Rejected` 或 stapler failure 都会被显式记录为 failed。
- 普通无关文本仍保持 `text artifact skipped`，避免 README/notes 等非证据文件制造噪音。成功 signing/notarization 证据的复制路径不变，未完整的 notarization 仍不会写入 `notary.log`。
- 新增 `import_ci_reports_failed_signing_and_notarization_logs`，覆盖 codesign、signtool、notarytool 和 stapler 失败日志的 import 分类，并确认普通 notes 仍 skipped。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_reports_failed_signing_and_notarization_logs -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 302 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 631 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 import-ci signed `.vst3` bundle 目录分类:

- `release-evidence import-ci` 现在发现 `.vst3` bundle 目录后，如果签名证据校验失败，会在 `import-ci-report.json` 中记录 `signed bundle evidence` 的 failed item，而不是静默忽略。缺失 `Contents/_CodeSignature/CodeResources`、`CodeResources` 不是可解析 plist、或 plist 缺少 `files` / `files2` 字典等情况都会留下可审计失败原因。
- 有效 macOS signed `.vst3` bundle 的复制路径保持不变，仍导入到 `signed-bundles/<bundle>.vst3`。无效 bundle 不会被复制到 release evidence 目录，因此不会被后续 release-check 当作签名通过证据。
- 新增 `import_ci_reports_failed_signed_bundle_directories`，覆盖一个有效 signed bundle、一个 placeholder `CodeResources` bundle、一个缺失 `CodeResources` bundle 的混合集合。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_reports_failed_signed_bundle_directories -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_reports_failed_signing_and_notarization_logs -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 303 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 632 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 import-ci 命名 JSON artifact 分类:

- `release-evidence import-ci` 现在遇到路径/文件名明显属于 release evidence 的 JSON 但无法解析为对应报告时，会在 `import-ci-report.json` 中记录 failed item，而不是降级成普通 `json artifact skipped`。覆盖 `doctor-*`、`release-check-*`、`release-action-plan-*`、`platform-smoke`、`validate` / `static-validate`、`publish-plan`、`crate-package`、`npm-pack`、`dependency-baseline-latest` 和 `vst3-sdk` 命名/目录指纹。
- 普通无关 JSON 仍保持 `json artifact skipped`，避免 CI 下载目录里的 notes/config JSON 造成假失败。命名 JSON 失败只提供诊断，不会复制到 release evidence 目录，也不会满足任何 release gate。
- 新增 `recognized_json_artifact_name_from_path()` 和 `import_ci_reports_malformed_named_json_artifacts`，覆盖坏 JSON 语法、schema 不匹配的 doctor/release-check/validator/static/platform-smoke artifact，以及普通 `notes.json` 仍 skipped。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_reports_failed_signed_bundle_directories -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 304 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 633 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 import-ci 自动发现 CI run URL provenance:

- `release-evidence import-ci` 自动扫描下载目录中的文本 artifact 时，命名为 `ci-run-url.txt` / `ci_run_url.txt` / `.log` 的 provenance 文件如果包含非 GitHub Actions run URL，现在会在 `import-ci-report.json` 记录 `ci run url` failed item，而不是被 `find_map(... ok())` 吞掉后只表现成“没有找到 URL”。
- pending 模板和普通无关文本仍不作为失败；普通 `notes.txt` 中出现畸形 `ci_run_url=...` 不会制造噪音。显式 `--ci-run-url-file` 的行为保持严格报错，自动发现路径则把问题留在 import report 中方便 CI artifact triage。
- 新增 `auto_discover_ci_run_url_evidence()`、`ci_run_url_evidence_path()` 和 `import_ci_reports_invalid_auto_discovered_ci_run_url_file`，覆盖命名 provenance 文件畸形 URL 被报告 failed、普通 notes 文本 skipped。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_reports_invalid_auto_discovered_ci_run_url_file -- --nocapture`、`rtk cargo test -p vesty-cli ci_run_url -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 305 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 634 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 release-evidence 标准报告槽位诊断:

- `release-check --release-evidence-dir` 现在遇到标准槽位中的 `publish-plan/publish-plan.json`、`crate-package/crate-package.json`、`npm-pack/npm-pack.json` 和 `dependency-baseline/dependency-baseline-latest.json` 时，只要文件真实存在且不是 symlink，就会把路径传给对应 release-check gate。无效文件不再在 discovery 阶段被忽略成“required evidence missing”，而是由各自 validator 报出具体 JSON/schema/coverage 错误。
- 有效标准路径行为不变；无效标准文件仍不会通过 gate，只是失败原因更精确。显式 CLI 路径本来已经走 validator，本次让 `--release-evidence-dir` 自动发现路径与显式路径的诊断语义对齐。
- 新增 `release_evidence_dir_keeps_invalid_standard_report_paths_for_diagnostics`，覆盖 publish plan、crate package、npm pack 和 dependency latest baseline 的无效标准文件都会保留路径并在对应 release-check item 中 failed，而不是 missing。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_report_paths_for_diagnostics -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 306 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 635 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 release-evidence 标准 validate 槽位诊断:

- `release-check --release-evidence-dir` 对根目录 `validate-report.json` 和 `static-validate-report.json` 现在区分 pending 模板与非模板坏证据。由 `write_release_evidence_templates()` 生成的 pending validate/static 模板仍不会计入 release evidence，避免刚初始化模板目录就产生噪声失败。
- 如果标准 validate/static 槽位被替换成 malformed JSON 或非 pending 的无效 report，则路径会被保留到 `validate_reports` / `static_validate_reports`，由 `vst3 validate reports` 或 `vst3 static validate reports` gate 报出具体失败；不再被 discovery 阶段静默忽略成 missing。
- 新增 `validate_report_is_pending_template()`、`push_standard_release_validate_report()`、`push_standard_static_validate_report()` 和 `release_evidence_dir_keeps_invalid_standard_validate_reports_for_diagnostics`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_validate_reports_for_diagnostics -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 307 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 636 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 release-evidence 标准 signing/notarization 槽位诊断:

- `release-check --release-evidence-dir` 对根目录标准槽位 `signing-macos.log`、`signing-windows.log` 和 `notary.log` 现在区分 pending 模板与非模板坏证据。由 `write_release_evidence_templates()` 生成的 `signed=pending` / `codesign verify=pending` / `signtool verify=pending` / `notarization=pending` / `stapled=pending` 模板仍会被忽略，避免刚初始化 evidence 目录就制造噪声失败。
- 如果标准 signing/notary 槽位被替换成真实但无效的 codesign、signtool 或 notary/stapler 日志，路径会被保留到 `signed_bundle_evidence` / `notarization_log`，再由 `signed bundle evidence` 或 `notarization log` gate 报出具体失败，例如 `invalid signature`、`number of errors: 1` 或 `status: rejected`；不再在 discovery 阶段静默忽略成 generic `required evidence missing`。
- 递归自动发现路径仍只接收内容校验通过的 signing/notarization 证据，避免普通 README/notes 文本变成失败项；只有约定标准槽位承担“坏证据也保留用于诊断”的严格语义。
- 新增 `signing_evidence_is_pending_template()`、`notarization_evidence_is_pending_template()`、`push_standard_signing_evidence()` 和 `release_evidence_dir_keeps_invalid_standard_signing_and_notary_logs_for_diagnostics`。同时调整 `release_evidence_templates_do_not_count_as_pass_or_overwrite_logs`，把“不会覆盖用户手写文件”和“纯 pending 模板不计入 release evidence”两个语义分开验证。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_signing_and_notary_logs_for_diagnostics -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture` 和 `rtk cargo test -p vesty-cli -- --nocapture`；`vesty-cli` 当前 308 passed。
- 完整质量门继续通过: `rtk cargo test --workspace -j1` 当前 637 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 release-evidence 标准 VST3 SDK surface 槽位诊断:

- `release-check --release-evidence-dir` 现在遇到标准槽位 `vst3-sdk/generated-bindings-surface.json` 或根目录 `generated-bindings-surface.json` 时，只要文件真实存在且不是 symlink，就会把路径传给 `vst3 SDK generated bindings surface` gate。无效 surface 文件不再在 discovery 阶段被忽略成 optional `not requested`，而是由 surface validator 报出具体 JSON/schema/coverage/baseline/`bindingsGenerated` 错误。
- 这让 VST3 SDK generated-headers audit artifact 的标准槽位语义和 manifest、binding-plan、scaffold、ABI seed、ABI layout、interface skeleton 保持一致: 缺失时仍然 optional/skipped；一旦存在就必须严格有效。该检查仍只是 generated-headers readiness/surface drift audit，不表示完整 SDK 3.8 bindings 已生成。
- 新增 `release_evidence_dir_keeps_invalid_standard_vst3_sdk_surface_for_diagnostics`，覆盖 malformed/invalid standard surface 文件会保留到 `vst3_sdk_binding_surface` 并由 release-check item failed，而不是 skipped/not requested。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_vst3_sdk_surface_for_diagnostics -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture` 和 `rtk cargo test -p vesty-cli vst3_sdk_binding_surface_release_check -- --nocapture`。
- 完整质量门继续通过: `rtk cargo test -p vesty-cli -- --nocapture` 当前 309 passed、`rtk cargo test --workspace -j1` 当前 638 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，VST3 SDK audit artifacts 因缺失保持 optional skipped，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 release-evidence 递归 validate/static report 诊断:

- `release-check --release-evidence-dir` 递归扫描 evidence 目录内其它 JSON 时，`validator/`、`*.validate.json`、`package/`、`*.static-validate.json` 等命名/目录明确属于 VST3 validate/static evidence 的无效 report 现在会被保留到 `validate_reports` / `static_validate_reports`，再由 `vst3 validate reports` 或 `vst3 static validate reports` gate 输出具体 parse/schema/status 错误；不再因为 `read_validate_report()` 失败或 schema 不匹配而静默跳过成 generic missing evidence。
- 为避免误伤，递归兜底只在文件无法匹配其它 release evidence schema 时启用。类似 `static-validate-release-check.json` 这种 release-check-shaped sidecar 即使文件名含 `static-validate`，也不会被错误纳入 static validate evidence；普通无关 JSON 仍保持 skipped。
- 新增 `validate_report_path_prefers_static()`、`validate_report_path_prefers_release()`、`json_file_matches_non_validate_release_schema()` 和 `release_evidence_dir_keeps_invalid_recursive_validate_reports_for_diagnostics`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_recursive_validate_reports_for_diagnostics -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture` 和 `rtk cargo test -p vesty-cli release_check -- --nocapture`。

本次 2026-06-12 收紧 import-ci 标准 static validate 目录诊断:

- `release-evidence import-ci` 现在把下载 artifact 中位于 `package/` 标准目录的 JSON 也视为 VST3 static validate evidence 候选，即使文件名不含 `static-validate`。如果内容 malformed 或 schema 不匹配，会在 `import-ci-report.json` 中记录 `vst3 static validate report` failed item，而不是降级成普通 `json artifact skipped`。
- 这让 CI artifact 导入侧与 `release-check --release-evidence-dir` 的标准目录语义对齐: `target/release-evidence/package/<bundle>.<platform>.static-validate.json` 以及同目录下误命名的 JSON 都不会静默遮蔽坏证据；无关目录里的普通 JSON 仍保持 skipped。
- 扩展 `import_ci_reports_malformed_named_json_artifacts`，覆盖 `package/report.json` 这种标准目录内但文件名不含 `static-validate` 的坏 JSON。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1` 当前 639 passed，以及 `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 import-ci VST3 SDK JSON artifact 分类:

- `release-evidence import-ci` 现在对 malformed 或 schema-mismatched 的 VST3 SDK JSON artifact 做具体分类: `vst3-sdk-headers.json` / `*vst3-sdk-headers*` 归为 `vst3 SDK header manifest`，`generated-bindings-plan.json` / `*generated-bindings-plan*` 归为 `vst3 SDK generated bindings plan`，`generated-bindings-surface.json` / `*generated-bindings-surface*` 归为 `vst3 SDK generated bindings surface`。其它 `vst3-sdk/` 目录中的未知 JSON 才保留泛化 `vst3 SDK artifact`。
- 这不改变有效 SDK manifest/plan/surface 的导入目标，也不会让无效 artifact 通过；它让 CI 导入报告的失败项与 release-check 的独立 SDK gates 对齐，避免所有坏 SDK JSON 都挤在一个泛化名称下。
- 扩展 `import_ci_reports_malformed_named_json_artifacts`，覆盖 malformed `vst3-sdk/vst3-sdk-headers.json`、malformed root `generated-bindings-plan.json` 和 schema-mismatched `vesty-vst3-sdk/generated-bindings-surface.json`。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1` 当前 639 passed，以及 `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，VST3 SDK audit artifacts 因缺失保持 optional skipped，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 CI release-check artifact 诊断保留:

- `ci_release_check_allows_failed_check()` 现在把 `crate package readiness` 加入允许失败的 release-prep evidence gate 列表。这样 per-OS CI `release-check-<OS>.json` 如果真实记录 crate package readiness 失败，会被 `ci release-check artifacts` gate 接受为可审计的 CI snapshot，而不是被误判为 unexpected local invariant failure 丢掉整份 artifact。
- 本地 invariant 仍然严格: `host profiles`、`protocol snapshot` 和 `vst3 binding baseline` 失败/伪造仍会让 CI release-check artifact 被拒绝。`crate package readiness` 自身仍需通过独立 `crate package readiness` gate 和 `crate-package/crate-package.json` release evidence 才能满足最终发布。
- 新增 `ci_release_check_artifacts_preserve_crate_package_readiness_failures`，覆盖 macOS release-check artifact 中 `crate package readiness = failed` 仍可作为 per-OS CI release-check snapshot 保留。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check_artifacts_preserve_crate_package_readiness_failures -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1` 当前 640 passed，以及 `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 CI release-check artifact platform-smoke 诊断保留:

- `ci_release_check_allows_failed_check()` 现在也把 `platform smoke artifacts` 加入允许失败的外部 release evidence gate 列表。这样 per-OS CI `release-check-<OS>.json` 如果真实记录 platform smoke evidence 缺失，会被 `ci release-check artifacts` gate 接受为可审计 CI snapshot，而不是误判为 unexpected local invariant failure。
- 这只影响 CI snapshot 的保留语义，不会让 `platform smoke artifacts` 通过最终发布门；最终 release 仍必须提供 `platform-smoke/{macos,windows-x64,linux-x11}.json` 真实报告并通过独立 gate。
- 新增 `ci_release_check_artifacts_preserve_platform_smoke_failures`，覆盖 macOS release-check artifact 中 `platform smoke artifacts = failed` 仍可作为 per-OS CI release-check snapshot 保留。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check_artifacts_preserve_platform_smoke_failures -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1` 当前 641 passed，以及 `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 CI release-check artifact VST3 SDK audit 诊断保留:

- `ci_release_check_allows_failed_check()` 现在把 7 个 optional VST3 SDK audit gates 加入允许失败列表: `vst3 SDK header manifest`、`vst3 SDK generated bindings plan`、`vst3 SDK generated bindings surface`、`vst3 SDK generated bindings scaffold`、`vst3 SDK generated bindings ABI seed`、`vst3 SDK generated bindings ABI layout` 和 `vst3 SDK generated bindings interface skeleton`。
- 这些 gate 在 final release-check 中仍是“缺失则 optional skipped、存在则严格验证”；本次只保证如果 CI 已启用 SDK audit 且其中一个 artifact invalid，per-OS `release-check-<OS>.json` 仍可作为诊断 snapshot 导入，而不是因 optional audit failure 被误判为 unexpected local invariant failure。
- 新增 `ci_release_check_artifacts_preserve_vst3_sdk_audit_failures`，覆盖同一 macOS release-check artifact 内 7 个 SDK audit failed checks 仍可作为 per-OS CI release-check snapshot 保留。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check_artifacts_preserve_vst3_sdk_audit_failures -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1` 当前 642 passed，以及 `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，当前 evidence dir 未提供 VST3 SDK audit artifacts 所以保持 skipped，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 CI release-check artifact 文件名发现:

- `collect_ci_release_check_reports()` 的递归目录模式现在用大小写不敏感的 `is_ci_release_check_report_path()` 识别 `release-check*.json`。例如 `Release-Check-Linux.JSON`、`RELEASE-CHECK-macOS.Json` 和 `release-check-WINDOWS.json` 都会被作为 per-OS CI release-check artifact 扫描。
- OS 推断本来已经基于 lowercase 文件名；本次补齐的是文件名过滤层，避免真实下载/手工整理的 CI artifact 因扩展名或前缀大小写不同被漏掉后误报 `no JSON release-check artifacts found` 或 OS coverage missing。
- 新增 `ci_release_check_artifacts_accept_case_insensitive_report_filenames`，覆盖三平台混合大小写文件名仍能完成 Linux/macOS/Windows coverage。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check_artifacts_accept_case_insensitive_report_filenames -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1` 当前 643 passed，以及 `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。

本次 2026-06-12 收紧 CI release-check artifact 路径 OS 推断:

- `collect_ci_release_check_reports()` 现在用完整 artifact path 推断 OS，而不是只依赖文件名。目录分组 CI 下载结构如 `Linux/release-check.json`、`macOS/release-check.json`、`Windows/release-check.json` 现在可覆盖 Linux/macOS/Windows coverage。
- 路径推断改为 token/别名匹配，支持 `linux`、`macos`、`darwin`、`osx`、`windows`、`win64` 等明确 token，但不会把 `swing-state/release-check.json` 这类包含 `win` 子串的无关路径误判为 Windows。
- 新增 `ci_release_check_artifacts_infer_os_from_parent_dirs` 和 `ci_release_check_artifacts_infer_os_from_path_tokens_not_substrings`，覆盖目录分组 artifact 和 substring false positive。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check_artifacts_infer_os_from_parent_dirs -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check_artifacts_infer_os_from_path_tokens_not_substrings -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`、`rtk cargo test --workspace -j1` 当前 650 passed，以及 `rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；这只是 evidence discovery/diagnostics hardening，不改变 final release gate。仍缺真实 DAW matrix、CI run URL/下载 artifacts、platform smoke、三示例三平台 validator/static coverage、签名和 notarization evidence。

本次 2026-06-12 同步收紧 CI doctor artifact 路径 OS 推断:

- `collect_doctor_reports()` 也改为使用完整 artifact path 推断 OS，并继续用 doctor report 内的 `os` 字段做一致性校验。目录分组 CI 下载结构如 `Linux/doctor.json`、`macOS/doctor.json`、`Windows/doctor.json` 现在可覆盖 CI doctor Linux/macOS/Windows coverage。
- 新增 `ci_doctor_artifacts_infer_os_from_parent_dirs` 和 `ci_doctor_artifacts_infer_os_from_path_tokens_not_substrings`，覆盖目录分组 doctor artifact 和 substring false positive。验证通过 `rtk cargo test -p vesty-cli ci_doctor_artifacts_infer_os_from_parent_dirs -- --nocapture`、`rtk cargo test -p vesty-cli ci_doctor_artifacts_infer_os_from_path_tokens_not_substrings -- --nocapture`、`rtk cargo test -p vesty-cli doctor_artifact -- --nocapture` 和完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 321 passed。
- 该改动仍只改善 CI doctor evidence discovery/diagnostics；最终 release gate 仍需要真实 GitHub Actions run URL 和下载 artifacts。

本次 2026-06-12 收紧 platform smoke artifact 路径平台一致性:

- `platform_smoke_release_check()` 现在会从 artifact path 中读取明确平台 token，并要求路径平台与 report `platform` 字段一致。例如 `Windows/platform-smoke.json` 不能携带 `platform = "macos"` 的 report。
- Linux 路径只有同时出现 `linux` 和 `x11` token 才会被当作 Linux X11 平台线索，避免 `linux-wayland/platform-smoke.json` 或泛 Linux 目录误满足 final Linux X11 gate。
- 新增 `platform_smoke_release_check_accepts_platform_parent_dirs`、`platform_smoke_release_check_rejects_path_platform_mismatch` 和 `platform_smoke_path_platform_inference_requires_linux_x11_token`，覆盖目录分组 platform smoke、错放 artifact 和 Wayland false positive。验证通过 `rtk cargo test -p vesty-cli platform_smoke -- --nocapture` 18 passed、`rtk cargo test -p vesty-cli release_check -- --nocapture` 48 passed、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 321 passed、`rtk cargo test --workspace -j1` 当前 650 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 和 `rtk npm test`。
- 这仍只是 evidence consistency hardening；最终 release gate 仍要求真实 macOS、Windows x64 和 Linux X11 platform smoke reports。

本次 2026-06-12 收紧 import-ci platform smoke 路径平台一致性:

- `release-evidence import-ci` 现在在复制 platform smoke artifact 前也复用 artifact path 平台推断。如果路径明确指向 `macOS`、`Windows` 或 `Linux-X11`，但 report 内 `platform` 字段不同，会记录 `platform smoke artifact` failed item，并且不会把该 artifact 复制到 `release-evidence/platform-smoke/<platform>.json`。
- 这避免了导入阶段把 `Windows/platform-smoke.json` 中的 macOS report 规范化成 `platform-smoke/macos.json`，让坏证据在进入 release evidence bundle 前就留下诊断。Linux 路径推断仍要求同时出现 `linux` 和 `x11` token，避免 Wayland 或泛 Linux artifact 被当作 Linux X11。
- 扩展 `import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts`，覆盖 path/report 平台不一致的 platform smoke artifact 不会被复制，并在 `import-ci-report.json` 中包含 `artifact path indicates windows-x64` / `report platform is macos`。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli platform_smoke -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 321 passed、`rtk cargo test --workspace -j1` 当前 650 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，VST3 SDK audit artifacts 因缺失保持 optional skipped，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 release evidence。
- 这仍只是 evidence import/discovery hardening；最终 release gate 仍要求真实 macOS、Windows x64 和 Linux X11 platform smoke reports。

本次 2026-06-12 收紧 import-ci VST3 validate/static report 路径平台一致性:

- `release-evidence import-ci` 现在在复制 VST3 validator-passed report 或 static validate report 前，会从文件名或单独父目录推断明确平台 token。若路径指向 `macos`、`windows-x64` 或 `linux-x64`，report 的 `static_check.binaries` 必须包含对应平台；否则记录 failed item，并且不会复制到 `validator/` 或 `package/`。
- 该规则只使用强平台信号: 文件名平台 token，以及父目录刚好是 `macOS`、`Windows`、`windows-x64`、`linux-x64` 等单独平台目录。文件名如果同时包含多个平台 token 也会 failed，而不是退回成“无明确平台”；宽泛 CI job 目录名如 `linux-vst3-static-validate/` 不参与推断，避免误拒同一下载目录里的多平台矩阵 reports。
- 新增 `validate_report_artifact_path_platform()`、`validate_report_platform_from_artifact_path()` 和 `import_ci_rejects_validate_artifact_path_platform_mismatch`。测试覆盖 `Windows/VestyGain.validate.json` 携带 macOS report、`macOS/ThirdParty.static-validate.json` 携带 Windows report、`VestyGain.macos.windows-x64.validate.json` 歧义平台文件名都被拒绝，同时 `linux-vst3-static-validate/` 目录中的 3x3 static validate matrix 仍可正常导入。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_rejects_validate_artifact_path_platform_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli example_coverage -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 322 passed、`rtk cargo test --workspace -j1` 当前 651 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings` 无 warning、`rtk npm test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- strict `rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json` 仍按预期 failed；本地 ok 项仍是 host profiles、protocol snapshot、VST3 binding baseline、crate publish plan、crate package readiness、npm package pack report 和 dependency latest baseline，失败项仍是真实 DAW matrix、CI run URL、CI doctor/release-check artifacts、platform smoke、三示例/三平台 validator/static coverage、签名和 notarization evidence 缺失。本次没有生成新的真实外部 validator/static evidence。
- 这仍只是 evidence import/discovery hardening，不生成真实 validator/static evidence。

本次 2026-06-12 收紧 signing evidence 路径平台一致性:

- `release-evidence import-ci` 现在在导入签名日志或 macOS signed `.vst3` bundle 前，会从文件名或单独父目录推断明确平台 token。若路径指向 macOS 或 Windows，内容必须分别证明 macOS `codesign` 或 Windows `signtool`；否则记录 `signed bundle evidence` failed item，并且不会复制到 `signing-macos.log`、`signing-windows.log` 或 `signed-bundles/`。
- 最终 `signed bundle evidence` release gate 也复用同一检查，防止手工整理 evidence 时把 `signing-macos.log` 和 `Windows/signing.log` 放反后仍凑齐 macOS + Windows coverage。文件名同时表达 macOS 和 Windows 会 failed，而不是退回成“无明确平台”。
- 新增 `validate_signing_artifact_path_platform()`、`signing_platform_from_artifact_path()`、`import_ci_rejects_signing_artifact_path_platform_mismatch` 和 `signing_evidence_rejects_path_platform_mismatch`。测试覆盖 `Windows/signing.log` 携带 `codesign=pass`、`signing-macos-windows.log` 歧义文件名、`Windows/VestyGain.vst3` 携带 macOS `CodeResources` 都被拒绝，同时普通 `signing-artifacts/` 下载目录里的 macOS/Windows 日志仍可正常导入。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_rejects_signing_artifact_path_platform_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli signing_evidence_rejects_path_platform_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli signing_evidence -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture` 和完整 `rtk cargo test -p vesty-cli -- --nocapture`。
- 本次仍只是 release evidence consistency hardening，不生成真实签名或 notarization evidence。

本次 2026-06-12 收紧 notarization evidence 路径平台一致性:

- `release-evidence import-ci` 和最终 `notarization log` release gate 现在把 notarytool/stapler evidence 视为 macOS-only。路径文件名或单独父目录明确表达 Windows 或 Linux 时，即使日志内容包含 accepted notarytool 与 stapler success marker，也会记录 failed item 或 release gate failed，不会复制成 normalized `notary.log`。
- 这避免了下载 artifact 错放后把 `Windows/notary.log` 或 `Linux/stapler.log` 误归档为 macOS notarization evidence；`macOS`、`darwin`、`osx` 等路径 token 仍可正常通过路径一致性检查。文件名或父目录同时表达多个平台现在也会 failed，而不是先匹配 macOS 后放行。
- 新增 `validate_notarization_artifact_path_platform()`、`notarization_platform_from_artifact_path()`、`import_ci_rejects_notarization_artifact_path_platform_mismatch` 和 `notarization_evidence_rejects_path_platform_mismatch`。测试覆盖 Windows/Linux 路径中的正向 notary/stapler 日志被拒绝、`notary-macos-windows.log` 歧义文件名被拒绝，并验证最终 release gate 会输出 `notarization evidence is macOS-only` 或 multiple platform labels 诊断。
- 验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_rejects_notarization_artifact_path_platform_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli notarization_evidence_rejects_path_platform_mismatch -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli notarization -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 326 passed、`rtk cargo test --workspace -j1` 当前 655 passed、`rtk npm test`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`。
- 本次仍只是 release evidence consistency hardening，不生成真实 notarization/stapler evidence。

本次 2026-06-12 收紧 interface skeleton binary export plan drift 校验:

- `vesty-cli` 的 `generated-interface-skeleton.rs` validator 现在不只检查 binary export symbol/tool plan marker 是否存在，还会按 `vesty-vst3-sys::binary_export_symbol_plans()` 和 `vesty_vst3_sys::binary_export_inspection_tool_plans()` 核对 count 常量与每个平台的 symbol/tool record。旧 skeleton 如果保留 marker 但遗漏或篡改 Windows/macOS/Linux 任一 required export symbol、inspection tool 顺序或 tool 参数，会被 `emit-interface-skeleton --check`、`import-ci` 和 `release-check` 拒绝。
- `release-evidence` README 模板、根 README 和 `.agents` 文档同步把 `BINARY_EXPORT_INSPECTION_TOOL_PLANS` / `binary_export_inspection_tools()` 写入同一审计边界: 它们固定 future binary inspection 的 expected tool order，不读取真实 `.vst3` binary，不替代 `vesty validate --strict` 的 `static_check.binary_exports` evidence。
- 新增 `vst3_sdk_interface_skeleton_validator_tracks_vst3_sys_export_plans`，覆盖 stale inspection tool count、stale Linux `llvm-nm` tool record 和 stale Windows `InitDll` symbol record 会被 validator 拒绝。

本次 2026-06-12 将 interface skeleton inspection-tool plan 漂移覆盖到 release evidence 外层路径:

- 新增测试 helper 生成“真实 interface skeleton module + 篡改 Linux `llvm-nm` inspection tool plan”的 stale artifact，用同一份 fixture 覆盖 `release-check` 和 `release-evidence import-ci` 两条用户实际路径。
- `vst3_sdk_generated_rust_artifact_release_checks_are_optional_but_strict_when_present` 现在确认 stale `BINARY_EXPORT_INSPECTION_TOOL_PLANS` record 会让 `vst3 SDK generated bindings interface skeleton` release-check item failed，并包含 `missing vesty-vst3-sys binary export inspection tool plan` 诊断。
- `import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts` 现在确认 stale `generated-interface-skeleton.rs` 会被记录为 failed import item，不会被复制进 normalized `release-evidence/vst3-sdk/generated-interface-skeleton.rs`。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli vst3_sdk_generated_rust_artifact_release_checks_are_optional_but_strict_when_present -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture` 和 `rtk cargo test -p vesty-cli import_ci -- --nocapture`。本次仍只是 release evidence drift/diagnostics hardening，不生成真实 DAW、CI、validator、platform smoke、签名或 notarization evidence。

本次 2026-06-12 收紧 interface skeleton binary export plan array 完整性:

- `validate_vst3_sdk_generated_bindings_interface_skeleton_text()` 现在会切出 `BINARY_EXPORT_SYMBOL_PLANS` 和 `BINARY_EXPORT_INSPECTION_TOOL_PLANS` 数组体，要求数组内 record 数量分别等于 `vesty-vst3-sys` public plan 长度，并要求每条 expected symbol/tool record 在数组中恰好出现一次。
- 这补上了旧校验只证明 expected records 存在、但不能拒绝额外或重复 record 夹带的缺口；现在额外 `BinaryExportInspectionToolPlan`、重复 `BinaryExportSymbolPlan` 或篡改后又夹带一份正确 record 的 artifact 都会 failed。
- `vst3_sdk_interface_skeleton_validator_tracks_vst3_sys_export_plans` 新增 extra inspection-tool record 和 duplicate symbol record 两个样例，覆盖 `binary export inspection tool plan array contains ... expected ...` 与 `appears 2 time(s), expected exactly once` 诊断。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli vst3_sdk_interface_skeleton_validator_tracks_vst3_sys_export_plans -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 和完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 328 passed。本次仍只是 SDK audit/release evidence drift hardening，不生成真实外部 release evidence。

本次 2026-06-12 补齐 VST3 SDK audit 标准路径坏证据诊断覆盖:

- `release_evidence_dir_keeps_invalid_standard_vst3_sdk_surface_for_diagnostics` 扩展为 `release_evidence_dir_keeps_invalid_standard_vst3_sdk_artifacts_for_diagnostics`，覆盖 `vst3-sdk/vst3-sdk-headers.json`、`generated-bindings-plan.json`、`generated-bindings-surface.json`、`generated.rs`、`generated-abi-seed.rs`、`generated-abi.rs` 和 `generated-interface-skeleton.rs` 七个标准路径。
- 测试现在确认 `apply_release_evidence_dir()` 即使遇到坏 JSON / 坏 Rust audit module，也会保留标准路径给对应 release-check item 输出具体 failed 诊断，而不是把它当成 `not requested` 或静默跳过。
- 已验证每个 artifact 的失败诊断会包含对应文件路径和稳定错误锚点，例如 invalid manifest/plan/surface JSON、`must not claim SDK bindings are generated`、`must not claim full COM bindings are generated`。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_vst3_sdk_artifacts_for_diagnostics -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture` 和完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 328 passed。本次仍只是 standard release evidence diagnostics hardening，不生成真实 DAW、CI、validator、platform smoke、签名或 notarization evidence。

本次 2026-06-12 补齐 import-ci malformed named VST3 SDK Rust artifacts 覆盖:

- `import_ci_reports_malformed_named_json_artifacts` 改名并扩展为 `import_ci_reports_malformed_named_artifacts`，继续覆盖 malformed named JSON artifacts，同时新增 `vst3-sdk/generated.rs`、`generated-abi-seed.rs`、`generated-abi.rs` 和 `generated-interface-skeleton.rs` 四个标准命名 `.rs` audit artifacts。
- 这些 `.rs` fixture 不包含 generator marker，只依赖标准文件名和 `vst3-sdk/` 路径触发候选识别；测试确认 `release-evidence import-ci` 会把它们记录为对应 VST3 SDK audit item 的 failed import，而不是作为普通 Rust/text artifact 跳过。
- 已验证失败诊断包含 source path 与稳定错误锚点，包括 `must not claim SDK bindings are generated` 和 `must not claim full COM bindings are generated`，且不会复制任何坏 `.rs` 到 normalized `release-evidence/vst3-sdk/`。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli import_ci_reports_malformed_named_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture` 和完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 328 passed。本次仍只是 import-ci artifact recognition/diagnostics hardening，不生成真实外部 release evidence。

本次 2026-06-12 收紧 JSBridge `deliverBatch()` 入站批量上限:

- `@vesty/plugin-ui` 和 `vesty-ui-wry` 注入 bootstrap 同步新增 `MAX_BRIDGE_BATCH_PACKETS = 4096`；`window.__VESTY_INTERNAL__.deliverBatch(packets)` 在处理前会要求 `packets` 是 array 且长度不超过该上限，超限 batch 整体忽略。
- 这降低页面脚本或异常 host 回推把超大 packet array 丢进 UI thread 造成长时间同步遍历/handler 分发的风险，同时不影响正常 30/60 Hz meter/event flush batch。
- JS SDK 测试新增超限 4097 packets 被忽略、4096 packets 仍可处理的行为覆盖；wry bootstrap 字符串测试新增常量和 guard 断言，保持 System WebView runtime 与 TS SDK 行为一致。
- 已验证 `rtk npm test`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo check -p vesty-ui-wry --features wry-backend` 和 `rtk cargo fmt --all --check`。本次是 JSBridge/UI-thread robustness hardening，不生成真实 DAW 或 platform smoke evidence。

本次 2026-06-12 补齐 Rust -> WebView batch 回推分块:

- `vesty-ui-wry` 新增 Rust 端 `MAX_BRIDGE_BATCH_PACKETS = 4096` 和 `batch_scripts()`；wry IPC handler 不再把 native handler 返回的所有 response/event packet 合成一个无上限 `deliverBatch(...)`，而是按同一上限切成多个 UI-thread script 逐块 `evaluate_script()`。
- 这避免框架内部生成 4097+ packet response batch 时，被前一条 JS 端 `deliverBatch()` guard 整批忽略；Rust 端上限与 bootstrap 常量现在由测试绑定，降低 TS SDK、wry bootstrap 和 native runtime 行为漂移风险。
- 新增测试覆盖 `MAX_BRIDGE_BATCH_PACKETS + 1` 个 packet 会生成两个 batch script、每个 script 只调用一次 `deliverBatch(...)`，并覆盖空 batch 不生成 script。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo check -p vesty-ui-wry --features wry-backend` 和 `rtk npm test`。本次仍是 JSBridge/UI-thread robustness hardening，不生成真实 DAW、CI、validator、platform smoke、签名或 notarization evidence。

本次 2026-06-12 前置 WebView -> Rust IPC 消息大小检查:

- `@vesty/plugin-ui` 和 `vesty-ui-wry` bootstrap fallback 现在在 `postMessage()` 前构造完整 bridge packet、预先 `JSON.stringify()`，并用 UTF-8 byte length 按 lane 检查 Rust 端同款上限: command/param/event/meter/log/lifecycle 为 64 KiB，state 为 256 KiB。
- 超限请求会在 WebView 侧返回 retryable `backpressure`，不会调用 `window.ipc.postMessage()`，不会登记 pending request，也不会消耗 JS request `seq`；合法后续请求仍从同一个 `js-N` / `seq` 继续，降低 UI thread 和 native IPC 边界处理异常大 command payload 的成本。
- JS 测试新增多字节 command payload 超限拒绝、后续合法请求仍为 `js-1`，并覆盖较大 state payload 可按 256 KiB state lane 上限发送；wry bootstrap 字符串测试固定 `MAX_COMMAND_MESSAGE_BYTES`、`MAX_STATE_MESSAGE_BYTES`、`maxMessageBytesForLane()` 和 `utf8ByteLength(message)` guard，并把 bootstrap 常量数值绑定到 `vesty-ipc` 的 Rust `MAX_COMMAND_MESSAGE_BYTES` / `MAX_STATE_MESSAGE_BYTES`，避免 Rust/JS lane size 上限漂移。
- `.agents/12-jsbridge-design.md` 同步改为预序列化 `message`、按 lane byte 上限检查后 `postMessage(message)`，避免文档继续示例旧的直接 `JSON.stringify(packet)` 发送路径；seq 语义也写明只有通过本地 shape/size 校验、即将发送的 request 才消耗 `seq`。TS SDK 和 wry bootstrap 中旧的未用 `nextSeq()` helper 已删除。
- 已验证 `rtk cargo fmt --all --check`、`rtk npm test`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo check -p vesty-ui-wry --features wry-backend`、`rtk cargo test -p vesty-bridge -p vesty-ipc -p vesty-ui-wry -- --nocapture` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍是本地 JSBridge/WebView IPC robustness hardening，不生成真实 DAW、CI、validator、platform smoke、签名或 notarization evidence。

本次 2026-06-12 限制 async `event.flush` in-flight 积压:

- `@vesty/plugin-ui` 和 `vesty-ui-wry` bootstrap 的 async event pump 现在每个 bridge instance 同时最多保留一个 in-flight `event.flush` request；若上一轮 flush 仍未 resolve/reject，下一次 16ms interval tick 会直接跳过，不再继续登记新的 pending request。
- 正常 response、error、timeout 或 `postMessage` reject 后都会通过 `.finally()` 清掉 in-flight 标记；停止 event pump 不会清掉仍未 settle 的 in-flight 标记，避免退订后立刻重订阅绕过单 in-flight 保护。unload cleanup 会 reject pending request，然后由同一 `.finally()` 释放标记。这降低 host/UI 卡顿时 meter/param/fault/log 订阅导致 JS pending table 和 timer 堆积的风险。
- JS 测试新增“多次 interval tick 在首个 `event.flush` 未返回前只发送一次，返回后下一 tick 才发送第二次”的覆盖，并覆盖退订停止 pump 后立刻重订阅时仍不会在第一个 flush settle 前发送第二个 flush；wry bootstrap 字符串测试固定 `eventFlushInFlight`、skip guard、finally reset，以及 stopEventPump 不直接清零 in-flight 标记。
- 已验证 `rtk npm test`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo check -p vesty-ui-wry --features wry-backend` 和 `rtk cargo fmt --all --check`。本次仍是本地 JSBridge/UI-thread robustness hardening，不生成真实 DAW、CI、validator、platform smoke、签名或 notarization evidence。

本次 2026-06-12 补齐 async `event.flush` watchdog:

- `@vesty/plugin-ui` 的普通 request timeout 仍遵守 `CreateBridgeOptions.timeoutMs`，但内部 async event pump 发送的 `event.flush` 现在总是使用固定 `EVENT_FLUSH_TIMEOUT_MS = 1000`。因此开发者把 `timeoutMs` 设为 `0` 时，普通 command/state/param request 可继续禁用 JS 侧 timeout，而 meter/param.changed/diagnostics/log 的 pump 不会因为某次 flush response 丢失而永久保持 `eventFlushInFlight = true`。
- `vesty-ui-wry` bootstrap fallback 也新增同名 `EVENT_FLUSH_TIMEOUT_MS`，并把内部 `request(type, lane, payload, options)` 扩展为可选 per-request timeout；非 finite timeout option 会回落到默认 `REQUEST_TIMEOUT_MS`，避免全局 internal 入口误把 `NaN` 当作禁用 timeout。
- watchdog 超时会移除对应 pending request 并 reject 该 flush promise，由 `.finally()` 释放 in-flight 标记；迟到的旧 response 因 pending 已清理而被忽略，下一次 interval tick 可以发送新的 latest-wins flush。
- JS 测试新增 fake timer 覆盖 `timeoutMs: 0` 下普通 request 不创建 timer、`event.flush` 创建 1000ms watchdog、watchdog 触发后下一 tick 恢复发送，以及迟到旧 response 不影响新 flush；既有 event pump 测试也补了显式 flush response，避免测试 suite 等待 watchdog timer。
- wry bootstrap focused test 固定 `EVENT_FLUSH_TIMEOUT_MS`、`request("event.flush", "event", {}, { timeoutMs: EVENT_FLUSH_TIMEOUT_MS })` 和 finite timeout guard。
- 已验证 `rtk npm test`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture` 和 `rtk cargo check -p vesty-ui-wry --features wry-backend`。本次仍是本地 JSBridge/UI-thread robustness hardening，不生成真实 DAW、CI、validator、platform smoke、签名或 notarization evidence。

本次 2026-06-13 收紧 Linux X11 platform smoke WebView 证据:

- `vesty platform-smoke --write-report` 和 `release-check --platform-smoke-dir` 的 `system_webview` 校验继续要求平台特异证据；Linux X11 现在不仅要求同时出现 `WebKitGTK` 和 `X11`，还会拒绝包含 `Wayland`、`not X11`、`no X11`、`fallback`、`experimental` 或 WebKitGTK/X11 disabled/unavailable 语义的文本，避免 Wayland/fallback smoke 被误当作首版 Linux X11 通过证据。
- macOS/Windows WebView evidence 也新增对应否定语义拒绝，例如 `not WKWebView`、`without WebKit.framework`、`not WebView2` 或 `WebView2 disabled`，同时继续拒绝 `system_webview=true` 这类泛 marker。
- platform smoke 模板 README、`.agents/07-build-packaging.md` 和 `.agents/08-developer-guide.md` 已同步写明 Linux X11 evidence 必须是 active X11，不能带 Wayland/fallback/not-X11 描述。
- 新增/扩展 `platform_smoke_requires_platform_specific_webview_evidence`，覆盖即使文本含有正确关键词、但同时含有平台否定/Wayland fallback 描述也会失败。已验证 `rtk cargo test -p vesty-cli platform_smoke -- --nocapture`。本次仍只是 release evidence gate hardening，不生成真实 macOS/Windows/Linux X11 platform smoke evidence。

本次 2026-06-13 收紧 platform smoke `os` metadata 与平台一致性:

- `validate_platform_smoke_report()` 现在在 shape validation 后，会在 `os` metadata 存在时要求它与 `platform` 匹配。`platform = "macos"` 不能携带 `os = "Windows 11 x64"`，`platform = "linux-x11"` 的 `os` 必须同时体现 Linux 和 X11，且不能写成 Wayland session。
- `os` 字段仍保持 optional，便于 legacy/platform smoke artifacts 继续由权威 `platform` 字段判断；这次只拒绝显式自相矛盾的 metadata，避免 release evidence 中平台字段与 human-readable OS 字段互相打架。
- 新增 `platform_smoke_rejects_os_metadata_platform_mismatch`。已验证 `rtk cargo test -p vesty-cli platform_smoke -- --nocapture` 当前 20 passed、`rtk cargo test -p vesty-cli platform_smoke_rejects_os_metadata_platform_mismatch -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 358 passed、`rtk cargo test --workspace -j1` 当前 703 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk npm test` 和 `rtk cargo fmt --all --check`。本次仍只是 platform smoke artifact metadata hardening，不生成真实 macOS、Windows x64 或 Linux X11 platform smoke evidence。

本次 2026-06-13 拒绝 ambiguous platform-smoke artifact 路径:

- `platform_smoke_platform_from_artifact_path()` 现在会收集 path 中的 macOS、Windows 和 Linux-X11 平台 token；如果同一路径同时命中多个平台，例如 `macos-windows/platform-smoke.json` 或 `linux-x11-windows/platform-smoke.json`，会返回 failed 诊断 `artifact path contains multiple platform tokens`，不再按第一个 token 归类。
- `release-check --platform-smoke-dir` 和 `release-evidence import-ci` 都复用该歧义拒绝逻辑；导入阶段遇到 ambiguous platform smoke artifact 不会复制到 normalized `release-evidence/platform-smoke/<platform>.json`。
- `.agents/07-build-packaging.md` 与 `.agents/08-developer-guide.md` 已同步说明多平台 token 路径会被视为 ambiguous evidence 并拒绝。
- 新增 `platform_smoke_release_check_rejects_ambiguous_path_platform_tokens`，并扩展 `import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts` 覆盖 ambiguous path 不导入。已验证 `rtk cargo test -p vesty-cli platform_smoke -- --nocapture` 和 `rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture`。本次仍只是 release evidence path-consistency hardening，不生成真实 platform smoke、DAW、CI、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 DAW matrix write-report 平台范围:

- `vesty daw-matrix --write-report` 的 `--platform` 现在必须能映射到对应 host profile 声明支持的平台。`macOS` / `Windows` / `Linux X11` 会分别归一到 `macos`、`windows`、`linux-x11`；若文本只写泛 `Linux` 而无 `X11`、包含 `Wayland`、缺少可识别平台，或 host profile 未声明该平台，例如 Ableton Live + Linux X11，会在写入任何 evidence 文件前失败。
- 这让 DAW matrix 的规范写入路径与 `.agents` 和 `vesty-core::host_profiles()` 的 MVP 支持面保持一致，避免把 out-of-scope Wayland 或 host/platform 不支持组合写成看似完整的 DAW smoke。
- 新增 `daw_matrix_write_report_validates_host_platform_scope`，覆盖 Bitwig Linux X11 可写、Bitwig Wayland 拒绝且不覆盖已有 X11 evidence、Ableton Linux 拒绝、Studio One 模糊平台拒绝。已验证 `rtk cargo test -p vesty-cli daw_matrix -- --nocapture`。本次仍只是本地 DAW evidence writer hardening，不生成真实 Cubase/Nuendo、REAPER、Bitwig、Ableton Live 或 Studio One smoke evidence。

本次 2026-06-13 收紧 DAW matrix 读取端平台证据:

- `daw_matrix_rows()` 现在按 `vesty-core::HostProfile` 收集每个 host 行；读取 `platform.txt` 时会复用同一套 host/platform scope 校验，并把结果写入内部 `platform_supported` 布尔字段。
- `daw_row_complete()`、`daw_missing_checks()`、`missing_smoke_checks()` 和 release-check 的 per-host smoke item 现在都会把 `platform_supported != true` 视为缺失 `platform` evidence。手工把已通过的 Ableton evidence 改成 `Linux X11`、把 Bitwig 改成 `Linux Wayland`，或只写泛 `Linux`，即使其它 smoke marker 都为 pass，也不会再让 `vesty daw-matrix --strict` 或 `vesty release-check` 误判为完整；Markdown 矩阵的 `Platform` 列也会标注 `(unsupported)` / `(unknown)`，让人工读报告时能直接看到问题。
- release-check JSON shape validator 也要求 `daw_matrix[].platform_supported` 如果存在必须是 boolean，避免外部报告用字符串 `"true"` 之类值伪装平台通过状态。
- 新增 `daw_matrix_read_rejects_host_unsupported_platform_evidence`、`daw_matrix_read_rejects_wayland_and_generic_linux_platform_evidence`、`daw_matrix_platform_text_marks_unsupported_or_unknown_platforms` 和 `release_check_report_shape_rejects_non_boolean_daw_platform_support`。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli daw_matrix -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 334 passed，以及 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 DAW evidence gate hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 DAW matrix marker host scope:

- DAW evidence marker 读取现在识别显式 host scope assignment: `host=...`、`daw=...`、`daw_host=...`、`host_profile=...` 或 `profile=...`。如果该值能映射到内置 host profile，必须匹配当前 evidence 目录对应的 profile；无法识别或指向另一个 DAW 的 marker 会按未通过处理。没有显式 host scope 字段的旧 marker log 仍保持兼容。
- `write_daw_smoke_report()` 的规范写入路径也复用同一 profile-aware marker parser，因此 `vesty daw-matrix --write-report --host bitwig --scan "host=Ableton Live\nscan=true"` 会在写入任何文件前失败。
- 新增 `daw_evidence_rejects_explicit_host_scope_mismatch_markers` 和 `daw_matrix_write_report_rejects_explicit_host_scope_mismatch_marker`，覆盖错放 host-scoped marker 不会让 Bitwig 行通过。已验证 `rtk cargo test -p vesty-cli daw_matrix -- --nocapture`、`rtk cargo test -p vesty-cli daw_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 336 passed，以及 `rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`。本次仍只是本地 DAW evidence gate hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 release-check DAW matrix 平台状态自校验:

- `validate_release_check_daw_matrix_shape()` 现在会在 row host 能映射到内置 `HostProfile` 时，复验 `platform_supported` 与 `platform` 文本是否一致。`platform_supported = true` 但平台文本无法通过该 host 的 supported platform 规则会失败；`platform_supported = false` 但平台文本其实是受支持平台也会失败。
- 这补上了手工编辑或外部 CI artifact 伪造 `daw_matrix[].platform_supported` 的缺口，让 `release-check` JSON snapshot 自身不能与 Vesty host profile 平台规则相矛盾。
- 新增 `release_check_report_shape_rejects_inconsistent_daw_platform_support`，覆盖 Ableton Live + Linux X11 被标 true、以及 macOS Ableton smoke 被标 false 两个方向。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 337 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 release evidence shape hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 release-check DAW matrix host 集合:

- `validate_release_check_daw_matrix_shape()` 现在要求 `daw_matrix` 精确包含当前 `vesty-core::host_profiles()` 的 canonical host set: REAPER、Cubase/Nuendo、Bitwig Studio、Ableton Live 和 Studio One。缺失行、重复 profile、无法映射 host、拼写错误、额外第三方 host 行，以及别名 host（例如 `reaper`）都会在 report shape validation 阶段失败。
- `host_profile_release_check()` 现在复用同一套 host-set diff；通过时 value 会精确列出当前 release profile 名称，失败时区分 missing/duplicate/unknown/non-canonical profile rows。CI per-OS release-check artifact invariant validator 也要求 `host profiles` check value 与当前 canonical host set 完全一致，不再只接受一个看似正确的数量。
- `validate_release_check_report_shape()` 现在还会从 `daw_matrix` 明细重新计算 `host profiles`、`daw matrix` 和每个 `daw smoke:<host>` check 的 expected status/value，并要求 report 中的对应 check 完全一致。手工或 CI artifact 如果把 summary 改成 ok/failed 但没有同步明细，或把某个缺项 host 的 `daw smoke` 行伪造成 ok，会在导入/写入 report 前失败。
- 这避免外部/手工 release-check JSON 用别名、重复行、缺行或伪造 `host profiles` value 让 release matrix 和 host profile coverage 语义变得含糊。第三方 DAW smoke 仍可作为独立补充文档，但不进入当前 MVP release gate。
- 新增/更新 `release_check_report_shape_rejects_unknown_daw_matrix_hosts`、`release_check_report_shape_requires_exact_canonical_daw_matrix_hosts`、`release_check_report_shape_rejects_daw_summary_drift`、`release_check_report_shape_rejects_daw_smoke_row_drift` 和 `ci_release_check_artifacts_reject_duplicate_or_forged_invariant_checks` 覆盖未知 host、缺行、重复 profile、别名 host、forged `host profiles` invariant、DAW summary drift 和 per-host smoke drift。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 341 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 release evidence shape hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 release evidence audit report item 语义:

- `validate_local_release_evidence_item_shape()` 现在要求 `status = ok` 的 local/collected evidence item 必须携带 evidence path，避免 ok 记录无法指向实际产物。
- `validate_import_ci_release_evidence_item_shape()` 现在校验 import item 的 status/source/path 语义: `ok` 和 `imported` 必须有 output path，`failed` 不能声明 output path，`skipped` 只有在 `destination exists; pass --overwrite to replace` 这种未覆盖已有目标的情况下才允许携带 output path。
- 这避免手工或损坏的 `import-ci-report.json` 把失败项写成已导入、有目标路径的假象，或把导入项写成没有输出落点的不可审计记录。该 report 仍只是 audit metadata，不作为 release-check pass evidence。
- 新增/扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 ok 无 path、imported 无 output、failed 带 output、skipped 非 destination-exists 却带 output，以及合法 skipped-existing item。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 341 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 release evidence audit hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 import-ci audit report 路径 containment:

- `validate_import_ci_release_evidence_report_shape()` 现在在 item status/source/path 语义之外，还会校验完整 `import-ci-report.json` 的路径归属: artifact item 的 `source` 必须词法上位于 report 声明的 `source` root 下，item 的 output `path` 必须位于 report 声明的 `evidence_dir` root 下。
- `validate_collected_release_evidence_report_shape()` 现在也要求 collect-signing / collect-notarization report 的顶层 `output` 和每个 ok item `path` 位于 report 声明的 `evidence_dir` 下。
- 显式 `ci run url` source 可能来自 `--ci-run-url-file`，允许作为可审计外部 source 例外；其它从下载 artifact root 扫描出的 evidence 仍必须同根。校验不做 filesystem canonicalize，因此适用于不存在的 report shape fixture；同时会拒绝 root path 中的 `..`，避免把可信 root 扩大到父目录。
- 新增 `LexicalReleaseReportPath` helper 覆盖 Unix/Windows 风格路径、`.`/`..` 归一化和绝对/相对形态隔离；report 中把 source 指向下载根之外、output 指向 release evidence 目录之外、或把 root 写成 `target/downloaded-artifacts/..` 都会失败。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 import source outside root、import output outside evidence root、collected output outside evidence dir、collected item path outside evidence dir、root escape、child escape 和合法显式 CI run URL 文件例外。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 341 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 release evidence audit hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 local release evidence report 自洽:

- `validate_local_release_evidence_report_shape()` 现在会先校验 `evidence_dir` 是安全 report root，不允许通过 `..` 扩大本地 evidence 根。
- 除 `protocol snapshot` item 外，`local-collect-report.json` 中所有带 path 的 item 必须位于顶层 `evidence_dir` 下；`protocol snapshot` 保持可位于独立 `target/vesty-protocol` 目录的既有设计。
- 顶层 `protocol_snapshot` 与明细 item 现在必须完全一致: 有顶层 `protocol_snapshot` 时必须存在且只存在一条 `name = "protocol snapshot"`、`status = ok`、`path = protocol_snapshot` 的 item；缺少顶层字段时不能出现 protocol snapshot item。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 local non-protocol item 路径越界、local root `..` escape、protocol item path drift、缺顶层 protocol snapshot 却有 item，以及重复 protocol snapshot item。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli collect_local -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 341 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 release evidence audit hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 release action plan sidecar evidence path 自洽:

- `validate_release_action_plan_sidecar()` 现在会对 Vesty 已知 action 类型复验 `evidence_path`。`protocol snapshot` action 必须指向顶层 `protocol_snapshot`；`daw matrix` 和 `daw smoke:<host>` 必须指向顶层 `evidence_root` 及其规范 host 子目录；CI、platform smoke、validator/static、publish/npm/dependency/VST3 SDK/signing/notary actions 必须指向顶层 `release_evidence_dir` 推导出的标准路径。
- 这避免 per-OS `release-action-plan-<OS>.json` sidecar 的 summary/commands 指向标准目录，但 `evidence_path` 被手工改到其它目录，从而误导外部采证。未知未来 action 暂不强制路径，避免破坏前向兼容。
- 更新 `test_release_action_plan()` fixture，使 `vst3 SDK header manifest` action 的 evidence path 指向当前生产格式 `target/release-evidence/vst3-sdk/vst3-sdk-headers.json`，不再使用旧的目录级路径。
- 扩展 `release_action_plan_sidecar_rejects_incomplete_actions` 覆盖 DAW evidence path 漂移、缺失 expected evidence path、release evidence path 漂移和 protocol evidence path 漂移。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 341 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 release action plan/checklist metadata hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 release action plan sidecar 路径安全:

- `validate_release_action_plan_sidecar()` 现在会对顶层 `protocol_snapshot`、`evidence_root`、`release_evidence_dir` 以及 action `evidence_path` 执行词法安全校验，复用 release evidence report 的 Unix/Windows 风格路径归一化 helper，并拒绝包含 `..` parent-directory component 的路径。
- 这补上了路径一致性校验中的 root escape 缺口: action plan 不能把顶层 root 写成 `target/daw-evidence/..` 后再让派生 `evidence_path` 与该不安全 root 匹配。`signed bundle evidence` 的复合 evidence 字符串仍按 exact-match 处理，避免把两个标准签名日志误当成单一路径解析。
- 扩展 `release_action_plan_sidecar_rejects_incomplete_actions` 覆盖 protocol snapshot root escape、DAW evidence root escape、release evidence dir escape 和 action evidence path escape；新增 `release_action_plan_sidecar_accepts_signed_bundle_compound_evidence_path` 锁定合法复合签名 evidence path。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 action-plan checklist metadata hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 local release evidence protocol snapshot 路径安全:

- `validate_local_release_evidence_report_shape()` 现在对顶层 `protocol_snapshot` 执行词法安全校验；`validate_local_release_evidence_item_paths()` 对 `name = "protocol snapshot"` 的 item path 也执行同样校验。
- 该例外路径仍允许位于 `evidence_dir` 外，例如独立的 `target/vesty-protocol`，但不再允许 `target/vesty-protocol/..` 这类 parent-directory component。这样 local audit report 不能用一个自洽但不安全的 protocol snapshot path 绕过 evidence dir containment 规则。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖顶层和 item 都写成 escaped protocol snapshot path 的情况。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli collect_local -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 audit report hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 collected release evidence report 输出自洽:

- `validate_collected_release_evidence_report_shape()` 现在要求 collected signing/notarization report 中每个 ok item 的 `path` 必须等于顶层 `output`。此前只要求二者都位于 `evidence_dir` 下，手工 JSON 可以把顶层 output 指向 `signing-macos.log`，但 item path 指向另一个标准槽位。
- 这与 `collect-signing` / `collect-notarization` 的实际生成模型一致: 每次 collection 只写一个 normalized evidence output，并用一个 item 描述该 output。该 report 仍只是 audit metadata，不作为 release pass evidence。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 item path 仍在 evidence dir 内、但与顶层 output 不一致的情况。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli collect_notarization -- --nocapture`、`rtk cargo test -p vesty-cli signing_evidence -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是本地 audit report hardening，不生成真实签名、notarization 或其它外部 release evidence。

本次 2026-06-13 收紧 import-ci 外部 CI run URL source 路径安全:

- `validate_import_ci_release_evidence_item_paths()` 仍允许 `name = "ci run url"` 的 source 来自 report 顶层 `source` root 之外，因为它可能对应显式 `--ci-run-url-file`。但该外部 source 现在也必须通过词法路径安全校验，不能包含 `..` parent-directory component。
- 这把例外权限限制为“允许外部文件”，而不是“跳过所有路径形态校验”。合法 `target/manual-ci-run-url.txt` 仍通过；`target/manual/../ci-run-url.txt` 会被拒绝。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖外部 `ci run url` source path escape。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli ci_run_url -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 import-ci audit report hardening，不生成真实 CI run 或其它外部 release evidence。

本次 2026-06-13 收紧 import-ci CI run URL item value 自洽:

- `validate_import_ci_release_evidence_item_shape()` 现在对 `name = "ci run url"` 且 `status = ok/imported` 的 item 要求 `value` 必须是有效 GitHub Actions run URL。这样手工或损坏的 `import-ci-report.json` 不能把 CI run URL item 标记为 imported，却把 value 写成普通说明文字。
- `status = skipped` 的无 URL 或 destination-exists 语义保持不变；已有合法导入路径仍会把真实 run URL 写入 value。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 imported CI run URL item value 不是 URL 的情况。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli ci_run_url -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 import-ci audit report hardening，不生成真实 GitHub Actions run 或下载 artifacts。

本次 2026-06-13 收紧 import-ci CI run URL output 槽位自洽:

- `validate_import_ci_release_evidence_item_paths()` 现在要求 `name = "ci run url"` 的 item output `path` 必须精确等于 `evidence_dir/ci-run-url.txt`。此前只要求 path 位于 `evidence_dir` 下，手工 JSON 可以把 CI provenance 指向其它文件名。
- 这与 `import_ci_run_url_evidence()` 的实际写入路径一致，并让 `import-ci-report.json` 的 provenance output 与 release evidence 模板标准槽位保持一致。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 CI run URL output path 位于 evidence dir 下但不是标准 `ci-run-url.txt` 的情况。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli ci_run_url -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 import-ci audit report hardening，不生成真实 CI provenance 或其它外部 release evidence。

本次 2026-06-13 收紧 release evidence template item path 自洽:

- `validate_local_release_evidence_item_paths()` 和 `validate_import_ci_release_evidence_item_paths()` 现在要求 `name = "release evidence template"` 的 ok item path 必须精确等于对应 report 的 `evidence_dir`。此前只要求路径位于 `evidence_dir` 下，手工 report 可以把模板初始化记录挂到任意子目录。
- 这与 `collect-local --template` 和 `import-ci --template` 的实际生成行为一致: 模板初始化描述的是整个 release evidence root，而不是某个子 artifact。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 local/import 两类 template item path drift。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli collect_local -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 local/import audit report hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 import-ci template item source 语义:

- `validate_import_ci_release_evidence_item_shape()` 现在要求 `name = "release evidence template"` 的 import-ci item 不能携带 `source`。该 item 表示本地在 release evidence dir 中初始化模板，而不是从下载 artifact root 识别出的外部 evidence。
- 这避免手工或损坏的 `import-ci-report.json` 把本地模板初始化记录伪装成来自 CI artifact 的 evidence。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 template item 带 source 的情况。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 import-ci audit report hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 release evidence template item status 语义:

- `validate_local_release_evidence_item_shape()` 和 `validate_import_ci_release_evidence_item_shape()` 现在要求 `name = "release evidence template"` 的 item status 必须为 `ok`。模板初始化是本地确定性动作，不应该被手工 report 标成 `skipped`、`imported` 或 `failed`。
- 这让 local/import audit report 中的模板记录同时满足: status 为 `ok`、path 等于 `evidence_dir`，且 import-ci 版本不带 source。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 local template `skipped` 和 import-ci template `imported` 两类 status drift。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli collect_local -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 local/import audit report hardening，不生成真实外部 release evidence。

本次 2026-06-13 扩展 import-ci 固定标准槽位校验:

- `expected_import_ci_release_evidence_item_path()` 现在除 `ci run url` 外，还覆盖固定输出路径的 import-ci item: `protocol snapshot`、crate publish/package/npm/dependency reports，以及 VST3 SDK manifest/plan/surface/scaffold/ABI seed/ABI layout/interface skeleton audit artifacts。
- 这些 item 的 output path 必须精确匹配 release evidence 标准布局，例如 `publish-plan/publish-plan.json`、`crate-package/crate-package.json`、`dependency-baseline/dependency-baseline-latest.json`、`vst3-sdk/generated.rs` 和 `notary.log`。动态路径的 validator/static matrix、platform smoke、per-OS CI snapshots、action plans 和 signed bundle evidence 暂不纳入该固定映射。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 `crate publish plan`、`vst3 SDK generated bindings scaffold` 和 `notarization log` 的 output path drift。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 342 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 import-ci audit report hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 import-ci 成功 evidence output 唯一性:

- `validate_import_ci_release_evidence_report_shape()` 现在会在完整 report 级别检查 `status = ok/imported` 的 item output path 唯一性。同一个 output path 不能被两个成功 item 同时认领，避免手工或损坏的 `import-ci-report.json` 在后续聚合时无法判断哪条 evidence 才是权威来源。
- `status = skipped` 且 value 为 `destination exists; pass --overwrite to replace` 的重复 output path 仍允许存在，用于保留重复 artifact 被跳过的正常诊断；失败项仍不能带 output path。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖两个 imported item 共享同一 output path 会失败，以及 imported + skipped-existing 共享 path 仍通过。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 和 `rtk cargo test -p vesty-cli release_evidence -- --nocapture`。本次仍只是 import-ci audit report hardening，不生成真实 CI、DAW、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 import-ci signed bundle evidence 动态路径形态:

- `validate_import_ci_release_evidence_item_paths()` 现在对 `name = "signed bundle evidence"` 的成功 output path 执行专用形态校验。允许的落点只有 `signing-macos.log`、`signing-windows.log`、`signing/<safe-name>.log` 和 `signed-bundles/<safe-bundle>.vst3`，全部都必须位于 report 的 `evidence_dir` 下。
- 这保留了 signed evidence 的真实动态语义: macOS codesign 日志、Windows signtool 日志、unknown/mixed signing 日志和 signed macOS `.vst3` bundle 可以共用同一个 item name；但手工 report 不能把成功签名 evidence 指到 `signing-copy.log`、`signed-bundles/foo.txt` 或其它随意槽位。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖四类合法 signing output，以及 root-level signing log drift 和 signed-bundles 非 `.vst3` drift。已验证 `rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 和 `rtk cargo test -p vesty-cli release_evidence -- --nocapture`。本次仍只是 import-ci audit report hardening，不生成真实签名或其它外部 release evidence。

本次 2026-06-13 收紧 import-ci 其它动态 artifact 输出形态:

- `validate_import_ci_release_evidence_item_paths()` 现在也会校验非固定但有生产规范的动态输出路径: `ci doctor artifact` 必须落在 `ci-doctor/doctor-<OS>.json`；`ci release-check artifact` 和 `release action plan sidecar` 必须落在 `ci-release-checks/release-check-<OS>.json` / `release-action-plan-<OS>.json`；`platform smoke artifact` 必须落在 `platform-smoke/<platform>.json`；`vst3 validate report` / `vst3 static validate report` 必须落在 `validator/<safe-bundle>.<platform>.validate.json` / `package/<safe-bundle>.<platform>.static-validate.json`。
- 这些规则复用 release report 的词法 path parser 和 `safe_evidence_filename_part()`，防止手工 `import-ci-report.json` 把成功 artifact 指到同一 evidence dir 下的任意文件名，例如 `ci-doctor/linux-doctor.json`、`platform-smoke/linux.json` 或 `validator/*.validator.json`。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 CI doctor、CI release-check、release action plan sidecar、platform smoke、validator 和 static validate 的动态 path drift。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 和 `rtk cargo test -p vesty-cli platform_smoke -- --nocapture`。本次仍只是 import-ci audit report hardening，不生成真实 CI、platform smoke、validator 或其它外部 release evidence。

本次 2026-06-13 收紧 release-check canonical gate 集合:

- `validate_release_check_report_shape()` 现在会要求 release-check report 的 check name 集合精确匹配当前 Vesty release gate，包括所有生产 gate 与由 `vesty_core::host_profiles()` 派生的 per-host DAW smoke checks。缺少 `ci run url` 这类当前 gate，或额外出现 `manual extra gate` 这类未知 gate，都会在 report shape validation 阶段失败。
- DAW matrix shape validation 仍优先执行，因此非 boolean `platform_supported`、unsupported platform 与 host-set mismatch 这类 DAW-specific 错误不会被 canonical check-set 错误掩盖；CI per-OS release-check fixture 改为复用生产 `build_release_check_report()`，相关失败测试通过修改已有 gate 而不是追加重复 check 来表达。
- 新增 `release_check_report_shape_requires_current_check_set` 覆盖缺失/未知 gate；更新 CI release-check artifact 测试覆盖 crate package、platform smoke 和 VST3 SDK audit gate 失败仍可作为允许的外部 evidence gap 被保留。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture`、`rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 release-check report hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 release action plan sidecar canonical gate 集合:

- `validate_release_action_plan_sidecar()` 现在要求 sidecar summary 的 `ok + failed + skipped` 总数等于当前生产 `build_release_check_report()` 派生出的 release gate 数量，并要求每个 action check 都属于当前 release gate。这样 per-OS `release-action-plan-<OS>.json` 不能少报当前 gate，也不能塞入 `manual follow-up` 之类未知 action。
- `expected_release_check_names()` 改为从生产 release-check builder 派生，而不是维护第二份手写 gate 列表，避免 action-plan/report canonical 校验和实际 release-check 输出再次漂移。相关测试数据也改成完整 canonical DAW host set 后再制造单个 host 缺项。
- 扩展 `release_action_plan_sidecar_rejects_incomplete_actions` 覆盖 summary 总数少于当前 gate 和未知 action check；已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 action-plan checklist metadata hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 local collect 固定 evidence 槽位:

- `validate_local_release_evidence_report_shape()` 现在除 evidence-dir containment、protocol snapshot 例外和 template item 规则外，还会复用固定 release evidence 槽位映射，要求本地 `collect-local-report.json` 中的 publish/package/npm/dependency 与 VST3 SDK audit item 指向标准路径，例如 `publish-plan/publish-plan.json`、`npm-pack/npm-pack.json`、`dependency-baseline/dependency-baseline-latest.json` 和 `vst3-sdk/generated-abi.rs`。
- `expected_import_ci_release_evidence_item_path()` 和 local report 校验共用 `fixed_release_evidence_item_relative_path()`，避免 local/import 两套固定槽位规则漂移。`protocol snapshot` 仍保持独立 `protocol_snapshot` 顶层一致性规则，`release evidence template` 仍要求 path 等于 `evidence_dir`。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 local item 在 evidence dir 内但偏离标准槽位的情况。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli collect_local -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 local audit report hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 collected signing/notarization 输出槽位:

- `validate_collected_release_evidence_report_shape()` 现在不仅要求 `collect-signing` / `collect-notarization` 的 JSON report 顶层 `output` 和 ok item `path` 位于 `evidence_dir` 下且彼此一致，还会按 kind/item 校验标准输出槽位: macOS signing 必须是 `signing-macos.log`，Windows signing 必须是 `signing-windows.log`，notarization 必须是 `notary.log`。
- 这让显式 `--out` 不能把成功采集的签名/公证 evidence 写到 `signing/codesign.log` 或 `notary-copy.log` 这类 release-check 自动发现不到的路径。Linux signing 仍不由 `collect-signing` 生成，保持 release-channel policy。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 collected signing output drift 和 collected notarization output drift。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli collect_notarization -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 collected audit report hardening，不生成真实签名或 notarization evidence。

本次 2026-06-13 收紧 platform smoke 报告标准 check 集合:

- `validate_platform_smoke_report_shape()` 现在要求 platform smoke report 的 check name 集合精确匹配当前 `REQUIRED_PLATFORM_SMOKE_CHECKS`。报告仍会先做文本安全、数量上限和重复 check 校验；随后未知 check 会以 `unknown platform smoke check(s)` 失败，缺少标准 check 会以 `platform smoke report missing required check(s)` 失败。
- pending template 继续由同一组标准 check 生成并保持可识别；真实 evidence report 不能再通过手工添加 `extra-check` 或删除 `jsbridge_roundtrip` 等必需项来伪造或弱化平台烟测覆盖。
- 扩展 `platform_smoke_rejects_malformed_report_shape` 覆盖未知 check 和缺失必需 check。已验证 `rtk cargo test -p vesty-cli platform_smoke -- --nocapture`、`rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 platform smoke artifact schema hardening，不生成真实 macOS、Windows x64 或 Linux X11 platform smoke evidence。

本次 2026-06-13 收紧 smoke-host 诊断报告标准 check 集合:

- `validate_smoke_host_report_shape()` 现在要求 smoke-host report 的 check name 集合精确匹配生产 `build_smoke_host_report()` 从 `SMOKE_HOST_EXAMPLES` 派生出的诊断项: workspace manifest、每个示例的 config/parameter sidecar、Web UI 示例的 UI assets、JSBridge trace 和 meter stream。
- 这让 CI 中上传的 `vesty-smoke-host` 诊断 artifact 不能手工多塞未知 `extra check`，也不能删除 `JSBridge trace` 等固定诊断项后仍通过 shape validation。该报告仍是本地 headless framework self-check，不替代真实 DAW、platform WebView、validator、签名或 notarization evidence。
- 扩展 `smoke_host_report_rejects_malformed_shape_fields` 覆盖未知 check、缺失必需 check 和固定集合下的文本清洗诊断。已验证 `rtk cargo test -p vesty-cli smoke_host -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 smoke-host diagnostic schema hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 CI doctor artifact 未知 check:

- `validate_doctor_report()` 现在会拒绝不属于当前 doctor 输出面的未知 check。已知集合覆盖 toolchain、Node/npm、VST3 binding baseline、VST3 SDK headers、validator、system WebView、Linux/macOS/Windows signing/notarization preflight、unsupported-platform signing fallback，以及五个 DAW install hint。
- 这样下载后的 `doctor-<OS>.json` 不能手工追加 `manual extra doctor check` 之类未知项来污染 CI doctor artifact；缺失或状态不合格的 required check 仍由 `ci_doctor_artifacts_release_check()` 按 Linux/macOS/Windows 路径 OS 做精确判断，legacy 无 `os` label 的报告仍保持兼容。
- 扩展 `ci_doctor_artifacts_reject_malformed_report_shape` 覆盖未知 doctor check，并修正测试 fixture 让每个畸形 case 从干净 Linux report 出发，避免错误优先级互相遮挡。已验证 `rtk cargo test -p vesty-cli ci_doctor -- --nocapture`、`rtk cargo test -p vesty-cli doctor_report_includes_toolchain_webview_and_validator_checks -- --nocapture`、`rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 CI doctor artifact schema hardening，不生成真实 GitHub Actions artifact。

本次 2026-06-13 收紧 CI doctor artifact 跨 OS check 集合:

- `missing_doctor_checks()` 现在除 required check 缺失/状态检查外，还会按 artifact/report 推断出的 OS 拒绝该 OS 不应出现的 doctor check。Linux doctor report 不能夹带 `signing: codesign` / `signing: notarytool` / `signing: signtool`，macOS report 不能夹带 Linux/Windows signing policy，Windows report 不能夹带 Linux/macOS signing/notarization preflight。
- 该校验在已确定 artifact OS 后执行，因此仍保留已有的 path/report `os` mismatch 诊断优先级；legacy 无 `os` label 的 report 会按路径 OS 做同样的跨 OS check-set 校验。`import-ci` 也复用同一路径，失败 artifact 不会被复制到 `release-evidence/ci-doctor/doctor-<OS>.json`。
- 新增 `ci_doctor_artifacts_reject_cross_os_checks`、`ci_doctor_artifacts_reject_legacy_cross_os_checks_from_path_os` 和 `import_ci_rejects_cross_os_doctor_artifacts`。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_doctor_artifacts -- --nocapture` 当前 14 passed、`rtk cargo test -p vesty-cli import_ci_rejects_cross_os_doctor_artifacts -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 357 passed、`rtk cargo test --workspace -j1` 当前 702 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk npm test`。本次仍只是 CI doctor evidence schema/semantic hardening，不生成真实 GitHub Actions artifact。

本次 2026-06-13 收紧 dependency baseline report 标准 check 集合:

- `validate_dependency_baseline_report_shape()` 现在会根据 report 是否包含 registry latest check，要求 check key 集合精确匹配生产 `dependency_baseline_report_with_optional_latest()` 派生出的 baseline-only 或 baseline+latest 集合。基础集合覆盖 workspace cargo baseline coverage、所有锁定 Rust 依赖、VST3 SDK/crate baseline、JS package TypeScript baseline、package-lock baseline 和 React/Vue/Svelte adapter latest-range/lockfile baseline；latest 集合额外覆盖 crates.io/npm registry latest checks。
- 未知 check 会以 `unknown dependency baseline check(s)` 失败，缺失基础项或 latest 项会以 `dependency baseline report missing required check(s)` 失败。`write_dependency_baseline_report()` 因为复用 shape validation，现在也不会写出带未知/缺失 check 的 report；测试中需要模拟下载来的坏 artifact 时改为手写 JSON。
- 扩展 `dependency_baseline_report_rejects_malformed_shape_fields` 覆盖未知 check 和缺失必需 check；更新 check-mode、release-evidence/import-ci 的坏 dependency artifact 测试以匹配新的 canonical-set 拒绝语义。已验证 `rtk cargo test -p vesty-cli dependency_baseline -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 dependency evidence schema hardening，不生成真实 registry review 或外部 release evidence。

本次 2026-06-13 收紧 VST3 SDK generated bindings plan 标准 check 集合:

- `validate_vst3_sdk_binding_plan_shape()` 现在要求 plan checks 精确匹配生产 `vesty_vst3_sys::generated_bindings_plan()` 的三项: `sdk header inputs`、`bindings module path` 和 `binding emitter`。blocked plan 仍可表达 failed checks 和 blockers，但不能手工添加 `manual extra plan check`，也不能删除 `binding emitter` 这类关键 metadata。
- 未知 check 会以 `unknown VST3 SDK binding plan check(s)` 失败，缺少必需 check 会以 `VST3 SDK binding plan missing required check(s)` 失败。content validation 仍继续要求 `bindings_generated = false`、`ready-for-binding-generator`、无 blockers、baseline/backend 匹配和 reserved binding emitter 语义。
- 扩展 `vst3_sdk_binding_plan_release_check_is_optional_but_strict_when_present` 覆盖 unknown/missing plan check。已验证 `rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 optional VST3 SDK audit evidence hardening，不表示完整 SDK 3.8 Rust bindings 已生成。

本次 2026-06-13 收紧 VST3 SDK generated bindings surface symbol 集合:

- `validate_vst3_sdk_binding_surface_content()` 现在从 `REQUIRED_GENERATED_HEADER_INPUTS` 与 `generated_bindings_surface_symbol_names_for_header()` 派生完整 expected symbol name 集合，并要求 report 中的 `symbols` 名称精确匹配。手工添加 `ManualExtraSymbol` 会以 `surface contains unknown symbol(s)` 失败，删除 `IMidiMapping` 等 expected symbol 会以 `surface missing expected symbol(s)` 失败。
- 现有 kind/header/purpose/header_present/symbol_present、missing_symbols、required_headers、notes 和 `bindings_generated = false` 校验继续保留；这次只把 symbol name surface 从“少量核心 required symbol 存在”升级为“完整生产 surface 不多不少”。
- 扩展 `vst3_sdk_binding_surface_release_check_is_optional_but_strict_when_present` 覆盖 unknown/missing surface symbol。已验证 `rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 optional VST3 SDK token-surface audit hardening，不解析 C++ AST、不验证 ABI，也不表示完整 SDK 3.8 Rust bindings 已生成。

本次 2026-06-13 收紧 VST3 SDK binding surface symbol metadata 精确匹配:

- `vesty-vst3-sys` 新增 public `GeneratedBindingsSurfaceSymbolSpec` 与 `generated_bindings_surface_symbol_specs()`，把内部 `GENERATED_BINDINGS_SURFACE_SYMBOLS` 的 name/kind/header/purpose 以只读 metadata 暴露给 CLI 审计使用；这不是 callable bindings API，也不生成 VST3 COM glue。
- `validate_vst3_sdk_binding_surface_content()` 现在除 symbol name set 外，还要求每个已知 symbol 的 `kind`、`header` 和 `purpose` 精确匹配生产 spec。篡改 `IPlugView` 的 kind/header/purpose 会分别报出 expected metadata，而不再只要 name 存在就通过。
- 扩展 `vst3_sdk_binding_surface_release_check_is_optional_but_strict_when_present` 覆盖 wrong metadata。已验证 `rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 343 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`、`rtk cargo clippy -p vesty-vst3-sys --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 optional token-surface audit metadata hardening，不解析 C++ AST、不验证 ABI，也不表示完整 SDK 3.8 Rust bindings 已生成。

本次 2026-06-13 收紧 crate package readiness 与 publish plan 绑定:

- `CratePackageReport` 现在携带同一次生成时的 embedded `publish_plan`。`crate_package_report()` 从 workspace Cargo metadata 生成 publish plan 后，会把完整 publishable crate 顺序、level、version、manifest path、internal dependencies 和 skipped private list 一起写入 crate-package readiness report。
- `validate_crate_package_report()` 现在会先验证 embedded publish plan，再要求 `packages` entries 与 embedded publish plan 逐项一致: entry 数量、name、version、manifest path、publishOrder 和 internal dependencies 都不能漂移。手工报告新增未知 package、漏掉 package、或把 `manifest_path` / version / deps 改成与 publish plan 不一致都会失败。
- `crate_package_release_check()` 在同时收到 `publish-plan` evidence 和 `crate-package` evidence 时，会要求 crate-package embedded publish plan 与外部 publish-plan report 逐项一致，避免两个 release artifacts 来自不同 workspace、不同提交或手工拼接。`vesty crate-package --check --out <report>` 仍保持离线 artifact 校验语义，不需要重新读取当前 workspace。
- 新增回归测试覆盖 publish-plan evidence version mismatch、embedded publish plan 缺项、未知 extra package 和 manifest path drift。已验证 `rtk cargo test -p vesty-cli crate_package -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 345 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 crate package release evidence hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 npm pack report 未知字段拒绝:

- `NpmPackEntry` 和 `NpmPackFile` 现在使用 `serde(deny_unknown_fields)`。下载或手写的 npm pack report 如果在 package entry 中携带未审计字段如 `scripts`，或在 file entry 中携带 `mode` 等额外字段，会在 JSON parse 阶段失败。
- 这保留了现有 package set、files whitelist、重复路径、数量上限和文本安全校验，同时避免 release-check/import-ci/check-mode 忽略未知 JSON 字段后把扩展过的 artifact 当作标准 npm dry-run evidence。
- 新增 `npm_pack_report_rejects_unknown_json_fields` 覆盖 package-level 和 file-level unknown field。已验证 `rtk cargo test -p vesty-cli npm_pack -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 346 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 npm release evidence schema hardening，不生成真实 npm publish 或外部 release evidence。

本次 2026-06-13 收紧 publish-plan / crate-package report 未知字段拒绝:

- `PublishPlan`、`PublishPlanPackage`、`CratePackageReport` 和 `CratePackageEntry` 现在都使用 `serde(deny_unknown_fields)`。我们自有 release evidence 格式不再静默忽略 `generated_by`、`checksum`、`generatedBy` 等顶层或条目级额外字段。
- 这让 `publish-plan --check`、`crate-package --check`、`release-check` 和 `import-ci` 在 JSON parse 阶段拒绝未审计扩展字段，而不是只验证已知字段后继续通过。
- 新增 `publish_plan_report_rejects_unknown_json_fields` 和 `crate_package_report_rejects_unknown_json_fields` 覆盖顶层与 package entry 两类 unknown field。已验证 `rtk cargo test -p vesty-cli publish_plan -- --nocapture`、`rtk cargo test -p vesty-cli crate_package -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 348 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 publish/crate release evidence schema hardening，不生成真实外部 release evidence。

本次 2026-06-13 收紧 release-check / release-action-plan JSON 未知字段拒绝:

- `ReleaseCheckReport`、`ReleaseCheckItem`、`ReleaseActionPlan`、`ReleaseActionPlanSummary` 和 `ReleaseActionItem` 现在使用 `serde(deny_unknown_fields)`。CI release-check snapshots 和 action-plan sidecars 作为 Vesty 自有 JSON artifact，不再静默忽略 `generatedBy`、`owner`、`pending` 等未审计字段。
- DAW matrix rows 仍保持 `serde_json::Value`，但由 DAW matrix shape/canonical host-set validator 固定允许字段、字段类型、host set、平台支持状态和 summary consistency。未知字段不会作为公共扩展面被静默接受。
- 新增 `release_check_report_rejects_unknown_json_fields` 和 `release_action_plan_sidecar_rejects_unknown_json_fields` 覆盖 report/action-plan 顶层、summary 和 item/action unknown field。已验证 `rtk cargo test -p vesty-cli release_action_plan -- --nocapture`、`rtk cargo test -p vesty-cli release_check -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 350 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 release-check/action-plan artifact schema hardening，不生成真实 CI artifact 或其它外部 release evidence。

本次 2026-06-13 收紧 release-check DAW matrix row 未知字段与字段类型:

- `validate_release_check_daw_matrix_shape()` 现在对每个 `daw_matrix` row 执行固定字段 allowlist/required-set，只允许且要求 `host`、`platform`、`platform_supported`、九个 smoke check 布尔字段和 `evidence`。`generatedBy` 等额外字段会失败，缺少 `meter_stream` 这类必需字段也会在 release-check report shape validation 阶段失败。
- `platform` 和 `evidence` 现在必须是字符串；`platform_supported` 与九个 smoke check 仍必须是 boolean。手工或 CI artifact 把这些 metadata 字段改成 bool/object/array 不会再被当作“缺失但可推导”的宽松行处理。
- 这补上了 `ReleaseCheckReport` 顶层 `deny_unknown_fields` 之外的 nested `serde_json::Value` 盲区，同时保留现有 canonical host-set、host/platform consistency、summary drift 和 per-host smoke drift 校验。
- 新增 `release_check_report_shape_rejects_unknown_daw_matrix_fields`、`release_check_report_shape_requires_complete_daw_matrix_fields` 和 `release_check_report_shape_rejects_non_string_daw_matrix_metadata`。已验证 `rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture` 当前 10 passed 和 `rtk cargo fmt --all --check`。本次仍只是本地 release-check report schema hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 CI release-check artifact OS metadata:

- `ReleaseCheckReport` 新增 optional `os` metadata。当前 CLI 生成 release-check report 时会按运行平台写入 `Linux` / `macOS` / `Windows`；测试 fixture 和旧 artifact 可继续省略该字段。
- `validate_release_check_report_shape()` 会校验 `os` 文本安全和平台 label 合法性；`ci_release_check_artifacts_release_check()` 在 artifact path 能推断 OS 且 report 带 `os` 时，会要求两者一致。`release-check-Linux.json` 里塞 `os = "Windows"` 现在会失败，而 legacy 无 `os` label 的 artifact 仍按路径 OS 推断通过。
- 新增 `ci_release_check_artifacts_reject_os_label_mismatch_when_present`、`ci_release_check_artifacts_allow_legacy_reports_without_os_label` 和 `release_check_report_shape_validates_optional_os_label`。已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture` 当前 15 passed、`rtk cargo test -p vesty-cli release_check_report_shape_validates_optional_os_label -- --nocapture`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 361 passed、`rtk cargo test --workspace -j1` 当前 706 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk npm test` 和 `rtk cargo fmt --all --check`。本次仍只是 CI release-check artifact metadata hardening，不生成真实 GitHub Actions artifact。

本次 2026-06-13 收紧 release evidence audit report 未知字段拒绝:

- `LocalReleaseEvidenceReport`、`LocalReleaseEvidenceItem`、`ImportCiReleaseEvidenceReport`、`ImportCiReleaseEvidenceItem` 和 `CollectedReleaseEvidenceReport` 现在使用 `serde(deny_unknown_fields)`。`collect-local-report.json`、`import-ci-report.json`、`collect-signing` / `collect-notarization` JSON report 不再静默忽略 `generatedBy`、`owner` 等未审计字段。
- 这些 report 仍只是 provenance/audit metadata，不是 pass evidence；现有 path containment、固定槽位、source/output 关系、template item、status 和 collected output 语义校验继续保留。
- 扩展 `release_evidence_audit_reports_reject_malformed_shape_fields` 覆盖 local/import/collected 三类顶层与 item unknown field。已验证 `rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 350 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 release evidence audit-report schema hardening，不生成真实 CI、DAW、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 validate report 未知字段与递归发现分类:

- `ValidateReport`、`StaticBundleCheck`、`ValidatorCheck` 和 `vesty-build::BinaryExportCheck` 现在使用 `serde(deny_unknown_fields)`。`vesty validate --report` 产物及 release-check/import-ci 读取的 validator/static JSON 不再静默忽略顶层、static check、validator check 或 `static_check.binary_exports[*]` 中的未审计字段，例如 `generatedBy`、`owner`、`pending` 或 `checksum`。
- `release-check --release-evidence-dir` 的递归 validate/static report fallback 现在会先排除可识别的非 validate Vesty artifact；同时增加最小 release-check/action-plan sidecar 形状识别，避免 `static-validate-release-check.json` 这类 release-check 旁路报告因文件名含 `static-validate` 被误归类为 static validate evidence。损坏但路径明确的 `validator/*.validate.json` / `package/*.static-validate.json` 仍会保留下来作为诊断 evidence，并在后续 gate 中失败。
- 已验证 `rtk cargo test -p vesty-cli release_evidence_dir_discovers_validate_reports_by_content -- --nocapture`、`rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_recursive_validate_reports_for_diagnostics -- --nocapture`、`rtk cargo test -p vesty-cli validate_report -- --nocapture`、`rtk cargo test -p vesty-build -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 350 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`、`rtk cargo clippy -p vesty-build --all-targets -- -D warnings` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 validate/static release evidence schema 与 discovery hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 VST3 SDK audit JSON 未知字段拒绝:

- `vesty-vst3-sys` 的 `SdkHeaderInputManifest`、`SdkHeaderInput`、`GeneratedBindingsPlan`、`GeneratedBindingsPlanCheck`、`GeneratedBindingsSurface` 和 `GeneratedBindingsSurfaceSymbol` 现在使用 `serde(rename_all = "camelCase", deny_unknown_fields)`。optional SDK audit evidence 不再静默忽略 manifest/header、binding-plan/check 或 binding-surface/symbol 中的未审计字段，例如 `generatedBy`、`checksum` 或 `owner`。
- 这只影响 JSON audit artifact 解析和 release-check/import-ci 校验；`generated.rs`、`generated-abi-seed.rs`、`generated-abi.rs` 和 `generated-interface-skeleton.rs` 仍是 metadata/scaffold 文本审计文件，不表示完整 SDK 3.8 bindings 已生成。
- 扩展 `vst3_sdk_json_artifacts_reject_malformed_shape_fields` 覆盖 top-level 与 nested unknown field。已验证 `rtk cargo test -p vesty-cli vst3_sdk_json_artifacts_reject_malformed_shape_fields -- --nocapture`、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture`、`rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture`、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 350 passed、`rtk cargo test --workspace -j1` 当前 684 passed、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`、`rtk cargo clippy -p vesty-vst3-sys --all-targets -- -D warnings`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 optional VST3 SDK audit evidence schema hardening，不生成真实 SDK bindings 或外部 release evidence。

本次 2026-06-13 收紧 bundle metadata JSON 未知字段拒绝:

- `vesty-build` 的 `AssetManifest`、`AssetFile`、`ModuleInfo`、`ModuleClassInfo`、`ParameterManifest` 和 `ParameterManifestEntry` 现在使用 `serde(deny_unknown_fields)`。打包写入并由 `vesty validate` 读取的 `assets.manifest.json`、`moduleinfo.json` 和 `parameters.manifest.json` 不再静默忽略顶层或 entry/class 中的未审计字段，例如 `generatedBy`、`mode`、`owner` 或 `checksum`。
- 这保留现有 manifest 内容校验: asset path/mime/sha256/size、moduleinfo name/vendor/version/class id/category、parameter stable VST3 ParamID 与 `ParamSpec` schema 仍照常验证；未知字段会更早在 JSON 解析阶段失败。
- 新增 `asset_manifest_rejects_unknown_json_fields`、`moduleinfo_rejects_unknown_json_fields` 和 `parameter_manifest_rejects_unknown_json_fields`。已验证 `rtk cargo test -p vesty-build unknown_json_fields -- --nocapture`、`rtk cargo test -p vesty-build -- --nocapture` 当前 80 passed、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 350 passed、`rtk cargo test --workspace -j1` 当前 687 passed、`rtk cargo clippy -p vesty-build --all-targets -- -D warnings`、`rtk cargo clippy -p vesty-cli --all-targets -- -D warnings`、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk npm test` 和 `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`。本次仍只是 bundle metadata schema hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 JSBridge 内建 request payload 与 wry runtime manifest:

- `vesty-ipc::BridgePacket` 顶层仍保持 forward-compatible，不使用 `deny_unknown_fields`；新增测试明确允许未知顶层字段，避免后续协议扩展被当前 runtime 一刀切拒绝。`BridgeHelloPayload` 也保持公共握手扩展语义: Rust serde、JSON Schema 和 `vesty-bridge` runtime 都允许 hello payload 携带未来 JS capability 字段，只继续校验 supported protocol versions、JS package version 和 page URL 的已知字段。
- `@vesty/plugin-ui` 的 ready payload 校验也明确保留公共协议扩展能力: ready response 顶层、capabilities、snapshot、param flags/kind 和 MIDI mapping 中的未知字段不会阻断握手，且成功后的 cached ready payload 会保留这些字段。新增 JS 回归测试覆盖扩展字段完成 `bridge.hello` -> `bridge.readyAck` 流程后可被 UI 读取，避免后续把公共 ready payload 误收紧成内部 manifest 语义。另一个 JS 回归测试覆盖 `bridge.hello.response.payload` accessor/getter 会在入站 packet guard 阶段 fail-closed 丢弃，不执行 getter、不发送 readyAck、不 settle ready promise，后续同一 pending request 收到合法 response 仍可恢复握手。Rust IPC 侧新增 `BridgeReadyPayload` 反序列化未知字段回归，并在 protocol export 测试中固定 ready payload schema 顶层不声明 `additionalProperties: false`。
- `vesty-bridge` 对固定内建 request payload 增加 allowlist 校验: `bridge.readyAck`、`subscription.add/remove`、`state.setConfig`、`state.setUiState`、`param.begin/perform/end`、`param.format` 和 `param.parse` 出现拼写错误或额外字段时返回 non-retryable `validation_error`，不会污染 subscription table、pending gesture queue 或 state snapshot。`snapshot.get`、`diagnostics.get`、`meter.flush` 和 `event.flush` 也纳入固定 payload 规则，只接受无 payload、`null` payload 或空 object；`payload: null` 在当前公共 envelope 的 `Option<Value>` serde 形态下等价于省略 payload。`@vesty/plugin-ui` 和 wry bootstrap 的 query/flush helper 统一发送 `{}`；非空 object 或字符串、数组、布尔等非 object payload 会返回 validation error。该校验只作用于 Vesty 内建命令 payload，不改变 generic command payload 的 JSON 透传能力。
- `vesty-ui-wry` 的 release runtime asset manifest mirror 现在与 `vesty-build::AssetManifest` shape 对齐，读取 `version/root/entry/files` 并使用 `serde(deny_unknown_fields)`；manifest 版本必须为 1，root 不能为空且不能含控制字符，顶层或 nested asset file 的未知字段会在 WebView custom protocol attach 前以 invalid data 失败。
- `vesty-build` static bundle validation 也开始校验 `assets.manifest.json` 的 version/root: `version` 必须为 1，`root` 必须非空且不能含控制字符。该字段仍作为 build-time provenance，不要求等于运行时 bundle 路径，因为 `.vst3` bundle 可以被移动。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-ipc -- --nocapture` 当前 14 passed、`rtk cargo test -p vesty-bridge -- --nocapture` 当前 73 passed、`rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture` 当前 20 passed、`rtk cargo test -p vesty-build -- --nocapture` 当前 80 passed、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk npm test`、`rtk cargo test --workspace -j1` 当前 690 passed 和 `rtk cargo clippy --workspace --all-targets -- -D warnings`；本次补充 ready/hello payload 扩展字段、accessor/getter payload、Rust IPC/schema/runtime 回归和 fixed empty-payload query/flush guards 后再次通过 `rtk npm test`、`rtk cargo test -p vesty-ipc -- --nocapture` 当前 16 passed、`rtk cargo test -p vesty-bridge -- --nocapture` 当前 75 passed、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1` 当前 694 passed 和 `rtk cargo clippy --workspace --all-targets -- -D warnings`。后续又补充 `snapshot.get` / `diagnostics.get` SDK helper 显式 `{}` payload 断言，以及 query/flush 对字符串、数组、布尔等非 object payload 的 Rust 回归测试；已验证 `rtk cargo test -p vesty-bridge builtin_request_payloads -- --nocapture`、`rtk npm --workspace @vesty/plugin-ui test`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk cargo fmt --all --check`。本次仍只是 local IPC/WebView/bundle metadata boundary hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧 param-manifest JSON report 未知字段拒绝:

- `vesty-cli` 的 `ParamManifestReport` 现在使用 `serde(deny_unknown_fields)`。`vesty param-manifest --format json` 输出属于 Vesty 自有 CLI JSON report，后续脚本或手工检查如果反序列化该 report，不应静默忽略 `generatedBy` 这类未审计字段。
- 新增 `param_manifest_report_rejects_unknown_json_fields` 覆盖顶层 unknown field 拒绝。已验证 `rtk cargo test -p vesty-cli param_manifest -- --nocapture` 当前 4 passed 和 `rtk cargo fmt --all --check`。本次仍只是 CLI report schema hardening，不生成真实 DAW、CI、platform smoke、validator、签名或 notarization evidence。

本次 2026-06-13 收紧泛用签名 marker 拒绝:

- `signing_evidence_platforms_from_text()` 现在不再把 `signed=true`、`signing=pass` 或 `signature=ok` 这类无法归属平台的泛用 marker 当作正向签名证据。签名 release evidence 必须能证明 macOS `codesign` 或 Windows `signtool` / verify summary；optional signing gate 也不会因为泛用 marker 返回 ok。
- `release-check --write-evidence-template` README 已同步说明泛用 signing/signature marker 会被拒绝；accepted signing marker 示例只保留 `codesign=pass`、`signtool=pass` 和 signtool `Number of errors: 0` 等平台可归属证据。
- `import-ci` 现在会把文件名明显像 signing evidence、但内容只有 `signed=true` 的日志记录为 failed，不会导入到 `signing-macos.log`、`signing-windows.log` 或 `signing/*.log`；普通 notes 中的 `signature=ok` 仍会作为 unrecognized text artifact 跳过，不会变成 pass evidence。
- 新增 `signing_evidence_rejects_generic_platformless_markers` 和 `import_ci_rejects_generic_platformless_signing_markers`。已验证 `rtk cargo test -p vesty-cli signing_evidence_rejects_generic_platformless_markers -- --nocapture`、`rtk cargo test -p vesty-cli import_ci_rejects_generic_platformless_signing_markers -- --nocapture`、`rtk cargo test -p vesty-cli signing -- --nocapture` 当前 37 passed、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 当前 20 passed、`rtk cargo test -p vesty-cli release_evidence -- --nocapture` 当前 33 passed、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 363 passed、`rtk cargo test --workspace -j1` 当前 708 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk npm test`。本次仍只是本地 signing release evidence parser/import hardening，不生成真实 macOS codesign 或 Windows signtool evidence。

本次 2026-06-13 收紧泛用 notarization accepted marker 拒绝:

- `notarization_evidence_from_text()` 现在不再把 `notarization=pass` 或 `notary=ok` 当作 accepted notarytool 证据。accepted 半边必须来自 `notarytool=pass`、`status: Accepted` 或 notarytool JSON accepted status；stapler 成功仍只证明 stapled 半边。
- `status: Accepted` 必须作为精确 `status` marker 或 notarytool JSON 字段出现，`The staple and validate action worked!` 必须作为独立 stapler success 行出现；普通说明文字里的 `note: paste status: accepted ...` / `note: paste The staple and validate action worked! ...` 不再因为 substring fallback 被误判为 accepted/stapled evidence。
- `release-check --write-evidence-template` README 已同步说明泛用 notarization/notary marker 会被拒绝；accepted notarization marker 示例保留 `notarytool=pass`、`status: Accepted` 和 stapler validate output。
- `import-ci` 现在会把只有泛用 accepted marker + stapler success 的 notary 日志记录为 failed，不会复制成 `notary.log`；strict release gate 也会报告缺少 `accepted notarytool result`。
- 新增 `notarization_evidence_rejects_generic_acceptance_markers` 和 `import_ci_rejects_generic_notarization_acceptance_markers`。已验证 `rtk cargo test -p vesty-cli notarization_evidence_rejects_generic_acceptance_markers -- --nocapture`、`rtk cargo test -p vesty-cli notarization_evidence_accepts_notarytool_json_status -- --nocapture`、`rtk cargo test -p vesty-cli strict_notarization_requires_accepted_and_stapled_evidence -- --nocapture`、`rtk cargo test -p vesty-cli notarization_evidence -- --nocapture` 当前 7 passed、`rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture` 当前 4 passed、`rtk cargo test -p vesty-cli import_ci_rejects_generic_notarization_acceptance_markers -- --nocapture`、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 当前 21 passed、`rtk cargo test -p vesty-cli release_evidence -- --nocapture` 当前 33 passed、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 365 passed、`rtk cargo test --workspace -j1` 当前 710 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk npm test`。本次仍只是本地 notarization release evidence parser/import hardening，不生成真实 notarytool 或 stapler evidence。

本次 2026-06-13 修复 collect-notarization 组合日志中的 notarytool JSON section 解析:

- `collect-notarization` 会把 notarytool 输出与 stapler 输出合并成 `notary.log`，形如 `notary_log=...`、`[notarytool]`、notarytool JSON、`stapler_log=...`、`[stapler]`。此前收紧 JSON accepted status 解析后，`[notarytool]` section 会把后续 `stapler_log=...` 元数据行也吞进去，导致原本合法的 `{ "status": "Accepted" }` 无法按 JSON 解析。
- `bracketed_log_section()` 现在在已进入目标 section 后，遇到下一段 bracket header 或 notarization collect metadata line 时结束当前 section。这样 `[notarytool]` 下的 JSON 不会被 `stapler_log=...` 污染，同时仍保持 prose/substring 拒绝语义，不重新接受 `note: paste {"status":"Accepted"} ...` 一类说明文字。
- 扩展 `notarization_evidence_accepts_notarytool_json_status` 覆盖真实 collect 组合日志格式: `[notarytool]` JSON accepted status + `stapler_log=...` + `[stapler]` exact success line 会通过；普通 prose 中的 inline JSON 仍失败。
- 已验证 `rtk cargo test -p vesty-cli collect_notarization -- --nocapture` 当前 2 passed、`rtk cargo test -p vesty-cli release_evidence -- --nocapture` 当前 33 passed、`rtk cargo test -p vesty-cli notarization_evidence -- --nocapture` 当前 7 passed、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 365 passed、`rtk cargo test --workspace -j1` 当前 710 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk npm test`。本次仍只是本地 collect-notarization parser regression fix，不生成真实 Apple notarytool 或 stapler evidence。

本次 2026-06-13 补强 VST3 SDK interface skeleton global vtable slot seed:

- `generated-interface-skeleton.rs` 的 `InterfaceVTableSlot` 现在除 `local_slot` 外还输出 `global_slot`。`FUnknown` 自身的 global slot 等于 local slot；其它接口的 global slot 从 `FUNKNOWN_VTABLE_ENTRIES` 三个基础 COM 方法之后开始，例如 `IAudioProcessor::process` 为 local slot 6 / global slot 9，`IUnitInfo::getProgramListInfo` 为 local slot 3 / global slot 6。
- skeleton 现在输出 `INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE = "com-vtable-global-slot-seed-audit"`，把未来 generated-headers emitter 需要的 COM absolute vtable slot 顺序纳入可审计 surface。生成的 helper test 会编译并执行 lookup，验证 `INTERFACE_VTABLE_SLOTS` 中 `IAudioProcessor::process` 的 global slot，以及 `FUNKNOWN_VTABLE_SLOTS` 中 `release` 的 global slot。
- `vesty-cli` 的 interface skeleton validator、`emit-interface-skeleton --check`、release-check/import-ci 的 SDK skeleton artifact 校验都已同步要求 `global_slot` 字段、global slot scope marker 和关键方法的 global slot metadata。旧的只含 local slot 的 skeleton artifact 会被拒绝。
- 已验证 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-vst3-sys generated_bindings_interface_skeleton -- --nocapture` 当前 2 passed、`rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture` 当前 13 passed、`rtk cargo test -p vesty-cli vst3_sdk_emit_interface_skeleton -- --nocapture` 当前 2 passed、`rtk cargo test -p vesty-cli vst3_sdk_interface_skeleton_validator -- --nocapture` 当前 1 passed、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture` 当前 27 passed、`rtk cargo test -p vesty-cli release_evidence -- --nocapture` 当前 33 passed、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 当前 21 passed、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 365 passed、`rtk cargo test --workspace -j1` 当前 710 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk npm test`。本次仍只是 generated SDK interface skeleton audit/emitter-prep hardening，不生成 callable Steinberg COM method implementations，也不表示完整 SDK 3.8 bindings 已生成。

本次 2026-06-13 补强 VST3 SDK interface skeleton vtable slot lookup seed:

- `generated-interface-skeleton.rs` 现在输出 `INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE = "pure-vtable-slot-lookup-seed-audit"`，并生成两个 pure lookup helper: `interface_vtable_slot_by_interface_and_method(interface, method)` 与 `interface_vtable_slot_by_interface_and_global_slot(interface, global_slot)`。
- 这些 helper 只读取 `INTERFACE_VTABLE_SLOTS` 审计数组，不生成 callable COM glue；它们把未来 generated-headers emitter 的 vtable dispatch metadata 消费路径提前固定下来。helper test 会编译并运行，验证 `IAudioProcessor::process` 可按 method 查到 global slot 9、可按 global slot 9 查到 method `process`，不存在的 global slot 返回 `None`。
- `vesty-cli` 的 interface skeleton validator、`emit-interface-skeleton --check`、release-check/import-ci 的 SDK skeleton artifact 校验都已同步要求 lookup scope marker 和两个 helper 存在。旧的只含 vtable slot arrays、但没有 pure lookup seed 的 skeleton artifact 会被拒绝。
- 已验证 `rtk cargo test -p vesty-vst3-sys generated_bindings_interface_skeleton -- --nocapture` 当前 2 passed、`rtk cargo test -p vesty-cli vst3_sdk_emit_interface_skeleton -- --nocapture` 当前 2 passed、`rtk cargo test -p vesty-cli vst3_sdk_interface_skeleton_validator -- --nocapture` 当前 1 passed、`rtk cargo test -p vesty-cli import_ci -- --nocapture` 当前 21 passed、`rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture` 当前 13 passed、`rtk cargo test -p vesty-cli vst3_sdk -- --nocapture` 当前 27 passed、`rtk cargo test -p vesty-cli release_evidence -- --nocapture` 当前 33 passed、`rtk cargo fmt --all --check`、完整 `rtk cargo test -p vesty-cli -- --nocapture` 当前 365 passed、`rtk cargo test --workspace -j1` 当前 710 passed、`rtk cargo clippy --workspace --all-targets -- -D warnings`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check` 和 `rtk npm test`。本次仍只是 generated SDK interface skeleton audit/emitter-prep hardening，不生成 callable Steinberg COM method implementations，也不表示完整 SDK 3.8 bindings 已生成。

## 仍未完成/不可本机证明

- 未完成 Cubase/Nuendo、Bitwig、Ableton Live、Studio One 的真实 DAW matrix；目前只有 REAPER 7.73/macOS-arm64 完成本机 scan/load/UI/UI->Host/meter stream/save-restore/automation/offline render smoke，且 2026-06-10 新增的 buffer/sample-rate change gate 仍需对 REAPER 和其它 DAW 补采。
- 已有本机 macOS 三个示例的 Steinberg validator passed reports；protocol snapshot、crate publish plan、crate package readiness、npm pack dry-run evidence、dependency latest baseline 和 headless `vesty smoke-host` 诊断 report 也可由本机生成并通过对应本地 gate。尚未有真实 GitHub Actions run URL、CI doctor/release-check artifact 下载目录、macOS/Windows/Linux X11 platform smoke reports、三示例/三平台 CI static validate 下载证据、三示例/三平台 validator passed reports、签名 verification log 或 notarization log 可供 `vesty release-check --require-release-artifacts` 通过；evidence gate 解析和模板已实现。
- VST3 meter/analyzer RT SPSC 到 controller/bridge/UI 的框架链路已接通，并已在 REAPER/macOS-arm64 中采到真实 60 Hz meter stream smoke；其它 DAW 尚未采集该项。
- Windows/WebView2 和 Linux/WebKitGTK/X11 的 bundle 结构已有单元测试约束，但尚未在真实 Windows/Linux host 上跑 validator、WebView attach 和 DAW smoke。
- 固定 latency/tail report 和 VST3 `kLatencyChanged` restart notification 已实现并有 fake COM 测试；运行中 latency 变化在 Cubase/Nuendo、Bitwig、Ableton Live、Studio One、Windows 和 Linux host 中仍需真实 smoke 验证。
- Effect optional sidechain MVP 已有本机 fake COM 覆盖，包括 sidechain 已提供、main-only input 和 empty inactive sidechain input；真实 DAW 中的 sidechain bus activation、routing、automation/save-restore 和 offline render 行为仍需采集 smoke evidence。
- Program list metadata、program-change 参数、controller-side program data envelope 和 `midi-synth` program/preset 示例已本地实现；真实 DAW program selection、program data roundtrip、save/restore 和 automation interaction 仍需采集 smoke evidence。
- Linux Wayland 仍应标记 experimental；首版承诺 X11。
- crash protection 仍是边界保护和 faulted silence，不是进程隔离；native crash 仍可能影响 DAW。
- VST3 `Sample64` 默认路径仍是 adapter-level f64<->f32 scratch bridge；原生 double-precision developer DSP API 已通过 `AudioKernel::SUPPORTS_F64` / `process_f64()`、`ProcessContext64` 和 `AudioBuffers64` opt-in 实现并有本机 fake COM 测试覆盖。仍需在真实 DAW/平台中补采 native f64 插件 smoke。
- VST3 SDK 3.8 的所有扩展 API 未逐项覆盖；当前以 `vst3 0.3.0` bindings 起步，`vesty-vst3-sys` 已预留 generated headers backend，并能生成/校验 SDK header input manifest、generated-bindings readiness plan、带 identifier-token 存在性检查的 generated-bindings symbol surface、metadata-only scaffold、ABI seed 与 foundational ABI layout，但尚未真正生成完整 header bindings。

## 下一步

- 补齐 Cubase/Nuendo、Bitwig、Ableton Live、Studio One 的真实 DAW matrix，记录 scan/load/UI/UI->Host/meter stream/automation/buffer-sample-rate change/save-restore/offline render。
- 在 Cubase/Nuendo、Bitwig、Ableton Live、Studio One 中补 UI 端 30/60 Hz meter stream smoke，并让 `vesty daw-matrix` 的对应 `Meter Stream` 列变为 pass。
- 将 realtime/crash-safety 测试继续扩展到更多 channel layout；`ParamAutomationSegments` 已加入 deterministic seeded fuzz 覆盖，VST3 offline render block mode 已暴露给 developer kernel。
- 增加 Windows/WebView2 和 Linux/WebKitGTK CI 或手工 smoke。
