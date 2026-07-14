---
title: CLI 工作流
description: 使用 vesty 命令执行可重复的工程操作。
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

模板包括原生 gain、instrument 以及 vanilla、React、Vue、Svelte Web UI starter。生成的 UI 从当前 `ready.paramValues` 初始化，并订阅确认后的参数变化。

## 开发与构建

```bash
vesty dev --config vesty.toml
vesty build --config vesty.toml
```

同时保留普通 Rust 和 npm 检查。CLI 不会替代编译器、linter 和单元测试结果。

## 协议与 manifest

```bash
vesty export-types --out target/vesty-protocol
vesty export-types --out target/vesty-protocol --check

vesty param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json \
  --check
```

CI 中使用 `--check` 捕获生成文件漂移。

## 诊断

```bash
vesty doctor --format json
vesty smoke-host --out target/smoke-host.json
```

`smoke-host` 检查仓库配置、sidecar 和可选 bridge/meter trace。它是 headless self-check，不是 DAW 或 validator 运行。

