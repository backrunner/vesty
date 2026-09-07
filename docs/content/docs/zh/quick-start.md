---
title: 快速开始
description: 从当前源码创建、构建、打包并验证你的第一个 Vesty 插件。
order: 0
---

Vesty 目前通过源码提供。框架 crates、`vesty-plugin-ui` npm 包和预编译 CLI **尚未发布**。请使用下方源码流程；发布版安装脚本和纯 registry 依赖需等首个版本发布后才能使用。

## 环境要求

- Rust 1.95+ 和平台原生编译工具（macOS 的 Xcode 命令行工具、Windows 的 MSVC Build Tools，或 Linux 的 C/C++ 工具链）。
- Git，以及后续测试用的 VST3 宿主。
- 仅 Web UI 项目需要 Node.js 24+ 和 npm。
- UI 使用系统 WebView：macOS 的 WKWebView、Windows 的 WebView2、Linux 的 WebKitGTK 4.1。Debian/Ubuntu 需要安装 `build-essential libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev pkg-config`。无界面模板不会启用 WebView 后端。

## 1. 从源码安装 CLI

在存放开发仓库的目录运行：

```bash
git clone https://github.com/backrunner/vesty.git vesty-source
cd vesty-source
export VESTY_SOURCE="$PWD"
cargo install --path crates/vesty-cli --locked
vesty --version
vesty doctor
cd ..
```

Windows PowerShell：

```powershell
git clone https://github.com/backrunner/vesty.git vesty-source
Set-Location vesty-source
$env:VESTY_SOURCE = (Get-Location).Path
cargo install --path crates/vesty-cli --locked
vesty --version
vesty doctor
Set-Location ..
```

Cargo 会把 CLI 安装到其 bin 目录（通常是 `~/.cargo/bin`），请确保它在 `PATH` 中。`doctor` 也会检查可选的 validator、UI 和签名工具，只需补齐当前流程需要的部分。

请保留源码仓库，生成项目会引用它的绝对路径。用 `git -C "$VESTY_SOURCE" rev-parse HEAD` 记录框架提交，并保留 Cargo/npm 锁文件以复现构建。升级时从同一份源码重新安装 CLI。打开新终端后，需要重新把 `VESTY_SOURCE` 设为该仓库的绝对路径。

## 2. 创建无界面效果器

下面使用 macOS/Linux 的 POSIX shell：

```bash
vesty templates
vesty new my-plugin --template gain --vesty-path "$VESTY_SOURCE/crates/vesty"
cd my-plugin
cargo test
vesty param-manifest --specs params.specs.json --out vesty-parameters.json --check
vesty build --config vesty.toml
```

PowerShell 将路径参数改为 `--vesty-path "$env:VESTY_SOURCE/crates/vesty"`，其它命令相同。

CLI 会创建独立 Cargo 项目、Rust 实现、`vesty.toml`、参数规格及稳定参数清单。gain 模板只启用 `vst3-bindings`，按采样位置处理增益自动化，不需要 Node.js 或 WebView。[完整插件教程](/docs/zh/guides/complete-plugin)继续介绍 DSP 测试和旁路。

## 3. 打包并验证

选择与你构建插件的平台对应的命令：

```bash
# macOS
vesty package --config vesty.toml --platform macos --binary target/release/libmy_plugin.dylib

# Linux
vesty package --config vesty.toml --platform linux --binary target/release/libmy_plugin.so

# Windows（PowerShell）
vesty package --config vesty.toml --platform windows --binary target/release/my_plugin.dll
```

然后执行：

```bash
vesty validate target/vesty/my-plugin.vst3 --static-only --strict
```

此步骤检查包结构、元数据、导出符号和参数清单，不会运行 DAW 或 Steinberg validator。宿主安装、validator 和平台发布检查见[打包](/docs/zh/tooling/packaging)与[发布证据](/docs/zh/tooling/release-evidence)。

## 4. 创建带 Web UI 的插件

先从同一份源码构建桥接包，再创建另一个插件。在 `my-plugin` 的父目录运行：

```bash
npm ci --prefix "$VESTY_SOURCE"
npm run build --prefix "$VESTY_SOURCE"
vesty new my-plugin-ui --template svelte-ui-param-demo \
  --vesty-path "$VESTY_SOURCE/crates/vesty" \
  --plugin-ui-path "$VESTY_SOURCE/packages/plugin-ui"
cd my-plugin-ui
npm install --prefix ui
npm run typecheck --prefix ui
npm run build --prefix ui
vesty build --config vesty.toml
```

PowerShell 使用 `$env:VESTY_SOURCE`，多行命令的续行符用反引号代替 `\`；也可以直接把每条命令写在一行。

本地覆盖参数会为 `vesty-plugin-ui` 生成 `file:` 依赖，因此需要先构建 SDK，提供编译后的导出文件。开发时，在一个终端运行 `npm run dev --prefix ui`，另一个终端运行 `vesty dev --config vesty.toml`。真实桥接需要在宿主中打开插件；普通浏览器预览不提供宿主参数状态。`vesty build` 会运行配置的 UI 构建命令；先完成构建，再执行 `vesty package`，后者只复制已有的 `dist` 资源。

| 模板 | 类型 | 编辑器 |
| --- | --- | --- |
| `gain` | 效果器 | 无 |
| `midi-synth` | 单声部乐器 | 无 |
| `web-ui-param-demo` | 效果器 | React |
| `vanilla-ui-param-demo` | 效果器 | TypeScript |
| `vue-ui-param-demo` | 效果器 | Vue |
| `svelte-ui-param-demo` | 效果器 | Svelte |
| `web-ui-instrument` | 单声部乐器 | React |

不指定 `--template` 时默认创建 React 效果器。使用未发布源码期间，必须传 `--vesty-path`；UI 模板还要传 `--plugin-ui-path`，否则生成项目会请求尚不存在的 registry 包。

## 下一步

- [完整插件教程](/docs/zh/guides/complete-plugin)：标识、DSP、测试与打包。
- [MIDI 乐器](/docs/zh/guides/midi)：音符身份、采样时间与密集事件分批。
- [Web UI](/docs/zh/guides/web-ui)：由宿主确认的参数编辑手势。
- [框架发布](/docs/zh/tooling/framework-release)：CLI、crates.io 和 npm 的后续分发方案。
