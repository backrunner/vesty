import type { JsonValue } from "./serde_json/JsonValue";
import type { BridgeErrorPayload } from "./protocol/BridgeErrorPayload";
import type { BridgeLane } from "./protocol/BridgeLane";
import type { BridgePacket as ProtocolBridgePacket } from "./protocol/BridgePacket";
import type { BridgeReadyPayload } from "./protocol/BridgeReadyPayload";
import type { BridgeDiagnosticsSnapshot } from "./protocol/BridgeDiagnosticsSnapshot";
import type { PluginSnapshot } from "./protocol/PluginSnapshot";

export type {
  BridgeCapabilities,
  BridgeDiagnosticsSnapshot,
  BridgeErrorCode,
  BridgeErrorPayload,
  BridgeHelloPayload,
  BridgeKind,
  BridgeLane,
  BridgeReadyPayload,
  ParamChangeSource,
  ParamChangedEvent,
  ParamFlags,
  ParamKind,
  ParamMidiMapping,
  ParamSpec,
  PluginFaultReport,
  PluginSnapshot,
  RtLogKind,
  RtLogLevel,
  RtLogQueue,
  RtLogRecord
} from "./protocol";

export type { JsonValue } from "./serde_json/JsonValue";

export interface BridgePacket<T = JsonValue> extends Omit<ProtocolBridgePacket, "v" | "payload" | "error"> {
  v: 1;
  payload?: T;
  error?: BridgeError;
}

export type BridgeError = BridgeErrorPayload;

export interface VestyBridge {
  ready<T = BridgeReadyPayload>(): Promise<T>;
  getSnapshot<T = unknown>(): Promise<T>;
  getDiagnostics<T = BridgeDiagnosticsSnapshot>(): Promise<T>;
  request<T = unknown>(type: string, payload?: unknown): Promise<T>;
  subscribe<T = unknown>(topic: string, handler: (event: T) => void): () => void;
  setConfig<T = unknown>(key: string, value: unknown, baseRevision: number): Promise<T>;
  setUiState<T = unknown>(value: unknown, baseRevision: number): Promise<T>;
  beginParamEdit(id: string, gestureId?: string): Promise<void>;
  performParamEdit(id: string, normalized: number, gestureId?: string): Promise<void>;
  endParamEdit(id: string, gestureId?: string): Promise<void>;
  setParam(id: string, normalized: number, gestureId?: string): Promise<void>;
  formatParam(id: string, normalized: number): Promise<string>;
  parseParam(id: string, text: string): Promise<number>;
}

export interface CreateBridgeOptions {
  timeoutMs?: number;
}

export interface SnapshotStoreOptions {
  topic?: string;
  refreshOnEvent?: boolean;
}

export type SnapshotListener<TSnapshot = PluginSnapshot> = (snapshot: TSnapshot) => void;

export interface VestySnapshotStore<TSnapshot = PluginSnapshot> {
  getSnapshot(): TSnapshot | undefined;
  refresh(): Promise<TSnapshot>;
  subscribe(listener: SnapshotListener<TSnapshot>): () => void;
  select<T>(selector: (snapshot: TSnapshot) => T): T | undefined;
  dispose(): void;
}

type Pending = {
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
  timer?: ReturnType<typeof setTimeout>;
};

type Listener = (payload: unknown) => void;

type PostOptions = {
  timeoutMs?: number;
};

export interface VestyHostWindow extends Window {
  ipc?: { postMessage(message: string): void };
  __VESTY__?: VestyBridge;
  __VESTY_INTERNAL__?: InternalBridge;
}

interface InternalBridge {
  deliver(packet: BridgePacket): void;
  deliverBatch(packets: BridgePacket[]): void;
}

const DEFAULT_REQUEST_TIMEOUT_MS = 5000;
const EVENT_FLUSH_TIMEOUT_MS = 1000;
const MAX_COMMAND_MESSAGE_BYTES = 64 * 1024;
const MAX_STATE_MESSAGE_BYTES = 256 * 1024;
const BRIDGE_PROTOCOL_VERSION = 1;
const PLUGIN_UI_PACKAGE_VERSION = "0.1.0";
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
const BRIDGE_ERROR_CODES = new Set([
  "parse_error",
  "unsupported_version",
  "unsupported_type",
  "validation_error",
  "permission_denied",
  "timeout",
  "backpressure",
  "host_rejected",
  "plugin_faulted",
  "state_conflict",
  "internal_error"
]);

function requestTimeout(type: string): BridgeError {
  return {
    code: "timeout",
    message: `Vesty request timed out: ${type}`,
    retryable: true
  };
}

function unloadError(): BridgeError {
  return {
    code: "internal_error",
    message: "Vesty WebView unloaded before the request completed",
    retryable: true
  };
}

function unsupportedProtocolError(protocolVersion: unknown): BridgeError {
  return {
    code: "unsupported_version",
    message: `Unsupported Vesty bridge protocol version: ${String(protocolVersion)}`,
    retryable: false
  };
}

function validationError(message: string): BridgeError {
  return {
    code: "validation_error",
    message,
    retryable: false
  };
}

function backpressureError(message: string): BridgeError {
  return {
    code: "backpressure",
    message,
    retryable: true
  };
}

function maxMessageBytesForLane(lane: BridgeLane): number {
  return lane === "state" ? MAX_STATE_MESSAGE_BYTES : MAX_COMMAND_MESSAGE_BYTES;
}

function clearPending(pending: Map<string, Pending>, id: string, item: Pending): void {
  if (item.timer) clearTimeout(item.timer);
  pending.delete(id);
}

function pageUrl(host: VestyHostWindow): string {
  const location = (host as { location?: { href?: unknown } }).location;
  return typeof location?.href === "string" ? location.href : "";
}

function helloPayload(host: VestyHostWindow): Record<string, unknown> {
  return {
    supportedProtocolVersions: [BRIDGE_PROTOCOL_VERSION],
    jsPackageVersion: PLUGIN_UI_PACKAGE_VERSION,
    pageUrl: pageUrl(host)
  };
}

function readyAckPayload(payload: unknown): Record<string, unknown> {
  const protocolVersion =
    payload &&
    typeof payload === "object" &&
    typeof (payload as { protocolVersion?: unknown }).protocolVersion === "number"
      ? (payload as { protocolVersion: number }).protocolVersion
      : BRIDGE_PROTOCOL_VERSION;
  return { protocolVersion };
}

function packetPayload(payload: unknown): JsonValue | undefined {
  if (payload === undefined) return undefined;
  if (!validJsonValue(payload)) throw validationError("request payload must be JSON-compatible");
  return payload;
}

function readyPayloadError(message: string): BridgeError {
  return validationError(`Invalid Vesty bridge ready payload: ${message}`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function containsControl(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code <= 0x1f || code === 0x7f || (code >= 0x80 && code <= 0x9f)) return true;
  }
  return false;
}

function utf8ByteLength(value: string): number {
  if (typeof TextEncoder !== "undefined") {
    return new TextEncoder().encode(value).length;
  }

  let bytes = 0;
  for (const char of value) {
    const codePoint = char.codePointAt(0) ?? 0;
    if (codePoint <= 0x7f) bytes += 1;
    else if (codePoint <= 0x7ff) bytes += 2;
    else if (codePoint <= 0xffff) bytes += 3;
    else bytes += 4;
  }
  return bytes;
}

function assertSubscriptionTopic(topic: unknown): void {
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

function assertRequestType(type: unknown): asserts type is string {
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

function validPacketString(value: unknown, maxBytes: number): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    utf8ByteLength(value) <= maxBytes &&
    !containsControl(value)
  );
}

function validPlainDataRecord(value: object): boolean {
  const proto = Object.getPrototypeOf(value);
  if (proto !== Object.prototype && proto !== null) return false;
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key !== "string") return false;
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    if (!descriptor || !descriptor.enumerable || !("value" in descriptor)) return false;
  }
  return true;
}

function hasOwnDataProperty(value: object, key: string): boolean {
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  return descriptor ? descriptor.enumerable === true && "value" in descriptor : false;
}

function ownDataValue(value: object, key: string): unknown {
  return Object.getOwnPropertyDescriptor(value, key)?.value;
}

function validJsonValue(
  value: unknown,
  depth = 0,
  stack = new Set<object>(),
  budget = { nodes: 0 }
): value is JsonValue {
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

function validBridgeError(value: unknown): value is BridgeError {
  if (!isRecord(value)) return false;
  if (!validPlainDataRecord(value)) return false;
  for (const key of Reflect.ownKeys(value)) {
    if (
      typeof key !== "string" ||
      (key !== "code" && key !== "message" && key !== "details" && key !== "retryable")
    ) {
      return false;
    }
  }
  const code = ownDataValue(value, "code");
  const message = ownDataValue(value, "message");
  const retryable = ownDataValue(value, "retryable");
  const hasDetails = hasOwnDataProperty(value, "details");
  const details = ownDataValue(value, "details");
  return (
    validPacketString(code, MAX_BRIDGE_PACKET_TYPE_BYTES) &&
    BRIDGE_ERROR_CODES.has(code) &&
    typeof message === "string" &&
    utf8ByteLength(message) <= MAX_BRIDGE_ERROR_MESSAGE_BYTES &&
    !containsControl(message) &&
    typeof retryable === "boolean" &&
    (!hasDetails || validJsonValue(details))
  );
}

function validPacketFlags(value: unknown): boolean {
  if (value === undefined) return true;
  if (!Array.isArray(value) || value.length > MAX_BRIDGE_PACKET_FLAGS) return false;
  return value.every((flag) => validPacketString(flag, MAX_BRIDGE_PACKET_FLAG_BYTES));
}

function validInboundPacket(packet: unknown): packet is BridgePacket {
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
  if (
    typeof seq !== "number" ||
    !Number.isSafeInteger(seq) ||
    seq < 0 ||
    seq > MAX_BRIDGE_PACKET_SEQ
  ) {
    return false;
  }
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

function assertSubscriptionHandler(handler: unknown): asserts handler is Listener {
  if (typeof handler !== "function") {
    throw validationError("subscription handler must be a function");
  }
}

function assertBridgeSessionValue(
  session: unknown,
  name: string,
  makeError: (message: string) => BridgeError
): asserts session is string {
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

function assertBridgeHost(host: unknown): asserts host is VestyHostWindow {
  if (!host || typeof host !== "object") {
    throw validationError("bridge host must be an object");
  }
  if (typeof (host as { addEventListener?: unknown }).addEventListener !== "function") {
    throw validationError("bridge host must provide addEventListener");
  }
}

function assertBridgeSession(session: unknown): asserts session is string {
  assertBridgeSessionValue(session, "initialSession", validationError);
}

function assertReadyEditorSessionId(session: unknown): asserts session is string {
  assertBridgeSessionValue(session, "editorSessionId", readyPayloadError);
}

function assertCreateBridgeOptions(options: unknown): asserts options is CreateBridgeOptions | undefined {
  if (options === undefined) return;
  if (!options || typeof options !== "object" || Array.isArray(options)) {
    throw validationError("bridge options must be an object");
  }
  const record = options as Record<string, unknown>;
  if ("timeoutMs" in record && record.timeoutMs !== undefined) {
    if (typeof record.timeoutMs !== "number" || !Number.isFinite(record.timeoutMs)) {
      throw validationError("timeoutMs must be a finite number");
    }
  }
}

function assertSnapshotStoreOptions(options: unknown): asserts options is SnapshotStoreOptions | undefined {
  if (options === undefined) return;
  if (!options || typeof options !== "object" || Array.isArray(options)) {
    throw validationError("snapshot store options must be an object");
  }
  const record = options as Record<string, unknown>;
  if ("topic" in record && record.topic !== undefined) assertSubscriptionTopic(record.topic);
  if ("refreshOnEvent" in record && record.refreshOnEvent !== undefined) {
    if (typeof record.refreshOnEvent !== "boolean") {
      throw validationError("refreshOnEvent must be boolean");
    }
  }
}

function assertConfigKey(key: unknown): void {
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

function assertBaseRevision(value: unknown): void {
  if (typeof value !== "number" || !Number.isFinite(value) || !Number.isInteger(value) || value < 0) {
    throw validationError("baseRevision must be a non-negative integer");
  }
}

function assertJsonValuePresent(value: unknown, name: string): void {
  if (value === undefined) {
    throw validationError(`${name} must not be undefined`);
  }
}

function assertParamId(id: unknown): void {
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

function assertNormalizedValue(normalized: unknown): void {
  if (typeof normalized !== "number" || !Number.isFinite(normalized)) {
    throw validationError("normalized value must be a finite number");
  }
}

function assertParamText(text: unknown): void {
  if (typeof text !== "string") {
    throw validationError("parameter text must be a string");
  }
}

function assertOptionalGestureId(gestureId: unknown): void {
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

function paramGesturePayload(id: string, gestureId: string | undefined): Record<string, unknown> {
  const payload: Record<string, unknown> = { id };
  if (gestureId !== undefined) payload.gestureId = gestureId;
  return payload;
}

function paramPerformPayload(
  id: string,
  normalized: number,
  gestureId: string | undefined
): Record<string, unknown> {
  const payload: Record<string, unknown> = { id, normalized };
  if (gestureId !== undefined) payload.gestureId = gestureId;
  return payload;
}

function assertRecord(value: unknown, name: string): Record<string, unknown> {
  if (!isRecord(value)) throw readyPayloadError(`${name} must be an object`);
  return value;
}

function assertNonEmptyString(value: unknown, name: string): string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw readyPayloadError(`${name} must be a non-empty string`);
  }
  if (containsControl(value)) throw readyPayloadError(`${name} must not contain control characters`);
  return value;
}

function assertFiniteNumber(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw readyPayloadError(`${name} must be a finite number`);
  }
  return value;
}

function assertRevision(value: unknown, name: string): void {
  const revision = assertFiniteNumber(value, name);
  if (!Number.isInteger(revision) || revision < 0) {
    throw readyPayloadError(`${name} must be a non-negative integer`);
  }
}

function assertCapabilities(value: unknown): void {
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

function assertPluginSnapshot(value: unknown): void {
  const snapshot = assertRecord(value, "snapshot");
  assertRevision(snapshot.revision, "snapshot.revision");
  assertRevision(snapshot.paramsRevision, "snapshot.paramsRevision");
  assertRevision(snapshot.configRevision, "snapshot.configRevision");
  assertRevision(snapshot.uiRevision, "snapshot.uiRevision");
  if (!("config" in snapshot)) throw readyPayloadError("snapshot.config is required");
  if (!("uiState" in snapshot)) throw readyPayloadError("snapshot.uiState is required");
}

function assertOptionalString(value: unknown, name: string): void {
  if (value === null) return;
  if (typeof value !== "string") throw readyPayloadError(`${name} must be a string or null`);
  if (containsControl(value)) throw readyPayloadError(`${name} must not contain control characters`);
}

function assertParamMidiMappings(value: unknown, name: string): void {
  if (!Array.isArray(value)) throw readyPayloadError(`${name}.midiMappings must be an array`);
  const seen = new Set<string>();
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

function assertParamSpec(value: unknown, index: number, ids: Set<string>): void {
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

function assertReadyParams(value: unknown): void {
  if (!Array.isArray(value)) throw readyPayloadError("params must be an array");
  const ids = new Set<string>();
  value.forEach((param, index) => assertParamSpec(param, index, ids));
}

function assertCompatibleReadyPayload(payload: unknown): asserts payload is BridgeReadyPayload {
  const ready = assertRecord(payload, "ready payload");
  const protocolVersion =
    typeof ready.protocolVersion === "number" ? ready.protocolVersion : undefined;
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

function looksLikePluginSnapshot(value: unknown): value is PluginSnapshot {
  if (!value || typeof value !== "object") return false;
  const snapshot = value as Partial<PluginSnapshot>;
  return (
    typeof snapshot.revision === "number" &&
    typeof snapshot.paramsRevision === "number" &&
    typeof snapshot.configRevision === "number" &&
    typeof snapshot.uiRevision === "number" &&
    "config" in snapshot &&
    "uiState" in snapshot
  );
}

function assertSnapshotListener<TSnapshot>(
  listener: unknown
): asserts listener is SnapshotListener<TSnapshot> {
  if (typeof listener !== "function") {
    throw validationError("snapshot listener must be a function");
  }
}

function assertSnapshotSelector<TSnapshot, TResult>(
  selector: unknown
): asserts selector is (snapshot: TSnapshot) => TResult {
  if (typeof selector !== "function") {
    throw validationError("snapshot selector must be a function");
  }
}

export function createSnapshotStore<TSnapshot extends PluginSnapshot = PluginSnapshot>(
  bridge: Pick<VestyBridge, "getSnapshot" | "subscribe">,
  initialSnapshot?: TSnapshot,
  options?: SnapshotStoreOptions
): VestySnapshotStore<TSnapshot> {
  assertSnapshotStoreOptions(options);
  let current = initialSnapshot;
  let unsubscribeBridge: (() => void) | undefined;
  const topic = options?.topic ?? "state.changed";
  const refreshOnEvent = options?.refreshOnEvent ?? true;
  const listeners = new Set<SnapshotListener<TSnapshot>>();

  function reportSnapshotListenerError(error: unknown): void {
    try {
      console.error("Vesty snapshot listener error", error);
    } catch {
      // Ignore console failures; listener isolation must not throw from store delivery.
    }
  }

  function notifySnapshotListener(listener: SnapshotListener<TSnapshot>, snapshot: TSnapshot): void {
    try {
      listener(snapshot);
    } catch (error) {
      reportSnapshotListenerError(error);
    }
  }

  function publish(snapshot: TSnapshot): void {
    current = snapshot;
    for (const listener of [...listeners]) notifySnapshotListener(listener, snapshot);
  }

  async function refresh(): Promise<TSnapshot> {
    const snapshot = await bridge.getSnapshot<TSnapshot>();
    publish(snapshot);
    return snapshot;
  }

  function startBridgeSubscription(): void {
    if (unsubscribeBridge) return;
    unsubscribeBridge = bridge.subscribe(topic, (payload) => {
      if (looksLikePluginSnapshot(payload)) {
        publish(payload as TSnapshot);
        return;
      }
      if (refreshOnEvent) void refresh().catch(() => undefined);
    });
  }

  function stopBridgeSubscription(): void {
    if (!unsubscribeBridge) return;
    unsubscribeBridge();
    unsubscribeBridge = undefined;
  }

  return {
    getSnapshot: () => current,
    refresh,
    subscribe(listener) {
      assertSnapshotListener<TSnapshot>(listener);
      listeners.add(listener);
      startBridgeSubscription();
      if (current) notifySnapshotListener(listener, current);
      return () => {
        listeners.delete(listener);
        if (listeners.size === 0) stopBridgeSubscription();
      };
    },
    select(selector) {
      assertSnapshotSelector<TSnapshot, ReturnType<typeof selector>>(selector);
      return current ? selector(current) : undefined;
    },
    dispose() {
      listeners.clear();
      stopBridgeSubscription();
    }
  };
}

export function createBridge(
  host: VestyHostWindow = window as VestyHostWindow,
  initialSession = "pending",
  options?: CreateBridgeOptions
): VestyBridge {
  assertBridgeHost(host);
  assertBridgeSession(initialSession);
  assertCreateBridgeOptions(options);
  let session = initialSession;
  let seq = 1;
  let eventPump: ReturnType<typeof setInterval> | undefined;
  let eventFlushInFlight = false;
  let readyPromise: Promise<BridgeReadyPayload> | undefined;
  let readyPayloadCache: BridgeReadyPayload | undefined;
  const pending = new Map<string, Pending>();
  const listeners = new Map<string, Set<Listener>>();
  const timeoutMs = Math.max(0, options?.timeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS);

  function stopEventPump(): void {
    if (!eventPump) return;
    clearInterval(eventPump);
    eventPump = undefined;
  }

  function rejectAllPending(error: BridgeError): void {
    for (const [id, item] of pending) {
      clearPending(pending, id, item);
      item.reject(error);
    }
  }

  function disposeForUnload(): void {
    stopEventPump();
    listeners.clear();
    readyPromise = undefined;
    readyPayloadCache = undefined;
    rejectAllPending(unloadError());
  }

  function adoptEditorSession(payload: BridgeReadyPayload): void {
    session = payload.editorSessionId;
  }

  host.addEventListener("pagehide", disposeForUnload, { once: true });
  host.addEventListener("beforeunload", disposeForUnload, { once: true });

  function post<T = unknown>(lane: BridgeLane, type: string, payload?: unknown, postOptions?: PostOptions): Promise<T> {
    assertRequestType(type);
    const ipc = host.ipc;
    if (!ipc?.postMessage) {
      return Promise.reject(new Error("Vesty IPC is unavailable"));
    }

    const outboundPayload = packetPayload(payload);
    const requestSeq = seq;
    const id = `js-${requestSeq}`;
    const packet: BridgePacket = {
      v: 1,
      session,
      seq: requestSeq,
      lane,
      kind: "request",
      type,
      id
    };
    if (outboundPayload !== undefined) packet.payload = outboundPayload;
    const message = JSON.stringify(packet);
    if (utf8ByteLength(message) > maxMessageBytesForLane(lane)) {
      return Promise.reject(backpressureError("bridge message too large"));
    }
    seq = seq >= MAX_BRIDGE_PACKET_SEQ ? 1 : seq + 1;

    return new Promise<T>((resolve, reject) => {
      const item: Pending = {
        resolve: resolve as (value: unknown) => void,
        reject
      };
      const requestTimeoutMs = Math.max(0, postOptions?.timeoutMs ?? timeoutMs);
      if (requestTimeoutMs > 0) {
        item.timer = setTimeout(() => {
          const timedOut = pending.get(id);
          if (!timedOut) return;
          pending.delete(id);
          timedOut.reject(requestTimeout(type));
        }, requestTimeoutMs);
      }
      pending.set(id, item);
      try {
        ipc.postMessage(message);
      } catch (error) {
        clearPending(pending, id, item);
        reject(error);
      }
    });
  }

  function reportListenerError(topic: string, error: unknown): void {
    try {
      console.error("Vesty bridge listener error", topic, error);
    } catch {
      // Ignore console failures; listener isolation must not throw from bridge delivery.
    }
  }

  function emit(topic: string, payload: unknown): void {
    const topicListeners = listeners.get(topic);
    if (!topicListeners) return;
    for (const listener of [...topicListeners]) {
      try {
        listener(payload);
      } catch (error) {
        reportListenerError(topic, error);
      }
    }
  }

  function packetMatchesSession(packet: BridgePacket): boolean {
    return ownDataValue(packet, "session") === session;
  }

  function isAsyncEventTopic(topic: string): boolean {
    return (
      topic.startsWith("meter.") ||
      topic === "param.changed" ||
      topic === "diagnostics.fault" ||
      topic === "log.rt"
    );
  }

  function hasAsyncEventSubscribers(): boolean {
    for (const [topic, topicListeners] of listeners) {
      if (isAsyncEventTopic(topic) && topicListeners.size > 0) return true;
    }
    return false;
  }

  function refreshEventPump(): void {
    if (hasAsyncEventSubscribers()) {
      if (!eventPump) {
        eventPump = setInterval(() => {
          if (eventFlushInFlight) return;
          eventFlushInFlight = true;
          void post("event", "event.flush", {}, { timeoutMs: EVENT_FLUSH_TIMEOUT_MS })
            .catch(() => undefined)
            .finally(() => {
              eventFlushInFlight = false;
            });
        }, 16);
      }
      return;
    }
    stopEventPump();
  }

  function readyHandshake<T = BridgeReadyPayload>(): Promise<T> {
    if (readyPayloadCache) return Promise.resolve(readyPayloadCache as T);
    if (!readyPromise) {
      readyPromise = (async () => {
        const payload = await post<BridgeReadyPayload>("command", "bridge.hello", helloPayload(host));
        assertCompatibleReadyPayload(payload);
        adoptEditorSession(payload);
        await post("command", "bridge.readyAck", readyAckPayload(payload));
        readyPayloadCache = payload;
        return payload;
      })().catch((error) => {
        readyPromise = undefined;
        throw error;
      });
    }
    return readyPromise as Promise<T>;
  }

  const internal: InternalBridge = {
    deliver(packet) {
      if (!validInboundPacket(packet)) return;
      if (!packetMatchesSession(packet)) return;
      const kind = ownDataValue(packet, "kind");
      const replyTo = ownDataValue(packet, "replyTo") as string | undefined;
      const error = ownDataValue(packet, "error") as BridgeError | undefined;
      const payload = ownDataValue(packet, "payload");
      const packetType = ownDataValue(packet, "type") as string;
      if (kind === "response" || kind === "error") {
        const key = replyTo;
        const item = key ? pending.get(key) : undefined;
        if (!item || !key) return;
        clearPending(pending, key, item);
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
      for (const packet of packets) internal.deliver(packet);
    }
  };

  host.__VESTY_INTERNAL__ = internal;

  return {
    ready<T = BridgeReadyPayload>() {
      return readyHandshake<T>();
    },
    getSnapshot: () => post("state", "snapshot.get", {}),
    getDiagnostics: () => post("command", "diagnostics.get", {}),
    request: (type, payload) => post("command", type, payload),
    subscribe(topic, handler) {
      assertSubscriptionTopic(topic);
      assertSubscriptionHandler(handler);
      let set = listeners.get(topic);
      const shouldSubscribe = !set || set.size === 0;
      if (!set) {
        set = new Set();
        listeners.set(topic, set);
      }
      set.add(handler as Listener);
      if (shouldSubscribe) {
        void post("command", "subscription.add", { topic }).catch(() => undefined);
      }
      refreshEventPump();
      return () => {
        const current = listeners.get(topic);
        if (!current) return;
        current.delete(handler as Listener);
        if (current.size === 0) {
          listeners.delete(topic);
          void post("command", "subscription.remove", { topic }).catch(() => undefined);
        }
        refreshEventPump();
      };
    },
    async setConfig(key, value, baseRevision) {
      assertConfigKey(key);
      assertJsonValuePresent(value, "config value");
      assertBaseRevision(baseRevision);
      return post("state", "state.setConfig", { baseRevision, key, value });
    },
    async setUiState(value, baseRevision) {
      assertJsonValuePresent(value, "ui state value");
      assertBaseRevision(baseRevision);
      return post("state", "state.setUiState", { baseRevision, value });
    },
    async beginParamEdit(id, gestureId) {
      assertParamId(id);
      assertOptionalGestureId(gestureId);
      return post("param", "param.begin", paramGesturePayload(id, gestureId));
    },
    async performParamEdit(id, normalized, gestureId) {
      assertParamId(id);
      assertNormalizedValue(normalized);
      assertOptionalGestureId(gestureId);
      return post("param", "param.perform", paramPerformPayload(id, normalized, gestureId));
    },
    async endParamEdit(id, gestureId) {
      assertParamId(id);
      assertOptionalGestureId(gestureId);
      return post("param", "param.end", paramGesturePayload(id, gestureId));
    },
    async setParam(id, normalized, gestureId) {
      assertParamId(id);
      assertNormalizedValue(normalized);
      assertOptionalGestureId(gestureId);
      await post("param", "param.begin", paramGesturePayload(id, gestureId));
      let failure: unknown;
      try {
        await post("param", "param.perform", paramPerformPayload(id, normalized, gestureId));
      } catch (error) {
        failure = error;
      }
      try {
        await post("param", "param.end", paramGesturePayload(id, gestureId));
      } catch (error) {
        failure ??= error;
      }
      if (failure) throw failure;
    },
    async formatParam(id, normalized) {
      assertParamId(id);
      assertNormalizedValue(normalized);
      return post("param", "param.format", { id, normalized });
    },
    async parseParam(id, text) {
      assertParamId(id);
      assertParamText(text);
      return post("param", "param.parse", { id, text });
    }
  };
}
