---
title: 状态与生命周期
description: 正确恢复宿主工程，并响应音频处理生命周期的变化。
order: 4
---

## 状态所有权

VST3 组件保存处理器状态，控制器同步宿主可见的参数和 UI 状态。序列化数据必须带版本且结果确定；不要保存临时 DSP 缓冲区或 WebView 对象。

## 音频内核生命周期

| 钩子 | 用途 |
| --- | --- |
| `prepare` | 采样率设置和预分配 |
| `reset` | 处理配置变化后清理算法历史 |
| `suspend` | 宿主停止处理时暂停算法的后台状态 |
| `resume` | 宿主重新开始处理时恢复 |

`setupProcessing()` 和激活状态的变化会触发重置；`setProcessing(false/true)` 分别对应暂停和恢复。

## Web UI 状态

Bridge 快照分别记录参数、配置和 UI 的修订号。配置与 UI 状态更新采用乐观并发检查；使用旧修订号的编辑器会收到状态冲突错误，而不会覆盖更新后的数据。

```ts
const snapshot = await bridge.getSnapshot();

await bridge.setUiState({
  expectedRevision: snapshot.uiRevision,
  value: { selectedTab: 'meters' }
});
```

UI 状态只是可选的表现层数据。即使编辑器从未打开，插件也必须产生正确的音频输出。

## 测试完整序列

测试应覆盖保存、销毁、恢复、激活、准备、处理、暂停、恢复，以及编辑器的重新打开和重新加载。不同 DAW 的调用顺序可能不同，因此单元测试不应只接受某一种宿主调用序列。
