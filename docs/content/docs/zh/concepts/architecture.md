---
title: 架构
description: 跟踪音频、参数、状态和 UI 消息在 Vesty 中的流动方式。
order: 1
---

## 运行时结构

```text
DAW / VST3 宿主
  ├─ factory + controller ── 参数编辑、状态、编辑器生命周期
  └─ audio processor ─────── 自动化 + 事件 + 音频缓冲
             │
             ▼
        AudioKernel
             │ 固定容量 telemetry queue
             ▼
      BridgeRuntime ─────── 系统 WebView ─────── @vesty/plugin-ui
```

Processor 和 controller 是两个独立 VST3 对象。Vesty 明确表达共享职责，不依赖全局可变单例。

## Plugin 与 kernel

`Plugin` 拥有长期存在的元数据和参数存储；`AudioKernel` 拥有可变 DSP 状态。适配层在音频 callback 外创建并准备 kernel，然后在每个宿主 block 调用 `process()`。

```rust
pub trait AudioKernel: Send + 'static {
    fn prepare(&mut self, context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
}
```

`prepare` 用于采样率配置和预分配；`reset` 用于处理配置变化后的状态清理；`suspend` / `resume` 对应宿主暂停和恢复处理。

## 参数与自动化

每个 block 开始时，适配层把宿主参数队列读取到预分配事件列表并按 sample offset 排序。首个自动化点之前仍使用上一 block 的值；block 完成后才把最终值写回原子参数存储。

## Web 编辑器边界

UI runtime 直接嵌入 `wry`，使用版本化 JSON 协议。音频线程不会调用 WebView API。Meter 和日志通过有界 queue 传递，允许用最新值替换过期数据，但绝不阻塞音频。

## Panic 边界

所有宿主侧 COM callback 都由 ABI panic boundary 包装。Panic 会转换成对应的 VST3 fallback，而不会穿越 FFI unwind。音频处理故障还会清零输出并发送诊断事件。

