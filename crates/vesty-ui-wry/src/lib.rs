#![deny(clippy::undocumented_unsafe_blocks)]

use thiserror::Error;
use vesty_ipc::BridgePacket;
#[cfg(feature = "wry-backend")]
use vesty_ipc::{
    BridgeErrorCode, BridgeErrorPayload, BridgeKind, advance_bridge_packet_seq,
    validate_bridge_packet_flags, validate_bridge_packet_id, validate_bridge_packet_seq,
    validate_bridge_session, validate_packet_type,
};
#[cfg(feature = "wry-backend")]
use vesty_ui::{EditorRuntime, EditorSize, UiDescriptor, UiError};

#[cfg(all(feature = "wry-backend", target_os = "macos"))]
use raw_window_handle::AppKitWindowHandle;
#[cfg(all(feature = "wry-backend", target_os = "windows"))]
use raw_window_handle::Win32WindowHandle;
#[cfg(all(
    feature = "wry-backend",
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
use raw_window_handle::XlibWindowHandle;
#[cfg(feature = "wry-backend")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle, WindowHandle};
#[cfg(feature = "wry-backend")]
use serde::Deserialize;
#[cfg(feature = "wry-backend")]
use sha2::{Digest, Sha256};
#[cfg(feature = "wry-backend")]
use std::collections::BTreeMap;
#[cfg(all(feature = "wry-backend", target_os = "windows"))]
use std::num::NonZeroIsize;
#[cfg(feature = "wry-backend")]
use std::{borrow::Cow, cell::Cell, ffi::c_void, path::PathBuf, ptr::NonNull, rc::Rc};

#[derive(Debug, Error)]
pub enum WryBridgeError {
    #[error("failed to serialize bridge packet: {0}")]
    Serialize(#[from] serde_json::Error),
    #[cfg(feature = "wry-backend")]
    #[error("wry error: {0}")]
    Wry(#[from] wry::Error),
}

pub const MAX_BRIDGE_BATCH_PACKETS: usize = 4096;

pub fn bootstrap_script() -> &'static str {
    r#"
(() => {
  if (window.__VESTY_INTERNAL__) return;
  const REQUEST_TIMEOUT_MS = 5000;
  const EVENT_FLUSH_TIMEOUT_MS = 1000;
  const MAX_COMMAND_MESSAGE_BYTES = 65536;
  const MAX_STATE_MESSAGE_BYTES = 262144;
  const BRIDGE_PROTOCOL_VERSION = 1;
  const BOOTSTRAP_VERSION = "0.1.0";
  const MAX_BRIDGE_SESSION_BYTES = 128;
  const MAX_BRIDGE_PACKET_TYPE_BYTES = 128;
  const MAX_BRIDGE_PACKET_ID_BYTES = 128;
  const MAX_BRIDGE_PACKET_SEQ = Number.MAX_SAFE_INTEGER;
  const MAX_BRIDGE_PACKET_FLAGS = 16;
  const MAX_BRIDGE_PACKET_FLAG_BYTES = 64;
  const MAX_BRIDGE_ERROR_MESSAGE_BYTES = 2048;
  const MAX_BRIDGE_BATCH_PACKETS = 4096;
  const MAX_BRIDGE_JSON_DEPTH = 64;
  const MAX_BRIDGE_JSON_ARRAY_ITEMS = 65536;
  const MAX_BRIDGE_JSON_OBJECT_KEYS = 16384;
  const MAX_BRIDGE_JSON_NODES = 262144;
  const MAX_BRIDGE_JSON_STRING_BYTES = 262144;
  const MAX_SUBSCRIPTION_TOPIC_BYTES = 128;
  const MAX_PARAM_GESTURE_ID_BYTES = 128;
  const MAX_CONFIG_KEY_BYTES = 128;
  const BRIDGE_INBOUND_LANES = new Set(["lifecycle", "command", "param", "state", "event", "meter", "log"]);
  const BRIDGE_INBOUND_KINDS = new Set(["response", "event", "ack", "error"]);
  const BRIDGE_REQUEST_LANES = new Set(["lifecycle", "command", "param", "state", "event", "meter", "log"]);
  const BRIDGE_ERROR_CODES = new Set(["parse_error", "unsupported_version", "unsupported_type", "validation_error", "permission_denied", "timeout", "backpressure", "host_rejected", "plugin_faulted", "state_conflict", "internal_error"]);
  const pending = new Map();
  const listeners = new Map();
  let eventPump = 0;
  let eventFlushInFlight = false;
  let readyPromise = 0;
  let readyPayloadCache = 0;
  let seq = 1;
  let session = "pending";

  function requestTimeout(type) {
    return {
      code: "timeout",
      message: `Vesty request timed out: ${type}`,
      retryable: true
    };
  }

  function unloadError() {
    return {
      code: "internal_error",
      message: "Vesty WebView unloaded before the request completed",
      retryable: true
    };
  }

  function unsupportedProtocolError(protocolVersion) {
    return {
      code: "unsupported_version",
      message: `Unsupported Vesty bridge protocol version: ${String(protocolVersion)}`,
      retryable: false
    };
  }

  function validationError(message) {
    return {
      code: "validation_error",
      message,
      retryable: false
    };
  }

  function backpressureError(message) {
    return {
      code: "backpressure",
      message,
      retryable: true
    };
  }

  function maxMessageBytesForLane(lane) {
    return lane === "state" ? MAX_STATE_MESSAGE_BYTES : MAX_COMMAND_MESSAGE_BYTES;
  }

  function containsControl(value) {
    for (let index = 0; index < value.length; index += 1) {
      const code = value.charCodeAt(index);
      if (code <= 0x1f || code === 0x7f || (code >= 0x80 && code <= 0x9f)) return true;
    }
    return false;
  }

  function utf8ByteLength(value) {
    if (typeof TextEncoder !== "undefined") {
      return new TextEncoder().encode(value).length;
    }
    let bytes = 0;
    for (const char of value) {
      const codePoint = char.codePointAt(0) || 0;
      if (codePoint <= 0x7f) bytes += 1;
      else if (codePoint <= 0x7ff) bytes += 2;
      else if (codePoint <= 0xffff) bytes += 3;
      else bytes += 4;
    }
    return bytes;
  }

  function assertSubscriptionTopic(topic) {
    if (typeof topic !== "string") {
      throw validationError("subscription topic must be a string");
    }
    if (topic.length === 0) {
      throw validationError("subscription topic must not be empty");
    }
    if (utf8ByteLength(topic) > MAX_SUBSCRIPTION_TOPIC_BYTES) {
      throw validationError("subscription topic too long");
    }
    if (containsControl(topic)) {
      throw validationError("subscription topic must not contain control characters");
    }
  }

  function assertRequestType(type) {
    if (typeof type !== "string") {
      throw validationError("request type must be a string");
    }
    if (type.length === 0) {
      throw validationError("request type must not be empty");
    }
    if (utf8ByteLength(type) > MAX_BRIDGE_PACKET_TYPE_BYTES) {
      throw validationError("request type too long");
    }
    if (containsControl(type)) {
      throw validationError("request type must not contain control characters");
    }
  }

  function assertRequestLane(lane) {
    if (typeof lane !== "string") {
      throw validationError("request lane must be a string");
    }
    if (!BRIDGE_REQUEST_LANES.has(lane)) {
      throw validationError("request lane must be a known bridge lane");
    }
  }

  function validPacketString(value, maxBytes) {
    return typeof value === "string"
      && value.length > 0
      && utf8ByteLength(value) <= maxBytes
      && !containsControl(value);
  }

  function validPlainDataRecord(value) {
    const proto = Object.getPrototypeOf(value);
    if (proto !== Object.prototype && proto !== null) return false;
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== "string") return false;
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      if (!descriptor || !descriptor.enumerable || !("value" in descriptor)) return false;
    }
    return true;
  }

  function hasOwnDataProperty(value, key) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    return !!descriptor && descriptor.enumerable && "value" in descriptor;
  }

  function ownDataValue(value, key) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    return descriptor && "value" in descriptor ? descriptor.value : undefined;
  }

  function validJsonValue(value, depth, stack, budget) {
    depth = depth || 0;
    stack = stack || new Set();
    budget = budget || { nodes: 0 };
    budget.nodes += 1;
    if (budget.nodes > MAX_BRIDGE_JSON_NODES) return false;
    if (depth > MAX_BRIDGE_JSON_DEPTH) return false;
    if (value === null) return true;
    switch (typeof value) {
      case "string":
        return utf8ByteLength(value) <= MAX_BRIDGE_JSON_STRING_BYTES;
      case "number":
        return Number.isFinite(value);
      case "boolean":
        return true;
      case "object":
        break;
      default:
        return false;
    }
    if (stack.has(value)) return false;
    stack.add(value);
    if (Array.isArray(value)) {
      if (value.length > MAX_BRIDGE_JSON_ARRAY_ITEMS) {
        stack.delete(value);
        return false;
      }
      for (const item of value) {
        if (!validJsonValue(item, depth + 1, stack, budget)) {
          stack.delete(value);
          return false;
        }
      }
      stack.delete(value);
      return true;
    }
    const proto = Object.getPrototypeOf(value);
    if (proto !== Object.prototype && proto !== null) {
      stack.delete(value);
      return false;
    }
    const keys = Reflect.ownKeys(value);
    if (keys.length > MAX_BRIDGE_JSON_OBJECT_KEYS) {
      stack.delete(value);
      return false;
    }
    for (const key of keys) {
      if (typeof key !== "string") {
        stack.delete(value);
        return false;
      }
      if (utf8ByteLength(key) > MAX_BRIDGE_JSON_STRING_BYTES) {
        stack.delete(value);
        return false;
      }
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      if (!descriptor || !descriptor.enumerable || !("value" in descriptor)) {
        stack.delete(value);
        return false;
      }
      const item = descriptor.value;
      if (!validJsonValue(item, depth + 1, stack, budget)) {
        stack.delete(value);
        return false;
      }
    }
    stack.delete(value);
    return true;
  }

  function validBridgeError(value) {
    if (!isRecord(value)) return false;
    if (!validPlainDataRecord(value)) return false;
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== "string" || (key !== "code" && key !== "message" && key !== "details" && key !== "retryable")) return false;
    }
    const code = ownDataValue(value, "code");
    const message = ownDataValue(value, "message");
    const retryable = ownDataValue(value, "retryable");
    const hasDetails = hasOwnDataProperty(value, "details");
    const details = ownDataValue(value, "details");
    return validPacketString(code, MAX_BRIDGE_PACKET_TYPE_BYTES)
      && BRIDGE_ERROR_CODES.has(code)
      && typeof message === "string"
      && utf8ByteLength(message) <= MAX_BRIDGE_ERROR_MESSAGE_BYTES
      && !containsControl(message)
      && typeof retryable === "boolean"
      && (!hasDetails || validJsonValue(details));
  }

  function validPacketFlags(value) {
    if (value === undefined) return true;
    if (!Array.isArray(value) || value.length > MAX_BRIDGE_PACKET_FLAGS) return false;
    return value.every((flag) => validPacketString(flag, MAX_BRIDGE_PACKET_FLAG_BYTES));
  }

  function validInboundPacket(packet) {
    if (!isRecord(packet)) return false;
    if (!validPlainDataRecord(packet)) return false;
    const version = ownDataValue(packet, "v");
    const session = ownDataValue(packet, "session");
    const seq = ownDataValue(packet, "seq");
    const lane = ownDataValue(packet, "lane");
    const kind = ownDataValue(packet, "kind");
    const packetType = ownDataValue(packet, "type");
    const flags = ownDataValue(packet, "flags");
    const payload = ownDataValue(packet, "payload");
    const replyTo = ownDataValue(packet, "replyTo");
    const error = ownDataValue(packet, "error");
    if (version !== BRIDGE_PROTOCOL_VERSION) return false;
    if (!validPacketString(session, MAX_BRIDGE_SESSION_BYTES)) return false;
    if (typeof seq !== "number" || !Number.isSafeInteger(seq) || seq < 0 || seq > MAX_BRIDGE_PACKET_SEQ) return false;
    if (typeof lane !== "string" || !BRIDGE_INBOUND_LANES.has(lane)) return false;
    if (typeof kind !== "string" || !BRIDGE_INBOUND_KINDS.has(kind)) return false;
    if (!validPacketString(packetType, MAX_BRIDGE_PACKET_TYPE_BYTES)) return false;
    if (!validPacketFlags(flags)) return false;
    if (hasOwnDataProperty(packet, "id")) return false;
    if (hasOwnDataProperty(packet, "payload") && !validJsonValue(payload)) return false;
    if (kind === "response" || kind === "error") {
      if (!validPacketString(replyTo, MAX_BRIDGE_PACKET_ID_BYTES)) return false;
      if (kind === "response" && hasOwnDataProperty(packet, "error")) return false;
      if (kind === "error" && !validBridgeError(error)) return false;
      if (kind === "error" && hasOwnDataProperty(packet, "payload")) return false;
    } else if (hasOwnDataProperty(packet, "replyTo") || hasOwnDataProperty(packet, "error")) {
      return false;
    }
    return true;
  }

  function assertSubscriptionHandler(handler) {
    if (typeof handler !== "function") {
      throw validationError("subscription handler must be a function");
    }
  }

  function assertBridgeSessionValue(session, name, makeError) {
    if (typeof session !== "string") {
      throw makeError(`${name} must be a string`);
    }
    if (session.length === 0) {
      throw makeError(`${name} must not be empty`);
    }
    if (utf8ByteLength(session) > MAX_BRIDGE_SESSION_BYTES) {
      throw makeError(`${name} too long`);
    }
    if (containsControl(session)) {
      throw makeError(`${name} must not contain control characters`);
    }
  }

  function assertReadyEditorSessionId(session) {
    assertBridgeSessionValue(session, "editorSessionId", readyPayloadError);
  }

  function assertConfigKey(key) {
    if (typeof key !== "string") {
      throw validationError("config key must be a string");
    }
    if (key.length === 0) {
      throw validationError("config key must not be empty");
    }
    if (utf8ByteLength(key) > MAX_CONFIG_KEY_BYTES) {
      throw validationError("config key too long");
    }
    if (containsControl(key)) {
      throw validationError("config key must not contain control characters");
    }
  }

  function assertBaseRevision(value) {
    if (typeof value !== "number" || !Number.isFinite(value) || !Number.isInteger(value) || value < 0) {
      throw validationError("baseRevision must be a non-negative integer");
    }
  }

  function assertJsonValuePresent(value, name) {
    if (value === undefined) {
      throw validationError(`${name} must not be undefined`);
    }
  }

  function assertParamId(id) {
    if (typeof id !== "string") {
      throw validationError("parameter id must be a string");
    }
    if (id.length === 0) {
      throw validationError("parameter id must not be empty");
    }
    if (containsControl(id)) {
      throw validationError("parameter id must not contain control characters");
    }
  }

  function assertNormalizedValue(normalized) {
    if (typeof normalized !== "number" || !Number.isFinite(normalized)) {
      throw validationError("normalized value must be a finite number");
    }
  }

  function assertParamText(text) {
    if (typeof text !== "string") {
      throw validationError("parameter text must be a string");
    }
  }

  function assertOptionalGestureId(gestureId) {
    if (gestureId === undefined) return;
    if (typeof gestureId !== "string") {
      throw validationError("gestureId must be a string");
    }
    if (gestureId.length === 0) {
      throw validationError("gestureId must not be empty");
    }
    if (utf8ByteLength(gestureId) > MAX_PARAM_GESTURE_ID_BYTES) {
      throw validationError("gestureId too long");
    }
    if (containsControl(gestureId)) {
      throw validationError("gestureId must not contain control characters");
    }
  }

  function paramGesturePayload(id, gestureId) {
    const payload = { id };
    if (gestureId !== undefined) payload.gestureId = gestureId;
    return payload;
  }

  function paramPerformPayload(id, normalized, gestureId) {
    const payload = { id, normalized };
    if (gestureId !== undefined) payload.gestureId = gestureId;
    return payload;
  }

  function clearPending(id, item) {
    if (item.timer) clearTimeout(item.timer);
    pending.delete(id);
  }

  function stopEventPump() {
    if (!eventPump) return;
    clearInterval(eventPump);
    eventPump = 0;
  }

  function rejectAllPending(error) {
    for (const [id, item] of pending) {
      clearPending(id, item);
      item.reject(error);
    }
  }

  function disposeForUnload() {
    stopEventPump();
    listeners.clear();
    readyPromise = 0;
    readyPayloadCache = 0;
    rejectAllPending(unloadError());
  }

  function adoptEditorSession(payload) {
    session = payload.editorSessionId;
  }

  function helloPayload() {
    return {
      supportedProtocolVersions: [BRIDGE_PROTOCOL_VERSION],
      jsPackageVersion: BOOTSTRAP_VERSION,
      pageUrl: window.location && typeof window.location.href === "string" ? window.location.href : ""
    };
  }

  function readyAckPayload(payload) {
    const protocolVersion = payload && typeof payload === "object" && typeof payload.protocolVersion === "number"
      ? payload.protocolVersion
      : BRIDGE_PROTOCOL_VERSION;
    return { protocolVersion };
  }

  function readyPayloadError(message) {
    return validationError(`Invalid Vesty bridge ready payload: ${message}`);
  }

  function isRecord(value) {
    return !!value && typeof value === "object" && !Array.isArray(value);
  }

  function assertRecord(value, name) {
    if (!isRecord(value)) throw readyPayloadError(`${name} must be an object`);
    return value;
  }

  function assertNonEmptyString(value, name) {
    if (typeof value !== "string" || value.trim().length === 0) {
      throw readyPayloadError(`${name} must be a non-empty string`);
    }
    if (containsControl(value)) throw readyPayloadError(`${name} must not contain control characters`);
    return value;
  }

  function assertFiniteNumber(value, name) {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      throw readyPayloadError(`${name} must be a finite number`);
    }
    return value;
  }

  function assertRevision(value, name) {
    const revision = assertFiniteNumber(value, name);
    if (!Number.isInteger(revision) || revision < 0) {
      throw readyPayloadError(`${name} must be a non-negative integer`);
    }
  }

  function assertCapabilities(value) {
    const capabilities = assertRecord(value, "capabilities");
    for (const key of [
      "paramGestures",
      "paramFormatParse",
      "stateConfig",
      "subscriptions",
      "meterStream",
      "reliableEvents",
      "diagnostics"
    ]) {
      if (typeof capabilities[key] !== "boolean") {
        throw readyPayloadError(`capabilities.${key} must be boolean`);
      }
    }
  }

  function assertPluginSnapshot(value) {
    const snapshot = assertRecord(value, "snapshot");
    assertRevision(snapshot.revision, "snapshot.revision");
    assertRevision(snapshot.paramsRevision, "snapshot.paramsRevision");
    assertRevision(snapshot.configRevision, "snapshot.configRevision");
    assertRevision(snapshot.uiRevision, "snapshot.uiRevision");
    if (!("config" in snapshot)) throw readyPayloadError("snapshot.config is required");
    if (!("uiState" in snapshot)) throw readyPayloadError("snapshot.uiState is required");
  }

  function assertOptionalString(value, name) {
    if (value === null) return;
    if (typeof value !== "string") throw readyPayloadError(`${name} must be a string or null`);
    if (containsControl(value)) throw readyPayloadError(`${name} must not contain control characters`);
  }

  function assertParamMidiMappings(value, name) {
    if (!Array.isArray(value)) throw readyPayloadError(`${name}.midiMappings must be an array`);
    const seen = new Set();
    value.forEach((mapping, mappingIndex) => {
      const mappingName = `${name}.midiMappings[${mappingIndex}]`;
      const record = assertRecord(mapping, mappingName);
      const controller = assertFiniteNumber(record.controller, `${mappingName}.controller`);
      if (!Number.isInteger(controller) || controller < 0 || controller > 140) {
        throw readyPayloadError(`${mappingName}.controller must be an integer within 0..=140`);
      }
      if (
        record.channel !== null &&
        (typeof record.channel !== "number" ||
          !Number.isInteger(record.channel) ||
          record.channel < 0 ||
          record.channel > 15)
      ) {
        throw readyPayloadError(`${mappingName}.channel must be an integer within 0..=15 or null`);
      }
      const key = `${controller}:${record.channel ?? "*"}`;
      if (seen.has(key)) throw readyPayloadError(`${name} has duplicate MIDI mapping ${key}`);
      seen.add(key);
    });
  }

  function assertParamSpec(value, index, ids) {
    const name = `params[${index}]`;
    const param = assertRecord(value, name);
    const id = assertNonEmptyString(param.id, `${name}.id`);
    if (ids.has(id)) throw readyPayloadError(`duplicate parameter id '${id}'`);
    ids.add(id);
    assertNonEmptyString(param.name, `${name}.name`);

    const defaultNormalized = assertFiniteNumber(param.defaultNormalized, `${name}.defaultNormalized`);
    if (defaultNormalized < 0 || defaultNormalized > 1) {
      throw readyPayloadError(`${name}.defaultNormalized must be within 0.0..=1.0`);
    }
    assertOptionalString(param.unit, `${name}.unit`);

    if (
      param.stepCount !== null &&
      (!Number.isInteger(param.stepCount) || typeof param.stepCount !== "number" || param.stepCount < 0)
    ) {
      throw readyPayloadError(`${name}.stepCount must be a non-negative integer or null`);
    }

    const flags = assertRecord(param.flags, `${name}.flags`);
    if (
      typeof flags.automatable !== "boolean" ||
      typeof flags.bypass !== "boolean" ||
      typeof flags.readOnly !== "boolean"
    ) {
      throw readyPayloadError(`${name}.flags must contain boolean automatable/bypass/readOnly fields`);
    }
    if (flags.programChange !== undefined && typeof flags.programChange !== "boolean") {
      throw readyPayloadError(`${name}.flags.programChange must be a boolean when present`);
    }
    if (flags.readOnly && flags.automatable) {
      throw readyPayloadError(`${name} is read-only and must not be automatable`);
    }
    assertParamMidiMappings(param.midiMappings, name);

    if (param.kind === "bool") return;
    const kind = assertRecord(param.kind, `${name}.kind`);
    const kindKeys = Object.keys(kind);
    if (kindKeys.length !== 1) throw readyPayloadError(`${name}.kind must have exactly one tag`);
    if ("float" in kind) {
      const float = assertRecord(kind.float, `${name}.kind.float`);
      const min = assertFiniteNumber(float.min, `${name}.kind.float.min`);
      const max = assertFiniteNumber(float.max, `${name}.kind.float.max`);
      if (min >= max) throw readyPayloadError(`${name}.kind.float requires min < max`);
      return;
    }
    if ("choice" in kind) {
      const choice = assertRecord(kind.choice, `${name}.kind.choice`);
      if (!Array.isArray(choice.values)) {
        throw readyPayloadError(`${name}.kind.choice.values must be an array`);
      }
      choice.values.forEach((label, valueIndex) => {
        assertNonEmptyString(label, `${name}.kind.choice.values[${valueIndex}]`);
      });
      return;
    }
    throw readyPayloadError(`${name}.kind has an unsupported tag`);
  }

  function assertReadyParams(value) {
    if (!Array.isArray(value)) throw readyPayloadError("params must be an array");
    const ids = new Set();
    value.forEach((param, index) => assertParamSpec(param, index, ids));
  }

  function assertCompatibleReadyPayload(payload) {
    const ready = assertRecord(payload, "ready payload");
    const protocolVersion = typeof ready.protocolVersion === "number"
      ? ready.protocolVersion
      : undefined;
    if (protocolVersion !== BRIDGE_PROTOCOL_VERSION) {
      throw unsupportedProtocolError(protocolVersion);
    }
    assertNonEmptyString(ready.instanceId, "instanceId");
    assertReadyEditorSessionId(ready.editorSessionId);
    if (typeof ready.devMode !== "boolean") throw readyPayloadError("devMode must be boolean");
    assertNonEmptyString(ready.pluginName, "pluginName");
    assertNonEmptyString(ready.vendor, "vendor");
    assertCapabilities(ready.capabilities);
    assertReadyParams(ready.params);
    assertPluginSnapshot(ready.snapshot);
  }

  function packetMatchesSession(packet) {
    return packet && ownDataValue(packet, "session") === session;
  }

  function reportListenerError(type, error) {
    try {
      console.error("Vesty bridge listener error", type, error);
    } catch (_) {
      // Ignore console failures; listener isolation must not throw from bridge delivery.
    }
  }

  function emit(type, payload) {
    const set = listeners.get(type);
    if (!set) return;
    for (const listener of Array.from(set)) {
      try {
        listener(payload);
      } catch (error) {
        reportListenerError(type, error);
      }
    }
  }

  function isAsyncEventTopic(topic) {
    return topic.startsWith("meter.")
      || topic === "param.changed"
      || topic === "diagnostics.fault"
      || topic === "log.rt";
  }

  function hasAsyncEventSubscribers() {
    for (const [topic, set] of listeners) {
      if (isAsyncEventTopic(topic) && set.size > 0) return true;
    }
    return false;
  }

  function refreshEventPump() {
    if (hasAsyncEventSubscribers()) {
      if (!eventPump) {
        eventPump = setInterval(() => {
          if (eventFlushInFlight) return;
          eventFlushInFlight = true;
          window.__VESTY_INTERNAL__.request("event.flush", "event", {}, { timeoutMs: EVENT_FLUSH_TIMEOUT_MS })
            .catch(() => {})
            .finally(() => { eventFlushInFlight = false; });
        }, 16);
      }
      return;
    }
    if (eventPump) {
      stopEventPump();
    }
  }

  window.addEventListener("pagehide", disposeForUnload, { once: true });
  window.addEventListener("beforeunload", disposeForUnload, { once: true });

  window.__VESTY_INTERNAL__ = {
    setSession(value) {
      assertBridgeSessionValue(value, "session", validationError);
      session = value;
    },
    deliver(packet) {
      if (!validInboundPacket(packet)) return;
      if (!packetMatchesSession(packet)) return;
      const kind = ownDataValue(packet, "kind");
      const replyTo = ownDataValue(packet, "replyTo");
      const error = ownDataValue(packet, "error");
      const payload = ownDataValue(packet, "payload");
      const packetType = ownDataValue(packet, "type");
      if (kind === "response" || kind === "error") {
        const item = pending.get(replyTo);
        if (!item) return;
        clearPending(replyTo, item);
        if (kind === "error") item.reject(error);
        else item.resolve(payload);
        return;
      }
      if (kind !== "event") return;
      emit(packetType, payload);
    },
    deliverBatch(packets) {
      if (!Array.isArray(packets)) return;
      if (packets.length > MAX_BRIDGE_BATCH_PACKETS) return;
      for (const packet of packets) this.deliver(packet);
    },
    request(type, lane, payload, options) {
      assertRequestType(type);
      assertRequestLane(lane);
      if (payload !== undefined && !validJsonValue(payload)) {
        return Promise.reject(validationError("request payload must be JSON-compatible"));
      }
      if (!window.ipc || !window.ipc.postMessage) {
        return Promise.reject(new Error("Vesty IPC is unavailable"));
      }
      const requestSeq = seq;
      const id = `js-${requestSeq}`;
      const packet = { v: 1, session, seq: requestSeq, lane, kind: "request", type, id };
      if (payload !== undefined) packet.payload = payload;
      const message = JSON.stringify(packet);
      if (utf8ByteLength(message) > maxMessageBytesForLane(lane)) {
        return Promise.reject(backpressureError("bridge message too large"));
      }
      seq = seq >= MAX_BRIDGE_PACKET_SEQ ? 1 : seq + 1;
      return new Promise((resolve, reject) => {
        const item = { resolve, reject, timer: 0 };
        const timeoutMs = Math.max(0, options
          && typeof options.timeoutMs === "number"
          && Number.isFinite(options.timeoutMs)
          ? options.timeoutMs
          : REQUEST_TIMEOUT_MS);
        if (timeoutMs > 0) {
          item.timer = setTimeout(() => {
            const timedOut = pending.get(id);
            if (!timedOut) return;
            pending.delete(id);
            timedOut.reject(requestTimeout(type));
          }, timeoutMs);
        }
        pending.set(id, item);
        try {
          window.ipc.postMessage(message);
        } catch (error) {
          clearPending(id, item);
          reject(error);
        }
      });
    },
    subscribe(topic, handler) {
      assertSubscriptionTopic(topic);
      assertSubscriptionHandler(handler);
      let set = listeners.get(topic);
      const shouldSubscribe = !set || set.size === 0;
      if (!set) listeners.set(topic, set = new Set());
      set.add(handler);
      if (shouldSubscribe) {
        window.__VESTY_INTERNAL__.request("subscription.add", "command", { topic }).catch(() => {});
      }
      refreshEventPump();
      return () => {
        const current = listeners.get(topic);
        if (!current) return;
        current.delete(handler);
        if (current.size === 0) {
          listeners.delete(topic);
          window.__VESTY_INTERNAL__.request("subscription.remove", "command", { topic }).catch(() => {});
        }
        refreshEventPump();
      };
    }
  };

  window.__VESTY__ = {
    ready() {
      if (readyPayloadCache) return Promise.resolve(readyPayloadCache);
      if (!readyPromise) {
        readyPromise = window.__VESTY_INTERNAL__.request("bridge.hello", "command", helloPayload()).then((payload) => {
          assertCompatibleReadyPayload(payload);
          adoptEditorSession(payload);
          return window.__VESTY_INTERNAL__.request("bridge.readyAck", "command", readyAckPayload(payload)).then(() => {
            readyPayloadCache = payload;
            return payload;
          });
        }).catch((error) => {
          readyPromise = 0;
          throw error;
        });
      }
      return readyPromise;
    },
    getSnapshot() { return window.__VESTY_INTERNAL__.request("snapshot.get", "state", {}); },
    getDiagnostics() { return window.__VESTY_INTERNAL__.request("diagnostics.get", "command", {}); },
    request(type, payload) { return window.__VESTY_INTERNAL__.request(type, "command", payload); },
    subscribe(topic, handler) { return window.__VESTY_INTERNAL__.subscribe(topic, handler); },
    async setConfig(key, value, baseRevision) {
      assertConfigKey(key);
      assertJsonValuePresent(value, "config value");
      assertBaseRevision(baseRevision);
      return window.__VESTY_INTERNAL__.request("state.setConfig", "state", { baseRevision, key, value });
    },
    async setUiState(value, baseRevision) {
      assertJsonValuePresent(value, "ui state value");
      assertBaseRevision(baseRevision);
      return window.__VESTY_INTERNAL__.request("state.setUiState", "state", { baseRevision, value });
    },
    async beginParamEdit(id, gestureId) {
      assertParamId(id);
      assertOptionalGestureId(gestureId);
      return window.__VESTY_INTERNAL__.request("param.begin", "param", paramGesturePayload(id, gestureId));
    },
    async performParamEdit(id, normalized, gestureId) {
      assertParamId(id);
      assertNormalizedValue(normalized);
      assertOptionalGestureId(gestureId);
      return window.__VESTY_INTERNAL__.request("param.perform", "param", paramPerformPayload(id, normalized, gestureId));
    },
    async endParamEdit(id, gestureId) {
      assertParamId(id);
      assertOptionalGestureId(gestureId);
      return window.__VESTY_INTERNAL__.request("param.end", "param", paramGesturePayload(id, gestureId));
    },
    async setParam(id, normalized, gestureId) {
      assertParamId(id);
      assertNormalizedValue(normalized);
      assertOptionalGestureId(gestureId);
      await this.beginParamEdit(id, gestureId);
      let failure;
      try {
        await this.performParamEdit(id, normalized, gestureId);
      } catch (error) {
        failure = error;
      }
      try {
        await this.endParamEdit(id, gestureId);
      } catch (error) {
        if (failure === undefined) failure = error;
      }
      if (failure !== undefined) throw failure;
    },
    async formatParam(id, normalized) {
      assertParamId(id);
      assertNormalizedValue(normalized);
      return window.__VESTY_INTERNAL__.request("param.format", "param", { id, normalized });
    },
    async parseParam(id, text) {
      assertParamId(id);
      assertParamText(text);
      return window.__VESTY_INTERNAL__.request("param.parse", "param", { id, text });
    }
  };
})();
"#
}

pub fn packet_script(packet: &BridgePacket) -> Result<String, WryBridgeError> {
    let packet = serde_json::to_string(packet)?;
    Ok(format!(
        "window.__VESTY_INTERNAL__ && window.__VESTY_INTERNAL__.deliver({packet});"
    ))
}

pub fn batch_script(packets: &[BridgePacket]) -> Result<String, WryBridgeError> {
    let packets = serde_json::to_string(packets)?;
    Ok(format!(
        "window.__VESTY_INTERNAL__ && window.__VESTY_INTERNAL__.deliverBatch({packets});"
    ))
}

pub fn batch_scripts(packets: &[BridgePacket]) -> Result<Vec<String>, WryBridgeError> {
    packets
        .chunks(MAX_BRIDGE_BATCH_PACKETS)
        .map(batch_script)
        .collect()
}

#[cfg(feature = "wry-backend")]
#[derive(Clone, Copy, Debug)]
pub enum NativeParent {
    #[cfg(target_os = "macos")]
    MacOsNsView(NonNull<c_void>),
    #[cfg(target_os = "windows")]
    WindowsHwnd(NonZeroIsize),
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    XlibWindow(std::ffi::c_ulong),
}

#[cfg(feature = "wry-backend")]
impl NativeParent {
    #[cfg(target_os = "macos")]
    /// # Safety
    ///
    /// `ns_view` must be a valid `NSView` owned by the host editor on the UI thread.
    pub unsafe fn macos_ns_view(ns_view: *mut c_void) -> Result<Self, UiError> {
        NonNull::new(ns_view)
            .map(Self::MacOsNsView)
            .ok_or(UiError::UnsupportedParent)
    }

    #[cfg(target_os = "windows")]
    /// # Safety
    ///
    /// `hwnd` must be a valid HWND owned by the current UI thread.
    pub unsafe fn windows_hwnd(hwnd: isize) -> Result<Self, UiError> {
        NonZeroIsize::new(hwnd)
            .map(Self::WindowsHwnd)
            .ok_or(UiError::UnsupportedParent)
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    /// # Safety
    ///
    /// `window` must be a valid Xlib `Window`; Wayland is intentionally not represented here.
    pub unsafe fn xlib_window(window: std::ffi::c_ulong) -> Result<Self, UiError> {
        if window == 0 {
            Err(UiError::UnsupportedParent)
        } else {
            Ok(Self::XlibWindow(window))
        }
    }
}

#[cfg(feature = "wry-backend")]
impl HasWindowHandle for NativeParent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
        let handle = match *self {
            #[cfg(target_os = "macos")]
            Self::MacOsNsView(ns_view) => RawWindowHandle::AppKit(AppKitWindowHandle::new(ns_view)),
            #[cfg(target_os = "windows")]
            Self::WindowsHwnd(hwnd) => RawWindowHandle::Win32(Win32WindowHandle::new(hwnd)),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            Self::XlibWindow(window) => RawWindowHandle::Xlib(XlibWindowHandle::new(window)),
        };
        // SAFETY: NativeParent constructors validate non-null/non-zero handles and the caller
        // guarantees the host-owned parent remains valid for the WebView attach lifetime.
        Ok(unsafe { WindowHandle::borrow_raw(handle) })
    }
}

#[cfg(feature = "wry-backend")]
type IpcHandler = Rc<dyn Fn(String) -> Vec<BridgePacket>>;

#[cfg(feature = "wry-backend")]
#[derive(Default)]
pub struct WryEditorRuntime {
    webview: Option<Box<wry::WebView>>,
    ipc_handler: Option<IpcHandler>,
}

#[cfg(feature = "wry-backend")]
impl WryEditorRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ipc_handler(handler: impl Fn(String) + 'static) -> Self {
        Self {
            webview: None,
            ipc_handler: Some(Rc::new(move |text| {
                handler(text);
                Vec::new()
            })),
        }
    }

    pub fn with_bridge_handler(handler: impl Fn(String) -> Vec<BridgePacket> + 'static) -> Self {
        Self {
            webview: None,
            ipc_handler: Some(Rc::new(handler)),
        }
    }

    pub fn set_ipc_handler(&mut self, handler: impl Fn(String) + 'static) {
        self.ipc_handler = Some(Rc::new(move |text| {
            handler(text);
            Vec::new()
        }));
    }

    pub fn set_bridge_handler(&mut self, handler: impl Fn(String) -> Vec<BridgePacket> + 'static) {
        self.ipc_handler = Some(Rc::new(handler));
    }

    pub fn evaluate_packet(&self, packet: &BridgePacket) -> Result<(), WryBridgeError> {
        if let Some(webview) = &self.webview {
            evaluate_packet(webview, packet)?;
        }
        Ok(())
    }
}

#[cfg(feature = "wry-backend")]
impl EditorRuntime for WryEditorRuntime {
    type Parent = NativeParent;

    fn attach(&mut self, parent: Self::Parent, descriptor: &UiDescriptor) -> Result<(), UiError> {
        let bounds = bounds_for_size(descriptor.width, descriptor.height);
        let mut builder = wry::WebViewBuilder::new()
            .with_initialization_script(bootstrap_script())
            .with_bounds(bounds)
            .with_devtools(use_devtools())
            .with_general_autofill_enabled(false);
        let use_dev_url = use_dev_url() && descriptor.dev_url.is_some();

        if !use_dev_url {
            builder = builder
                .with_navigation_handler(release_navigation_allowed)
                .with_download_started_handler(|_, _| false)
                .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny);
        }

        let webview_ptr = Rc::new(Cell::new(std::ptr::null::<wry::WebView>()));
        if let Some(handler) = self.ipc_handler.clone() {
            let webview_ptr = webview_ptr.clone();
            let release_ipc_only = !use_dev_url;
            builder = builder.with_ipc_handler(move |request| {
                if release_ipc_only {
                    let uri = request.uri().to_string();
                    if !release_ipc_allowed(&uri) {
                        return;
                    }
                }
                let packets = call_ipc_handler_guarded(&handler, request.body().clone());
                if packets.is_empty() {
                    return;
                }
                let Ok(scripts) = batch_scripts(&packets) else {
                    return;
                };
                let ptr = webview_ptr.get();
                if !ptr.is_null() {
                    // SAFETY: `ptr` is set immediately after WebView construction and cleared by
                    // dropping the runtime on the same UI thread. The IPC closure only uses it
                    // synchronously to evaluate a response batch while the WebView is alive.
                    unsafe {
                        for script in scripts {
                            let _ = (&*ptr).evaluate_script(&script);
                        }
                    }
                }
            });
        }

        if use_dev_url {
            let Some(dev_url) = descriptor.dev_url.as_ref() else {
                return Err(UiError::RuntimeUnavailable(
                    "VESTY_UI_DEV requested a dev URL, but UiDescriptor.dev_url is empty"
                        .to_string(),
                ));
            };
            builder = builder.with_url(dev_url);
        } else {
            let assets_dir = PathBuf::from(&descriptor.assets_dir);
            let asset_manifest = load_asset_manifest(&assets_dir).map_err(|error| {
                UiError::RuntimeUnavailable(format!(
                    "failed to load UI asset manifest for {}: {error}",
                    assets_dir.display()
                ))
            })?;
            let protocol_root = assets_dir.clone();
            builder = builder
                .with_custom_protocol("vesty".to_string(), move |_id, request| {
                    asset_response(&protocol_root, &asset_manifest, request.uri().path())
                })
                .with_url("vesty://assets/index.html");
        }

        let webview = Box::new(
            builder
                .build_as_child(&parent)
                .map_err(|error| UiError::RuntimeUnavailable(error.to_string()))?,
        );
        webview_ptr.set(webview.as_ref() as *const wry::WebView);
        self.webview = Some(webview);
        Ok(())
    }

    fn resize(&mut self, size: EditorSize) -> Result<(), UiError> {
        if let Some(webview) = &self.webview {
            webview
                .set_bounds(bounds_for_size(size.width, size.height))
                .map_err(|error| UiError::RuntimeUnavailable(error.to_string()))?;
        }
        Ok(())
    }

    fn detach(&mut self) {
        self.webview = None;
    }
}

#[cfg(feature = "wry-backend")]
fn call_ipc_handler_guarded(handler: &IpcHandler, body: String) -> Vec<BridgePacket> {
    let panic_response = ipc_handler_panic_response(&body);
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(body))) {
        Ok(packets) => packets,
        Err(_) => panic_response,
    }
}

#[cfg(feature = "wry-backend")]
fn ipc_handler_panic_response(body: &str) -> Vec<BridgePacket> {
    let Ok(packet) = vesty_ipc::parse_packet(body) else {
        return Vec::new();
    };
    if packet.kind != BridgeKind::Request
        || validate_bridge_session(&packet.session).is_err()
        || validate_packet_type(&packet.packet_type).is_err()
        || validate_bridge_packet_seq(packet.seq).is_err()
        || validate_bridge_packet_flags(&packet.flags).is_err()
        || packet
            .id
            .as_deref()
            .is_none_or(|id| validate_bridge_packet_id(id).is_err())
        || packet.reply_to.is_some()
        || packet.error.is_some()
    {
        return Vec::new();
    }

    vec![packet.error_to(
        advance_bridge_packet_seq(packet.seq),
        BridgeErrorPayload::new(
            BridgeErrorCode::InternalError,
            "native IPC handler panicked",
            true,
        ),
    )]
}

#[cfg(feature = "wry-backend")]
fn use_dev_url() -> bool {
    ui_env_flag_or_default("VESTY_UI_DEV", cfg!(debug_assertions))
}

#[cfg(feature = "wry-backend")]
fn use_devtools() -> bool {
    ui_env_flag_or_default("VESTY_UI_DEVTOOLS", cfg!(debug_assertions))
}

#[cfg(feature = "wry-backend")]
fn ui_env_flag_or_default(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| ui_env_flag_truthy(&value))
        .unwrap_or(default)
}

#[cfg(feature = "wry-backend")]
fn ui_env_flag_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(feature = "wry-backend")]
fn bounds_for_size(width: u32, height: u32) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(0, 0).into(),
        size: wry::dpi::LogicalSize::new(width, height).into(),
    }
}

#[cfg(feature = "wry-backend")]
fn asset_response(
    root: &std::path::Path,
    manifest: &BTreeMap<String, RuntimeAssetEntry>,
    request_path: &str,
) -> wry::http::Response<Cow<'static, [u8]>> {
    let mime = asset_request_key(request_path)
        .ok()
        .and_then(|key| manifest.get(&key))
        .map(|entry| entry.mime.as_str())
        .unwrap_or_else(|| mime_for_path(request_path));
    match read_asset(root, request_path, manifest) {
        Ok(bytes) => {
            asset_http_response(200, mime, Cow::Owned(bytes), mime.starts_with("text/html"))
        }
        Err(_) => asset_not_found_response(),
    }
}

#[cfg(feature = "wry-backend")]
fn asset_not_found_response() -> wry::http::Response<Cow<'static, [u8]>> {
    asset_http_response(
        404,
        "text/plain; charset=utf-8",
        Cow::Borrowed(&b"not found"[..]),
        false,
    )
}

#[cfg(feature = "wry-backend")]
fn asset_http_response(
    status: u16,
    content_type: &str,
    body: Cow<'static, [u8]>,
    include_csp: bool,
) -> wry::http::Response<Cow<'static, [u8]>> {
    let mut response = wry::http::Response::new(body);
    *response.status_mut() = wry::http::StatusCode::from_u16(status)
        .unwrap_or(wry::http::StatusCode::INTERNAL_SERVER_ERROR);
    let content_type = wry::http::HeaderValue::from_str(content_type)
        .unwrap_or_else(|_| wry::http::HeaderValue::from_static("application/octet-stream"));
    response.headers_mut().insert(
        wry::http::HeaderName::from_static("content-type"),
        content_type,
    );
    response.headers_mut().insert(
        wry::http::HeaderName::from_static("x-content-type-options"),
        wry::http::HeaderValue::from_static("nosniff"),
    );
    if include_csp {
        response.headers_mut().insert(
            wry::http::HeaderName::from_static("content-security-policy"),
            wry::http::HeaderValue::from_static(release_asset_csp()),
        );
    }
    response
}

#[cfg(feature = "wry-backend")]
fn release_asset_csp() -> &'static str {
    "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; media-src 'self' data: blob:; connect-src 'self'; worker-src 'self' blob:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'"
}

#[cfg(feature = "wry-backend")]
fn release_navigation_allowed(url: String) -> bool {
    url == "about:blank" || release_asset_url_allowed(&url)
}

#[cfg(feature = "wry-backend")]
fn release_ipc_allowed(url: &str) -> bool {
    release_asset_url_allowed(url)
}

#[cfg(feature = "wry-backend")]
fn release_asset_url_allowed(url: &str) -> bool {
    if url.is_empty()
        || url.trim() != url
        || url
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return false;
    }

    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    let (authority, path) = rest
        .split_once('/')
        .map_or((rest, ""), |(authority, path)| (authority, path));
    if authority.is_empty() || authority.contains('@') || authority.contains(':') {
        return false;
    }
    let allowed_origin =
        scheme.eq_ignore_ascii_case("vesty") && authority.eq_ignore_ascii_case("assets");
    if !allowed_origin {
        return false;
    }
    if path.is_empty() {
        return true;
    }
    if path == "/" {
        return true;
    }
    is_safe_runtime_manifest_path(path)
}

#[cfg(feature = "wry-backend")]
fn safe_asset_path(
    root: &std::path::Path,
    request_path: &str,
    manifest: &BTreeMap<String, RuntimeAssetEntry>,
) -> std::io::Result<PathBuf> {
    let key = asset_request_key(request_path)?;
    if !manifest.contains_key(&key) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset path is not listed in manifest",
        ));
    }
    let root = root.canonicalize()?;
    let relative = key.split('/').collect::<PathBuf>();
    let unresolved = root.join(relative);
    let metadata = std::fs::symlink_metadata(&unresolved)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset path is not a regular file",
        ));
    }
    let candidate = unresolved.canonicalize()?;
    if candidate.starts_with(&root) {
        Ok(candidate)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset path escapes root",
        ))
    }
}

#[cfg(feature = "wry-backend")]
fn read_asset(
    root: &std::path::Path,
    request_path: &str,
    manifest: &BTreeMap<String, RuntimeAssetEntry>,
) -> std::io::Result<Vec<u8>> {
    let key = asset_request_key(request_path)?;
    let path = safe_asset_path(root, request_path, manifest)?;
    let bytes = std::fs::read(path)?;
    if let Some(entry) = manifest.get(&key) {
        verify_manifest_entry(entry, &bytes)?;
    }
    Ok(bytes)
}

#[cfg(feature = "wry-backend")]
fn verify_manifest_entry(entry: &RuntimeAssetEntry, bytes: &[u8]) -> std::io::Result<()> {
    if bytes.len() as u64 != entry.size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset size does not match manifest",
        ));
    }
    if entry.sha256.len() != 64 || !entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "asset sha256 is not a valid hex digest",
        ));
    }

    let digest = Sha256::digest(bytes);
    let actual = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(&entry.sha256) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "asset sha256 does not match manifest",
        ));
    }
    Ok(())
}

#[cfg(feature = "wry-backend")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeAssetManifest {
    version: u32,
    root: String,
    entry: String,
    files: Vec<RuntimeAssetFile>,
}

#[cfg(feature = "wry-backend")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeAssetFile {
    path: String,
    mime: String,
    sha256: String,
    size: u64,
}

#[cfg(feature = "wry-backend")]
#[derive(Debug)]
struct RuntimeAssetEntry {
    mime: String,
    sha256: String,
    size: u64,
}

#[cfg(feature = "wry-backend")]
fn load_asset_manifest(
    root: &std::path::Path,
) -> std::io::Result<BTreeMap<String, RuntimeAssetEntry>> {
    let root_metadata = std::fs::symlink_metadata(root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "UI asset root is not a regular directory",
        ));
    }
    let candidates = [
        root.parent()
            .map(|parent| parent.join("assets.manifest.json")),
        Some(root.join("assets.manifest.json")),
    ];
    let mut manifest_path = None;
    for path in candidates.into_iter().flatten() {
        if let Some(path) = manifest_candidate_file(path)? {
            manifest_path = Some(path);
            break;
        }
    }
    let manifest_path = manifest_path.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "assets.manifest.json was not found next to or inside the UI assets directory",
        )
    })?;
    let text = std::fs::read_to_string(&manifest_path)?;
    let manifest = serde_json::from_str::<RuntimeAssetManifest>(&text).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "invalid asset manifest {}: {error}",
                manifest_path.display()
            ),
        )
    })?;
    runtime_asset_entries(manifest)
}

#[cfg(feature = "wry-backend")]
fn manifest_candidate_file(path: PathBuf) -> std::io::Result<Option<PathBuf>> {
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "assets.manifest.json must not be a symlink",
        ));
    }
    Ok(metadata.is_file().then_some(path))
}

#[cfg(feature = "wry-backend")]
fn runtime_asset_entries(
    manifest: RuntimeAssetManifest,
) -> std::io::Result<BTreeMap<String, RuntimeAssetEntry>> {
    if manifest.version != 1 {
        return Err(invalid_manifest(format!(
            "unsupported asset manifest version {}",
            manifest.version
        )));
    }
    if manifest.root.trim().is_empty() || manifest.root.chars().any(char::is_control) {
        return Err(invalid_manifest("asset manifest root is invalid"));
    }
    if manifest.files.is_empty() {
        return Err(invalid_manifest("asset manifest files must not be empty"));
    }

    let entry = normalize_runtime_manifest_path(&manifest.entry)?;
    let mut entries = BTreeMap::new();
    for file in manifest.files {
        let path = normalize_runtime_manifest_path(&file.path)?;
        let mime = validate_runtime_manifest_mime(&path, &file.mime)?;
        validate_runtime_manifest_sha(&path, &file.sha256)?;
        let previous = entries.insert(
            path.clone(),
            RuntimeAssetEntry {
                mime,
                sha256: file.sha256,
                size: file.size,
            },
        );
        if previous.is_some() {
            return Err(invalid_manifest(format!(
                "asset manifest contains duplicate path: {path}"
            )));
        }
    }

    if !entries.contains_key(&entry) {
        return Err(invalid_manifest(format!(
            "asset manifest entry is missing from files: {entry}"
        )));
    }
    Ok(entries)
}

#[cfg(feature = "wry-backend")]
fn normalize_runtime_manifest_path(path: &str) -> std::io::Result<String> {
    if !is_safe_runtime_manifest_path(path) {
        return Err(invalid_manifest(format!(
            "asset manifest path is not safe: {path}"
        )));
    }
    Ok(path.to_string())
}

#[cfg(feature = "wry-backend")]
fn is_safe_runtime_manifest_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(is_safe_runtime_manifest_segment)
}

#[cfg(feature = "wry-backend")]
fn is_safe_runtime_manifest_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'%' | b'?' | b'#' | b':'))
}

#[cfg(feature = "wry-backend")]
fn validate_runtime_manifest_mime(path: &str, mime: &str) -> std::io::Result<String> {
    let trimmed = mime.trim();
    if trimmed.is_empty() {
        return Err(invalid_manifest(format!("asset {path} has an empty mime")));
    }
    if trimmed != mime {
        return Err(invalid_manifest(format!(
            "asset {path} mime must not have leading or trailing whitespace"
        )));
    }
    wry::http::HeaderValue::from_str(mime).map_err(|_| {
        invalid_manifest(format!(
            "asset {path} mime is not a valid HTTP header value"
        ))
    })?;
    Ok(mime.to_string())
}

#[cfg(feature = "wry-backend")]
fn validate_runtime_manifest_sha(path: &str, sha256: &str) -> std::io::Result<()> {
    if sha256.len() == 64 && sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(invalid_manifest(format!(
            "asset {path} sha256 is not a 64-byte hex digest"
        )))
    }
}

#[cfg(feature = "wry-backend")]
fn invalid_manifest(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}

#[cfg(feature = "wry-backend")]
fn asset_request_key(request_path: &str) -> std::io::Result<String> {
    let key = request_path.trim_start_matches('/');
    if is_safe_runtime_manifest_path(key) {
        Ok(key.to_string())
    } else {
        Err(invalid_manifest(format!(
            "asset request path is not safe: {request_path}"
        )))
    }
}

#[cfg(feature = "wry-backend")]
fn mime_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(feature = "wry-backend")]
pub fn evaluate_packet(
    webview: &wry::WebView,
    packet: &BridgePacket,
) -> Result<(), WryBridgeError> {
    let script = packet_script(packet)?;
    webview.evaluate_script(&script)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vesty_ipc::{BridgeKind, BridgeLane, MAX_COMMAND_MESSAGE_BYTES, MAX_STATE_MESSAGE_BYTES};

    #[cfg(feature = "wry-backend")]
    static PANIC_HOOK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[cfg(feature = "wry-backend")]
    fn test_sha256(text: &str) -> String {
        Sha256::digest(text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[cfg(feature = "wry-backend")]
    fn with_suppressed_panic_hook<T>(f: impl FnOnce() -> T) -> T {
        let _guard = PANIC_HOOK_LOCK.lock().unwrap();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::panic::set_hook(previous);
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    #[test]
    fn renders_delivery_script() {
        let packet = BridgePacket::request("s", 1, BridgeLane::Command, "bridge.hello");
        let script = packet_script(&packet).unwrap();
        assert!(script.contains("deliver"));
        assert!(script.contains("bridge.hello"));
        assert!(script.contains("\"kind\":\"request\""));
        assert_eq!(packet.kind, BridgeKind::Request);
    }

    #[test]
    fn renders_batch_delivery_once() {
        let packet = BridgePacket::request("s", 1, BridgeLane::Command, "bridge.hello");
        let script = batch_script(&[packet]).unwrap();
        assert_eq!(script.matches("deliverBatch(").count(), 1);
        assert!(script.contains("bridge.hello"));
    }

    #[test]
    fn batch_scripts_chunks_at_bridge_batch_limit() {
        let packets = (0..=MAX_BRIDGE_BATCH_PACKETS)
            .map(|index| {
                BridgePacket::request("s", index as u64 + 1, BridgeLane::Command, "bridge.hello")
            })
            .collect::<Vec<_>>();

        let scripts = batch_scripts(&packets).unwrap();

        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0].matches("deliverBatch(").count(), 1);
        assert_eq!(
            scripts[0].matches("\"bridge.hello\"").count(),
            MAX_BRIDGE_BATCH_PACKETS
        );
        assert_eq!(scripts[1].matches("deliverBatch(").count(), 1);
        assert_eq!(scripts[1].matches("\"bridge.hello\"").count(), 1);
    }

    #[test]
    fn batch_scripts_returns_no_scripts_for_empty_batch() {
        let scripts = batch_scripts(&[]).unwrap();
        assert!(scripts.is_empty());
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn ipc_handler_guard_returns_packets_on_success() {
        let handler: IpcHandler = Rc::new(|body| {
            let packet = vesty_ipc::parse_packet(&body).unwrap();
            vec![packet.response_to(2, Some(serde_json::json!({ "ok": true })))]
        });

        let packets = call_ipc_handler_guarded(
            &handler,
            r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello"}"#.to_string(),
        );

        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].kind, BridgeKind::Response);
        assert_eq!(packets[0].reply_to.as_deref(), Some("hello"));
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn ipc_handler_guard_converts_panic_to_internal_error() {
        let handler: IpcHandler = Rc::new(|_| panic!("simulated ipc panic"));

        let packets = with_suppressed_panic_hook(|| {
            call_ipc_handler_guarded(
                &handler,
                r#"{"v":1,"session":"s","seq":41,"lane":"param","kind":"request","type":"param.perform","id":"drag","payload":{"id":"gain","normalized":0.5}}"#.to_string(),
            )
        });

        assert_eq!(packets.len(), 1);
        let packet = &packets[0];
        assert_eq!(packet.kind, BridgeKind::Error);
        assert_eq!(packet.packet_type, "param.perform.error");
        assert_eq!(packet.reply_to.as_deref(), Some("drag"));
        assert_eq!(packet.seq, 42);
        let error = packet.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::InternalError);
        assert_eq!(error.message, "native IPC handler panicked");
        assert!(error.retryable);
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn ipc_handler_guard_drops_unparseable_panic_response() {
        let handler: IpcHandler = Rc::new(|_| panic!("simulated ipc panic"));

        let packets = with_suppressed_panic_hook(|| {
            call_ipc_handler_guarded(&handler, "{not json".to_string())
        });

        assert!(packets.is_empty());
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn ipc_handler_guard_drops_malformed_panic_response_envelopes() {
        let cases = [
            r#"{"v":1,"session":"","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello"}"#,
            r#"{"v":1,"session":"s","seq":9007199254740992,"lane":"command","kind":"request","type":"bridge.hello","id":"hello"}"#,
            r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"response","type":"bridge.hello","id":"hello","replyTo":"server"}"#,
            r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello\u0007","id":"hello"}"#,
            r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":""}"#,
            r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello","flags":["bad\u0007flag"]}"#,
            r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello","replyTo":"server"}"#,
            r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello","error":{"code":"internal_error","message":"client supplied error","retryable":false}}"#,
        ];

        for body in cases {
            let handler: IpcHandler = Rc::new(|_| panic!("simulated ipc panic"));
            let packets =
                with_suppressed_panic_hook(|| call_ipc_handler_guarded(&handler, body.to_string()));
            assert!(packets.is_empty(), "unexpected panic fallback for {body}");
        }
    }

    #[test]
    fn bootstrap_script_registers_host_subscriptions() {
        let script = bootstrap_script();
        assert!(script.contains("\"subscription.add\""));
        assert!(script.contains("\"subscription.remove\""));
        assert!(script.contains("MAX_COMMAND_MESSAGE_BYTES"));
        assert!(script.contains(&format!(
            "const MAX_COMMAND_MESSAGE_BYTES = {};",
            MAX_COMMAND_MESSAGE_BYTES
        )));
        assert!(script.contains("MAX_STATE_MESSAGE_BYTES"));
        assert!(script.contains(&format!(
            "const MAX_STATE_MESSAGE_BYTES = {};",
            MAX_STATE_MESSAGE_BYTES
        )));
        assert!(script.contains("MAX_SUBSCRIPTION_TOPIC_BYTES"));
        assert!(script.contains("MAX_BRIDGE_PACKET_TYPE_BYTES"));
        assert!(script.contains("MAX_BRIDGE_PACKET_ID_BYTES"));
        assert!(script.contains("MAX_BRIDGE_PACKET_SEQ"));
        assert!(script.contains("MAX_BRIDGE_PACKET_FLAGS"));
        assert!(script.contains("MAX_BRIDGE_PACKET_FLAG_BYTES"));
        assert!(script.contains("MAX_BRIDGE_ERROR_MESSAGE_BYTES"));
        assert!(script.contains("MAX_BRIDGE_BATCH_PACKETS"));
        assert!(script.contains(&format!(
            "const MAX_BRIDGE_BATCH_PACKETS = {};",
            MAX_BRIDGE_BATCH_PACKETS
        )));
        assert!(script.contains("MAX_BRIDGE_JSON_DEPTH"));
        assert!(script.contains("MAX_BRIDGE_JSON_ARRAY_ITEMS"));
        assert!(script.contains("MAX_BRIDGE_JSON_OBJECT_KEYS"));
        assert!(script.contains("MAX_BRIDGE_JSON_NODES"));
        assert!(script.contains("MAX_BRIDGE_JSON_STRING_BYTES"));
        assert!(script.contains("BRIDGE_INBOUND_LANES"));
        assert!(script.contains("BRIDGE_INBOUND_KINDS"));
        assert!(script.contains("BRIDGE_REQUEST_LANES"));
        assert!(script.contains("BRIDGE_ERROR_CODES"));
        assert!(script.contains("function assertSubscriptionTopic"));
        assert!(script.contains("function assertRequestType"));
        assert!(script.contains("function assertRequestLane"));
        assert!(script.contains("function maxMessageBytesForLane"));
        assert!(script.contains("function validPacketString"));
        assert!(script.contains("function validPlainDataRecord"));
        assert!(script.contains("function hasOwnDataProperty"));
        assert!(script.contains("function ownDataValue"));
        assert!(script.contains("function validJsonValue"));
        assert!(script.contains("Number.isFinite(value)"));
        assert!(script.contains("Reflect.ownKeys(value)"));
        assert!(script.contains("Object.getOwnPropertyDescriptor(value, key)"));
        assert!(script.contains("function validBridgeError"));
        assert!(script.contains(
            "key !== \"code\" && key !== \"message\" && key !== \"details\" && key !== \"retryable\""
        ));
        assert!(script.contains("BRIDGE_ERROR_CODES.has(code)"));
        assert!(script.contains("utf8ByteLength(message) <= MAX_BRIDGE_ERROR_MESSAGE_BYTES"));
        assert!(script.contains("!hasDetails || validJsonValue(details)"));
        assert!(script.contains("function validPacketFlags"));
        assert!(script.contains("Number.isSafeInteger(seq)"));
        assert!(script.contains("kind === \"response\" && hasOwnDataProperty(packet, \"error\")"));
        assert!(script.contains("kind === \"error\" && hasOwnDataProperty(packet, \"payload\")"));
        assert!(script.contains("function validInboundPacket"));
        assert!(script.contains("function assertSubscriptionHandler"));
        assert!(script.contains("request type must be a string"));
        assert!(script.contains("request type must not be empty"));
        assert!(script.contains("request type too long"));
        assert!(script.contains("request type must not contain control characters"));
        assert!(script.contains("assertRequestType(type)"));
        assert!(script.contains("request lane must be a string"));
        assert!(script.contains("request lane must be a known bridge lane"));
        assert!(script.contains("assertRequestLane(lane)"));
        assert!(script.contains("subscription topic must be a string"));
        assert!(script.contains("subscription topic must not be empty"));
        assert!(script.contains("subscription topic too long"));
        assert!(script.contains("subscription topic must not contain control characters"));
        assert!(script.contains("subscription handler must be a function"));
        assert!(script.contains("validation_error"));
        assert!(script.contains("TextEncoder"));
        assert!(script.contains("MAX_BRIDGE_SESSION_BYTES"));
        assert!(script.contains("function assertBridgeSessionValue"));
        assert!(script.contains("function assertReadyEditorSessionId"));
        assert!(
            script.contains(
                "assertBridgeSessionValue(session, \"editorSessionId\", readyPayloadError)"
            )
        );
        assert!(script.contains("${name} too long"));
        assert!(script.contains("${name} must not contain control characters"));
        assert!(script.contains("session = payload.editorSessionId"));
        assert!(script.contains("assertBridgeSessionValue(value, \"session\", validationError)"));
        assert!(script.contains("setSession(value)"));
        assert!(script.contains("MAX_PARAM_GESTURE_ID_BYTES"));
        assert!(script.contains("MAX_CONFIG_KEY_BYTES"));
        assert!(script.contains("function assertConfigKey"));
        assert!(script.contains("function assertBaseRevision"));
        assert!(script.contains("function assertParamId"));
        assert!(script.contains("function assertNormalizedValue"));
        assert!(script.contains("function assertOptionalGestureId"));
        assert!(script.contains("function paramGesturePayload"));
        assert!(script.contains("function paramPerformPayload"));
        assert!(script.contains("if (gestureId !== undefined) payload.gestureId = gestureId"));
        assert!(script.contains("config key must not be empty"));
        assert!(script.contains("baseRevision must be a non-negative integer"));
        assert!(script.contains("parameter id must not be empty"));
        assert!(script.contains("normalized value must be a finite number"));
        assert!(script.contains("gestureId too long"));
        assert!(script.contains("function reportListenerError"));
        assert!(script.contains("console.error(\"Vesty bridge listener error\", type, error)"));
        assert!(script.contains("for (const listener of Array.from(set))"));
        assert!(script.contains("reportListenerError(type, error)"));
        assert!(script.contains("const shouldSubscribe = !set || set.size === 0"));
        assert!(script.contains("if (shouldSubscribe)"));
        assert!(script.contains("current.size === 0"));
        assert!(script.contains("\"state.setConfig\""));
        assert!(script.contains("\"state.setUiState\""));
        assert!(script.contains("\"event.flush\""));
        assert!(script.contains("topic === \"param.changed\""));
        assert!(script.contains("topic === \"diagnostics.fault\""));
        assert!(script.contains("topic === \"log.rt\""));
        assert!(script.contains("topic.startsWith(\"meter.\")"));
        assert!(script.contains("eventFlushInFlight"));
        assert!(script.contains("if (eventFlushInFlight) return"));
        assert!(script.contains("eventFlushInFlight = true"));
        assert!(script.contains("EVENT_FLUSH_TIMEOUT_MS"));
        assert!(script.contains(
            "request(\"event.flush\", \"event\", {}, { timeoutMs: EVENT_FLUSH_TIMEOUT_MS })"
        ));
        assert!(script.contains("eventFlushInFlight = false"));
        assert!(script.contains(".finally(() => { eventFlushInFlight = false; })"));
        assert!(!script.contains(
            "clearInterval(eventPump);\n    eventPump = 0;\n    eventFlushInFlight = false;"
        ));
        assert!(script.contains("if (!eventPump)"));
        assert!(script.contains("clearInterval(eventPump)"));
        assert!(script.contains("setInterval"));
        assert!(script.contains("REQUEST_TIMEOUT_MS"));
        assert!(script.contains("Number.isFinite(options.timeoutMs)"));
        assert!(script.contains("setTimeout"));
        assert!(script.contains("clearTimeout"));
        assert!(script.contains("Vesty request timed out"));
        assert!(script.contains("rejectAllPending"));
        assert!(script.contains("listeners.clear()"));
        assert!(script.contains("pagehide"));
        assert!(script.contains("beforeunload"));
        assert!(script.contains("Vesty WebView unloaded"));
        assert!(script.contains("adoptEditorSession"));
        assert!(script.contains("editorSessionId"));
        assert!(script.contains("supportedProtocolVersions"));
        assert!(script.contains("jsPackageVersion"));
        assert!(script.contains("pageUrl"));
        assert!(script.contains("readyAckPayload"));
        assert!(script.contains("\"bridge.readyAck\""));
        assert!(script.contains("let readyPromise = 0"));
        assert!(script.contains("let readyPayloadCache = 0"));
        assert!(
            script.contains("if (readyPayloadCache) return Promise.resolve(readyPayloadCache)")
        );
        assert!(script.contains("if (!readyPromise)"));
        assert!(script.contains("readyPayloadCache = payload"));
        assert!(script.contains("async setParam(id, normalized, gestureId)"));
        assert!(script.contains("async setConfig"));
        assert!(script.contains("async setUiState"));
        assert!(script.contains("async beginParamEdit"));
        assert!(script.contains("async beginParamEdit(id, gestureId)"));
        assert!(script.contains("\"param.begin\", \"param\", paramGesturePayload(id, gestureId)"));
        assert!(script.contains("async performParamEdit"));
        assert!(script.contains(
            "\"param.perform\", \"param\", paramPerformPayload(id, normalized, gestureId)"
        ));
        assert!(script.contains("async endParamEdit"));
        assert!(script.contains("async formatParam"));
        assert!(script.contains("async parseParam"));
        assert!(script.contains("await this.beginParamEdit(id, gestureId)"));
        assert!(script.contains("await this.performParamEdit(id, normalized, gestureId)"));
        assert!(script.contains("await this.endParamEdit(id, gestureId)"));
        assert!(script.contains("if (failure !== undefined) throw failure"));
        assert!(script.contains("unsupportedProtocolError"));
        assert!(script.contains("unsupported_version"));
        assert!(script.contains("assertCompatibleReadyPayload"));
        assert!(script.contains("assertSubscriptionHandler(handler)"));
        assert!(script.contains("readyPayloadError"));
        assert!(script.contains("assertCapabilities"));
        assert!(script.contains("assertPluginSnapshot"));
        assert!(script.contains("assertReadyParams"));
        assert!(script.contains("assertParamSpec"));
        assert!(script.contains("assertParamMidiMappings"));
        assert!(script.contains("duplicate parameter id"));
        assert!(script.contains("midiMappings must be an array"));
        assert!(script.contains("capabilities.${key} must be boolean"));
        assert!(script.contains("snapshot.revision"));
        assert!(script.contains("params must be an array"));
        assert!(script.contains("Unsupported Vesty bridge protocol version"));
        assert!(script.contains("packetMatchesSession"));
        assert!(script.contains("ownDataValue(packet, \"session\") === session"));
        assert!(script.contains("if (!validInboundPacket(packet)) return"));
        assert!(script.contains("if (!Array.isArray(packets)) return"));
        assert!(script.contains("if (packets.length > MAX_BRIDGE_BATCH_PACKETS) return"));
        assert!(script.contains("const kind = ownDataValue(packet, \"kind\")"));
        assert!(script.contains("if (kind === \"response\" || kind === \"error\")"));
        assert!(script.contains("if (!validPacketFlags(flags)) return false"));
        assert!(script.contains("if (hasOwnDataProperty(packet, \"id\")) return false"));
        assert!(script.contains(
            "if (hasOwnDataProperty(packet, \"payload\") && !validJsonValue(payload)) return false"
        ));
        assert!(script.contains(
            "else if (hasOwnDataProperty(packet, \"replyTo\") || hasOwnDataProperty(packet, \"error\"))"
        ));
        assert!(script.contains("if (kind !== \"event\") return"));
        assert!(script.contains("pending.set(id"));
        assert!(script.contains("request payload must be JSON-compatible"));
        assert!(script.contains("if (payload !== undefined) packet.payload = payload"));
        assert!(script.contains("utf8ByteLength(message) > maxMessageBytesForLane(lane)"));
        assert!(script.contains("postMessage(message)"));
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn dev_url_is_explicit_in_release() {
        // In debug builds dev URLs remain convenient by default. Release/plugin smoke is covered
        // by the VESTY_UI_DEV env gate.
        if cfg!(debug_assertions) {
            assert!(use_dev_url());
        } else {
            // SAFETY: This test is single-threaded with respect to VESTY_UI_DEV and restores no
            // shared invariant beyond checking this crate's environment flag parser.
            unsafe {
                std::env::remove_var("VESTY_UI_DEV");
            }
            assert!(!use_dev_url());
            // SAFETY: See the note above; this mutation is scoped to this test process.
            unsafe {
                std::env::set_var("VESTY_UI_DEV", "1");
            }
            assert!(use_dev_url());
        }
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn devtools_policy_defaults_to_debug_and_allows_env_override() {
        assert!(ui_env_flag_truthy("1"));
        assert!(ui_env_flag_truthy(" true "));
        assert!(ui_env_flag_truthy("YES"));
        assert!(ui_env_flag_truthy("on"));
        assert!(!ui_env_flag_truthy("0"));
        assert!(!ui_env_flag_truthy("false"));
        assert!(!ui_env_flag_truthy("off"));

        // SAFETY: This unit test owns the VESTY_UI_DEVTOOLS setting for the duration of the test
        // process and only exercises this crate's environment flag parser.
        unsafe {
            std::env::remove_var("VESTY_UI_DEVTOOLS");
        }
        assert_eq!(use_devtools(), cfg!(debug_assertions));

        // SAFETY: See the note above; the mutation is intentionally process-local for the test.
        unsafe {
            std::env::set_var("VESTY_UI_DEVTOOLS", "0");
        }
        assert!(!use_devtools());

        // SAFETY: See the note above; the mutation is intentionally process-local for the test.
        unsafe {
            std::env::set_var("VESTY_UI_DEVTOOLS", "yes");
        }
        assert!(use_devtools());

        // SAFETY: See the note above; cleanup is scoped to the same test process.
        unsafe {
            std::env::remove_var("VESTY_UI_DEVTOOLS");
        }
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn asset_protocol_uses_manifest_allowlist() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ui");
        let index_html = "<html></html>";
        let app_js = "console.log('ok')";
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("index.html"), index_html).unwrap();
        std::fs::write(root.join("assets/app.js"), app_js).unwrap();
        std::fs::write(root.join("secret.txt"), "do not serve").unwrap();
        std::fs::write(dir.path().join("outside.txt"), "outside").unwrap();
        std::fs::write(
            dir.path().join("assets.manifest.json"),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256(index_html),
                        "size": index_html.len(),
                    },
                    {
                        "path": "assets/app.js",
                        "mime": "text/javascript",
                        "sha256": test_sha256(app_js),
                        "size": app_js.len(),
                    },
                ],
            })
            .to_string(),
        )
        .unwrap();

        let manifest = load_asset_manifest(&root).expect("manifest");
        assert!(safe_asset_path(&root, "/index.html", &manifest).is_ok());
        assert!(safe_asset_path(&root, "/assets/app.js", &manifest).is_ok());
        assert!(safe_asset_path(&root, "/secret.txt", &manifest).is_err());
        assert!(safe_asset_path(&root, "/../outside.txt", &manifest).is_err());
        assert!(safe_asset_path(&root, "/assets/%2e%2e/app.js", &manifest).is_err());
        assert!(safe_asset_path(&root, "/assets//app.js", &manifest).is_err());

        let response = asset_response(&root, &manifest, "/index.html");
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.headers().get("Content-Type").unwrap(), "text/html");
        assert_eq!(
            response.headers().get("Content-Security-Policy").unwrap(),
            release_asset_csp()
        );
        assert_eq!(
            response.headers().get("X-Content-Type-Options").unwrap(),
            "nosniff"
        );

        let response = asset_response(&root, &manifest, "/secret.txt");
        assert_eq!(response.status().as_u16(), 404);
        assert_eq!(
            response.headers().get("X-Content-Type-Options").unwrap(),
            "nosniff"
        );

        std::fs::write(root.join("assets/app.js"), "console.log('no')").unwrap();
        let response = asset_response(&root, &manifest, "/assets/app.js");
        assert_eq!(response.status().as_u16(), 404);
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn asset_manifest_is_required_and_invalid_json_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ui");
        std::fs::create_dir_all(&root).unwrap();

        let missing = load_asset_manifest(&root).unwrap_err();
        assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);

        std::fs::write(dir.path().join("assets.manifest.json"), "{bad json").unwrap();
        let invalid = load_asset_manifest(&root).unwrap_err();
        assert_eq!(invalid.kind(), std::io::ErrorKind::InvalidData);
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn asset_manifest_rejects_unknown_json_fields() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ui");
        std::fs::create_dir_all(&root).unwrap();
        let manifest_path = dir.path().join("assets.manifest.json");

        for manifest in [
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("<html></html>"),
                        "size": "<html></html>".len(),
                    },
                ],
                "integrity": "extra",
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("<html></html>"),
                        "size": "<html></html>".len(),
                        "integrity": "extra",
                    },
                ],
            }),
        ] {
            std::fs::write(&manifest_path, manifest.to_string()).unwrap();

            let error = load_asset_manifest(&root).unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert!(
                error.to_string().contains("unknown field"),
                "unexpected error: {error}"
            );
        }
    }

    #[cfg(unix)]
    #[cfg(feature = "wry-backend")]
    #[test]
    fn asset_manifest_rejects_symlinked_manifest_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ui");
        std::fs::create_dir_all(&root).unwrap();
        let external_manifest = dir.path().join("external-assets.manifest.json");
        std::fs::write(
            &external_manifest,
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("<html></html>"),
                        "size": "<html></html>".len(),
                    },
                ],
            })
            .to_string(),
        )
        .unwrap();
        std::os::unix::fs::symlink(&external_manifest, dir.path().join("assets.manifest.json"))
            .unwrap();

        let error = load_asset_manifest(&root).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("must not be a symlink"));
    }

    #[cfg(unix)]
    #[cfg(feature = "wry-backend")]
    #[test]
    fn asset_manifest_rejects_symlinked_asset_root() {
        let dir = tempfile::tempdir().unwrap();
        let external_root = dir.path().join("external-ui");
        let root = dir.path().join("ui");
        std::fs::create_dir(&external_root).unwrap();
        std::os::unix::fs::symlink(&external_root, &root).unwrap();

        let error = load_asset_manifest(&root).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("UI asset root"));
    }

    #[cfg(unix)]
    #[cfg(feature = "wry-backend")]
    #[test]
    fn asset_protocol_rejects_symlinked_manifest_assets() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ui");
        let target_js = "console.log('target')";
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("index.html"), "<html></html>").unwrap();
        std::fs::write(root.join("assets/target.js"), target_js).unwrap();
        std::os::unix::fs::symlink(root.join("assets/target.js"), root.join("assets/link.js"))
            .unwrap();
        std::fs::write(
            dir.path().join("assets.manifest.json"),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("<html></html>"),
                        "size": "<html></html>".len(),
                    },
                    {
                        "path": "assets/link.js",
                        "mime": "text/javascript",
                        "sha256": test_sha256(target_js),
                        "size": target_js.len(),
                    },
                ],
            })
            .to_string(),
        )
        .unwrap();

        let manifest = load_asset_manifest(&root).expect("manifest");

        assert!(safe_asset_path(&root, "/assets/link.js", &manifest).is_err());
        let response = asset_response(&root, &manifest, "/assets/link.js");
        assert_eq!(response.status().as_u16(), 404);
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn asset_manifest_rejects_invalid_entries_before_serving() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ui");
        std::fs::create_dir_all(&root).unwrap();
        let manifest_path = dir.path().join("assets.manifest.json");

        for manifest in [
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "missing.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html\nx-bad: 1",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "../secret.txt",
                        "mime": "text/plain",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html?cache=1",
                        "mime": "text/html",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html#fragment",
                        "mime": "text/html",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "C:index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "assets/%2e%2e/app.js",
                        "mime": "text/javascript",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": test_sha256("ok"),
                        "size": 2,
                    },
                ],
            }),
            serde_json::json!({
                "version": 1,
                "root": "ui",
                "entry": "index.html",
                "files": [
                    {
                        "path": "index.html",
                        "mime": "text/html",
                        "sha256": "not-a-sha",
                        "size": 2,
                    },
                ],
            }),
        ] {
            std::fs::write(&manifest_path, manifest.to_string()).unwrap();
            let error = load_asset_manifest(&root).unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        }
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn release_navigation_only_allows_bundle_asset_urls() {
        assert!(release_navigation_allowed("about:blank".to_string()));
        assert!(release_navigation_allowed(
            "vesty://assets/index.html".to_string()
        ));
        assert!(release_navigation_allowed("VESTY://ASSETS/".to_string()));

        assert!(!release_navigation_allowed(
            "http://vesty.assets/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "https://vesty.assets/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "https://example.com/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "http://localhost:5173/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://other/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "file:///tmp/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://assets.evil/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://assets:443/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://user@assets/index.html".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://assets/index.html?cache=1".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://assets/index.html#fragment".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://assets/assets/%2e%2e/app.js".to_string()
        ));
        assert!(!release_navigation_allowed(
            "vesty://assets/assets//app.js".to_string()
        ));
        assert!(!release_navigation_allowed(
            " vesty://assets/index.html".to_string()
        ));
    }

    #[cfg(feature = "wry-backend")]
    #[test]
    fn release_ipc_only_allows_bundle_asset_urls() {
        assert!(release_ipc_allowed("vesty://assets/index.html"));
        assert!(release_ipc_allowed("VESTY://ASSETS/"));

        assert!(!release_ipc_allowed("about:blank"));
        assert!(!release_ipc_allowed("http://vesty.assets/index.html"));
        assert!(!release_ipc_allowed("https://vesty.assets/index.html"));
        assert!(!release_ipc_allowed("https://example.com/index.html"));
        assert!(!release_ipc_allowed("http://localhost:5173/index.html"));
        assert!(!release_ipc_allowed("vesty://other/index.html"));
        assert!(!release_ipc_allowed("file:///tmp/index.html"));
        assert!(!release_ipc_allowed("https://vesty.assets.evil/index.html"));
        assert!(!release_ipc_allowed("https://vesty.assets:443/index.html"));
        assert!(!release_ipc_allowed("https://user@vesty.assets/index.html"));
        assert!(!release_ipc_allowed(
            "https://vesty.assets/index.html?cache=1"
        ));
        assert!(!release_ipc_allowed(
            "https://vesty.assets/index.html#fragment"
        ));
        assert!(!release_ipc_allowed(
            "https://vesty.assets/assets/%2e%2e/app.js"
        ));
        assert!(!release_ipc_allowed("https://vesty.assets/assets\\app.js"));
        assert!(!release_ipc_allowed("https://vesty.assets/assets//app.js"));
    }
}
