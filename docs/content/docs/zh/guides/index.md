---
title: 开发指南
description: 实现参数、DSP、状态和 WebView 编辑器。
order: 3
---

先完成端到端教程，再把其余页面作为专项参考：

1. [构建一个完整插件](/docs/zh/guides/complete-plugin)：从脚手架走到通过验证的立体声 VST3 bundle。
2. [参数](/docs/zh/guides/parameters)：声明稳定且宿主可见的控制项。
3. [DSP kernel](/docs/zh/guides/dsp)：在不破坏实时约束的情况下处理音频和事件。
4. [Web UI](/docs/zh/guides/web-ui)：通过类型化 bridge 连接编辑器。
5. [状态与生命周期](/docs/zh/guides/state-and-lifecycle)：恢复工程并正确响应宿主生命周期。

这些指南关注层与层之间的契约，不绑定某个 UI 框架或 DSP 算法。
