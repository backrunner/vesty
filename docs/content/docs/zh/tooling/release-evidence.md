---
title: 发布证据
description: 区分本地检查和必须来自真实宿主或平台的证据。
order: 3
---

Vesty 的发布 gate 有意保持严格。单元测试不能证明 DAW 已扫描插件、Windows 上 WebView2 已 attach，或 Apple 已接受公证。

## 本地证据

- Workspace test 与 clippy。
- 生成协议和参数 manifest 漂移检查。
- Crate 发布顺序与 package readiness。
- npm pack 边界和 dependency latest baseline。
- 当前平台的 strict static bundle validation。

## 外部证据

- 支持 DAW 中的 scan、load、UI、自动化、保存/恢复和 offline render。
- macOS、Windows x64、Linux X11 系统 WebView smoke report。
- 每个示例和平台的 Steinberg validator report。
- GitHub Actions run URL 和各 OS artifact。
- macOS/Windows 签名验证。
- Apple notarytool accepted 和 stapler 成功。

## 运行最终 gate

```bash
vesty release-check \
  --strict \
  --require-release-artifacts \
  --release-evidence-dir target/release-evidence \
  --report target/release-check.json \
  --plan target/release-action-plan.json
```

失败的 report 仍然有价值：生成的 action plan 会列出缺失证据和规范路径。绝不能用伪造 marker 代替外部证据。

## 当前状态

Vesty 仍是 alpha。本地实现 gate 已通过，但真实 DAW、跨平台 WebView、完整 validator matrix、签名和公证仍是发布要求。

