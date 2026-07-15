---
title: 参数
description: 声明强类型参数、稳定 ID、参数标记和宿主自动化。
order: 1
---

## 声明参数集合

`#[derive(Params)]` 会生成基于索引的参数句柄访问方式，因此音频处理期间不需要按字符串搜索参数。

```rust
#[derive(Params)]
struct FilterParams {
    cutoff: FloatParam,
    resonance: FloatParam,
    bypass: BoolParam,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            cutoff: FloatParam::new("cutoff", "Cutoff", 20.0, 20_000.0, 1_000.0)
                .with_unit("Hz"),
            resonance: FloatParam::new("resonance", "Resonance", 0.1, 12.0, 0.7),
            bypass: BoolParam::new("bypass", "Bypass", false).with_bypass(true),
        }
    }
}
```

字符串 ID 是持久的工程契约。发布后不要随意改名，除非同时提供迁移策略。

## 只解析一次参数句柄

```rust
fn create_kernel(&self, _init: KernelInit) -> FilterKernel {
    FilterKernel {
        cutoff: self.params.resolve_or_invalid("cutoff"),
        resonance: self.params.resolve_or_invalid("resonance"),
    }
}
```

无效句柄会安全地返回 `None`，不会让 `panic` 穿出宿主初始化流程。

## 采样级精确自动化

算法需要精确采样位置时，应读取 `ProcessContext` 中的参数事件。适配层会在第一个事件之前保留上一音频块的值，按采样偏移排列事件，并在整块处理完成后提交最终值。

对于不必逐采样更新的系数，可以把事件处理与预分配的平滑器结合起来，避免每个采样点都重新计算昂贵状态。

## 生成参数清单

```bash
cargo run -p vesty-cli -- param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json

cargo run -p vesty-cli -- param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json \
  --check
```

参数清单应纳入 Git。打包时，Vesty 会把它复制到 VST3 插件包；严格验证还会确认清单与参数规格保持一致。
