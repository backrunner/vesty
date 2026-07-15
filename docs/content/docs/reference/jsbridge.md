---
title: JSBridge protocol
description: Look up the editor handshake, commands, events, state, and error model.
order: 2
---

## Envelope

Every packet carries a protocol version, session, sequence number, lane, kind, and type. Requests additionally carry an ID; responses and errors use `replyTo`. Inbound values are bounded JSON and sequence numbers stay within JavaScript's safe integer range.

## Handshake

```text
WebView                         Native runtime
   |  bridge.hello                   |
   | ------------------------------> |
   |  BridgeReadyPayload             |
   | <------------------------------ |
   |  bridge.readyAck                |
   | ------------------------------> |
```

`BridgeReadyPayload` contains capabilities, parameter metadata, current parameter values, and a state snapshot. The editor adopts the returned `editorSessionId` only after validating the complete payload.

## Parameter commands

| Type | Purpose |
| --- | --- |
| `param.begin` | Begin a host gesture |
| `param.perform` | Request a normalized edit |
| `param.end` | End a host gesture |
| `param.format` | Format a normalized value |
| `param.parse` | Parse display text |
| `param.changed` | Confirm host/controller state |

Perform edits with a stable gesture ID. The runtime may coalesce pending perform requests while preserving all request IDs that require a response.

## State and subscriptions

Reliable topics require an explicit subscription. Snapshot/config/UI commands use revisions to reject stale writes. Meter and realtime log streams are bounded; latest meter values may replace older frames.

## Error codes

Important codes include `validation_error`, `unsupported_version`, `unsupported_type`, `timeout`, `backpressure`, `host_rejected`, `plugin_faulted`, `state_conflict`, and `internal_error`.

Errors are data, not thrown native panics. The JavaScript SDK turns an error packet into a rejected Promise with `code`, `message`, `retryable`, and optional bounded details.
