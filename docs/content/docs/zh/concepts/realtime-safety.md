---
title: 实时安全
description: 让 process() 保持确定、有界且不阻塞。
order: 2
---

实时安全不是后期优化，而是设计约束。宿主可能每秒在严格 deadline 下调用 `process()` 数百次。

## process() 中禁止的操作

- 分配内存或让集合扩容。
- 获取 mutex 或等待 condition variable。
- 文件、网络、JSON 或 WebView 操作。
- 格式化字符串或使用普通日志系统。
- 调用阻塞行为不明确的 API。

## 使用有界原语

Vesty 提供固定事件列表、预分配 sample 转换 scratch、原子参数值、SPSC queue、实时 meter producer 和用于测试的 allocation guard。

在处理开始前解析参数 ID：

```rust
fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
    MyKernel {
        cutoff: self.params.resolve_or_invalid("cutoff"),
    }
}
```

在 callback 内以常数时间读取 handle：

```rust
let cutoff = context.param_normalized(self.cutoff).unwrap_or(0.5);
```

不要在音频 callback 中调用 `specs()` 或搜索字符串。实时路径是 `get_normalized_by_handle()` 和 `set_normalized_by_handle()`。

## 不等待地通信

UI meter 只向实时 producer 发布小型数值帧，由 UI 线程稍后 drain。诊断应使用固定 realtime log record，不要在 callback 内格式化消息。

## 故障行为

如果 kernel panic，或宿主 block 超过预分配容量，Vesty 会清零输出并标记 silence，而不是执行部分处理或临时扩容。这能保护宿主，但不能替代测试。

## 必须保留的测试

- 在 `NoAllocGuard` 下运行处理。
- 覆盖最大 block size 以及超过上限的 block。
- 测试首个自动化点之前、当下和之后的样本。
- 覆盖 suspend、resume、reset 和采样率变化。
- 验证 faulted processing 输出静音。

