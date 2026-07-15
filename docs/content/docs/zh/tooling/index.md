---
title: 工具链
description: 使用 Vesty CLI 创建、打包、验证并审计插件工程。
order: 4
---

`vesty` CLI 把仓库约定封装为可重复执行的命令。

- [CLI 工作流](/docs/zh/tooling/cli)介绍脚手架、开发命令和本地检查。
- [打包与验证](/docs/zh/tooling/packaging)生成结构清晰、可检查的 VST3 插件包。
- [发布证据](/docs/zh/tooling/release-evidence)区分本地检查结果与必须从外部环境取得的证明。
- [AI 辅助开发](/docs/zh/tooling/ai-development)说明如何安装配套 Skill，并让 AI 遵守 Vesty 的工程约束。
- [框架发布](/docs/zh/tooling/framework-release)说明如何通过 CI 发布版本一致的 registry 包与 CLI 二进制。

安装 CLI 后运行 `vesty --help` 即可查看所有命令。框架贡献者仍可以在 Vesty 工作区中使用 `cargo run -p vesty-cli -- --help`。
