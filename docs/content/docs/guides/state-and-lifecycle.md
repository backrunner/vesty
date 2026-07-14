---
title: State and lifecycle
description: Restore projects and respond correctly to host processing changes.
order: 4
---

## State ownership

The VST3 component stores processor state, while the controller mirrors host-visible parameter and UI state. Keep serialized data versioned and deterministic. Do not serialize transient DSP buffers or WebView objects.

## Kernel lifecycle

| Hook | Use it for |
| --- | --- |
| `prepare` | Sample-rate setup and preallocation |
| `reset` | Clear algorithm history after processing configuration changes |
| `suspend` | Pause background algorithm state when processing stops |
| `resume` | Resume after the host restarts processing |

`setupProcessing()` and active-state changes trigger reset behavior. `setProcessing(false/true)` maps to suspend/resume.

## Web UI state

The bridge snapshot separates parameter, configuration, and UI revisions. Configuration and UI state updates use optimistic revision checks so stale editors receive a state conflict instead of overwriting newer data.

```ts
const snapshot = await bridge.getSnapshot();

await bridge.setUiState({
  expectedRevision: snapshot.uiRevision,
  value: { selectedTab: 'meters' }
});
```

Treat UI state as optional presentation state. Audio output must remain valid when no editor has ever opened.

## Test the full sequence

Cover save, destroy, restore, activate, prepare, process, suspend, resume, reopen editor, and reload editor. Real DAWs differ in ordering, so unit tests should accept valid host sequences without assuming one application.

