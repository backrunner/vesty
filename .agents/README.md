# Vesty Agents 文档索引

本文档集记录 Vesty 的第一版技术调研、产品需求、架构和实施计划。Vesty 的目标是让开发者用 Rust 写实时音频/MIDI 处理核心，用任意 Web 技术写 UI，并由框架打包成 VST3 插件。

## 文档顺序

1. [00-research.md](00-research.md): 技术调研结论与来源。
2. [01-requirements.md](01-requirements.md): 产品目标、范围、非目标和验收标准。
3. [02-architecture.md](02-architecture.md): 总体架构、线程模型、数据流。
4. [03-module-design.md](03-module-design.md): crates、trait、公共 API 和内部模块设计。
5. [04-vst3-adapter.md](04-vst3-adapter.md): VST3 标准适配方案。
6. [05-webview-ui.md](05-webview-ui.md): 基于 wry 的系统 WebView UI 方案。
7. [06-realtime-memory.md](06-realtime-memory.md): 实时安全、内存和线程通信规范。
8. [07-build-packaging.md](07-build-packaging.md): 脚手架、构建、资源和 VST3 打包。
9. [08-developer-guide.md](08-developer-guide.md): 插件开发者使用指南草案。
10. [09-crash-safety-and-testing.md](09-crash-safety-and-testing.md): 防崩溃、测试和 DAW 兼容性计划。
11. [10-roadmap.md](10-roadmap.md): 分阶段 roadmap。
12. [11-latest-deps-feasibility.md](11-latest-deps-feasibility.md): 最新依赖基线与不可承诺/不可实现项。
13. [12-jsbridge-design.md](12-jsbridge-design.md): 基于 wry 的 JSBridge、状态共享与事件通信设计。
14. [13-implementation-status.md](13-implementation-status.md): 当前 alpha 实现状态、验证结果和剩余外部验收项。
15. [14-completion-audit.md](14-completion-audit.md): 原始实现计划到当前证据的完成度审计和 release gate 清单。

## 第一版架构判断

- 只实现 VST3，不做 VST2、CLAP、AU、AAX。
- Rust 产物以 `cdylib` 形式进入 VST3 bundle。
- UI 使用系统 WebView，通过 wry 作为底层封装；不引入 Tauri 的窗口、菜单、更新器、文件系统权限模型等应用框架层。
- 音频实时线程不做 WebView IPC、不分配、不加锁、不等待、不格式化日志。
- 参数和状态通过 host 可见的 VST3 参数、原子快照、预分配 ring buffer 和控制线程桥接。
- Web UI 技术栈对开发者开放，React/Vue/Svelte/纯前端均可，Vesty 只提供 JS bridge、资源协议和打包约定。
