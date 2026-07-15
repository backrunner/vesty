---
title: 框架发布
description: 通过 CI 发布版本一致的 crates、npm 包、CLI 二进制与 GitHub Release。
order: 5
---

Vesty 使用 tag 驱动发布。一个 release tag 必须把框架 SDK 与 CLI 作为一组兼容产物发布；只有 CLI 二进制、没有对应 Rust 与 npm 依赖的发布是不完整的。

## 配置仓库环境

第一次发布前，在 GitHub 中创建两个受保护环境：

- `crates-io` 保存 `CRATES_IO_TOKEN` secret，该 token 只授予发布 Vesty crates 所需的权限。
- `npm` 在四个全新 `@vesty/*` 包首次发布时保存 `NPM_TOKEN` secret。完成 bootstrap release 后，在每个包的 trusted publisher 设置中绑定 `.github/workflows/release.yml` 的 `publish-npm` job，然后删除长期 token；没有 token 时，该 job 会自动使用 OIDC。

两个环境都应启用 reviewer approval。tag workflow 本身不会向 pull request 暴露 secret，而环境审批还可以防止维护者误推 tag 后立即发布。

## 准备版本

`[workspace.package].version`、所有 Vesty crate 和每个 `packages/*/package.json` 必须使用同一个 SemVer。内部 `@vesty/plugin-ui` peer dependency 与开发依赖也必须固定到这个准确版本。

例如，alpha 版本可以使用 `v0.1.0-alpha.1`，稳定版使用 `v0.1.0`。release tag 不要携带 build metadata。

创建 tag 前运行：

```bash
node scripts/release/verify-version.mjs v0.1.0-alpha.1
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm test
cargo run -p vesty-cli -- publish-plan --out target/publish-plan.json
cargo run -p vesty-cli -- crate-package --out target/crate-package.json
cargo run -p vesty-cli -- npm-pack --out target/npm-pack.json
```

最终稳定版仍必须满足[发布证据](/docs/zh/tooling/release-evidence)中列出的真实 DAW、平台 WebView、validator、签名和 notarization 要求。

## 通过 CI 发布

版本提交进入 `main` 后，创建并推送 annotated tag：

```bash
git tag -a v0.1.0-alpha.1 -m "Vesty v0.1.0-alpha.1"
git push origin v0.1.0-alpha.1
```

release workflow 会依次完成：

1. 验证所有 Rust 与 npm 包版本都与 tag 一致。
2. 生成并检查 publish-plan、crate-package 与 npm-pack 证据。
3. 构建 Linux x64、macOS universal 与 Windows x64 CLI 压缩包。
4. 按依赖顺序发布 13 个 crate，并等待每个版本进入 crates.io 索引。
5. 先发布 `@vesty/plugin-ui`，再发布 React、Vue 与 Svelte adapter。
6. 使用各平台发布二进制创建 Rust 工程，并额外构建一个 React 模板工程。
7. 生成 `SHA256SUMS`、构建来源证明和 GitHub Release。

registry 版本不可修改，也不能回滚。如果部分包发布后 workflow 失败，请修复问题并重新运行同一个 tag workflow。发布脚本会跳过 registry 中已有的版本，从第一个缺失包继续。只有全部冒烟测试通过后才会创建 GitHub Release。

预发布版本使用 SemVer channel 作为 npm dist-tag，例如 `alpha`、`beta` 或 `rc`；稳定版使用 `latest`。

## 验证发布结果

从完成的 workflow 或 GitHub Release 下载压缩包，使用 `SHA256SUMS` 校验后运行：

```bash
vesty --version
vesty templates
vesty new release-smoke --template gain
cargo check --manifest-path release-smoke/Cargo.toml
```

测试预发布版本安装脚本时，显式传入 tag：

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/orchiliao/vesty/main/scripts/install.sh \
  | VESTY_VERSION=v0.1.0-alpha.1 sh
```
