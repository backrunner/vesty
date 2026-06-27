# 05. WebView UI 设计

## 目标

Vesty UI 层只承载 Web 前端，不绑定具体前端框架。React/Vue/Svelte/Preact/Solid/纯 HTML 都应能工作。框架提供:

- 系统 WebView 容器。
- JS bridge。
- 参数与事件 schema。
- dev/release assets 加载。
- 与 VST3 editor 生命周期绑定的 resize/focus/destroy。

## wry 后端

`vesty-ui-wry` 使用 wry，但不引入 Tauri:

- child webview: 使用 host editor parent 作为父窗口。
- custom protocol: release 模式用 `vesty://assets/...` 或平台等价 URL。
- IPC: JS 调 `window.ipc.postMessage`，Rust handler 解析 JSON。
- Rust 调 JS: UI thread 调 `WebView::evaluate_script`。

## UI 线程模型

约束:

- wry `WebView` 是 `!Send`/`!Sync`。
- WebView 必须在创建它的 UI 线程上操作。
- audio thread 不能直接调用任何 UI API。

设计:

```text
VST3 controller/editor thread
  owns EditorHandle
  creates UiRuntime
  creates WebView on UI thread
  receives IPC from JS
  forwards valid commands to ControllerBridge

Audio thread
  only writes meters to RT queue
  never sees WebView
```

如果 host 在非主线程调用 editor attach:

- macOS: 尝试 dispatch 到 main thread 创建 WKWebView，保留同步初始化结果。
- Windows: 在 attach 线程创建 child WebView2，失败则 fallback no-editor。
- Linux: 需要 GTK init 和 loop integration；第一版优先 X11，Wayland 标记 experimental。

## Assets 加载

### Dev 模式

`vesty.toml`:

```toml
[ui]
dir = "ui"
dev_url = "http://localhost:5173"
build = "npm run build"
dist = "dist"
```

`vesty dev`:

- 启动 UI dev server。
- 设置环境变量或生成 dev manifest。
- 插件 editor 优先加载 `dev_url`。
- DevTools 默认按 debug/release policy: debug 构建开启，release 构建关闭；可用 `VESTY_UI_DEVTOOLS=1|true|yes|on` 显式开启，或设为其它值显式关闭。

### Release 模式

流程:

1. CLI 运行 UI build。
2. 扫描 dist。
3. 生成 `assets.manifest.json`，包含 `version = 1`、build-time `root` provenance、entry、path、hash、mime、size。
4. 复制到 VST3 bundle Resources 或压缩嵌入 Rust binary。
5. WebView 通过 custom protocol 读取 manifest 内资源。

第一版建议把 assets 放入 `Contents/Resources/ui`，custom protocol 从 bundle resource 路径读取。这样 bundle 可检查、调试方便。后续可选 `embed-ui-assets` feature 把资源编译进 binary。

## JS Bridge

注入全局:

```ts
type ParamId = string;

interface VestyBridge {
  ready(): Promise<PluginSnapshot>;
  getSnapshot(): Promise<PluginSnapshot>;
  setParam(id: ParamId, normalized: number): Promise<void>;
  beginParamEdit(id: ParamId): Promise<void>;
  performParamEdit(id: ParamId, normalized: number, gestureId?: string): Promise<void>;
  endParamEdit(id: ParamId, gestureId?: string): Promise<void>;
  formatParam(id: ParamId, normalized: number): Promise<string>;
  parseParam(id: ParamId, text: string): Promise<number>;
  subscribe(topic: string, cb: (payload: unknown) => void): () => void;
  request<T = unknown>(type: string, payload?: unknown): Promise<T>;
}

declare global {
  interface Window {
    __VESTY__: VestyBridge;
  }
}
```

消息 envelope:

```json
{
  "v": 1,
  "session": "ui-session",
  "seq": 42,
  "lane": "param",
  "kind": "request",
  "id": "req-123",
  "type": "param.perform",
  "payload": {
    "id": "gain",
    "normalized": 0.75
  }
}
```

Rust response:

```json
{
  "v": 1,
  "session": "ui-session",
  "seq": 43,
  "lane": "param",
  "kind": "response",
  "replyTo": "req-123",
  "type": "param.perform",
  "payload": null
}
```

## 参数 UI 规则

- 拖拽开始时 `beginParamEdit`。
- 拖拽过程中节流调用 `performParamEdit`，默认最大 120 Hz。
- 拖拽结束时 `endParamEdit`。
- 单击按钮可以 begin/perform/end 合并在一个 UI transaction。
- UI 收到 host 参数变化时更新控件，但不得再次回写，避免 feedback loop。

## Meter/Analyzer

- audio thread 只产生 compact binary/struct frame。
- UI thread 采样 latest frame。
- JS 侧通过 requestAnimationFrame 绘制。
- 队列满则丢弃旧帧或新帧，具体由 meter 类型声明。
- 禁止把音频 sample buffer 直接发给 JS。

## 安全策略

Release WebView:

- 禁止任意外部导航。
- 禁止 `window.open` 默认行为。
- 禁止下载。
- 当前 `vesty-ui-wry` release asset mode 已通过 wry handler 只允许 `vesty://assets/...` bundle asset 导航；`about:blank` 只允许用于 WebView 生命周期导航，HTTP(S) shim origin、remote URL、localhost dev URL、其它 custom protocol host 均会被 release 导航拒绝，并继续拒绝下载和 `window.open`。
- release IPC handler 只接受来自 `vesty://assets/...` bundle asset URL 的 `window.ipc.postMessage`；`about:blank`、HTTP(S) shim origin、remote URL、localhost dev URL 和其它 custom protocol host 的 release IPC 会被丢弃。
- custom protocol 只服务 manifest 内路径；当前 `vesty-ui-wry` release asset protocol 必须加载 `assets.manifest.json`，并在 attach 时校验 manifest `version = 1`、build-time `root` 非空且无 control characters、entry 存在、files 非空、path URL-safe 且不重复、mime 非空且可作为 HTTP `Content-Type` header value、sha256 为 64 位 hex。manifest 顶层和 file entry 的未知字段会被拒绝；`root` 只作为 provenance，不要求等于运行时 bundle 路径，因为 `.vst3` bundle 可以被移动。URL-safe path 必须是相对路径，不能包含反斜杠、空段、`.`/`..` 段、ASCII control、`%`、`?`、`#` 或 `:`，避免 percent traversal、query/fragment 和 drive-like path 在不同 WebView/custom protocol 实现中出现歧义。加载成功后只允许 manifest path，并继续做 symlink 拒绝和 canonical root 检查防路径穿越，同时校验 manifest 里的 `size` 和 `sha256`，文件被篡改时返回 404。缺失或无效 manifest 会让 editor attach 返回 runtime unavailable，避免 release 模式裸服务整个 assets 目录。
- release HTML 响应带默认 CSP，禁止远程 script/connect，并附加 `X-Content-Type-Options: nosniff`。
- custom protocol response 使用不可失败的 response 组装路径；异常 asset、篡改 asset 或非法 manifest metadata 不会通过 `expect`/panic 穿过 WebView 请求回调。
- IPC schema 严格校验。
- DevTools release 默认关闭；只有显式 `VESTY_UI_DEVTOOLS=1|true|yes|on` 且当前 wry/platform 支持时才开启。

Dev WebView:

- 允许 dev server。
- 默认允许 DevTools；可用 `VESTY_UI_DEVTOOLS=0|false|off` 关闭。
- 显示明显 dev mode 标记只在 debug 构建中启用。

## Fallback

WebView 初始化失败时:

- 插件仍加载并处理音频。
- controller 仍暴露 host 参数。
- editor 显示最小 native fallback 或返回不可创建 editor。
- 日志报告具体原因，如 WebView2 runtime 缺失、WebKitGTK 缺失、parent handle 不支持。
