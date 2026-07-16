---
title: Web UI
description: 嵌入系统 WebView，并始终以宿主确认的参数状态为准。
order: 3
---

Vesty 直接嵌入 `wry`，不引入 Tauri 运行时。编辑器可以使用原生 JavaScript、React、Vue 或 Svelte，并通过 `vesty-plugin-ui` 与原生层通信。

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

开发时，编辑器从 `dev_url` 加载。发布打包时，Vesty 会把 `dist` 复制到插件包，并生成记录资源大小与哈希值的清单。

## 完成握手

```ts
import { createBridge } from 'vesty-plugin-ui';

const bridge = createBridge(window, 'pending');
const ready = await bridge.ready();

const gain = ready.paramValues.find((value) => value.id === 'gain');
console.log(gain?.normalized);
```

`ready.params` 描述参数元数据，`ready.paramValues` 则包含宿主和控制器当前确认的值。恢复已有工程时，不要使用 `defaultNormalized` 初始化控件。

## 发起宿主编辑

```ts
const gestureId = crypto.randomUUID();

await bridge.beginParamEdit('gain', gestureId);
await bridge.performParamEdit('gain', 0.72, gestureId);
await bridge.endParamEdit('gain', gestureId);
```

只有宿主接受 `performEdit` 后，控制器才会更新参数。若宿主拒绝编辑，Bridge 会返回 `host_rejected` 错误，界面不应继续保留未经确认的本地值。

## 订阅确认事件

```ts
const unsubscribe = bridge.subscribe('param.changed', (event) => {
  if (event.id === 'gain') updateGainControl(event.normalized);
});
```

使用宿主确认的事件同步多个编辑器、宿主自动化、预设切换和程序切换。

## 重新加载编辑器

待连接会话收到新的 `bridge.hello` 后，会重置旧订阅、编辑手势、电平数据和编辑器会话。每次握手前，原生端点都会刷新 `paramValues`，因此重新加载或重新打开的编辑器不会显示过期值。

## 音频线程之外的工作

JSON 解析、WebView 脚本执行、订阅管理和状态快照都在 `process()` 之外运行。实时电平数据通过有界队列传递，再由 UI 线程异步取走。
