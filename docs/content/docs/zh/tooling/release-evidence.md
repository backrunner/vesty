---
title: 发布证据
description: 区分本地检查和必须来自真实宿主或平台的证据。
order: 3
---

Vesty 有意采用严格的发布门槛。单元测试无法证明 DAW 已经扫描并加载插件，也无法证明 WebView2 已在 Windows 上正确附加，或 Apple 已接受公证请求。

## 本地证据

- 工作区测试与 Clippy 检查。
- 生成协议和参数清单的漂移检查。
- Crate 发布顺序与包发布准备状态。
- npm 打包边界与依赖版本基线。
- 当前平台上的严格静态插件包验证。

## 外部证据

- 在受支持 DAW 中完成扫描、加载、UI、自动化、保存与恢复，以及离线渲染测试。
- macOS、Windows x64 和 Linux X11 上的系统 WebView 冒烟测试报告。
- 每个示例、每个平台对应的 Steinberg validator 报告。
- GitHub Actions 运行链接与各操作系统的构建制品。
- macOS/Windows 签名验证。
- Apple `notarytool` 的接受记录与 `stapler` 成功记录。

## 运行最终发布检查

```bash
vesty release-check \
  --strict \
  --require-release-artifacts \
  --release-evidence-dir target/release-evidence \
  --report target/release-check.json \
  --plan target/release-action-plan.json
```

失败的报告仍然有价值：生成的行动计划会列出缺失的证据及其规范存放路径。绝不能用伪造的标记文件代替外部证据。

## 当前状态

Vesty 仍处于 alpha 阶段。当前本地实现检查已经通过，但真实 DAW 测试、跨平台 WebView 验证、完整 validator 矩阵、签名和公证仍然是正式发布的必要条件。
