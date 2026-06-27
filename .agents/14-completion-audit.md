# Vesty 完成度审计

更新时间: 2026-06-13

本文档把原始实现计划拆成可验收条目，区分三类状态:

- `Done`: 已有代码实现，并有本机自动化测试或 smoke 记录支撑。
- `Implemented, Needs External Evidence`: 本地代码路径已实现，但发布声明需要真实 DAW、平台或 CI artifact。
- `Blocked By Scope/Evidence`: 不是当前 MVP 范围，或无法在本机无外部证据下证明。

## 总体结论

Vesty 当前达到 alpha skeleton: Rust workspace、核心 DSP API、VST3 adapter、实时参数/事件系统、wry Web UI runtime、JSBridge、CLI、打包、validator/evidence gate 和三个示例插件均已落地并通过本机 Rust/JS 检查。

不能宣称完整 release-ready。发布级声明仍缺真实 Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One smoke，Windows/WebView2 与 Linux/WebKitGTK/X11 实机验证，真实 GitHub Actions artifact 下载证据，macOS/Windows 签名证据，以及 accepted + stapled notarization 证据。

`vesty smoke-host` 已提供本地 headless framework self-check，用于检查 example config、参数 sidecar、Web UI asset manifest 以及可选 JSBridge/meter trace marker。它不加载插件二进制，也不替代 DAW、平台 WebView、Steinberg validator、签名或 notarization evidence。

## 原始计划映射

| 计划项 | 状态 | 当前证据 | 剩余工作 |
| --- | --- | --- | --- |
| Cargo workspace 与核心 crates | Done | workspace 包含 `vesty` facade、core/params/rt/vst3/ipc/bridge/ui/build/cli/macros/sys crates；`cargo test --workspace` 通过；根 README/license 与 crates.io metadata 已补齐；`vesty publish-plan --out` 可从 Cargo metadata 生成并复验 dependency-safe crate 发布顺序；`vesty crate-package --out` 可对当前无内部依赖的叶子 crate 执行真实 `cargo package` smoke，并把仍依赖内部 workspace crates 的 package 标记为 deferred；crate-package report 现在内嵌同一次 publish plan，验证时要求 package entries 与 embedded plan 的 name/version/manifest path/order/deps 逐项一致，release-check 同时拿到 publish-plan 与 crate-package evidence 时还会交叉校验两份 publish plan 一致 | 发布前做 API/semver review，并按 `vesty publish-plan` 执行 registry 发布 |
| Rust public API: `Plugin` / `AudioKernel` / `ProcessContext` / `Params` derive / `UiDescriptor` / `export_vst3!` | Done | facade prelude、derive tests、examples、README minimal plugin snippet 和 `vesty new` 模板均使用该 API；`Params` derive 已支持 `#[param(skip)]`、`#[param(id = "...")]` 和 BoolParam `#[param(bypass)]`；`vesty-params::validate_param_specs()` 与 VST3 controller gate 已覆盖参数 schema 基础合法性；README snippet drift 由 `workspace_packages_have_release_metadata` 守住 | API freeze 前做 semver review |
| VST3 adapter 最小链路 | Done | fake COM tests 覆盖 factory、processor/controller、稳定正数 31-bit VST3 `ParamID`、state、bus、main bus `getRoutingInfo()`、`IComponent::activateBus()` 对已声明 bus 的验证和 active-state 跟踪、effect optional sidechain aux input bus、multi-output instrument main/aux output bus metadata + arrangement + sample32 processing path、`kSample32`/`kSample64` sidechain process path、sample-accurate automation、NoteOn/Off、PolyPressure、legacy MIDI CC、PitchBend、ChannelPressure、fixed-buffer SysEx data event、Note Expression value/int/text event、opt-in `INoteExpressionController` value metadata 和 opt-in `INoteExpressionPhysicalUIMapping` static mapping metadata、`IMidiMapping` 参数映射、`IUnitInfo` root unit/program-list metadata、static program attributes/pitch names、opt-in `Plugin::apply_program()` program selection、controller-side program-change 参数 `setParamNormalized()` / edit relay selection、audio `process()` 内 program-change 参数 automation 作为 realtime-safe 普通参数事件与 atomic 参数更新处理且不会调用 `apply_program()` / `load_program_data()`、controller-side `IProgramListData` program data helper、program apply/data load/program-change 参数选择后 Web UI `param.changed source = "program"`、VST3 `kIsProgramChange` 参数 metadata、transport、panic/faulted silence、IPlugView resize/open-close stress；`examples/midi-synth` 现在提供 concrete program/preset workflow 示例，覆盖 host-visible program 参数、factory program list、attributes/pitch names、controller-side `apply_program()` 和 per-program JSON data roundtrip，也提供固定 SysEx level override 和 Note Expression brightness/tuning metadata + DSP 消费示例；`Sample64` 默认路径仍是 adapter-level f64<->f32 scratch bridge，`setupProcessing()` 非实时预分配 main/sidechain/output scratch，realtime `process()` 不扩容；未调用 `setupProcessing()` 或 host 传入超过预分配 capacity 的默认 `kSample64` block 时，adapter 会清零 host f64 outputs、设置 silence flags，并且测试覆盖不创建/进入 kernel、不做 realtime 扩容；原生 double-precision developer DSP API 已以 opt-in 方式实现: `AudioKernel::SUPPORTS_F64 = true` 时，VST3 adapter 以 `ProcessContext64` / `AudioBuffers64` 直接调用 `process_f64()`，fake COM 测试覆盖不进入 f32 fallback、`NoAllocGuard` active、realtime path 0 allocation 和超过 scratch capacity 的 native f64 block；非实时 processor/controller message path 的内部 `IAttributeList` 已覆盖 `int` / `float` / `string` / `binary` attribute set/get、string truncation nul-termination 和 invalid pointer/missing value 防御；稳定 ParamID helper 已移到 `vesty-params` 供 VST3 adapter 和 build metadata 共用；`vesty-vst3-sys` 可生成/校验官方 SDK header input manifest，锁定后续 generated-headers 所需 `pluginterfaces` headers、size、sha256 和 missing headers，其中包括 program/unit `ivstunits.h` 与 Note Expression `ivstnoteexpression.h`；也可生成/校验 `GeneratedBindingsPlan` readiness report 和 schema v2 `GeneratedBindingsSurface` symbol surface report，证明 SDK inputs、`.rs` output module path、expected symbol/header surface 和 identifier-token presence 已可审计但 `bindingsGenerated = false`、`missingSymbols = []`、`symbolPresent = true`；`vesty vst3-sdk emit-scaffold` 可生成/check metadata-only `generated.rs` scaffold，固定 output module 落点但不包含 VST3 COM/API bindings；`vesty vst3-sdk emit-abi-seed` 可生成/check deterministic `generated-abi-seed.rs`，固定基础 VST3 ABI aliases/constants 和 metadata，同时保持 `BINDINGS_GENERATED = false` 与 `FULL_COM_BINDINGS_GENERATED = false`；`vesty vst3-sdk emit-abi` 可生成/check deterministic `generated-abi.rs` foundational ABI layout，固定基础 aliases/constants 与 `repr(C)` `TUID`、`FUnknownVTable`、`FUnknown`、`ViewRect`、`ProgramListInfo`、`UnitInfo`、`NoteExpressionValueDescription`、`NoteExpressionTypeInfo`、`PhysicalUIMap`、`PhysicalUIMapList` 等少量 layout，并输出 `ABI_LAYOUT_RECORDS` size/alignment 指纹与 `ABI_FIELD_OFFSETS` 关键字段 offset 指纹，同时保持 `ABI_LAYOUT_GENERATED = true`、`BINDINGS_GENERATED = false` 与 `FULL_COM_BINDINGS_GENERATED = false`；`vesty vst3-sdk emit-interface-skeleton` 可生成/check deterministic `generated-interface-skeleton.rs` interface/vtable skeleton 与 method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata，固定 interface placeholder、vtable skeleton、per-interface method arrays、global method list、per-interface vtable slot seed arrays、global vtable slot seed list、per-interface callback type arrays、global callback type list、callback type aliases、repr(C) callback field layout、`offset_of!` field offset 指纹、upstream `vst3 0.3.0` IID words、`INTERFACE_IDS`、`QUERY_INTERFACE_ENTRIES`、`QUERY_INTERFACE_IID_LOOKUP_SCOPE`、`interface_id_for_iid()` / `query_interface_entry_by_interface()` / `query_interface_entry_for_iid()` / `com_object_query_interface_dispatch_by_interface()` / `com_object_query_interface_dispatch_for_iid()` 纯查找 helper、`COM_OBJECT_INTERFACES`、`COM_OBJECT_IDENTITY_PLANS`、`COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`、`FACTORY_EXPORT_PLAN`、`FACTORY_CLASS_PLANS`、`MODULE_EXPORT_PLANS`、`BINARY_EXPORT_SYMBOL_PLANS`、`BINARY_EXPORT_INSPECTION_TOOL_PLANS`、`binary_export_symbol_plan_by_platform_and_symbol()` / `binary_export_inspection_tools()` / `required_binary_export_symbol_count()` / `first_missing_binary_export_symbol()` / `binary_export_required_symbols_present()` 纯 binary export required-symbol helpers，并保持 `INTERFACE_SKELETON_GENERATED = true`、`BINDINGS_GENERATED = false`、`FULL_COM_BINDINGS_GENERATED = false` 与 `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`，不包含 callable `queryInterface` glue、generated factory exports、generated module exports、binary inspection tooling、factory glue 或 Steinberg method implementations；本机 validator smoke 已记录 | 更多 host quirk 实机验证；sidechain 仍需真实 DAW routing/automation smoke；multi-output instrument 仍需真实 DAW routing smoke；真实 DAW program workflow、真实 SysEx workflow 和真实 expression workflow 仍需采集；完整 SDK 3.8 generated bindings emitter 仍未生成 |
| 实时安全参数/事件系统 | Done | atomics、fixed event list、`rtrb` SPSC、`NoAllocGuard`、sample-accurate automation segments、meter/log queues 均有单测；参数 flag builder 已覆盖 read-only、non-automatable 和 bypass metadata | 更多 channel layout 和长时间 soak |
| wry/system WebView runtime | Implemented, Needs External Evidence | `vesty-ui-wry` 实现 attach/detach/resize、dev URL、manifest-backed custom protocol、navigation/IPC allowlist、IPC handler panic boundary 和 bootstrap tests；release asset loading 现在拒绝 symlinked asset root、symlinked `assets.manifest.json` 和 manifest-listed symlink assets，并继续校验 URL-safe path、manifest allowlist、size/sha256 和 canonical root；runtime asset manifest mirror 已与 build-time `assets.manifest.json` shape 对齐，读取 `version/root/entry/files`，拒绝 unsupported version、invalid root 和顶层/asset file 未知字段；manifest `root` 只作为 build-time provenance，不绑定运行时 bundle 路径；crate 级 deny undocumented unsafe blocks 已覆盖平台 handle/IPC unsafe 点 | Windows WebView2、Linux X11 WebKitGTK、真实 DAW editor open/close/resize stress |
| JSBridge message bus | Done | typed packets、hello/readyAck、session adoption、request timeout、subscription、state revisions、param gestures、meter/log/diagnostics tests；`BridgeRuntime::new()` / `try_new()` 在 ready store/param map 建立前复用 `validate_param_specs()` 并以 `ParamSpecError` 拒绝非法参数 schema；`BridgeReadyPayload.capabilities` 已由 Rust `BridgeRuntime` 强制执行，禁用能力会返回 non-retryable `unsupported_type`，且不会污染 subscription table、pending param gesture queue、state snapshot 或 meter queue；`ParamSpec.midiMappings` / `ParamMidiMapping` 已进入 Rust/TS protocol snapshot，`@vesty/plugin-ui` `ready()` 也会运行时校验 ready payload/params shape 并以 `validation_error` 支持失败后重试；公共 hello/ready payload 与 BridgePacket 顶层保持 forward-compatible，测试覆盖 hello 未来 JS capability 字段可完成握手、ready 顶层/capabilities/snapshot/param flags/kind/MIDI mapping 扩展字段可完成握手并被 cached payload 保留；`@vesty/plugin-ui` 和 wry bootstrap 现在还会 fail-closed 校验 Rust -> JS `deliver()` / `deliverBatch()` packet shape，畸形入站 packet 不触发 listener、不错误 settle pending request；`deliverBatch()` 入站最多处理 4096 个 packet，Rust `vesty-ui-wry::batch_scripts()` 也按同一上限把 native response/event batch 切块后逐块 `evaluate_script()`，避免内部超限 batch 被 JS guard 整批丢弃；WebView -> Rust request 在 `postMessage()` 前按 UTF-8 byte length 复用 Rust lane 上限，普通 command/param/event/meter/log/lifecycle 64 KiB，state 256 KiB，超限时返回 retryable `backpressure` 且不登记 pending/不消耗 seq；async event pump 每个 bridge instance 同时最多保留一个 in-flight `event.flush`，host/UI 卡顿时不会按 60 Hz 堆积 pending flush，且内部 `event.flush` 即使在 `timeoutMs = 0` 时仍有 1000ms watchdog 防止 pump 永久停摆；Bridge envelope `seq` 已在 Rust/JS/wry 三侧限制为 JavaScript safe integer，并在发送端到达上限后回绕到 `1`；Rust native bridge 对 Vesty 内建 request payload 执行固定 allowlist，拼写错误或额外字段会返回 `validation_error` 且不污染 runtime state，`snapshot.get`、`diagnostics.get`、`meter.flush` 和 `event.flush` 只接受无 payload、`null` payload 或空 object，SDK/bootstrap query helper 发送 `{}`；wry bootstrap fallback internal `setSession()` / `request(type, lane, payload)` 也会校验 session 与 lane，避免全局 internal 入口构造畸形 session 或未知 lane request；`npm test` 覆盖 SDK sequencing、本地 reload/close pending cleanup stress 和 malformed inbound delivery | 长时间 UI reload/close stress in real hosts |
| State model: snapshot + revision + typed command | Done | bridge state conflict tests、VST3 state roundtrip、active UI restore sync tests | 跨 DAW save/restore smoke |
| JS package `@vesty/plugin-ui` | Done | TypeScript build/typecheck/test；protocol export snapshot check；React/Vue/Svelte thin adapters exist；JS package metadata/exports/files 已补齐；`vesty npm-pack --out` 会生成并复验 npm workspace dry-run report；`release-check --npm-pack-report` 可验证四个 JS package 的 dry-run 发布边界 | npm publish execution 和 registry credentials |
| CLI scaffold/build/package/validate/doctor | Done | `vesty new/dev/build/package/validate/doctor/daw-matrix/platform-smoke/smoke-host/release-evidence/release-check/export-types/vst3-sdk/notarize/publish-plan/crate-package/dependency-baseline/npm-pack/param-manifest/templates` implemented with unit tests; `vesty templates` 提供内置 starter gallery，`vesty new --template <id>` 支持 gain、midi-synth、Web UI param demo 和 framework-specific starters；`vesty new path/to/my-plugin` 只用最后一级目录名生成 project/plugin/crate metadata，目标路径不再污染 VST3 display name、Rust type name 或 bundle id；Web UI 模板跟随 plugin kind 绑定 starter 主参数，effect UI 使用 `gain`，instrument UI 使用 `volume`，全量 starter 测试会拒绝 instrument UI 中残留的 `gain` bridge 调用；Web UI starter 以 `BridgeReadyPayload.params[].defaultNormalized` 初始化控件，并订阅 `param.changed` 同步 host/controller/UI confirmed 参数值，避免误把 `PluginSnapshot` 当当前参数值来源；本机 template gallery smoke 已生成全部 7 个内置 starter 临时项目并通过独立 `cargo check`，其中 5 个 Web UI starter 还通过 `npm install`、`npm run build` 和 `npm run typecheck`；`param-manifest --specs`、`smoke-host --check --out`、`smoke-host --bridge-trace`、`smoke-host --meter-log`、example parameter specs 读取和 CLI 通用 TOML/JSON reader 现在拒绝 symlinked inputs；`smoke-host` 提供本地 headless framework self-check 但不作为 release pass evidence；`publish-plan` / `crate-package` / `npm-pack` / `dependency-baseline` 均有 drift/release evidence gate；`vesty vst3-sdk manifest`、`binding-plan`、`binding-surface`、`emit-scaffold`、`emit-abi-seed`、`emit-abi` 和 `emit-interface-skeleton` 可生成/check optional generated-headers 审计文件，其中 `generated.rs`、`generated-abi-seed.rs`、`generated-abi.rs` 和 `generated-interface-skeleton.rs` 会在显式传入或被 `--release-evidence-dir` 标准路径发现时由 `release-check` 严格校验，必须保持 `BINDINGS_GENERATED = false` / `FULL_COM_BINDINGS_GENERATED = false`，但仍只证明 drift/audit metadata 有效，不证明完整 SDK bindings 或最终 release readiness；`collect-local` 和 `import-ci` 可规范化本地/CI artifacts，包括 VST3 SDK manifest/plan/surface/scaffold/ABI/interface skeleton、crate package、dependency latest、npm pack、签名和 notarization helpers，且 `collect-signing` 现在也拒绝 symlinked bundle root、Windows payload dir、显式 Windows binary symlink、bundle 外 Windows binary、非 `.vst3` Windows binary、通过 payload 子目录 symlink 跳出 bundle 的 binary、显式 verification `--tool` leaf/parent symlink，并且 macOS/Linux 不会静默接受 `--binary`；`import-ci` / `release-check --release-evidence-dir` 都会拒绝互相矛盾的显式 CI run URL 来源、symlinked `ci-run-url.txt`、symlinked release evidence root、symlinked import-ci source root、既有 symlink import-ci output dir、缺失 import-ci output root 的 symlinked parent、symlinked import-ci destination parent，以及模板标准 evidence 文件/目录槽位中的 symlink；显式传入的 release evidence files/logs 和递归 artifact roots 会在读取前拒绝 symlink，CLI 写出的 report/log/smoke marker 会拒绝既有 symlink 输出文件和用户可替换的 symlink 输出父目录，evidence template 初始化文件/目录槽位以及创建路径中的已存在祖先目录也会拒绝 symlink；`vesty new` 模板、strict `vesty.toml` schema、CI doctor OS invariant、CI per-OS release-check invariant 和 platform smoke gate 均已实现 | End-to-end release pipeline artifact collection；二进制插件 metadata introspection 仍故意不做，避免 CLI 加载执行任意插件代码；第三方模板 registry、远程模板签名和缓存仍是后续生态工作 |
| Cross-platform bundle structure | Implemented, Needs External Evidence | macOS/Windows/Linux bundle path and binary format static tests; bundle id shape, macOS fallback `CFBundleIdentifier`, metadata control-character rejection, static validator rejection of invalid macOS identifiers, `CFBundleExecutable`/moduleinfo binary drift, `CFBundleName`/moduleinfo name drift and plist/moduleinfo version drift are locally validated；`vesty-build` now rejects symlinked config/parameter sidecar inputs, package binary inputs, bundle root/Contents/Resources, `moduleinfo.json`, packaged parameter manifest, platform binary dirs, macOS `Info.plist` / `PkgInfo`, Web UI dist roots, packaged `Contents/Resources/ui`, `assets.manifest.json`, unsupported asset manifest version/root, manifest-listed symlink assets, and existing package output file slots before reading, writing, copying, hashing or serving | Real Windows/Linux validator and host scan |
| Examples: gain, midi-synth, web-ui-param-demo | Done | all examples compile; package/validator smoke recorded for macOS; Web UI demo has UI asset build scripts and meter subscription；`midi-synth` 展示 program list metadata、program-change 参数、controller-side program data JSON roundtrip、固定 SysEx level override、Note Expression brightness/tuning metadata 和 DSP 消费路径；example `PluginInfo` 不再包含占位 contact metadata；三个 examples 均包含 `params.specs.json` / `vesty-parameters.json` 并在 `vesty.toml` 引用参数 sidecar；当前 `smoke-host --strict` 看到 `midi-synth` sidecar 为 2 个参数；本机 macOS ParamID v2 package/static/validator smoke 证明三示例 bundle 均包含 `Contents/Resources/parameters.manifest.json`，且 validator 均为 47 passed / 0 failed | Windows/Linux packages and multi-DAW smoke |
| CI workflow | Implemented, Needs External Evidence | `.github/workflows/ci.yml` exists and parses; local equivalent commands pass；CI 会上传 doctor、protocol、publish-plan、`vesty-crate-package`、`vesty-dependency-baseline`、npm-pack、`vesty-smoke-host` diagnostic、package/static validate、release-check 和可选 `vesty-vst3-sdk-headers` artifact；package smoke 的三个 example static validate 命令已使用 `vesty validate --static-only --strict`，导出符号工具缺失/skipped evidence 会在 package job 阶段失败并仍写出 JSON report；该可选 SDK artifact 在有官方 SDK checkout 时包含 header manifest、generated-bindings plan/surface、metadata scaffold、ABI seed、ABI layout 和 interface skeleton；`release-evidence import-ci` 会内容验证并规范化到 release evidence 目录，但 scaffold/ABI seed/ABI layout/interface skeleton 仍只表示 drift/audit，不表示 bindings 已生成 | Actual GitHub Actions run URL and downloaded artifacts |
| Release evidence gates | Done | `vesty release-check` can require DAW matrix, protocol snapshot, CI doctor, CI per-OS release-check snapshots, macOS/Windows x64/Linux X11 platform smoke, validator reports, static validate reports/matrix, Vesty example parameter sidecar evidence, dependency latest baseline, publish/npm evidence, signing and notarization evidence；`--require-release-artifacts` 下缺失 `vst3 static validate reports` 汇总报告也会明确 failed，而不再作为 skipped 诊断项；platform smoke reports must use platform-specific WebView markers (`WebKit.framework`/`WKWebView`, `WebView2`, `WebKitGTK` + `X11`) and Steinberg/VST3 validator passed/0 failed summaries, so generic `system_webview=true` / `vst3_validator=true` markers are rejected; platform smoke `os` metadata remains optional but, when present, must match the authoritative `platform` field and Linux X11 metadata must not describe Wayland；VST3 validate/static reports must have self-consistent static/validator status fields before import/discovery/release-check use, so stale errors, contradictory positive evidence, unknown statuses, passed validator reports without path/exit/counts, and skipped validator reports with run fields are rejected；crate package readiness reports now embed their publish plan and must match package entries exactly; when release-check also has a publish-plan report, the two publish plans must match so artifacts from different commits/workspaces cannot be combined silently；publish-plan, crate-package, npm pack, release-check, release-action-plan, collect-local, import-ci and collected signing/notarization JSON now reject unknown top-level/entry/file/item fields before semantic validation, and nested release-check `daw_matrix` rows now reject unknown fields, missing required fields and non-string metadata through their shape validator even though they remain represented as `serde_json::Value`；CI doctor artifacts reject unknown check names and now also reject cross-OS check pollution after artifact OS is known, so Linux/macOS/Windows doctor reports cannot smuggle another platform's signing/notarization preflight checks while preserving legacy no-`os` reports via path OS inference；CI release-check snapshots now carry optional `os` metadata in newly generated reports, and CI artifact validation rejects path/report OS mismatch when that field is present while preserving legacy no-`os` reports through path inference；signing/notarization logs now reject contradictory failure evidence, so `codesign=pass` / `signtool=pass` / `status: Accepted` / `stapled=true` cannot override invalid signature, nonzero signtool error count, rejected/invalid notary status or stapler failure in the same evidence；Windows signing evidence now distinguishes signing from verification, so `Successfully signed` alone does not satisfy the Windows signtool verification gate；macOS `.vst3` directory evidence now rejects symlinked `Contents`, `_CodeSignature`, and `CodeResources` paths before plist parsing；ok static report paths must also belong to the same `.vst3` bundle named by `ValidateReport.bundle`, so moduleinfo/binary/export/parameter/asset evidence cannot be copied from another bundle or suffix-spoofed；CI per-OS release-check snapshots reject duplicate check names, forged host profile counts, vague protocol skip values and binding baseline values missing the current SDK/crate/backend baseline；`import-ci` now rejects mismatched explicit `--ci-run-url` / `--ci-run-url-file` sources, symlinked artifact source roots, existing symlink output dirs and source/output overlap, while `release-check --release-evidence-dir` rejects mismatched explicit `--ci-run-url` / directory `ci-run-url.txt` sources, symlinked evidence roots, symlinked `ci-run-url.txt`, and symlinks in template-standard fixed evidence files/directories instead of silently following external replaceable paths；explicit recursive artifact roots for CI doctor, CI release-check and platform smoke evidence reject symlinks before parsing, explicit release evidence files/logs such as validate reports、publish/npm/dependency/VST3 SDK reports、signing logs and notarization logs also reject symlinks before parsing, and `collect-signing` rejects symlinked bundle roots/payload dirs/explicit Windows binaries plus bundle-external or non-`.vst3` Windows binaries and explicit verification `--tool` leaf/parent symlinks before running verification tools while rejecting `--binary` outside Windows；dependency latest reports must also revalidate expected/actual equality and complete crates.io/npm latest coverage；imported release action plan sidecars must be structurally complete, summary-consistent, have nonempty/control-safe top-level protocol/evidence path metadata, unique `action.check` values, failed plans must contain at least one failed action, and all trim-detected `vesty ...` suggestions must parse against the current CLI; they remain checklist metadata, not pass evidence；generated action-plan `vesty ...` suggestions are parsed by current Clap definitions in tests so command/flag drift is caught before external evidence collection, and locally re-checkable evidence actions now include matching `--check` commands for publish-plan、crate-package、npm-pack、dependency latest baseline and VST3 SDK audit artifacts；VST3 SDK header manifest、generated-bindings readiness plan 和 generated-bindings symbol surface 作为 optional generated-headers audit evidence 缺失时 skipped、存在时 strict；metadata scaffold、ABI seed、ABI layout 和 interface skeleton 现在也可由 `release-check --vst3-sdk-scaffold/--vst3-sdk-abi-seed/--vst3-sdk-abi/--vst3-sdk-interface-skeleton` 或 `--release-evidence-dir/vst3-sdk/*.rs` 可选校验，存在时必须通过 marker/flag/metadata validators，缺失时 skipped；这些 `.rs` 文件仍仅作为 drift/audit metadata，不作为完整 bindings 或 final release pass evidence | Feed real evidence and require strict gate pass |
| Crash protection | Done Within Stated Limits | panic guard, COM boundary error handling, faulted silence and diagnostics/log event path tested；wry native IPC handler panic 被转换为 retryable bridge `internal_error` fallback；packaging JSON metadata serialization now returns `BuildError` instead of panicking；release evidence/platform smoke pending JSON templates and UI scaffold JSON string literal generation no longer rely on `expect()`；examples、README 和 `vesty new` Rust templates now use `ParamCollection::resolve_or_invalid()` so parameter ID typos in `create_kernel()` fall back to an invalid handle instead of panicking through host initialization；`vesty-ui-wry` unsafe block lint 已收紧；`vesty-vst3` crate root 和 COM binding file 显式 deny `unsafe_op_in_unsafe_fn` 与 `clippy::undocumented_unsafe_blocks` | Native crash isolation is out of current MVP; would need process isolation architecture |
| Wayland | Blocked By Scope/Evidence | Docs mark Linux X11 supported and Wayland experimental | Design/test Wayland embedding separately |

JSBridge message bus 本地证据补充: Rust native bridge 现在与 JS SDK / wry bootstrap 对齐 request id / replyTo 边界，`vesty-ipc` 提供 `MAX_BRIDGE_PACKET_ID_BYTES = 128` 和 `validate_bridge_packet_id()`，`BridgePacket::response_to()` / `error_to()` 只反射合法 id，`BridgeRuntime::handle_packet()` 在分发前拒绝缺失、空、超长或包含控制字符的 request id，recoverable parse-error 路径也不会回复畸形 id。`BridgeRuntime::handle_packet()` 也会在错误处理前 fail-closed 丢弃空、超长或控制字符 session，避免把畸形 session 反射到 response envelope；合法但 stale 的 session 仍返回 `permission_denied` 供调试/恢复。JS -> Rust runtime contract 现在只接受 request，伪造 response/event/ack/error 包会 fail-closed 丢弃，request 也不能携带 `replyTo` 或 `error`。新增 focused tests 覆盖 id validator、replyTo 清洗、bad request id 不分发、bad parse-error id 不响应、bad inbound session 不反射、stale valid session 仍可诊断、non-request 入站丢弃，以及 request envelope 字段污染被拒绝；`vesty-ipc` 为 6 passed，`vesty-bridge` 为 61 passed，bridge clippy 无 warning。

wry native IPC panic fallback 本地证据补充: `ipc_handler_panic_response()` 现在只会对合法 request envelope 生成 retryable `internal_error`，并复用 session/type/request-id validators；畸形 session/type/id、非 request kind、混入 `replyTo` 或 `error` 的 request 都会 fail-closed 不回包。`vesty-ui-wry --features wry-backend` 当前为 17 passed，feature clippy 无 warning。

Rust -> JS delivery 本地证据补充: `@vesty/plugin-ui` 与 wry bootstrap 的 `validInboundPacket()` 现在拒绝 server packet 携带 `id`，并拒绝 event/ack 混入 `replyTo` 或 `error`；`deliver()` 只把 `event` 派发给 subscription listener，`ack` 保留但静默忽略。新增 JS malformed inbound tests 覆盖 event/id、event/replyTo、event/error、ack 不派发和 response/id 不结算 pending；wry bootstrap 脚本断言同步覆盖。`npm test` 与 `vesty-ui-wry --features wry-backend` 均通过。

JSBridge flags 本地证据补充: `vesty-ipc` 现在定义并校验 packet flags 边界，最多 16 个 flag，每个 flag 非空、最长 64 bytes、无控制字符。Rust bridge request dispatch、wry panic fallback、`@vesty/plugin-ui` 和 wry bootstrap Rust -> JS delivery 都复用/镜像该规则；合法 `latest` meter flag 继续通过，畸形 flags fail-closed。`vesty-ipc` 当前为 7 passed，`vesty-bridge` 为 62 passed，`vesty-ui-wry --features wry-backend` 为 17 passed，plugin-ui tests passed。

JSBridge seq 本地证据补充: `vesty-ipc` 现在定义 `MAX_BRIDGE_PACKET_SEQ = 9_007_199_254_740_991`、`validate_bridge_packet_seq()` 和 `advance_bridge_packet_seq()`，把 Bridge envelope `seq` 固定在 JavaScript `Number.MAX_SAFE_INTEGER` 范围。Rust bridge request dispatch、recoverable parse-error fallback、wry native IPC panic fallback、`@vesty/plugin-ui` 和 wry bootstrap inbound delivery 都拒绝超出范围的 seq；Rust 和 JS 发送端到达上限后回绕到 `1`。新增 Rust/JS tests 覆盖 unsafe seq 拒绝和 outbound wrap；本轮验证通过 `vesty-ipc` 8 passed、`vesty-bridge` 65 passed、`vesty-ui-wry --features wry-backend` 17 passed、workspace Rust 609 passed、workspace clippy 无 warning、JS workspace tests passed、protocol export check passed 和 `vesty-vst3` wry-ui feature check passed。strict release-check 仍按预期失败，只缺真实 DAW/CI/platform/validator/static/signing/notarization evidence。

JSBridge fixed-payload 与 wry runtime manifest 本地证据补充: `BridgePacket` 顶层保持 forward-compatible，未知顶层字段仍可被忽略以支持未来协议扩展；`BridgeHelloPayload` 也保持公共握手扩展语义，Rust serde、JSON Schema 和 bridge runtime 均允许 hello payload 携带未来 JS capability 字段；`@vesty/plugin-ui` ready payload 也保留公共协议扩展语义，ready 顶层、capabilities、snapshot、param flags/kind 和 MIDI mapping 扩展字段不会阻断握手，且会保留在 cached ready payload 中；Rust `BridgeReadyPayload` 反序列化同样允许这些未知字段，导出的 ready payload JSON Schema 顶层也保持 extension-friendly；`bridge.hello.response.payload` accessor/getter 会在入站 packet guard 阶段 fail-closed 丢弃，不执行 getter、不发送 readyAck、不 settle ready promise，后续合法 response 仍可恢复握手。Vesty 内建 request payload 现在按命令固定 allowlist 校验，`bridge.readyAck`、subscription、state config/UI state 和 param begin/perform/end/format/parse 的额外字段或拼写错误字段都会返回 non-retryable `validation_error`；`snapshot.get`、`diagnostics.get`、`meter.flush` 和 `event.flush` 只接受无 payload、`null` payload 或空 object，SDK/bootstrap 发送 `{}` 的路径继续通过，字符串、数组、布尔、数字或非空 object payload 被拒绝。`vesty-ui-wry` release runtime asset manifest mirror 已对齐 build-time `version/root/entry/files` shape，并拒绝 unsupported version、invalid root 和顶层/asset file unknown fields；`vesty-build` static bundle validation 现在同样拒绝 unsupported asset manifest version 和 invalid root，但不要求 build-time `root` 匹配运行时 bundle 路径。新增 tests 覆盖 public envelope extension、hello payload extension、ready payload extension、ready payload accessor guard、内建 request payload unknown fields、fixed empty-payload query/flush guards、runtime asset manifest unknown fields，以及 static validate 的 asset manifest version/root 拒绝；本轮验证通过 `vesty-ipc` 14 passed、`vesty-bridge` 73 passed、`vesty-ui-wry --features wry-backend` 20 passed、`vesty-build` 80 passed、protocol export check、JS workspace tests、workspace Rust 690 passed 和 workspace clippy 无 warning；ready/hello payload extension、ready accessor、fixed empty-payload guards 与 Rust IPC/schema/runtime 回归后再次通过 `rtk npm test`、`rtk cargo test -p vesty-ipc -- --nocapture` 当前 16 passed、`rtk cargo test -p vesty-bridge -- --nocapture` 当前 75 passed、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`、`rtk cargo fmt --all --check`、`rtk cargo test --workspace -j1` 当前 694 passed 和 `rtk cargo clippy --workspace --all-targets -- -D warnings`。该补充仍是本地 IPC/WebView/bundle metadata boundary evidence，不替代真实 DAW、平台 WebView、validator/static、签名或 notarization evidence。

JSBridge error payload 本地证据补充: `@vesty/plugin-ui` 和 wry bootstrap 的 `validBridgeError()` 现在要求 Rust -> JS error packet 的 `code` 必须是协议枚举内的 `BridgeErrorCode`，未知 code 会 fail-closed 丢弃，不会 settle pending Promise。新增 JS malformed inbound case 覆盖 unknown error code 后续合法 response 仍能 resolve；wry bootstrap script test 固定 `BRIDGE_ERROR_CODES` 白名单。本轮验证通过 plugin-ui tests、wry bootstrap/fallback focused tests、workspace Rust 609 passed、workspace clippy 无 warning、JS workspace tests passed、protocol export check passed 和 `vesty-vst3` wry-ui feature check passed。strict release-check 仍按预期失败，只缺真实 DAW/CI/platform/validator/static/signing/notarization evidence。

JSBridge response/error 字段互斥本地证据补充: `@vesty/plugin-ui` 和 wry bootstrap 的 `validInboundPacket()` 现在拒绝 response packet 携带 `error`、error packet 携带 `payload`。新增 JS malformed inbound cases 覆盖 response+error 和 error+payload 后续合法 response 仍能 resolve；wry bootstrap script test 固定对应 guard。本轮验证通过 plugin-ui tests、`vesty-ui-wry --features wry-backend` 17 passed、workspace Rust 609 passed、workspace clippy 无 warning、JS workspace tests passed、protocol export check passed 和 `vesty-vst3` wry-ui feature check passed。strict release-check 仍按预期失败，只缺真实 DAW/CI/platform/validator/static/signing/notarization evidence。

JSBridge error message 本地证据补充: `vesty-ipc` 现在定义 `MAX_BRIDGE_ERROR_MESSAGE_BYTES = 2048` 和 `validate_bridge_error_message()`，`@vesty/plugin-ui` 与 wry bootstrap 的 `validBridgeError()` 镜像该上限。超长 error message 会 fail-closed 丢弃，不会 settle pending Promise。新增 JS malformed inbound case 覆盖 2049-byte message 后续合法 response 仍能 resolve；wry bootstrap script test 固定 message 长度 guard。本轮验证通过 `vesty-ipc` focused test、plugin-ui tests、`vesty-ui-wry --features wry-backend` 17 passed、workspace Rust 610 passed、workspace clippy 无 warning、JS workspace tests passed、protocol export check passed 和 `vesty-vst3` wry-ui feature check passed。strict release-check 仍按预期失败，只缺真实 DAW/CI/platform/validator/static/signing/notarization evidence。

JSBridge batch size 本地证据补充: `@vesty/plugin-ui` 与 wry bootstrap 的 `deliverBatch()` 入站现在要求 batch 是 array 且长度不超过 `MAX_BRIDGE_BATCH_PACKETS = 4096`；`vesty-ui-wry` Rust 端新增同名上限和 `batch_scripts()`，native IPC handler 返回 4097+ packet 时会拆成多个 `deliverBatch(...)` script 逐块回推，而不是生成一个会被 JS guard 整体忽略的超限 batch。新增 Rust tests 覆盖空 batch 与 4096+1 分块边界，wry bootstrap 测试绑定 JS 常量与 Rust 常量，JS SDK 测试覆盖 4097 忽略和 4096 处理。验证通过 `rtk cargo fmt --all --check`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture`、`rtk cargo check -p vesty-ui-wry --features wry-backend` 和 `rtk npm test`。这仍是本地 JSBridge/UI-thread robustness evidence，不替代真实 DAW/CI/platform evidence。

JSBridge outbound size 本地证据补充: `@vesty/plugin-ui` 与 wry bootstrap fallback 现在在 `postMessage()` 前对完整 packet 的 UTF-8 byte length 做 lane-specific size guard: command/param/event/meter/log/lifecycle 64 KiB，state 256 KiB。超限 WebView 请求返回 retryable `backpressure`，不进入 pending table、不调用 native IPC、不消耗 JS `seq`；合法后续请求仍保持连续 `js-N` / `seq`。JS tests 覆盖多字节 command payload 超限、后续合法请求仍为 `js-1`，以及较大 state payload 可通过 state lane 上限；wry bootstrap focused test 固定 constants、`maxMessageBytesForLane()`、`utf8ByteLength(message)` 和复用预序列化 `postMessage(message)`，并把 bootstrap 常量数值绑定到 `vesty-ipc` 的 Rust lane-size constants 防漂移。`.agents/12-jsbridge-design.md` 同步记录只有通过本地 shape/size 校验、即将发送的 request 才消耗 `seq`，TS SDK 和 wry bootstrap 中旧的未用 `nextSeq()` helper 已删除。验证通过 `rtk cargo fmt --all --check`、`rtk npm test`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture` 和 `rtk cargo check -p vesty-ui-wry --features wry-backend`。这仍是本地 bridge robustness evidence，不替代真实 DAW/CI/platform release evidence。

JSBridge async pump 本地证据补充: `@vesty/plugin-ui` 与 wry bootstrap 的 60 Hz async event pump 现在用 `eventFlushInFlight` 限制每个 bridge instance 同时最多一个 `event.flush` request；上一轮未 settle 时 interval tick 会跳过，settle/timeout/reject 后通过 `finally` 复位。stopEventPump 只停止 interval，不直接清零未 settle 的 in-flight 标记，避免退订后立刻重订阅绕过保护；unload cleanup 会 reject pending 后由 `finally` 释放。内部 `event.flush` 现在使用固定 1000ms watchdog，所以即使普通 `CreateBridgeOptions.timeoutMs = 0` 禁用常规 request timeout，异步事件泵也不会因为一个丢失 response 永久停摆；迟到的旧 flush response 会因 pending 已移除而被忽略，符合 meter/log/fault latest-wins 语义。JS tests 覆盖连续 tick 不堆积 pending flush、首个 flush 返回后才允许下一次发送、退订/重订阅仍不重复发送，以及 watchdog 超时后下一 tick 可恢复发送；wry bootstrap focused test 固定 skip guard、dedicated timeout、finally reset 和 stop 不清零 in-flight 标记。验证通过 `rtk npm test`、`rtk cargo test -p vesty-ui-wry -- --nocapture`、`rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture` 和 `rtk cargo check -p vesty-ui-wry --features wry-backend`。这仍是本地 UI-thread robustness evidence，不替代真实 DAW/host stress evidence。

VST3 adapter 本地证据补充: multi-output instrument 的 process path 现在还覆盖 host 只交付 main output bus、以及 host 交付 trailing empty inactive aux output bus 两种形态；optional sidechain effect 的 process path 也覆盖 host 只交付 main input bus、以及 host 交付 trailing empty inactive sidechain input bus 两种形态。新增 Sample64 scratch fallback fake-host tests 后，optional sidechain effect 在 `kSample64` 下同样覆盖 main-only 和 trailing empty inactive sidechain 两种形态，并验证实时路径 0 allocation、main path 输出和 silence flags。新增 native f64 sidechain fake-host test 后，`AudioKernel::SUPPORTS_F64 = true` 的 `kSample64` path 也覆盖直接通过 `ProcessContext64` 读取 main/sidechain f64 input、跳过 f32 fallback、NoAllocGuard active、0 allocation、输出和 silence flags 正确。真实 DAW 的 aux/sidechain routing/activation/native-f64 行为仍需外部 smoke evidence。

VST3 process boundary 本地证据补充: `process()` 现在会在构造 host input bus slice 前按插件声明 input bus 数量校验 `numInputs`，并拒绝非零 input count 搭配 null input pointer；也会在构造 per-bus channel pointer slice 前校验 `AudioBusBuffers::numChannels`，避免按异常 host 声称的超大 channel count 创建 raw slice；还会在事件收集和 developer kernel 前拒绝负 `numSamples`。fake COM 测试覆盖 host 传超量 input bus 时清零输出、oversized input channel count 时 DSP 看到空输入、oversized output channel count 时不进入 developer kernel、负 block size 时保留输出并设置 silence flags、返回 `kResultOk` 且不越过边界；sample64 over-capacity fallback 和 native f64 大 block 行为继续由既有测试覆盖。

Release evidence 本地证据补充: validate/static validate report parser 现在会按规范化 report path key 拒绝重复 binary/export evidence，因此 `Gain.vst3/...`、`./Gain.vst3/...` 和反斜杠写法不会被算作多个独立 evidence 条目；同一个规范化 key 也用于 strict static binary/export membership 匹配，保证合法路径变体可通过，但不能用路径拼写变体绕开 bundle 自洽检查。新增 focused tests 通过，完整 `vesty-cli` 当前为 283 passed；这仍不替代真实 DAW/CI/platform/validator/signing/notarization 外部证据。

Release example matrix 本地证据补充: example validator/static coverage gate 现在要求每份 Vesty example report 只证明一个 release platform；混入多个 platform binary 的单份 report 会失败，不能把一次 validator/static run 拼成多个平台 coverage。文件名中的平台标签如果存在，也必须与 `static_check.binaries` 推断的平台一致，例如 `VestyGain.windows-x64.validate.json` 不能携带 macOS binary evidence。平台标签识别使用完整 token 序列匹配，`windows-x64` / `windows_x64` 会被识别，而 `mywindowsx64note` 不会被误判为 Windows 标签。同一 `bundle@platform` 的重复 validator/static example reports 现在会失败，不会被静默去重。文件名中的 example bundle 标签也必须与 JSON `report.bundle` 一致，例如 `VestyGain.macos.validate.json` 不能携带 `VestyMIDISynth.vst3` report；bundle 标签使用完整 token 序列匹配并支持 compact token，因此 `VestyGain`、`Vesty-Gain` 和 `Vesty_Gain` 都会被识别，`MyVestyGainNote` 仍不会被误判，并且同一文件名包含多个 example bundle 标签会失败。新增 focused tests 通过，完整 `vesty-cli` 当前为 292 passed；真实 3x3 矩阵仍必须来自实际 macOS/Windows x64/Linux x64 validator/static artifacts。

VST3 COM boundary 本地证据补充: `IPluginFactory::getFactoryInfo()` / `getClassInfo()` 现在拒绝 null output pointer；`createInstance()` 拒绝 null class id/interface id/output pointer，并会在失败路径清空可写 output pointer；factory 创建 processor/controller 前都会校验参数 schema 与稳定 VST3 `ParamID` registry，重复/非法 schema 返回 `kResultFalse` 且 output pointer 保持 null；`IComponent::getControllerClassId()` 也会拒绝 null output pointer，并在正常 output pointer 下写入 controller CID；`IEditController` 参数 metadata/format/parse callbacks 拒绝 null host pointers 和负参数 index；`IPlugView` 拒绝 null platform/native parent handle。这些 fake COM 测试覆盖 validator/host 的坏指针 probe，不把空指针写入、stale instance 泄回宿主或误报 editor attach 成功。

VST3 String boundary 本地证据补充: controller 参数 parse 与 Note Expression parse 现在按 VST3 `String128` 固定 128 UTF-16 单元上限读取 host 输入；fake COM 测试覆盖非 NUL 结尾但边界内有效的输入，避免异常 host 输入触发无界 NUL 扫描。

VST3 lifecycle 本地证据补充: `IAudioProcessor::setProcessing(false)` 后的异常 `process()` 调用会在实时分配守卫内清零输出、设置 silence flags 且不进入 developer kernel；`setProcessing(true)` 后恢复正常处理。真实 DAW 的 start/stop、offline render 与 transport lifecycle 仍需外部 smoke evidence。

VST3 IO mode 本地证据补充: `IComponent::setIoMode()` 现在接受并记录 `kSimple`、`kAdvanced` 和 `kOfflineProcessing`，未知 mode 返回 `kInvalidArgument` 且保持上一有效状态；实际 per-block DSP mode 仍由 `ProcessData.processMode` 映射。真实 DAW 的 IO mode negotiation 仍需外部 lifecycle evidence。

VST3 setup 本地证据补充: `IAudioProcessor::setupProcessing()` 会拒绝 null pointer、unsupported sample size、非有限/非正 sample rate、非正或异常大的 `maxSamplesPerBlock`；fake COM 测试验证无效 setup 不会创建 kernel、不会调用 `prepare()`，也不会为 sample64 scratch 预分配异常大小。

## Release Gate Still Required

Before claiming complete multi-DAW release readiness, collect and run:

```bash
cargo run -p vesty-cli -- daw-matrix \
  --evidence-root target/daw-evidence \
  --format json \
  --strict

cargo run -p vesty-cli -- export-types \
  --out target/vesty-protocol \
  --check

cargo run -p vesty-cli -- release-evidence collect-local \
  --dir target/release-evidence \
  --protocol-snapshot target/vesty-protocol

# Optional generated-headers audit when an official SDK checkout is available:
cargo run -p vesty-cli -- release-evidence collect-local \
  --dir target/release-evidence \
  --protocol-snapshot target/vesty-protocol \
  --vst3-sdk-dir /path/to/VST_SDK \
  --vst3-sdk-bindings-module target/vst3-sdk/generated.rs

cargo run -p vesty-cli -- release-check \
  --evidence-root target/daw-evidence \
  --release-evidence-dir target/release-evidence \
  --protocol-snapshot target/vesty-protocol \
  --strict \
  --require-release-artifacts \
  --report target/release-evidence/release-check.json
```

The strict release gate must include:

- Protocol snapshot drift check from `vesty export-types --out target/vesty-protocol --check`; final `--require-release-artifacts` rejects `--skip-protocol`.
- REAPER、Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One scan/load/UI/UI->Host/meter stream/automation/buffer-sample-rate change/save-restore/offline render evidence.
- macOS、Windows x64、Linux X11 platform smoke JSON reports covering platform-specific system WebView evidence, Steinberg/VST3 validator passed/0 failed summary, example scan, WebView attach/resize, asset protocol, JSBridge roundtrip and nonzero meter stream; Linux Wayland remains experimental and is rejected for the final gate.
- Three examples times three validator platforms: macOS、Windows x64、Linux x64 Steinberg validator-passed reports, not only static-only validate reports; each example/platform report must include parameter sidecar evidence at the corresponding bundle `Contents/Resources/parameters.manifest.json` path and a self-consistent `ok` `static_check.binary_exports` entry with complete required/found symbols for the matching platform.
- Three examples times three package platforms: macOS、Windows x64、Linux x64 static validate JSON reports; each example/platform report must include parameter sidecar evidence at the corresponding bundle `Contents/Resources/parameters.manifest.json` path and a self-consistent `ok` `static_check.binary_exports` entry with complete required/found symbols for the matching platform; `skipped` export checks are diagnostics only.
- GitHub Actions run URL plus matching `doctor-Linux.json`、`doctor-macOS.json`、`doctor-Windows.json`.
- CI per-OS `release-check-Linux.json`、`release-check-macOS.json`、`release-check-Windows.json` snapshots with local invariant checks passing, self-consistent invariant values and matching GitHub repo/run id provenance.
- `vesty-publish-plan` artifact or `vesty publish-plan --out <path>` / `vesty release-evidence collect-local` report proving dependency-safe crate publish order.
- `vesty-crate-package` artifact or `vesty crate-package --out <path>` / `vesty release-evidence collect-local --crate-package` report proving currently packageable leaf crates pass real `cargo package` smoke while internal-dependent crates are explicitly deferred; ordinary local release-check may skip it, but final `--require-release-artifacts` requires it.
- `vesty-npm-pack` artifact or `vesty npm-pack --out <path>` / `vesty release-evidence collect-local` report proving JS package publish boundaries.
- `vesty-dependency-baseline` artifact containing `dependency-baseline-latest.json`, or an explicit `vesty dependency-baseline --latest --out <path>` report, proving the workspace external dependency coverage check plus crates.io/npm registry latest checks were run and matched the reviewed baseline, including all current external workspace Rust dependencies plus TypeScript and React/Vue/Svelte adapter dependencies.
- Optional `vesty-vst3-sdk-headers` artifact, `release-check --vst3-sdk-manifest <path>`, `release-check --vst3-sdk-binding-plan <path>` and `release-check --vst3-sdk-binding-surface <path>` audit evidence when generated-header inputs/readiness/surface are being tracked. Absence is skipped while the upstream `vst3` crate backend is active; any present manifest/plan/surface must be complete and valid, plan/surface must keep `bindingsGenerated = false` until a real emitter exists, and surface must keep `missingSymbols = []` plus `symbolPresent = true` for every required symbol, including program/unit and Note Expression symbols. Optional `vst3-sdk/generated.rs` scaffold, `vst3-sdk/generated-abi-seed.rs` ABI seed, `vst3-sdk/generated-abi.rs` ABI layout and `vst3-sdk/generated-interface-skeleton.rs` interface skeleton may be collected/imported or passed to `release-check` as deterministic drift/audit files; all four must keep generated-bindings/full-COM flags false, ABI layout must also keep `ABI_LAYOUT_GENERATED = true` plus size/alignment/field-offset fingerprints, interface skeleton must keep `INTERFACE_SKELETON_GENERATED = true` plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan/binary-export-inspection-tool-plan metadata, `INTERFACE_IDS`, `QUERY_INTERFACE_ENTRIES`, `QUERY_INTERFACE_IID_LOOKUP_SCOPE`, `interface_id_for_iid()`, `query_interface_entry_by_interface()`, `query_interface_entry_for_iid()`, `com_object_query_interface_dispatch_by_interface()`, `com_object_query_interface_dispatch_for_iid()`, `COM_OBJECT_INTERFACES`, `COM_OBJECT_INTERFACE_SCOPE`, `COM_OBJECT_IDENTITY_PLANS`, `COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES`, `FACTORY_EXPORT_PLAN`, `FACTORY_CLASS_PLANS`, `MODULE_EXPORT_PLANS`, `BINARY_EXPORT_SYMBOL_PLANS`, `BINARY_EXPORT_SYMBOL_PLAN_SCOPE`, `BINARY_EXPORT_INSPECTION_TOOL_PLANS`, `BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE`, `binary_export_inspection_tools()`, `BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED`, `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`, `binary_export_symbol_plan_by_platform_and_symbol()`, `required_binary_export_symbol_count()`, `first_missing_binary_export_symbol()`, `binary_export_required_symbols_present()`, key `*_IID` constants and `iid_from_words()` present, and none is proof that full SDK 3.8 bindings are generated or that plugin binaries were inspected.
- macOS codesign verification and Windows signtool verification. Generic `signed=true` / `signature=ok` markers are rejected because they do not prove either platform. `vesty release-evidence collect-signing` can normalize real logs, but cannot replace the external signing run itself.
- macOS notarization log proving notarytool accepted and stapler success. Generic `notarization=pass` / `notary=ok` markers are rejected because they do not prove notarytool accepted status. `vesty release-evidence collect-notarization` can normalize real notarytool/stapler logs, but cannot replace Apple notarization.

`vesty-smoke-host` / `vesty smoke-host` can be kept as a CI diagnostic artifact, but it is intentionally not part of the strict release evidence list above.

## Current Local Verification

Additional 2026-06-11 hardening verification:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli daw -- --nocapture
rtk cargo test -p vesty-ui-wry bootstrap_script -- --nocapture
rtk cargo test -p vesty-cli smoke_host -- --nocapture
rtk cargo test -p vesty-cli dependency_baseline -- --nocapture
rtk cargo test -p vesty-cli symlink -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo run -p vesty-cli -- release-check --format json --strict --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current.json
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json
```

Latest 2026-06-12 action-plan strict validator/static validate refresh:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan_uses_default_release_evidence_paths -- --nocapture
rtk cargo test -p vesty-cli release_action_plan_vesty_commands_parse_with_current_cli -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture
rtk cargo test -p vesty-cli example_validate_coverage_requires_all_release_platforms -- --nocapture
rtk cargo test -p vesty-cli example_static_validate_coverage_rejects_partial_platform_matrix -- --nocapture
rtk cargo test -p vesty-cli example_static_validate_coverage_requires_all_release_platforms -- --nocapture
rtk cargo test -p vesty-cli strict_validate_requires_ok_binary_export_evidence -- --nocapture
rtk cargo test -p vesty-cli validate_command_accepts_strict_flag -- --nocapture
rtk cargo test -p vesty-cli ci_package_static_validate_uses_strict_binary_export_gate -- --nocapture
rtk ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
rtk go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

The latest refresh keeps `vesty validate --strict`, validator-passed release evidence, CI package smoke and release action-plan guidance aligned. `release_action_plan_uses_default_release_evidence_paths` now asserts that `vst3 example validator coverage` recommends `vesty validate <bundle.vst3> --strict ...`, while both `vst3 static validate reports` and `ci example static validate coverage` recommend `vesty validate <bundle.vst3> --static-only --strict ...`. The release evidence template now tells manual validator collectors to replace pending reports with `vesty validate --strict --report`, so missing/skipped binary export evidence fails during collection rather than only at final aggregation. The release-check missing/partial coverage hints now also name `vesty validate --strict --report <path>` and `vesty validate --static-only --strict --report <path>`, so failed gates point collectors at the strict evidence path. YAML parsing and actionlint passed, full `vesty-cli` passed with 281 tests, full workspace Rust tests passed with 580 tests, clippy reported no issues, and JS workspace tests passed. The strict `release-check --require-release-artifacts` command still fails as intended because real DAW matrix, CI run/artifacts, platform smoke, validator/static coverage, signing and notarization evidence are not present.

The final full rerun after the matrix-command refresh passed `rtk cargo fmt --all --check`, `rtk cargo test -p vesty-cli -- --nocapture` (281 passed), `rtk cargo test --workspace -j1` (580 passed), `rtk cargo clippy --workspace --all-targets -- -D warnings`, `rtk npm test`, YAML parsing and actionlint. The refreshed `target/release-action-plan-current-require-artifacts.json` has summary 7 ok / 16 failed / 7 skipped / 23 actions. Its `vst3 validate reports`, `vst3 example validator coverage`, `vst3 static validate reports` and `ci example static validate coverage` actions each contain 11 commands: one generic strict command, one matrix note and nine concrete VestyGain/VestyWebUIDemo/VestyMIDISynth x macOS/Windows x64/Linux x64 commands. Validator commands use `vesty validate ... --strict ... --validator-log ...`; static commands use `vesty validate ... --static-only --strict ...`. The strict release-check report still fails only on missing external DAW/CI/platform/validator/signing/notarization evidence, while host profiles, protocol snapshot, VST3 binding baseline, crate publish plan, crate package readiness, npm pack report and dependency latest baseline are ok.

Follow-up hardening tightened `release_action_plan_uses_default_release_evidence_paths`: it now asserts every concrete 3x3 validator/static command appears exactly once for each required example/platform pair, with validator commands carrying `--strict` plus validator logs and static commands carrying `--static-only --strict` without validator logs. The same exact-once validator matrix assertion now covers both `vst3 validate reports` and `vst3 example validator coverage`, so the generic validator action and example coverage action cannot drift independently. Focused action-plan tests, full `vesty-cli` tests and `cargo clippy -p vesty-cli --all-targets -- -D warnings` passed after this change.

The action-plan command parsing regression test now reuses the production `split_release_action_command()` parser instead of a separate test-only splitter, so imported sidecar validation and generated command parsing tests exercise the same quote/comment/unterminated-quote behavior. Focused action-plan tests, full `vesty-cli` tests, full workspace Rust tests, workspace clippy, JS tests, YAML parsing and actionlint passed after this change. The strict release gate still fails as intended on missing external DAW/CI/platform/validator/signing/notarization evidence; the refreshed action plan remains 7 ok / 16 failed / 7 skipped / 23 actions, with all four validator/static actions still at 11 commands each.

The same regression test now also reuses production `release_action_command_starts_with_vesty()` after trimming each command, so leading-whitespace `vesty ...` suggestions are parsed in tests the same way they are validated for sidecar import/write. Focused command parsing and stale-command tests plus `cargo clippy -p vesty-cli --all-targets -- -D warnings` passed after this tightening.

The latest example-matrix hardening now also binds report file-name bundle labels to JSON `report.bundle`. A file named for `VestyGain` but containing `VestyMIDISynth.vst3` fails for both validator and static coverage, bundle label matching supports compact and tokenized spellings (`VestyGain`, `Vesty-Gain`, `Vesty_Gain`) without substring matching, and ambiguous names containing multiple known example bundle labels fail. Focused bundle-label tests, full `vesty-cli` tests (`292 passed`), `cargo clippy -p vesty-cli --all-targets -- -D warnings`, and `cargo fmt --all --check` passed. Strict `release-check --require-release-artifacts` still fails as intended only on missing external DAW/CI/platform/validator/static/signing/notarization evidence.

The latest dependency evidence refresh also passed. `vesty dependency-baseline --latest --check --out target/dependency-baseline-current/dependency-baseline-latest.json` verified the current Cargo and npm baselines against crates.io/npm registry latest data, including the requested core versions (`wry 0.55.1`, `vst3 0.3.0`, `raw-window-handle 0.6.2`, `rtrb 0.3.4`, `serde 1.0.228`, `serde_json 1.0.150`, `ts-rs 12.0.1`, `clap 4.6.1`). The strict release-check accepted that refreshed dependency report as `dependency latest baseline = ok`, while still failing on missing external DAW/CI/platform/validator/static/signing/notarization evidence.

The latest full local verification passed after the bundle-label and dependency evidence refreshes: `cargo test --workspace -j1` reported 594 passed, workspace Clippy reported no warnings with `-D warnings`, and `npm test` passed across `@vesty/plugin-ui`, `@vesty/react`, `@vesty/svelte` and `@vesty/vue`. This is local implementation evidence only; it still does not prove multi-DAW, cross-platform WebView, validator matrix, signing or notarization readiness.

The latest wry release WebView hardening also passed locally. Release navigation and IPC now only accept bundle custom-protocol asset URLs under `vesty://assets/...`; `about:blank` remains navigation-only, and the former `http(s)://vesty.assets/...` shim origin is rejected. Focused allowlist tests, full `vesty-ui-wry --features wry-backend` tests, feature clippy, full workspace Rust tests, workspace clippy, JS tests and formatting all passed. This strengthens the system-WebView release boundary but still needs real macOS/Windows/Linux host smoke to prove embedding behavior on each platform.

The latest JSBridge session-boundary hardening also passed locally. `vesty-ipc` now owns the shared 128-byte session shape rule, and `vesty-bridge::BridgeRuntime::new()/try_new()` rejects empty, overlong, or control-character initial sessions and `editorSessionId` values before the native bridge state machine is created. Focused IPC/bridge tests, protocol export check, VST3 wry feature check, full workspace Rust tests (`595 passed`), workspace clippy and JS tests passed. This aligns Rust authority with the existing JS SDK and wry bootstrap guards; it is still local bridge evidence, not DAW/platform release evidence.

Earlier hardening refreshes passed targeted DAW/smoke-host/dependency-baseline/symlink/release-check/release-action-plan/ci-doctor/platform-smoke/import-ci tests, the focused `vesty-ui-wry` bootstrap test, full workspace Rust tests, full workspace clippy and JS workspace tests. Those historical runs covered DAW evidence no-follow paths, JSBridge malformed inbound delivery, release action plan command drift, bounded CI doctor/release-check/platform-smoke metadata, and dependency latest baseline drift. The current latest verification is the refresh immediately above.

Latest local checks after protocol skip hardening, fake-host editor stress, CI doctor OS evidence hardening, CI per-OS release-check artifact gate, platform smoke release evidence gate, npm pack evidence gate, crate package readiness evidence gate, dependency baseline drift gate, headless smoke-host diagnostic gate, scaffold UI package publish safety, unsafe-block hardening, parameter schema gates, stable positive VST3 `ParamID`, generated parameter manifest sidecars, Vesty example parameter sidecar release evidence gate, packaged binary export static evidence, MIDI mapping / `IMidiMapping`, ParamID v2 Steinberg validator smoke, `publish-plan` release-order guardrail, VST3 SDK header input manifest, VST3 SDK generated-bindings readiness plan, VST3 SDK generated-bindings symbol surface, VST3 SDK metadata scaffold emitter, VST3 SDK ABI seed emitter, VST3 SDK ABI layout emitter, VST3 SDK interface skeleton emitter, VST3 SDK Unit/Program and Note Expression surface audit, VST3 SDK method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/factory-class-plan/module-export-plan/binary-export-symbol-plan metadata, VST3 SDK manifest/plan/surface/scaffold/ABI seed/ABI layout/interface skeleton release evidence and CI aggregation, local release evidence collection, and signing/notarization evidence collection helpers:

Additional 2026-06-10 VST3 SDK generated-header audit targeted checks:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo test -p vesty-cli import_ci -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test --workspace -j1
npm test
cargo build -p vesty-example-gain --release
cargo run -p vesty-cli -- package --config examples/gain/vesty.toml --platform macos --binary target/release/libvesty_example_gain.dylib --out target/binary-export-smoke
cargo run -p vesty-cli -- validate target/binary-export-smoke/VestyGain.vst3 --static-only --format json --report target/binary-export-smoke/gain-static-validate.json
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --check --strict
cargo run -p vesty-cli -- release-check --format json --strict --report target/release-check-binary-export-static-evidence.json
```

The formatting check, full workspace clippy gate, targeted VST3 SDK tests, `import_ci` / `release_evidence` tests, workspace Rust tests and JS tests all passed locally; `cargo clippy --workspace --all-targets -- -D warnings` now reports no issues and `cargo test --workspace -j1` now reports 430 passed. `smoke-host --check --strict` also passed and verified example configs, parameter sidecars, Web UI assets, JSBridge trace and meter marker. The packaged gain smoke validates that `static_check.binary_exports` can be produced from a real macOS `.vst3` bundle using `nm -gU` and that `_GetPluginFactory` plus macOS entry/exit aliases are present. The release evidence parser now also rejects malformed explicit binary export checks while still accepting legacy reports with no `binary_exports`; strict example validator/static matrices reject `skipped` export checks. These checks also verify that `ivstunits.h` / `ivstnoteexpression.h` are part of the generated-header locked input set, that Unit/Program and Note Expression symbols are present in the surface audit, that `generated-abi.rs` locks foundational Unit/Program and Note Expression `repr(C)` data layouts plus size/alignment/field-offset fingerprints, and that `generated-interface-skeleton.rs` contains method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/vtable-callback-field-layout-seed/vtable-field-offset-fingerprint/interface-id/query-interface-dispatch-plan/pure-IID-dispatch-lookup/com-object-interface-exposure-plan/com-object-identity-plan/per-object-query-interface-dispatch-plan/factory-export-plan/module-export-plan/binary-export-symbol-plan metadata while still keeping `BINDINGS_GENERATED = false` and `FULL_COM_BINDINGS_GENERATED = false`.

`release-check --format json --strict` failed as expected and wrote `target/release-check-binary-export-static-evidence.json`: host profiles, protocol snapshot and VST3 binding baseline were ok, while real REAPER/Cubase/Bitwig/Ableton/Studio One DAW smoke, platform smoke, validator matrix, CI, signing and notarization evidence remain missing/skipped. This is the intended release gate behavior, not a local compile/test failure.

Additional 2026-06-10 implementation-plan audit rerun:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -j1
npm test
cargo run -p vesty-cli -- --help
cargo run -p vesty-cli -- doctor --format json
cargo run -p vesty-cli -- release-check --format json --strict
cargo check --workspace -j1
rm -rf target/vesty-protocol-audit
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-audit
cargo run -p vesty-cli -- export-types --out target/vesty-protocol-audit --check
```

`cargo test --workspace -j1`、`npm test`、`cargo check --workspace -j1` 和 protocol export/check all passed. `release-check --format json --strict` failed as expected because real DAW smoke evidence is missing; it reported REAPER install/scan only, required the new `buffer_sample_rate_change` check, and kept Cubase/Nuendo、Bitwig Studio、Ableton Live、Studio One incomplete. This is the intended release gate behavior, not a framework compile/runtime failure.

Additional 2026-06-10 quality-gate rerun:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p vesty-vst3 --features "vst3-bindings wry-ui"
cargo clippy -p vesty-vst3 --all-targets --features "vst3-bindings wry-ui" -- -D warnings
cargo test -p vesty-ui-wry --features wry-backend
cargo clippy -p vesty-ui-wry --all-targets --features wry-backend -- -D warnings
cargo test --workspace -j1
npm test
```

`cargo clippy --workspace --all-targets -- -D warnings` now passes as a real workspace gate, and CI now also runs feature-specific clippy for `vesty-vst3` with `vst3-bindings wry-ui` and `vesty-ui-wry` with `wry-backend`. The cleanup replaced the long-argument `NoteExpressionValueType::new(...)` API with a small chainable constructor (`new(...).with_units(...).with_range(...).with_step_count(...).with_flags(...)`), migrated the `midi-synth` example and developer-guide snippet, fixed bridge/CLI clippy findings, and added missing VST3 unsafe-block `SAFETY:` comments plus equivalent iterator loops in the f64 scratch path. Follow-up verification passed: `cargo fmt --all --check`, workspace clippy, both feature-gated test/clippy pairs, `cargo test --workspace -j1` with 430 passed, and `npm test`.

```bash
cargo fmt --all --check
cargo check -p vesty-cli -j1
cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
cargo test -p vesty-cli vst3_sdk -- --nocapture
cargo test -p vesty-cli import_ci -- --nocapture
cargo test -p vesty-cli signing_evidence -- --nocapture
cargo test -p vesty-cli signing_verification -- --nocapture
cargo test -p vesty-cli notarization -- --nocapture
cargo test -p vesty-cli platform_smoke -- --nocapture
cargo test -p vesty-cli collect_local_release_evidence -- --nocapture
cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
cargo test -p vesty-cli release_evidence -- --nocapture
cargo test -p vesty-cli protocol_release_check -- --nocapture
cargo test -p vesty-cli release_check_requires_release_artifacts_when_requested -- --nocapture
cargo test -p vesty-cli smoke_host -- --nocapture
cargo test -p vesty-cli
cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
cargo run -p vesty-cli -- publish-plan --out target/publish-plan-smoke.json
cargo run -p vesty-cli -- publish-plan --check --out target/publish-plan-smoke.json
rm -rf target/crate-package-smoke
cargo run -p vesty-cli -- crate-package --out target/crate-package-smoke/crate-package.json
cargo run -p vesty-cli -- crate-package --check --out target/crate-package-smoke/crate-package.json
cargo run -p vesty-cli -- dependency-baseline --out target/dependency-baseline/dependency-baseline.json
cargo run -p vesty-cli -- dependency-baseline --check --out target/dependency-baseline/dependency-baseline.json
cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline/dependency-baseline-latest.json --format text
cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline/dependency-baseline-latest.json --format text
cargo run -p vesty-cli -- npm-pack --out target/npm-pack-smoke.json
cargo run -p vesty-cli -- npm-pack --check --out target/npm-pack-smoke.json
npm run build --prefix examples/web-ui-param-demo/ui
mkdir -p target/smoke-host
printf '%s\n' '{"type":"param.begin","result":0}' '{"type":"param.perform","result":0}' '{"type":"param.end","result":0}' 'result=0' > target/smoke-host/bridge-trace.log
printf '%s\n' 'meter_flush sent=1' > target/smoke-host/meter.log
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --strict
cargo run -p vesty-cli -- smoke-host --bridge-trace target/smoke-host/bridge-trace.log --meter-log target/smoke-host/meter.log --out target/smoke-host/smoke-host.json --check --strict
rm -rf target/release-evidence-local-smoke target/vesty-protocol-local-smoke
cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke
cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-local-smoke --protocol-snapshot target/vesty-protocol-local-smoke > target/release-check-strict-post-collect-local.json
cargo run -p vesty-cli -- release-check --strict --require-release-artifacts --skip-protocol --format json ... --report target/vesty-paramid-v2-smoke/release-check-strict-skip-protocol.json
cargo run -p vesty-cli -- release-check --strict --require-release-artifacts --format json --protocol-snapshot target/vesty-protocol --publish-plan-report target/publish-plan-smoke.json --crate-package-report target/crate-package-smoke/crate-package.json --npm-pack-report target/npm-pack-smoke.json ... --report target/vesty-paramid-v2-smoke/release-check-strict-local-artifacts.json
```

The `collect-local` strict release-check is expected to keep failing until real external evidence exists, but its protocol snapshot, crate publish plan and npm package pack report items should be `ok`; crate-package requires explicit `collect-local --crate-package` or `--crate-package-report` because it runs real package smoke, and final `--require-release-artifacts` now fails if that evidence is absent. The `--skip-protocol` strict release-check is expected to fail at `protocol snapshot`, proving final release artifacts cannot skip protocol drift. The local-artifacts strict check is also expected to keep failing until full external evidence exists; locally generated protocol, publish-plan, crate-package, npm-pack, macOS validator and macOS static reports are accepted as local inputs, but final validator coverage still requires the full macOS/Windows x64/Linux x64 example matrix, and final platform smoke still requires real macOS/Windows x64/Linux X11 evidence.

Additional 2026-06-10 local CI/release subchain rerun:

```bash
rtk npm run typecheck
rtk npm run build
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol-ci-local
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol-ci-local --check
rtk zsh -c 'npx tsc --strict --noEmit --target ES2022 --module ESNext --moduleResolution Bundler $(find target/vesty-protocol-ci-local/typescript -name "*.ts" | sort)'
rtk cargo run -p vesty-cli -- publish-plan --out target/publish-plan-ci-local/publish-plan.json
rtk cargo run -p vesty-cli -- publish-plan --check --out target/publish-plan-ci-local/publish-plan.json
rtk cargo run -p vesty-cli -- npm-pack --out target/npm-pack-ci-local/npm-pack.json
rtk cargo run -p vesty-cli -- npm-pack --check --out target/npm-pack-ci-local/npm-pack.json
rtk cargo run -p vesty-cli -- dependency-baseline --out target/dependency-baseline-ci-local/dependency-baseline.json
rtk cargo run -p vesty-cli -- dependency-baseline --check --out target/dependency-baseline-ci-local/dependency-baseline.json
rtk cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline-ci-local/dependency-baseline-latest.json --format text
rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-ci-local/dependency-baseline-latest.json --format text
rtk cargo run -p vesty-cli -- crate-package --out target/crate-package-ci-local/crate-package.json
rtk cargo run -p vesty-cli -- crate-package --check --out target/crate-package-ci-local/crate-package.json
rtk cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-ci-local --protocol-snapshot target/vesty-protocol-ci-local --crate-package --dependency-baseline-latest --format json
rtk cargo run -p vesty-cli -- release-check --format json --strict --protocol-snapshot target/vesty-protocol-ci-local --publish-plan-report target/publish-plan-ci-local/publish-plan.json --crate-package-report target/crate-package-ci-local/crate-package.json --npm-pack-report target/npm-pack-ci-local/npm-pack.json --dependency-baseline-report target/dependency-baseline-ci-local/dependency-baseline-latest.json --report target/release-check-local-ci-subchain.json
rtk cargo run -p vesty-cli -- release-check --format json --strict --release-evidence-dir target/release-evidence-ci-local --protocol-snapshot target/vesty-protocol-ci-local --report target/release-check-local-ci-evidence-dir.json
rtk cargo run -p vesty-cli -- release-check --format json --strict --release-evidence-dir target/release-evidence-ci-local --protocol-snapshot target/vesty-protocol-ci-local --plan target/release-action-plan-local-ci.json --report target/release-check-local-ci-with-plan.json
```

JS typecheck/build/tests, protocol export/check, exported protocol strict TypeScript compile, publish-plan check, npm-pack check, dependency baseline/check, dependency latest/check, crate-package/check and `release-evidence collect-local` all passed locally. The collected evidence directory contains protocol, publish-plan, crate-package, npm-pack and dependency latest reports, with dependency latest covering 44 baseline checks and 29 registry latest checks. Aggregate strict release checks still failed as intended because real external evidence is missing, while all local invariant checks were accepted: protocol snapshot, VST3 binding baseline, crate publish plan, crate package readiness, npm package pack report and dependency latest baseline were `ok`. The `--plan` run additionally wrote `target/release-action-plan-local-ci.json`, a machine-readable checklist of remaining failed/skipped evidence actions; it is not pass evidence and does not alter the release gate result.

Additional 2026-06-11 VST3 validate evidence hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli validate_report -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli example_coverage -- --nocapture
rtk cargo test -p vesty-cli static_validate -- --nocapture
```

The validate report release parser now rejects contradictory VST3 validate evidence before it can be imported from CI artifacts, auto-discovered from a release evidence directory, or accepted by `release-check`: `static_check.status = ok` cannot retain stale errors, failed static checks cannot carry module/binary/manifest evidence, passed validator reports require path/exit/test-count proof and cannot include stale reason/error, skipped/not-run/not-found validator reports cannot include run result fields, and unknown statuses are rejected. Ok static reports also bind moduleinfo, binaries, binary export checks, parameter manifests and asset manifests to the report bundle path; absolute paths and `./Bundle.vst3/...` are accepted, while cross-bundle evidence, suffix-spoofed paths, export checks for unlisted binaries and asset manifest/count mismatches are rejected. Targeted `vesty-cli` tests for validate reports, example coverage, release evidence discovery/import, release-check aggregation and static validate reports passed locally. Strict release readiness is still expected to fail until real DAW/platform/validator/CI/signing/notarization evidence exists.

Additional 2026-06-11 validate/smoke-host artifact shape hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli validate_report -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo test -p vesty-cli smoke_host -- --nocapture
```

The VST3 validate report parser now has a dedicated shape gate before static/validator semantics: top-level bundle/static/validator metadata is nonempty, control-safe, unsafe-Unicode-format-safe and bounded; validator stdout/stderr are bounded log fields that allow tab/newline but reject NUL/other control characters; static binary/export/symbol arrays have explicit count limits and duplicate detection. These checks run when writing validate reports, accepting release validator reports and accepting static validate reports, so malformed artifacts cannot be counted by release evidence import, auto-discovery or release-check.

`smoke-host` reports now also have a shape gate: generator/workspace/status/external note and each check name/status/value/hint are bounded and reject control/unsafe Unicode format characters; reports require 1-64 checks and reject duplicate normalized check names. Generated `smoke_host_ok/skipped/failed` values are sanitized to single-line bounded text, so local diagnostic errors can still be reported without producing malformed JSON evidence. Focused smoke-host and release-check tests passed after this change, followed by `rtk cargo test --workspace -j1` with 537 passed, `rtk cargo clippy --workspace --all-targets -- -D warnings`, and `rtk npm test`.

Additional 2026-06-11 publish/crate/npm report shape hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli publish -- --nocapture
rtk cargo test -p vesty-cli crate_package -- --nocapture
rtk cargo test -p vesty-cli npm_pack -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

Publish-plan reports now validate shape before write/check: package rows are bounded, skipped-private rows are bounded, package text fields reject empty/control/unsafe Unicode format/overlong values, and duplicate skipped-private/dependency entries are rejected before release evidence import or release-check can count them. Crate-package reports now apply the same boundary to generator/status/package fields/reasons/dependencies, cap package and dependency counts, reject duplicate dependencies, and sanitize captured `cargo package` stdout/stderr summaries into single-line bounded diagnostic text. NPM pack reports now cap package/file counts, reject malformed package name/version/filename/path text, and reject duplicate packed paths before the existing package allowlist and release-file checks.

Focused publish/crate/npm/release-check tests passed, followed by full workspace Rust tests with 538 passed, full workspace clippy with `-D warnings`, and JS workspace tests. This remains local release evidence hardening only; DAW/platform/CI/signing/notarization evidence is still external and still required for final release readiness.

The strict release gate was rerun with `--require-release-artifacts` and still failed as intended: host profiles, protocol snapshot, VST3 binding baseline and dependency latest baseline were ok, while real DAW matrix, platform smoke, CI artifacts, validator/static matrix, publish/npm evidence, signing and notarization evidence remained missing.

Additional 2026-06-11 DAW smoke marker input hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli daw -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

`write_daw_smoke_report()` now validates the host field as strict release metadata before host-profile lookup. Each required DAW smoke marker also passes a text boundary before semantic marker parsing: platform stays single-line metadata, while scan/load/UI/UI-host/meter/automation/buffer-save-render marker fields allow tab/newline for real DAW logs, are capped at 256 KiB, and reject NUL/other control characters plus unsafe Unicode format characters. The existing DAW pass/fail semantics remain unchanged, including pending/false/negative marker rejection, nonzero meter enforcement, render_file path safety and post-write parser verification.

Focused DAW tests now cover malformed host/platform/scan/ui/load marker text and multiline marker logs. Full workspace Rust tests passed with 540 tests, followed by full workspace clippy with `-D warnings` and JS workspace tests. This is still local evidence input hardening; it does not replace real Cubase/Nuendo, Bitwig, Ableton Live, Studio One, platform smoke, validator matrix, signing or notarization evidence.

The strict release gate was rerun with `--require-release-artifacts` after the DAW marker hardening and still failed only on the expected missing external evidence categories: real DAW matrix, platform smoke, CI artifacts, validator/static matrix, publish/npm evidence, signing and notarization. Host profiles, protocol snapshot, VST3 binding baseline and dependency latest baseline remained ok.

Additional 2026-06-11 signing/notarization evidence log hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli signing -- --nocapture
rtk cargo test -p vesty-cli notarization -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

Signing and notarization evidence logs now pass a bounded text gate before semantic parsing: logs must be nonempty, at most 512 KiB, may contain tab/newline for real tool output, and reject NUL/other control characters plus unsafe Unicode format characters. The macOS signed `.vst3` bundle evidence path now also rejects symlinked `Contents`, `Contents/_CodeSignature`, and `Contents/_CodeSignature/CodeResources` paths and caps that plist at 16 MiB before parsing. This does not alter positive/negative signing or notarization semantics; it only prevents malformed logs or internal symlink artifacts from being accepted as release evidence.

Focused signing/notarization/release-evidence/release-check tests passed, followed by full workspace Rust tests with 542 passed, full workspace clippy with `-D warnings`, and JS workspace tests. The strict release gate was rerun with `--require-release-artifacts` and still failed only on the expected missing external evidence categories; host profiles, protocol snapshot, VST3 binding baseline and dependency latest baseline remained ok.

Additional 2026-06-11 dependency/report audit shape hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports -- --nocapture
rtk cargo test -p vesty-cli collect_local_release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli signing -- --nocapture
rtk cargo test -p vesty-cli notarization -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json
```

Dependency baseline reports now validate shape before semantic acceptance and write: report metadata and per-check fields are bounded, control-safe and unsafe-Unicode-format-safe; hint text is capped at 64 KiB; reports are capped at 256 checks; duplicate `kind:name` check keys are rejected.

Release evidence audit metadata now has its own shape gate. `local-collect-report.json`, `import-ci-report.json`, and the JSON stdout reports for `collect-signing` / `collect-notarization` validate top-level evidence dir/source/workspace/output/kind/external note and item name/status/path/source/value fields before write/print. Reports are capped at 1024 items; local/signing/notarization reports require at least one item; import-ci allows an empty import but whitelists item statuses. Import diagnostics are sanitized to single-line bounded text before entering `import-ci-report.json`, so malformed parser errors cannot inject control characters, NUL, bidi/zero-width Unicode or unbounded text into audit metadata.

Focused release-evidence/import/signing/notarization tests passed, followed by full workspace Rust tests with 544 passed, workspace clippy with `-D warnings`, and JS workspace tests. The strict `--require-release-artifacts` gate still fails as intended: host profiles, protocol snapshot, VST3 binding baseline and dependency latest baseline are ok, while real DAW matrix, platform smoke, CI artifacts, validator/static matrix, publish/npm evidence, signing evidence and notarization evidence remain missing.

Additional 2026-06-11 VST3 SDK JSON artifact shape hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json
```

VST3 SDK manifest, generated-bindings plan and generated-bindings surface JSON now have explicit shape gates before write and before semantic acceptance in `release-check` / `import-ci`. Manifest metadata, header rows and missing-header rows are bounded and control-safe; header paths must be relative normalized paths; manifest header arrays are capped and duplicate headers are rejected. Binding plan metadata, embedded header manifest, checks, blockers and next steps are bounded; checks are capped and duplicate check names are rejected; next steps must be present. Binding surface metadata, embedded header manifest, header/symbol/blocker/note arrays and symbol rows are bounded; symbols are capped, notes must be present, duplicate `(name, kind, header)` rows are rejected and symbol header paths must be relative normalized paths.

Focused VST3 SDK/import/release-check tests passed, followed by full workspace Rust tests with 545 passed, workspace clippy with `-D warnings`, and JS workspace tests. This still does not mean SDK 3.8 bindings are generated: the VST3 SDK manifest/plan/surface artifacts remain optional audit/drift evidence, `bindingsGenerated` must remain false, and strict release readiness still fails until real DAW/platform/CI/validator/static/publish/npm/signing/notarization evidence exists.

Additional 2026-06-11 release-check report writer shape hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli ci_release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json
```

`release-check --report` now validates the report shape before writing JSON. The shared shape gate checks top-level status and optional GitHub Actions run URL, check count, check name/status/value/hint text boundaries, duplicate check names, status consistency with failed checks, and basic DAW matrix row shape. CI release-check artifact validation now reuses this shared shape gate before applying CI-specific local invariant checks.

Focused release-check/import/CI-release-check tests passed, followed by full workspace Rust tests with 546 passed, workspace clippy with `-D warnings`, and JS workspace tests. The strict release-check command still fails as intended but successfully writes the bounded report: local invariants are ok, while real DAW matrix, platform smoke, CI artifacts, validator/static matrix, publish/npm evidence, signing evidence and notarization evidence are still missing.

Additional 2026-06-11 CLI report print-boundary hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli doctor -- --nocapture
rtk cargo test -p vesty-cli smoke_host -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli publish -- --nocapture
rtk cargo test -p vesty-cli validate -- --nocapture
rtk cargo test -p vesty-cli dependency_baseline -- --nocapture
rtk cargo test -p vesty-cli npm_pack -- --nocapture
rtk cargo test -p vesty-cli crate_package -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json
```

`run_doctor()` now validates generated doctor reports before text/json output, and `print_smoke_host_report()` validates smoke-host reports before text/json output. The doctor generator now shares the same shape boundary used for CI doctor artifact acceptance, and smoke-host printing rejects malformed reports even when called outside the normal `run_smoke_host()` path.

The VST3 SDK audit print paths for manifest, generated-bindings plan and generated-bindings surface now invoke their existing shape gates before stdout/text output. Local/import/collected release evidence, crate package, dependency baseline, npm pack, publish plan and validate report print paths now also validate their existing report shape before printing. This keeps generation, write, import/release-check acceptance and stdout contracts aligned around bounded, control-safe and unsafe-Unicode-format-safe report data.

Focused tests passed across the touched CLI report families, followed by full workspace Rust tests with 546 passed, workspace clippy with `-D warnings`, and JS workspace tests. The strict release gate still fails as intended and writes the bounded report; local invariants are ok, while real DAW matrix, CI run URL, CI doctor/release-check artifacts, platform smoke, validator/static coverage, publish/npm evidence, signing evidence and notarization evidence remain missing.

Additional 2026-06-11 latest dependency and local evidence refresh:

```bash
rtk npm view vue version
rtk npm install --workspace @vesty/vue vue@latest --save-dev
rtk cargo test -p vesty-cli dependency_baseline -- --nocapture
rtk cargo run -p vesty-cli -- dependency-baseline --latest --out target/dependency-baseline-current/dependency-baseline-latest.json --format text
rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-current/dependency-baseline-latest.json --format text
rtk cargo run -p vesty-cli -- release-evidence collect-local --dir target/release-evidence-current --crate-package --dependency-baseline-latest --format json
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json
```

The live npm registry now reports `vue 3.5.38`. The Vue adapter keeps its devDependency range as `latest`, while `package-lock.json` / `node_modules` now resolve Vue and its `@vue/*` packages to `3.5.38`. The CLI dependency latest baseline expectation and test assertion were updated to `3.5.38`, and both the regenerated latest report and the follow-up `--check` run passed with 44 baseline checks and 29 registry latest checks.

`release-evidence collect-local` now successfully refreshes `target/release-evidence-current` with protocol snapshot, crate publish plan, crate package readiness, npm pack and dependency latest artifacts. A strict `release-check --require-release-artifacts --release-evidence-dir target/release-evidence-current` still fails as intended, but local evidence is auto-discovered and accepted: protocol snapshot, VST3 binding baseline, crate publish plan, crate package readiness, npm package pack report and dependency latest baseline are `ok`. Remaining failures are the external evidence categories: DAW matrix, CI run URL, CI doctor/release-check artifacts, platform smoke, validator/static coverage, signing and notarization.

After the refresh, `rtk cargo fmt --all --check`, `rtk cargo test -p vesty-cli dependency_baseline -- --nocapture`, `rtk npm test`, `rtk cargo test --workspace -j1` and `rtk cargo clippy --workspace --all-targets -- -D warnings` all passed locally; workspace Rust tests still report 546 passed.

Additional 2026-06-11 crate package readiness final-gate hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_check_requires_release_artifacts_when_requested -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

`crate package readiness` is now required when `release-check --require-release-artifacts` is used. Ordinary local checks can still show the missing report as skipped, but final release artifact mode fails unless `--crate-package-report` or `release-evidence-dir/crate-package/crate-package.json` is present and valid. Focused release-check/release-evidence tests passed, and the strict gate now reports `crate package readiness` as `failed` / required evidence missing when only dependency baseline evidence is provided.

With `target/release-evidence-current`, the same strict gate auto-discovers the crate package report and marks `crate package readiness` as `ok`, alongside protocol snapshot, VST3 binding baseline, crate publish plan, npm pack and dependency latest baseline. The remaining failures are still real external evidence categories: DAW matrix, CI run URL, CI doctor/release-check artifacts, platform smoke, validator/static coverage, signing and notarization. Full workspace Rust tests passed with 546 tests, clippy had no warnings, and JS workspace tests passed.

Additional 2026-06-11 release action plan final-gate sync:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-current/dependency-baseline-latest.json --format text
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

`release_action_plan_lists_required_and_optional_evidence` now asserts that final artifact mode includes `crate package readiness` as a required failed action when the report is missing, with `crate-package/crate-package.json` evidence path and a `vesty crate-package --out ...` command. This keeps the generated checklist aligned with the stricter `release-check --require-release-artifacts` gate. The generated `target/release-action-plan-current-require-artifacts.json` also contains `crate package readiness` as `failed` / `required` with `vesty crate-package --out target/release-evidence/crate-package/crate-package.json`.

The live latest dependency check still passes without lockfile changes. Current registry latest evidence matches the reviewed baseline for the Rust workspace dependencies and JS toolchain/framework adapters, including `wry 0.55.1`, `vst3 0.3.0`, `raw-window-handle 0.6.2`, `rtrb 0.3.4`, `serde 1.0.228`, `serde_json 1.0.150`, `ts-rs 12.0.1`, `clap 4.6.1`, `typescript 6.0.3`, `react 19.2.7`, `@types/react 19.2.17`, `vue 3.5.38` and `svelte 5.56.3`.

Focused tests passed: release action plan 7 passed and release_check 38 passed. Full workspace Rust tests passed with 546 tests, clippy reported no warnings, and JS workspace tests passed.

Additional 2026-06-11 release action plan default evidence paths:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

`release_action_evidence_path()` now defaults to `target/release-evidence` when no explicit release evidence dir is passed, matching the path already used by generated checklist commands. The generated action plan now carries machine-readable `evidence_path` entries such as `target/release-evidence/ci-run-url.txt`, `target/release-evidence/crate-package/crate-package.json`, `target/release-evidence/signing-macos.log and target/release-evidence/signing-windows.log`, and `target/release-evidence/notary.log`.

Focused tests passed: release action plan 8 passed and release_check 38 passed. Full workspace Rust tests passed with 547 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended because external DAW/platform/CI/validator/signing/notarization evidence is missing.

Additional 2026-06-11 validator/static action plan evidence paths:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

Validator and static validate checklist actions now point to the same release evidence directories that `import-ci` writes and `release-check --release-evidence-dir` auto-discovers. Validator actions use `target/release-evidence/validator` and suggest report/log paths like `target/release-evidence/validator/<bundle>.<platform>.validate.json`; static validate actions use `target/release-evidence/package` and suggest `target/release-evidence/package/<bundle>.<platform>.static-validate.json`. This makes the action plan reflect the required example/platform matrix instead of implying a single summary file is enough.

`release-check --write-evidence-template` now also creates `validator/` and `package/` directories and describes them as the recommended matrix slots. The root `validate-report.json` and `static-validate-report.json` pending files remain for legacy/single-plugin manual evidence, but the generated README now points framework releases toward `validator/<bundle>.<platform>.validate.json` and `package/<bundle>.<platform>.static-validate.json`.

Focused tests passed: release evidence template 3 passed, release action plan 8 passed and release_check 38 passed. Full workspace Rust tests passed with 547 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended because real DAW matrix, platform smoke, CI artifacts, validator/static coverage, signing evidence and notarization evidence are missing.

Additional 2026-06-11 validator/package symlink evidence boundary:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_dir_rejects_validator_and_package_symlink_dirs -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates_reject_validator_and_package_symlink_dirs -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

The new `validator/` and `package/` matrix directories are now covered by explicit symlink regression tests. `--release-evidence-dir` rejects symlinked matrix directories during recursive JSON discovery, and `--write-evidence-template` refuses to create template contents through existing symlinked matrix directories. Focused release evidence tests passed with 26 checks, full workspace Rust tests passed with 549 tests, clippy reported no warnings, and JS workspace tests passed.

Additional 2026-06-11 action plan overview evidence paths:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

The generated release action plan now includes machine-readable evidence paths for overview checks too. `daw matrix` points to the active evidence root, defaulting to `target/daw-evidence`, and `protocol snapshot` points to the protocol snapshot directory used by the release-check run. Focused release action plan tests passed with 8 checks, release_check tests passed with 38 checks, full workspace Rust tests passed with 549 tests, clippy reported no warnings, and JS workspace tests passed.

Additional 2026-06-11 VST3 SDK `.rs` audit artifact release-check loop:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-check` now has optional inputs for `--vst3-sdk-scaffold`, `--vst3-sdk-abi-seed`, `--vst3-sdk-abi` and `--vst3-sdk-interface-skeleton`, and `--release-evidence-dir` discovers the standard `vst3-sdk/generated*.rs` audit files. If those files exist or are passed explicitly, release-check reuses the existing marker validators and fails malformed artifacts; if absent, they remain skipped. They are still drift/audit metadata only, not proof that full VST3 SDK 3.8 bindings, callable COM glue, generated factory exports or binary export verification exist.

Focused tests passed: VST3 SDK 24 passed, release_evidence 26 passed, release_check 39 passed and release_action_plan 8 passed. Full `vesty-cli` tests passed with 255 tests, full workspace Rust tests passed with 550 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended while writing report and action plan; local invariant checks are ok, optional SDK `.rs` audit checks are skipped unless evidence exists, and missing real DAW/platform/CI/validator/signing/notarization evidence remains the release blocker.

Additional 2026-06-11 CLI output parent symlink boundary:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_check_report_writer_rejects_symlink_output_parent -- --nocapture
rtk cargo test -p vesty-cli release_check_report_writer -- --nocapture
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Generic `write_text_file()` now rejects existing symlink output files and symlinked output parent directories below the root-level system prefix. This covers release-check reports/action plans and other JSON/text/log writers while still allowing macOS system temp paths that pass through root-owned `/var` or `/tmp` symlinks. Focused report writer tests passed with 3 checks, release action plan passed with 8 checks, release_check passed with 40 checks, full `vesty-cli` passed with 256 tests, full workspace Rust tests passed with 551 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended because real DAW matrix, platform smoke, CI artifacts, validator/static coverage, signing evidence and notarization evidence are missing.

Additional 2026-06-11 import-ci destination parent symlink boundary:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_writers_reject_symlink_output_parents -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now rejects symlinked destination parent directories before importing copied files, generated text evidence, or directory contents such as signed `.vst3` bundles. Existing destination symlink files are still unlinked without following them when `--overwrite` is active, but a replaceable evidence subdirectory like `release-evidence/signing -> external` no longer receives imported artifacts. Focused import-ci writer regression passed with 1 check, the import_ci suite passed with 10 checks, full `vesty-cli` passed with 257 tests, full workspace Rust tests passed with 552 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended because real DAW matrix, platform smoke, CI artifacts, validator/static coverage, signing evidence and notarization evidence are missing. This is a local artifact trust-boundary hardening only; it does not synthesize release pass evidence.

Additional 2026-06-11 import-ci output root parent symlink boundary:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_symlink_output_parent -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now rejects a missing output root when any existing parent directory in that path is a symlink, before `create_dir_all()` can follow it. This complements the existing checks for symlinked `--source`, existing symlinked `--dir`, and symlinked imported artifact destination parents. Focused output-root parent regression passed with 1 check, the import_ci suite passed with 11 checks, full `vesty-cli` passed with 258 tests, full workspace Rust tests passed with 553 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended because real DAW matrix, platform smoke, CI artifacts, validator/static coverage, signing evidence and notarization evidence are missing. This is still only a local evidence-bundle trust-boundary hardening; it does not create external release evidence.

Additional 2026-06-11 evidence template ancestor symlink boundary:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli evidence_template_dirs_reject_nested_symlink_output_parent -- --nocapture
rtk cargo test -p vesty-cli evidence_templates -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Evidence template directory creation now checks every existing output ancestor below the root-level system prefix before `create_dir_all()`, so nested missing paths such as `linked-parent/missing/release-evidence` cannot follow an earlier symlinked ancestor. This applies to DAW evidence templates, platform smoke templates, release evidence templates and `collect-local` template initialization. Focused nested-parent regression passed with 1 check, evidence_templates passed with 8 checks, full `vesty-cli` passed with 259 tests, full workspace Rust tests passed with 554 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended because real DAW matrix, platform smoke, CI artifacts, validator/static coverage, signing evidence and notarization evidence are missing. Pending templates remain checklist scaffolding only and do not count as release evidence.

Additional 2026-06-11 live dependency latest re-check:

```bash
rtk cargo run -p vesty-cli -- dependency-baseline --latest --check --out target/dependency-baseline-current/dependency-baseline-latest.json --format text
```

The live dependency latest gate still passes against current crates.io/npm registry data. It verified 44 baseline checks and 29 latest registry checks, including `wry 0.55.1`, `vst3 0.3.0`, `raw-window-handle 0.6.2`, `rtrb 0.3.4`, `serde 1.0.228`, `serde_json 1.0.150`, `ts-rs 12.0.1`, `clap 4.6.1`, `typescript 6.0.3`, `react 19.2.7`, `@types/react 19.2.17`, `vue 3.5.38` and `svelte 5.56.3`. The refreshed report remains at `target/dependency-baseline-current/dependency-baseline-latest.json`. This only proves dependency baseline freshness; it does not replace DAW/platform/CI/validator/signing/notarization release evidence.

Additional 2026-06-11 CLI evidence/project output root hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli symlink_output_parent -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli collect_release_evidence_commands_reject_symlink_output_parents -- --nocapture
rtk cargo test -p vesty-cli create_project_rejects_symlink_output_parent -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence collect-local`, `collect-signing`, `collect-notarization`, `platform-smoke --write-report`, and `vesty new` now reject symlinked output parents before creating directories. This extends the existing report/template/import no-follow policy to the remaining local evidence collection and project scaffolding roots, so normalized evidence and starter projects cannot be redirected into a symlinked external directory. Focused regressions passed, `vesty-cli` passed with 262 tests, full workspace Rust tests passed with 557 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended while writing report/action plan: host profiles, protocol snapshot, VST3 binding baseline, crate publish plan, crate package readiness, npm package pack report, and dependency latest baseline are ok; real DAW matrix, CI run URL, CI doctor/release-check artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence, and notarization evidence remain missing.

Additional 2026-06-11 package output no-follow hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-build package_rejects_symlinked_output_dir -- --nocapture
rtk cargo test -p vesty-build package_rejects_existing_symlinked_ui_output_dir -- --nocapture
rtk cargo test -p vesty-build -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty-build::package_vst3()` now rejects symlinked package output ancestors before creating the `.vst3/Contents/Resources` tree, platform binary parent directories, or packaged UI asset output directories. Repackaging also rejects an existing symlinked `Contents/Resources/ui` before deleting the old UI tree, so a replaced packaged UI directory cannot cause `remove_dir_all()` to follow and delete an external target. Focused regressions passed, the full `vesty-build` suite passed with 73 tests, full workspace Rust tests passed with 559 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended while writing report/action plan: local invariants and local evidence remain ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence, and notarization evidence are still missing. This is local packaging-output trust-boundary hardening; it does not replace real Windows/Linux package smoke, validator, host scan, signing, or notarization evidence.

Additional 2026-06-11 package output file no-follow hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-build package_rejects_existing_symlinked_output_files -- --nocapture
rtk cargo test -p vesty-build package_rejects_existing_symlinked_parameter_manifest_output -- --nocapture
rtk cargo test -p vesty-build -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty-build::package_vst3()` now also rejects existing symlinked output files before copying/writing platform binaries, macOS `Info.plist` / `PkgInfo`, `moduleinfo.json`, configured `parameters.manifest.json`, `assets.manifest.json`, and copied UI asset files. `write_macos_plist()` serializes to memory and writes through the no-follow output writer, instead of using a direct path-based plist writer. Focused regressions passed, the full `vesty-build` suite passed with 75 tests, full workspace Rust tests passed with 561 tests, clippy reported no warnings, and JS workspace tests passed; each regression checks that the external symlink target remains unchanged. The strict release-check command still fails as intended while writing report/action plan: local invariants and local evidence remain ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence, and notarization evidence are still missing. This is still local packaging-output trust-boundary hardening and does not provide external release evidence.

Additional 2026-06-11 install-dev copy-mode no-follow hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli install_dev_bundle -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty package --install-dev --install-mode copy` now rejects a symlinked source `.vst3`, a symlinked install directory ancestor, and symlinked entries inside the source bundle before copying. Existing destination symlinks are unlinked without following them, including dangling symlinks, so replacing a development install cannot delete or overwrite an external target. Focused install-dev tests passed with 5 checks, the full `vesty-cli` suite passed with 265 tests, full workspace Rust tests passed with 564 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended while writing report/action plan: local invariants and local evidence remain ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence, and notarization evidence are still missing. Symlink install mode remains explicit: it creates the final bundle symlink only after the install root has been verified as a real directory.

Additional 2026-06-12 release evidence template wording and binary export helper audit refresh:

```bash
rtk cargo fmt --all --check
rtk npm test
rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

The release evidence template tests now assert the current boundary wording: interface skeleton artifacts provide pure required-symbol helpers and expected export-name plans, while binary inspection tooling and real release evidence remain absent. The stale generated-binary-export wording was removed from `.agents` docs. Focused release_evidence_templates passed with 4 checks, `vesty-cli` passed with 278 tests, full workspace Rust tests passed with 577 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended while writing report/action plan: local invariants are ok, while real DAW matrix, CI run URL/artifacts, platform smoke, validator/static coverage, signing evidence, and notarization evidence are still missing.

Additional 2026-06-12 `vesty validate --strict` binary export gate:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli strict_validate -- --nocapture
rtk cargo test -p vesty-cli validate_command_accepts_strict_flag -- --nocapture
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty validate` now accepts `--strict`. Default static validation still writes diagnostic reports when export-symbol inspection is skipped, while strict mode requires every inferred platform binary to have matching `ok` `static_check.binary_exports` evidence and returns nonzero after writing the report if evidence is missing/skipped/incomplete. Release evidence templates, release-check hints and action-plan commands now recommend `vesty validate --static-only --strict --report <path>` for package/static validate evidence. Focused strict/action-plan/template tests passed, `vesty-cli` passed with 280 tests, full workspace Rust tests passed with 579 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended: local invariants are ok, while real DAW matrix, CI run URL/artifacts, platform smoke, validator/static coverage, signing evidence, and notarization evidence are still missing.

Additional 2026-06-12 CI package smoke strict static validate alignment:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_package_static_validate_uses_strict_binary_export_gate -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture
rtk ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'
rtk go run github.com/rhysd/actionlint/cmd/actionlint@latest .github/workflows/ci.yml
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

The GitHub Actions package smoke job now passes `--strict` to all three `vesty validate --static-only` example bundle checks, so skipped export-symbol inspection fails in the package matrix instead of surfacing only at final release aggregation. A regression test reads `.github/workflows/ci.yml` and requires the static validate step to contain three validate commands, three `--static-only` flags and three `--strict` flags. YAML parsing and actionlint passed, `vesty-cli` passed with 281 tests, full workspace Rust tests passed with 580 tests, clippy reported no warnings, and JS workspace tests passed. The strict release-check command still fails as intended because real DAW matrix, CI run URL/artifacts, platform smoke, validator/static coverage, signing evidence and notarization evidence are still missing.

Additional 2026-06-12 JSBridge JSON payload/details hardening:

```bash
rtk cargo fmt --all --check
rtk npm --workspace packages/plugin-ui test
rtk cargo test -p vesty-ui-wry --features wry-backend bootstrap_script_registers_host_subscriptions -- --nocapture
rtk cargo test -p vesty-ui-wry --features wry-backend -- --nocapture
rtk npm test
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`@vesty/plugin-ui` and wry bootstrap now require Rust -> JS inbound `payload` and `error.details` to be bounded JSON-compatible values, and they use descriptor-safe packet/error reads so hostile direct calls to `window.__VESTY_INTERNAL__.deliver()` cannot trigger getters. JS -> Rust request payloads use the same guard, so `undefined`, functions, symbols, `NaN`, `Infinity`, cycles, accessor properties and non-plain objects fail before `JSON.stringify` can silently rewrite them. Param gesture helpers now omit absent `gestureId` fields instead of emitting `gestureId: undefined`. Focused JS and wry bootstrap regressions passed, `vesty-ui-wry` wry-backend passed with 17 tests, full workspace Rust tests passed with 610 tests, clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: host profiles, protocol snapshot, VST3 binding baseline, crate publish plan, crate package readiness, npm package pack report and dependency latest baseline are ok; real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence are still missing.

Additional 2026-06-12 Rust outbound Bridge error message hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-ipc bridge_error_payload_new_sanitizes_message -- --nocapture
rtk cargo test -p vesty-ipc -- --nocapture
rtk cargo test -p vesty-bridge -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`BridgeErrorPayload::new()` now sanitizes Rust-generated error messages at the protocol boundary: clean messages remain exact, control characters become spaces, and overlong messages are truncated at a UTF-8 character boundary to the 2048 byte WebView limit. The IPC constructor test covers clean/control/ASCII-long/multibyte truncation cases and verifies every sanitized result with `validate_bridge_error_message()`. `vesty-bridge` runtime error assertions now also check that emitted error messages satisfy the same validator. Focused IPC and bridge suites passed, full workspace Rust tests passed with 611 tests, clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: local protocol/package/dependency gates are ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 Rust outbound Bridge error details hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-ipc validates_and_sanitizes_bridge_json_values -- --nocapture
rtk cargo test -p vesty-ipc bridge_packet_error_to_sanitizes_error_details -- --nocapture
rtk cargo test -p vesty-ipc -- --nocapture
rtk cargo test -p vesty-bridge stale_state_set_config_returns_conflict -- --nocapture
rtk cargo test -p vesty-bridge stale_state_set_ui_state_returns_conflict -- --nocapture
rtk cargo test -p vesty-bridge -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Rust outbound error details now mirror the WebView JSON-compatible bounds in `vesty-ipc`: max depth 64, array items 65536, object keys 16384, nodes 262144, and string/key bytes 262144. `BridgeErrorPayload::set_details()` / `with_details()` sanitize details, and `BridgePacket::error_to()` performs a final details sanitize before emitting an error packet. Invalid details are downgraded to `{ "dropped": true, "reason": "..." }`, so a diagnostic error still reaches the UI instead of being discarded by the JS inbound validator. State conflict snapshot details remain intact and are now validated in bridge tests. Focused IPC and bridge regressions passed, full workspace Rust tests passed with 613 tests, clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: local protocol/package/dependency gates are ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 Rust outbound Bridge response/event payload hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-ipc bridge_packet_response_to_sanitizes_payload -- --nocapture
rtk cargo test -p vesty-bridge outbound_event_payload_is_sanitized_before_send -- --nocapture
rtk cargo test -p vesty-bridge latest_meter_payload_is_sanitized_before_flush -- --nocapture
rtk cargo test -p vesty-ipc -- --nocapture
rtk cargo test -p vesty-bridge -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Response and event payloads now share the same outbound JSON boundary as error details. `BridgePacket::response_to()` sanitizes response payloads, while `vesty-bridge` sanitizes reliable event, latest meter and RT log payloads before transport delivery. Invalid payloads are downgraded to `{ "dropped": true, "reason": "..." }`, so pending requests and subscribed listeners still receive a diagnostic packet instead of losing the message at JS inbound validation. Focused IPC/bridge regressions passed, full workspace Rust tests passed with 616 tests, clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: local protocol/package/dependency gates are ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 Rust inbound Bridge request payload hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-bridge request_payload_json_bounds_are_validated_before_dispatch -- --nocapture
rtk cargo test -p vesty-bridge request_flags_are_validated_before_dispatch -- --nocapture
rtk cargo test -p vesty-bridge -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty-bridge::validate_inbound_request_shape()` now validates parsed request payloads with the same `validate_bridge_json_value()` boundary before dispatching to bridge hello, state, param or subscription handlers. This closes the direct native IPC path where a page could bypass the JS SDK/wry bootstrap request guard and call `window.ipc.postMessage(...)` with an over-deep/oversized JSON payload. The regression uses a small over-depth payload to avoid the lane byte-size gate and proves the runtime returns a non-retryable validation error without acknowledging `bridge.hello`. Full workspace Rust tests passed with 617 tests, clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: local protocol/package/dependency gates are ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 Rust outbound Bridge topic/type hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-bridge invalid_packet_type -- --nocapture
rtk cargo test -p vesty-bridge -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty-bridge` now validates outbound reliable event, latest meter and RT log topics with the same packet `type` rule used by JS SDK, wry bootstrap and inbound native IPC. Invalid topics fail closed before send/queue, and `flush_latest_meters()` re-checks drained topics to protect against future internal direct insertion. Focused bridge tests passed with 4 invalid-topic regressions, the full `vesty-bridge` suite passed with 72 tests, full workspace Rust tests passed with 621 tests, workspace clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: local protocol/package/dependency gates are ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 DAW smoke evidence parser hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli contradictory_marker -- --nocapture
rtk cargo test -p vesty-cli generic_daw_evidence_accepts_explicit_smoke_markers -- --nocapture
rtk cargo test -p vesty-cli daw_matrix_write_report_accepts_reaper_generic_markers -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty daw-matrix` / `release-check` now reject contradictory manual DAW marker files when reading existing evidence directories, not only when using `--write-report`: a file containing positive markers plus `pending`, `false`, `failed`, `error`, `timeout`, `crashed` or similar negative evidence no longer counts as a pass. REAPER scan collection now prefers explicit scan evidence from the evidence directory; the local REAPER scan cache is only used when no explicit scan marker exists, so cache/install state cannot override a failed marker in release evidence. Focused DAW parser regressions passed, full `vesty-cli` passed with 294 tests, full workspace Rust tests passed with 623 tests, workspace clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: local protocol/package/dependency gates are ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 platform-smoke evidence parser hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli platform_smoke_rejects_contradictory_positive_values -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`vesty platform-smoke` / `release-check` now reject contradictory platform smoke check values when reading JSON reports: positive markers plus `pending`, `false`, `failed`, `error`, `timeout`, `crashed` or similar negative evidence no longer count as pass evidence. The missing-assignment parser also handles semicolon-separated markers on one line, so `jsbridge_roundtrip=true; roundtrip=false` is rejected. The `vst3_validator` check intentionally remains exempt from generic negative-word filtering because legitimate validator summaries include phrases like `0 failed`; it is still governed by the stricter validator-specific parser requiring Steinberg/VST3 validator identity, nonzero passed tests and zero failed tests. Focused and platform-smoke tests passed, full `vesty-cli` passed with 295 tests, full workspace Rust tests passed with 624 tests, workspace clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check command still fails honestly: local protocol/package/dependency gates are ok, while real DAW matrix, CI run URL/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing. No new real platform smoke evidence was generated by this hardening.

Additional 2026-06-12 inline signing/notarization marker hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli signing_evidence_rejects_contradictory_success_markers -- --nocapture
rtk cargo test -p vesty-cli notarization_evidence_rejects_contradictory_success_markers -- --nocapture
rtk cargo test -p vesty-cli explicit_truthy_marker_rejects_substring_keys_and_false_values -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli contradictory_marker -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

The shared explicit marker parser now evaluates semicolon-separated fragments on the same line. Inline contradictory signing and notarization evidence such as `codesign=pass; codesign=false`, `signtool=pass; signtool=failed`, `status: Accepted; status: Rejected` and `stapled=true; stapled=false` is rejected by the same negative-evidence path as multi-line contradictory logs. Focused signing/notarization parser regressions passed, platform smoke and DAW contradictory marker regressions still pass, and full `vesty-cli` remains at 295 passed.

Additional 2026-06-12 DAW bridge-trace auxiliary evidence hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli daw_evidence_rejects_contradictory_bridge_trace_markers -- --nocapture
rtk cargo test -p vesty-cli contradictory_marker -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

`collect_reaper_evidence()` and `collect_generic_daw_evidence()` now run `bridge-trace.log` auxiliary evidence through the same `daw_marker_positive()` contradiction gate before using it for UI->Host or meter-stream pass detection. A trace containing valid `param.begin/perform/end` or `meter.main` packets plus `failed`, `error`, `timeout`, `false`, `pending` or similar negative evidence no longer upgrades DAW matrix cells to pass. The new regression covers both generic DAW and REAPER collection, and full `vesty-cli` now passes with 296 tests.

Final local verification for this hardening pass also passed: `rtk cargo fmt --all --check`, `rtk cargo test --workspace -j1` (625 passed), `rtk cargo clippy --workspace --all-targets -- -D warnings`, `rtk npm test`, `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`, and `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`. The strict release-check with `--require-release-artifacts` still fails only because real external release evidence is absent: DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence. Local implementation/package/protocol/dependency gates remain ok.

Additional 2026-06-12 smoke-host diagnostic evidence hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli smoke_host_rejects_contradictory_bridge_and_meter_evidence -- --nocapture
rtk cargo test -p vesty-cli smoke_host -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

`vesty smoke-host --bridge-trace` and `--meter-log` now reuse the same contradiction gate as DAW marker evidence. Local diagnostic files containing otherwise valid `readyAck/reply`, param gesture or nonzero meter markers plus `failed`, `error`, `timeout`, `false`, `pending` or similar negative evidence no longer produce misleading ok checks. This only improves local diagnostic fidelity; `vesty smoke-host` still does not replace real DAW, platform WebView, Steinberg validator, signing or notarization evidence. Focused smoke-host tests passed, the smoke-host suite passed, and full `vesty-cli` now passes with 297 tests.

Final local verification for this smoke-host pass also passed: `rtk cargo fmt --all --check`, `rtk cargo test --workspace -j1` (626 passed), `rtk cargo clippy --workspace --all-targets -- -D warnings`, `rtk npm test`, `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`, and `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`. The strict release-check with `--require-release-artifacts` still fails only because real external release evidence is absent: DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence. Local implementation/package/protocol/dependency gates remain ok.

Additional 2026-06-12 validator-passed report log consistency hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli validate_report_rejects_validator_passed_log_summary_mismatch -- --nocapture
rtk cargo test -p vesty-cli validate_report_rejects_inconsistent_validator_passed_fields -- --nocapture
rtk cargo test -p vesty-cli validator_summary_extracts_passed_and_failed_counts -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

Validator-passed reports now cross-check parseable `validator.stdout` / `validator.stderr` summaries against the JSON `tests_passed` and `tests_failed` fields. A report that claims `tests_failed = 0` while retaining validator output such as `47 tests passed, 1 tests failed` is rejected instead of being accepted as release evidence. Minimal historical reports without parseable stdout/stderr summaries still rely on the existing strict fields: path present, exit code 0, nonzero passed count, zero failed count, and no error/reason. Focused validator report regressions passed, and full `vesty-cli` now passes with 298 tests.

Final local verification for this validator-report pass also passed: `rtk cargo fmt --all --check`, `rtk cargo test --workspace -j1` (627 passed), `rtk cargo clippy --workspace --all-targets -- -D warnings`, `rtk npm test`, `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`, and `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`. The strict release-check with `--require-release-artifacts` still fails only because real external release evidence is absent: DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence. Local implementation/package/protocol/dependency gates remain ok.

Additional 2026-06-12 platform-smoke VST3 validator runtime-failure hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli platform_smoke_requires_validator_identity_and_zero_fail_summary -- --nocapture
rtk cargo test -p vesty-cli platform_smoke_accepts_alternate_system_webview_and_validator_markers -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

Platform-smoke validator evidence still avoids generic `failed` filtering so legitimate `0 failed` summaries remain valid, but it now rejects validator runtime-failure terms such as `not found`, `missing`, `unavailable`, `not installed`, `failed to run`, `validator error`, `validator timeout/timed out` and `validator crashed`. This prevents a platform smoke value from combining a positive `47 passed, 0 failed` summary with an actual validator timeout/crash/error marker. Focused platform-smoke regressions passed, and full `vesty-cli` remains at 298 tests passed.

Final local verification for this platform-smoke validator pass also passed: `rtk cargo fmt --all --check`, `rtk cargo test --workspace -j1` (627 passed), `rtk cargo clippy --workspace --all-targets -- -D warnings`, `rtk npm test`, `rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check`, and `rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"`. The strict release-check with `--require-release-artifacts` still fails only because real external release evidence is absent: DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence. Local implementation/package/protocol/dependency gates remain ok.

Additional 2026-06-12 static validate static-only evidence hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli example_coverage_rejects_web_ui_report_without_asset_evidence -- --nocapture
rtk cargo test -p vesty-cli example_coverage_rejects_example_report_without_parameter_manifest_evidence -- --nocapture
rtk cargo test -p vesty-cli static_validate_reports_reject_validator_run_reports -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Static validate release evidence is now static-only by construction. `validate_static_validate_report()` rejects any report whose `validator.status` is not `skipped`, `not_run` or `not_found`, so validator-run reports with `passed` or `failed` cannot satisfy CI static package evidence. The new regression covers both passed and failed validator-run reports; related example coverage tests now use separate static-only fixtures when asserting missing asset or parameter-manifest evidence. Full `vesty-cli` passed with 299 tests, full workspace Rust tests passed with 628 tests, workspace clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check still fails honestly: local host/profile/protocol/package/dependency gates are ok, while real DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 validator-passed runtime-failure log hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli validate_report_rejects_validator_passed_runtime_failure_logs -- --nocapture
rtk cargo test -p vesty-cli validate_report_rejects_validator_passed_log_summary_mismatch -- --nocapture
rtk cargo test -p vesty-cli platform_smoke_requires_validator_identity_and_zero_fail_summary -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Validator-passed reports now reject runtime-failure evidence in `validator.stdout` / `validator.stderr` before they can count as release validator evidence. This catches logs that still contain `validator timeout`, `validator crashed`, `validator error`, `failed to run`, `not installed`, `unavailable`, `missing` or similar markers even if the JSON fields were edited to `tests_passed > 0` and `tests_failed = 0`. The helper is shared with platform-smoke validator parsing and still avoids generic `failed` filtering, so legitimate `0 failed` summaries remain valid. Full `vesty-cli` passed with 300 tests, full workspace Rust tests passed with 629 tests, workspace clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check still fails honestly: local host/profile/protocol/package/dependency gates are ok, while real DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 validator-failed report self-consistency hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli validate_report_rejects_contradictory_validator_failed_fields -- --nocapture
rtk cargo test -p vesty-cli static_validate_reports_reject_validator_run_reports -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Validator-failed reports now reject contradictory green-looking fields: partial test counts fail, and `status = "failed"` with exit code 0, zero failed tests and no runtime-failure marker is not considered self-consistent. A true runtime failure such as `validator timeout` remains valid failed diagnostic evidence. Full `vesty-cli` passed with 301 tests, full workspace Rust tests passed with 630 tests, workspace clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check still fails honestly: local host/profile/protocol/package/dependency gates are ok, while real DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 import-ci signing/notarization failure classification hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_reports_failed_signing_and_notarization_logs -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now reports actionable failed signing/notarization text artifacts as failed items instead of generic skipped text. Negative codesign/signtool logs, rejected notarytool logs and stapler failures are visible in `import-ci-report.json`, while ordinary unrelated text remains skipped. This improves CI artifact triage without turning failed logs into release pass evidence. Full `vesty-cli` passed with 302 tests, full workspace Rust tests passed with 631 tests, workspace clippy reported no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check still fails honestly: local host/profile/protocol/package/dependency gates are ok, while real DAW matrix, CI run/artifacts, macOS/Windows/Linux X11 platform smoke, validator/static coverage, signing evidence and notarization evidence remain missing.

Additional 2026-06-12 import-ci signed bundle directory failure classification hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_reports_failed_signed_bundle_directories -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_reports_failed_signing_and_notarization_logs -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

`release-evidence import-ci` now reports invalid `.vst3` bundle directories as failed signed bundle evidence instead of silently ignoring them. Placeholder or missing `Contents/_CodeSignature/CodeResources` is visible in `import-ci-report.json`, and invalid bundles are not copied into `signed-bundles/`. A valid macOS signed bundle fixture still imports normally. Full `vesty-cli` passed with 303 tests. This is a parser/evidence hygiene improvement only; it does not create real signing evidence.

Additional 2026-06-12 import-ci named JSON artifact failure classification hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_reports_failed_signed_bundle_directories -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now treats malformed or schema-mismatched JSON files with release-evidence names/paths as failed diagnostic items instead of generic skipped JSON. This covers doctor, release-check, release-action-plan, platform-smoke, validator/static-validate, publish-plan, crate-package, npm-pack, dependency-baseline and VST3 SDK artifact naming conventions. Ordinary unrelated JSON remains skipped, so downloaded notes/config files do not create noisy failures. Full `vesty-cli` passed with 304 tests, workspace Rust passed with 633 tests, clippy reported no warnings, JS tests passed, protocol export matched and the `vesty-vst3` binding/UI feature check passed. The strict release-check still fails honestly because real external DAW, CI, platform-smoke, validator/static coverage, signing and notarization evidence is missing. This only improves import triage; it does not synthesize CI, DAW, validator, signing or notarization pass evidence.

Additional 2026-06-12 import-ci auto-discovered CI run URL provenance hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_reports_invalid_auto_discovered_ci_run_url_file -- --nocapture
rtk cargo test -p vesty-cli ci_run_url -- --nocapture
rtk cargo test -p vesty-cli import_ci_release_evidence_normalizes_downloaded_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now reports malformed auto-discovered `ci-run-url.txt` / `ci_run_url.txt` provenance files as failed `ci run url` items instead of silently swallowing the parse failure during text-file scanning. Pending templates and ordinary notes/config text remain non-failing, while explicit `--ci-run-url-file` still errors strictly. Full `vesty-cli` passed with 305 tests, workspace Rust passed with 634 tests, clippy reported no warnings, JS tests passed, protocol export matched and the `vesty-vst3` binding/UI feature check passed. The strict release-check still fails honestly because real external DAW, CI, platform-smoke, validator/static coverage, signing and notarization evidence is missing. This improves CI artifact triage only; it does not create or replace a real GitHub Actions run URL.

Additional 2026-06-12 release-evidence standard report slot diagnostics hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_report_paths_for_diagnostics -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir_populates_standard_evidence_paths -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-check --release-evidence-dir` now preserves present standard report-slot paths for publish-plan, crate-package, npm-pack and dependency latest baseline even when their contents are invalid. The corresponding release-check items now fail with validator diagnostics instead of falling through to generic "required evidence missing". Valid standard paths are unchanged, and invalid standard files still cannot satisfy the gate. Full `vesty-cli` passed with 306 tests, workspace Rust passed with 635 tests, clippy reported no warnings, JS tests passed, protocol export matched and the `vesty-vst3` binding/UI feature check passed. The strict release-check still fails honestly because real external DAW, CI, platform-smoke, validator/static coverage, signing and notarization evidence is missing.

Additional 2026-06-12 release-evidence standard validate slot diagnostics hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_validate_reports_for_diagnostics -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-check --release-evidence-dir` now distinguishes pending template validate/static reports from malformed or invalid standard-slot reports. Template `validate-report.json` and `static-validate-report.json` remain ignored so a freshly initialized evidence directory is quiet. Replaced malformed/non-pending invalid reports are preserved in the release evidence options and fail through the validator/static gate with diagnostics instead of becoming generic missing evidence. Full `vesty-cli` passed with 307 tests, workspace Rust passed with 636 tests, clippy reported no warnings, JS tests passed, protocol export matched and the `vesty-vst3` binding/UI feature check passed. The strict release-check still fails honestly because real external DAW, CI, platform-smoke, validator/static coverage, signing and notarization evidence is missing.

Additional 2026-06-12 release-evidence standard signing/notarization slot diagnostics hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_signing_and_notary_logs_for_diagnostics -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates_do_not_count_as_pass_or_overwrite_logs -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-check --release-evidence-dir` now distinguishes pending template signing/notarization logs from non-template invalid standard-slot evidence. Template `signing-macos.log`, `signing-windows.log` and `notary.log` remain ignored while they contain pending markers, but replaced invalid logs are preserved and fail through the signed bundle / notarization gates with concrete diagnostics such as `invalid signature`, `number of errors: 1` or `status: rejected` instead of becoming generic missing evidence. Recursive text discovery still accepts only content-valid signing/notarization evidence to avoid noisy unrelated notes. Full `vesty-cli` passed with 308 tests, workspace Rust passed with 637 tests, clippy reported no warnings, JS tests passed, protocol export matched and the `vesty-vst3` binding/UI feature check passed. The strict release-check still fails honestly because real external DAW, CI, platform-smoke, validator/static coverage, signing and notarization evidence is missing.

Additional 2026-06-12 release-evidence standard VST3 SDK surface slot diagnostics hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_standard_vst3_sdk_surface_for_diagnostics -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk_binding_surface_release_check -- --nocapture
```

`release-check --release-evidence-dir` now preserves a present standard `vst3-sdk/generated-bindings-surface.json` or root `generated-bindings-surface.json` path even when the surface file is malformed or invalid. The VST3 SDK generated bindings surface gate now reports the concrete JSON/schema/baseline/coverage failure instead of the discovery layer silently treating the optional audit artifact as `not requested`. Absence still remains skipped, and this remains readiness/surface audit only; it is not proof that full SDK 3.8 bindings have been generated.

Full verification for this change also passed: `rtk cargo test -p vesty-cli -- --nocapture` reported 309 passed, `rtk cargo test --workspace -j1` reported 638 passed, clippy had no warnings, JS workspace tests passed, protocol export matched, and `vesty-vst3` with `vst3-bindings wry-ui` checked successfully. The strict release-check still fails honestly because the remaining failures require real external DAW, CI, platform smoke, validator/static coverage, signing and notarization evidence.

Additional 2026-06-12 recursive validate/static release-evidence diagnostics hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_recursive_validate_reports_for_diagnostics -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
```

`release-check --release-evidence-dir` now preserves invalid recursive validator/static report paths when their location or filename clearly identifies them as VST3 validate evidence, for example `validator/*.validate.json` or `package/*.static-validate.json`. Those paths now fail through the validator/static gates with concrete parse/schema/status diagnostics instead of being silently skipped and later reported as missing. The fallback excludes JSON that parses as another release evidence schema, so release-check/action-plan sidecars with `static-validate` in the filename are not misclassified as static validate reports.

Additional 2026-06-12 import-ci standard package/static validate directory diagnostics hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now treats JSON files under the standard `package/` evidence directory as VST3 static validate candidates even when the file name itself does not contain `static-validate`. Malformed or schema-mismatched `package/*.json` artifacts are now recorded as failed `vst3 static validate report` items in `import-ci-report.json` instead of being skipped as generic JSON. Full local verification passed: focused malformed named JSON import test, all `import_ci` tests, all `release_evidence_dir` tests, JS workspace tests, protocol export check, `vesty-vst3` `vst3-bindings wry-ui` check, workspace Rust tests with 639 passed, and workspace clippy with no warnings.

The strict release-check still exits failed by design. Current local ok gates are host profiles, protocol snapshot, VST3 binding baseline, crate publish plan, crate package readiness, npm package pack report and dependency latest baseline. Missing evidence remains external: real DAW matrix, GitHub Actions run URL and downloaded CI artifacts, macOS/Windows/Linux X11 platform smoke, three examples x three platforms validator and static validate reports, macOS/Windows signing verification and notarization/stapler logs.

Additional 2026-06-12 import-ci VST3 SDK JSON artifact classification hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_reports_malformed_named_json_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir -- --nocapture
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now classifies malformed or schema-mismatched VST3 SDK JSON artifacts using the specific release-check gate names when the path identifies the artifact: `vst3 SDK header manifest`, `vst3 SDK generated bindings plan`, or `vst3 SDK generated bindings surface`. Other unknown JSON under `vst3-sdk/` still uses the generic `vst3 SDK artifact` label. This does not make invalid SDK audit artifacts pass; it improves CI import triage so failed SDK audit evidence points at the exact gate. Full local verification passed with workspace Rust tests still at 639 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean.

The strict release-check still exits failed by design for missing external evidence. Local gates remain ok for host profiles, protocol snapshot, VST3 binding baseline, crate publish plan, crate package readiness, npm package pack report and dependency latest baseline; optional VST3 SDK audit artifacts are skipped because no current evidence directory was provided for them.

Additional 2026-06-12 CI release-check artifact crate-package diagnostics preservation:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_release_check_artifacts_preserve_crate_package_readiness_failures -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Per-OS CI `release-check-<OS>.json` artifacts now preserve a failed `crate package readiness` check as part of the CI release-check snapshot instead of rejecting the whole artifact as an unexpected local invariant failure. This keeps real CI package-readiness failures visible in the release evidence trail. It does not satisfy the independent `crate package readiness` release gate; that still requires valid `crate-package/crate-package.json` evidence. Local invariants remain strict: forged or failed host profiles, protocol snapshot, or VST3 binding baseline still reject the CI release-check artifact.

Full local verification passed with workspace Rust tests at 640 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean. Strict release-check still exits failed by design because external DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-12 CI release-check artifact platform-smoke diagnostics preservation:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_release_check_artifacts_preserve_platform_smoke_failures -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Per-OS CI `release-check-<OS>.json` artifacts now also preserve a failed `platform smoke artifacts` check as part of the CI release-check snapshot instead of rejecting the whole artifact as an unexpected local invariant failure. This keeps missing platform-smoke evidence visible when importing CI artifacts. It does not satisfy the independent platform-smoke release gate; final release still requires real macOS, Windows x64 and Linux X11 smoke reports.

Full local verification passed with workspace Rust tests at 641 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean. Strict release-check still exits failed by design because external DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-12 CI release-check artifact optional VST3 SDK audit diagnostics preservation:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_release_check_artifacts_preserve_vst3_sdk_audit_failures -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Per-OS CI `release-check-<OS>.json` artifacts now preserve failed optional VST3 SDK audit checks as CI diagnostics instead of rejecting the whole artifact as an unexpected local invariant failure. Covered gates: SDK header manifest, generated bindings plan, generated bindings surface, scaffold, ABI seed, ABI layout and interface skeleton. This does not satisfy those optional SDK audit gates; final release-check still skips them when absent and strictly fails them when present but invalid.

Full local verification passed with workspace Rust tests at 642 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean. Strict release-check still exits failed by design because external DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-12 CI release-check artifact filename discovery hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_release_check_artifacts_accept_case_insensitive_report_filenames -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

Recursive CI release-check artifact discovery now matches `release-check*.json` case-insensitively, so mixed-case artifact names such as `Release-Check-Linux.JSON`, `RELEASE-CHECK-macOS.Json` and `release-check-WINDOWS.json` are still discovered and counted toward Linux/macOS/Windows coverage. This improves evidence import robustness only; it does not synthesize CI release-check evidence or satisfy the missing external CI gate.

Full local verification passed with workspace Rust tests at 643 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean. Strict release-check still exits failed by design because external DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-12 CI release-check artifact path OS inference hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_release_check_artifacts_infer_os_from_parent_dirs -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts_infer_os_from_path_tokens_not_substrings -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

CI release-check artifact discovery now infers OS from the full artifact path, not only the filename. Directory-grouped downloads such as `Linux/release-check.json`, `macOS/release-check.json` and `Windows/release-check.json` satisfy Linux/macOS/Windows coverage. The path inference uses token aliases rather than raw substring matching, so unrelated names such as `swing-state/release-check.json` are not treated as Windows evidence just because they contain `win`.

Full local verification passed with workspace Rust tests at 647 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean. The strict release-check still exits failed by design because real DAW matrix, real GitHub Actions run/artifacts, platform smoke, three-example/three-platform validator and static reports, signing verification and notarization/stapler evidence remain missing.

Additional 2026-06-12 CI doctor artifact path OS inference hardening:

```bash
rtk cargo test -p vesty-cli ci_doctor_artifacts_infer_os_from_parent_dirs -- --nocapture
rtk cargo test -p vesty-cli ci_doctor_artifacts_infer_os_from_path_tokens_not_substrings -- --nocapture
rtk cargo test -p vesty-cli doctor_artifact -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

CI doctor artifact discovery now uses the same full-path OS inference as CI release-check artifact discovery. Directory-grouped downloads such as `Linux/doctor.json`, `macOS/doctor.json` and `Windows/doctor.json` satisfy Linux/macOS/Windows doctor coverage, while token matching prevents unrelated paths from becoming platform evidence through raw substring matches. Doctor reports that include an `os` field still must match the OS inferred from the artifact path.

This is evidence discovery hardening only. Final release readiness still requires a real GitHub Actions run URL plus downloaded doctor/release-check artifacts, and the strict release-check remains failed until the broader external evidence set exists.

Additional 2026-06-12 platform smoke artifact path consistency hardening:

```bash
rtk cargo test -p vesty-cli platform_smoke_release_check_accepts_platform_parent_dirs -- --nocapture
rtk cargo test -p vesty-cli platform_smoke_release_check_rejects_path_platform_mismatch -- --nocapture
rtk cargo test -p vesty-cli platform_smoke_path_platform_inference_requires_linux_x11_token -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
```

Platform smoke evidence now rejects artifact/report platform mismatches when the artifact path contains a platform token. This catches misplaced downloads such as `Windows/platform-smoke.json` containing a macOS report. Linux path inference requires both `linux` and `x11`, so Wayland or generic Linux directories do not satisfy the final Linux X11 release gate by path alone.

This is still local evidence consistency only. Real macOS, Windows x64 and Linux X11 platform smoke reports are still required before release readiness can pass.

Additional 2026-06-12 import-ci platform smoke path consistency hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_release_evidence_rejects_invalid_or_incomplete_artifacts -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now rejects platform smoke artifacts whose path platform token disagrees with the report platform before copying them into the normalized release evidence bundle. A misplaced artifact such as `Windows/platform-smoke.json` containing a macOS report is recorded as a failed `platform smoke artifact` item and is not copied to `platform-smoke/macos.json`. Linux path inference still requires both `linux` and `x11` tokens, so Wayland or generic Linux artifacts do not become final Linux X11 smoke evidence during import.

Full local verification passed with `vesty-cli` tests at 321 passed, workspace Rust tests at 650 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean. Strict release-check still exits failed by design because real DAW matrix, real GitHub Actions run/artifacts, platform smoke, validator/static reports, signing verification and notarization/stapler evidence remain missing.

This is import/discovery hardening only. Real macOS, Windows x64 and Linux X11 platform smoke reports are still required before strict release readiness can pass.

Additional 2026-06-12 import-ci VST3 validate/static path consistency hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_rejects_validate_artifact_path_platform_mismatch -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli example_coverage -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
rtk cargo run -p vesty-cli -- release-check --format json --strict --require-release-artifacts --release-evidence-dir target/release-evidence-current --dependency-baseline-report target/dependency-baseline-current/dependency-baseline-latest.json --report target/release-check-current-require-artifacts.json --plan target/release-action-plan-current-require-artifacts.json
```

`release-evidence import-ci` now checks platform consistency before normalizing VST3 validator-passed and static validate artifacts. If the file name or a dedicated parent directory clearly says `macos`, `windows-x64` or `linux-x64`, the report must include matching `static_check.binaries`; otherwise the artifact is recorded as a failed `vst3 validate report` or `vst3 static validate report` and is not copied into `validator/` or `package/`. File names that contain multiple platform labels now fail instead of falling back to "no clear platform". Broad CI job directory names such as `linux-vst3-static-validate/` are intentionally not treated as a hard platform signal, so multi-platform downloaded matrices still import normally.

Full local verification passed with `vesty-cli` tests at 322 passed, workspace Rust tests at 651 passed, JS workspace tests passing, protocol export matching, `vesty-vst3` binding/UI feature check passing, and clippy clean. Strict release-check still exits failed by design because real DAW matrix, real GitHub Actions run/artifacts, platform smoke, validator/static reports, signing verification and notarization/stapler evidence remain missing.

This is import/discovery hardening only. Real validator-passed and static validate reports for all examples/platforms are still required before strict release readiness can pass.

Additional 2026-06-12 signing evidence path consistency hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_rejects_signing_artifact_path_platform_mismatch -- --nocapture
rtk cargo test -p vesty-cli signing_evidence_rejects_path_platform_mismatch -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli signing_evidence -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
```

`release-evidence import-ci` and the final `signed bundle evidence` gate now reject signing evidence whose path platform token disagrees with the content. `Windows/signing.log` containing macOS `codesign` evidence, ambiguous `signing-macos-windows.log`, and a macOS signed `.vst3` bundle placed under `Windows/` are recorded as failed evidence instead of being normalized into the release bundle.

This is evidence consistency hardening only. Real macOS codesign, Windows signtool, and macOS notarization/stapler evidence are still required before strict release readiness can pass.

Additional 2026-06-12 notarization evidence path consistency hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli import_ci_rejects_notarization_artifact_path_platform_mismatch -- --nocapture
rtk cargo test -p vesty-cli notarization_evidence_rejects_path_platform_mismatch -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli notarization -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk npm test
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
```

`release-evidence import-ci` and the final `notarization log` gate now treat notarization/stapler evidence as macOS-only. Positive logs placed under `Windows/` or `Linux/` artifact paths are rejected with a platform mismatch diagnostic instead of being normalized to `notary.log`; file names or parent directories containing multiple platform labels, such as `notary-macos-windows.log`, are rejected as ambiguous evidence.

This is evidence consistency hardening only. Real accepted notarytool output plus stapler success evidence from macOS is still required before strict release readiness can pass.

Additional 2026-06-12 VST3 SDK binary export required-symbol plan API:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-vst3-sys binary_export -- --nocapture
rtk cargo test -p vesty-cli validate_report_binary_export_expectations_use_vst3_sys_plan -- --nocapture
rtk cargo test -p vesty-cli validate_report -- --nocapture
```

`vesty-vst3-sys` now exposes the binary export expected-symbol plan as public read-only API: `binary_export_symbol_plans()`, `binary_export_symbol_plan()`, `required_binary_export_tool_symbols()`, `required_binary_export_symbol_count()`, `first_missing_binary_export_symbol()` and `binary_export_required_symbols_present()`. `vesty-cli` validate/release report checks now reuse `vesty_vst3_sys::required_binary_export_tool_symbols()` instead of carrying a second hard-coded expected-symbol table.

This reduces drift between the generated-headers audit path and the real `vesty validate --strict` binary export gate. It still does not inspect binaries by itself; real `static_check.binary_exports` evidence remains required for strict release readiness.

Additional 2026-06-12 binary export plan single-source refresh:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-vst3-sys -- --nocapture
rtk cargo test -p vesty-build -- --nocapture
rtk cargo test -p vesty-cli validate_report -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo check -p vesty-vst3 --features "vst3-bindings wry-ui"
```

`vesty-build` now reuses `vesty-vst3-sys` for both required binary export symbols and export-symbol inspection tool ordering. `vesty-vst3-sys` also exposes `BinaryExportInspectionToolPlan` / `binary_export_inspection_tools()`, and the interface skeleton audit module now records `BINARY_EXPORT_INSPECTION_TOOL_PLANS` plus a pure lookup helper while keeping `BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED = false`. This is still audit metadata only; final release readiness still requires real `vesty validate --strict` binary export evidence plus DAW/CI/platform/signing/notarization evidence.

Additional 2026-06-13 DAW matrix platform evidence hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli daw_matrix -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

DAW matrix reading now revalidates `platform.txt` against each host profile instead of trusting a manually edited marker. Rows carry `platform_supported`; `daw-matrix --strict` and `release-check` treat unsupported, Wayland, generic Linux-without-X11, or host-unsupported platform text as missing `platform` evidence even if scan/load/UI/automation markers all pass. Release-check report shape validation also requires `platform_supported` to be boolean when present. This is evidence gate hardening only; real DAW/platform/validator/CI/signing/notarization evidence is still required for release readiness.

Additional 2026-06-13 DAW marker host-scope hardening:

```bash
rtk cargo test -p vesty-cli daw_matrix -- --nocapture
rtk cargo test -p vesty-cli daw_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
```

DAW marker logs that explicitly include `host=...`, `daw=...`, `daw_host=...`, `host_profile=...` or `profile=...` are now scoped to the matching Vesty host profile. A log marked `host=Ableton Live` no longer satisfies Bitwig evidence if copied into the Bitwig evidence directory, and `daw-matrix --write-report` rejects such mismatched marker text before writing normalized files. Logs without explicit host-scope fields remain backward-compatible. This is local evidence gate hardening only; it still does not provide real DAW smoke evidence.

Additional 2026-06-13 release-check DAW platform snapshot consistency:

```bash
rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
```

`release-check` report shape validation now cross-checks `daw_matrix[].platform_supported` against the row host profile and platform text. A report cannot claim `platform_supported=true` for Ableton Live on Linux X11, and cannot mark a supported macOS/Windows host platform as unsupported without failing validation. This protects imported or hand-edited release-check JSON snapshots from contradicting the same DAW platform rules used by `daw-matrix`.

Additional 2026-06-13 release-check DAW host-set consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`release-check` report shape validation now requires the `daw_matrix` snapshot to contain exactly the canonical release host set: REAPER, Cubase/Nuendo, Bitwig Studio, Ableton Live and Studio One. Missing rows, duplicate profiles, aliases such as `reaper`, unknown hosts and extra third-party hosts are rejected. The `host profiles` release-check item and CI per-OS release-check invariant validation now also require the value to exactly list the current canonical host set instead of trusting only a numeric coverage count. This is local evidence shape hardening only; third-party host smoke can be documented separately, and real multi-DAW smoke evidence is still required for release readiness.

Additional 2026-06-13 release-check DAW summary/detail consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`release-check` report shape validation now recomputes `host profiles`, `daw matrix` and every `daw smoke:<host>` check from the `daw_matrix` detail rows. Hand-edited or imported JSON cannot claim a matrix summary or per-host smoke status/value that contradicts the detail rows. The local `vesty-cli` suite now has 341 passing tests after this hardening. This still does not provide real DAW, CI, platform, validator, signing or notarization evidence.

Additional 2026-06-13 release evidence audit item semantics:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`local-collect-report.json`, collected signing/notarization reports and `import-ci-report.json` now enforce item status/path semantics. Local/collected `ok` items must point at evidence, import `ok`/`imported` items must include an output path, import `failed` items cannot claim an output path, and import `skipped` items may include an output path only for the destination-already-exists case. This makes audit metadata harder to misread as copied/pass evidence, but still does not replace real DAW, CI, platform, validator, signing or notarization artifacts.

Additional 2026-06-13 import-ci audit path containment:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`import-ci-report.json` shape validation now also checks lexical source/output containment. Imported artifact sources must be under the report's CI artifact source root, output paths must be under the report's release evidence directory, and root paths containing `..` are rejected so a report cannot widen its trusted root. Explicit `ci run url` source files remain allowed outside the artifact root because they may come from `--ci-run-url-file`. Collected signing/notarization reports now apply the same evidence-dir containment to their top-level output path and item paths. This is audit hardening only; real DAW, CI, platform smoke, validator, signing and notarization evidence is still missing for full release readiness.

Additional 2026-06-13 local release evidence report consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli collect_local -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`local-collect-report.json` shape validation now checks the report root and protocol summary/detail consistency. Non-protocol item paths must stay under the report evidence directory, while the protocol snapshot remains allowed to live in a separate `target/vesty-protocol` directory. A top-level `protocol_snapshot` must match exactly one ok `protocol snapshot` item path, and protocol items cannot appear without the top-level field. This is local audit hardening only; it still does not provide the external release evidence needed for final readiness.

Additional 2026-06-13 release action plan evidence-path consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Release action plan sidecars now cross-check known action `evidence_path` values against the plan's top-level `protocol_snapshot`, `evidence_root` and `release_evidence_dir`. DAW actions must point at the canonical host evidence subdirectories, release-evidence actions must point at the standard release evidence layout, and protocol actions must point at the protocol snapshot. This keeps checklist metadata from contradicting the commands and top-level roots, but it remains checklist metadata rather than release pass evidence.

Additional 2026-06-13 release action plan path-safety hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Release action plan sidecars now also reject lexical path escapes in top-level `protocol_snapshot`, `evidence_root`, `release_evidence_dir` and action `evidence_path` values. A plan cannot widen a root with `..` and then satisfy the expected-path equality check with a derived unsafe path. The valid compound `signed bundle evidence` path remains accepted as an exact expected string. Full `vesty-cli` passed with 342 tests, clippy reported no issues and protocol export still matches. This is local checklist/audit hardening only; real DAW, CI, platform smoke, validator/static coverage, signing and notarization evidence remain missing.

Additional 2026-06-13 local release evidence protocol path-safety hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli collect_local -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`local-collect-report.json` validation now treats the protocol snapshot exception as outside-the-evidence-dir but still path-safe: top-level `protocol_snapshot` and the matching `protocol snapshot` item path reject `..` parent-directory components. This prevents a self-consistent local audit report from naming an escaped protocol snapshot path while preserving the intended ability to store protocol output in `target/vesty-protocol`. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This is local audit report hardening only; release readiness still depends on the missing external evidence listed above.

Additional 2026-06-13 collected release evidence output consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli collect_notarization -- --nocapture
rtk cargo test -p vesty-cli signing_evidence -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Collected signing/notarization audit reports now require every ok item path to match the top-level `output`, in addition to staying under `evidence_dir`. This prevents a report from naming one normalized output while its detail row points at another standard evidence slot. The rule matches the current collection model, where one collection command produces one normalized evidence output. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This still does not create or prove real signing, notarization, DAW, CI, platform smoke or validator evidence.

Additional 2026-06-13 import-ci external CI run URL source path-safety hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli ci_run_url -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`import-ci-report.json` still allows the `ci run url` item source to live outside the downloaded artifact root when it came from an explicit `--ci-run-url-file`, but that external source path must now be lexically safe and reject `..` parent-directory components. This preserves the intended provenance exception without allowing unsafe path shapes in audit metadata. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This does not create a real GitHub Actions run URL or downloaded CI artifacts; those external release gates remain unproven.

Additional 2026-06-13 import-ci CI run URL item value consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli ci_run_url -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`import-ci-report.json` validation now requires a `ci run url` item with status `ok` or `imported` to carry the actual GitHub Actions run URL in `value`. Skipped no-URL and destination-exists cases keep their existing wording. This prevents imported provenance metadata from claiming a CI run URL while storing only generic copy/status text. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This is still audit hardening, not evidence that a real GitHub Actions run and artifacts have been collected.

Additional 2026-06-13 import-ci CI run URL output slot consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli ci_run_url -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`import-ci-report.json` validation now also requires the `ci run url` item output path to be exactly `evidence_dir/ci-run-url.txt`, matching the template and the real import writer. A report can no longer claim CI provenance while pointing the item at another file under the evidence directory. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This is local provenance/audit hardening only; it still does not provide a real CI run URL or downloaded CI artifacts.

Additional 2026-06-13 release evidence template item path consistency:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli collect_local -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Local and import-ci audit reports now require a `release evidence template` ok item path to equal the report `evidence_dir`, matching the actual template writer. Template initialization describes the evidence root as a whole, not an arbitrary child artifact path. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This is local audit hardening only; external release evidence remains missing.

Additional 2026-06-13 import-ci template item source semantics:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`import-ci-report.json` validation now rejects a `release evidence template` item with a `source`. Template creation is local bookkeeping for the evidence root, not an imported CI artifact. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This is local audit hardening only; it still does not provide external release evidence.

Additional 2026-06-13 release evidence template item status semantics:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli collect_local -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Local and import-ci audit reports now require `release evidence template` items to have status `ok`. Combined with the existing path/source rules, template entries now consistently describe local evidence-root initialization rather than imported, skipped or failed external evidence. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This is local audit hardening only; external release evidence remains missing.

Additional 2026-06-13 import-ci fixed standard output slot coverage:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`import-ci-report.json` validation now locks fixed-output items to their standard release evidence paths: protocol snapshot, publish/package/npm/dependency reports, VST3 SDK audit artifacts, and the notarization `notary.log` slot. Dynamic matrix/platform/bundle paths remain outside this fixed mapping. Full `vesty-cli` remains at 342 passing tests, clippy is clean and protocol export matches. This is local audit hardening only; external release evidence remains missing.

Additional 2026-06-13 import-ci successful output uniqueness:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
```

`import-ci-report.json` validation now rejects duplicate output paths among successful `ok` / `imported` items, so two successful evidence rows cannot both claim the same normalized artifact. `skipped` rows from the destination-exists/no-overwrite path remain allowed to share the destination path, preserving useful duplicate-artifact diagnostics. This is local audit report hardening only; real DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-13 import-ci signed evidence output shape:

```bash
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli signing -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
```

`import-ci-report.json` validation now gives `signed bundle evidence` its own dynamic output shape rule. Successful signing items may point only at `signing-macos.log`, `signing-windows.log`, `signing/<safe-name>.log`, or `signed-bundles/<safe-bundle>.vst3` under the release evidence root. This preserves real macOS/Windows/mixed-log/signed-bundle import semantics while rejecting arbitrary successful signing output slots. This is audit hardening only; it does not create real signing evidence.

Additional 2026-06-13 import-ci dynamic artifact output shape:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
```

`import-ci-report.json` validation now also locks dynamic-but-conventional artifact outputs to their production shapes: CI doctor reports, per-OS release-check snapshots, release action plan sidecars, platform smoke reports, VST3 validator reports and static validate reports. This prevents successful imported rows from pointing at arbitrary files under the evidence root while still preserving the generated dynamic file names. This is local audit hardening only; real CI artifacts, platform smoke and validator evidence remain missing until collected from external runs.

Additional 2026-06-13 release-check canonical gate set:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`release-check` report validation now requires the check-name set to match the current Vesty release gate exactly, including production gates and the current host-profile-derived DAW smoke checks. Missing current gates and unknown extra gates are rejected, while DAW matrix shape errors still keep their DAW-specific diagnostics. CI release-check artifact fixtures now reuse the production report builder and mutate existing gates for allowed external-evidence failures instead of appending duplicate checks. Full `vesty-cli` is at 343 passing tests, clippy is clean and protocol export matches. This is local release-check hardening only; real DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-13 release action plan canonical gate set:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Release action plan sidecars now require their summary total to match the current production release-check gate count, and every sidecar action check must belong to that current gate set. The expected gate set is derived from `build_release_check_report()` rather than a second handwritten list, so report validation and action-plan validation share one source of truth. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is action-plan/checklist metadata hardening only; real DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-13 local collect fixed evidence slots:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli collect_local -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`local-collect-report.json` validation now requires local publish/package/npm/dependency and VST3 SDK audit evidence items to use the same fixed release-evidence slots as the actual writer, rather than any path under the evidence root. Local and import-ci report validators share the fixed-slot mapping, while protocol snapshot and template entries keep their dedicated consistency rules. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is local audit report hardening only; real external release evidence remains missing.

Additional 2026-06-13 collected signing/notarization output slots:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli collect_notarization -- --nocapture
rtk cargo test -p vesty-cli signing -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`collect-signing` / `collect-notarization` JSON report validation now binds successful collected output to the standard release-evidence slots: `signing-macos.log`, `signing-windows.log`, or `notary.log`. The report still requires top-level output and item path to match, but explicit `--out` can no longer create successful evidence in paths that final release-check auto-discovery would miss. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is collected audit report hardening only; real signing and notarization runs remain external evidence.

Additional 2026-06-13 platform smoke canonical check set:

```bash
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Platform smoke report validation now requires the check-name set to match the current required platform smoke checks exactly. Unknown extras and missing required checks fail during report shape validation, after text safety/duplicate/count checks. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is platform smoke artifact schema hardening only; real macOS, Windows x64 and Linux X11 platform smoke evidence remains external and missing.

Additional 2026-06-13 smoke-host canonical check set:

```bash
rtk cargo test -p vesty-cli smoke_host -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Smoke-host diagnostic report validation now requires the check-name set to match the production `build_smoke_host_report()` shape derived from `SMOKE_HOST_EXAMPLES`: workspace manifest, example config/parameter sidecars, Web UI assets, JSBridge trace and meter stream. Unknown extras and missing required diagnostic checks fail during shape validation. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is local diagnostic artifact hardening only; smoke-host still does not replace real DAW, platform WebView, Steinberg validator, signing or notarization evidence.

Additional 2026-06-13 CI doctor unknown-check hardening:

```bash
rtk cargo test -p vesty-cli ci_doctor -- --nocapture
rtk cargo test -p vesty-cli doctor_report_includes_toolchain_webview_and_validator_checks -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Doctor report validation now rejects unknown check names outside the current doctor surface: toolchain, Node/npm, VST3 binding/SDK/validator, system WebView, platform signing/notarization preflight, unsupported-platform signing fallback and DAW install hints. Required-check coverage and accepted statuses remain enforced by the CI doctor artifact gate using the artifact path OS, preserving legacy reports without an `os` field. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is CI doctor artifact schema hardening only; real GitHub Actions artifacts and cross-platform doctor output still need to be collected externally.

Additional 2026-06-13 dependency baseline canonical check set:

```bash
rtk cargo test -p vesty-cli dependency_baseline -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Dependency baseline report validation now requires a canonical check set. Baseline-only reports must contain exactly the generated workspace dependency, VST3 SDK/crate and JS/npm lockfile baseline checks; reports that include registry latest checks must additionally contain the complete crates.io/npm latest set and no unknown checks. Missing required checks and unknown extra checks fail during shape validation, and the report writer now refuses to emit such malformed reports. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is dependency evidence schema hardening only; real registry review and external release evidence remain separate.

Additional 2026-06-13 VST3 SDK binding-plan canonical check set:

```bash
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

VST3 SDK generated bindings plan validation now requires the production three-check set: `sdk header inputs`, `bindings module path` and `binding emitter`. Unknown extra checks and missing required checks fail during shape validation, while content validation still enforces `bindings_generated = false`, current baselines/backend and reserved emitter semantics. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is optional SDK audit evidence hardening only; it still does not prove full SDK 3.8 Rust bindings are generated.

Additional 2026-06-13 VST3 SDK binding-surface canonical symbol set:

```bash
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

VST3 SDK generated bindings surface validation now derives the full expected symbol-name set from `REQUIRED_GENERATED_HEADER_INPUTS` and `generated_bindings_surface_symbol_names_for_header()`, then rejects unknown extra symbols and missing expected symbols. Existing checks for kind/header/purpose, token presence, missing-symbol consistency, required headers, notes and `bindings_generated = false` remain in place. Full `vesty-cli` remains at 343 passing tests, clippy is clean and protocol export matches. This is optional token-surface audit hardening only; it still does not parse C++ AST, verify ABI or prove full SDK 3.8 Rust bindings are generated.

Additional 2026-06-13 VST3 SDK binding-surface exact metadata:

```bash
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo clippy -p vesty-vst3-sys --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

`vesty-vst3-sys` now exposes `GeneratedBindingsSurfaceSymbolSpec` plus `generated_bindings_surface_symbol_specs()` as read-only audit metadata. CLI validation uses that production spec to require exact `name` / `kind` / `header` / `purpose` matching for every binding-surface symbol. Full `vesty-cli` remains at 343 passing tests, both `vesty-cli` and `vesty-vst3-sys` clippy are clean, and protocol export matches. This is optional token-surface audit metadata hardening only; it still does not parse C++ AST, verify ABI or prove full SDK 3.8 Rust bindings are generated.

Additional 2026-06-13 crate-package / publish-plan binding:

```bash
rtk cargo test -p vesty-cli crate_package -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Crate-package reports now embed the publish plan used to generate them, and validation requires package entries to match that embedded plan exactly. When `release-check` sees both crate-package and publish-plan evidence, it also requires the embedded plan to match the external publish-plan report, including order, level, name, version, manifest path and internal dependencies. Focused crate-package/release/import tests pass, full `vesty-cli` is now 345 passing tests, clippy is clean and protocol export matches. This is release evidence hardening only; real DAW, CI, platform smoke, validator/static, signing and notarization evidence remains missing.

Additional 2026-06-13 npm-pack unknown-field hardening:

```bash
rtk cargo test -p vesty-cli npm_pack -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

NPM pack report deserialization now uses `deny_unknown_fields` for package entries and file entries, so hidden fields such as package-level `scripts` or file-level `mode` fail before release-check/import-ci validation. Existing required package-set and `package.json`/`dist/**` file boundary checks remain unchanged. Full `vesty-cli` is now 346 passing tests, clippy is clean and protocol export matches. This is npm release evidence schema hardening only; it does not prove npm publish execution or any external release evidence.

Additional 2026-06-13 publish/crate report unknown-field hardening:

```bash
rtk cargo test -p vesty-cli publish_plan -- --nocapture
rtk cargo test -p vesty-cli crate_package -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Publish-plan and crate-package evidence structs now use `deny_unknown_fields` for top-level reports and package entries. Unknown fields such as `generated_by`, `generatedBy` or `checksum` fail during JSON parsing in check-mode/import/release-check paths. Full `vesty-cli` is now 348 passing tests, clippy is clean and protocol export matches. This is publish/crate release evidence schema hardening only; it does not produce any external release evidence.

The same final local pass also ran `rtk cargo test --workspace -j1` (682 passed), `rtk cargo clippy --workspace --all-targets -- -D warnings` and `rtk npm test`. These are local implementation checks only; they still do not replace real DAW, CI artifact, platform smoke, validator/static matrix, signing or notarization evidence.

Additional 2026-06-13 release-check/action-plan unknown-field hardening:

```bash
rtk cargo test -p vesty-cli release_action_plan -- --nocapture
rtk cargo test -p vesty-cli release_check -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Release-check reports, release-check items, release action plans, action-plan summaries and action items now use `deny_unknown_fields`; unknown fields like `generatedBy`, `owner` or `pending` fail at JSON parsing before sidecar/import validation. DAW matrix rows remain `serde_json::Value` and are still governed by the existing canonical host-set and row-shape validators. Full `vesty-cli` is now 350 passing tests, clippy is clean and protocol export matches. This is release artifact schema hardening only; it does not produce real CI or external release evidence.

Additional 2026-06-13 release evidence audit-report unknown-field hardening:

```bash
rtk cargo test -p vesty-cli release_evidence_audit_reports_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Collect-local reports, import-ci reports, collected signing/notarization reports and their local/import item records now use `deny_unknown_fields`. Unknown fields such as `generatedBy` or `owner` fail during JSON parsing before existing path, status, source/output, fixed-slot and collected-output validators run. Full `vesty-cli` remains 350 passing tests, clippy is clean and protocol export matches. These reports are provenance/audit metadata only; this does not create real CI, DAW, platform smoke, validator/static, signing or notarization evidence.

Additional 2026-06-13 validate/static report unknown-field and discovery hardening:

```bash
rtk cargo test -p vesty-cli release_evidence_dir_discovers_validate_reports_by_content -- --nocapture
rtk cargo test -p vesty-cli release_evidence_dir_keeps_invalid_recursive_validate_reports_for_diagnostics -- --nocapture
rtk cargo test -p vesty-cli validate_report -- --nocapture
rtk cargo test -p vesty-build -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo clippy -p vesty-build --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Validate reports, static bundle checks, validator checks and nested binary export checks now use `deny_unknown_fields`; hidden fields in `static_check.binary_exports[*]`, such as `checksum`, fail during JSON parsing before release-check/import-ci validation. Recursive release evidence discovery also excludes minimal release-check/action-plan sidecars before applying the validate/static filename fallback, so files such as `static-validate-release-check.json` are not misclassified as static validate evidence while genuinely malformed `validator/*.validate.json` or `package/*.static-validate.json` files remain available for diagnostics. Full `vesty-cli` remains 350 passing tests, `vesty-build` tests pass, clippy is clean and protocol export matches. This is validate/static evidence schema and discovery hardening only; it does not produce real DAW, CI, platform smoke, validator/static matrix, signing or notarization evidence.

Additional 2026-06-13 VST3 SDK audit JSON unknown-field hardening:

```bash
rtk cargo test -p vesty-cli vst3_sdk_json_artifacts_reject_malformed_shape_fields -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo clippy -p vesty-vst3-sys --all-targets -- -D warnings
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

VST3 SDK JSON audit artifacts now reject unknown fields at every generated JSON layer: SDK header manifests, header entries, generated bindings plans, plan checks, generated bindings surfaces and surface symbols. This prevents hidden `generatedBy`, `checksum`, `owner`-style fields from being ignored in optional SDK audit evidence. Full `vesty-cli` remains 350 passing tests, workspace Rust tests now pass with 684 tests, generated-bindings tests pass, workspace clippy is clean, JS workspace tests pass and protocol export matches. This is optional SDK audit schema hardening only; it still does not generate complete SDK 3.8 Rust bindings or external release evidence.

Additional 2026-06-13 bundle metadata JSON unknown-field hardening:

```bash
rtk cargo test -p vesty-build unknown_json_fields -- --nocapture
rtk cargo test -p vesty-build -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy -p vesty-build --all-targets -- -D warnings
rtk cargo clippy -p vesty-cli --all-targets -- -D warnings
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk npm test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
```

Bundle metadata JSON now rejects unknown fields in `assets.manifest.json`, `moduleinfo.json` and `parameters.manifest.json`, including nested asset file, module class and parameter manifest entries. Existing semantic checks for asset integrity, module metadata/class IDs and stable VST3 parameter IDs still run, but forged extra fields fail earlier during deserialization. `vesty-build` now passes 80 tests, `vesty-cli` remains 350 passing tests, workspace Rust tests pass with 687 tests, workspace clippy is clean, JS workspace tests pass and protocol export matches. This is local bundle metadata schema hardening only; it does not add real DAW, CI, platform smoke, validator/static matrix, signing or notarization evidence.

Additional 2026-06-13 JSBridge fixed-empty payload hardening:

```bash
rtk cargo test -p vesty-bridge builtin_request_payloads -- --nocapture
rtk npm --workspace @vesty/plugin-ui test
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk cargo fmt --all --check
```

The Rust bridge now has explicit regression coverage that fixed empty-payload built-ins (`snapshot.get`, `diagnostics.get`, `meter.flush`, `event.flush`) accept absent/null/empty-object payloads and reject non-object payloads such as strings, arrays and booleans. The public `BridgePacket` envelope still treats `payload: null` as equivalent to an omitted optional payload through the existing `Option<Value>` serde shape. `@vesty/plugin-ui` now sends `{}` for `getSnapshot()` and `getDiagnostics()`, matching the wry bootstrap helper behavior, and JS tests assert that packet shape. Focused bridge tests, the plugin-ui test/build, protocol export check and formatting all pass. This is local JSBridge boundary hardening only; it does not add external DAW, CI, platform smoke, validator/static matrix, signing or notarization evidence.

Additional 2026-06-13 param-manifest report unknown-field hardening:

```bash
rtk cargo test -p vesty-cli param_manifest -- --nocapture
rtk cargo fmt --all --check
```

`ParamManifestReport`, the JSON report shape printed by `vesty param-manifest --format json`, now uses `serde(deny_unknown_fields)`. The new regression test rejects top-level unknown fields such as `generatedBy`, aligning this CLI-owned report with the stricter release/evidence report policy used elsewhere. Focused `param_manifest` tests pass with 4 tests. This is local CLI report schema hardening only; it does not add external DAW, CI, platform smoke, validator/static matrix, signing or notarization evidence.

Additional 2026-06-13 release-check DAW matrix row unknown-field hardening:

```bash
rtk cargo test -p vesty-cli release_check_report_shape -- --nocapture
rtk cargo fmt --all --check
```

`validate_release_check_daw_matrix_shape()` now applies a fixed allowlist, required field set and metadata type check to each nested `daw_matrix` row, so unknown fields such as `generatedBy`, missing smoke fields such as `meter_stream`, and non-string `platform` / `evidence` metadata fail shape validation even though the row remains represented as `serde_json::Value`. The focused release-check shape suite passes with 10 tests. This is local release-check report schema hardening only; it does not add external DAW, CI, platform smoke, validator/static matrix, signing or notarization evidence.

Additional 2026-06-13 CI doctor cross-OS check-set hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_doctor_artifacts -- --nocapture
rtk cargo test -p vesty-cli import_ci_rejects_cross_os_doctor_artifacts -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk npm test
```

CI doctor artifact validation now rejects cross-OS check pollution after the artifact OS is known. A Linux doctor report cannot include macOS `codesign` / `notarytool` or Windows `signtool` checks, and legacy reports without an `os` label are still checked against the OS inferred from their artifact path. `import-ci` uses the same validation path and will not copy a failing doctor artifact into `release-evidence/ci-doctor/doctor-<OS>.json`. Focused CI doctor artifacts tests pass with 14 tests, full `vesty-cli` passes with 357 tests, full workspace Rust tests pass with 702 tests, workspace clippy is clean, protocol export matches and JS workspace tests pass. This is local CI doctor evidence hardening only; it does not add real GitHub Actions artifacts or other external release evidence.

Additional 2026-06-13 platform smoke OS metadata consistency hardening:

```bash
rtk cargo test -p vesty-cli platform_smoke -- --nocapture
rtk cargo test -p vesty-cli platform_smoke_rejects_os_metadata_platform_mismatch -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk npm test
rtk cargo fmt --all --check
```

Platform smoke reports now reject explicit `os` metadata that contradicts the authoritative `platform` field. The field remains optional for legacy artifacts, but if it is present, macOS/Windows/Linux X11 reports must describe the matching platform; Linux X11 `os` metadata must include X11 and must not describe Wayland. Focused platform smoke tests pass with 20 tests, full `vesty-cli` passes with 358 tests, full workspace Rust tests pass with 703 tests, workspace clippy is clean, protocol export matches, JS workspace tests pass and formatting is clean. This is local platform-smoke metadata hardening only; it does not add real macOS, Windows x64 or Linux X11 platform smoke evidence.

Additional 2026-06-13 CI release-check OS metadata hardening:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli ci_release_check_artifacts -- --nocapture
rtk cargo test -p vesty-cli release_check_report_shape_validates_optional_os_label -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk npm test
```

Newly generated release-check reports now include optional `os` metadata (`Linux`, `macOS` or `Windows`). Shape validation rejects unrecognized or control-character OS labels, and CI release-check artifact validation rejects path/report OS mismatches when the field is present. Legacy reports without `os` are still accepted through artifact path OS inference. Focused CI release-check artifact tests pass with 15 tests, full `vesty-cli` passes with 361 tests, full workspace Rust tests pass with 706 tests, workspace clippy is clean, protocol export matches, JS workspace tests pass and formatting is clean. This is local CI release-check metadata hardening only; it does not add real GitHub Actions artifacts or other external release evidence.

Additional 2026-06-13 generic signing marker hardening:

```bash
rtk cargo test -p vesty-cli signing_evidence_rejects_generic_platformless_markers -- --nocapture
rtk cargo test -p vesty-cli import_ci_rejects_generic_platformless_signing_markers -- --nocapture
rtk cargo test -p vesty-cli signing -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
```

Signing evidence no longer accepts platformless `signed=true`, `signing=pass` or `signature=ok` markers as positive evidence. The signing gate and `import-ci` now require evidence attributable to macOS `codesign` or Windows `signtool` / verify summaries, and clearly signing-named weak logs are reported as failed rather than imported. Focused signing tests pass with 37 tests, import-ci tests pass with 20 tests and release-evidence tests pass with 33 tests; the final local rerun also passed formatting, full `vesty-cli` with 363 tests, full workspace Rust with 708 tests, workspace clippy, protocol export check and JS workspace tests. This is local parser/import hardening only; it does not add real macOS codesign or Windows signtool verification evidence.

Additional 2026-06-13 generic notarization acceptance marker hardening:

```bash
rtk cargo test -p vesty-cli notarization_evidence_rejects_generic_acceptance_markers -- --nocapture
rtk cargo test -p vesty-cli notarization_evidence -- --nocapture
rtk cargo test -p vesty-cli release_evidence_templates -- --nocapture
rtk cargo test -p vesty-cli import_ci_rejects_generic_notarization_acceptance_markers -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
```

Notarization evidence no longer accepts platformless `notarization=pass` or `notary=ok` markers as accepted notarytool evidence. Accepted status must come from `notarytool=pass`, exact `status: Accepted` marker syntax or notarytool JSON accepted status; stapler success must be either `stapled=true` or an exact standalone `The staple and validate action worked!` line. Prose such as `note: paste status: accepted ...` or `note: paste The staple and validate action worked! ...` no longer satisfies either half through substring matching. `import-ci` now reports weak accepted+stapled logs as failed instead of copying them to `notary.log`. Focused notarization tests pass with 7 tests, release evidence template tests pass with 4 tests, import-ci tests pass with 21 tests and release-evidence tests pass with 33 tests; the final local rerun also passed formatting, full `vesty-cli` with 365 tests, full workspace Rust with 710 tests, workspace clippy, protocol export check and JS workspace tests. This is local parser/import hardening only; it does not add real Apple notarytool or stapler evidence.

Additional 2026-06-13 collect-notarization combined-log parser fix:

```bash
rtk cargo test -p vesty-cli collect_notarization -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli notarization_evidence -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk npm test
```

`collect-notarization` combined logs now preserve strict JSON accepted-status parsing while correctly handling the generated `notary_log=...`, `[notarytool]`, `stapler_log=...`, `[stapler]` layout. The bracketed notarytool section stops before the next section header or notarization collect metadata line, so `[notarytool]` JSON such as `{ "status": "Accepted" }` is not polluted by the following `stapler_log=...` line. A regression test now covers collected notarytool JSON plus exact stapler success, while prose containing inline JSON remains rejected. Focused collect-notarization tests pass with 2 tests, release-evidence tests pass with 33 tests, notarization-evidence tests pass with 7 tests, full `vesty-cli` passes with 365 tests, full workspace Rust passes with 710 tests, workspace clippy is clean, protocol export matches and JS workspace tests pass. This is local parser regression hardening only; it does not add real Apple notarytool or stapler evidence.

Additional 2026-06-13 VST3 SDK interface skeleton global vtable slot seed:

```bash
rtk cargo fmt --all --check
rtk cargo test -p vesty-vst3-sys generated_bindings_interface_skeleton -- --nocapture
rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk_emit_interface_skeleton -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk_interface_skeleton_validator -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk npm test
```

`generated-interface-skeleton.rs` now emits a `global_slot` for each `InterfaceVTableSlot` plus `INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE = "com-vtable-global-slot-seed-audit"`. The global slot locks the future COM vtable absolute slot order including the three inherited FUnknown slots: for example `IAudioProcessor::process` is local slot 6 / global slot 9, while `FUnknown::release` remains local/global slot 2. The generated helper check compiles and executes these lookups, and the `vesty-cli` skeleton validator now rejects older skeleton artifacts that only include local slot metadata. Focused generated-bindings tests pass with 13 tests, focused CLI VST3 SDK tests pass with 27 tests, release-evidence/import-ci still pass, full `vesty-cli` passes with 365 tests, full workspace Rust passes with 710 tests, workspace clippy is clean, protocol export matches and JS workspace tests pass. This is generated SDK interface skeleton audit/emitter-prep hardening only; it does not generate callable Steinberg COM method implementations or prove complete SDK 3.8 bindings.

Additional 2026-06-13 VST3 SDK interface skeleton vtable slot lookup seed:

```bash
rtk cargo test -p vesty-vst3-sys generated_bindings_interface_skeleton -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk_emit_interface_skeleton -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk_interface_skeleton_validator -- --nocapture
rtk cargo test -p vesty-cli import_ci -- --nocapture
rtk cargo test -p vesty-vst3-sys generated_bindings -- --nocapture
rtk cargo test -p vesty-cli vst3_sdk -- --nocapture
rtk cargo test -p vesty-cli release_evidence -- --nocapture
rtk cargo fmt --all --check
rtk cargo test -p vesty-cli -- --nocapture
rtk cargo test --workspace -j1
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo run -p vesty-cli -- export-types --out target/vesty-protocol --check
rtk npm test
```

`generated-interface-skeleton.rs` now emits `INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE = "pure-vtable-slot-lookup-seed-audit"` plus pure lookup helpers for `INTERFACE_VTABLE_SLOTS`: `interface_vtable_slot_by_interface_and_method()` and `interface_vtable_slot_by_interface_and_global_slot()`. The generated helper check compiles and runs these helpers, verifying `IAudioProcessor::process` can be found by method and by global slot 9, and that absent slots return `None`. The `vesty-cli` skeleton validator now requires the lookup scope marker and helper functions, so older skeleton artifacts with arrays but no lookup seed fail release/import validation. Focused generated-bindings tests pass with 13 tests, focused CLI VST3 SDK tests pass with 27 tests, release-evidence/import-ci still pass, full `vesty-cli` passes with 365 tests, full workspace Rust passes with 710 tests, workspace clippy is clean, protocol export matches and JS workspace tests pass. This is generated SDK interface skeleton audit/emitter-prep hardening only; it does not generate callable Steinberg COM method implementations or prove complete SDK 3.8 bindings.
