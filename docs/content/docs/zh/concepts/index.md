---
title: 核心概念
description: 在扩展插件功能前，先理解 Vesty 的所有权与线程边界。
order: 2
---

Vesty 围绕一个约束设计：宿主决定音频何时运行，音频线程不能等待其他线程或外部资源。

## 四个层次

| 层次 | 职责 |
| --- | --- |
| 插件 | 静态元数据、参数所有权、状态钩子与 UI 描述 |
| VST3 适配层 | 宿主 ABI、总线、事件、自动化、状态与编辑器生命周期 |
| 音频内核 | 可变 DSP 状态与每个音频块内的有界处理 |
| Web 编辑器 | 通过强类型、异步 JSBridge 与原生层交互 |

先阅读[架构](/docs/zh/concepts/architecture)，然后在实现 DSP 前阅读[实时安全](/docs/zh/concepts/realtime-safety)。

## 宿主权威

宿主是自动化参数和插件状态的最终来源。Web UI 发起编辑请求，控制器将请求转发给宿主；只有宿主接受的编辑才会成为已确认状态。编辑器重新加载时，会通过 `BridgeReadyPayload.paramValues` 收到宿主当前保存的值。

## 稳定契约

面向开发者的参数字符串 ID 必须保持稳定。Vesty 会据此生成 31 位正整数 VST3 ID，并导出参数清单。JSBridge 的 TypeScript 协议由 Rust 类型生成，并通过检查防止两端定义发生漂移。
