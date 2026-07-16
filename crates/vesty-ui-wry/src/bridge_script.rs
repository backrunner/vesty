use thiserror::Error;
use vesty_ipc::BridgePacket;

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
    return ids;
  }

  function assertReadyParamValues(value, paramIds) {
    if (!Array.isArray(value)) throw readyPayloadError("paramValues must be an array");
    if (value.length !== paramIds.size) {
      throw readyPayloadError("paramValues must contain one value for every parameter");
    }
    const seen = new Set();
    value.forEach((entry, index) => {
      const name = `paramValues[${index}]`;
      const record = assertRecord(entry, name);
      const id = assertNonEmptyString(record.id, `${name}.id`);
      if (!paramIds.has(id)) throw readyPayloadError(`${name}.id references an unknown parameter`);
      if (seen.has(id)) throw readyPayloadError(`duplicate current parameter value '${id}'`);
      seen.add(id);
      const normalized = assertFiniteNumber(record.normalized, `${name}.normalized`);
      if (normalized < 0 || normalized > 1) {
        throw readyPayloadError(`${name}.normalized must be within 0.0..=1.0`);
      }
    });
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
    const paramIds = assertReadyParams(ready.params);
    assertReadyParamValues(ready.paramValues, paramIds);
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
