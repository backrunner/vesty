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

```bash
vesty new my-plugin
vesty new my-plugin --template web-ui-react
vesty templates
```

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
