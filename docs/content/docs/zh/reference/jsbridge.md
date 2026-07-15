---
title: JSBridge 协议
description: 查询编辑器握手、参数命令、事件、状态和错误模型。
order: 2
---

## 消息结构

每个数据包都包含协议版本、会话、序列号、通道、消息类别和类型。请求还会携带 ID，响应和错误则使用 `replyTo` 指向对应请求。入站 JSON 必须符合大小限制，序列号也不能超出 JavaScript 的安全整数范围。

## 握手

```text
WebView                         Native runtime
   |  bridge.hello                   |
   | ------------------------------> |
   |  BridgeReadyPayload             |
   | <------------------------------ |
   |  bridge.readyAck                |
   | ------------------------------> |
```

`BridgeReadyPayload` 包含运行时能力、参数元数据、当前参数值和状态快照。编辑器只有在完整验证这份数据后，才能采用返回的 `editorSessionId`。

## 参数命令

| Type | 用途 |
| --- | --- |
| `param.begin` | 开始一次宿主编辑手势 |
| `param.perform` | 请求写入归一化值 |
| `param.end` | 结束宿主编辑手势 |
| `param.format` | 格式化归一化值 |
| `param.parse` | 解析显示文本 |
| `param.changed` | 确认宿主与控制器状态 |

同一次编辑应始终使用同一个手势 ID。运行时可以合并尚未处理的 `param.perform` 请求，但必须保留每一个需要响应的请求 ID。

## 状态与订阅

可靠主题需要显式订阅。快照、配置和 UI 命令使用修订号拒绝过期写入。电平数据流与实时日志流都有容量上限，最新电平帧可以替换旧帧。

## 错误码

常见错误包括 `validation_error`、`unsupported_version`、`timeout`、`backpressure`、`host_rejected`、`plugin_faulted`、`state_conflict` 和 `internal_error`。

错误属于协议数据，不会表现为原生端 `panic`。JavaScript SDK 会把错误数据包转换为被拒绝的 Promise，其中包含 `code`、`message`、`retryable` 和可选的受限详情。
