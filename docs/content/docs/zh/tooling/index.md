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

在当前工作区中运行 `cargo run -p vesty-cli -- --help` 即可使用 CLI；相关 crate 发布后，也可以直接安装命令行工具。
