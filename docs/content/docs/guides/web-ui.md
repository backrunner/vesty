---
title: Web UI
description: Embed a system WebView and keep parameters host-authoritative.
order: 3
---

Vesty embeds `wry` directly. It does not add a Tauri runtime. The editor can be vanilla JavaScript, React, Vue, or Svelte and communicates through `vesty-plugin-ui`.

## Configure assets

```toml title="vesty.toml"
[ui]
dir = "ui"
dev_url = "http://localhost:5173"
build = "npm run build"
dist = "dist"
width = 900
height = 560
min_width = 640
min_height = 420
```

Development uses `dev_url`. Release packaging copies `dist` into the bundle and generates a size/hash asset manifest.

## Complete the handshake

```ts
import { createBridge } from 'vesty-plugin-ui';

const bridge = createBridge(window, 'pending');
const ready = await bridge.ready();

const gain = ready.paramValues.find((value) => value.id === 'gain');
console.log(gain?.normalized);
```

`ready.params` describes parameter metadata. `ready.paramValues` contains current host/controller values. Do not initialize controls from `defaultNormalized` when restoring an existing project.

## Perform a host edit

```ts
const gestureId = crypto.randomUUID();

await bridge.beginParamEdit('gain', gestureId);
await bridge.performParamEdit('gain', 0.72, gestureId);
await bridge.endParamEdit('gain', gestureId);
```

The controller updates its parameter only after the host accepts `performEdit`. A rejection becomes a `host_rejected` bridge error instead of optimistic local state.

## Subscribe to confirmation

```ts
const unsubscribe = bridge.subscribe('param.changed', (event) => {
  if (event.id === 'gain') updateGainControl(event.normalized);
});
```

Use confirmed events to synchronize multiple editors, host automation, preset changes, and program changes.

## Reload behavior

A new `bridge.hello` on a pending session resets subscriptions, gestures, meters, and the old editor session. The native endpoint refreshes `paramValues` before each handshake, so reloaded and reopened editors do not display stale controls.

## Keep work off the audio thread

JSON parsing, WebView evaluation, subscriptions, and state snapshots run outside `process()`. Realtime meters cross a bounded queue and are drained asynchronously by the UI thread.

