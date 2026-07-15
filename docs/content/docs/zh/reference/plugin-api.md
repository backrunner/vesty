---
title: Plugin API
description: 快速查询 Vesty 的核心 Rust 插件开发接口。
order: 1
---

## Plugin

```rust
pub trait Plugin: Send + Sync + 'static {
    const INFO: PluginInfo;
    type Params: ParamCollection;
    type Kernel: AudioKernel;

    fn params(&self) -> &Self::Params;
    fn create_kernel(&self, init: KernelInit) -> Self::Kernel;
}
```

可选钩子用于提供 UI 描述、总线、程序列表、状态、MIDI 映射、音符表情、延迟和尾音信息。只应实现插件真正支持的能力。

## AudioKernel

```rust
pub trait AudioKernel: Send + 'static {
    const SUPPORTS_F64: bool = false;
    fn prepare(&mut self, context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
    fn process_f64(&mut self, context: &mut ProcessContext64<'_>) -> ProcessResult;
}
```

`InstrumentKernel` 是 `AudioKernel` 的别名；效果器和乐器使用相同实时契约。

## 处理上下文

`ProcessContext` 和 `ProcessContext64` 暴露：

- 输入、侧链和输出音频缓冲区。
- 按采样偏移排序的事件。
- 播放状态与处理模式。
- 常数时间的参数句柄读取。
- 实时电平与诊断信息生产端。

处理上下文只在一次音频回调内借用宿主缓冲区；`process()` 返回后不能继续保留这些引用。

## 参数

`FloatParam`、`BoolParam` 和选择参数都会生成 `ParamSpec`。`#[derive(Params)]` 会实现 `ParamCollection`，其中包括不分配内存的参数句柄访问。只有语义已经稳定时，才应设置显式 ID、旁路标记或 `#[param(skip)]`。

## 导出

```rust
vesty::export_vst3!(MyPlugin);
```

该宏会导出带 `panic` 防护的平台工厂和模块入口点。插件类型必须实现 `Default`，因为宿主工厂无法提供应用自定义的构造参数。
