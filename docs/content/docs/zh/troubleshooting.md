---
title: 故障排查
description: 诊断常见构建、编辑器、自动化和发布问题。
order: 6
---

## WebView feature 无法编译

安装平台 WebView 开发依赖，然后运行与 CI 相同的 feature gate：

```bash
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo test -p vesty-ui-wry --features wry-backend
```

Linux 需要 GTK/WebKitGTK/X11 开发包，Windows 使用 WebView2，macOS 使用 WKWebView。

## 编辑器显示默认值而不是恢复值

从 `ready.paramValues` 初始化控件，不要使用 `ready.params[].defaultNormalized`。订阅 `param.changed`，并让控件跟随宿主确认状态。

## 参数编辑返回 host_rejected

宿主拒绝了 `performEdit`。不要更新 controller 或保留乐观 UI 状态。结束 gesture，恢复最后确认值，并检查宿主自动化与 read-only flag。

## Strict validation 报告缺失 export

在与二进制相同的平台执行验证。确认导出了 `GetPluginFactory` 和平台 module entry，并安装所需检查工具，例如 `nm` 或 `dumpbin`。

## 测试通过后 release-check 仍失败

缺少外部证据时这是预期行为。打开生成的 action plan，收集对应 DAW、平台、validator、CI、签名或公证 artifact。最终发布不能使用 `--skip-protocol` 或 placeholder evidence。

## 音频 callback fault

Vesty 会在 panic 后静音输出，但仍需修复算法。将 block 重现为单元测试，在 `NoAllocGuard` 下运行，检查容量假设，并在 callback 外读取 realtime diagnostics。
