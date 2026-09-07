---
title: CLI 工作流
description: 使用 vesty 命令重复执行脚手架、构建、生成和诊断流程。
order: 1
---

## 查看命令

```bash
vesty --help
vesty package --help
vesty release-check --help
```

## 创建工程

先完成[源码安装](/docs/zh/quick-start)，设置 `VESTY_SOURCE`。目前 registry 包尚未发布。

```bash
vesty templates
vesty new my-plugin --template gain --vesty-path "$VESTY_SOURCE/crates/vesty"
vesty new my-plugin-ui --template web-ui-param-demo \
  --vesty-path "$VESTY_SOURCE/crates/vesty" \
  --plugin-ui-path "$VESTY_SOURCE/packages/plugin-ui"
```

安装生成 UI 的依赖前，先运行 `npm ci --prefix "$VESTY_SOURCE"` 和 `npm run build --prefix "$VESTY_SOURCE"` 构建本地 SDK。不指定模板时默认使用 React；显式传入的 `--kind` 和 `--ui` 会覆盖模板默认值。

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
