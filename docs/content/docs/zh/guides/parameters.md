---
title: 参数
description: 声明类型化参数、稳定 ID、flag 和宿主自动化。
order: 1
---

## 声明参数集合

`#[derive(Params)]` 生成基于索引的 handle 访问，音频处理时无需搜索字符串。

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

## 只解析一次 handle

```rust
fn create_kernel(&self, _init: KernelInit) -> FilterKernel {
    FilterKernel {
        cutoff: self.params.resolve_or_invalid("cutoff"),
        resonance: self.params.resolve_or_invalid("resonance"),
    }
}
```

无效 handle 会安全返回 `None`，不会从宿主初始化流程 panic。

## Sample-accurate 自动化

需要精确 offset 时，消费 `ProcessContext` 中的参数事件。适配层会保留首个事件之前的上一 block 值，按 sample offset 排序，并在处理完成后提交最终值。

对于更新频率更低的系数，将事件处理与预分配 smoother 结合，避免每个 sample 重算昂贵状态。

## 生成 manifest

```bash
cargo run -p vesty-cli -- param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json

cargo run -p vesty-cli -- param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json \
  --check
```

Manifest 应纳入 Git。打包时会复制到 VST3 bundle，strict validation 会验证它仍与参数规格一致。

