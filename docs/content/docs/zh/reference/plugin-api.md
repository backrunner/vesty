---
title: Plugin API
description: Rust 插件开发接口的精简参考。
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

可选 hook 暴露 UI descriptor、bus、program、state、MIDI mapping、note expression、latency 和 tail。只实现插件真实支持的能力。

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

## Process context

`ProcessContext` 和 `ProcessContext64` 暴露：

- 输入、sidechain 和输出音频 buffer。
- 按 sample offset 排序的事件。
- Transport state 与 process mode。
- 常数时间参数 handle 读取。
- Realtime meter 与诊断 producer。

Context 只在一次 callback 内借用宿主 buffer，`process()` 返回后不能保留引用。

## 参数

`FloatParam`、`BoolParam` 和 choice 参数生成 `ParamSpec`。`#[derive(Params)]` 实现 `ParamCollection`，包括无分配 handle 访问。只有语义稳定时才使用显式 ID、bypass flag 和 `#[param(skip)]`。

## 导出

```rust
vesty::export_vst3!(MyPlugin);
```

宏会导出带 panic guard 的平台 factory 和 module entry。插件类型必须实现 `Default`，因为宿主 factory 不持有应用自定义构造参数。

