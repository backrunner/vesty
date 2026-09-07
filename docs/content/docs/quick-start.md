---
title: Get started
description: Build, package, and validate your first Vesty plugin from the current source.
order: 0
---

Vesty is currently distributed as source. The framework crates, `vesty-plugin-ui` npm package, and prebuilt CLI releases have **not yet been published**. Use the checkout workflow below; the release installer and registry-only dependencies are for a future published release.

## Requirements

- Rust 1.95+ and your platform's native linker/toolchain (Xcode command line tools on macOS, MSVC Build Tools on Windows, or a C/C++ toolchain on Linux).
- Git, and a VST3 host for the eventual DAW test.
- Node.js 24+ and npm only for a Web UI.
- UI builds use the system WebView: WKWebView on macOS, WebView2 on Windows, and WebKitGTK 4.1 on Linux. On Debian/Ubuntu install `build-essential libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev pkg-config`. Headless templates do not enable the WebView backend.

## 1. Install the CLI from source

Run this in the directory where you keep development checkouts:

```bash
git clone https://github.com/backrunner/vesty.git vesty-source
cd vesty-source
export VESTY_SOURCE="$PWD"
cargo install --path crates/vesty-cli --locked
vesty --version
vesty doctor
cd ..
```

In Windows PowerShell:

```powershell
git clone https://github.com/backrunner/vesty.git vesty-source
Set-Location vesty-source
$env:VESTY_SOURCE = (Get-Location).Path
cargo install --path crates/vesty-cli --locked
vesty --version
vesty doctor
Set-Location ..
```

Cargo installs the executable into its bin directory (normally `~/.cargo/bin`); ensure that directory is on `PATH`. Some `doctor` checks concern optional validator, UI, or signing tools, so only install the tools required for your current workflow.

Keep the source checkout: generated projects depend on its absolute path. Record `git -C "$VESTY_SOURCE" rev-parse HEAD` alongside your project and retain Cargo/npm lockfiles to reproduce a build. Reinstall the CLI from the same checkout when upgrading it. In a new terminal, set `VESTY_SOURCE` again to that checkout's absolute path.

## 2. Create a headless effect

The following commands use a POSIX shell (macOS/Linux):

```bash
vesty templates
vesty new my-plugin --template gain --vesty-path "$VESTY_SOURCE/crates/vesty"
cd my-plugin
cargo test
vesty param-manifest --specs params.specs.json --out vesty-parameters.json --check
vesty build --config vesty.toml
```

For PowerShell, use `--vesty-path "$env:VESTY_SOURCE/crates/vesty"`; the other commands are the same.

The CLI creates a standalone Cargo project, Rust implementation, `vesty.toml`, parameter specifications, and a stable parameter manifest. The gain starter enables only `vst3-bindings`, processes gain automation at sample offsets, and needs no Node.js or WebView. The [complete plugin tutorial](/docs/guides/complete-plugin) adds DSP tests and bypass.

## 3. Package and validate

Choose the command matching the machine on which you built the plugin:

```bash
# macOS
vesty package --config vesty.toml --platform macos --binary target/release/libmy_plugin.dylib

# Linux
vesty package --config vesty.toml --platform linux --binary target/release/libmy_plugin.so

# Windows (PowerShell)
vesty package --config vesty.toml --platform windows --binary target/release/my_plugin.dll
```

Then:

```bash
vesty validate target/vesty/my-plugin.vst3 --static-only --strict
```

This checks the bundle, metadata, exports, and manifest. It does not run a DAW or Steinberg's validator. Follow [Packaging](/docs/tooling/packaging) and [Release evidence](/docs/tooling/release-evidence) for host installation, validator runs, and platform release checks.

## 4. Start with a Web UI

First build the local bridge package from the same checkout, then create a separate plugin. Run from the parent directory of `my-plugin`:

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

PowerShell uses `$env:VESTY_SOURCE` and a backtick instead of `\` for multiline commands; you can also put each command on one line.

The source override produces a local `file:` dependency for `vesty-plugin-ui`; building the SDK first supplies its compiled exports. Run `npm run dev --prefix ui` in one terminal and `vesty dev --config vesty.toml` in another. Open the plugin in a host to exercise the real bridge; a browser preview alone does not provide host parameter state. `vesty build` runs the configured UI build. Run it before `vesty package`, which copies the existing `dist` assets.

| Template | Kind | Editor |
| --- | --- | --- |
| `gain` | Effect | None |
| `midi-synth` | Monophonic instrument | None |
| `web-ui-param-demo` | Effect | React |
| `vanilla-ui-param-demo` | Effect | TypeScript |
| `vue-ui-param-demo` | Effect | Vue |
| `svelte-ui-param-demo` | Effect | Svelte |
| `web-ui-instrument` | Monophonic instrument | React |

Without `--template`, the CLI defaults to a React effect. While using unreleased source, always supply `--vesty-path`, plus `--plugin-ui-path` for a UI template. Otherwise the generated project requests unavailable registry packages.

## Continue

- [Complete plugin tutorial](/docs/guides/complete-plugin): identities, DSP, tests, and packaging.
- [MIDI instruments](/docs/guides/midi): note identity, sample timing, and dense event batches.
- [Web UI](/docs/guides/web-ui): host-authoritative parameter gestures.
- [Framework releases](/docs/tooling/framework-release): planned CLI, crates.io, and npm distribution.
