# 11. 最新依赖基线与可行性边界

核查日期: 2026-07-16

核查方式:

- crates.io sparse index (`https://index.crates.io/...`) 查询 Rust crate 当前版本。
- `git ls-remote --tags` 查询 Steinberg VST3 SDK tag。
- 本机工具链: `rustc 1.95.0`、`cargo 1.95.0`。

2026-06-08 本轮复核补充:

- `cargo search` 确认 `wry 0.55.1`、`vst3 0.3.0`、`raw-window-handle 0.6.2`、`rtrb 0.3.4`、`serde 1.0.228`、`serde_json 1.0.150`、`clap 4.6.1`。
- `cargo info ts-rs` 确认 `ts-rs 12.0.1`；`cargo search ts-rs` 当次请求遇到 crates.io 500，因此以 `cargo info` 的 crates.io metadata 结果为准。
- Steinberg 官方 VST 3 SDK change history 仍显示 3.8.0 为当前发布线，vcpkg 包名仍对应 `v3.8.0_build_66`。

2026-06-09 本轮复核补充:

- `cargo search` 重新确认 `wry 0.55.1`、`vst3 0.3.0`、`raw-window-handle 0.6.2`、`rtrb 0.3.4`、`serde 1.0.228`、`serde_json 1.0.150`、`ts-rs 12.0.1`、`clap 4.6.1` 仍是 crates.io 当前返回版本。
- workspace 中的 `schemars 1.2.1`、`toml 1.1.2`、`sha2 0.11.0`、`tempfile 3.27.0`、`thiserror 2.0.18` 已按当前 crates.io 最新基线锁定。`toml` 的 Cargo requirement 保持 `1.1.2`，因为 Cargo 会忽略 SemVer build metadata；联网 latest gate 会单独比对 crates.io 展示的 `1.1.2+spec-1.1.0`。
- `npm outdated --workspaces --long` 显示 TypeScript 最新为 `6.0.3`；四个 JS workspace package 已把 `typescript` devDependency 升到 `^6.0.3`，`package-lock.json` 中 `node_modules/typescript` 为 `6.0.3`，并通过 `npm test` 与 `npm run typecheck`。
- 新增 `vesty dependency-baseline` 离线门禁，校验当前仓库 Cargo workspace dependency、VST3 SDK binding baseline、JS package TypeScript range、React/Vue/Svelte adapter devDependency range 和 lockfile installed version 均匹配已复核基线；默认不联网，只防止仓库版本从最新基线漂移。
- 新增可选 `vesty dependency-baseline --latest` 联网门禁，会调用 `cargo search` / `npm view` 查询 crates.io 与 npm registry 当前 latest，并把 registry latest checks 写入同一份 report；本次真实运行通过。
- `release-check --require-release-artifacts` 现在可以把 `dependency-baseline/dependency-baseline-latest.json` 作为最终 release evidence。该 report 必须来自显式 `vesty dependency-baseline --latest`，包含 `cargo workspace external dependency baseline coverage` 和所有 crates.io/npm registry latest checks；普通离线 `dependency-baseline.json` 只用于 drift 防护，不能满足 final latest gate。

2026-06-10 本轮复核补充:

- `vesty dependency-baseline` 现在覆盖当前 `Cargo.toml [workspace.dependencies]` 中所有外部 Rust 依赖，而不只是 MVP 关键依赖抽样。当前覆盖 `arc-swap`、`atomic_float`、`camino`、`cargo_metadata`、`wry`、`vst3`、`raw-window-handle`、`rtrb`、`serde`、`serde_json`、`ts-rs`、`clap`、`mime_guess`、`plist`、`proc-macro-crate`、`proc-macro2`、`quote`、`schemars`、`syn`、`toml`、`sha2`、`tempfile`、`thiserror` 和 `tracing`。
- crates.io 查询使用 `cargo search` 优先，并在 search 失败或没有精确结果行时 fallback 到 `cargo info`。本次真实复核中 `cargo search ts-rs --limit 1` 返回 crates.io 500，但 `cargo info ts-rs` 返回 `version: 12.0.1`，最终 latest baseline 通过。
- `vesty dependency-baseline` 固定单个 `vesty-plugin-ui` package 的 dev dependency 语义：`react` / `@types/react`、`vue` 和 `svelte` range 必须保持 `latest`，并校验 package-lock 中的实际安装版本。对应 adapter 通过 `/react`、`/vue`、`/svelte` 子路径导出。
- `vesty dependency-baseline --latest` 现在会额外查询 npm registry latest `react`、`@types/react`、`vue` 和 `svelte`，确认 lockfile installed version 与 registry latest 一致。本次真实运行通过: `react 19.2.7`、`@types/react 19.2.17`、`vue 3.5.38`、`svelte 5.56.3`。
- `release-check` 现在会拒绝缺少 `cargo workspace external dependency baseline coverage` 的 latest report，即使 registry latest checks 本身完整；这防止新增外部 workspace dependency 未进入复核基线。

## 依赖策略

Vesty 应使用“最新兼容稳定依赖”作为默认策略:

- runtime/audio path 只引入稳定版、license 清晰、行为可控的依赖。
- CLI/dev tooling 可以接受 RC 依赖，但必须隔离在 `vesty-cli` 或 dev feature 中。
- 官方 VST3 SDK 以最新官方 tag 为准，Rust `vst3` crate 只是 binding 起点，不作为标准版本的唯一来源。
- `Cargo.lock` 对 examples、CLI 和工具 crate 入库；library crate 保持 semver dependency range。
- 每个 release 前跑 `cargo update` + validator + DAW smoke matrix，再决定是否提升依赖基线。
- 每次依赖复核后运行 `vesty dependency-baseline --out target/dependency-baseline/dependency-baseline.json` 与 `vesty dependency-baseline --check --out ...`，让本地和 CI 能发现仓库版本漂移。
- release 前运行 `vesty dependency-baseline --latest --out target/dependency-baseline/dependency-baseline-latest.json` 与 `--latest --check`，显式联网确认 crates.io/npm latest 仍匹配已复核 baseline，并把该文件纳入 release evidence。

## 当前最新版本建议

### VST3/FFI

| 用途 | 依赖 | 当前版本 | 结论 |
| --- | --- | --- | --- |
| 官方标准来源 | Steinberg VST3 SDK | `v3.8.0_build_66` tag | 作为 VST3 API source of truth |
| Rust binding | `vst3` | `0.3.0` | 可作为起点，但需要封装和版本审计 |
| native handle | `raw-window-handle` | `0.6.2` | 用于 wry child view 和平台 handle 抽象 |

2026-06-08 重新核对: Steinberg 官方 `vst3sdk` 最新 tag 仍为 `v3.8.0_build_66`，`vst3` crate 仍为 `0.3.0`，`raw-window-handle` 仍为 `0.6.2`。

2026-06-09 重新核对: crates.io 当前返回 `vst3 0.3.0`、`raw-window-handle 0.6.2`；VST3 SDK source of truth 仍按 `vesty-vst3-sys::STEINBERG_VST3_SDK_BASELINE = v3.8.0_build_66` 记录。

风险:

- `vst3` crate 最新版不一定完全跟上 VST3 SDK 3.8.0。Vesty 已加入 `vesty-vst3-sys` binding source 层记录当前基线；必要时启用/扩展 `generated-headers` backend，从最新 SDK headers 生成/维护自己的 bindings。

### UI/WebView

| 用途 | 依赖 | 当前版本 | 结论 |
| --- | --- | --- | --- |
| system WebView | `wry` | `0.55.1` | 默认 UI 后端 |
| WebView parent handle | `raw-window-handle` | `0.6.2` | 与 wry 0.55.x 配套 |

2026-06-08 重新核对: `wry` 仍为 `0.55.1`。

2026-06-09 重新核对: crates.io 当前返回 `wry 0.55.1`。

wry 0.55.1 信息:

- license: Apache-2.0 OR MIT。
- MSRV: Rust 1.77。
- default features: `protocol`、`os-webview`、`x11`。
- Linux default 依赖 WebKitGTK/GTK/soup3，并启用 X11。

建议 feature:

```toml
wry = { version = "0.55.1", default-features = false, features = ["protocol", "os-webview"] }

[target.'cfg(target_os = "linux")'.dependencies]
wry = { version = "0.55.1", default-features = false, features = ["protocol", "os-webview", "x11"] }
```

说明: 实际 Cargo 不能在同一 crate 中简单重复声明同名依赖，最终需要用 workspace dependency 或 cfg-specific feature 组织。这里表达的是平台意图。

### 实时通信与参数

| 用途 | 依赖 | 当前版本 | 结论 |
| --- | --- | --- | --- |
| SPSC realtime queue | `rtrb` | `0.3.4` | 首选，MIT/Apache，明确 realtime-safe |
| SPSC FIFO 备选 | `ringbuf` | `0.5.0` | 备选或 feature |
| atomic float | `atomic_float` | `1.1.0` | 参数镜像可用 |
| snapshot swap | `arc-swap` | `1.9.1` | control/audio 边界可用，但 audio thread 只做 load |
| latest-wins buffer | `triple_buffer` | `9.0.0` | 可选 feature，注意 MPL-2.0 license |

建议:

- core runtime 默认只用 `rtrb` + atomics。
- `triple_buffer` 因 license 是 MPL-2.0，先作为 optional feature 或暂缓。

### IPC/State/Schema

| 用途 | 依赖 | 当前版本 | 结论 |
| --- | --- | --- | --- |
| serde | `serde` | `1.0.228` | 标准选择 |
| JSON IPC | `serde_json` | `1.0.150` | WebView IPC 使用 |
| binary state | `postcard` | `1.1.3` | state 和内部消息可用 |
| JSON Schema | `schemars` | `1.2.1` | 可生成 UI schema |
| TypeScript type | `ts-rs` | `12.0.1` | 可生成 JS bridge typings |
| errors | `thiserror` | `2.0.18` | runtime/library error |
| logging facade | `tracing` | `0.1.44` | 非实时线程使用 |

2026-06-08 重新核对: `serde 1.0.228`、`serde_json 1.0.150`、`ts-rs 12.0.1` 仍为 crates.io 当前版本。

2026-06-09 重新核对: crates.io 当前返回 `serde 1.0.228`、`serde_json 1.0.150`、`ts-rs 12.0.1`；`schemars 1.2.1`、`thiserror 2.0.18` 也与 workspace baseline 一致。

### CLI/Build/Packaging

| 用途 | 依赖 | 当前版本 | 结论 |
| --- | --- | --- | --- |
| CLI parser | `clap` | `4.6.1` | `vesty-cli` |
| cargo metadata | `cargo_metadata` | `0.23.1` | 读取用户项目 |
| TOML config | `toml` | `1.1.2+spec-1.1.0` | `vesty.toml` |
| macOS plist | `plist` | `1.9.0` | Info.plist |
| MIME | `mime_guess` | `2.0.5` | asset protocol |
| SHA-2 | `sha2` | `0.11.0` | asset hash，需确认是否稳定发布策略合适 |
| UTF-8 paths | `camino` | `1.2.2` | CLI/build paths |
| temp files | `tempfile` | `3.27.0` | packaging tests |
| file watch | `notify` | `9.0.0-rc.4` | 仅 dev tooling；若不接受 RC，用上一个稳定大版本 |

2026-06-08 重新核对: `clap 4.6.1` 和 `notify 9.0.0-rc.4` 仍为 crates.io 当前版本；`notify` 继续只允许 CLI/dev watcher 使用，不进入 runtime/audio path。

2026-06-09 重新核对: crates.io 当前返回 `clap 4.6.1`；workspace `toml 1.1.2`、`sha2 0.11.0`、`tempfile 3.27.0` 也与当前复核基线一致。`cargo info toml` / `cargo search toml` 返回的 registry latest 是 `1.1.2+spec-1.1.0`，`vesty dependency-baseline --latest` 会按该展示版本校验。

### Proc macros

| 用途 | 依赖 | 当前版本 | 结论 |
| --- | --- | --- | --- |
| derive parsing | `syn` | `2.0.117` | `vesty-macros` |
| token generation | `quote` | `1.0.45` | `vesty-macros` |
| token stream bridge | `proc-macro2` | `1.0.106` | `vesty-macros` |
| facade crate path lookup | `proc-macro-crate` | `3.5.0` | 让 `#[derive(Params)]` 在普通插件 crate 和 facade crate 测试中都能解析到正确路径 |

### JS SDK/tooling

| 用途 | 依赖 | 当前版本 | 结论 |
| --- | --- | --- | --- |
| TypeScript compiler | `typescript` | `7.0.2` | `vesty-plugin-ui` devDependency 使用 `^7.0.2`；lockfile installed version 为 `7.0.2` |
| React adapter dev dep | `react` | `19.2.7` | `vesty-plugin-ui` devDependency range 为 `latest`；lockfile installed version 必须等于 npm registry latest |
| React types dev dep | `@types/react` | `19.2.17` | `vesty-plugin-ui` devDependency range 为 `latest`；lockfile installed version 必须等于 npm registry latest |
| Vue adapter dev dep | `vue` | `3.5.39` | `vesty-plugin-ui` devDependency range 为 `latest`；lockfile installed version 必须等于 npm registry latest |
| Svelte adapter dev dep | `svelte` | `5.56.5` | `vesty-plugin-ui` devDependency range 为 `latest`；lockfile installed version 必须等于 npm registry latest |

2026-06-09 重新核对: `npm outdated --workspaces --long` 显示 TypeScript 最新为 `6.0.3`，已升级并通过 workspace JS test/typecheck。

2026-06-10 重新核对: `vesty dependency-baseline --latest --out target/dependency-baseline-latest-current.json` 联网确认 TypeScript、React、@types/react、Vue 和 Svelte 当前 npm registry latest 均与 package-lock installed version 一致。

### 机器可复验门禁

`vesty dependency-baseline` 当前校验:

- Cargo workspace dependencies: `arc-swap`、`atomic_float`、`camino`、`cargo_metadata`、`wry`、`vst3`、`raw-window-handle`、`rtrb`、`serde`、`serde_json`、`ts-rs`、`clap`、`mime_guess`、`plist`、`proc-macro-crate`、`proc-macro2`、`quote`、`schemars`、`syn`、`toml`、`sha2`、`tempfile`、`thiserror`、`tracing`。
- VST3 SDK/binding baseline: Steinberg SDK `v3.8.0_build_66`、upstream `vst3` crate `0.3.0`。
- JS workspace: `vesty-plugin-ui` 的 `typescript` devDependency 和 `package-lock.json` installed version，以及 React/Vue/Svelte adapter 所需 framework devDependency range 与 lockfile installed version。

命令:

```bash
cargo run -p vesty-cli -- dependency-baseline --out target/dependency-baseline/dependency-baseline.json
cargo run -p vesty-cli -- dependency-baseline --check --out target/dependency-baseline/dependency-baseline.json
cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline/dependency-baseline-latest.json
cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline/dependency-baseline-latest.json
```

注意: 默认命令是离线 drift gate，不联网确认“最新”。显式 `--latest` 才会联网查询 crates.io/npm registry；Rust 侧 latest 查询优先用 `cargo search`，失败时用 `cargo info` fallback，以降低 crates.io search 临时 500 对 release evidence 的误伤。`release-check --require-release-artifacts` 只接受带 registry latest checks 的 report。npm latest checks 当前覆盖 TypeScript、React、@types/react、Vue 和 Svelte；Steinberg SDK 仍需按官方 tag/change history 单独复核。

## 不能承诺或做不到的部分

### 1. 不能保证插件崩溃完全不影响 DAW

VST3 插件加载在 DAW 进程内。Rust panic 可以在 ABI 边界捕获，WebView 错误可以隔离到 UI 层，但 native crash、abort、栈溢出、非法内存访问、系统 WebView 崩溃仍可能带崩 host。

可做:

- safe public API。
- ABI `catch_unwind`。
- audio faulted silence。
- UI reload fallback。
- validator、stress tests、host matrix。

不可承诺:

- “插件任何崩溃都不影响 DAW”。

### 2. 不能在音频实时线程使用 WebView/JS/JSON

WebView 和 JS IPC 都是非实时系统。`serde_json`、wry IPC、`evaluate_script`、UI thread dispatch 都不能进 `process` callback。

可做:

- UI/control thread 与 audio thread 通过 atomics、SPSC、预分配 snapshot 通信。
- meter/analyzer 降采样发送给 UI。

不可做:

- “JS 直接参与每个 audio block/sample 的实时处理”。

### 3. 不能保证所有 DAW 行为完全一致

VST3 标准减少差异，但 DAW 在扫描、editor parent、resize、DPI、automation flush、offline render、state restore 上仍有差异。

可做:

- 通过 Steinberg validator。
- 维护 host quirk table。
- 做 Cubase/REAPER/Bitwig/Live/Studio One 等 smoke matrix。

不可承诺:

- “兼容所有 DAW 的所有版本和所有插件场景”。

### 4. VST3-only 无法覆盖非 VST3 host

Logic Pro 主要使用 Audio Unit，Pro Tools 使用 AAX。只做 VST3 就不能原生进入这些 host。

可做:

- 明确 MVP 只支持 VST3-compatible DAW。
- 后续另开 AU/AAX wrapper roadmap。

不可做:

- “只输出 VST3，同时原生支持 AU/AAX-only host”。

### 5. Linux Wayland + wry child WebView 不能在 MVP 中强保证

wry child webview 对 macOS、Windows、Linux X11 是主要路径。Linux Wayland 场景下，wry 更偏向 GTK container/top-level window 组合；VST3 editor 给到的 parent handle 与 GTK container 的整合存在 host 差异。

可做:

- MVP 支持 Linux X11。
- Wayland 标记 experimental。
- 必要时为 Linux 写更低层的 native WebKitGTK backend。

不可承诺:

- “所有 Linux 桌面环境、所有 Wayland host 都稳定嵌入系统 WebView”。

### 6. 不能保证系统 WebView API 完全一致

系统 WebView 不是同一个浏览器:

- Windows 是 WebView2。
- macOS 是 WKWebView。
- Linux 是 WebKitGTK。

可做:

- JS bridge 使用保守 Web API。
- UI template 设定 browserlist。
- runtime doctor 检查 WebView2/WebKitGTK。

不可承诺:

- “像内置同一版 Chromium 一样完全一致”。

### 7. Windows 上不能可靠热替换正在被 DAW 加载的插件 DLL

很多 host 会锁住已加载的 `.vst3` binary。开发模式不能假设可以覆盖正在加载的 DLL。

可做:

- versioned dev bundle。
- standalone harness。
- DAW 重启/重新扫描提示。
- symlink/copy 策略按平台区分。

不可承诺:

- “Rust 代码改完，已加载 DAW 实例无重启热替换 native binary”。

### 8. 不能静态证明开发者 DSP 永远零分配

Rust 类型系统不能简单证明任意用户 `process` 实现完全无 allocation/lock。

可做:

- allocation guard 测试。
- lint/documentation。
- `ProcessContext` 不暴露会分配的 API。
- examples 和 CI 强约束。

不可承诺:

- “任意第三方代码编译通过就必然 realtime-safe”。

## 结论

Vesty 的核心目标可实现:

- Rust 编写 VST3 DSP 底层。
- Web 技术编写 UI。
- 不用 Tauri，只用 wry/system WebView。
- 保持音频实时路径与 UI/IPC 隔离。
- 提供脚手架、bundle 和 validator。

需要在产品承诺上收紧:

- 崩溃防护是降低风险，不是进程级隔离。
- DAW 兼容是矩阵验证，不是全宇宙保证。
- Linux 首发应写清 X11 优先，Wayland experimental。
- VST3-only 不覆盖 Logic Pro/AAX-only 场景。
- 最新依赖要分 runtime 稳定依赖和 CLI/dev RC 依赖。
