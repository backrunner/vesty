---
title: CLI 工作流
description: 使用 vesty 命令重复执行脚手架、构建、生成和诊断流程。
order: 1
---

## 查看命令

```bash
cargo run -p vesty-cli -- --help
cargo run -p vesty-cli -- package --help
cargo run -p vesty-cli -- release-check --help
```

## 创建工程

```bash
vesty new my-plugin
vesty new my-plugin --template web-ui-react
vesty templates
```

模板包括原生增益效果器、乐器，以及原生 JavaScript、React、Vue 和 Svelte Web UI 起始工程。生成的 UI 会从当前 `ready.paramValues` 初始化，并订阅宿主确认后的参数变化。

## 开发与构建

```bash
vesty dev --config vesty.toml
vesty build --config vesty.toml
```

仍然需要运行常规的 Rust 与 npm 检查。CLI 不会替代编译器、代码检查器或单元测试提供的反馈。

## 协议与参数清单

```bash
vesty export-types --out target/vesty-protocol
vesty export-types --out target/vesty-protocol --check

vesty param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json \
  --check
```

在 CI 中使用 `--check`，可以在生成文件与源码定义不一致时让检查失败。

## 诊断

```bash
vesty doctor --format json
vesty smoke-host --out target/smoke-host.json
```

`smoke-host` 检查仓库配置、附属文件，以及可选的 Bridge 和电平跟踪记录。它是一项无界面的自检，不等同于真实 DAW 测试或 Steinberg validator 运行。
