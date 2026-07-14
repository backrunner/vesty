---
title: Core concepts
description: Understand Vesty's ownership boundaries before adding features.
order: 2
---

Vesty is designed around one constraint: the host controls when audio runs, and the audio thread cannot wait for anything else.

## The four layers

| Layer | Responsibility |
| --- | --- |
| Plugin | Static metadata, parameter ownership, state hooks, UI descriptor |
| VST3 adapter | Host ABI, bus layout, events, automation, state and editor lifecycle |
| Audio kernel | Mutable DSP state and bounded per-block processing |
| Web editor | User interaction through a typed, asynchronous JSBridge |

Start with [Architecture](/docs/concepts/architecture), then read [Realtime safety](/docs/concepts/realtime-safety) before implementing DSP.

## Host authority

The host is authoritative for automated parameters and plugin state. A Web UI sends an edit request; the controller relays it to the host; only a successful host edit becomes confirmed state. Reloaded editors receive current values through `BridgeReadyPayload.paramValues`.

## Stable contracts

Parameter string IDs are developer-facing and stable. Vesty derives positive 31-bit VST3 IDs from them and exports a parameter manifest for packaging and release validation. The JSBridge protocol is generated from Rust types and checked for drift.

