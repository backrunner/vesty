# 07. 脚手架、构建与打包

## 用户项目结构

```text
my-plugin/
  Cargo.toml
  vesty.toml
  src/
    lib.rs
  ui/
    package.json
    src/
    index.html
  assets/
    icon.png
    snapshots/
```

`Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
vesty = { version = "0.1", features = ["vst3", "webview-wry"] }
```

`vesty.toml`:

```toml
[plugin]
name = "My Plugin"
vendor = "My Company"
version = "0.1.0"
kind = "effect"
class_id = "01234567-89ab-cdef-0123-456789abcdef"

[ui]
dir = "ui"
dev_url = "http://localhost:5173"
build = "npm run build"
dist = "dist"
width = 900
height = 560
min_width = 640
min_height = 420

[package]
bundle_id = "com.mycompany.myplugin"
category = "Fx"
signing = "Developer ID Application: My Company"
parameter_manifest = "target/vesty-parameters.json"
```

`[ui].width/height` 和 `[ui].min_width/min_height` 必须成对出现且大于 0；如果同时设置 initial size 和 min size，min 不能超过 initial。`dir/dev_url/build/dist` 在出现时不能为空。

`vesty.toml` 使用严格 schema：未知的顶层 table 或未知字段会在 `read_config()` 阶段失败，而不是被静默忽略。当前 schema 支持 effect 插件用 `[plugin].sidechain = true` 声明一个可选 stereo sidechain input bus；instrument 开启 sidechain 会被拒绝。Instrument 多输出当前通过 Rust `Plugin::output_buses()` runtime API 声明，尚未提供 `[bus]` 配置 DSL；Wayland 开关、installer DSL 等配置也不应提前写入 `vesty.toml`。对应配置能力落地时再扩展 schema 和迁移规则。

配置校验:

- `[plugin].name`、`[plugin].vendor`、`[plugin].version`、`[plugin].kind` 和 `[plugin].class_id` 必填且不能为空。
- 配置中的文本 metadata 不能包含 ASCII/Unicode control characters；覆盖 `[plugin].name/vendor/version/kind`、`[package].bundle_id/category/signing/parameter_manifest` 和 `[ui].dir/dev_url/build/dist` 等字段，避免把换行或不可见字符写入 bundle metadata、签名 identity、sidecar path 或 UI 路径/命令。
- `read_config()` 读取 `vesty.toml` 前会拒绝 symlinked config file；配置入口必须是普通文件，避免打包时跟随到项目目录外部的可替换配置。
- 未知字段会被拒绝，例如顶层 `[bus]`、`[ui].experimental_wayland` 或 `[package].installer`。这能避免开发者误以为未来 scope 已被当前打包器识别。
- `[plugin].sidechain = true` 仅对 effect/fx/audio-effect/audio_effect 生效，表示运行时 VST3 bus metadata 暴露 main input + optional aux sidechain input；打包器不把 sidechain 写入 `moduleinfo.json`。
- `[plugin].kind` 只接受 Vesty MVP 支持的 effect/instrument 语义: `effect`、`fx`、`audio-effect`、`audio_effect` 或 `instrument`。更细的 VST3 分类应写到 `[package].category`。
- `[plugin].class_id` 必须是 16-byte hex UUID/FUID；接受标准 UUID 或 32 位 hex。
- `[package].bundle_id` 如果填写则不能为空，必须是 conservative reverse-DNS shape: 至少包含一个 `.`，每段非空，只允许 ASCII 字母、数字和 `-`，且每段不能以 `-` 开头或结尾；未填写时 macOS `Info.plist` 会按 bundle executable 生成 `dev.vesty.<name>` fallback。
- `[package].category` 如果为空或缺失，`moduleinfo.json` class category 会按 `[plugin].kind` 映射为 `Fx` 或 `Instrument`。
- `[package].signing` 如果填写则不能为空；未填写时 `vesty package` 不执行内置签名步骤。
- `[package].parameter_manifest` 可选，指向一个由 `vesty-build::ParameterManifest::from_param_specs()` 或 `vesty param-manifest --specs params.specs.json --out vesty-parameters.json` 生成的 JSON；打包时会复制为 `Contents/Resources/parameters.manifest.json` 并校验每个 `vst3ParamId` 是否匹配 `vesty-params::stable_vst3_param_id()`。未填写时不会生成该 sidecar，也不会尝试从已编译 binary 推断参数 schema。
- `vesty new` 默认生成 `[package]`: `bundle_id = "dev.vesty.<crate-name>"`；effect category 为 `Fx`，instrument category 为 `Instrument`，并写入 `parameter_manifest = "vesty-parameters.json"`。

## CLI 命令

### `vesty templates`

```bash
vesty templates
vesty templates --format json
```

职责:

- 列出内置 starter gallery，包括 `gain`、`midi-synth`、`web-ui-param-demo`、`vanilla-ui-param-demo`、`vue-ui-param-demo`、`svelte-ui-param-demo` 和 `web-ui-instrument`。
- `--format text|json` 分别面向人类快速浏览和工具/IDE 集成。
- gallery 只提供一组 first-party starter；后续第三方模板生态应单独设计版本、来源校验和下载缓存，不应混入当前 runtime crates。

### `vesty new`

```bash
vesty new my-web-demo --template web-ui-param-demo
vesty new my-synth --template midi-synth
vesty new my-plugin --kind effect --ui react
vesty new my-synth --kind instrument --ui svelte
vesty new headless-gain --kind effect --ui none
vesty new local-demo --ui vanilla --vesty-path /path/to/vesty/crates/vesty --plugin-ui-path /path/to/vesty/packages/plugin-ui
```

生成:

- Rust plugin skeleton。
- UI template。
- `--template <id>` 可从内置 starter gallery 选择默认 plugin kind 和 UI 模板；显式 `--kind` 或 `--ui` 会覆盖 starter 默认值。
- 生成的 Web UI 会跟随 plugin kind 绑定 starter 主参数: effect 模板绑定 `gain`，instrument 模板绑定 `volume`，避免 UI bridge gesture 指向不存在的参数。
- 生成的 Web UI 以 `bridge.ready()` 的 `BridgeReadyPayload.params[].defaultNormalized` 初始化 slider 默认值，并订阅 `param.changed` 同步 host/controller/UI 确认后的参数值；`PluginSnapshot` 只承载 config/ui state revision，不作为当前参数值来源。
- `--vesty-path` 可让 Rust crate 使用本地 `vesty` path dependency。
- `--plugin-ui-path` 可让 UI `package.json` 使用本地 `@vesty/plugin-ui` file dependency，便于框架仓库内 smoke。
- 输出项目路径必须不存在，且创建前会拒绝既有 symlink 输出父目录；这样 `vesty new linked-parent/my-plugin` 不会把 starter 文件写入 symlink 指向的外部 workspace。
- `vesty.toml`。
- `params.specs.json`，作为可编辑的参数 schema 输入。
- `vesty-parameters.json`，由 `vesty param-manifest` 生成并被 `vesty.toml` 引用。
- `README.md`，包含项目布局、UI build、`vesty build/package/validate` 和 `vesty doctor` 起步命令。
- `Cargo.toml` 会显式写入 `publish = false`，因为生成项目默认作为 `.vst3` bundle 分发，而不是 crates.io library crate。
- 有 UI 的模板会在 `ui/package.json` 写入 `"private": true`，因为该包默认是插件 UI asset app，而不是 npm library package。
- example tests。
- `.cargo/config.toml` 可选。

### `vesty dev`

职责:

- 启动 UI dev server。
- 构建 debug plugin。
- 可选创建到用户 VST3 dev folder 的 symlink/copy。
- 打印 DAW 扫描路径。
- 可选启动 standalone harness。

当前 CLI:

- `--install-dev` 会在 `cargo build` 成功后打包并安装 `.vst3`。
- `--binary <path>` 可选；显式传入时作为 cdylib override。
- 未传 `--binary` 时，CLI 会用 Cargo metadata 匹配当前项目 `Cargo.toml` 的 `cdylib` target，并按 debug/release profile 自动推断 `target/<profile>/lib*.dylib`、`*.dll` 或 `lib*.so`。若 metadata 不能唯一识别 plugin package，则提示开发者显式传 `--binary`。
- `--platform macos|windows-x64|linux-x64`、`--out <path>`、`--vst3-dir <path>` 和 `--install-mode copy|symlink` 与 `vesty package` 的语义一致；不传 `--platform` 时按当前 OS 推断。

### `vesty build`

职责:

- 编译 Rust cdylib。
- 运行 UI build。
- 生成 resource manifest。
- 生成 metadata。
- 默认执行 release build；`--debug` 可切换为 debug build，`--release` 可显式声明 release，二者不能同时使用。
- `--no-ui` 可跳过 `[ui] build/dist`，只构建 Rust 插件。
- UI build 会在 `[ui].dir` 缺失、`[ui].build` 为空/失败、`[ui].dist` 缺失、UI 尺寸配置无效或 manifest 生成失败时返回带目录/字段/命令上下文的错误。

### `vesty param-manifest`

职责:

- 从显式参数 schema JSON 生成 `vesty-build::ParameterManifest`。
- 输入支持 `ParamSpec[]` 或 `{ "version": 1, "parameters": [...] }`。
- 输出包含 `version = 1`、`idAlgorithm = "vesty.vst3.param.fnv1a31-positive.v2"`、字符串参数 ID、稳定正数 VST3 `ParamID` 和完整 `ParamSpec`，其中 flags 使用 JS-friendly camelCase，包括 `readOnly` 和 `programChange`。
- `--check` 会读取已有 `--out` manifest 并按语义比较；参数 schema、算法名或 `vst3ParamId` 漂移时返回非零。
- `--specs` 输入必须是真实文件，不能是 symlink；`--out` 复验读取同样复用 `vesty-build` 的 no-follow parameter manifest reader。
- 该命令不会加载 `.dylib` / `.dll` / `.so`，因此不会把任意插件代码执行到 CLI 进程中。

```bash
vesty param-manifest --specs params.specs.json --out vesty-parameters.json
vesty param-manifest --specs params.specs.json --out vesty-parameters.json --check
```

### `vesty package`

职责:

- 按平台组装 `.vst3` bundle。
- macOS 生成 Info.plist、PkgInfo。
- Windows/Linux 生成 `Contents/<arch-platform>/`。
- 复制 UI assets 到 `Contents/Resources/ui`。
- 生成 `Contents/Resources/moduleinfo.json`。
- 校验 `[plugin].name`、`vendor`、`version`、`kind` 必须非空且不含 control characters；`[package].bundle_id` 如果存在也必须非空、无 control characters 且符合 conservative reverse-DNS shape；`[package].category` 如果存在可为空以启用 kind fallback，但不能包含 control characters；`[package].signing` 和 `[package].parameter_manifest` 如果存在必须非空且不含 control characters。
- 校验 `[plugin].class_id` 是 16-byte hex UUID/FUID；接受标准 UUID 或 32 位 hex，写入 `moduleinfo.json` 时规范化为小写 UUID。
- `moduleinfo.json` class category 优先使用 `[package].category`；为空或缺失时按 `[plugin].kind` 映射为 `Fx` 或 `Instrument`。
- 如果配置了 `[package].parameter_manifest`，读取该 JSON，要求 `version = 1`、`idAlgorithm = "vesty.vst3.param.fnv1a31-positive.v2"`、`parameters[].id == parameters[].spec.id`，并重新计算每个 `vst3ParamId`；通过后写入 `Contents/Resources/parameters.manifest.json`，保留 `ParamSpec.flags.programChange` 等 host metadata。
- 打包输入的 plugin binary、`[package].parameter_manifest` sidecar 和 UI dist root 都必须是普通文件/真实目录，不能是 symlink；`vesty-build` 会在复制 binary、读取参数 sidecar 或递归复制 UI assets 前先做 no-follow metadata 检查。打包输出目录创建也会逐段拒绝既有 symlink ancestor 或 symlink leaf；如果同一 bundle 中已有 platform binary、`Info.plist`、`PkgInfo`、`moduleinfo.json`、`parameters.manifest.json`、`assets.manifest.json` 或 `Contents/Resources/ui` 被替换成 symlink，重新打包会失败而不是跟随写入、删除或覆盖外部目标。
- 如果 `[package].signing` 非空:
  - macOS 调用 `codesign --force --deep --options runtime --timestamp --sign <identity> <bundle.vst3>`。
  - Windows x64 调用 `signtool.exe sign /fd SHA256 /td SHA256 /tr http://timestamp.digicert.com /n <identity> <binary.vst3>`。
  - Linux 不内置 bundle signing；按发行渠道对安装包或发行物做外部签名。
- 支持 `--install-dev` 在打包后把 `.vst3` 安装到本机 VST3 dev 目录。
- `--platform <target>` 可覆盖打包平台；不传时按当前 OS 推断为 macOS、Windows x64 或 Linux x64。
- `--vst3-dir <path>` 可覆盖安装目录；不传时默认 macOS `~/Library/Audio/Plug-Ins/VST3`、Windows `%CommonProgramFiles%/VST3`、Linux `~/.vst3`。
- `--install-mode copy|symlink` 控制安装方式；默认 `copy`，会覆盖同名旧 dev bundle。copy 模式会拒绝 symlinked source bundle、symlinked install dir ancestor 和 copy 过程中的 symlinked bundle entry；如果旧 destination 是 symlink，会 unlink symlink 本身而不会跟随删除外部目标。`symlink` 模式只在最终 destination 创建一个明确的 bundle symlink，install root 仍必须是真实目录。

### `vesty notarize`

职责:

- macOS-only release helper。
- 使用 `ditto -c -k --keepParent` 把 `.vst3` bundle 打成 notary upload zip。
- 调用 `xcrun notarytool submit <archive> --wait`。
- 支持 `--keychain-profile <profile>` 或 `--apple-id <id> --team-id <team> --password <app-password>`。
- 默认 notarization 成功后调用 `xcrun stapler staple <bundle.vst3>`；`--no-wait` 必须同时配合 `--no-staple`。
- 该命令只封装 Apple notarization workflow，不替代 release CI artifact 留证。

### `vesty validate`

职责:

- 先运行 Vesty 静态 bundle 检查: `.vst3/Contents` 结构、`moduleinfo.json` 顶层 name/vendor/plugin_version、class name/category/cid、平台 binary、平台 binary 文件魔数/架构、macOS `Info.plist`/`PkgInfo` 内容。`moduleinfo.json` 的顶层 name/vendor/plugin_version 和每个 class 的 name/category 必须非空且不能包含 control characters。
- macOS metadata gate 会解析 `Info.plist`，要求根节点为 dictionary、`CFBundlePackageType = BNDL`、`CFBundleExecutable` 非空、指向 `Contents/MacOS/<executable>` 文件且与 `moduleinfo.json` 推导出的 macOS binary name 一致，`CFBundleName` 与 `moduleinfo.json` name 一致，`CFBundleShortVersionString` 和 `CFBundleVersion` 与 `moduleinfo.json` plugin_version 一致，`CFBundleIdentifier` 非空且符合同一套 conservative reverse-DNS shape，并要求 `PkgInfo` 精确为 `BNDL????`。
- 如果存在 `parameters.manifest.json`，静态检查会先拒绝 symlinked sidecar，再按同一 schema 重新校验参数 sidecar，防止字符串 ID、VST3 数值 ID 或参数显示 metadata 漂移。
- `.vst3` bundle root、`Contents`、`Resources`、`moduleinfo.json`、platform binary dirs/binaries 以及 macOS `Info.plist` / `PkgInfo` 都会在读取或扫描前做 no-follow metadata 检查；symlinked metadata、binary 或平台目录会导致静态校验失败。
- 如果 bundle 包含 Web UI，校验 `assets.manifest.json` schema、`version = 1`、build-time `root` 非空且无 control characters、entry、files 非空、asset path allowlist、重复 path、mime、sha256 格式、文件存在、size 和 sha256。manifest 顶层和 file entry 的未知字段会被拒绝；`root` 只作为 provenance，不要求等于运行时 bundle 路径，因为 `.vst3` bundle 可以被移动。`Contents/Resources/ui`、`assets.manifest.json` 和 manifest-listed files 都不能是 symlink。asset path 必须是 URL-safe 相对路径，不能包含反斜杠、空段、`.`/`..` 段、ASCII control、`%`、`?`、`#` 或 `:`。
- 调用 Steinberg validator。
- `--static-only` 可只运行 Vesty 静态检查，适合没有 Steinberg SDK 的 CI 先做 bundle/resource gate；静态检查会确认 macOS binary 是 64-bit Mach-O/fat Mach-O，Windows x64 binary 是 PE/MZ 且 COFF machine 为 x86_64，Linux x64 binary 是 64-bit ELF 且 `e_machine = x86_64`，防止把错误平台/架构产物或文本占位文件作为 CI static validate evidence。默认模式会把导出符号工具缺失记录为 `static_check.binary_exports[].status = "skipped"` 并继续写 report；`--strict` 会要求每个可识别平台 binary 都有匹配的 `ok` binary export evidence，缺失/skipped 都会在 report 写出后返回非零。
- 支持 `--format text|json`；JSON report 包含静态检查结果、validator path、exit code、stdout/stderr，以及从 Steinberg 输出解析出的 `tests_passed` / `tests_failed`。解析器支持 canonical `Result: 47 tests passed, 0 tests failed`、大小写变化和 `Tests passed: 47` / `Tests failed: 0` 拆行格式；release gate 仍要求两个计数字段都存在。
- 支持 `--report <path>` 把完整 JSON report 写入文件；静态 bundle 检查失败时也会先写 report 再返回非零。
- 支持 `--validator-log <path>` 把 Steinberg validator 原始 stdout/stderr 写入文件，便于 CI artifact 留档。
- 后续可扩展调用 pluginval；Vesty 自带的本地 headless self-check 已由 `vesty smoke-host` 提供。

### `vesty smoke-host`

职责:

- 对 Vesty workspace 做本地 headless framework self-check。
- 检查根 `Cargo.toml`、三个 MVP examples 的 `vesty.toml`、`params.specs.json` 到 `vesty-parameters.json` 是否一致。
- 对 `web-ui-param-demo` 检查已构建的 `ui/dist` 资产是否能通过 `AssetManifest::from_dir()` 生成 manifest。
- 可选读取 `--bridge-trace <path>`，接受 readyAck/reply roundtrip 或 param begin/perform/end trace marker。
- 可选读取 `--meter-log <path>`，接受非零 meter stream marker，例如 `meter_flush sent=1`。
- 支持 `--out <path>` 写出 JSON report，schema 包含 `version = 1`、`generator = "vesty-cli.smoke-host.v1"`、`status = ok|partial|failed`、`checks[]` 和 `externalEvidenceNote`。
- 支持 `--check --out <path>` 复验已有 report 是否与当前 workspace 状态一致。
- 支持 `--strict`，任一 failed/skipped check 都会返回非零。
- `--bridge-trace`、`--meter-log`、`--check --out` 以及 example `params.specs.json` 读取都使用 no-follow file 检查；symlinked trace/log/report/specs 会失败而不是跟随到 workspace 外部目标。

示例:

```bash
npm run build --prefix examples/web-ui-param-demo/ui
cargo run -p vesty-cli -- smoke-host --out target/smoke-host/smoke-host.json
cargo run -p vesty-cli -- smoke-host --out target/smoke-host/smoke-host.json --check
```

严格模式可传入显式桥接和 meter 日志:

```bash
mkdir -p target/smoke-host
printf '%s\n' '{"type":"param.begin","result":0}' '{"type":"param.perform","result":0}' '{"type":"param.end","result":0}' 'result=0' > target/smoke-host/bridge-trace.log
printf '%s\n' 'meter_flush sent=1' > target/smoke-host/meter.log
cargo run -p vesty-cli -- smoke-host \
  --bridge-trace target/smoke-host/bridge-trace.log \
  --meter-log target/smoke-host/meter.log \
  --out target/smoke-host/smoke-host.json \
  --strict
```

边界:

- `vesty smoke-host` 不加载 `.dylib` / `.dll` / `.so`，也不执行插件二进制 metadata introspection。
- 它不替代真实 DAW smoke、macOS/Windows/Linux platform WebView smoke、Steinberg validator passed report、签名 verification 或 notarization/stapling evidence。
- CI 上传的 `vesty-smoke-host` artifact 只作为诊断留档，不进入 `release-evidence import-ci` 或 final release gate。

### `vesty export-types`

职责:

- 从 `vesty-ipc`/`vesty-params` 的 Rust protocol structs 生成 TypeScript 类型。
- 同时输出 JSON Schema，用于 JSBridge payload validation、文档和 CI drift check。
- 默认输出到 `target/vesty-protocol`，目录结构为 `typescript/` 和 `json-schema/`。
- `--check` 会导出到临时目录并与 `--out` 指向的 snapshot 做逐文件比较；发现 missing/changed/extra 文件时返回非零，用于 protocol drift gate。

### `vesty vst3-sdk`

职责:

- `vesty vst3-sdk manifest --sdk-dir <official-vst3sdk> --out <path>` 从官方 Steinberg SDK checkout 生成 deterministic SDK header input manifest。
- `--check --out <path>` 复验已有 manifest；header 内容、缺失项、baseline、generator、version 或 SHA-256 漂移时返回非零。
- `vesty vst3-sdk binding-plan --sdk-dir <official-vst3sdk> --bindings-module target/vst3-sdk/generated.rs --out <path>` 生成 generated-bindings readiness report；`--check --out <path>` 复验 SDK headers、output module path、active backend baseline、reserved binding emitter check 和 next steps 是否漂移。
- `vesty vst3-sdk binding-surface --sdk-dir <official-vst3sdk> --out <path>` 生成 generated-bindings symbol surface report；`--check --out <path>` 复验 SDK headers、required symbol/header surface、identifier-token 存在性、active backend baseline 和 audit notes 是否漂移。
- `vesty vst3-sdk emit-scaffold --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated.rs` 生成 deterministic metadata-only Rust module；`--check --out <path>` 会逐字节复验输出漂移。该 module 只固定 generator、header inputs、baseline 和 `BINDINGS_GENERATED = false`，不是完整 SDK 3.8 binding 输出。
- `vesty vst3-sdk emit-abi-seed --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi-seed.rs` 生成 deterministic ABI seed Rust module；`--check --out <path>` 会逐字节复验输出漂移。该 module 只固定基础 VST3 ABI aliases/constants、generator/header/baseline metadata、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`，不是完整 SDK 3.8 COM/API binding 输出。
- `vesty vst3-sdk emit-abi --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-abi.rs` 生成 deterministic foundational ABI layout Rust module；`--check --out <path>` 会逐字节复验输出漂移。该 module 固定基础 aliases/constants 和少量 `#[repr(C)]` layouts，例如 `TUID`、`FUnknownVTable`、`FUnknown`、`ViewRect`、`ProgramListInfo`、`UnitInfo`、`NoteExpressionValueDescription`、`NoteExpressionTypeInfo`、`PhysicalUIMap` 和 `PhysicalUIMapList`，并输出 `ABI_LAYOUT_RECORDS` size/alignment 指纹与 `ABI_FIELD_OFFSETS` 关键字段 offset 指纹；它保持 `ABI_LAYOUT_GENERATED = true`、`BINDINGS_GENERATED = false` 和 `FULL_COM_BINDINGS_GENERATED = false`，不是完整 SDK 3.8 COM/API binding 输出，也不等同完整 ABI 验证。
- `vesty vst3-sdk emit-interface-skeleton --sdk-dir <official-vst3sdk> --out target/vst3-sdk/generated-interface-skeleton.rs` 生成 deterministic interface/vtable skeleton Rust module；`--check --out <path>` 会逐字节复验输出漂移。该 module 固定发现到的 VST3 interface `#[repr(C)]` placeholder、vtable skeleton type、method-surface metadata、per-interface slot-order metadata、signature-intent metadata、vtable-slot-seed metadata、callback-type-alias-seed metadata、vtable-callback-field-layout-seed/vtable-field-offset-fingerprint metadata、upstream `vst3 0.3.0` IID words、per-interface `*_IID` constants、`InterfaceId` records、`QueryInterfaceEntry` planned dispatch records、`iid_from_words()` byte-order helper、`QUERY_INTERFACE_IID_LOOKUP_SCOPE` 以及 `interface_id_for_iid()` / `query_interface_entry_by_interface()` / `query_interface_entry_for_iid()` / `com_object_query_interface_dispatch_by_interface()` / `com_object_query_interface_dispatch_for_iid()` 纯查找 helper、`COM_OBJECT_INTERFACES` object-to-interface exposure records、`COM_OBJECT_IDENTITY_PLANS` object FUnknown identity records、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` per-object dispatch records、`FACTORY_EXPORT_PLAN` factory export plan、`FACTORY_CLASS_PLANS` processor/controller class plans、`MODULE_EXPORT_PLANS` platform module export plans、`BINARY_EXPORT_SYMBOL_PLANS` per-platform binary export symbol plans、`BINARY_EXPORT_INSPECTION_TOOL_PLANS` per-platform inspection tool order，以及 `binary_export_symbol_plan_by_platform_and_symbol()` / `binary_export_inspection_tools()` / `required_binary_export_symbol_count()` / `first_missing_binary_export_symbol()` / `binary_export_required_symbols_present()` 纯 helper，并保持 `INTERFACE_SKELETON_GENERATED = true`、`BINDINGS_GENERATED = false`、`FULL_COM_BINDINGS_GENERATED = false` 和 `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`，不包含 callable `queryInterface` glue、generated factory exports、generated module exports、binary inspection tooling、factory glue、Steinberg method implementation 或完整 COM/API binding。
- 未显式传 `--sdk-dir` 时读取 `VESTY_VST3_SDK_DIR`。
- manifest 锁定后续 generated-headers backend 所需的 required `pluginterfaces` headers、header size、lowercase SHA-256、Steinberg SDK baseline、upstream `vst3` crate baseline、generator 和 missing headers。SDK header probe、manifest generation、binding surface header reads 和 SDK version hint reads 都使用 no-follow regular-file 检查；required header 是 symlink 时 probe 会把它视为 missing，manifest/surface 会 fail closed。
- binding-plan 嵌入同一 header manifest，要求 output module path 指向 `.rs` 文件，并且必须报告 `bindingsGenerated = false`；它是 generated bindings 输入和计划审计证据，不是完整 SDK 3.8 Rust bindings 已生成的声明。
- binding-surface 嵌入同一 header manifest，要求 required symbol surface 完整，且每个 symbol 的 `symbolPresent` 为 true、`missingSymbols` 为空，并且必须报告 `bindingsGenerated = false`；它只锁定 future emitter 的 interface/type/constant 文本 token 审计面，不解析 C++ AST、不验证 ABI、不生成 Rust bindings。
- ABI seed 嵌入同一 plan/surface readiness 结果，只输出基础 aliases/constants 和 metadata；它必须保持 `BINDINGS_GENERATED = false` 与 `FULL_COM_BINDINGS_GENERATED = false`，不能被当作完整 COM/API bindings。
- ABI layout 嵌入同一 plan/surface readiness 结果，输出少量基础 aliases/constants 和 `repr(C)` layout；当前覆盖基础 COM identity/editor rect、program/unit 和 Note Expression 数据结构，并带 `ABI_LAYOUT_RECORDS` size/alignment 与 `ABI_FIELD_OFFSETS` field-offset 指纹。它必须保持 `ABI_LAYOUT_GENERATED = true`、`BINDINGS_GENERATED = false` 与 `FULL_COM_BINDINGS_GENERATED = false`，不能被当作完整 COM/API bindings 或完整 ABI 验证。
- interface skeleton 嵌入同一 plan/surface readiness 结果，输出 interface/vtable `repr(C)` placeholder 和 method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata；slot 只表示接口内审计顺序，signature 只表示方法签名意图，vtable slot seed、callback type alias seed、field layout seed、offset fingerprint、IID records、queryInterface planned dispatch entries、`QUERY_INTERFACE_IID_LOOKUP_SCOPE` 与纯查找 helper、`COM_OBJECT_INTERFACES` records、`COM_OBJECT_IDENTITY_PLANS` records、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES` records、`FACTORY_EXPORT_PLAN` / `FACTORY_CLASS_PLANS` records、`MODULE_EXPORT_PLANS` records、`BINARY_EXPORT_SYMBOL_PLANS` / `BINARY_EXPORT_INSPECTION_TOOL_PLANS` records、binary export required-symbol helpers 和 `binary_export_inspection_tools()` 只固定 future emitter 的 local slot、field name、callback type name、签名意图、repr(C) callback field layout、Rust 侧 `offset_of!` 指纹、upstream IID words、future interface dispatch lookup seed、当前 Vesty adapter object-to-interface exposure plan、object FUnknown identity plan、per-object dispatch plan、current factory/class export plan、current `export_vst3!` platform entry symbol plan 和 future binary inspection expected symbol/tool spelling + required-symbol 判定。它必须保持 `INTERFACE_SKELETON_GENERATED = true`、`BINDINGS_GENERATED = false`、`FULL_COM_BINDINGS_GENERATED = false` 与 `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`，不能被当作 callable `queryInterface` glue、generated factory exports、generated module exports、binary inspection tooling、factory glue、method implementation、完整 COM/API bindings 或完整 ABI 验证。
- 这些文件是 reserved generated-headers backend 的审计证据；当前 active backend 仍是 upstream `vst3` crate，因此缺失 manifest/plan/surface/scaffold/ABI seed/ABI layout/interface skeleton 不会阻止 release-check。若显式提供或被 evidence 目录自动发现，则必须完整有效。
- release evidence 目录中的约定路径是 `vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json`、`vst3-sdk/generated-bindings-surface.json`、`vst3-sdk/generated.rs`、`vst3-sdk/generated-abi-seed.rs`、`vst3-sdk/generated-abi.rs` 和 `vst3-sdk/generated-interface-skeleton.rs`，也可用根目录 `vst3-sdk-headers.json` / `generated-bindings-plan.json` / `generated-bindings-surface.json` 作为兼容 JSON 输入。

### `vesty doctor`

职责:

- 检查 Rust target。
- 检查 VST3 binding baseline: Steinberg SDK baseline、upstream `vst3` crate baseline 和当前 binding backend。
- 检查 `VESTY_VST3_SDK_DIR` 指向的官方 SDK checkout 是否包含 generated headers 后备路径所需的关键 `pluginterfaces` headers；未设置时显示为 skipped，因为当前默认 backend 仍是 upstream `vst3` crate。
- 检查 VST3 validator。
- 检查 WebView runtime: WebView2/WebKitGTK/WebKit。
- 检查 Node/package manager。
- 检查 release signing/notarization 前置工具: macOS `codesign` / `xcrun notarytool`、Windows SDK `signtool.exe`，Linux 标记为 release-channel policy。
- 检查常见 DAW 安装路径: REAPER、Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One。安装检测只说明 host 可能存在，不会替代真实 smoke evidence。
- 支持 `--format text|json`，JSON report 以 `checks[]` 输出每项的 `status`、`value` 和可选修复提示。
- 签名工具检查只是预检，不替代真实签名、公证、安装包签名或 CI artifact 证明。
- 签名 evidence 会检查路径平台 token 与内容平台一致。文件名或单独父目录明确表达 `macOS` / `Windows` 时，内容必须分别证明 `codesign` / `signtool`；错放日志或 macOS signed `.vst3` bundle 到 `Windows/` 目录会被 `import-ci` 和最终 release gate 拒绝。
- Notarization/stapler evidence 是 macOS-only。路径明确表达 `Windows` 或 `Linux`，或同时表达多个平台时，即使内容含 accepted/stapled marker，也会被 `import-ci` 和最终 release gate 拒绝。

### `vesty daw-matrix`

职责:

- 汇总 REAPER、Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One 的 smoke evidence。
- 支持 `--format markdown|json`。
- 支持 `--write-template` 生成外部 DAW 验收证据目录，模板使用 pending 值且不会覆盖已有日志；如果 evidence root、标准模板文件、host 目录槽位或创建路径中的已存在祖先目录是 symlink，则会拒绝写入而不是跟随到外部目标。
- 支持 `--evidence-root <dir>` 使用统一根目录，按 `<dir>/reaper`、`<dir>/cubase`、`<dir>/bitwig`、`<dir>/ableton`、`<dir>/studio-one` 读取或写入 host evidence；不传时保留五个默认/分项路径。
- `--write-template --evidence-root <dir>` 会为每个 host 创建 `README.md`、`platform.txt`、`scan-smoke.log`、`load-smoke.log`、`ui-smoke.log`、`ui-host-smoke.log`、`meter-stream.log`、`automation-smoke.log`、`buffer-sample-rate.log`、`restore-smoke.log` 和 `offline-render.log`。
- 支持 `--write-report --host <reaper|cubase|bitwig|ableton|studio-one>` 在真实 DAW smoke 完成后写入规范 evidence 文件；调用者必须显式传入 `--platform`、`--scan`、`--load`、`--ui`、`--ui-host-param`、`--meter-stream`、`--automation`、`--buffer-sample-rate-change`、`--save-restore` 和 `--offline-render`。`--platform` 必须能映射到对应 host profile 的 supported platform；Linux Wayland、泛 Linux 但无 X11、或 host profile 未声明的平台组合会在写入前失败。写入前会拒绝 pending/false/占位值、`manual platform pending`、zero meter、明显负面 marker（如 failed/not found/unavailable/crashed）和无法被 matrix parser 识别的模糊 marker，避免留下半套无效 evidence；写入后仍会复用 matrix parser 验证该 host 行完整。
- `release-check` report 的 `daw_matrix` 行只允许当前 `vesty-core::host_profiles()` 中的 release host；未知 host、拼写错误或额外第三方 host 行会在 report shape validation 阶段失败。第三方 DAW 兼容性可作为额外文档记录，但不应混入 MVP release gate 的五个 host matrix。
- 读取 host evidence 时会拒绝 symlinked evidence root / host dir；读取 marker 时同样使用 no-follow file 检查，`platform.txt` 或任一 `*-smoke.log` 是 symlink 时不会跟随外部目标。缺失/被拒绝的目录或 marker 会按未通过处理，`platform.txt` 会退回 `manual evidence` 或 missing row 的 `manual matrix pending`。已读取的 `platform.txt` 也会重新映射到对应 host profile 支持的平台；Wayland、泛 Linux 但无 X11、或 Ableton Live + Linux X11 这类 profile 未支持组合会把该行 `platform_supported` 置为 false，Markdown `Platform` 列显示 `(unsupported)`，`--strict` 与 `release-check` 都会把它当作缺失 `platform` evidence，而不是只看其它 smoke marker 通过。`release-check` 写出或导入的 JSON report 也会复验 `daw_matrix[].platform_supported` 与 `daw_matrix[].platform` 是否一致，拒绝 `platform_supported=true` 但平台文本实际不受 host 支持、或反向矛盾的手工编辑报告。
- 如果 marker log 显式带有 `host=...`、`daw=...`、`daw_host=...`、`host_profile=...` 或 `profile=...`，读取端会把该值映射到内置 host profile 并要求它匹配当前 evidence 目录；例如 Ableton Live 日志被放进 Bitwig 目录时，对应 smoke marker 会按未通过处理。没有这些显式 host scope 字段的旧日志格式仍保持兼容。
- 生成的每个 host `README.md` 会列出 accepted pass markers，例如 `scan=true`、`ui_host_param=true`、`meter_flush sent=1`、`buffer_sample_rate_change=true`、`offline_render=true` 和 `render_file=/absolute/path/to/rendered.wav`，并包含 `vesty daw-matrix --evidence-root ... --strict` 验证命令。
- `offline-render.log` / REAPER `render-smoke.log` 可用显式 marker `offline_render=true|pass|ok` / `render=true|pass|ok` / `render_ok=true|pass` 证明；也可写 `render_file=/absolute/path.wav`、`render_file = "rendered.wav"` 或 `render_file='rendered.wav'`。绝对路径必须指向一个已存在、非 symlink 且非空的渲染文件；相对路径按当前 host evidence 目录解析，且不允许 `..` 父目录跳转。
- 支持 `--strict`，任意 host/check 未通过时在打印矩阵后返回非零，用于 release/CI gate。

### `vesty host-quirks`

职责:

- 输出 Vesty 内置 host profile/quirk registry。
- 支持 `--format markdown|json`。
- 支持 `--host <alias>` 过滤，例如 `--host bitwig`、`--host cubase`、`--host live`。
- 每个 profile 包含目标平台、release 必需 smoke checks、quirk area/severity、summary 和 mitigation。
- 该命令只提供兼容性注意事项和验证计划，不代表对应 DAW 已经通过 smoke。

### `vesty platform-smoke`

职责:

- 汇总系统 WebView/VST3 editor 基础平台 smoke evidence。
- 支持 `--format markdown|json|text`。
- 支持 `--write-template --dir <dir>` 生成 `macos.json`、`windows-x64.json`、`linux-x11.json` pending 模板和 `README.md`；模板不会覆盖已有文件，且会拒绝既有 symlink 模板文件、目录槽位或创建路径中的已存在 symlink 祖先目录；pending 模板不会被当作平台通过证据，也不会把普通 optional release-check 从 `skipped` 变成 `failed`。
- 支持 `--write-report --dir <dir> --platform <macos|windows-x64|linux-x11>` 从显式 marker 写入规范 JSON report；调用者必须提供 `--system-webview`、`--vst3-validator`、`--vst3-example-scan`、`--webview-attach`、`--webview-resize`、`--asset-protocol`、`--jsbridge-roundtrip` 和 `--meter-stream`。写入前会复用同一套 platform smoke validator，拒绝 pending/false/占位值、Linux Wayland 和 zero meter evidence；`system_webview=true` / `vst3_validator=true` 这类泛 marker 不算发布证据。输出目录创建也会拒绝既有 symlink 祖先，避免规范化后的平台 smoke JSON 被写到 evidence bundle 外部。
- 支持 `--check --dir <dir>` 检查已有平台 smoke evidence。
- 支持 `--strict`，任意缺失或无效 evidence 会在打印报告后返回非零。
- 每个平台 report 必须包含 `system_webview`、`vst3_validator`、`vst3_example_scan`、`webview_attach`、`webview_resize`、`asset_protocol`、`jsbridge_roundtrip` 和 `meter_stream`。其中 `system_webview` 必须是平台特异证据: macOS 提到 `WebKit.framework` 或 `WKWebView`，Windows x64 提到 `WebView2`，Linux X11 同时提到 `WebKitGTK` 和 active `X11`，且不能包含 Wayland/fallback/not-X11 这类否定或实验路径描述；`vst3_validator` 必须识别 Steinberg/VST3 validator 输出，并包含 passed tests 与 0 failed 语义；meter 必须证明至少有非零帧/批次送达，不能只写 pending/false/0。
- 最终 release gate 要求 macOS、Windows x64 和 Linux X11 三份真实 report。Linux Wayland 仍是 experimental，不进入首版 release platform-smoke gate。

### `vesty release-check`

职责:

- 聚合 release readiness 报告。
- 复用 `daw-matrix` evidence 判定，逐 host 输出缺失 smoke 项。
- 支持 `--evidence-root <dir>` 与 `daw-matrix` 相同的 host evidence 根目录约定。
- 运行 protocol snapshot drift check，默认检查 `target/vesty-protocol`，可用 `--protocol-snapshot <path>` 指定；失败时 structured report 的 `protocol snapshot` 项会列出 missing/changed/extra 的相对文件路径摘要，CI artifact 不需要再单独翻 stderr 才能定位 drift。
- 记录 VST3 binding baseline，包含 Steinberg SDK baseline、upstream `vst3` crate baseline 和当前 binding backend。
- 支持 `--skip-protocol`，但只用于本地临时检查或 per-OS CI release-check snapshot；最终 `--strict --require-release-artifacts` gate 会拒绝该参数，必须提供并检查真实 `--protocol-snapshot`。
- 可选检查 GitHub Actions run URL: `--ci-run-url` 必须指向精确的 `https://github.com/<org>/<repo>/actions/runs/<numeric-id>` run 页面，可带 `/attempts/<n>`、query 或 fragment；普通 Actions 页面、job URL、非数字 run id 或非 HTTPS URL 不会被当作 release evidence。当 `release-check` 同时收到显式 `--ci-run-url` 和 `--release-evidence-dir/ci-run-url.txt` 时，两者必须有效且指向同一个 repo/run id，同一 run 的不同 attempt 允许通过，避免命令行参数静默覆盖 evidence 目录里的 provenance；`ci-run-url.txt` 必须是普通文件，不能是 symlink。
- 可选检查 CI doctor artifacts: `--ci-doctor-dir` 会递归读取 doctor JSON，OS 可从文件名或父目录路径推断，因此 `doctor-Linux.json` / `doctor-macOS.json` / `doctor-Windows.json` 和 `Linux/doctor.json` / `macOS/doctor.json` / `Windows/doctor.json` 这类目录分组下载都可覆盖三平台；显式传入的文件/目录 root 不能是 symlink。它验证每个平台 report 至少包含 toolchain、Node/npm、VST3 binding baseline、VST3 SDK headers probe、WebView、validator 和对应 signing/notarization 前置检查，并要求关键 check 状态为 `ok`；`vst3 SDK headers` 可为 `skipped`，因为默认 upstream `vst3` backend 不要求 SDK checkout；Linux `signing: linux release policy` 可为 `unknown`，因为它只是发布策略记录而不是系统签名工具。GitHub Actions 中生成的 doctor JSON 会带可选 `ci_run_url`，当 `release-check` 同时收到 `--ci-run-url` / release evidence `ci-run-url.txt` 时，会拒绝 run id 或 repo 不一致的新 doctor artifact；旧 doctor JSON 没有该字段时保持兼容。
- 可选检查 CI per-OS release-check artifacts: `--ci-release-check-dir` 会递归读取大小写不敏感的 `release-check*.json`，OS 可从文件名或父目录路径推断，因此 `release-check-Linux.json` / `release-check-macOS.json` / `release-check-Windows.json` 和 `Linux/release-check.json` / `macOS/release-check.json` / `Windows/release-check.json` 这类目录分组下载都可覆盖三平台；显式传入的文件/目录 root 不能是 symlink。它确认三平台 report 可解析，并且 host profile coverage、protocol snapshot skip/ok、VST3 binding baseline 等本地 invariant 通过。该 gate 会拒绝重复 check name、未知 check status、顶层 status 与内部 failed checks 不一致、伪造的 host profile coverage 数量、非 `--skip-protocol` 形态的 protocol skip，以及缺少当前 Steinberg SDK baseline / upstream `vst3` crate baseline / binding backend 的 VST3 binding baseline 值。目录中同名 CI artifact 附带的 `release-action-plan-*.json` 会被忽略，因为 action plan 只是采证清单，不是 per-OS release-check report。当 `release-check` 同时收到 `--ci-run-url` / release evidence `ci-run-url.txt` 时，还会要求每个 snapshot 的 `ci_run_url` 指向同一个 GitHub repo/run id。该 gate 允许 DAW smoke、validator、signing、notarization 等外部证据仍显示 failed，因为这些缺口由独立 gate 负责；它只证明每个 runner 真的跑过 release-check 快照且本地 invariant 没坏。这里的 `protocol snapshot skip` 只允许出现在 per-OS runner snapshot 中，不能替代最终 consolidated release gate 的 protocol drift check。
- 可选检查平台 smoke artifacts: `--platform-smoke-dir <dir>` 会递归读取 `vesty platform-smoke --format json` 风格的 JSON report，且显式传入的文件/目录 root 不能是 symlink；普通检查允许局部平台覆盖并给出 missing hint；如果 artifact path 带有平台 token，例如 `macOS/platform-smoke.json`、`Windows/platform-smoke.json` 或 `Linux-X11/platform-smoke.json`，该 token 必须与 report 内 `platform` 字段一致，避免错放 artifact 污染平台覆盖。若路径同时带有多个平台 token，例如 `macos-windows/` 或 `linux-x11-windows/`，会作为 ambiguous evidence 拒绝。Linux 路径只有同时出现 `linux` 和 `x11` token 才会被当作 final Linux X11 evidence；Linux Wayland 仍不满足最终 gate。`--require-release-artifacts` 会要求 macOS、Windows x64 和 Linux X11 三份真实 evidence，Linux Wayland report 会被拒绝。pending 模板不会被当作 pass evidence。
- 可选自动发现 release evidence 目录: `--release-evidence-dir <dir>` 会先要求 `<dir>` 是真实目录且不是 symlink，再按模板约定读取 `ci-run-url.txt`、`ci-doctor/`、`ci-release-checks/`、`platform-smoke/`、`publish-plan/publish-plan.json`、`crate-package/crate-package.json`、`npm-pack/npm-pack.json`、`dependency-baseline/dependency-baseline-latest.json`、`vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json`、`vst3-sdk/generated-bindings-surface.json`、`vst3-sdk/generated.rs`、`vst3-sdk/generated-abi-seed.rs`、`vst3-sdk/generated-abi.rs`、`vst3-sdk/generated-interface-skeleton.rs`、`validate-report.json`、`static-validate-report.json`、`signing-macos.log`、`signing-windows.log` 和 `notary.log`；`ci-run-url.txt` 只接受裸 GitHub Actions run URL，或 `ci_run_url=` / `ci-run-url=` key，忽略其它 `key=value` 说明行和 `pending`，且该文件不能是 symlink。模板约定中的标准 evidence 文件和中间目录也会用 `symlink_metadata()` 检查并拒绝 symlink，包括 `publish-plan/`、`crate-package/`、`npm-pack/`、`dependency-baseline/`、`vst3-sdk/`、validate/static validate JSON、签名日志和 `notary.log`，避免 evidence bundle 的标准槽位指向目录外可替换文件。其中 crate package readiness 在普通本地检查中缺失保持 `skipped`，但在 `--require-release-artifacts` 下必须存在，存在时要求无内部依赖的 crate 为 `packaged`、有内部依赖的 crate 为 `deferred`；dependency latest baseline 只接受同时包含 `cargo workspace external dependency baseline coverage` 和所有 crates.io/npm registry latest checks 的 `dependency-baseline-latest.json`，普通离线 `dependency-baseline.json` 不会满足最终 release gate。标准 validate/static validate JSON、signing 日志和 `notary.log` 的 pending 模板不会把 optional release-check 从 skipped 变成 failed；如果这些标准槽位被替换成非 pending 的 malformed/invalid 内容，路径会被保留并由对应 gate 报出具体诊断，而不是静默降级成 generic missing evidence；`platform-smoke/` 也只有存在非 pending report 或无法解析的 JSON 时才会自动启用，单独生成的 pending 平台 smoke 模板不会把 optional release-check 从 skipped 变成 failed。VST3 SDK header manifest、generated-bindings plan、generated-bindings surface 和四个 SDK `.rs` 审计文件只有约定文件存在时才会启用，缺失保持 `skipped`；显式传入或自动发现后会严格校验内容完整性，且 `.rs` 文件仍只证明 drift/audit metadata 有效，不证明完整 SDK bindings 已生成。它同时会递归扫描目录内其它 `*.json`，按 `ValidateReport` 内容把 validator-passed reports 归入 release validate evidence，把 static-only/skipped reports 归入 CI static validate smoke。它还会递归扫描 `.log` / `.txt` 文件和 macOS `.vst3` bundle，按内容自动发现 codesign/signtool 签名证据和 accepted notarization/stapler 证据；递归自动发现只接收内容校验通过的签名/公证证据，避免普通 README/notes 文本制造失败项；标准槽位中的非 pending 无效签名/公证日志则会保留用于 release-check 诊断。macOS `.vst3` 目录必须包含可解析的 `Contents/_CodeSignature/CodeResources` plist，且 `Contents`、`Contents/_CodeSignature` 和 `CodeResources` 都不能是 symlink，plist 中还必须有 `files` 或 `files2` dictionary 条目，单纯占位文件或字符串值不算签名证据。显式传入的单项参数仍优先并保持严格校验。
- 可选采集本地 release evidence: `vesty release-evidence collect-local --dir <dir> --protocol-snapshot <path>` 会写入 release evidence 模板、重新导出并检查 protocol snapshot、生成 `publish-plan/publish-plan.json` 和 `npm-pack/npm-pack.json`，同时写入 `local-collect-report.json`。输出 evidence dir 创建前会拒绝既有 symlink 祖先，避免本地规范化 evidence 写入外部可替换目录。如果显式传入 `--crate-package`，还会运行真实 `vesty crate-package` 并写入/复验 `crate-package/crate-package.json`；该步骤会执行多个 `cargo package -p <crate> --allow-dirty --no-verify`，因此默认不跑。如果显式传入 `--dependency-baseline-latest`，会联网生成并立即复验 `dependency-baseline/dependency-baseline-latest.json`；默认不联网。如果显式传入 `--vst3-sdk-dir <official-vst3sdk>`，还会生成并立即复验 `vst3-sdk/vst3-sdk-headers.json`、`vst3-sdk/generated-bindings-plan.json`、`vst3-sdk/generated-bindings-surface.json`、metadata-only `vst3-sdk/generated.rs` scaffold、ABI seed `vst3-sdk/generated-abi-seed.rs`、ABI layout `vst3-sdk/generated-abi.rs` 和 interface/vtable skeleton `vst3-sdk/generated-interface-skeleton.rs`；`--vst3-sdk-bindings-module <path>` 可覆盖默认 `.rs` output module path。该 helper 只运行本机真实命令、显式请求的 crate package readiness、显式请求的 dependency latest review 和显式请求的 VST3 SDK 审计，不会生成 DAW matrix、platform smoke、Steinberg validator-passed、真实 CI、签名或 notarization 证据；这些外部证据仍必须独立采集。
- 可选导入已下载 CI artifacts: `vesty release-evidence import-ci --source <downloaded-artifacts-dir> --dir <release-evidence-dir> --ci-run-url <GitHub Actions run URL>` 会递归扫描真实下载目录，按内容识别 doctor、per-OS release-check、per-OS release action plan sidecar、protocol snapshot、publish-plan、crate-package、npm-pack、同时带 workspace external dependency coverage 和 registry latest checks 的 dependency baseline、VST3 SDK header manifest、VST3 SDK generated-bindings plan、VST3 SDK generated-bindings surface、VST3 SDK metadata-only `generated.rs` scaffold、VST3 SDK ABI seed `generated-abi-seed.rs`、VST3 SDK ABI layout `generated-abi.rs`、VST3 SDK interface skeleton `generated-interface-skeleton.rs`、platform smoke、validator/static validate、签名和 notarization artifacts，并只把通过对应 parser/validator 的内容复制到标准 release evidence 目录；`--source` 必须是已存在的真实目录且不能是 symlink，`--source` 和 `--dir` 不能相同，也不能互相嵌套，避免导入时扫描或覆盖自身输出；如果 `--dir` 已存在则不能是 symlink，如果缺失则只会通过真实父目录创建，symlinked output parent 会被拒绝；如果同时传入 `--ci-run-url` 和 `--ci-run-url-file`，两者都必须是有效 GitHub Actions run URL 且指向同一个 repo/run id，同一 run 的不同 `/attempts/<n>` 允许通过；默认保留已有文件，`--overwrite` 才会替换，且 overwrite 遇到目标 symlink 时只 unlink symlink 本身，不跟随删除外部目标；导入写入/复制前还会逐段拒绝 symlinked destination parent，避免标准 evidence 子目录被替换到 bundle 外部。无效 crate package report 会记录为 failed 且不会复制；无效 release action plan sidecar 会记录为 failed 且不会复制；离线 `dependency-baseline.json` 或缺少 workspace coverage 的 latest report 会被记录为 failed，不会复制为最终 dependency latest evidence。有效 per-OS action plan sidecar 会复制到 `ci-release-checks/release-action-plan-<OS>.json`，只用于人工采证追踪，不作为 release-check pass evidence；有效 scaffold 会复制到 `vst3-sdk/generated.rs`，有效 ABI seed 会复制到 `vst3-sdk/generated-abi-seed.rs`，有效 ABI layout 会复制到 `vst3-sdk/generated-abi.rs`，有效 interface skeleton 会复制到 `vst3-sdk/generated-interface-skeleton.rs`，四者会被 `release-check` 作为可选 SDK audit 项严格校验，但只证明 drift/audit metadata 有效，不证明完整 SDK bindings 或最终 release readiness。该 helper 写入 `import-ci-report.json`，记录 imported/skipped/failed 项；报告本身不是 pass evidence，也不会生成 DAW、validator、签名或公证通过证据。
- `release-evidence import-ci` 导入 platform smoke report 时也会检查 artifact path 中的明确平台 token: `Windows/platform-smoke.json` 不能携带 `platform = "macos"` 的 report，`Linux-X11/platform-smoke.json` 必须携带 Linux X11 report；不一致会记录为 `platform smoke artifact` failed item，并且不会复制到 `platform-smoke/<platform>.json`。带有多个平台 token 的路径会被视为 ambiguous 并拒绝导入。Linux 路径推断仍要求同时出现 `linux` 和 `x11` token，避免 Wayland 或泛 Linux artifact 被误导入为最终 X11 evidence。
- `release-evidence import-ci` 导入 VST3 validator/static validate report 时也会检查明确平台路径。文件名或单独父目录如果表达 `macos`、`windows-x64` 或 `linux-x64`，report 的 `static_check.binaries` 必须包含对应平台；否则会记录 `vst3 validate report` 或 `vst3 static validate report` failed item，并且不会复制到 `validator/` 或 `package/`。文件名同时表达多个平台也会失败，避免 `VestyGain.macos.windows-x64.validate.json` 这类歧义 artifact 被静默导入。宽泛 CI job 目录名例如 `linux-vst3-static-validate/` 不作为强平台 token，避免误拒一个 job 下载目录里包含的多平台矩阵 artifacts。
- 可选采集签名和公证 release evidence: `vesty release-evidence collect-signing <bundle.vst3> --platform macos|windows-x64 --dir <dir>` 会运行真实 `codesign --verify --deep --strict --verbose=2` 或 `signtool verify /pa /v`，捕获 stdout/stderr，并且只有输出被签名 evidence parser 接受后才写入 `signing-macos.log` / `signing-windows.log`；输入 bundle root、自动推断用的 `Contents/x86_64-win` payload dir 和显式 `--binary` 都必须是真实目录/文件而不是 symlink，`--binary` 只支持 Windows 采证，macOS 采证固定验证整个 `.vst3` bundle 并会拒绝该参数；显式 Windows `--binary` 还必须是 canonical path 位于该 bundle `Contents/x86_64-win` 内的 `.vst3` 文件，避免把外部 signed binary 的验证结果挂到另一个 bundle 上；显式 `--tool <path>` 也会在构造 `codesign` / `signtool` 验证命令前拒绝 symlinked leaf 或 symlinked parent path，默认工具查找仍保留 bare command fallback；输出 evidence dir 创建前也会拒绝既有 symlink 祖先。Linux 签名仍是 release-channel policy，不由该 helper 伪造。`vesty release-evidence collect-notarization --notary-log <log> --stapler-log <log> --dir <dir>` 会合并真实 notarytool/stapler 日志，并且只有同时证明 accepted notarytool result 和 stapler success 后才写入 `notary.log`；该输出 evidence dir 同样不允许经由 symlink 父目录创建。
- 可选生成/检查 crate publish plan: `vesty publish-plan --out <path>` 从 Cargo metadata 写入规范 JSON，并立即复用 release gate 的发布顺序校验；`vesty publish-plan --check --out <path>` 只复验已有 report。`release-check --publish-plan-report <path>` 验证 package order 从 1 开始连续、level 非零、package name/order 唯一、内部依赖引用存在，并且每个内部依赖都排在 dependent 之前且 level 更低；显式 report 文件不能是 symlink。普通检查缺失时为 `skipped`；`--require-release-artifacts` 会要求该 evidence 存在并有效。
- CLI 写出 JSON/text report 的通用路径会在覆盖前拒绝既有 symlink 输出文件，并逐段拒绝用户可替换的 symlink 输出父目录（允许 macOS `/var`/`/tmp` 这类 root-level 系统前缀），覆盖 `publish-plan --out`、`crate-package --out`、`npm-pack --out`、`dependency-baseline --out`、`validate --report`、`validate --validator-log`、`release-check --report` / `--plan`、`smoke-host --out`、`platform-smoke --write-report` 和 DAW smoke marker 写入等路径；`vesty new`、`collect-local`、`collect-signing`、`collect-notarization` 和 `platform-smoke --write-report` 的目录创建入口也会拒绝既有 symlink 输出祖先。CLI 通用 TOML/JSON 输入 reader、`smoke-host --bridge-trace` / `--meter-log` / `--check --out`、dependency baseline workspace/package inputs，以及 `collect-signing` 的 bundle/payload/binary/tool 输入也会拒绝 symlinked files/directories；`collect-signing --platform windows-x64 --binary` 还会拒绝 bundle 外文件、非 `.vst3` 文件，以及通过 payload 子目录 symlink 跳出 bundle 的路径；显式 `collect-signing --tool` 会拒绝 symlinked tool leaf 和 symlinked parent path。evidence template 初始化路径同样拒绝既有 symlink 文件、目录槽位和创建路径中的已存在 symlink 祖先目录。这样本地采证不会把报告或 pending 模板写到 symlink 指向的 evidence bundle 外部文件/目录，也不会从可替换外部文件读取本地 release/diagnostic evidence。
- 可选生成/检查 npm package dry-run report: `vesty npm-pack --out <path>` 会运行 `npm pack --workspaces --dry-run --json`，写入规范 JSON，并立即复用 release gate 的包边界校验；`vesty npm-pack --check --out <path>` 只复验已有 report。`release-check --npm-pack-report <path>` 要求 `@vesty/plugin-ui`、`@vesty/react`、`@vesty/vue` 和 `@vesty/svelte` 全部存在，且 packed files 只包含 `dist/**` 和 `package.json`；显式 report 文件不能是 symlink。普通检查缺失时为 `skipped`；`--require-release-artifacts` 会要求该 evidence 存在并有效，避免 npm package 边界退化后仍宣称 release-ready。
- 可选检查 VST3 SDK header manifest: `--vst3-sdk-manifest <path>` 验证 `vesty vst3-sdk manifest` 生成的 JSON，检查 manifest version/generator、Steinberg SDK baseline、upstream `vst3` crate baseline、required header set、duplicates/unexpected headers、`missingHeaders` complement、`complete`、非零 size 和 lowercase SHA-256 shape；显式 manifest 文件不能是 symlink。缺失 manifest 即使在 `--require-release-artifacts` 下也保持 `skipped`，因为当前 active backend 仍是 upstream `vst3` crate；但只要显式提供或被 `--release-evidence-dir` 自动发现，无效或 incomplete manifest 就会使 release-check 失败。
- 可选检查 VST3 SDK generated-bindings plan: `--vst3-sdk-binding-plan <path>` 验证 `vesty vst3-sdk binding-plan` 生成的 JSON，检查 plan version/generator、`bindingsGenerated = false`、`status = ready-for-binding-generator`、无 blockers、SDK/crate baseline、active backend、`.rs` module path、embedded header manifest 完整性、reserved binding emitter check 和 next steps；显式 plan 文件不能是 symlink。缺失 plan 即使在 `--require-release-artifacts` 下也保持 `skipped`；但只要显式提供或被 `--release-evidence-dir` 自动发现，无效、blocked 或声称已生成 bindings 的 plan 就会使 release-check 失败。
- 可选检查 VST3 SDK generated-bindings surface: `--vst3-sdk-binding-surface <path>` 验证 `vesty vst3-sdk binding-surface` 生成的 JSON，检查 surface version/generator、`bindingsGenerated = false`、`status = ready-for-binding-emitter`、无 blockers、SDK/crate baseline、active backend、embedded header manifest 完整性、required header set、required symbols、symbol/header mapping、`symbolPresent = true`、`missingSymbols = []` 和 audit notes；显式 surface 文件不能是 symlink。缺失 surface 即使在 `--require-release-artifacts` 下也保持 `skipped`；但只要显式提供或被 `--release-evidence-dir` 自动发现，无效、blocked、缺失 symbol token 或声称已生成 bindings 的 surface 就会使 release-check 失败。
- 可选检查 VST3 SDK generated Rust audit artifacts: `--vst3-sdk-scaffold <path>`、`--vst3-sdk-abi-seed <path>`、`--vst3-sdk-abi <path>` 和 `--vst3-sdk-interface-skeleton <path>` 分别验证 `emit-scaffold` / `emit-abi-seed` / `emit-abi` / `emit-interface-skeleton` 生成的 `.rs` 审计文件；显式 `.rs` 文件不能是 symlink。缺失这些文件即使在 `--require-release-artifacts` 下也保持 `skipped`；但只要显式提供或被 `--release-evidence-dir/vst3-sdk/*.rs` 自动发现，marker、baseline、layout fingerprint、interface metadata 或 generated/full-COM flags 不合法都会使 release-check 失败。这些检查是 drift/audit strictness，不表示完整 SDK 3.8 bindings 已生成。
- 可选检查 VST3 validate report: `--validate-report <path>` 可重复传入 `vesty validate --strict --report <path>` 生成的 JSON，显式 report 文件不能是 symlink；要求 `static_check.status = ok`、包含 moduleinfo/binary，若包含 `static_check.binary_exports` 则每条 check 必须平台、required symbols、found symbols、status 和 skipped reason 自洽，并且 `validator.status = passed`、`exit_code = 0`、`tests_passed > 0`、`tests_failed = 0`。报告会从 binary paths 推断并显示 macOS / Windows x64 / Linux x64 platform coverage；新报告还会写入 `static_check.binary_exports`，记录 `nm` / `llvm-nm` / `llvm-objdump` / `dumpbin` 可观察到的 VST3 导出符号，工具缺失或无法解析时记录 skipped 而不是伪造 pass；release validator evidence 推荐始终使用 `--strict`，让 skipped export-symbol evidence 在采集阶段就失败。
- `release-check` 会从 validator-passed reports 推断 Vesty 三个示例插件的 Steinberg validator 覆盖: `VestyGain.vst3`、`VestyWebUIDemo.vst3`、`VestyMIDISynth.vst3` 乘以 `linux-x64`、`macos`、`windows-x64`。普通本地检查允许部分覆盖并给出 hint；`--require-release-artifacts` 会要求完整 3x3 validator 覆盖。三个内置示例的 report 都必须包含指向对应 bundle `Contents/Resources/parameters.manifest.json` 的 `static_check.parameter_manifest`，证明参数 sidecar 已进入 packaged `.vst3`；`VestyWebUIDemo.vst3` 的示例覆盖还要求报告包含指向 `Contents/Resources/assets.manifest.json` 的 UI `asset_manifest` 且 `asset_count > 0`；最终 strict gate 还要求每个示例/platform report 含匹配平台的 `ok` `static_check.binary_exports`，证明 `GetPluginFactory` 和对应平台 entry/exit 导出符号已被真实工具观察到。`release-check --plan` 和 evidence template 都推荐 `vesty validate --strict` 来生成这些 validator-passed reports。
- 可选检查 CI 静态 validate report: `--static-validate-report <path>` 可重复传入 `vesty validate --static-only --report <path>` 生成的 JSON，显式 report 文件不能是 symlink；只要求静态 bundle check 为 `ok`、包含 moduleinfo/binary，并且显式 `static_check.binary_exports` 自洽，用于证明 CI packaging smoke，不会被当作 release validator 通过证据。报告同样会显示从 static bundle binary paths 推断出的 platform coverage。release CI 推荐用 `vesty validate --static-only --strict --report <path>` 生成该 report，使导出符号工具缺失或 skipped evidence 在 package job 阶段就失败。
- `release-check` 还会从 CI static validate reports 推断 Vesty 三个示例插件的覆盖矩阵: `VestyGain.vst3`、`VestyWebUIDemo.vst3`、`VestyMIDISynth.vst3` 乘以 `linux-x64`、`macos`、`windows-x64`。普通 per-OS package job 只要求当前平台三示例齐全；`--require-release-artifacts` 会要求完整 3x3 覆盖。三个内置示例的 CI static validate coverage 都必须包含指向对应 bundle `Contents/Resources/parameters.manifest.json` 的 `static_check.parameter_manifest`；`VestyWebUIDemo.vst3` 同样要求 UI asset manifest evidence 指向 `Contents/Resources/assets.manifest.json`；最终 strict gate 还要求每个示例/platform static report 含匹配平台的 `ok` `static_check.binary_exports`。`status = "skipped"` 只能说明当前 runner 缺少可用导出符号工具，不能满足最终 binary export evidence 要求。
- 可选检查签名证据: `--signed-bundle-evidence` 可重复传入 codesign/signtool 验证日志，或带可解析 `Contents/_CodeSignature/CodeResources` plist 且含 `files` / `files2` dictionary 条目的 macOS `.vst3` bundle；显式日志文件或 bundle root 不能是 symlink，macOS bundle 证据内部的 `Contents`、`Contents/_CodeSignature` 和 `CodeResources` 也不能是 symlink。普通检查接受局部签名证据；`--require-release-artifacts` 会要求同时覆盖 macOS codesign 和 Windows signtool。泛用 `signed=true` / `signature=ok` marker 只算 supplemental，不可单独证明任一平台。发布流水线中保留 codesign/signtool 验证日志仍是更强证据，因为目录证据只证明 bundle 已有 macOS code signature 资源结构。Windows 自动识别只接受 verification 语义，例如 `signtool verify` 输出里的 `Successfully verified` 或 `Number of errors: 0`，单独的 `Successfully signed` 只证明签名动作完成，不证明验证通过。同一日志里如果出现 invalid signature、非零 `Number of errors`、`SignTool Error` 等明确失败证据，正向 marker 不会覆盖失败。
- 可选检查 notarization 证据: `--notarization-log` 需要包含 accepted notarytool/stapler 证据，显式日志文件不能是 symlink。普通检查接受局部 notarization 证据；`--require-release-artifacts` 会要求同时包含 accepted notarytool 输出和 stapler success。泛用 `notarization=pass` 可证明 notary acceptance，但不能单独证明 stapling。同一日志里如果出现 rejected/invalid notary status、notarytool submission failure 或 stapler failure，正向 marker 不会覆盖失败。
- 支持 `--write-evidence-template <dir>` 生成 release artifact 证据模板，包括 README、CI run URL 占位、`ci-doctor/README.md`、`ci-release-checks/README.md`、`platform-smoke/README.md`、`publish-plan/README.md`、`crate-package/README.md`、`npm-pack/README.md`、`vst3-sdk/README.md`、pending validator validate report、可选 static validate report、签名日志和 notary 日志；模板不会覆盖已有文件，且会拒绝既有 symlink 文件、标准目录槽位或创建路径中的已存在 symlink 祖先目录；pending 值、`ci-doctor/README.md`、`ci-release-checks/README.md`、`platform-smoke/README.md`、`publish-plan/README.md`、`crate-package/README.md`、`npm-pack/README.md` 和 `vst3-sdk/README.md` 不会被判定为 pass。`vst3-sdk/README.md` 同时说明 `vst3-sdk-headers.json`、`generated-bindings-plan.json`、`generated-bindings-surface.json`、`generated.rs` scaffold、`generated-abi-seed.rs` ABI seed、`generated-abi.rs` ABI layout 和 `generated-interface-skeleton.rs` interface/vtable skeleton 的生成命令，但不会创建虚假的 JSON/Rust evidence。需要平台 smoke JSON 模板时可另跑 `vesty platform-smoke --write-template --dir <dir>/platform-smoke`。
- 生成的 release artifact `README.md` 会列出 accepted marker 示例，例如 `signed=true`、`codesign=pass`、`signtool=pass`、`signtool ... Number of errors: 0`、`notarization=pass`、`stapled=true`、`status: Accepted` 和 `The staple and validate action worked!`。轻量 marker 必须是独立的精确 `key=value` 或 `key: value` 行；`pending`、`false`、说明文字、子串命中或同一日志中的矛盾失败证据不会被当作通过证据。
- 默认缺失 release artifact evidence 会标记为 `skipped`；加 `--require-release-artifacts` 后这些证据缺失或无效会使 release-check 失败，包括 CI per-OS release-check snapshots、macOS/Windows/Linux X11 platform smoke、crate publish plan、npm pack report、Vesty framework release 的三示例/三平台 Steinberg validator 覆盖、三示例参数 sidecar evidence 和三示例/三平台 CI static validate 覆盖。VST3 SDK header manifest、generated-bindings plan、generated-bindings surface、`generated.rs` scaffold、`generated-abi-seed.rs` ABI seed、`generated-abi.rs` ABI layout 和 `generated-interface-skeleton.rs` interface skeleton 是 reserved generated-headers backend 的可选审计证据，缺失仍保持 `skipped`；如果存在则必须有效，且 plan/surface 和 `.rs` flags 不允许声明完整 bindings 已生成。`.rs` artifact 通过检查只表示 drift/audit metadata 有效，不表示完整 SDK 3.8 bindings 已生成。最终 `--require-release-artifacts` gate 同时要求 protocol snapshot 参与检查，传入 `--skip-protocol` 会直接失败。
- 支持 `--report <path>` 写出完整 JSON report，即使终端输出选择 markdown 也会保存机器可读 JSON。
- 支持 `--plan <path>` 写出机器可读 release action plan。该 plan 从当前 `release-check` report 派生，列出 failed/skipped checks、priority、evidence path 和建议命令，便于真实 DAW、平台、CI、签名和 notarization 采证逐项推进；对 publish-plan、crate-package、npm-pack、dependency latest baseline 和 VST3 SDK audit artifacts 这类可本地复验的 evidence，建议命令会同时给出生成命令和对应 `--check` 复验命令。它不是 pass evidence，也不会改变 `release-check` 的通过/失败结果。CI matrix 的 release readiness step 会同时上传 `release-check-<OS>.json` 和 `release-action-plan-<OS>.json`，后者用于人工采证追踪，release gate 不把它当作通过证据。Action plan sidecar validation 会拒绝顶层 `protocol_snapshot`、`evidence_root`、`release_evidence_dir` 或 action `evidence_path` 中的 `..` parent-directory component，并会把已知 action 的 evidence path 与标准 protocol/DAW/release-evidence 布局精确比对。
- 支持 `--format markdown|json`。
- 支持 `--strict`，任意 required check 失败时在打印报告后返回非零。
- 该命令不会替代 Steinberg validator、真实 DAW smoke 或真实签名/公证流程；它只把这些 gate 的当前证据汇总到一份机器可读报告。

### `vesty publish-plan`

职责:

- 从 `cargo metadata --no-deps --format-version=1` 生成 workspace 内可发布 Rust crate 的 dependency-safe 发布顺序。
- `vesty release-order` 是同一命令的 alias，便于发布脚本表达“按依赖顺序发布”。
- 支持 `--workspace <dir>` 指向 workspace 根或项目目录，默认当前目录。
- 支持 `--format text|json`；JSON 输出包含每个 package 的 `order`、`level`、`name`、`version`、`manifest_path` 和 `internal_dependencies`，便于 CI/release tooling 读取。
- 自动跳过 `publish = false` 的 workspace package，例如 `examples/*`；这些包会列入 `skipped_private`。
- 如果可发布 workspace crate 的 normal/build dependency 指向 `publish = false` 的 workspace package，命令会返回非零，避免生成一个 crates.io 上不可执行的发布计划。
- 该命令只生成/检查发布顺序，不执行 `cargo publish`，也不替代 API/semver review、registry credential 配置或 release artifact evidence。

### `vesty crate-package`

职责:

- 从 Cargo workspace publish plan 出发，生成 crate package readiness report。
- 对当前没有内部 workspace 依赖的 publishable crates 运行真实 `cargo package -p <crate> --allow-dirty --no-verify`，证明这些叶子 crate 当前可被 Cargo 打包。
- 对仍依赖其它 Vesty workspace crate 的 package 标记为 `deferred`，直到依赖 crate 按 `vesty publish-plan` 顺序发布后再逐层打包。
- 支持 `--workspace <dir>` 指向 workspace 根或项目目录，默认当前目录。
- 支持 `--out <path>` 写出 JSON report；report schema 包含 `version`、`generator`、整体 `status`，以及每个 package 的 `name`、`version`、`manifestPath`、`publishOrder`、`internalDependencies`、`status` 和 `reason`。
- 支持 `--check --out <path>` 只复验已有 report；校验要求无内部依赖 package 必须是 `packaged`，有内部依赖 package 必须是 `deferred`，且内部依赖顺序早于 dependent。
- `release-check --crate-package-report <path>` 或 `--release-evidence-dir` 中的 `crate-package/crate-package.json` 会把该 report 作为预发布 readiness evidence；普通本地检查缺失时保持 `skipped`，但 `--require-release-artifacts` 下缺失会失败，存在则严格校验。它不替代 `vesty publish-plan`、`cargo publish`、API review 或 registry credentials。

## VST3 bundle 结构

macOS:

```text
MyPlugin.vst3/
  Contents/
    Info.plist
    PkgInfo
    MacOS/
      MyPlugin
    Resources/
      moduleinfo.json
      ui/
```

Windows:

```text
MyPlugin.vst3/
  Contents/
    Resources/
      moduleinfo.json
      ui/
    x86_64-win/
      MyPlugin.vst3
```

Linux:

```text
MyPlugin.vst3/
  Contents/
    Resources/
      moduleinfo.json
      ui/
    x86_64-linux/
      MyPlugin.so
```

Vesty 应支持 merged bundle，即同一个 `.vst3` 目录里放多个平台/架构二进制。

当前实现状态:

- `vesty-build::binary_relative_path()` 已固定 macOS、Windows x86_64、Linux x86_64 的二进制相对路径。
- `vesty-build::package_vst3()` 已生成三平台 bundle 结构、`Contents/Resources/moduleinfo.json`、UI `assets.manifest.json` 和 `Contents/Resources/ui`。
- `vesty-build::ParameterManifest` 已支持显式参数 metadata sidecar；配置 `[package].parameter_manifest` 时，`package_vst3()` 会写入 `Contents/Resources/parameters.manifest.json`，`validate_vst3_bundle()` 会重新校验字符串 ID、稳定 VST3 `ParamID` 和 `ParamSpec`；`vesty param-manifest` 可从 `params.specs.json` 生成/检查该 sidecar，并会在新生成的 JSON 中显式输出 `programChange`。
- `vesty-build::normalize_class_id()` 已接入 package/validate，非法 class id 会被拒绝，合法 class id 会规范化为小写 UUID 形式。
- `vesty-build::validate_config()` 和 `validate_vst3_bundle()` 已拒绝配置与 `moduleinfo.json` metadata 中的 control characters，防止不可见字符污染 host scan metadata、bundle display name、category 或 signing identity。
- macOS bundle 会生成 `Contents/Info.plist` 和 `Contents/PkgInfo`，并在 `validate_vst3_bundle()` 中校验 package type、executable、bundle identifier shape 和 PkgInfo 内容。
- Windows/Linux bundle 不生成 macOS-only 的 `Info.plist`/`PkgInfo`。
- 同一个 `.vst3` 输出目录支持连续打入 macOS、Windows x64、Linux x64 三个平台二进制，`validate_vst3_bundle()` 会收集并验证 merged bundle 中保留的所有平台 binary。
- `validate_vst3_bundle()` 会根据 `moduleinfo.json` 的 plugin name 计算规范 binary 名称，并要求 macOS `Contents/MacOS/<name>`、Windows `Contents/x86_64-win/<name>.vst3`、Linux `Contents/x86_64-linux/<name>.so` 存在，避免错名或陈旧 binary 混入 bundle。
- 单元测试覆盖三平台路径映射、macOS/Windows/Linux package 结构、UI manifest 复制，以及 symlink asset rejection。
- `vesty doctor` 已覆盖 release signing/notarization 前置工具检查；具体签名、公证和 installer/package 签名流程仍应在发布流水线中执行和留证。
- `vesty package` 已接入 `[package].signing`: macOS 使用 `codesign` 签 `.vst3` bundle，Windows 使用 `signtool.exe` 签 platform binary，Linux 明确返回 release-channel 外部签名提示。
- `vesty package --install-dev` 已接入 dev install: 默认 copy，可用 `--install-mode symlink`，并支持 `--vst3-dir` 覆盖目标目录。
- GitHub Actions Rust matrix 会在 Ubuntu/macOS/Windows 上传 `doctor-<OS>` JSON artifact，用于记录 toolchain、WebView、validator、signing/notarization 预检和 DAW install hints；同时上传非严格 `release-check-<OS>` JSON artifact 和 `release-action-plan-<OS>` sidecar，分别记录当前 release readiness 快照和剩余采证清单。`publish plan` job 会上传 `vesty-publish-plan` artifact，包含 JSON publish plan 和 text release order；同一 job 也会上传 `vesty-crate-package` artifact，包含 `vesty crate-package --out target/crate-package/crate-package.json` 生成并复验的 crate package readiness report。`dependency baseline` job 会上传 `vesty-dependency-baseline` artifact，包含离线 `dependency-baseline.json` 及联网 `dependency-baseline-latest.json`，两者都生成并 `--check` 复验；离线 report 防止 Cargo/VST3/JS TypeScript/React/Vue/Svelte adapter dependency baseline 漂移，latest report 为最终 release gate 提供 workspace coverage 加 crates.io/npm registry latest review evidence。`js sdk` job 会上传 `vesty-npm-pack` artifact，包含 `vesty npm-pack --out target/npm-pack/npm-pack.json` 生成并复验的 dry-run report。`vst3 sdk generated bindings inputs` job 会上传 `vesty-vst3-sdk-headers` artifact；当 `VESTY_VST3_SDK_DIR` 指向官方 SDK checkout 时包含 `vst3-sdk-headers.json`、`generated-bindings-plan.json`、`generated-bindings-surface.json`、metadata-only `generated.rs` scaffold、ABI seed `generated-abi-seed.rs`、ABI layout `generated-abi.rs` 和 interface skeleton `generated-interface-skeleton.rs`，否则只包含 README skip note。`headless smoke host` job 会构建 `web-ui-param-demo` UI、生成桥接/meter marker、运行 `vesty smoke-host --strict` 和 `--check --strict`，并上传 `vesty-smoke-host` 诊断 artifact；该 artifact 不会进入 release evidence gate。package matrix 会在 Ubuntu/macOS/Windows 先运行 `examples/web-ui-param-demo/ui` 的零依赖 `npm run build`，再对三个 examples 执行 `vesty param-manifest --check`，之后上传 linux-x64/macos/windows-x64 `.vst3` bundle 和 `vesty validate --static-only --report` JSON，可用 `--release-evidence-dir` 自动聚合为 CI packaging smoke。`release-evidence` job 会先用 `vesty release-evidence collect-local --no-protocol --no-publish-plan --no-npm-pack` 初始化模板/本地采集报告，再把同一 workflow 的 artifacts 下载到 `target/downloaded-artifacts` staging 目录，包括 `vesty-crate-package` 和 `vesty-dependency-baseline`，调用 `vesty release-evidence import-ci` 内容验证并规范化到统一 evidence 目录，写入当前 GitHub Actions run URL，并运行非严格 consolidated `release-check` 生成 `release-evidence-consolidated` artifact。它们不替代真实 DAW smoke、macOS/Windows/Linux X11 platform smoke、Steinberg validator passed reports 或 signed/notarized release artifact。
- `vesty release-check` 可通过 `--release-evidence-dir` 一次性读取模板 evidence 目录，也可通过 `--ci-doctor-dir`、`--ci-release-check-dir`、`--platform-smoke-dir`、`--ci-run-url`、`--publish-plan-report`、`--crate-package-report`、`--npm-pack-report`、`--dependency-baseline-report`、`--vst3-sdk-manifest`、`--vst3-sdk-binding-plan`、`--vst3-sdk-binding-surface`、`--vst3-sdk-scaffold`、`--vst3-sdk-abi-seed`、`--vst3-sdk-abi`、`--vst3-sdk-interface-skeleton`、`--static-validate-report`、`--validate-report`、`--signed-bundle-evidence`、`--notarization-log` 分项聚合这些外部 artifact；显式传入的 file/log evidence 路径在读取前会拒绝 symlink，显式传入的递归 artifact root 也会拒绝 symlinked file/directory root。只有传入 `--require-release-artifacts` 时才把缺失 release artifact 视为失败。crate package readiness 普通本地检查缺失时保持 skipped，`--require-release-artifacts` 下缺失会失败，存在则必须完整有效；dependency baseline release evidence 必须是 `vesty dependency-baseline --latest` 生成的 report，并且必须同时包含 workspace 外部依赖覆盖检查和 registry latest checks，普通离线 report 不满足 final latest gate。静态 validate report 仍不替代 Steinberg validator passed report；VST3 SDK header manifest、generated-bindings plan、generated-bindings surface 和 SDK `.rs` audit artifact 是可选审计证据，缺失时即使在 strict release gate 下也保持 skipped，但存在时必须严格有效，且 plan/surface/`.rs` flags 不允许声明完整 bindings 已生成。Vesty framework release gate 会分别检查 CI 三平台 release-check 快照、本地 invariant、macOS/Windows/Linux X11 platform smoke、三示例/三平台 validator-passed 覆盖、三示例/三平台 CI package static validate 覆盖、dependency latest baseline，以及四个 JS package 的 npm pack dry-run 边界。`--release-evidence-dir` 只有在 `ci-doctor/` / `ci-release-checks/` 中存在 JSON artifacts 时才自动启用对应目录，`platform-smoke/` 中只有 README 或全 pending 模板时也不会自动启用，空模板不会把 skipped 变成 failed；`vst3-sdk/README.md` 也不会被当成 manifest/plan/surface/`.rs` evidence。

## macOS 签名与 notarization

MVP:

- 本地开发不强制签名。
- release package 使用 `[package].signing` 作为 signing identity。
- `vesty package` 使用 `codesign --deep --options runtime --timestamp` 签 `.vst3` bundle。
- `vesty notarize` 使用 `ditto`、`xcrun notarytool` 和 `xcrun stapler` 封装可选 notarization workflow。

注意:

- 使用 WebKit.framework。
- release 不启用 wry devtools/transparent/fullscreen 等可能触发私有 API 风险的 feature，除非用户明确开启。

## Windows 分发

MVP:

- MSVC target。
- 生成 `.vst3` bundle folder。
- 检查 WebView2 runtime，可给用户提示安装 evergreen runtime。
- `[package].signing` 非空时使用 Authenticode `signtool.exe` 签 Windows platform binary。

Windows on Arm:

- 第一版支持 x86_64-win。
- 后续支持 arm64ec-win 和 arm64x-win，因为 VST3 官方建议关注 Windows on Arm host/plugin 架构组合。

## Linux 分发

MVP:

- x86_64-linux。
- 依赖 WebKitGTK 4.1/GTK3。
- `vesty doctor` 检查 `pkg-config`、WebKitGTK、GTK。
- X11 child webview 优先，Wayland 通过 GTK container 实验支持。

## UI assets manifest

示例:

```json
{
  "version": 1,
  "root": "ui",
  "entry": "index.html",
  "files": [
    {
      "path": "index.html",
      "mime": "text/html",
      "sha256": "..."
    },
    {
      "path": "assets/app.js",
      "mime": "text/javascript",
      "sha256": "..."
    }
  ]
}
```

custom protocol 只允许访问 manifest 里的 path。当前 `vesty-ui-wry` 会在 release asset root 的父目录或 root 内查找 `assets.manifest.json`；release attach 必须成功加载 manifest，否则返回 runtime unavailable。运行时加载会先拒绝 symlinked asset root 和 symlinked `assets.manifest.json`，再校验 schema、`version = 1`、build-time `root` 非空且无 control characters、entry 存在、files 非空、path URL-safe 且不重复、mime 非空、sha256 为 64 位 hex，并拒绝顶层和 file entry 未知字段。URL-safe path 必须是相对路径，不能包含反斜杠、空段、`.`/`..` 段、ASCII control、`%`、`?`、`#` 或 `:`；`root` 只作为 provenance，不要求等于运行时 bundle 路径；`vesty-build` 生成 manifest 时也会拒绝 symlinked UI dist root、dist 内 symlink、以及 dist 中含这些歧义字符的资源路径。加载成功后只服务 manifest `files[].path` 中列出的资源，并继续做 symlink 拒绝和 canonical root 检查。读取文件后会校验 manifest 中的 `size` 和 `sha256`，不匹配时返回 404。

## CI

当前仓库包含 `.github/workflows/ci.yml`，本地可自动验证的矩阵:

- `rust`: `ubuntu-latest` / `macos-latest` / `windows-latest`。
- `js sdk`: Ubuntu + Node 24。
- `protocol export`: Ubuntu + Rust + Node 24。
- `publish plan`: Ubuntu 运行 `vesty publish-plan --out target/publish-plan/publish-plan.json` 并 `--check` 复验，同时生成 `vesty release-order` 文本 artifact；同一 job 运行 `vesty crate-package --out target/crate-package/crate-package.json` 并 `--check` 复验，上传 crate package readiness artifact。
- `dependency baseline`: Ubuntu 运行离线 `vesty dependency-baseline --out target/dependency-baseline/dependency-baseline.json` / `--check` 复验，用于防止已复核依赖基线漂移，包括全部当前外部 Cargo workspace dependencies、VST3/TypeScript 和 React/Vue/Svelte adapter dev dependency lockfile 基线；同时显式运行 `vesty dependency-baseline --latest --out target/dependency-baseline/dependency-baseline-latest.json` / `--latest --check`，为最终 release gate 留存 workspace coverage 加 crates.io/npm registry latest evidence。
- `npm pack`: Ubuntu 运行 `vesty npm-pack --out target/npm-pack/npm-pack.json` 并 `--check` 复验，作为 JS package publish boundary evidence。
- `vst3 sdk generated bindings inputs`: Ubuntu 在 `VESTY_VST3_SDK_DIR` 存在时运行 `vesty vst3-sdk manifest --out target/vst3-sdk/vst3-sdk-headers.json`、`vesty vst3-sdk binding-plan --bindings-module target/vst3-sdk/generated.rs --out target/vst3-sdk/generated-bindings-plan.json`、`vesty vst3-sdk binding-surface --out target/vst3-sdk/generated-bindings-surface.json`、`vesty vst3-sdk emit-scaffold --out target/vst3-sdk/generated.rs`、`vesty vst3-sdk emit-abi-seed --out target/vst3-sdk/generated-abi-seed.rs`、`vesty vst3-sdk emit-abi --out target/vst3-sdk/generated-abi.rs` 和 `vesty vst3-sdk emit-interface-skeleton --out target/vst3-sdk/generated-interface-skeleton.rs`，并分别用 `--check` 复验；binding-surface 额外要求所有 expected symbol identifier token 都能在对应 locked header 中找到，当前 locked inputs 覆盖 program/unit 和 Note Expression headers，ABI seed、ABI layout 和 interface skeleton 额外要求 `FULL_COM_BINDINGS_GENERATED = false`，ABI layout 还要求 `ABI_LAYOUT_RECORDS` / `ABI_FIELD_OFFSETS` 指纹存在，interface skeleton 还要求 method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata、`INTERFACE_IDS`、`QUERY_INTERFACE_ENTRIES`、`QUERY_INTERFACE_IID_LOOKUP_SCOPE`、`interface_id_for_iid()`、`query_interface_entry_by_interface()`、`query_interface_entry_for_iid()`、`com_object_query_interface_dispatch_by_interface()`、`com_object_query_interface_dispatch_for_iid()`、`COM_OBJECT_INTERFACES`、`COM_OBJECT_INTERFACE_SCOPE`、`COM_OBJECT_IDENTITY_PLANS`、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`、`FACTORY_EXPORT_PLAN`、`FACTORY_CLASS_PLANS`、`MODULE_EXPORT_PLANS`、`BINARY_EXPORT_SYMBOL_PLANS`、`BINARY_EXPORT_INSPECTION_TOOL_PLANS`、`BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED`、`BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`、`binary_export_symbol_plan_by_platform_and_symbol()`、`binary_export_inspection_tools()`、`required_binary_export_symbol_count()`、`first_missing_binary_export_symbol()`、`binary_export_required_symbols_present()`、关键 `*_IID` constants 和 `iid_from_words()` 存在。`vesty-vst3-sys` 的 public binary-export plan API 是 CLI validate/release gate 的 required-symbol/tool-order single source of truth。未设置 SDK 目录时上传 README skip note。该 job 是可选审计证据，不会让 upstream `vst3` backend 的 release gate 变成 SDK checkout 强依赖，也不表示完整 SDK 3.8 bindings 已生成。
- `headless smoke host`: Ubuntu 构建 `examples/web-ui-param-demo/ui`，写入本地 bridge/meter marker，运行 `vesty smoke-host --strict` 和 `--check --strict`，上传 `vesty-smoke-host` 诊断 artifact。该 job 是本地 framework self-check，不加载插件二进制，不替代真实 DAW/validator/platform/signing evidence。
- `scaffold smoke`: Ubuntu 生成 local `--vesty-path` + `--plugin-ui-path` 项目，并跑 Rust/UI build。
- `package smoke`: Ubuntu/macOS/Windows 先检查三个 examples 的 `params.specs.json` / `vesty-parameters.json` 无 drift，再 release build 三个 examples，按 linux-x64/macos/windows-x64 package `.vst3`，跑 `vesty validate --static-only --strict --format json`。生成的 static validate JSON 会包含 `static_check.binary_exports`；若本平台导出符号工具成功运行但缺少 `GetPluginFactory` 或平台 entry/exit symbol，static validation 会失败。若工具缺失或无法解析当前格式，report 会记录 skipped，且 `--strict` 会在写出 report 后返回非零，不能满足最终 strict release 的 binary export evidence 要求。
- `release evidence snapshot`: 初始化 release evidence 模板和 `local-collect-report.json`，下载本 workflow 的 doctor、protocol、publish-plan、crate-package、npm-pack、VST3 SDK header manifest/generated-bindings plan/generated-bindings surface、package/static validate 和 per-OS release-check artifacts 到 staging 目录，通过 `vesty release-evidence import-ci` 规范化并生成 `import-ci-report.json`，再生成 consolidated release evidence report；真实 platform smoke 仍需要在 macOS、Windows x64 和 Linux X11 桌面/host 环境单独采集并放入 `platform-smoke/`。

步骤:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- feature gated test + clippy: `vesty-vst3` with `vst3-bindings wry-ui`、`vesty-ui-wry` with `wry-backend`。
- JS SDK `npm run typecheck` / `npm test` / `npm run build`。
- protocol TypeScript/JSON Schema export + deterministic `--check` + exported TypeScript strict check。
- publish plan JSON/text export，确认 crate registry 发布顺序可由 CI artifact 留档。
- crate package readiness JSON export/check，确认无内部依赖的叶子 crate 当前可 `cargo package`，有内部依赖的 crate 被明确标记为 deferred。
- dependency baseline JSON export/check，确认全部当前外部 Cargo workspace dependencies、VST3 SDK/binding baseline、JS TypeScript range、React/Vue/Svelte adapter devDependency range 和 lockfile installed version 与已复核基线一致；CI 也会显式生成/check `dependency-baseline-latest.json`，确认 crates.io/npm registry latest。
- optional VST3 SDK header manifest, generated-bindings plan, generated-bindings surface, metadata scaffold, ABI seed, ABI layout and interface skeleton export/check when `VESTY_VST3_SDK_DIR` is configured; otherwise upload an explicit skip note.
- headless `vesty smoke-host` report/check，确认 example config、参数 sidecar、Web UI demo asset manifest、bridge marker 和 meter marker 的本地框架链路没有漂移。
- generated scaffold matrix: `none` 跑 Rust check；`vanilla` / `react` / `vue` / `svelte` 跑 Rust check、UI install/build/typecheck。
- example release build + Linux/macOS/Windows static VST3 bundle/resource validation。
- consolidated `vesty release-check --release-evidence-dir ... --protocol-snapshot ...` report。
- UI assets manifest test。
- wry custom protocol manifest allowlist / path traversal test。

CI artifact:

- `doctor-Linux` / `doctor-macOS` / `doctor-Windows`: 每个平台的 `vesty doctor --format json`。
- `release-check-Linux` / `release-check-macOS` / `release-check-Windows`: 每个平台的非严格 release readiness 快照，包含 `release-check-<OS>.json` 和 checklist-only `release-action-plan-<OS>.json` sidecar；后者可被 `import-ci` 保存到 `ci-release-checks/`，但不会被 `--ci-release-check-dir` 当作 pass evidence。
- `vesty-protocol`: exported TypeScript + JSON Schema。
- `vesty-publish-plan`: `publish-plan.json` 和 `release-order.txt`，用于 crates.io 发布顺序留档。
- `vesty-crate-package`: `crate-package.json`，用于记录当前 packageable leaf crates 的真实 `cargo package` smoke，以及因内部依赖尚未发布而 deferred 的 crates。`release-evidence import-ci` 会把有效 report 复制到 `crate-package/crate-package.json`；该 artifact 是可选 readiness evidence，不代表已经执行 `cargo publish`。
- `vesty-dependency-baseline`: `dependency-baseline.json` 和 `dependency-baseline-latest.json`。前者用于 CI 防止已复核的 Cargo/VST3/JS TypeScript/React/Vue/Svelte adapter dependency baseline 漂移；后者必须包含 `cargo workspace external dependency baseline coverage`、所有当前外部 workspace Rust dependency 的 crates.io latest checks 和 npm registry latest checks，`release-evidence import-ci` 只会把这份 latest report 复制到 `dependency-baseline/dependency-baseline-latest.json` 作为 final release evidence。该 artifact 不替代 Steinberg SDK latest review、DAW/platform/signing release evidence。
- `vesty-vst3-sdk-headers`: 可选官方 SDK header input manifest / generated-bindings readiness artifact；有 SDK checkout 时包含 `vst3-sdk-headers.json`、`generated-bindings-plan.json`、`generated-bindings-surface.json`、metadata-only `generated.rs` scaffold、ABI seed `generated-abi-seed.rs`、ABI layout `generated-abi.rs` 和 interface skeleton `generated-interface-skeleton.rs`，否则包含 README skip note。`release-evidence import-ci` 只会复制有效完整 manifest 到 `vst3-sdk/vst3-sdk-headers.json`，复制 ready 且 `bindingsGenerated = false` 的 plan 到 `vst3-sdk/generated-bindings-plan.json`，复制 ready、`bindingsGenerated = false`、`missingSymbols = []` 且所有 `symbolPresent = true` 的 surface 到 `vst3-sdk/generated-bindings-surface.json`，在 scaffold markers、baseline、complete header metadata 和 `BINDINGS_GENERATED = false` 都有效时复制 scaffold 到 `vst3-sdk/generated.rs`，在 ABI seed marker、基础 alias/constant surface 和 `FULL_COM_BINDINGS_GENERATED = false` 都有效时复制 seed 到 `vst3-sdk/generated-abi-seed.rs`，在 ABI layout marker、基础/program/unit/Note Expression `repr(C)` layout surface、size/alignment/field-offset 指纹、`ABI_LAYOUT_GENERATED = true` 和 `FULL_COM_BINDINGS_GENERATED = false` 都有效时复制 layout 到 `vst3-sdk/generated-abi.rs`，并在 interface skeleton marker、interface/vtable skeleton surface、method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata、`INTERFACE_IDS`、`QUERY_INTERFACE_ENTRIES`、`QUERY_INTERFACE_IID_LOOKUP_SCOPE`、`interface_id_for_iid()`、`query_interface_entry_by_interface()`、`query_interface_entry_for_iid()`、`com_object_query_interface_dispatch_by_interface()`、`com_object_query_interface_dispatch_for_iid()`、`COM_OBJECT_INTERFACES`、`COM_OBJECT_INTERFACE_SCOPE`、`COM_OBJECT_IDENTITY_PLANS`、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`、`FACTORY_EXPORT_PLAN`、`FACTORY_CLASS_PLANS`、`MODULE_EXPORT_PLANS`、`BINARY_EXPORT_SYMBOL_PLANS`、`BINARY_EXPORT_INSPECTION_TOOL_PLANS`、`BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED`、`BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`、`binary_export_symbol_plan_by_platform_and_symbol()`、`binary_export_inspection_tools()`、`required_binary_export_symbol_count()`、`first_missing_binary_export_symbol()`、`binary_export_required_symbols_present()`、关键 `*_IID` constants、`iid_from_words()`、`INTERFACE_SKELETON_GENERATED = true` 和 `FULL_COM_BINDINGS_GENERATED = false` 都有效时复制 skeleton 到 `vst3-sdk/generated-interface-skeleton.rs`；surface 是 symbol-token audit artifact，scaffold、ABI seed、ABI layout 和 interface skeleton 是 drift-check artifact，都不代表 bindings 已生成。
- `vesty-smoke-host`: 本地 headless self-check report、bridge marker 和 meter marker。该 artifact 只用于诊断 CI/workspace 漂移，不会由 `release-evidence import-ci` 导入为 release pass evidence。
- `linux-vst3-static-validate` / `macos-vst3-static-validate` / `windows-vst3-static-validate`: packaged `.vst3` bundles + static validate JSON。
- `release-evidence-consolidated`: 合并后的 release evidence 目录，包含 CI run URL、doctor artifacts、protocol snapshot、publish plan、crate package readiness、npm pack report、dependency latest baseline report、可选 VST3 SDK header manifest/generated-bindings plan/generated-bindings surface/metadata scaffold/ABI seed/ABI layout/interface skeleton、三平台 package static validate reports、`local-collect-report.json`、`import-ci-report.json`、`platform-smoke/README.md` 指引和 consolidated `release-check-consolidated.json`。真实 `macos.json`、`windows-x64.json`、`linux-x11.json` platform smoke reports 仍需外部采集后补入。

Action baseline:

- 2026-06-08 通过官方 Git tags 核对: `actions/checkout@v6`、`actions/setup-node@v6`、`actions/upload-artifact@v7` 和 `actions/download-artifact@v8` 均存在。
- release evidence snapshot 下载 artifact 使用 `actions/download-artifact@v8`；上传 artifact 使用 `actions/upload-artifact@v7`。

仍需外部证据:

- Steinberg validator binary 不假设存在于 GitHub runner；真实 validator smoke 仍由本地/手工命令或专用 runner 提供。
- Cubase/Nuendo、Bitwig、Ableton Live、Studio One、Windows/WebView2 和 Linux/WebKitGTK/X11 platform smoke 仍需要安装对应 host/runtime 的机器。
