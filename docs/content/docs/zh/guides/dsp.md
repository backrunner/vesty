---
title: DSP 内核
description: 在实时约束内准备、处理并测试原生 DSP。
order: 2
---

## 在音频回调之外完成准备

使用 `prepare()` 根据宿主提供的采样率和最大块大小配置系数，并预分配所需存储空间。

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

`resize` 必须在 `prepare` 中完成，绝不能出现在 `process` 中。

## 访问音频缓冲区

`ProcessContext` 为当前音频块提供不持有所有权的输入和输出声道。代码必须处理输入、输出声道数不同的情况，并清零所有没有写入的输出声道。

如果整个输出确定为静音，返回 `ProcessResult::Silence`；否则返回 `Continue`。

## 事件与播放状态

处理上下文包含按采样偏移排序的参数事件和音符事件，以及当前播放状态的快照。乐器应在准确的采样位置处理 NoteOn、NoteOff、压力、弯音和表情事件；效果器可以直接读取速度与工程位置，无需在音频回调中再次查询宿主。

## 双精度

默认处理路径使用 `f32`。只有算法确实受益时，才启用原生 `f64` 处理：

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

未启用原生 `f64` 时，如果宿主请求 64 位缓冲区，Vesty 会使用预分配的临时缓冲区完成 `f64` 与 `f32` 之间的转换。

## 测试

先在不依赖 DAW 的情况下独立测试音频内核，再运行适配层的自动化、总线、事件、静音标记和容量测试。发布前仍然需要在真实宿主中完成冒烟测试。
