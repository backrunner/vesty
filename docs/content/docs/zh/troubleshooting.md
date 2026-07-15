---
title: 故障排查
description: 诊断常见构建、编辑器、自动化和发布问题。
order: 6
---

## WebView 功能无法编译

安装当前平台的 WebView 开发依赖，然后运行与 CI 相同的功能检查：

```bash
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo test -p vesty-ui-wry --features wry-backend
```

Linux 需要 GTK/WebKitGTK/X11 开发包，Windows 使用 WebView2，macOS 使用 WKWebView。

## 编辑器显示默认值而不是恢复值

从 `ready.paramValues` 初始化控件，不要使用 `ready.params[].defaultNormalized`。订阅 `param.changed`，并让控件跟随宿主确认状态。

## 参数编辑返回 `host_rejected`

宿主拒绝了 `performEdit`。此时不要更新控制器，也不要保留未经确认的 UI 状态。结束当前编辑手势，恢复最后一次确认的值，并检查宿主自动化状态和只读标记。

## 严格验证报告缺少导出符号

应在构建二进制文件的平台上执行验证。确认模块导出了 `GetPluginFactory` 和平台入口点，并安装所需的检查工具，例如 `nm` 或 `dumpbin`。

## 测试通过后 `release-check` 仍然失败

缺少外部证据时，这是预期行为。打开生成的行动计划，收集其中要求的 DAW、平台、validator、CI、签名或公证证据。最终发布不能依赖 `--skip-protocol`，也不能使用占位文件冒充真实证据。

## 音频回调发生故障

Vesty 会在发生 `panic` 后静音输出，但算法问题仍然必须修复。把对应的音频块重现为单元测试，在 `NoAllocGuard` 下运行，检查所有容量假设，并在音频回调之外读取实时诊断信息。
