---
title: AI 辅助开发
description: 安装 Vesty Skill，让 AI 生成的修改遵守框架的实时与发布契约。
order: 4
---

Vesty 在 `skills/vesty-plugin-dev` 中提供了一个随仓库分发的 Codex Skill。它补充了通用 Rust 知识无法覆盖的项目约束，包括稳定的 VST3 身份、参数清单、音频内核实时规则、JSBridge 所有权、CLI 验证流程，以及必须来自真实环境的发布证据。

Skill 可以提供流程指导和自动化检查，但不能替代听音测试、代码审查、Steinberg validator 或真实 DAW 测试。

## 安装 Skill

将它安装到个人 Codex Skills 目录：

```bash
mkdir -p "${CODEX_HOME:-$HOME/.codex}/skills"
cp -R skills/vesty-plugin-dev \
  "${CODEX_HOME:-$HOME/.codex}/skills/vesty-plugin-dev"
```

如果你正在参与框架开发，并希望本地修改立即生效，可以改用符号链接：

```bash
ln -s "$PWD/skills/vesty-plugin-dev" \
  "${CODEX_HOME:-$HOME/.codex}/skills/vesty-plugin-dev"
```

替换前，请先删除旧的安装目录或符号链接。安装完成后重新启动 Codex 会话，使 Skill 目录重新加载。

团队也可以把 Skill 保留在仓库中，并在请求里告诉 AI 助手它的准确路径。当插件固定使用某个 Vesty 版本，不应受到全局新版 Skill 影响时，这种方式更合适。

## 显式调用

当任务涉及音频代码、宿主契约、打包或发布时，请在请求中明确写出 `$vesty-plugin-dev`：

```text
使用 $vesty-plugin-dev 创建一个立体声压缩器，包含阈值、压缩比、
启动时间、释放时间、补偿增益和旁路参数。先完成无界面版本和音频
内核测试，暂时不要添加 Web UI。
```

```text
使用 $vesty-plugin-dev 给现有插件添加 Svelte 编辑器。保留所有参数 ID，
使用宿主当前值初始化控件，并验证宿主拒绝编辑和编辑器重新加载的行为。
```

```text
使用 $vesty-plugin-dev 审查这个音频内核的实时安全性。先列出问题，
只有找到明确问题后才修改代码。
```

```text
使用 $vesty-plugin-dev 为 macOS 打包当前插件，并运行环境中所有可用的
本地验证。无法取得的 DAW、签名和公证证据必须明确标记为待完成。
```

清晰的请求应说明目标行为、效果器或乐器类型、是否需要 UI 及其框架、兼容性限制，以及任务应在哪一步停止。如果没有提供所需的宿主、平台、签名身份和外部 validator，就不要笼统地要求 AI 助手“完成发布准备”。

## Skill 会要求 AI 做什么

Skill 会要求 AI 助手：

1. 修改前读取最近的 `AGENTS.md`、插件元数据、参数规格和已有架构。
2. 创建工程时优先选择维护中的 `vesty new` 模板。
3. 保留 Class ID、Bundle ID、参数 ID、Bridge 字段和生成的参数清单。
4. 在 `process()` 外解析参数句柄，并让音频回调保持有界且不阻塞。
5. 在增加 UI 复杂度之前，先完成音频内核测试。
6. 通过 `ready.paramValues`、参数编辑手势与确认事件，让 WebView 始终以宿主状态为准。
7. 运行结果确定的检查，并单独报告所有跳过的检查。
8. 不使用伪造的标记文件代替 DAW、validator、签名、CI 或公证证据。

## 运行配套验证脚本

在包含 `Cargo.toml` 的插件工程中运行：

```bash
/path/to/vesty-plugin-dev/scripts/verify.sh .
```

脚本会运行 Rust 格式检查、测试和 Clippy。存在 `ui/package.json` 时，它还会运行 UI 检查；能够找到 Vesty CLI 时，也会检查参数清单。

如果插件使用同级目录中的 Vesty 源码，请显式指定 CLI 所在的工作区：

```bash
VESTY_MANIFEST=../vesty/Cargo.toml \
  /path/to/vesty-plugin-dev/scripts/verify.sh .
```

所有警告都应视为尚未完成的工作，尤其是“跳过参数清单检查”绝不能算作检查通过。

## 保留人工审查

接受 AI 生成的 DSP 前，要检查 `process()` 能够调用到的每个函数，确认其中没有内存分配、加锁、阻塞、`panic`、I/O 或无界工作。听辨参数变化处是否出现不连续或爆音，并比较离线渲染与实时渲染结果。最后检查代码差异，确认没有意外修改任何持久 ID。

对于发布候选版本，仍然需要人工运行并保存以下证据：

- 官方 validator 输出；
- DAW 扫描、加载、自动化、保存与恢复、UI 和离线渲染证据；
- 各平台的 WebView 冒烟测试；
- 签名与公证验证；
- 每个声明支持的目标所对应的 CI 构建制品。

Skill 能帮助 AI 助手遵守这些门槛并整理结果，但无法在当前环境不具备的宿主或平台上完成测试。

## 让 Skill 与 Vesty 一起演进

当 Vesty 修改 `Plugin`、`AudioKernel`、参数清单、JSBridge 消息、CLI 标志或发布门槛时，应在同一个拉取请求中更新 `skills/vesty-plugin-dev`。使用下面的命令验证 Skill 包结构：

```bash
python3 /path/to/skill-creator/scripts/quick_validate.py \
  skills/vesty-plugin-dev
```

`agents/openai.yaml` 中只保留简短的 UI 元数据；详细知识应放在 `SKILL.md` 和按需读取的 `references/` 文件中。
