# Web UI and JSBridge reference

Read this reference for vanilla, React, Vue, Svelte, `vesty-plugin-ui`, adapter, protocol, or WebView work.

## Boundary

Vesty embeds the system WebView directly through `wry`; do not add Tauri. The Web editor communicates with the native controller over a typed, versioned JSBridge. JSON parsing, script evaluation, asset loading, and subscription dispatch stay outside the audio callback.

## Editor lifecycle

1. Create the bridge and wait for `ready()`.
2. Build controls from `ready.params` metadata.
3. Initialize values from `ready.paramValues`, never from defaults when restoring a project.
4. Subscribe to `param.changed` before accepting edits.
5. Wrap a user gesture in `beginParamEdit`, one or more `performParamEdit` calls, and `endParamEdit` with one gesture ID.
6. Render confirmed values from host events; do not retain an optimistic value after host rejection.
7. Unsubscribe and end active gestures when the component/editor is destroyed.

```ts
import { createBridge } from 'vesty-plugin-ui';

const bridge = createBridge(window, 'pending');
const ready = await bridge.ready();
const current = ready.paramValues.find((value) => value.id === 'gain');

const unsubscribe = bridge.subscribe('param.changed', (event) => {
  if (event.id === 'gain') renderGain(event.normalized);
});

const gestureId = crypto.randomUUID();
await bridge.beginParamEdit('gain', gestureId);
await bridge.performParamEdit('gain', 0.72, gestureId);
await bridge.endParamEdit('gain', gestureId);
```

## Protocol changes

When changing the bridge protocol:

- update Rust exports and `packages/plugin-ui` TypeScript sources together;
- update every React/Vue/Svelte adapter affected by the contract;
- regenerate exported types and run the drift check;
- test handshake, current values, gestures, host rejection, subscriptions, reload, and multiple editors;
- preserve version negotiation or make the compatibility break explicit.

Use:

```bash
vesty export-types --out target/vesty-protocol
vesty export-types --out target/vesty-protocol --check
```

## Packaging

Configure `[ui]` in `vesty.toml` with the UI directory, development URL, build command, output directory, and size constraints. Development may load `dev_url`; release packaging must build and copy static `dist` assets plus their size/hash manifest. Do not package `node_modules` or require a development server in a release bundle.

Meters and diagnostics must cross bounded realtime queues. Treat a full queue as a dropped observation, never as a reason for the audio thread to wait.
