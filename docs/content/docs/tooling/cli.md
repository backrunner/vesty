---
title: CLI workflows
description: Use vesty commands for repeatable project operations.
order: 1
---

## Discover commands

```bash
vesty --help
vesty package --help
vesty release-check --help
```

## Create a project

First complete the [source installation](/docs/quick-start) and set `VESTY_SOURCE`. Registry packages are not published yet.

```bash
vesty templates
vesty new my-plugin --template gain --vesty-path "$VESTY_SOURCE/crates/vesty"
vesty new my-plugin-ui --template web-ui-param-demo \
  --vesty-path "$VESTY_SOURCE/crates/vesty" \
  --plugin-ui-path "$VESTY_SOURCE/packages/plugin-ui"
```

Build the local SDK before installing a generated UI: `npm ci --prefix "$VESTY_SOURCE"` and `npm run build --prefix "$VESTY_SOURCE"`. With no template option, `new` defaults to React; explicit `--kind` and `--ui` override template defaults.

Templates include native gain and instrument examples plus vanilla, React, Vue, and Svelte Web UI starters. Generated UI controls initialize from current `ready.paramValues` and subscribe to confirmed changes.

## Development and build

```bash
vesty dev --config vesty.toml
vesty build --config vesty.toml
```

Keep ordinary Rust and npm checks in the loop as well. The CLI does not replace compiler, linter, or unit-test output.

## Protocol and manifests

```bash
vesty export-types --out target/vesty-protocol
vesty export-types --out target/vesty-protocol --check

vesty param-manifest \
  --specs params.specs.json \
  --out vesty-parameters.json \
  --check
```

Use `--check` in CI to detect generated-file drift.

## Diagnostics

```bash
vesty doctor --format json
vesty smoke-host --out target/smoke-host.json
```

`smoke-host` checks repository configuration, sidecars, and optional bridge/meter traces. It is a headless self-check, not a DAW or validator run.
