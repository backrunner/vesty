# 10. Roadmap

## Phase 0: 技术验证

目标: 证明 Rust VST3 + wry child WebView 可以在真实 host 中跑通。

交付:

- 最小 VST3 factory。
- passthrough processor。
- gain 参数。
- macOS 或 Windows 单平台 WebView editor。
- dev URL 加载。
- panic guard 原型。

退出条件:

- 官方 validator 基础通过。
- 一个 DAW 能加载插件、打开 UI、调 gain。

## Phase 1: 核心 API

目标: 形成开发者可用的 Rust trait。

交付:

- `vesty-core`。
- `AudioKernel` / `Plugin` trait。
- `vesty-params`。
- `#[derive(Params)]` 原型。
- state v1。
- gain example。

退出条件:

- 不需要写 unsafe 即可实现 gain effect。
- process allocation guard 通过。

## Phase 2: VST3 完整 MVP

目标: 补齐 VST3 插件生命周期。

交付:

- processor/controller split。
- parameter automation。
- bus negotiation。
- bypass。
- state get/set。
- MIDI event input。
- simple synth example。
- validator 集成。

退出条件:

- effect 和 instrument examples 通过 validator。
- host 保存恢复工程可用。

## Phase 3: Web UI MVP

目标: Web UI 成为一等能力。

交付:

- `vesty-ui`。
- `vesty-ui-wry`。
- JS bridge。
- custom protocol assets。
- dev/release 模式。
- resize constraints。
- Web UI parameter demo。

退出条件:

- React/Vue/Svelte 至少各一个薄模板能构建。当前 `vesty new --ui react|vue|svelte` 已生成 Vite 薄模板，并在本机 smoke 中完成 `npm install` + `vite build`。
- UI 参数变更经 host 自动化路径进入 DSP。
- 本地 fake-host editor open/close/resize stress 通过；真实 DAW/WebView stress 进入 Phase 5/release evidence。

## Phase 4: CLI 和打包

目标: 开发者可从空项目生成 VST3。

交付:

- `vesty new`。
- `vesty dev`。
- `vesty build`。
- `vesty package`。
- `vesty validate`。
- bundle structure checker。
- macOS/Windows/Linux packaging。

退出条件:

- 一条命令生成 gain 项目。
- release bundle 可被 DAW 扫描。

## Phase 5: 稳定性与兼容性

目标: 面向 alpha 用户。

交付:

- host quirk table。
- non-blocking logger。
- UI fallback。
- crash/fault reporting。
- DAW smoke matrix。
- CI artifacts。

退出条件:

- 至少两个平台、三个 DAW smoke test 通过；每个 DAW smoke 需覆盖 scan/load/UI/UI->Host/meter stream/automation/buffer-sample-rate change/save-restore/offline render。
- 文档覆盖常见失败排查。

## Phase 6: Beta 能力

目标: 面向更广泛插件类型。

交付:

- sidechain MVP: one optional mono/stereo aux input bus for effects 已本地实现；本地 fake-host 覆盖 sidechain 已提供、main-only input 和 empty inactive sidechain input；仍需真实 DAW smoke。
- multi-output instrument MVP 已本地实现: `Plugin::output_buses()` 可声明最多 4 个 mono/stereo output bus，VST3 adapter 暴露 main + aux output buses，sample32/native sample64/scratch sample64 process path 会以零分配方式展平多 output bus；仍需真实 DAW 多输出 routing smoke。
- latency change notification。
- program list metadata + opt-in `apply_program()` + controller-side program data MVP 已本地实现；program-change 参数 metadata 已映射到 VST3 `kIsProgramChange`，并已支持 controller/control-thread `setParamNormalized()` / edit relay 选择可见 program；`examples/midi-synth` 已作为 concrete program/preset workflow 示例，覆盖 host-visible program 参数、program list metadata、program attributes/pitch names 和 per-program JSON data roundtrip；audio `process()` 内 program-change 参数 automation 已本地验证为 realtime-safe 普通参数事件与 atomic 参数更新，不会调用 program apply/data load；真实 host program workflow 仍待外部验证。
- SysEx data event translation MVP 已本地实现；固定 256-byte payload buffer，`examples/midi-synth` 已展示固定 SysEx level override 的 realtime-safe DSP 消费路径；真实 host SysEx workflow 仍待验证。
- Note Expression value/int/text event translation、opt-in controller value metadata 和 static physical UI mapping metadata MVP 已本地实现；`examples/midi-synth` 已展示 brightness/tuning expression metadata 与 DSP 消费路径；自定义 expression editor workflow 和 real-host expression workflow 仍待实现。
- TypeScript package `vesty-plugin-ui`。
- plugin template gallery 已本地实现: `vesty templates` 可列出内置 starter，`vesty new --template <id>` 可选择 gain、midi-synth、Web UI param demo 和 framework-specific starters；真实第三方模板生态仍是后续扩展。

退出条件:

- 五个主流 DAW 的回归记录。
- beta API freeze 文档。

## Phase 7: 1.0

目标: API 稳定和可维护发布。

交付:

- semver-stable public API。
- migration guide。
- 完整 book。
- signing/notarization guide。
- benchmark dashboard。
- examples release。

退出条件:

- 真实第三方插件完成迁移/发布。
- API freeze 至少一个 minor cycle 无破坏性变更。
