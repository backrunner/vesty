---
title: DSP kernel
description: 准备、处理并测试有界的原生音频工作。
order: 2
---

## 在 callback 外准备

使用 `prepare()` 根据宿主采样率和最大 block size 配置系数并预分配存储。

```rust
impl AudioKernel for DelayKernel {
    fn prepare(&mut self, context: PrepareContext) {
        self.sample_rate = context.sample_rate;
        self.delay_line.resize(context.max_block_size * 8, 0.0);
        self.write = 0;
    }

    fn reset(&mut self) {
        self.delay_line.fill(0.0);
        self.write = 0;
    }

    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        // 这里只能进行有界读写、计算和预分配状态访问。
        ProcessResult::Continue
    }
}
```

`resize` 必须位于 `prepare`，绝不能出现在 `process`。

## Buffer 访问

`ProcessContext` 暴露当前 block 的非 owning 输入与输出 channel。需要处理输入/输出 channel 数不同的宿主，并清零没有写入的输出。

如果整个输出确定为静音，返回 `ProcessResult::Silence`；否则返回 `Continue`。

## 事件与 transport

Context 包含按 offset 排序的参数和 note 事件，以及 transport snapshot。乐器应按 sample offset 消费 NoteOn、NoteOff、pressure、pitch bend 和 expression；效果器可以读取 tempo 和工程位置，无需在 callback 中查询宿主。

## 双精度

默认路径为 f32。只有算法确实需要时才启用原生 f64：

```rust
impl AudioKernel for MasteringKernel {
    const SUPPORTS_F64: bool = true;

    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        ProcessResult::Continue
    }

    fn process_f64(&mut self, context: &mut ProcessContext64<'_>) -> ProcessResult {
        ProcessResult::Continue
    }
}
```

不 opt-in 时，Vesty 会使用预分配 scratch 完成 f64↔f32 转换。

## 测试

先独立测试 kernel，再运行适配层的自动化、bus、事件、silence flag 和容量测试。发布前仍需真实宿主 smoke test。

