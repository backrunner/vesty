---
title: 状态与生命周期
description: 恢复工程并正确响应宿主处理状态变化。
order: 4
---

## 状态所有权

VST3 component 保存 processor state，controller 镜像宿主可见参数和 UI state。序列化数据应版本化且确定，不要保存临时 DSP buffer 或 WebView 对象。

## Kernel 生命周期

| Hook | 用途 |
| --- | --- |
| `prepare` | 采样率设置和预分配 |
| `reset` | 处理配置变化后清理算法历史 |
| `suspend` | 宿主停止处理时暂停后台状态 |
| `resume` | 宿主重新开始处理时恢复 |

`setupProcessing()` 和 active state 变化会触发 reset；`setProcessing(false/true)` 对应 suspend/resume。

## Web UI 状态

Bridge snapshot 分离参数、配置和 UI revision。配置与 UI state 更新使用乐观 revision 检查，过期编辑器会收到 state conflict，而不是覆盖更新数据。

```ts
const snapshot = await bridge.getSnapshot();

await bridge.setUiState({
  expectedRevision: snapshot.uiRevision,
  value: { selectedTab: 'meters' }
});
```

UI state 只是可选的表现层状态。即使编辑器从未打开，音频输出也必须正确。

## 测试完整序列

覆盖保存、销毁、恢复、activate、prepare、process、suspend、resume、重新打开和 reload 编辑器。不同 DAW 的调用顺序不同，不要把测试绑定到单一宿主顺序。

