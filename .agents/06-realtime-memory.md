# 06. 实时与内存管理规范

## 黄金规则

`AudioKernel::process` 中禁止:

- heap allocation/free。
- mutex/RwLock/condvar。
- blocking channel。
- thread join/sleep/yield。
- 文件、网络、stdout/stderr。
- WebView、IPC、serde JSON。
- panic 作为控制流。

允许:

- 栈上小对象。
- 已预分配 buffer。
- atomics。
- lock-free/wait-free SPSC。
- bounded event list。
- `Copy` 类型处理。
- 固定结构的 RT log event `try_push`，例如 `RtLogEvent::Faulted { code }`；不能在 audio thread 格式化字符串。

## 内存生命周期

### 初始化阶段

可分配:

- kernel DSP state。
- delay lines、oscillator tables、FFT plan。
- parameter registry。
- ring buffers。
- RT meter/log SPSC channel。
- UI/control snapshots。

入口:

- plugin constructor。
- `prepare` / `setupProcessing`。
- `setActive(true)` 前。

### 处理阶段

不可分配。所有处理都使用:

- host 提供的 input/output buffers。
- kernel 预分配 state。
- stack。
- fixed-capacity event arrays。

### 重新配置阶段

host 改 sample rate、max block size、bus arrangement 时:

1. host 停止 processing。
2. wrapper 调用 `setProcessing(false)` 或 `setActive(false)` 路径。
3. control side 重新分配。
4. 重新 `prepare` kernel。
5. 再恢复 processing。

## 参数同步

参数有三份视图:

- registry: controller/control thread 的完整 metadata。
- atomic mirror: audio thread 可读的当前值。
- automation events: 当前 block 的 sample-accurate 参数变化。

`AudioKernel` 读取参数:

```rust
pub struct Kernel {
    cutoff: ParamHandle,
    bypass: ParamHandle,
}

impl AudioKernel for Kernel {
    fn process(&mut self, ctx: &mut ProcessContext<'_>) -> ProcessResult {
        let cutoff_normalized = ctx.param_normalized(self.cutoff).unwrap_or(0.5);
        let bypass = ctx.param_normalized(self.bypass).unwrap_or(0.0) >= 0.5;
        ProcessResult::Continue
    }
}
```

内部实现:

- `AtomicU32` 存 f32 bits 或使用 atomic float wrapper。
- f64 normalized 可在 control side 转换为 f32 plain value，audio side 只读所需类型。
- smoothing state 属于 audio kernel 或 param runtime 的 audio-only state。
- public API 提供 `ParamHandle`、`ParamCollection::resolve()`、`ProcessContext::param_normalized()`、`ProcessContext::param_automation()` 和 `ProcessContext::latest_param_automation()`；推荐在 `create_kernel()` 中解析 handle，process 内不做字符串查找。
- `param_automation(handle)` 只遍历当前 block 的 event slice 并按 `ParamHandle` 过滤，不分配、不锁、不访问 host/UI；VST3 adapter 会在进入 kernel 前保证事件按 sample offset 排序，手写 `ProcessContext`/测试 fixture 也应提供 sample-order event slice。
- `ParamAutomationSegments` 可直接消费 sample-order event slice，把 block 切成参数区间；同 offset 的同参数 point 以最后值生效，超出 block 的 point 会被夹到 block 末尾，不生成越界音频范围。

## 队列策略

### UI/control -> audio

用途:

- 非参数命令。
- preset/snapshot 切换请求。
- 需要在安全点应用的状态。

策略:

- SPSC。
- 固定容量。
- 满时返回错误给 control side。
- audio side 每个 block 开头 drain 限定数量，避免长尾。

### audio -> UI/control

用途:

- meter。
- analyzer。
- MIDI learn preview。
- 非阻塞日志。

策略:

- 队列满则丢弃。
- meter 优先 latest-wins，可用 triple buffer。
- 日志可计数丢弃条数。

## 日志

当前实现:

- `vesty-rt::RtLogEvent` 是固定 enum payload，只包含 queue id、level、code 和数字值，不包含 `String`。
- `vesty-rt::log_spsc(capacity)` 基于 `rtrb` 创建 `RtLogProducer` / `RtLogConsumer`。
- `QueueOverflow`、`Faulted`、`HostWarning`、`Custom` 覆盖常见 RT 诊断事件。
- `RtLogEvent::level()` 给非实时 drain 侧提供格式化时的 severity。

音频线程:

- 不格式化字符串。
- 写入固定 enum/code/数字参数。
- ring buffer 满则丢弃。

日志线程:

- drain ring buffer。
- 格式化成 tracing/log。
- 可写文件或 console。

示例:

```rust
use vesty::rt::{QueueId, RtLogEvent};

rt_log.try_push(RtLogEvent::QueueOverflow {
    queue: QueueId::Meter,
    dropped: 1,
});
```

## Allocation Guard

测试模式提供 global allocator wrapper:

- 在 VST3 `process` 的 sample-size 检查后设置 thread-local `NO_ALLOC = true`。
- guard 覆盖 host event collection、sample-order sort、transport mirror、buffer/context 组装、meter/log SPSC push 和 developer `AudioKernel::process`。
- kernel 创建和 `prepare` 必须发生在 `setupProcessing` / `setActive(true)` 等非实时生命周期；缺 kernel 的异常 `process` 调用只清零输出并写固定 `HostWarning` RT log，不在 guard 内兜底分配。
- 若发生 allocation，测试 panic 或记录失败。
- release 中默认关闭，避免 allocator 开销。

CI 应对示例插件跑:

- 32/64/128/1024 block size。
- 44.1/48/96/192 kHz。
- 随机自动化。
- open/close UI 并发测试。

当前本地测试还包含 deterministic fuzz-style automation segment 覆盖: 多个 block size、空事件、block 起点事件、同 offset 多点、其它 handle 干扰和越界 point 会被校验为连续覆盖整个 block、无越界、同 offset 最后值生效。

## Panic 策略

目标是避免 panic，而不是依赖捕获:

- 参数 ID 查找在初始化时解析为 typed handle，process 内不做 `unwrap`。
- 固定容量写入返回错误，调用方决定丢弃。
- 所有 user callback 返回 `Result` 或 `ProcessResult`。
- debug build 可 aggressive assert，release build 转 fault。

VST3 ABI 边界:

- 用 `catch_unwind` 保护。
- panic hook 写非阻塞日志。
- audio panic 后进入 `FaultState::Silenced`。

## Unsafe 规则

- unsafe 只允许在 `vesty-vst3`、`vesty-ui-wry` 平台 handle、少量 `vesty-rt` ring buffer wrapper 中出现。
- 每个 unsafe block 必须有 `SAFETY:` 注释；`vesty-ui-wry` 和 `vesty-vst3` 已用 `#![deny(clippy::undocumented_unsafe_blocks)]` 固定这一要求。
- `vesty-vst3/src/lib.rs` 与 `vesty-vst3/src/bindings_impl.rs` 已显式 `#![deny(unsafe_op_in_unsafe_fn)]`，避免在 host-facing COM trait 方法里重新引入隐式 unsafe operation。
- 不把 raw pointer 交给开发者。
- host buffer lifetime 只在 `ProcessContext<'a>` 内有效。
- 不跨线程移动 `!Send` 类型。
