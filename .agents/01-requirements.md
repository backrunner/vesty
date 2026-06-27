# 01. 需求文档

## 愿景

Vesty 是一个面向 Rust 开发者的 VST3 插件框架。开发者只需要实现音频/MIDI 处理模块和可选的 Web UI，Vesty 负责 VST3 适配、参数自动化、状态保存、实时安全通信、系统 WebView 承载、资源打包和 DAW 兼容性基础设施。

## 目标用户

- 想用 Rust 写音频效果器或乐器的独立开发者。
- 已有 Web 前端能力，希望用 React/Vue/Svelte 构建插件 UI 的团队。
- 需要低延迟、低崩溃风险、可跨 DAW 分发 VST3 插件的音频工具开发者。

## MVP 范围

- 只输出 VST3 插件 bundle。
- 支持 macOS、Windows、Linux x86_64；macOS Apple Silicon 支持作为第一优先级。
- Rust DSP 核心通过安全 trait 实现。
- 支持 effect 和 instrument 两类插件。
- 支持 audio input/output、event input、参数自动化、host state get/set。
- 支持 Web UI assets 打包进 VST3 bundle，通过系统 WebView 显示。
- Web UI framework agnostic，Vesty 不绑定 React/Vue/Svelte。
- 提供 CLI 脚手架: new、dev、build、package、validate。
- 提供至少三个示例: gain effect、MIDI synth、Web UI parameter demo。

## 非目标

- 不支持 VST2。
- MVP 不支持 CLAP、AU、AAX。
- 不使用 Tauri 架构，不引入 Tauri runtime、窗口模型、命令系统和 updater。
- 不承诺崩溃完全隔离 DAW 进程，因为插件仍加载在 host 进程内。
- 不在音频线程里执行 JS、WebView IPC、网络、文件、动态分配或阻塞同步。
- 不提供 UI 组件库，只提供 JS bridge、typing、适配器和示例。

## 功能需求

### 插件核心

- 开发者实现 `AudioKernel` 或 `InstrumentKernel`。
- 框架提供 `prepare`、`reset`、`process`、`suspend/resume` 生命周期。
- 处理输入/输出 buffer、sample rate、block size、transport、事件和参数变化。
- 支持 mono/stereo 基础 bus；effect 可声明一个 optional mono/stereo sidechain input bus；instrument 可通过 `Plugin::output_buses()` 声明多个 mono/stereo output bus；surround/immersive layout 放入后续扩展。
- 支持 bypass 参数约定。
- 支持 latency 报告和变更通知。

### 参数系统

- 参数有稳定 ID、名称、默认值、范围、单位、step count、automatable/read-only/bypass/program-change 等 flags。
- 内部使用类型化参数，VST3 暴露 normalized value。
- 支持 smoothing，避免自动化 zipper noise。
- UI 修改参数必须发送 beginEdit/performEdit/endEdit 语义。
- host 自动化回放进入 processor 时要 sample accurate。
- 可暴露静态 program list metadata 供 VST3 host 查询，并允许插件用 opt-in `apply_program()` 在 controller 非实时路径响应 host program selection；参数可标记为 VST3 `kIsProgramChange` metadata，并可在 controller/control-thread `setParamNormalized()` / edit relay 中按可见 program index 触发 opt-in program apply。audio `process()` 内 program-change 参数 automation 按普通 sample-accurate 参数事件与 atomic 参数更新处理，不会在实时线程套用 program/state。框架和 `examples/midi-synth` 已提供本地 program data roundtrip 示例；真实 DAW workflow evidence 属于后续外部验收。

### MIDI/Event

- 支持 NoteOn、NoteOff、PolyPressure、PitchBend、ChannelPressure、CC mapping、SysEx data event 和 Note Expression value/int/text event 基础能力；`examples/midi-synth` 已本地展示固定 SysEx payload 与 Note Expression brightness/tuning 在 realtime-safe DSP 中消费。
- VST3 内部以 event bus 和参数映射表达 MIDI 概念。
- instrument 插件可以根据 event 产生音频输出。
- 可暴露静态 Note Expression value/physical UI mapping metadata；真实 host SysEx/expression workflow 放入后续外部验收。
- MIDI 2.0 放入后续阶段。

### Web UI

- UI 由 wry 创建为 host editor parent 的 child view。
- 支持 dev 模式加载 `http://localhost:*`。
- 支持 release 模式加载 bundle 内 assets。
- 提供 `window.__VESTY__` JS bridge。
- 支持 UI 请求参数快照、设置参数、订阅参数变化、订阅 meter/analyzer 帧。
- 支持 resize，遵守 VST3 IPlugView/IPlugFrame 尺寸协商。
- 支持 UI crash/reload fallback。

### 构建打包

- `vesty new` 生成 Rust crate + UI 目录 + `vesty.toml`。
- `vesty build` 构建 Rust cdylib 和 UI assets。
- `vesty package` 生成平台 VST3 bundle。
- `vesty validate` 调用 Steinberg validator，并可扩展调用 pluginval。
- 生成 `moduleinfo.json`、Info.plist、资源 manifest、快照占位。

### 安全与稳定性

- exported ABI 和 host callback 边界 catch panic。
- 音频线程 panic 后进入 faulted 模式，输出静音并标记实例不可继续处理。
- UI thread panic 不应影响 audio thread。
- 日志走非阻塞 ring buffer，音频线程满则丢弃。
- 资源协议限制路径穿越、MIME、外链导航、下载和窗口打开。

## 性能需求

- process callback 内零 heap allocation。
- process callback 内零 blocking wait。
- 常规 gain 示例在 48 kHz、64 sample block 下额外 CPU 开销不可显著高于手写 VST3 wrapper。
- UI 参数刷新默认 30 Hz，可配置到 60 Hz；meter 数据满队列时丢帧。
- bundle asset 加载不影响 audio thread。

## 兼容性需求

- 通过 Steinberg VST3 validator。
- 在至少 5 个主流 VST3 host 做 smoke test。
- 支持 host 多次打开/关闭 editor。
- 支持 host 保存/恢复工程。
- 支持 offline render、不同 sample rate、不同 block size。
- host 不支持 WebView 或 WebView 初始化失败时，插件仍可无 UI 运行并暴露 host 参数。

## 验收标准

MVP 通过以下检查才可称为 alpha:

- `vesty new gain --ui react` 可以生成项目。
- 示例 gain 插件可在 macOS/Windows 至少一个 DAW 中加载、显示 Web UI、自动化参数、保存恢复状态。
- `vesty validate` 可运行官方 validator 并产出报告。
- 音频线程 allocation guard 测试通过。
- Web UI 连续打开/关闭 100 次无资源泄漏或崩溃。
- 参数从 host、UI、state restore 三个方向更新时无反馈循环。
