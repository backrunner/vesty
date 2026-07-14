---
title: AI 辅助开发
description: 安装 Vesty skill，并让 AI 修改始终遵守框架的实时契约。
order: 4
---

Vesty 在 `skills/vesty-plugin-dev` 中提供了一个随仓库分发的 Codex skill。它补充了通用 Rust 知识无法覆盖的项目约束：稳定的 VST3 身份、参数 manifest、realtime-safe kernel 规则、JSBridge 所有权、CLI 验证流程，以及不能被虚构的发布证据。

Skill 能提供流程和自动化，但不能代替听音测试、代码审查、Steinberg validator 和真实 DAW。

## 安装 skill

将它安装到个人 Codex skills 目录：

```bash
mkdir -p "${CODEX_HOME:-$HOME/.codex}/skills"
cp -R skills/vesty-plugin-dev \
  "${CODEX_HOME:-$HOME/.codex}/skills/vesty-plugin-dev"
```

如果你正在贡献框架，并希望本地修改立刻生效，可以使用 symbolic link：

```bash
ln -s "$PWD/skills/vesty-plugin-dev" \
  "${CODEX_HOME:-$HOME/.codex}/skills/vesty-plugin-dev"
```

替换前先删除旧的安装目录或 link。安装完成后重新启动 Codex session，让 skill catalog 重新加载。

团队也可以把 skill 保留在仓库内，并在请求中告诉 agent 它的准确路径。当插件固定在特定 Vesty revision，不希望受到全局新版本影响时，这种方式更合适。

## 显式调用

涉及 audio code、宿主契约、打包或发布时，在请求中写出 `$vesty-plugin-dev`：

```text
使用 $vesty-plugin-dev 创建一个立体声压缩器，包含 threshold、ratio、
attack、release、makeup gain 和 bypass。先完成 headless 版本和 kernel
测试，暂时不要添加 Web UI。
```

```text
使用 $vesty-plugin-dev 给现有插件添加 Svelte editor。保留所有参数 ID，
使用宿主当前值初始化控件，并验证宿主拒绝 gesture 和 editor reload。
```

```text
使用 $vesty-plugin-dev review 这个 kernel 的 realtime safety。先列出问题，
只有找到明确问题后才修改代码。
```

```text
使用 $vesty-plugin-dev 为 macOS 打包当前插件，并运行环境中所有可用的
本地验证。缺少的 DAW、签名和 notarization 证据必须保持 pending。
```

清晰的请求应说明目标行为、effect 或 instrument 类型、是否使用 UI 及其框架、兼容性限制和预期停止点。如果没有提供全部宿主、平台、签名身份与外部 validator，就不要笼统要求 agent “做到 release-ready”。

## Skill 会约束 agent 做什么

Skill 会要求 agent：

1. 修改前读取最近的 `AGENTS.md`、插件元数据、参数规格和已有架构。
2. 创建工程时优先选择维护中的 `vesty new` 模板。
3. 保留 class ID、bundle ID、参数 ID、bridge 字段和生成的 manifest。
4. 在 `process()` 外解析参数 handle，并保持 callback 有界、非阻塞。
5. 在增加 UI 复杂度前先完成 kernel 测试。
6. 通过 `ready.paramValues`、edit gesture 与确认事件，让 WebView 始终以宿主状态为准。
7. 运行确定性检查，并把 skipped check 单独报告。
8. 不用伪造 marker file 代替 DAW、validator、签名、CI 或 notarization 证据。

## 运行配套验证脚本

在包含 `Cargo.toml` 的插件工程中运行：

```bash
/path/to/vesty-plugin-dev/scripts/verify.sh .
```

脚本会运行 Rust format、test 和 clippy。存在 `ui/package.json` 时还会运行 UI 检查；能够找到 Vesty CLI 时也会检查参数 manifest。

如果插件使用相邻的框架 checkout，显式指定 CLI workspace：

```bash
VESTY_MANIFEST=../vesty/Cargo.toml \
  /path/to/vesty-plugin-dev/scripts/verify.sh .
```

所有 warning 都应该当作未完成工作处理，尤其是 “skipped parameter manifest check” 不能算作检查通过。

## 保留人工审查

接受 AI 生成的 DSP 前，要检查 `process()` 可达的每个调用是否分配、加锁、阻塞、panic、执行 I/O 或包含无界工作。听取参数变化处是否有 discontinuity，并比较 offline 与 realtime rendering。检查 diff 中是否意外修改了任何持久 ID。

对于 release candidate，仍然需要人工运行并保存：

- 官方 validator 输出；
- DAW scan、load、automation、save/restore、UI 和 offline-render 证据；
- 平台 WebView smoke test；
- 签名与 notarization 验证；
- 每个声明 target 对应的 CI artifact。

Skill 能帮助 agent 遵守 gate 并整理结果，但无法在当前环境不存在的宿主或平台上完成测试。

## 让 skill 与 Vesty 一起演进

当 Vesty 修改 `Plugin`、`AudioKernel`、参数 manifest、JSBridge message、CLI flag 或发布 gate 时，应在同一个 pull request 中更新 `skills/vesty-plugin-dev`。使用下面的命令验证包结构：

```bash
python3 /path/to/skill-creator/scripts/quick_validate.py \
  skills/vesty-plugin-dev
```

`agents/openai.yaml` 中只保留简短的 UI metadata；详细知识应放在 `SKILL.md` 与按需读取的 `references/` 中。
