---
title: 核心概念
description: 在增加功能前理解 Vesty 的所有权边界。
order: 2
---

Vesty 围绕一个约束设计：宿主决定音频何时运行，音频线程不能等待其他线程或外部资源。

## 四个层次

| 层次 | 职责 |
| --- | --- |
| Plugin | 静态元数据、参数所有权、状态 hook、UI 描述 |
| VST3 适配层 | 宿主 ABI、bus、事件、自动化、状态与编辑器生命周期 |
| Audio kernel | 可变 DSP 状态和有界的逐 block 处理 |
| Web 编辑器 | 通过类型化、异步 JSBridge 完成交互 |

先阅读[架构](/docs/zh/concepts/architecture)，然后在实现 DSP 前阅读[实时安全](/docs/zh/concepts/realtime-safety)。

## 宿主权威

宿主是自动化参数和插件状态的权威来源。Web UI 发起编辑请求，controller 将请求转发给宿主，只有成功的宿主编辑才成为确认状态。编辑器重新加载时会通过 `BridgeReadyPayload.paramValues` 收到当前值。

## 稳定契约

参数字符串 ID 面向开发者且必须稳定。Vesty 从中生成正数 31-bit VST3 ID，并导出参数 manifest。JSBridge TypeScript 协议则由 Rust 类型生成并执行漂移检查。
