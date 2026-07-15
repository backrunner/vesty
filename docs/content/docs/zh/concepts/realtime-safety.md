---
title: 实时安全
description: 让 process() 保持确定、有界并且不阻塞音频线程。
order: 2
---

实时安全不是后期优化，而是从设计开始就必须遵守的约束。宿主可能每秒调用 `process()` 数百次，而且每次调用都有严格的完成时限。

## process() 中禁止的操作

- 分配内存或让集合扩容。
- 获取互斥锁，或等待条件变量。
- 执行文件、网络、JSON 或 WebView 操作。
- 格式化字符串或使用普通日志系统。
- 调用阻塞行为不明确的 API。

## 使用有界原语

Vesty 提供固定容量的事件列表、预分配的采样格式转换缓冲区、原子参数值、SPSC 队列、实时电平生产端，以及测试用的内存分配检测器。

在处理开始前解析参数 ID：

```rust
fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
    MyKernel {
        cutoff: self.params.resolve_or_invalid("cutoff"),
    }
}
```

在音频回调中，以常数时间读取参数句柄：

```rust
let cutoff = context.param_normalized(self.cutoff).unwrap_or(0.5);
```

不要在音频回调中调用 `specs()` 或按字符串搜索参数。实时路径应使用 `get_normalized_by_handle()` 和 `set_normalized_by_handle()`。

## 在不等待的前提下通信

UI 电平表只向实时生产端发布小型数值帧，再由 UI 线程异步取走。诊断信息应使用固定大小的实时日志记录，不要在音频回调中格式化字符串。

## 故障行为

如果音频内核发生 `panic`，或者宿主提供的音频块超过预分配容量，Vesty 会清零输出并标记静音，而不是只处理一部分数据或临时扩容。这可以保护宿主进程，但不能替代充分的测试。

## 必须保留的测试

- 在 `NoAllocGuard` 下运行处理。
- 覆盖配置允许的最大块大小，以及超过该上限的音频块。
- 测试首个自动化点之前、当下和之后的样本。
- 覆盖暂停、恢复、重置和采样率变化对应的生命周期调用。
- 验证处理故障后输出保持静音。
