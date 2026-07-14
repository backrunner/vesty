---
title: JSBridge 协议
description: 查询编辑器握手、命令、事件和错误模型。
order: 2
---

## Envelope

每个 packet 都包含协议版本、session、sequence、lane、kind 和 type。Request 额外包含 ID；response/error 使用 `replyTo`。入站值必须是有界 JSON，sequence 不超过 JavaScript safe integer。

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

`BridgeReadyPayload` 包含 capability、参数元数据、当前参数值和 state snapshot。编辑器只有在完整验证 payload 后才采用返回的 `editorSessionId`。

## 参数命令

| Type | 用途 |
| --- | --- |
| `param.begin` | 开始宿主 gesture |
| `param.perform` | 请求 normalized edit |
| `param.end` | 结束宿主 gesture |
| `param.format` | 格式化 normalized value |
| `param.parse` | 解析显示文本 |
| `param.changed` | 确认宿主/controller 状态 |

同一次编辑应使用稳定 gesture ID。Runtime 可以合并待处理的 perform request，但必须保留所有需要 response 的 request ID。

## 状态与订阅

Reliable topic 需要显式订阅。Snapshot/config/UI 命令使用 revision 拒绝过期写入。Meter 与 realtime log stream 是有界的，最新 meter 可以替换旧帧。

## 错误码

常见错误包括 `validation_error`、`unsupported_version`、`timeout`、`backpressure`、`host_rejected`、`plugin_faulted`、`state_conflict` 和 `internal_error`。

错误是协议数据，不是 native panic。JavaScript SDK 会把 error packet 转换成带 `code`、`message`、`retryable` 和可选 details 的 rejected Promise。

