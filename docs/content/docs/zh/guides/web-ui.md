---
title: Web UI
description: 嵌入系统 WebView，并保持参数由宿主确认。
order: 3
---

Vesty 直接嵌入 `wry`，不会添加 Tauri runtime。编辑器可以使用原生 JavaScript、React、Vue 或 Svelte，并通过 `@vesty/plugin-ui` 通信。

## 配置资源

```toml title="vesty.toml"
[ui]
dir = "ui"
dev_url = "http://localhost:5173"
build = "npm run build"
dist = "dist"
width = 900
height = 560
min_width = 640
min_height = 420
```

开发环境使用 `dev_url`。Release 打包会把 `dist` 复制到 bundle，并生成包含大小和 hash 的 asset manifest。

## 完成握手

```ts
import { createBridge } from '@vesty/plugin-ui';

const bridge = createBridge(window, 'pending');
const ready = await bridge.ready();

const gain = ready.paramValues.find((value) => value.id === 'gain');
console.log(gain?.normalized);
```

`ready.params` 描述参数元数据，`ready.paramValues` 才是当前宿主/controller 值。恢复已有工程时不要用 `defaultNormalized` 初始化控件。

## 发起宿主编辑

```ts
const gestureId = crypto.randomUUID();

await bridge.beginParamEdit('gain', gestureId);
await bridge.performParamEdit('gain', 0.72, gestureId);
await bridge.endParamEdit('gain', gestureId);
```

只有宿主接受 `performEdit` 后，controller 才更新参数。拒绝会转换成 `host_rejected`，而不是保留乐观本地状态。

## 订阅确认事件

```ts
const unsubscribe = bridge.subscribe('param.changed', (event) => {
  if (event.id === 'gain') updateGainControl(event.normalized);
});
```

使用确认事件同步多个编辑器、宿主自动化、preset 与 program change。

## Reload 行为

Pending session 上的新 `bridge.hello` 会重置旧订阅、gesture、meter 和 editor session。每次握手前 native endpoint 都会刷新 `paramValues`，重新加载或重新打开的编辑器不会显示旧值。

## 音频线程之外的工作

JSON、WebView evaluation、订阅和 snapshot 都在 `process()` 外运行。Realtime meter 通过有界 queue 传递，由 UI 线程异步 drain。

