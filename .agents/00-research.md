# 00. 技术调研

调研日期: 2026-06-07

## 关键结论

Vesty 可以成立，但它不是一个普通 Rust GUI crate 的组合题。VST3 插件运行在 DAW 进程内，音频线程由 host 调用，UI 生命周期也由 host 控制。因此设计重点是隔离实时路径、控制 unsafe 边界、把 WebView 当成可选的非实时编辑器，而不是把插件做成一个小桌面应用。

## VST3 标准

VST3 官方 SDK 当前主线是 3.8.x，SDK README 显示包含 VST3 API、实现 helper、示例、Test Host/Validator 等工具，并且 SDK 采用 MIT license。VST3 的设计明确包括模块化、处理和 UI 分离、sample accurate automation、多动态 I/O、可缩放 UI、factory 可导出多个插件等能力。

VST3 的核心形态:

- 插件以 VST-MA/COM-like 组件模型暴露，host 通过 `GetPluginFactory` 创建组件。
- macOS 必须导出 `bundleEntry`/`bundleExit`，Windows 可选 `InitDll`/`ExitDll`，各平台再导出 `GetPluginFactory`。
- 一个插件一般拆成 processor 和 edit controller。processor 处理音频、事件、总线和状态；controller 发布参数、单位、UI 和 host 交互。
- 参数由 controller 发布，必须有稳定 host ID，自动化参数值以 `0.0..=1.0` normalized value 和 host 交互。Vesty 侧把字符串参数 ID 映射到 validator-safe 的正数 31-bit `ParamID`，避免部分 validator/host 把高位 `u32` ID 当作负数处理。
- VST3 不直接把 MIDI 当成 VST2 式 MIDI 流，MIDI 概念映射到 event bus、note events、parameter MIDI mapping、Note Expression、SysEx data event 等。
- 官方 bundle 结构跨平台不同，现代 Windows/Linux 也使用 `.vst3` 文件夹 bundle，而不是单 DLL 文件。

## Rust VST3 生态

可选技术路线:

1. 基于 `vst3` crate。
   - 优点: 直接提供从官方 C++ headers 生成的 Rust VST3 bindings、COM pointer 和 COM interface 实现辅助。
   - 缺点: 文档覆盖很低，crate 自己声明不抽象 VST3 API，unsafe 复杂度需要 Vesty 封装。

2. 基于 Steinberg `vst3_c_api`。
   - 优点: 官方 C API header，可用于生成更稳定的 C ABI 风格 bindings。
   - 缺点: 仓库最近 release 是 3.7.7，低于当前 3.8.x SDK；还要自己处理完整 COM/VST3 适配。

3. 参考 NIH-plug。
   - 优点: Rust 音频插件框架里参数系统、实时安全异步任务、bundler 和 examples 很有参考价值。
   - 缺点: NIH-plug 以 VST3 + CLAP 为目标，并且当前 README 说明框架处于 maintenance mode。Vesty 需要只聚焦 VST3 + WebView UI，不建议直接依赖其架构。

建议: 第一版用 `vst3` crate 做 binding 起点，同时保留 `vesty-vst3-sys` 生成层的退路。所有 unsafe/COM 代码封在 `vesty-vst3`，上层插件开发者只看到安全 Rust trait。

## wry/system WebView

wry 是跨平台 WebView rendering library，底层使用系统 WebView:

- Windows: WebView2。
- macOS: WebKit/WKWebView。
- Linux: WebKitGTK。

wry 支持:

- child webview: `WebViewBuilder::build_as_child` 可在 macOS、Windows、Linux X11 中把 WebView 创建为父窗口子视图/子窗口。
- custom protocol: `with_custom_protocol`/`with_asynchronous_custom_protocol` 可注册自定义 scheme，用于加载打包后的 UI assets。
- IPC: `with_ipc_handler` 接收 JS 的 `window.ipc.postMessage(...)`。
- Rust 调 JS: `WebView::evaluate_script` 和 callback 版本。

关键限制:

- `WebView` 和 `WebViewBuilder` 是 `!Send`/`!Sync`，必须由 UI 线程持有和操作。
- Linux 需要 GTK/WebKitGTK，并且事件循环要推进 GTK loop；child webview 在 X11 可用，Wayland 需要 GTK container 路线。
- Windows custom protocol URL 形态和 macOS/Linux 不同，资源协议层要统一抽象。
- wry 是 WebView 底层库，不提供 Tauri 的应用层能力。Vesty 需要自己定义 IPC、资源协议、生命周期、调试和安全策略。

## 实时音频与内存

音频线程必须避免:

- heap allocation 和释放。
- mutex、RwLock、condvar、blocking channel。
- 文件 I/O、网络 I/O、WebView 调用。
- `println!`、format 大字符串、同步日志。
- panic unwind 穿过 FFI/VST3 ABI 边界。

推荐原语:

- 参数值: `AtomicU32`/`AtomicF32` 风格原子镜像，音频线程按 block 或 sample 读取。
- UI/控制线程到音频线程: 预分配 SPSC ring buffer；满时返回 backpressure，不阻塞。
- 音频线程到 UI: SPSC ring buffer 或 triple buffer；满时丢弃 meter/analyzer 帧。
- 复杂状态: control thread 组装 immutable snapshot，音频线程在非处理状态或安全切换点换入。

`rtrb` 的 SPSC ring buffer 明确说明构造后不再分配，读写 lock-free/wait-free，并且满/空时立即返回错误，很适合作为 Vesty 的第一选择。`ringbuf` 提供 lock-free SPSC FIFO 和静态存储模式，也可作为替代或兼容层。`triple_buffer` 适合单生产者频繁更新、单消费者读取最新状态的场景，比如 UI meter snapshot。

## 防崩溃与 panic

Rust 官方 FFI 文档要求谨慎处理 unwinding。非 `-unwind` ABI 边界不应让 panic 穿过；如果要更温和处理 panic，需要在边界内 `catch_unwind`，但它只捕获 unwind panic，不能捕获 abort panic，也不应当当作普通错误处理机制。

对插件而言:

- release 插件不应配置 `panic = "abort"` 作为默认策略，因为 abort 会直接终止 DAW 进程。
- VST3 exported function 和 host callback 边界需要统一包 `catch_unwind`，把 panic 转为 VST3 `tresult`、静音输出、禁用实例或 UI 错误状态。
- 音频线程内仍应设计为“不 panic”，`catch_unwind` 是最后一道保险，不是实时错误处理手段。
- 真正的崩溃隔离只能通过 out-of-process 插件或辅助进程实现，但本项目明确不走 Tauri/多进程应用架构，因此 MVP 只能降低崩溃概率和限制损害，不能承诺完全隔离 DAW。

## 主要来源

- Steinberg VST3 SDK: https://github.com/steinbergmedia/vst3sdk
- VST3 Developer Portal: https://steinbergmedia.github.io/vst3_dev_portal/
- VST Module Architecture: https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/VST%2BModule%2BArchitecture/Index.html
- VST3 Parameters and Automation: https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/Parameters%2BAutomation/Index.html
- VST3 About MIDI: https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/About%2BMIDI/Index.html
- VST3 Workflow Diagrams: https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/Workflow%2BDiagrams/Index.html
- VST3 Plugin Format Structure: https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/Locations%2BFormat/Plugin%2BFormat.html
- Steinberg VST3 C API: https://github.com/steinbergmedia/vst3_c_api
- Rust `vst3` crate: https://docs.rs/vst3/latest/vst3/
- NIH-plug: https://github.com/robbert-vdh/nih-plug
- wry crate: https://docs.rs/wry/latest/wry/
- wry `WebViewBuilder`: https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html
- wry `WebView`: https://docs.rs/wry/latest/wry/struct.WebView.html
- Rust FFI and unwinding: https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding
- Rust `catch_unwind`: https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
- Cargo panic profiles: https://doc.rust-lang.org/cargo/reference/profiles.html#panic
- `rtrb`: https://docs.rs/rtrb/latest/rtrb/
- `ringbuf`: https://docs.rs/ringbuf/latest/ringbuf/
- `triple_buffer`: https://docs.rs/triple_buffer/latest/triple_buffer/
