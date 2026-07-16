# 09. 防崩溃、测试与兼容性

## 崩溃模型

VST3 插件在 DAW 进程内运行。Vesty 可以降低崩溃概率、捕获 Rust panic、隔离 UI 错误、避免实时线程阻塞，但不能保证 native crash 不影响 DAW。非法内存访问、栈溢出、abort、操作系统 WebView 崩溃都可能终止 host 进程。

## 防崩溃层级

### 编译期

- safe public API。
- unsafe 限制在少数 crate。
- 参数 ID derive 宏检测重复。
- `vesty-vst3` crate root 和 COM binding file 显式 deny `unsafe_op_in_unsafe_fn`，防止 VST3/COM trait method surface 重新出现隐式 unsafe operation。
- `vesty-vst3` crate root 和 COM binding file 显式 deny `clippy::undocumented_unsafe_blocks`，要求 production helper、fake COM tests 和 raw callback tests 都保留 `SAFETY:` 注释。
- `vesty-ui-wry` 使用 `#![deny(clippy::undocumented_unsafe_blocks)]`，要求平台 handle / WebView attach unsafe block 保留 `SAFETY:` 注释。
- feature gate WebView 后端。

### 初始化期

- 验证参数 ID、bus layout、UI manifest。
- `vesty validate` 静态检查 bundle 结构、moduleinfo、platform binary、macOS plist/pkginfo、parameter manifest、UI asset manifest size/sha256，并在读取这些 bundle metadata/input 前拒绝 symlink。
- 检查 block size/sample rate 范围。
- 分配所有实时资源。
- WebView runtime doctor。

### 运行期

- VST3 ABI 边界 catch panic。
- audio fault state 输出静音。
- processor panic fault state 通过 controller telemetry channel 暴露到 JSBridge，UI 可 `diagnostics.get` 或订阅 `diagnostics.fault` 查看 `faulted` / `faultCount`。
- processor panic 首次 fault transition 会向 RT log SPSC 推入固定 `RtLogEvent::Faulted`；controller/UI 线程 drain 后通过 `log.rt` best-effort 事件发送。
- VST3 COM factory、plug view 和 telemetry bind message 创建失败时返回 VST3 error/null pointer，不让 `unwrap()` panic 穿过 host callback。
- VST3 factory 创建 processor/controller 前都会校验参数 schema 与稳定 VST3 `ParamID` registry；重复/非法参数 schema 返回 `kResultFalse` 并保持 output pointer 为 null，不会让 host 拿到半初始化 processor/controller。
- VST3 message `IAttributeList` 在非实时 processor/controller message path 上对 null id/output pointer、缺失 attribute、字符串 buffer 过小、非空 binary data null pointer 做显式 `tresult` 返回；字符串截断时强制 nul-terminate，空 binary getter 返回 null data + size 0。
- VST3 editor attach 失败不会 panic；unsupported platform、unsupported parent 和 wry attach error 会在 `VESTY_BRIDGE_TRACE` 中记录 `editor_attach_*` marker，并向 host 返回 `kResultFalse`。
- wry WebView IPC callback 使用 panic boundary 包住 native bridge handler；如果 handler panic 且原始 IPC 是可解析 request，会向 JS Promise 回传 retryable `internal_error`，无法解析的消息则丢弃，不让 unwind 穿过 WebView/host UI callback。
- wry release asset custom protocol 会在 attach 阶段拒绝 symlinked asset root、symlinked `assets.manifest.json`、unsafe manifest path、重复 path、非法 sha256 和不能作为 HTTP `Content-Type` header value 的 MIME；response 组装使用显式 fallback，不通过 `expect()` panic 穿过 WebView 请求回调。
- `vesty-build` 的打包/静态校验会拒绝 symlinked `vesty.toml`、参数 specs/manifest、打包 binary 输入、UI dist root、`.vst3` bundle root、`Contents` / `Resources`、`moduleinfo.json`、packaged parameter manifest、platform binary dirs、macOS `Info.plist` / `PkgInfo` 和 Web UI asset roots/manifests/files，避免本地构建或采证路径静默跟随到可替换外部文件。
- `vesty param-manifest --specs`、`vesty smoke-host` 的 example parameter specs / `--bridge-trace` / `--meter-log` / `--check --out` 输入、DAW matrix evidence root / host dir / marker / `render_file` 文件证据，以及 CLI 通用 TOML/JSON report readers 都拒绝 symlinked trust-boundary input，避免本地诊断、DAW/release matrix、dependency baseline 或参数 sidecar 复验时跟随到项目目录外部的可替换文件。
- UI IPC schema validation；JS SDK 与 wry bootstrap 会在本地预校验 generic `request(type)`、subscription topic 和 handler 类型；非法 request type 不会产生 IPC，非函数 handler 不会污染 listener 表或触发 `subscription.add` IPC。Rust `BridgeRuntime` 仍会在 native IPC 边界权威校验 packet type，直接伪造 IPC 的空/超长/控制字符 type 会收到 non-retryable `validation_error`，且错误回包使用 `bridge.invalidType.error` 避免回显畸形 type；可识别当前 session/id 的反序列化失败 request 会收到 non-retryable `parse_error`。
- `vesty-plugin-ui` 的 `createBridge()` 会校验 host 必须能注册 unload listener、`initialSession` 必须是非空/长度受限/无控制字符字符串、`options` 必须是 object，`timeoutMs` 必须是 finite number；无效输入返回 non-retryable `validation_error`，不会注册 unload listener、创建 `__VESTY_INTERNAL__` 或发送 IPC。
- `vesty-plugin-ui` 与 wry bootstrap 会校验 ready payload 的 `editorSessionId` 必须是非空、最长 128 UTF-8 bytes 且无控制字符的字符串；畸形 ready session 会以 non-retryable `validation_error` 拒绝，不会采纳 session 或发送 `bridge.readyAck`。
- Rust -> JS delivery 入口也 fail closed: `vesty-plugin-ui` 和 wry bootstrap 的 `__VESTY_INTERNAL__.deliver()` / `deliverBatch()` 会先校验 packet/batch shape、protocol version、session/type/id 字符串、seq、lane、kind 和 error payload；畸形 packet 或非数组 batch 会被丢弃，不会抛出、不会触发 subscription handler，也不会错误结算 pending request。
- JSBridge event listener isolation: 单个 UI subscription handler 抛错时会记录到 console 并继续派发同 topic 其它 handler 与同批后续 packet，避免 UI 层异常打断 bridge delivery。
- `createSnapshotStore()` runtime validation/isolation: `options` 必须是 object，`topic` 复用 subscription topic 校验，`refreshOnEvent` 必须是 boolean，`subscribe(listener)` 会先校验 listener 必须是 function，`select(selector)` 会先校验 selector 必须是 function；无效输入返回 non-retryable `validation_error` 且不会触发底层 `subscription.add`。单个 snapshot listener 抛错时只记录 console error，不会阻断其它 snapshot listener，也不会让 `refresh()` 因 UI 回调异常而 reject。
- ring buffer backpressure。
- resource protocol path allowlist。

### 恢复期

- UI crash 可 reload。
- audio kernel panic 后默认不可恢复，只能禁用实例或重新加载 plugin。
- state restore 失败不 panic，返回 host error 并保留默认状态；VST3 adapter 会先验证/migrate state 并加载 custom state，成功后才写参数，避免 custom state 失败造成参数半恢复。
- VST3 state future/unsupported version 会被迁移入口拒绝，不会静默套用到参数、custom state 或 UI bridge state。

## Panic Guard

每个 VST3 callback 使用统一 wrapper:

```rust
fn vst_guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            report_panic(payload);
            fallback
        }
    }
}
```

process callback 特殊:

- panic 后设置 `FaultState::Silenced`。
- `FaultState::report()` 可读取 faulted 状态和 fault count；当前 VST3+wry 接线会把该快照映射为 JSBridge `PluginFaultReport`。
- panic fault log 使用固定 numeric code，不在 realtime path 创建字符串或 JSON。
- 当前 block output 清零。
- 后续 block 直接清零并返回成功或合适状态，避免 host 反复触发 panic。

## 测试分层

### Unit tests

- parameter normalized conversion。
- smoothing。
- state serialization/migration。
- event translation。
- ring buffer wrapper。
- asset manifest resolver。
- symlink/no-follow trust boundaries for config, parameter sidecars/specs, smoke-host traces/reports, DAW evidence roots/host dirs/markers/render files, dependency TOML/JSON inputs, package binary input, bundle metadata and Web UI assets。

### Realtime tests

- allocation guard。
- no lock guard。
- random automation。
- queue overflow。
- denormal handling。
- block size/sample rate matrix: 当前 VST3 fake COM 测试覆盖 32/64/128/1024 samples 和 44.1/48/96/192 kHz，并验证 `setupProcessing()` 会驱动 `AudioKernel::prepare()`。
- VST3 `Sample64` paths:
  - 默认 f32 developer kernel 走 adapter scratch fallback；fake COM 测试覆盖未调用 `setupProcessing()` 和 host block 超过预分配 `maxSamplesPerBlock` 时，realtime `process()` 清零 f64 outputs、设置 silence flags、不创建/进入 kernel、不在 realtime path 扩容。
  - opt-in native f64 kernel 走 `AudioKernel::SUPPORTS_F64` / `process_f64()`；fake COM 测试覆盖直接处理 host f64 buffers、不进入 f32 fallback、`NoAllocGuard` active、realtime path 0 allocation，并证明该路径不依赖 scratch capacity。
- fuzzed block sizes。

### VST3 adapter tests

- factory metadata。
- class ID uniqueness。
- processor/controller lifecycle。
- `setupProcessing()` -> `KernelInit` / `PrepareContext` lifecycle。
- bus arrangement negotiation。
  - effect: mono->mono、mono->stereo、stereo->stereo。
  - instrument: event input + one or more mono/stereo output buses, with bus 0 as main and later buses as aux.
- output silence flags。
  - `ProcessResult::Silence` 清零输出并设置每个 output channel bit。
  - `ProcessResult::Continue` 清理 stale silence flags。
- `kSample64` process path。
  - 默认 f32 DSP API: f64 host buffers 经预分配 f32 scratch 桥接到 `AudioKernel::process()`。
  - 原生 f64 DSP API: `AudioKernel::SUPPORTS_F64 = true` 时，adapter 以 `ProcessContext64` / `AudioBuffers64` 直接调用 `AudioKernel::process_f64()`。
  - fallback scratch capacity 不足或缺少 setup lifecycle 时静音返回，不在 audio thread 分配；native f64 路径不依赖 scratch capacity，但仍被 realtime allocation guard 覆盖。
- get/set state roundtrip。
- parameter begin/perform/end ordering。

### UI tests

- custom protocol path traversal。
- IPC schema validation，包括 invalid subscription topic/handler。
- native IPC handler panic -> bridge `internal_error` fallback。
- throwing JS subscription handler 不会阻断其它 listener 或后续 batch packet。
- throwing snapshot store listener 不会阻断其它 listener 或让 refresh 失败。
- JS bridge ready handshake。
- param feedback loop prevention。
- resize min/max constraints。
- 本地 fake-host open/close/resize editor stress。
- 真实 DAW/WebView open/close/resize stress 仍属于 release evidence。

### Validator tests

- Steinberg VST3 validator。
- `vesty validate` 静态 bundle/resource validation。
- Vesty smoke host 仍是后续增强项。
- 可选 pluginval 集成。

## DAW 兼容性矩阵

第一轮手工 smoke test 目标:

- Steinberg Cubase/Nuendo: 标准参考 host。
- REAPER: 常用跨平台 host，插件扫描和重载灵活。
- Bitwig Studio: Linux/Windows/macOS 覆盖好。
- Ableton Live: 用户量大，VST3 支持广。
- Studio One: 参数自动化和 UI 行为测试。

注意:

- Logic Pro 主要使用 Audio Unit，不属于 VST3 MVP 兼容目标。
- 每个 DAW 的具体版本和平台支持会变化，发布前需要重新确认。
- `vesty host-quirks` 输出内置 host profile/quirk registry，作为 smoke 准备清单；它不替代真实 host evidence。
- `vesty daw-matrix --write-template` 生成的每个 host README 会内联对应 smoke checklist、platform 和 quirk/mitigation；这些模板仍只是采集入口，不能替代真实 host evidence。

## Host 场景

每个 host 至少测试:

- 插件扫描。
- 创建实例。
- 播放/停止。
- 参数自动化录制和回放。
- 保存/关闭/重新打开工程。
- offline render。
- 打开/关闭 UI 20 次。
- 改变 sample rate。
- 改变 buffer size。
- bypass。
- 删除实例。

## WebView 场景

Windows:

- WebView2 runtime 存在。
- runtime 缺失 fallback。
- high DPI resize。
- DAW 多窗口/浮动窗口。

macOS:

- Intel 和 Apple Silicon。
- Retina scale。
- plugin editor close/reopen。
- code signed bundle。

Linux:

- X11。
- WebKitGTK runtime。
- GTK loop integration。
- Wayland experimental 标记。

## 性能基准

基准插件:

- passthrough。
- gain。
- simple synth。
- meter-heavy UI demo。

指标:

- process 平均/99p/最大耗时。
- allocation count。
- queue overflow count。
- UI frame latency。
- state restore latency。
- editor open time。

## Release gate

alpha release 必须:

- 官方 validator 通过。
- 三个 examples 在至少 macOS + Windows 构建成功。
- gain example 在至少两个 DAW 手工可用。
- allocation guard 无失败。
- Web UI fallback 可验证。
- `vesty release-check --strict` 必须让当前要求的 protocol snapshot 和 DAW evidence gate 返回通过；DAW gate 覆盖 scan/load/UI/UI->Host/meter stream/automation/buffer-sample-rate change/save-restore/offline render。当前本机 protocol snapshot 已通过，但完整 DAW evidence 仍未满足，因为 REAPER 还需补采 buffer/sample-rate change，且 Cubase/Nuendo、Bitwig、Ableton Live 和 Studio One 的真实 DAW smoke evidence 尚未采集。
- 如果要声明 release artifact 完整，还必须运行 `vesty release-check --strict --require-release-artifacts` 并传入 CI run URL、CI doctor artifacts、CI per-OS release-check snapshots、macOS/Windows x64/Linux X11 platform smoke reports、三示例/三平台的 `vesty validate --strict --report` 且 validator passed 的 JSON、三示例/三平台 CI static validate reports、macOS codesign + Windows signtool 签名证据，以及同时包含 accepted notarytool 和 stapler success 的 notarization log；默认 skipped 的 artifact check 或 pending platform-smoke 模板不能当作发布证据。

beta release 必须:

- macOS/Windows/Linux matrix 通过。
- 五个 DAW smoke test 记录完成。
- crash/fault telemetry 可读；当前已有 `diagnostics.get`、`diagnostics.fault` 和 `log.rt` 接线，beta 前仍需在目标 DAW/OS matrix 中验证 UI 可见性和 host 行为。
- 文档覆盖脚手架、UI、参数、state、发布。
