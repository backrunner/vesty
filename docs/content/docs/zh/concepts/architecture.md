---
title: 架构
description: 了解音频、参数、状态和 UI 消息如何在 Vesty 各层之间流动。
order: 1
---

## 运行时结构

```text
DAW / VST3 宿主
  ├─ 工厂 + 控制器 ─────── 参数编辑、状态、编辑器生命周期
  └─ 音频处理器 ────────── 自动化 + 事件 + 音频缓冲区
             │
             ▼
        AudioKernel
             │ 固定容量遥测队列
             ▼
      BridgeRuntime ─────── 系统 WebView ─────── @vesty/plugin-ui
```

音频处理器和控制器是两个独立的 VST3 对象。Vesty 会明确划分两者的共享职责，而不是把状态隐藏在全局可变单例中。

## 插件与音频内核

`Plugin` 持有长期存在的元数据和参数存储；`AudioKernel` 持有可变的 DSP 状态。适配层会在音频回调之外创建并准备内核，然后针对宿主提供的每个音频块调用 `process()`。

```rust
pub trait AudioKernel: Send + 'static {
    fn prepare(&mut self, context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
}
```

`prepare` 用于配置采样率、最大块大小并完成预分配；`reset` 用于在处理配置变化后清理状态；`suspend` 和 `resume` 对应宿主暂停与恢复处理。

## 参数与自动化

每个音频块开始时，适配层会把宿主参数队列读入预分配的事件列表，并按采样偏移排序。第一个自动化点之前继续使用上一块的值；整块处理完成后，才把最终值写回原子参数存储。

## Web 编辑器边界

UI 运行时直接嵌入 `wry`，并使用带版本号的 JSON 协议。音频线程不会调用任何 WebView API。电平数据和日志通过有界队列传递；队列可以用最新值替换过期数据，但绝不能阻塞音频线程。

## `panic` 防护边界

所有面向宿主的 COM 回调都包裹在 ABI `panic` 防护边界中。发生 `panic` 时，Vesty 会返回相应的 VST3 回退结果，避免 Rust 展开过程穿过 FFI 边界。若故障发生在音频处理期间，Vesty 还会清零输出并发送诊断事件。
