import assert from "node:assert/strict";
import { createBridge, createSnapshotStore } from "../dist/index.js";

function makeHost() {
  const listeners = new Map();
  const posted = [];
  const host = {
    location: { href: "vesty://assets/index.html" },
    ipc: {
      postMessage(message) {
        posted.push(JSON.parse(message));
      }
    },
    addEventListener(name, handler) {
      listeners.set(name, handler);
    }
  };
  return { host, listeners, posted };
}

function deliverResponse(host, packet, payload = {}) {
  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: packet.session,
    seq: packet.seq,
    lane: packet.lane,
    kind: "response",
    type: `${packet.type}.response`,
    replyTo: packet.id,
    payload
  });
}

function deliverError(host, packet, error) {
  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: packet.session,
    seq: packet.seq,
    lane: packet.lane,
    kind: "error",
    type: `${packet.type}.error`,
    replyTo: packet.id,
    error
  });
}

function readyPayload(overrides = {}) {
  return {
    protocolVersion: 1,
    instanceId: "instance",
    editorSessionId: "editor-test",
    devMode: true,
    pluginName: "Test",
    vendor: "Vesty",
    capabilities: {
      paramGestures: true,
      paramFormatParse: true,
      stateConfig: true,
      subscriptions: true,
      meterStream: true,
      reliableEvents: true,
      diagnostics: true
    },
    params: [
      {
        id: "gain",
        name: "Gain",
        kind: { float: { min: -60, max: 12 } },
        defaultNormalized: 0.5,
        unit: "dB",
        stepCount: null,
        flags: {
          automatable: true,
          bypass: false,
          readOnly: false,
          programChange: false
        },
        midiMappings: []
      }
    ],
    snapshot: {
      revision: 0,
      paramsRevision: 0,
      configRevision: 0,
      uiRevision: 0,
      config: {},
      uiState: {}
    },
    ...overrides
  };
}

{
  const { host, listeners, posted } = makeHost();
  const invalidOptions = [
    [null, /bridge options must be an object/],
    [[], /bridge options must be an object/],
    ["options", /bridge options must be an object/],
    [{ timeoutMs: "10" }, /timeoutMs must be a finite number/],
    [{ timeoutMs: Number.NaN }, /timeoutMs must be a finite number/],
    [{ timeoutMs: Infinity }, /timeoutMs must be a finite number/]
  ];

  for (const [options, message] of invalidOptions) {
    assert.throws(
      () => createBridge(host, "session", options),
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, message);
        return true;
      }
    );
  }
  assert.equal(posted.length, 0);
  assert.equal(listeners.size, 0);
}

{
  const invalidHosts = [
    [null, /bridge host must be an object/],
    [{}, /bridge host must provide addEventListener/],
    [{ addEventListener: null }, /bridge host must provide addEventListener/]
  ];

  for (const [host, message] of invalidHosts) {
    assert.throws(
      () => createBridge(host, "session", { timeoutMs: 0 }),
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, message);
        return true;
      }
    );
    assert.equal(host?.__VESTY_INTERNAL__, undefined);
  }
}

{
  const invalidSessions = [
    [null, /initialSession must be a string/],
    ["", /initialSession must not be empty/],
    ["x".repeat(129), /initialSession too long/],
    ["界".repeat(43), /initialSession too long/],
    ["session\u0007", /initialSession must not contain control characters/]
  ];

  for (const [session, message] of invalidSessions) {
    const { host, listeners, posted } = makeHost();
    assert.throws(
      () => createBridge(host, session, { timeoutMs: 0 }),
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, message);
        return true;
      }
    );
    assert.equal(posted.length, 0);
    assert.equal(listeners.size, 0);
    assert.equal(host.__VESTY_INTERNAL__, undefined);
  }
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const response = bridge.request("demo.request", { ok: true });

  assert.equal(posted.length, 1);
  assert.equal(posted[0].id, "js-1");
  assert.equal(posted[0].seq, 1);
  assert.equal(posted[0].session, "session");

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "session",
    seq: 1,
    lane: "command",
    kind: "response",
    type: "demo.request.response",
    replyTo: "js-1",
    payload: { ok: true }
  });
  assert.deepEqual(await response, { ok: true });
}

{
  const invalidRequestTypes = [
    [null, /request type must be a string/],
    ["", /request type must not be empty/],
    ["x".repeat(129), /request type too long/],
    ["界".repeat(43), /request type too long/],
    ["demo\u0007", /request type must not contain control characters/]
  ];

  for (const [type, message] of invalidRequestTypes) {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    assert.throws(
      () => bridge.request(type, { ok: true }),
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, message);
        return true;
      }
    );
    assert.equal(posted.length, 0);

    const response = bridge.request("demo.request", { ok: true });
    assert.equal(posted.length, 1);
    assert.equal(posted[0].type, "demo.request");
    deliverResponse(host, posted[0], { ok: true });
    assert.deepEqual(await response, { ok: true });
  }
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });

  await assert.rejects(
    bridge.request("demo.request", { blob: "界".repeat(22 * 1024) }),
    (error) => {
      assert.equal(error.code, "backpressure");
      assert.equal(error.retryable, true);
      assert.match(error.message, /bridge message too large/);
      return true;
    }
  );
  assert.equal(posted.length, 0);

  const response = bridge.request("demo.request", { ok: true });
  assert.equal(posted.length, 1);
  assert.equal(posted[0].id, "js-1");
  assert.equal(posted[0].seq, 1);
  deliverResponse(host, posted[0], { ok: true });
  assert.deepEqual(await response, { ok: true });
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const value = "界".repeat(15 * 1024);

  const response = bridge.setConfig("large", value, 0);
  assert.equal(posted.length, 1);
  assert.equal(posted[0].lane, "state");
  assert.equal(posted[0].type, "state.setConfig");
  deliverResponse(host, posted[0], { revision: 1, config: { large: value } });
  assert.deepEqual(await response, { revision: 1, config: { large: value } });
}

{
  const { host } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const response = bridge.request("demo.request", { ok: true });
  let settled = false;
  response.then(() => {
    settled = true;
  });

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "stale-session",
    seq: 1,
    lane: "command",
    kind: "response",
    type: "demo.request.response",
    replyTo: "js-1",
    payload: { ok: false }
  });
  await Promise.resolve();
  assert.equal(settled, false);

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "session",
    seq: 2,
    lane: "command",
    kind: "response",
    type: "demo.request.response",
    replyTo: "js-1",
    payload: { ok: true }
  });
  assert.deepEqual(await response, { ok: true });
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const events = [];
  bridge.subscribe("state.changed", (payload) => events.push(payload));
  assert.equal(posted.length, 1);

  const circularEventPayload = { revision: 13 };
  circularEventPayload.self = circularEventPayload;
  const getterEventPacket = {
    v: 1,
    session: "session",
    seq: 16,
    lane: "event",
    kind: "event",
    type: "state.changed"
  };
  Object.defineProperty(getterEventPacket, "payload", {
    enumerable: true,
    get() {
      throw new Error("payload getter should not run");
    }
  });

  assert.doesNotThrow(() => {
    host.__VESTY_INTERNAL__.deliver(null);
    host.__VESTY_INTERNAL__.deliver({});
    host.__VESTY_INTERNAL__.deliver({
      v: 2,
      session: "session",
      seq: 1,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: { revision: 1 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 2,
      lane: "event",
      kind: "request",
      type: "state.changed",
      payload: { revision: 2 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 3,
      lane: "event",
      kind: "event",
      type: "state.changed\u0007",
      payload: { revision: 3 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 4,
      lane: "event",
      kind: "event",
      type: "state.changed",
      id: "server-event",
      payload: { revision: 4 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 5,
      lane: "event",
      kind: "event",
      type: "state.changed",
      replyTo: "js-1",
      payload: { revision: 5 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 6,
      lane: "event",
      kind: "event",
      type: "state.changed",
      error: { code: "internal_error", message: "event error", retryable: false },
      payload: { revision: 6 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 7,
      lane: "event",
      kind: "ack",
      type: "state.changed",
      payload: { revision: 7 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 8,
      lane: "event",
      kind: "event",
      type: "state.changed",
      flags: ["bad\u0007flag"],
      payload: { revision: 8 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 9,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: undefined
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 10,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: { revision: undefined }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 11,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: { revision: () => 11 }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 12,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: { revision: Symbol("bad") }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 13,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: { revision: Number.NaN }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 14,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: circularEventPayload
    });
    host.__VESTY_INTERNAL__.deliver(getterEventPacket);
    host.__VESTY_INTERNAL__.deliverBatch("not an array");
    host.__VESTY_INTERNAL__.deliverBatch(
      Array.from({ length: 4097 }, (_, index) => ({
        v: 1,
        session: "session",
        seq: 1000 + index,
        lane: "event",
        kind: "event",
        type: "state.changed",
        payload: { revision: 1000 + index }
      }))
    );
    host.__VESTY_INTERNAL__.deliverBatch([
      {
        v: 1,
        session: "session",
        seq: -1,
        lane: "event",
        kind: "event",
        type: "state.changed",
        payload: { revision: 4 }
      },
      {
        v: 1,
        session: "session",
        seq: Number.MAX_SAFE_INTEGER + 1,
        lane: "event",
        kind: "event",
        type: "state.changed",
        payload: { revision: 8 }
      },
      {
        v: 1,
        session: "session",
        seq: 15,
        lane: "event",
        kind: "event",
        type: "state.changed",
        flags: ["latest"],
        payload: { revision: 15 }
      }
    ]);
  });

  assert.deepEqual(events, [{ revision: 15 }]);
}

{
  const { host } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const events = [];
  bridge.subscribe("state.changed", (event) => events.push(event));
  host.__VESTY_INTERNAL__.deliverBatch(
    Array.from({ length: 4096 }, (_, index) => ({
      v: 1,
      session: "session",
      seq: index + 1,
      lane: "event",
      kind: "event",
      type: "state.changed",
      payload: { revision: index + 1 }
    }))
  );

  assert.equal(events.length, 4096);
  assert.deepEqual(events[0], { revision: 1 });
  assert.deepEqual(events[4095], { revision: 4096 });
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const response = bridge.request("demo.request", { ok: true });
  let settled = false;
  response.then(
    () => {
      settled = true;
    },
    () => {
      settled = true;
    }
  );

  assert.doesNotThrow(() => {
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 1,
      lane: "command",
      kind: "response",
      type: "demo.request.response",
      payload: { ok: false }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 2,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: { code: "internal_error", message: 42, retryable: true }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 3,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: { code: "not_a_bridge_error", message: "unknown code", retryable: true }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 4,
      lane: "command",
      kind: "response",
      type: "demo.request.response",
      id: "server-response",
      replyTo: "js-1",
      payload: { ok: false }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 5,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: { code: "internal_error", message: "x".repeat(2049), retryable: true }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 6,
      lane: "command",
      kind: "response",
      type: "demo.request.response",
      replyTo: "js-1",
      error: { code: "internal_error", message: "response polluted", retryable: true },
      payload: { ok: false }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 7,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      payload: { ok: false },
      error: { code: "internal_error", message: "error polluted", retryable: true }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 8,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: {
        code: "internal_error",
        message: "undefined details",
        details: undefined,
        retryable: true
      }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 9,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: {
        code: "internal_error",
        message: "function details",
        details: { value: () => undefined },
        retryable: true
      }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 10,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: {
        code: "internal_error",
        message: "symbol details",
        details: { value: Symbol("bad") },
        retryable: true
      }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 11,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: {
        code: "internal_error",
        message: "non-finite details",
        details: { value: Number.POSITIVE_INFINITY },
        retryable: true
      }
    });
    const circularDetails = { value: true };
    circularDetails.self = circularDetails;
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 12,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: {
        code: "internal_error",
        message: "circular details",
        details: circularDetails,
        retryable: true
      }
    });
    host.__VESTY_INTERNAL__.deliver({
      v: 1,
      session: "session",
      seq: 13,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1",
      error: {
        code: "internal_error",
        message: "extra error key",
        details: { ok: true },
        retryable: true,
        [Symbol("bad")]: true
      }
    });
    const getterErrorPacket = {
      v: 1,
      session: "session",
      seq: 14,
      lane: "command",
      kind: "error",
      type: "demo.request.error",
      replyTo: "js-1"
    };
    Object.defineProperty(getterErrorPacket, "error", {
      enumerable: true,
      get() {
        throw new Error("error getter should not run");
      }
    });
    host.__VESTY_INTERNAL__.deliver(getterErrorPacket);
  });
  await Promise.resolve();
  assert.equal(settled, false);

  deliverResponse(host, posted[0], { ok: true });
  assert.deepEqual(await response, { ok: true });
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const ready = bridge.ready();

  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "bridge.hello");
  assert.equal(posted[0].id, "js-1");
  assert.equal(posted[0].seq, 1);
  assert.equal(posted[0].session, "pending");
  assert.deepEqual(posted[0].payload.supportedProtocolVersions, [1]);
  assert.equal(posted[0].payload.pageUrl, "vesty://assets/index.html");

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "pending",
    seq: 1,
    lane: "command",
    kind: "response",
    type: "bridge.hello.response",
    replyTo: "js-1",
    payload: readyPayload({ editorSessionId: "editor-test" })
  });
  await Promise.resolve();

  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "bridge.readyAck");
  assert.equal(posted[1].id, "js-2");
  assert.equal(posted[1].seq, 2);
  assert.equal(posted[1].session, "editor-test");

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "editor-test",
    seq: 2,
    lane: "command",
    kind: "response",
    type: "bridge.readyAck.response",
    replyTo: "js-2",
    payload: { ready: true }
  });

  assert.deepEqual(await ready, readyPayload({ editorSessionId: "editor-test" }));
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const ready = bridge.ready();
  const payload = readyPayload({
    editorSessionId: "editor-extension",
    hostExtensions: { canPinEditor: true },
    capabilities: {
      ...readyPayload().capabilities,
      vendorDiagnostics: true
    },
    snapshot: {
      ...readyPayload().snapshot,
      vendorState: { theme: "dark" }
    }
  });
  payload.params[0].flags.vendorVisible = true;
  payload.params[0].kind.float.displayHint = "decibel";
  payload.params[0].midiMappings[0] = { controller: 7, channel: null, source: "host" };

  deliverResponse(host, posted[0], payload);
  await Promise.resolve();

  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "bridge.readyAck");
  assert.equal(posted[1].session, "editor-extension");
  deliverResponse(host, posted[1], { ready: true });

  const resolved = await ready;
  assert.equal(resolved.hostExtensions.canPinEditor, true);
  assert.equal(resolved.capabilities.vendorDiagnostics, true);
  assert.equal(resolved.snapshot.vendorState.theme, "dark");
  assert.equal(resolved.params[0].flags.vendorVisible, true);
  assert.equal(resolved.params[0].kind.float.displayHint, "decibel");
  assert.equal(resolved.params[0].midiMappings[0].source, "host");
  assert.strictEqual(await bridge.ready(), resolved);
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const ready = bridge.ready();
  let settled = false;
  ready.then(
    () => {
      settled = true;
    },
    () => {
      settled = true;
    }
  );

  const getterReadyResponse = {
    v: 1,
    session: "pending",
    seq: 1,
    lane: "command",
    kind: "response",
    type: "bridge.hello.response",
    replyTo: "js-1"
  };
  Object.defineProperty(getterReadyResponse, "payload", {
    enumerable: true,
    get() {
      throw new Error("ready payload getter should not run");
    }
  });

  assert.doesNotThrow(() => host.__VESTY_INTERNAL__.deliver(getterReadyResponse));
  await Promise.resolve();
  assert.equal(settled, false);
  assert.equal(posted.length, 1);

  deliverResponse(host, posted[0], readyPayload({ editorSessionId: "editor-after-getter-payload" }));
  await Promise.resolve();
  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "bridge.readyAck");
  assert.equal(posted[1].session, "editor-after-getter-payload");
  deliverResponse(host, posted[1], { ready: true });
  assert.equal((await ready).editorSessionId, "editor-after-getter-payload");
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const ready = bridge.ready();

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "pending",
    seq: 1,
    lane: "command",
    kind: "response",
    type: "bridge.hello.response",
    replyTo: "js-1",
    payload: readyPayload({ protocolVersion: 2, editorSessionId: "editor-v2" })
  });

  await assert.rejects(ready, (error) => {
    assert.equal(error.code, "unsupported_version");
    assert.equal(error.retryable, false);
    assert.match(error.message, /protocol version: 2/);
    return true;
  });
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "bridge.hello");

  const retry = bridge.ready();
  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "bridge.hello");
  assert.equal(posted[1].session, "pending");

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "pending",
    seq: 2,
    lane: "command",
    kind: "response",
    type: "bridge.hello.response",
    replyTo: "js-2",
    payload: readyPayload({ editorSessionId: "editor-retry" })
  });
  await Promise.resolve();

  assert.equal(posted.length, 3);
  assert.equal(posted[2].type, "bridge.readyAck");
  assert.equal(posted[2].session, "editor-retry");
  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "editor-retry",
    seq: 3,
    lane: "command",
    kind: "response",
    type: "bridge.readyAck.response",
    replyTo: "js-3",
    payload: { ready: true }
  });
  assert.equal((await retry).editorSessionId, "editor-retry");
}

{
  const invalidEditorSessions = [
    [42, /editorSessionId must be a string/],
    ["", /editorSessionId must not be empty/],
    ["x".repeat(129), /editorSessionId too long/],
    ["界".repeat(43), /editorSessionId too long/],
    ["editor\u0007", /editorSessionId must not contain control characters/]
  ];

  for (const [editorSessionId, message] of invalidEditorSessions) {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "pending", { timeoutMs: 0 });
    const ready = bridge.ready();

    deliverResponse(host, posted[0], readyPayload({ editorSessionId }));

    await assert.rejects(ready, (error) => {
      assert.equal(error.code, "validation_error");
      assert.equal(error.retryable, false);
      assert.match(error.message, message);
      return true;
    });
    assert.equal(posted.length, 1);
    assert.equal(posted[0].type, "bridge.hello");

    const retry = bridge.ready();
    assert.equal(posted.length, 2);
    assert.equal(posted[1].type, "bridge.hello");
    assert.equal(posted[1].session, "pending");

    deliverResponse(host, posted[1], readyPayload({ editorSessionId: "editor-after-invalid-session" }));
    await Promise.resolve();
    assert.equal(posted.length, 3);
    assert.equal(posted[2].type, "bridge.readyAck");
    assert.equal(posted[2].session, "editor-after-invalid-session");
    deliverResponse(host, posted[2], { ready: true });

    assert.equal((await retry).editorSessionId, "editor-after-invalid-session");
  }
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const ready = bridge.ready();
  const invalid = readyPayload({ editorSessionId: "editor-invalid-schema" });
  invalid.params = [
    invalid.params[0],
    {
      ...invalid.params[0],
      name: "Duplicate Gain"
    }
  ];

  deliverResponse(host, posted[0], invalid);

  await assert.rejects(ready, (error) => {
    assert.equal(error.code, "validation_error");
    assert.equal(error.retryable, false);
    assert.match(error.message, /duplicate parameter id 'gain'/);
    return true;
  });
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "bridge.hello");

  const retry = bridge.ready();
  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "bridge.hello");
  assert.equal(posted[1].session, "pending");

  deliverResponse(host, posted[1], readyPayload({ editorSessionId: "editor-after-invalid-schema" }));
  await Promise.resolve();
  assert.equal(posted.length, 3);
  assert.equal(posted[2].type, "bridge.readyAck");
  assert.equal(posted[2].session, "editor-after-invalid-schema");
  deliverResponse(host, posted[2], { ready: true });

  assert.equal((await retry).editorSessionId, "editor-after-invalid-schema");
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const ready = bridge.ready();
  const invalid = readyPayload({ editorSessionId: "editor-invalid-midi-mapping" });
  invalid.params[0].midiMappings = [
    { controller: 7, channel: null },
    { controller: 7, channel: null }
  ];

  deliverResponse(host, posted[0], invalid);

  await assert.rejects(ready, (error) => {
    assert.equal(error.code, "validation_error");
    assert.equal(error.retryable, false);
    assert.match(error.message, /duplicate MIDI mapping 7:\*/);
    return true;
  });
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "bridge.hello");
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const ready = bridge.ready();
  const invalid = readyPayload({ editorSessionId: "editor-invalid-program-change-flag" });
  invalid.params[0].flags.programChange = "yes";

  deliverResponse(host, posted[0], invalid);

  await assert.rejects(ready, (error) => {
    assert.equal(error.code, "validation_error");
    assert.equal(error.retryable, false);
    assert.match(error.message, /flags\.programChange must be a boolean/);
    return true;
  });
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "bridge.hello");
}

{
  const realSetTimeout = globalThis.setTimeout;
  const realClearTimeout = globalThis.clearTimeout;
  const timers = [];
  const cleared = [];
  globalThis.setTimeout = (callback, ms) => {
    const token = { callback, ms };
    timers.push(token);
    return token;
  };
  globalThis.clearTimeout = (token) => {
    cleared.push(token);
  };

  try {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "pending", { timeoutMs: 10 });
    const first = bridge.ready();

    assert.equal(posted.length, 1);
    assert.equal(posted[0].type, "bridge.hello");
    assert.equal(timers.length, 1);
    assert.equal(timers[0].ms, 10);

    timers[0].callback();
    await assert.rejects(first, (error) => {
      assert.equal(error.code, "timeout");
      assert.equal(error.retryable, true);
      return true;
    });

    const retry = bridge.ready();
    assert.equal(posted.length, 2);
    assert.equal(posted[1].type, "bridge.hello");
    assert.equal(posted[1].session, "pending");

    deliverResponse(host, posted[1], readyPayload({ editorSessionId: "editor-timeout-retry" }));
    await Promise.resolve();

    assert.equal(posted.length, 3);
    assert.equal(posted[2].type, "bridge.readyAck");
    assert.equal(posted[2].session, "editor-timeout-retry");
    deliverResponse(host, posted[2], { ready: true });

    assert.equal((await retry).editorSessionId, "editor-timeout-retry");
    assert.ok(cleared.includes(timers[1]));
    assert.ok(cleared.includes(timers[2]));
  } finally {
    globalThis.setTimeout = realSetTimeout;
    globalThis.clearTimeout = realClearTimeout;
  }
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const events = [];
  const unsubscribeA = bridge.subscribe("state.changed", (payload) => events.push(["a", payload]));
  const unsubscribeB = bridge.subscribe("state.changed", (payload) => events.push(["b", payload]));

  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "subscription.add");
  assert.deepEqual(posted[0].payload, { topic: "state.changed" });

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "stale-session",
    seq: 1,
    lane: "event",
    kind: "event",
    type: "state.changed",
    payload: { revision: 1 }
  });
  assert.deepEqual(events, []);

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "session",
    seq: 2,
    lane: "event",
    kind: "event",
    type: "state.changed",
    payload: { revision: 2 }
  });
  assert.deepEqual(events, [
    ["a", { revision: 2 }],
    ["b", { revision: 2 }]
  ]);

  unsubscribeA();
  assert.equal(posted.length, 1);

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "session",
    seq: 3,
    lane: "event",
    kind: "event",
    type: "state.changed",
    payload: { revision: 3 }
  });
  assert.deepEqual(events, [
    ["a", { revision: 2 }],
    ["b", { revision: 2 }],
    ["b", { revision: 3 }]
  ]);

  unsubscribeB();
  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "subscription.remove");
  assert.deepEqual(posted[1].payload, { topic: "state.changed" });
}

{
  const realConsoleError = console.error;
  const listenerErrors = [];
  console.error = (...args) => listenerErrors.push(args);

  try {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    const events = [];
    bridge.subscribe("state.changed", (payload) => {
      events.push(["throwing", payload]);
      throw new Error("listener failed");
    });
    bridge.subscribe("state.changed", (payload) => events.push(["state", payload]));
    bridge.subscribe("custom.event", (payload) => events.push(["custom", payload]));

    assert.equal(posted.length, 2);
    assert.equal(posted[0].type, "subscription.add");
    assert.deepEqual(posted[0].payload, { topic: "state.changed" });
    assert.equal(posted[1].type, "subscription.add");
    assert.deepEqual(posted[1].payload, { topic: "custom.event" });

    assert.doesNotThrow(() => {
      host.__VESTY_INTERNAL__.deliverBatch([
        {
          v: 1,
          session: "session",
          seq: 2,
          lane: "event",
          kind: "event",
          type: "state.changed",
          payload: { revision: 2 }
        },
        {
          v: 1,
          session: "session",
          seq: 3,
          lane: "event",
          kind: "event",
          type: "custom.event",
          payload: { id: "gain", normalized: 0.75 }
        }
      ]);
    });

    assert.deepEqual(events, [
      ["throwing", { revision: 2 }],
      ["state", { revision: 2 }],
      ["custom", { id: "gain", normalized: 0.75 }]
    ]);
    assert.equal(listenerErrors.length, 1);
    assert.equal(listenerErrors[0][0], "Vesty bridge listener error");
    assert.equal(listenerErrors[0][1], "state.changed");
    assert.match(listenerErrors[0][2].message, /listener failed/);
  } finally {
    console.error = realConsoleError;
  }
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const invalidSubscriptions = [
    [() => bridge.subscribe(null, () => undefined), /subscription topic must be a string/],
    [() => bridge.subscribe("", () => undefined), /subscription topic must not be empty/],
    [
      () => bridge.subscribe("meter.main\u0007", () => undefined),
      /subscription topic must not contain control characters/
    ],
    [() => bridge.subscribe("x".repeat(129), () => undefined), /subscription topic too long/],
    [() => bridge.subscribe("界".repeat(43), () => undefined), /subscription topic too long/],
    [() => bridge.subscribe("state.changed", null), /subscription handler must be a function/],
    [() => bridge.subscribe("state.changed", {}), /subscription handler must be a function/]
  ];

  for (const [call, message] of invalidSubscriptions) {
    assert.throws(
      call,
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, message);
        return true;
      }
    );
  }

  assert.equal(posted.length, 0);
}

{
  const realSetInterval = globalThis.setInterval;
  const realClearInterval = globalThis.clearInterval;
  const intervals = [];
  const cleared = [];
  globalThis.setInterval = (callback, ms) => {
    const token = { callback, ms };
    intervals.push(token);
    return token;
  };
  globalThis.clearInterval = (token) => {
    cleared.push(token);
  };

  try {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    const unsubscribeA = bridge.subscribe("meter.main", () => undefined);
    const unsubscribeB = bridge.subscribe("meter.main", () => undefined);

    assert.equal(posted.length, 1);
    assert.equal(posted[0].type, "subscription.add");
    assert.equal(intervals.length, 1);
    assert.equal(intervals[0].ms, 16);

    intervals[0].callback();
    assert.equal(posted.length, 2);
    assert.equal(posted[1].type, "event.flush");
    assert.equal(posted[1].lane, "event");
    deliverResponse(host, posted[1], { pendingMeterTopics: 0, pendingParamGestures: 0 });

    unsubscribeA();
    assert.equal(cleared.length, 0);

    unsubscribeB();
    assert.equal(posted.length, 3);
    assert.equal(posted[2].type, "subscription.remove");
    assert.equal(cleared.length, 1);
    assert.equal(cleared[0], intervals[0]);
  } finally {
    globalThis.setInterval = realSetInterval;
    globalThis.clearInterval = realClearInterval;
  }
}

{
  const realSetInterval = globalThis.setInterval;
  const realClearInterval = globalThis.clearInterval;
  const intervals = [];
  const cleared = [];
  globalThis.setInterval = (callback, ms) => {
    const token = { callback, ms };
    intervals.push(token);
    return token;
  };
  globalThis.clearInterval = (token) => {
    cleared.push(token);
  };

  try {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    const unsubscribe = bridge.subscribe("meter.main", () => undefined);

    intervals[0].callback();
    assert.equal(posted.length, 2);
    assert.equal(posted[1].type, "event.flush");

    unsubscribe();
    assert.equal(cleared.length, 1);

    bridge.subscribe("meter.main", () => undefined);
    assert.equal(intervals.length, 2);
    assert.equal(posted.length, 4);
    assert.equal(posted[3].type, "subscription.add");
    intervals[1].callback();
    assert.equal(posted.length, 4);
    assert.equal(posted[2].type, "subscription.remove");

    deliverResponse(host, posted[1], { pendingMeterTopics: 0, pendingParamGestures: 0 });
    await Promise.resolve();
    await Promise.resolve();

    intervals[1].callback();
    assert.equal(posted.length, 5);
    assert.equal(posted[4].type, "event.flush");
  } finally {
    globalThis.setInterval = realSetInterval;
    globalThis.clearInterval = realClearInterval;
  }
}

{
  const realSetInterval = globalThis.setInterval;
  const realClearInterval = globalThis.clearInterval;
  const intervals = [];
  globalThis.setInterval = (callback, ms) => {
    const token = { callback, ms };
    intervals.push(token);
    return token;
  };
  globalThis.clearInterval = () => undefined;

  try {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    bridge.subscribe("meter.main", () => undefined);

    assert.equal(intervals.length, 1);
    intervals[0].callback();
    assert.equal(posted.length, 2);
    assert.equal(posted[1].type, "event.flush");

    intervals[0].callback();
    intervals[0].callback();
    assert.equal(posted.length, 2);

    deliverResponse(host, posted[1], { pendingMeterTopics: 0, pendingParamGestures: 0 });
    await Promise.resolve();
    await Promise.resolve();

    intervals[0].callback();
    assert.equal(posted.length, 3);
    assert.equal(posted[2].type, "event.flush");
  } finally {
    globalThis.setInterval = realSetInterval;
    globalThis.clearInterval = realClearInterval;
  }
}

{
  const realSetInterval = globalThis.setInterval;
  const realClearInterval = globalThis.clearInterval;
  const realSetTimeout = globalThis.setTimeout;
  const realClearTimeout = globalThis.clearTimeout;
  const intervals = [];
  const timeouts = [];
  const clearedTimeouts = [];
  globalThis.setInterval = (callback, ms) => {
    const token = { callback, ms };
    intervals.push(token);
    return token;
  };
  globalThis.clearInterval = () => undefined;
  globalThis.setTimeout = (callback, ms) => {
    const token = { callback, ms };
    timeouts.push(token);
    return token;
  };
  globalThis.clearTimeout = (token) => {
    clearedTimeouts.push(token);
  };

  try {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    const normalRequest = bridge.request("demo.request", { ok: true });
    bridge.subscribe("meter.main", () => undefined);

    assert.equal(timeouts.length, 0);
    deliverResponse(host, posted[0], { ok: true });
    await normalRequest;

    assert.equal(intervals.length, 1);
    intervals[0].callback();
    assert.equal(posted.length, 3);
    assert.equal(posted[2].type, "event.flush");
    assert.equal(timeouts.length, 1);
    assert.equal(timeouts[0].ms, 1000);

    intervals[0].callback();
    assert.equal(posted.length, 3);

    timeouts[0].callback();
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(clearedTimeouts.length, 0);

    intervals[0].callback();
    assert.equal(posted.length, 4);
    assert.equal(posted[3].type, "event.flush");
    assert.equal(timeouts.length, 2);
    assert.equal(timeouts[1].ms, 1000);

    deliverResponse(host, posted[2], { stale: true });
    deliverResponse(host, posted[3], { pendingMeterTopics: 0, pendingParamGestures: 0 });
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(clearedTimeouts.length, 1);
    assert.equal(clearedTimeouts[0], timeouts[1]);
  } finally {
    globalThis.setInterval = realSetInterval;
    globalThis.clearInterval = realClearInterval;
    globalThis.setTimeout = realSetTimeout;
    globalThis.clearTimeout = realClearTimeout;
  }
}

{
  const realSetInterval = globalThis.setInterval;
  const realClearInterval = globalThis.clearInterval;
  const intervals = [];
  const cleared = [];
  globalThis.setInterval = (callback, ms) => {
    const token = { callback, ms };
    intervals.push(token);
    return token;
  };
  globalThis.clearInterval = (token) => {
    cleared.push(token);
  };

  try {
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    const unsubscribe = bridge.subscribe("param.changed", () => undefined);

    assert.equal(posted.length, 1);
    assert.equal(posted[0].type, "subscription.add");
    assert.equal(intervals.length, 1);
    assert.equal(intervals[0].ms, 16);

    intervals[0].callback();
    assert.equal(posted.length, 2);
    assert.equal(posted[1].type, "event.flush");
    assert.equal(posted[1].lane, "event");
    deliverResponse(host, posted[1], { pendingMeterTopics: 0, pendingParamGestures: 0 });

    unsubscribe();
    assert.equal(posted.length, 3);
    assert.equal(posted[2].type, "subscription.remove");
    assert.equal(cleared.length, 1);
    assert.equal(cleared[0], intervals[0]);
  } finally {
    globalThis.setInterval = realSetInterval;
    globalThis.clearInterval = realClearInterval;
  }
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const setParam = bridge.setParam("gain", 0.75);

  assert.equal(posted.length, 1);
  assert.equal(posted[0].lane, "param");
  assert.equal(posted[0].type, "param.begin");
  assert.deepEqual(posted[0].payload, { id: "gain" });
  deliverResponse(host, posted[0], { id: "gain", phase: "begin" });
  await Promise.resolve();

  assert.equal(posted.length, 2);
  assert.equal(posted[1].lane, "param");
  assert.equal(posted[1].type, "param.perform");
  assert.deepEqual(posted[1].payload, { id: "gain", normalized: 0.75 });
  deliverResponse(host, posted[1], { id: "gain", normalized: 0.75 });
  await Promise.resolve();

  assert.equal(posted.length, 3);
  assert.equal(posted[2].lane, "param");
  assert.equal(posted[2].type, "param.end");
  assert.deepEqual(posted[2].payload, { id: "gain" });
  deliverResponse(host, posted[2], { id: "gain", phase: "end" });

  await setParam;
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const setParam = bridge.setParam("gain", 0.75, "drag-1");

  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "param.begin");
  assert.deepEqual(posted[0].payload, { id: "gain", gestureId: "drag-1" });
  deliverResponse(host, posted[0], { id: "gain", phase: "begin" });
  await Promise.resolve();

  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "param.perform");
  assert.deepEqual(posted[1].payload, { id: "gain", normalized: 0.75, gestureId: "drag-1" });
  deliverResponse(host, posted[1], { id: "gain", normalized: 0.75 });
  await Promise.resolve();

  assert.equal(posted.length, 3);
  assert.equal(posted[2].type, "param.end");
  assert.deepEqual(posted[2].payload, { id: "gain", gestureId: "drag-1" });
  deliverResponse(host, posted[2], { id: "gain", phase: "end" });

  await setParam;
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const begin = bridge.beginParamEdit("gain", "drag-1");

  assert.equal(posted.length, 1);
  assert.equal(posted[0].lane, "param");
  assert.equal(posted[0].type, "param.begin");
  assert.deepEqual(posted[0].payload, { id: "gain", gestureId: "drag-1" });
  deliverResponse(host, posted[0], { id: "gain", phase: "begin" });

  await begin;
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const invalidAsyncCalls = [
    [() => bridge.setConfig("", "dark", 0), /config key must not be empty/],
    [() => bridge.setConfig("x".repeat(129), "dark", 0), /config key too long/],
    [() => bridge.setConfig("bad\nkey", "dark", 0), /config key must not contain control characters/],
    [() => bridge.setConfig("theme", undefined, 0), /config value must not be undefined/],
    [() => bridge.setConfig("theme", "dark", -1), /baseRevision must be a non-negative integer/],
    [() => bridge.setUiState(undefined, 0), /ui state value must not be undefined/],
    [() => bridge.setUiState({}, Number.NaN), /baseRevision must be a non-negative integer/],
    [() => bridge.beginParamEdit(""), /parameter id must not be empty/],
    [() => bridge.beginParamEdit("gain", ""), /gestureId must not be empty/],
    [() => bridge.beginParamEdit("gain", "x".repeat(129)), /gestureId too long/],
    [() => bridge.beginParamEdit("gain", "drag\u0007"), /gestureId must not contain control characters/],
    [() => bridge.performParamEdit("gain", Number.POSITIVE_INFINITY), /normalized value must be a finite number/],
    [() => bridge.performParamEdit("gain", 0.5, ""), /gestureId must not be empty/],
    [() => bridge.performParamEdit("gain", 0.5, "x".repeat(129)), /gestureId too long/],
    [() => bridge.endParamEdit("gain", "drag\u0007"), /gestureId must not contain control characters/],
    [() => bridge.setParam("gain", 0.5, ""), /gestureId must not be empty/],
    [() => bridge.setParam("gain", 0.5, "x".repeat(129)), /gestureId too long/],
    [() => bridge.setParam("gain", 0.5, "drag\u0007"), /gestureId must not contain control characters/],
    [() => bridge.formatParam("gain", Number.NaN), /normalized value must be a finite number/],
    [() => bridge.parseParam("gain", 7), /parameter text must be a string/]
  ];

  for (const [call, message] of invalidAsyncCalls) {
    await assert.rejects(call(), (error) => {
      assert.equal(error.code, "validation_error");
      assert.equal(error.retryable, false);
      assert.match(error.message, message);
      return true;
    });
  }

  await assert.rejects(bridge.setParam("gain", Number.NaN), (error) => {
    assert.equal(error.code, "validation_error");
    assert.equal(error.retryable, false);
    assert.match(error.message, /normalized value must be a finite number/);
    return true;
  });

  assert.equal(posted.length, 0);
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const cyclicPayload = { ok: true };
  cyclicPayload.self = cyclicPayload;
  const invalidPayloads = [
    { value: undefined },
    { value: () => undefined },
    { value: Symbol("bad") },
    { value: Number.NaN },
    { value: Number.POSITIVE_INFINITY },
    cyclicPayload
  ];

  for (const payload of invalidPayloads) {
    assert.throws(
      () => bridge.request("demo.request", payload),
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, /request payload must be JSON-compatible/);
        return true;
      }
    );
  }

  assert.equal(posted.length, 0);
  const response = bridge.request("demo.request", { ok: true });
  assert.equal(posted.length, 1);
  assert.equal(posted[0].id, "js-1");
  assert.equal(posted[0].seq, 1);
  deliverResponse(host, posted[0], { ok: true });
  assert.deepEqual(await response, { ok: true });
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const setParam = bridge.setParam("gain", 0.75);
  deliverError(host, posted[0], {
    code: "validation_error",
    message: "unknown parameter id",
    retryable: false
  });

  await assert.rejects(setParam, (error) => {
    assert.equal(error.code, "validation_error");
    assert.equal(error.retryable, false);
    return true;
  });
  assert.equal(posted.length, 1);
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const setParam = bridge.setParam("gain", 0.75);

  deliverResponse(host, posted[0], { id: "gain", phase: "begin" });
  await Promise.resolve();
  deliverError(host, posted[1], {
    code: "backpressure",
    message: "pending parameter gesture queue full",
    retryable: true
  });
  await Promise.resolve();

  assert.equal(posted.length, 3);
  assert.equal(posted[2].type, "param.end");
  deliverResponse(host, posted[2], { id: "gain", phase: "end" });

  await assert.rejects(setParam, (error) => {
    assert.equal(error.code, "backpressure");
    assert.equal(error.retryable, true);
    return true;
  });
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "pending", { timeoutMs: 0 });
  const first = bridge.ready();
  const second = bridge.ready();

  assert.equal(first, second);
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "bridge.hello");

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "pending",
    seq: 1,
    lane: "command",
    kind: "response",
    type: "bridge.hello.response",
    replyTo: "js-1",
    payload: readyPayload({ editorSessionId: "editor-test" })
  });
  await Promise.resolve();

  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "bridge.readyAck");
  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "editor-test",
    seq: 2,
    lane: "command",
    kind: "response",
    type: "bridge.readyAck.response",
    replyTo: "js-2",
    payload: { ready: true }
  });

  const payload = await first;
  assert.equal(payload.editorSessionId, "editor-test");
  const third = await bridge.ready();
  assert.equal(third, payload);
  assert.equal(posted.length, 2);
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const formatted = bridge.formatParam("gain", 0.5);

  assert.equal(posted[0].lane, "param");
  assert.equal(posted[0].type, "param.format");
  assert.deepEqual(posted[0].payload, { id: "gain", normalized: 0.5 });
  deliverResponse(host, posted[0], "0.50 dB");
  assert.equal(await formatted, "0.50 dB");

  const parsed = bridge.parseParam("gain", "0.50 dB");
  assert.equal(posted[1].lane, "param");
  assert.equal(posted[1].type, "param.parse");
  assert.deepEqual(posted[1].payload, { id: "gain", text: "0.50 dB" });
  deliverResponse(host, posted[1], 0.5);
  assert.equal(await parsed, 0.5);
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const saved = bridge.setUiState({ panel: "advanced" }, 4);

  assert.equal(posted[0].lane, "state");
  assert.equal(posted[0].type, "state.setUiState");
  assert.deepEqual(posted[0].payload, {
    baseRevision: 4,
    value: { panel: "advanced" }
  });
  deliverResponse(host, posted[0], {
    revision: 8,
    paramsRevision: 1,
    configRevision: 2,
    uiRevision: 5,
    config: {},
    uiState: { panel: "advanced" }
  });
  assert.equal((await saved).uiRevision, 5);
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });

  const snapshot = bridge.getSnapshot();
  const diagnostics = bridge.getDiagnostics();

  assert.equal(posted.length, 2);
  assert.equal(posted[0].type, "snapshot.get");
  assert.equal(posted[0].lane, "state");
  assert.deepEqual(posted[0].payload, {});
  assert.equal(posted[1].type, "diagnostics.get");
  assert.equal(posted[1].lane, "command");
  assert.deepEqual(posted[1].payload, {});

  deliverResponse(host, posted[0], { revision: 7 });
  deliverResponse(host, posted[1], { readyAcknowledged: true });
  assert.deepEqual(await snapshot, { revision: 7 });
  assert.deepEqual(await diagnostics, { readyAcknowledged: true });
}

{
  const initialSnapshot = {
    revision: 1,
    paramsRevision: 1,
    configRevision: 0,
    uiRevision: 0,
    config: { theme: "dark" },
    uiState: {}
  };
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const store = createSnapshotStore(bridge, initialSnapshot);
  const seen = [];
  const unsubscribe = store.subscribe((snapshot) => seen.push(snapshot.revision));

  assert.deepEqual(seen, [1]);
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "subscription.add");
  assert.deepEqual(posted[0].payload, { topic: "state.changed" });
  assert.equal(store.select((snapshot) => snapshot.config.theme), "dark");

  const refreshed = store.refresh();
  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "snapshot.get");
  assert.deepEqual(posted[1].payload, {});
  deliverResponse(host, posted[1], {
    revision: 2,
    paramsRevision: 1,
    configRevision: 1,
    uiRevision: 0,
    config: { theme: "light" },
    uiState: {}
  });
  assert.equal((await refreshed).revision, 2);
  assert.deepEqual(seen, [1, 2]);
  assert.equal(store.getSnapshot().config.theme, "light");

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "session",
    seq: 3,
    lane: "event",
    kind: "event",
    type: "state.changed",
    payload: {
      revision: 3,
      paramsRevision: 2,
      configRevision: 1,
      uiRevision: 0,
      config: { theme: "blue" },
      uiState: {}
    }
  });
  assert.deepEqual(seen, [1, 2, 3]);
  assert.equal(store.select((snapshot) => snapshot.paramsRevision), 2);

  unsubscribe();
  assert.equal(posted.length, 3);
  assert.equal(posted[2].type, "subscription.remove");
  assert.deepEqual(posted[2].payload, { topic: "state.changed" });
  store.dispose();
}

{
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const invalidOptions = [
    [null, /snapshot store options must be an object/],
    [[], /snapshot store options must be an object/],
    [{ topic: "" }, /subscription topic must not be empty/],
    [{ topic: "x".repeat(129) }, /subscription topic too long/],
    [{ topic: "state.changed\u0007" }, /subscription topic must not contain control characters/],
    [{ refreshOnEvent: "yes" }, /refreshOnEvent must be boolean/]
  ];

  for (const [options, message] of invalidOptions) {
    assert.throws(
      () => createSnapshotStore(bridge, undefined, options),
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, message);
        return true;
      }
    );
  }
  assert.equal(posted.length, 0);
}

{
  const initialSnapshot = {
    revision: 1,
    paramsRevision: 1,
    configRevision: 0,
    uiRevision: 0,
    config: { theme: "dark" },
    uiState: {}
  };
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const store = createSnapshotStore(bridge, initialSnapshot, {
    topic: "custom.state",
    refreshOnEvent: false
  });
  const seen = [];

  assert.throws(
    () => store.select(null),
    (error) => {
      assert.equal(error.code, "validation_error");
      assert.equal(error.retryable, false);
      assert.match(error.message, /snapshot selector must be a function/);
      return true;
    }
  );
  assert.equal(posted.length, 0);

  const unsubscribe = store.subscribe((snapshot) => seen.push(snapshot.revision));
  assert.deepEqual(seen, [1]);
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "subscription.add");
  assert.deepEqual(posted[0].payload, { topic: "custom.state" });

  host.__VESTY_INTERNAL__.deliver({
    v: 1,
    session: "session",
    seq: 4,
    lane: "event",
    kind: "event",
    type: "custom.state",
    payload: { patch: true }
  });
  assert.equal(posted.length, 1);
  assert.deepEqual(seen, [1]);

  unsubscribe();
  assert.equal(posted.length, 2);
  assert.equal(posted[1].type, "subscription.remove");
  assert.deepEqual(posted[1].payload, { topic: "custom.state" });
  store.dispose();
}

{
  const initialSnapshot = {
    revision: 1,
    paramsRevision: 1,
    configRevision: 0,
    uiRevision: 0,
    config: {},
    uiState: {}
  };
  const { host, posted } = makeHost();
  const bridge = createBridge(host, "session", { timeoutMs: 0 });
  const store = createSnapshotStore(bridge, initialSnapshot);

  for (const invalidListener of [null, {}, "listener"]) {
    assert.throws(
      () => store.subscribe(invalidListener),
      (error) => {
        assert.equal(error.code, "validation_error");
        assert.equal(error.retryable, false);
        assert.match(error.message, /snapshot listener must be a function/);
        return true;
      }
    );
  }
  assert.equal(posted.length, 0);

  const seen = [];
  const unsubscribe = store.subscribe((snapshot) => seen.push(snapshot.revision));
  assert.deepEqual(seen, [1]);
  assert.equal(posted.length, 1);
  assert.equal(posted[0].type, "subscription.add");
  unsubscribe();
  store.dispose();
}

{
  const realConsoleError = console.error;
  const listenerErrors = [];
  console.error = (...args) => listenerErrors.push(args);

  try {
    const initialSnapshot = {
      revision: 1,
      paramsRevision: 1,
      configRevision: 0,
      uiRevision: 0,
      config: { theme: "dark" },
      uiState: {}
    };
    const { host, posted } = makeHost();
    const bridge = createBridge(host, "session", { timeoutMs: 0 });
    const store = createSnapshotStore(bridge, initialSnapshot);
    const seen = [];
    let unsubscribeThrowing;

    assert.doesNotThrow(() => {
      unsubscribeThrowing = store.subscribe((snapshot) => {
        seen.push(["throwing", snapshot.revision]);
        throw new Error("snapshot listener failed");
      });
    });
    const unsubscribeOk = store.subscribe((snapshot) => seen.push(["ok", snapshot.revision]));

    assert.deepEqual(seen, [
      ["throwing", 1],
      ["ok", 1]
    ]);
    assert.equal(listenerErrors.length, 1);
    assert.equal(listenerErrors[0][0], "Vesty snapshot listener error");
    assert.match(listenerErrors[0][1].message, /snapshot listener failed/);
    assert.equal(posted.length, 1);
    assert.equal(posted[0].type, "subscription.add");

    const refreshed = store.refresh();
    assert.equal(posted.length, 2);
    assert.equal(posted[1].type, "snapshot.get");
    assert.deepEqual(posted[1].payload, {});
    deliverResponse(host, posted[1], {
      revision: 2,
      paramsRevision: 1,
      configRevision: 1,
      uiRevision: 0,
      config: { theme: "light" },
      uiState: {}
    });

    assert.equal((await refreshed).revision, 2);
    assert.deepEqual(seen, [
      ["throwing", 1],
      ["ok", 1],
      ["throwing", 2],
      ["ok", 2]
    ]);
    assert.equal(listenerErrors.length, 2);
    assert.match(listenerErrors[1][1].message, /snapshot listener failed/);

    unsubscribeThrowing();
    unsubscribeOk();
    assert.equal(posted.length, 3);
    assert.equal(posted[2].type, "subscription.remove");
    store.dispose();
  } finally {
    console.error = realConsoleError;
  }
}

{
  const realSetInterval = globalThis.setInterval;
  const realClearInterval = globalThis.clearInterval;
  const intervals = [];
  const cleared = [];
  globalThis.setInterval = (callback, ms) => {
    const token = { callback, ms };
    intervals.push(token);
    return token;
  };
  globalThis.clearInterval = (token) => {
    cleared.push(token);
  };

  try {
    for (let i = 0; i < 32; i += 1) {
      const session = `reload-session-${i}`;
      const { host, listeners, posted } = makeHost();
      const bridge = createBridge(host, session, { timeoutMs: 0 });
      let meterEvents = 0;

      const slow = bridge.request("slow.request", { i });
      const unsubscribe = bridge.subscribe("meter.main", () => {
        meterEvents += 1;
      });

      assert.equal(posted.length, 2);
      assert.equal(posted[0].type, "slow.request");
      assert.equal(posted[1].type, "subscription.add");
      assert.equal(intervals.length, i + 1);
      const interval = intervals.at(-1);

      interval.callback();
      assert.equal(posted.length, 3);
      assert.equal(posted[2].type, "event.flush");

      const pagehide = listeners.get("pagehide");
      assert.equal(typeof pagehide, "function");
      pagehide();

      await assert.rejects(slow, (error) => {
        assert.equal(error.code, "internal_error");
        assert.equal(error.retryable, true);
        return true;
      });
      assert.ok(cleared.includes(interval));

      host.__VESTY_INTERNAL__.deliver({
        v: 1,
        session,
        seq: 99,
        lane: "event",
        kind: "event",
        type: "meter.main",
        payload: { i }
      });
      assert.equal(meterEvents, 0);

      const postedAfterUnload = posted.length;
      unsubscribe();
      assert.equal(posted.length, postedAfterUnload);

      host.__VESTY_INTERNAL__.deliver({
        v: 1,
        session,
        seq: 100,
        lane: "command",
        kind: "response",
        type: "slow.request.response",
        replyTo: posted[0].id,
        payload: { late: true }
      });
      assert.equal(meterEvents, 0);
    }
  } finally {
    globalThis.setInterval = realSetInterval;
    globalThis.clearInterval = realClearInterval;
  }
}

console.log("plugin-ui bridge sequencing, subscriptions, async event pump, param helpers, and snapshot store ok");
