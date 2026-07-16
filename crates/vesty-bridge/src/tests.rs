use super::*;
use serde_json::{Value, json};
use vesty_core::MeterFrame;
use vesty_ipc::*;
use vesty_params::{ParamSpec, ParamSpecError};
use vesty_rt::{QueueId, RtLogEvent};

fn ready_payload(params: Vec<ParamSpec>) -> BridgeReadyPayload {
    let param_values = params
        .iter()
        .map(|param| ParamValueSnapshot {
            id: param.id.clone(),
            normalized: param.default_normalized,
        })
        .collect();
    BridgeReadyPayload {
        protocol_version: 1,
        instance_id: "instance".to_string(),
        editor_session_id: "session".to_string(),
        dev_mode: true,
        plugin_name: "Test".to_string(),
        vendor: "Vesty".to_string(),
        capabilities: BridgeCapabilities::v1_default(),
        params,
        param_values,
        snapshot: PluginSnapshot::default(),
    }
}

fn runtime() -> BridgeRuntime<MemoryTransport> {
    let mut gain = ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0);
    gain.unit = Some("dB".to_string());
    let mode = ParamSpec::choice("mode", "Mode", ["Clean", "Drive", "Fuzz"], 0);
    let ready = ready_payload(vec![gain, mode]);
    BridgeRuntime::new("session", ready, MemoryTransport::default()).unwrap()
}

fn runtime_with_capabilities(capabilities: BridgeCapabilities) -> BridgeRuntime<MemoryTransport> {
    let mut gain = ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0);
    gain.unit = Some("dB".to_string());
    let mut ready = ready_payload(vec![gain]);
    ready.capabilities = capabilities;
    BridgeRuntime::new("session", ready, MemoryTransport::default()).unwrap()
}

fn disable_diagnostics(capabilities: &mut BridgeCapabilities) {
    capabilities.diagnostics = false;
}

fn disable_subscriptions(capabilities: &mut BridgeCapabilities) {
    capabilities.subscriptions = false;
}

fn disable_state_config(capabilities: &mut BridgeCapabilities) {
    capabilities.state_config = false;
}

fn disable_param_gestures(capabilities: &mut BridgeCapabilities) {
    capabilities.param_gestures = false;
}

fn disable_param_format_parse(capabilities: &mut BridgeCapabilities) {
    capabilities.param_format_parse = false;
}

fn disable_meter_stream(capabilities: &mut BridgeCapabilities) {
    capabilities.meter_stream = false;
}

fn perform_hello(runtime: &mut BridgeRuntime<MemoryTransport>) {
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello","payload":{"supportedProtocolVersions":[1],"jsPackageVersion":"test","pageUrl":"vesty://assets/index.html"}}"#)
        .unwrap();
}

fn receive_request(
    runtime: &mut BridgeRuntime<MemoryTransport>,
    lane: &str,
    packet_type: &str,
    id: &str,
    payload: Option<Value>,
) {
    let mut packet = json!({
        "v": 1,
        "session": "session",
        "seq": 1,
        "lane": lane,
        "kind": "request",
        "type": packet_type,
        "id": id,
    });
    if let Some(payload) = payload {
        packet["payload"] = payload;
    }
    runtime.receive_json(&packet.to_string()).unwrap();
}

fn assert_last_error(
    runtime: &BridgeRuntime<MemoryTransport>,
    reply_to: &str,
    code: BridgeErrorCode,
    message: &str,
) {
    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.reply_to.as_deref(), Some(reply_to));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, code);
    assert_eq!(error.message, message);
    assert!(validate_bridge_error_message(&error.message).is_ok());
    if let Some(details) = error.details.as_ref() {
        assert!(validate_bridge_json_value(details).is_ok());
    }
}

fn assert_last_validation_error(
    runtime: &BridgeRuntime<MemoryTransport>,
    reply_to: &str,
    message: &str,
) {
    assert_last_error(runtime, reply_to, BridgeErrorCode::ValidationError, message);
}

#[test]
fn bridge_runtime_try_new_rejects_invalid_param_schema() {
    let duplicate = ready_payload(vec![
        ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0),
        ParamSpec::bool("gain", "Duplicate Gain", false),
    ]);
    assert!(matches!(
        BridgeRuntime::try_new("session", duplicate, MemoryTransport::default()),
        Err(BridgeRuntimeCreateError::ParamSpec(ParamSpecError::DuplicateId { id })) if id == "gain"
    ));

    let empty_id = ready_payload(vec![ParamSpec::float("", "Gain", -60.0, 12.0, 0.0)]);
    assert!(matches!(
        BridgeRuntime::try_new("session", empty_id, MemoryTransport::default()),
        Err(BridgeRuntimeCreateError::ParamSpec(
            ParamSpecError::EmptyId { index: 0 }
        ))
    ));
}

#[test]
fn bridge_runtime_try_new_rejects_invalid_sessions() {
    let ready = ready_payload(Vec::new());
    assert!(matches!(
        BridgeRuntime::try_new("", ready.clone(), MemoryTransport::default()),
        Err(BridgeRuntimeCreateError::Session(
            "bridge session must not be empty"
        ))
    ));
    assert!(matches!(
        BridgeRuntime::try_new(
            "s".repeat(MAX_BRIDGE_SESSION_BYTES + 1),
            ready.clone(),
            MemoryTransport::default()
        ),
        Err(BridgeRuntimeCreateError::Session("bridge session too long"))
    ));
    assert!(matches!(
        BridgeRuntime::try_new("bad\nsession", ready.clone(), MemoryTransport::default()),
        Err(BridgeRuntimeCreateError::Session(
            "bridge session must not contain control characters"
        ))
    ));

    let mut bad_ready = ready;
    bad_ready.editor_session_id = "bad\u{7}editor".to_string();
    assert!(matches!(
        BridgeRuntime::try_new("session", bad_ready, MemoryTransport::default()),
        Err(BridgeRuntimeCreateError::Session(
            "bridge session must not contain control characters"
        ))
    ));
}

#[test]
fn hello_returns_ready_payload() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"1","payload":{"supportedProtocolVersions":[1],"jsPackageVersion":"test","pageUrl":"vesty://assets/index.html"}}"#)
        .unwrap();
    assert_eq!(runtime.transport.sent.len(), 1);
    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Response);
    let payload = runtime.transport.sent[0].payload.as_ref().unwrap();
    assert_eq!(payload["capabilities"]["paramGestures"], true);
    assert_eq!(payload["capabilities"]["meterStream"], true);
    assert_eq!(payload["capabilities"]["diagnostics"], true);
    assert_eq!(
        payload["params"][0]["defaultNormalized"],
        0.8333333333333334
    );
    assert_eq!(payload["params"][0]["stepCount"], serde_json::Value::Null);
    assert_eq!(payload["params"][0]["flags"]["readOnly"], false);
    assert_eq!(payload["params"][0]["kind"]["float"]["min"], -60.0);
    assert_eq!(payload["params"][1]["kind"]["choice"]["values"][1], "Drive");
    assert!(payload["params"][0].get("default_normalized").is_none());
    assert!(payload["params"][0].get("step_count").is_none());
    assert!(payload["params"][0]["flags"].get("read_only").is_none());
}

#[test]
fn disabled_bridge_capabilities_reject_request_classes() {
    type DisabledCapabilityCase = (
        &'static str,
        fn(&mut BridgeCapabilities),
        &'static str,
        &'static str,
        Option<Value>,
        &'static str,
    );
    let cases: [DisabledCapabilityCase; 6] = [
        (
            "diagnostics-disabled",
            disable_diagnostics,
            "command",
            "diagnostics.get",
            None,
            "diagnostics capability is disabled",
        ),
        (
            "subscriptions-disabled",
            disable_subscriptions,
            "command",
            "subscription.add",
            Some(json!({ "topic": "state.changed" })),
            "subscriptions capability is disabled",
        ),
        (
            "state-config-disabled",
            disable_state_config,
            "state",
            "state.setConfig",
            Some(json!({
                "baseRevision": 0,
                "key": "theme",
                "value": "dark",
            })),
            "state config capability is disabled",
        ),
        (
            "param-gestures-disabled",
            disable_param_gestures,
            "param",
            "param.begin",
            Some(json!({ "id": "gain" })),
            "param gestures capability is disabled",
        ),
        (
            "param-format-disabled",
            disable_param_format_parse,
            "param",
            "param.format",
            Some(json!({ "id": "gain", "normalized": 0.5 })),
            "param format/parse capability is disabled",
        ),
        (
            "meter-stream-disabled",
            disable_meter_stream,
            "meter",
            "meter.flush",
            None,
            "meter stream capability is disabled",
        ),
    ];

    for (id, disable, lane, packet_type, payload, message) in cases {
        let mut capabilities = BridgeCapabilities::v1_default();
        disable(&mut capabilities);
        let mut runtime = runtime_with_capabilities(capabilities);

        receive_request(&mut runtime, lane, packet_type, id, payload);

        assert_last_error(&runtime, id, BridgeErrorCode::UnsupportedType, message);
        let error = runtime
            .transport
            .sent
            .last()
            .unwrap()
            .error
            .as_ref()
            .unwrap();
        assert!(!error.retryable);
        assert!(runtime.subscriptions.is_empty());
        assert_eq!(runtime.pending_param_gesture_count(), 0);
        assert_eq!(runtime.snapshot().revision, 0);
    }
}

#[test]
fn disabled_topic_capabilities_reject_matching_subscriptions() {
    let mut no_meter = BridgeCapabilities::v1_default();
    no_meter.meter_stream = false;
    let mut runtime = runtime_with_capabilities(no_meter);
    receive_request(
        &mut runtime,
        "command",
        "subscription.add",
        "meter-sub",
        Some(json!({ "topic": "meter.main" })),
    );
    assert_last_error(
        &runtime,
        "meter-sub",
        BridgeErrorCode::UnsupportedType,
        "meter stream capability is disabled",
    );
    assert!(runtime.subscriptions.is_empty());

    let mut no_diagnostics = BridgeCapabilities::v1_default();
    no_diagnostics.diagnostics = false;
    let mut runtime = runtime_with_capabilities(no_diagnostics);
    receive_request(
        &mut runtime,
        "command",
        "subscription.add",
        "diag-sub",
        Some(json!({ "topic": "diagnostics.fault" })),
    );
    assert_last_error(
        &runtime,
        "diag-sub",
        BridgeErrorCode::UnsupportedType,
        "diagnostics capability is disabled",
    );
    assert!(runtime.subscriptions.is_empty());

    let mut no_reliable_events = BridgeCapabilities::v1_default();
    no_reliable_events.reliable_events = false;
    let mut runtime = runtime_with_capabilities(no_reliable_events);
    receive_request(
        &mut runtime,
        "command",
        "subscription.add",
        "state-sub",
        Some(json!({ "topic": "state.changed" })),
    );
    assert_last_error(
        &runtime,
        "state-sub",
        BridgeErrorCode::UnsupportedType,
        "reliable events capability is disabled",
    );
    assert!(runtime.subscriptions.is_empty());

    receive_request(
        &mut runtime,
        "command",
        "subscription.remove",
        "state-unsub",
        Some(json!({ "topic": "state.changed" })),
    );
    assert_last_error(
        &runtime,
        "state-unsub",
        BridgeErrorCode::UnsupportedType,
        "reliable events capability is disabled",
    );
}

#[test]
fn disabled_topic_capabilities_suppress_native_events_and_meter_queue() {
    let mut no_meter = BridgeCapabilities::v1_default();
    no_meter.meter_stream = false;
    let mut runtime = runtime_with_capabilities(no_meter);
    assert!(!runtime.queue_latest_meter("meter.main", json!({ "peak": 0.5 })));
    assert_eq!(runtime.pending_latest_meter_count(), 0);
    assert_eq!(runtime.flush_latest_meters().unwrap(), 0);
    assert!(runtime.transport.sent.is_empty());

    let mut no_diagnostics = BridgeCapabilities::v1_default();
    no_diagnostics.diagnostics = false;
    let mut runtime = runtime_with_capabilities(no_diagnostics);
    assert!(
        runtime
            .emit_fault_report(
                "diagnostics.fault",
                PluginFaultReport {
                    faulted: true,
                    fault_count: 1,
                },
            )
            .is_ok()
    );
    assert!(
        runtime
            .emit_rt_log_event("log.rt", 1, RtLogEvent::Faulted { code: 7 },)
            .is_ok()
    );
    assert!(runtime.transport.sent.is_empty());
    assert_eq!(
        runtime.diagnostics_snapshot().fault,
        Some(PluginFaultReport {
            faulted: true,
            fault_count: 1,
        })
    );

    let mut no_reliable_events = BridgeCapabilities::v1_default();
    no_reliable_events.reliable_events = false;
    let mut runtime = runtime_with_capabilities(no_reliable_events);
    runtime
        .emit_event("state.changed", json!({ "revision": 1 }))
        .unwrap();
    assert!(runtime.transport.sent.is_empty());
}

#[test]
fn invalid_request_type_returns_validation_error_without_dispatching() {
    let cases = [
        ("", "empty-type", "request type must not be empty"),
        (
            &"x".repeat(MAX_BRIDGE_PACKET_TYPE_BYTES + 1),
            "long-ascii-type",
            "request type too long",
        ),
        (&"界".repeat(43), "long-utf8-type", "request type too long"),
        (
            "bridge.hello\u{7}",
            "control-type",
            "request type must not contain control characters",
        ),
    ];

    for (packet_type, id, message) in cases {
        let mut runtime = runtime();
        receive_request(
            &mut runtime,
            "command",
            packet_type,
            id,
            Some(json!({
                "supportedProtocolVersions": [1],
                "jsPackageVersion": "test",
                "pageUrl": "vesty://assets/index.html",
            })),
        );

        assert_eq!(runtime.transport.sent.len(), 1);
        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        assert_eq!(response.packet_type, "bridge.invalidType.error");
        assert_eq!(response.reply_to.as_deref(), Some(id));
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::ValidationError);
        assert_eq!(error.message, message);
        assert!(!error.retryable);
        assert!(!runtime.ready_acknowledged());
    }
}

#[test]
fn invalid_request_id_returns_validation_error_without_dispatching() {
    let cases = [
        (json!(null), "request id must be a string"),
        (json!(""), "request id must not be empty"),
        (
            json!("x".repeat(MAX_BRIDGE_PACKET_ID_BYTES + 1)),
            "request id too long",
        ),
        (
            json!("hello\u{7}request"),
            "request id must not contain control characters",
        ),
    ];

    for (id, message) in cases {
        let mut runtime = runtime();
        let mut packet = json!({
            "v": 1,
            "session": "session",
            "seq": 1,
            "lane": "command",
            "kind": "request",
            "type": "bridge.hello",
            "payload": {
                "supportedProtocolVersions": [1],
                "jsPackageVersion": "test",
                "pageUrl": "vesty://assets/index.html",
            },
        });
        if !id.is_null() {
            packet["id"] = id;
        }

        runtime.receive_json(&packet.to_string()).unwrap();

        assert_eq!(runtime.transport.sent.len(), 1);
        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        assert_eq!(response.packet_type, "bridge.hello.error");
        assert_eq!(response.reply_to, None);
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::ValidationError);
        assert_eq!(error.message, message);
        assert!(!error.retryable);
        assert!(!runtime.ready_acknowledged());
    }
}

#[test]
fn inbound_non_request_packets_are_dropped_without_response() {
    let cases = ["response", "event", "ack", "error"];

    for kind in cases {
        let mut runtime = runtime();
        let mut packet = json!({
            "v": 1,
            "session": "session",
            "seq": 1,
            "lane": "command",
            "kind": kind,
            "type": "bridge.hello",
            "id": "hello",
            "replyTo": "server-request",
        });
        if kind == "error" {
            packet["error"] = json!({
                "code": "internal_error",
                "message": "client supplied error",
                "retryable": false,
            });
        }

        runtime.receive_json(&packet.to_string()).unwrap();

        assert!(runtime.transport.sent.is_empty());
        assert!(!runtime.ready_acknowledged());
    }
}

#[test]
fn request_reply_to_and_error_fields_are_rejected() {
    let cases = [
        (
            json!({ "replyTo": "server-request" }),
            "request replyTo must not be set",
        ),
        (
            json!({
                "error": {
                    "code": "internal_error",
                    "message": "client supplied error",
                    "retryable": false,
                },
            }),
            "request error must not be set",
        ),
    ];

    for (extra, message) in cases {
        let mut runtime = runtime();
        let mut packet = json!({
            "v": 1,
            "session": "session",
            "seq": 1,
            "lane": "command",
            "kind": "request",
            "type": "bridge.hello",
            "id": "hello",
            "payload": {
                "supportedProtocolVersions": [1],
                "jsPackageVersion": "test",
                "pageUrl": "vesty://assets/index.html",
            },
        });
        let object = extra.as_object().unwrap();
        for (key, value) in object {
            packet[key] = value.clone();
        }

        runtime.receive_json(&packet.to_string()).unwrap();

        assert_eq!(runtime.transport.sent.len(), 1);
        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        assert_eq!(response.reply_to.as_deref(), Some("hello"));
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::ValidationError);
        assert_eq!(error.message, message);
        assert!(!runtime.ready_acknowledged());
    }
}

#[test]
fn request_payload_json_bounds_are_validated_before_dispatch() {
    let mut runtime = runtime();
    let mut too_deep = Value::Null;
    for _ in 0..=vesty_ipc::MAX_BRIDGE_JSON_DEPTH {
        too_deep = Value::Array(vec![too_deep]);
    }
    receive_request(
        &mut runtime,
        "command",
        "bridge.hello",
        "hello",
        Some(too_deep),
    );

    assert_eq!(runtime.transport.sent.len(), 1);
    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.reply_to.as_deref(), Some("hello"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::ValidationError);
    assert_eq!(error.message, "bridge JSON value exceeds maximum depth");
    assert!(!runtime.ready_acknowledged());
}

#[test]
fn request_flags_are_validated_before_dispatch() {
    let cases = [
        (
            json!(vec!["flag"; MAX_BRIDGE_PACKET_FLAGS + 1]),
            "bridge packet has too many flags",
        ),
        (json!([""]), "bridge packet flag must not be empty"),
        (
            json!(["x".repeat(MAX_BRIDGE_PACKET_FLAG_BYTES + 1)]),
            "bridge packet flag too long",
        ),
        (
            json!(["bad\u{7}flag"]),
            "bridge packet flag must not contain control characters",
        ),
    ];

    for (flags, message) in cases {
        let mut runtime = runtime();
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": 1,
            "lane": "command",
            "kind": "request",
            "type": "bridge.hello",
            "id": "hello",
            "flags": flags,
            "payload": {
                "supportedProtocolVersions": [1],
                "jsPackageVersion": "test",
                "pageUrl": "vesty://assets/index.html",
            },
        });

        runtime.receive_json(&packet.to_string()).unwrap();

        assert_eq!(runtime.transport.sent.len(), 1);
        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        assert_eq!(response.reply_to.as_deref(), Some("hello"));
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::ValidationError);
        assert_eq!(error.message, message);
        assert!(!runtime.ready_acknowledged());
    }
}

#[test]
fn request_seq_is_validated_before_dispatch() {
    let mut runtime = runtime();
    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": MAX_BRIDGE_PACKET_SEQ + 1,
        "lane": "command",
        "kind": "request",
        "type": "bridge.hello",
        "id": "hello",
        "payload": {
            "supportedProtocolVersions": [1],
            "jsPackageVersion": "test",
            "pageUrl": "vesty://assets/index.html",
        },
    });

    runtime.receive_json(&packet.to_string()).unwrap();

    assert_eq!(runtime.transport.sent.len(), 1);
    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.reply_to.as_deref(), Some("hello"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::ValidationError);
    assert_eq!(
        error.message,
        "bridge packet seq exceeds JavaScript safe integer range"
    );
    assert!(!runtime.ready_acknowledged());
}

#[test]
fn outbound_seq_wraps_before_js_safe_integer_overflow() {
    let mut runtime = runtime();
    runtime.seq = MAX_BRIDGE_PACKET_SEQ;

    assert_eq!(runtime.next_seq(), MAX_BRIDGE_PACKET_SEQ);
    assert_eq!(runtime.next_seq(), 1);
    assert_eq!(runtime.next_seq(), 2);
}

#[test]
fn recoverable_parse_error_rejects_unsafe_seq() {
    let mut runtime = runtime();
    let text = format!(
        r#"{{"v":1,"session":"session","seq":{},"lane":"command","kind":"request","type":42,"id":"bad-type"}}"#,
        MAX_BRIDGE_PACKET_SEQ + 1
    );

    assert!(runtime.receive_json(&text).is_err());
    assert!(runtime.transport.sent.is_empty());
}

#[test]
fn invalid_inbound_session_is_dropped_without_reflection() {
    let cases = [
        "".to_string(),
        "s".repeat(MAX_BRIDGE_SESSION_BYTES + 1),
        "bad\u{7}session".to_string(),
    ];

    for session in cases {
        let mut runtime = runtime();
        let packet = json!({
            "v": 1,
            "session": session,
            "seq": 1,
            "lane": "command",
            "kind": "request",
            "type": "bridge.hello",
            "id": "hello",
            "payload": {
                "supportedProtocolVersions": [1],
                "jsPackageVersion": "test",
                "pageUrl": "vesty://assets/index.html",
            },
        });

        runtime.receive_json(&packet.to_string()).unwrap();

        assert!(runtime.transport.sent.is_empty());
        assert!(!runtime.ready_acknowledged());
    }
}

#[test]
fn stale_but_valid_inbound_session_returns_permission_denied() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"stale-session","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello","payload":{"supportedProtocolVersions":[1],"jsPackageVersion":"test","pageUrl":"vesty://assets/index.html"}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent.len(), 1);
    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.session, "stale-session");
    assert_eq!(response.reply_to.as_deref(), Some("hello"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::PermissionDenied);
    assert_eq!(error.message, "session mismatch");
    assert!(!runtime.ready_acknowledged());
}

#[test]
fn recoverable_parse_error_replies_to_current_session_request_id() {
    let mut runtime = runtime();
    runtime
        .receive_json(
            r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":42,"id":"bad-type"}"#,
        )
        .unwrap();

    assert_eq!(runtime.transport.sent.len(), 1);
    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.packet_type, "bridge.parseError.error");
    assert_eq!(response.reply_to.as_deref(), Some("bad-type"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::ParseError);
    assert_eq!(error.message, "failed to parse bridge packet");
    assert!(!error.retryable);
}

#[test]
fn recoverable_parse_error_rejects_invalid_request_id() {
    let cases = [
        r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":42,"id":""}"#.to_string(),
        format!(
            r#"{{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":42,"id":"{}"}}"#,
            "x".repeat(MAX_BRIDGE_PACKET_ID_BYTES + 1)
        ),
        r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":42,"id":"bad\u0007id"}"#.to_string(),
    ];

    for text in cases {
        let mut runtime = runtime();
        assert!(runtime.receive_json(&text).is_err());
        assert!(runtime.transport.sent.is_empty());
    }
}

#[test]
fn unrecoverable_parse_error_does_not_send_response() {
    let mut stale_session = runtime();
    assert!(stale_session
        .receive_json(
            r#"{"v":1,"session":"stale","seq":1,"lane":"command","kind":"request","type":42,"id":"bad-type"}"#,
        )
        .is_err());
    assert!(stale_session.transport.sent.is_empty());

    let mut non_request = runtime();
    assert!(non_request
        .receive_json(
            r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"event","type":42,"id":"bad-type"}"#,
        )
        .is_err());
    assert!(non_request.transport.sent.is_empty());
}

#[test]
fn ready_ack_marks_runtime_ready() {
    let mut runtime = runtime();
    perform_hello(&mut runtime);
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"command","kind":"request","type":"bridge.readyAck","id":"ack","payload":{"protocolVersion":1}}"#)
        .unwrap();

    assert!(runtime.ready_acknowledged());
    assert_eq!(runtime.transport.sent.len(), 2);
    assert_eq!(
        runtime.transport.sent[1].packet_type,
        "bridge.readyAck.response"
    );
    assert_eq!(
        runtime.transport.sent[1].payload.as_ref().unwrap()["ready"],
        true
    );
}

#[test]
fn ready_ack_requires_prior_hello() {
    let mut runtime = runtime();

    receive_request(
        &mut runtime,
        "command",
        "bridge.readyAck",
        "ack-before-hello",
        Some(json!({ "protocolVersion": 1 })),
    );

    assert_last_error(
        &runtime,
        "ack-before-hello",
        BridgeErrorCode::PermissionDenied,
        "readyAck requires bridge.hello",
    );
    assert!(!runtime.ready_acknowledged());
}

#[test]
fn ready_ack_payload_shape_is_validated() {
    let cases = [
        (None, "missing readyAck protocolVersion"),
        (Some(json!({})), "missing readyAck protocolVersion"),
        (
            Some(json!({ "protocolVersion": null })),
            "readyAck protocolVersion must be a non-negative integer",
        ),
        (
            Some(json!({ "protocolVersion": "1" })),
            "readyAck protocolVersion must be a non-negative integer",
        ),
        (
            Some(json!({ "protocolVersion": -1 })),
            "readyAck protocolVersion must be a non-negative integer",
        ),
        (
            Some(json!({ "protocolVersion": 1.5 })),
            "readyAck protocolVersion must be a non-negative integer",
        ),
    ];

    for (index, (payload, message)) in cases.into_iter().enumerate() {
        let mut runtime = runtime();
        perform_hello(&mut runtime);
        let id = format!("ack-invalid-{index}");
        receive_request(&mut runtime, "command", "bridge.readyAck", &id, payload);

        assert_last_validation_error(&runtime, &id, message);
        assert!(!runtime.ready_acknowledged());
    }
}

#[test]
fn ready_ack_rejects_unsupported_protocol_version() {
    let mut runtime = runtime();
    perform_hello(&mut runtime);

    receive_request(
        &mut runtime,
        "command",
        "bridge.readyAck",
        "ack-v2",
        Some(json!({ "protocolVersion": 2 })),
    );

    assert_last_error(
        &runtime,
        "ack-v2",
        BridgeErrorCode::UnsupportedVersion,
        "unsupported bridge readyAck protocol",
    );
    assert!(!runtime.ready_acknowledged());
}

#[test]
fn hello_requires_supported_protocol_payload() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"missing"}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"command","kind":"request","type":"bridge.hello","id":"unsupported","payload":{"supportedProtocolVersions":[2],"jsPackageVersion":"test","pageUrl":"vesty://assets/index.html"}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Error);
    assert_eq!(
        runtime.transport.sent[0].reply_to.as_deref(),
        Some("missing")
    );
    assert_eq!(
        runtime.transport.sent[0].error.as_ref().unwrap().code,
        BridgeErrorCode::ValidationError
    );
    assert_eq!(runtime.transport.sent[1].kind, BridgeKind::Error);
    assert_eq!(
        runtime.transport.sent[1].reply_to.as_deref(),
        Some("unsupported")
    );
    assert_eq!(
        runtime.transport.sent[1].error.as_ref().unwrap().code,
        BridgeErrorCode::UnsupportedVersion
    );
}

#[test]
fn hello_payload_allows_unknown_fields_for_protocol_extension() {
    let mut runtime = runtime();

    runtime
        .receive_json(
            r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello","payload":{"supportedProtocolVersions":[1],"jsPackageVersion":"test","pageUrl":"vesty://assets/index.html","jsCapabilities":{"supportsDocking":true}}}"#,
        )
        .unwrap();

    assert_eq!(runtime.transport.sent.len(), 1);
    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Response);
    assert_eq!(
        runtime.transport.sent[0].packet_type,
        "bridge.hello.response"
    );
    assert!(runtime.hello_acknowledged);
}

#[test]
fn hello_promotes_pending_session_to_editor_session() {
    let ready = BridgeReadyPayload {
        protocol_version: 1,
        instance_id: "instance".to_string(),
        editor_session_id: "editor-session-42".to_string(),
        dev_mode: true,
        plugin_name: "Test".to_string(),
        vendor: "Vesty".to_string(),
        capabilities: BridgeCapabilities::v1_default(),
        params: Vec::new(),
        param_values: Vec::new(),
        snapshot: PluginSnapshot::default(),
    };
    let mut runtime = BridgeRuntime::new("pending", ready, MemoryTransport::default()).unwrap();

    runtime
        .receive_json(r#"{"v":1,"session":"pending","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello","payload":{"supportedProtocolVersions":[1],"jsPackageVersion":"test","pageUrl":"vesty://assets/index.html"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"pending","seq":2,"lane":"state","kind":"request","type":"snapshot.get","id":"stale"}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"editor-session-42","seq":3,"lane":"state","kind":"request","type":"snapshot.get","id":"fresh"}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"editor-session-42","seq":4,"lane":"command","kind":"request","type":"bridge.readyAck","id":"ready","payload":{"protocolVersion":1}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Response);
    assert_eq!(runtime.transport.sent[0].reply_to.as_deref(), Some("hello"));
    assert_eq!(runtime.transport.sent[1].kind, BridgeKind::Error);
    assert_eq!(runtime.transport.sent[1].reply_to.as_deref(), Some("stale"));
    assert_eq!(
        runtime.transport.sent[1].error.as_ref().unwrap().code,
        BridgeErrorCode::PermissionDenied
    );
    assert_eq!(runtime.transport.sent[2].kind, BridgeKind::Response);
    assert_eq!(runtime.transport.sent[2].reply_to.as_deref(), Some("fresh"));
    assert_eq!(runtime.transport.sent[3].reply_to.as_deref(), Some("ready"));
    assert!(runtime.ready_acknowledged());
}

#[test]
fn reload_hello_resets_session_state_and_refreshes_param_values() {
    let mut ready = ready_payload(vec![ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5)]);
    ready.editor_session_id = "editor-reload".to_string();
    let mut runtime = BridgeRuntime::new("pending", ready, MemoryTransport::default()).unwrap();

    runtime
        .receive_json(r#"{"v":1,"session":"pending","seq":1,"lane":"command","kind":"request","type":"bridge.hello","id":"hello-1","payload":{"supportedProtocolVersions":[1]}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"editor-reload","seq":2,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"param.changed"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"editor-reload","seq":3,"lane":"param","kind":"request","type":"param.begin","id":"begin","payload":{"id":"gain"}}"#)
        .unwrap();
    assert_eq!(runtime.diagnostics_snapshot().subscription_count, 1);
    assert_eq!(runtime.pending_param_gesture_count(), 1);

    runtime.set_ready_param_values(vec![ParamValueSnapshot {
        id: "gain".to_string(),
        normalized: 0.75,
    }]);
    runtime
        .receive_json(r#"{"v":1,"session":"pending","seq":4,"lane":"command","kind":"request","type":"bridge.hello","id":"hello-2","payload":{"supportedProtocolVersions":[1]}}"#)
        .unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    assert_eq!(response.reply_to.as_deref(), Some("hello-2"));
    assert_eq!(
        response.payload.as_ref().unwrap()["paramValues"][0]["normalized"],
        0.75
    );
    assert_eq!(runtime.diagnostics_snapshot().subscription_count, 0);
    assert_eq!(runtime.pending_param_gesture_count(), 0);
    assert!(!runtime.ready_acknowledged());
}

#[test]
fn oversized_command_returns_backpressure() {
    let mut runtime = runtime();
    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": 1,
        "lane": "command",
        "kind": "request",
        "type": "bridge.hello",
        "id": "oversized",
        "payload": { "blob": "x".repeat(MAX_COMMAND_MESSAGE_BYTES) },
    })
    .to_string();
    assert!(packet.len() > MAX_COMMAND_MESSAGE_BYTES);

    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.reply_to.as_deref(), Some("oversized"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::Backpressure);
    assert!(error.retryable);
}

#[test]
fn state_messages_use_larger_size_limit() {
    let mut runtime = runtime();
    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": 1,
        "lane": "state",
        "kind": "request",
        "type": "state.setConfig",
        "id": "large-state",
        "payload": {
            "baseRevision": 0,
            "key": "large",
            "value": "x".repeat(MAX_COMMAND_MESSAGE_BYTES),
        },
    })
    .to_string();
    assert!(packet.len() > MAX_COMMAND_MESSAGE_BYTES);

    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    assert_eq!(response.reply_to.as_deref(), Some("large-state"));
}

#[test]
fn param_gesture_requests_are_supported() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"param","kind":"request","type":"param.begin","id":"1","payload":{"id":"gain","gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"param","kind":"request","type":"param.perform","id":"2","payload":{"id":"gain","normalized":1.2,"gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":3,"lane":"param","kind":"request","type":"param.end","id":"3","payload":{"id":"gain","gestureId":"drag-1"}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent.len(), 3);
    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Response);
    assert_eq!(
        runtime.transport.sent[1].payload.as_ref().unwrap()["normalized"],
        1.0
    );
    assert_eq!(runtime.pending_param_gesture_count(), 3);
    assert_eq!(
        runtime.drain_param_gestures(),
        vec![
            ParamGesture {
                phase: ParamGesturePhase::Begin,
                id: "gain".to_string(),
                normalized: None,
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["1".to_string()],
            },
            ParamGesture {
                phase: ParamGesturePhase::Perform,
                id: "gain".to_string(),
                normalized: Some(1.0),
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["2".to_string()],
            },
            ParamGesture {
                phase: ParamGesturePhase::End,
                id: "gain".to_string(),
                normalized: None,
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["3".to_string()],
            },
        ]
    );
    assert_eq!(runtime.pending_param_gesture_count(), 0);
}

#[test]
fn param_performs_are_coalesced_until_gesture_boundary() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"param","kind":"request","type":"param.begin","id":"begin","payload":{"id":"gain","gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"param","kind":"request","type":"param.perform","id":"perform-1","payload":{"id":"gain","normalized":0.2,"gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":3,"lane":"param","kind":"request","type":"param.perform","id":"perform-2","payload":{"id":"gain","normalized":0.8,"gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":4,"lane":"param","kind":"request","type":"param.end","id":"end","payload":{"id":"gain","gestureId":"drag-1"}}"#)
        .unwrap();

    assert_eq!(runtime.pending_param_gesture_count(), 3);
    assert_eq!(
        runtime.transport.sent[2].payload.as_ref().unwrap()["coalesced"],
        true
    );
    assert_eq!(
        runtime.drain_param_gestures(),
        vec![
            ParamGesture {
                phase: ParamGesturePhase::Begin,
                id: "gain".to_string(),
                normalized: None,
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["begin".to_string()],
            },
            ParamGesture {
                phase: ParamGesturePhase::Perform,
                id: "gain".to_string(),
                normalized: Some(0.8),
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["perform-1".to_string(), "perform-2".to_string()],
            },
            ParamGesture {
                phase: ParamGesturePhase::End,
                id: "gain".to_string(),
                normalized: None,
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["end".to_string()],
            },
        ]
    );
}

#[test]
fn param_perform_coalescing_does_not_cross_end_boundary() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"param","kind":"request","type":"param.begin","id":"begin","payload":{"id":"gain","gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"param","kind":"request","type":"param.perform","id":"perform-1","payload":{"id":"gain","normalized":0.2,"gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":3,"lane":"param","kind":"request","type":"param.end","id":"end","payload":{"id":"gain","gestureId":"drag-1"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":4,"lane":"param","kind":"request","type":"param.perform","id":"perform-2","payload":{"id":"gain","normalized":0.7,"gestureId":"late"}}"#)
        .unwrap();

    assert_eq!(runtime.pending_param_gesture_count(), 4);
    assert_eq!(
        runtime.drain_param_gestures(),
        vec![
            ParamGesture {
                phase: ParamGesturePhase::Begin,
                id: "gain".to_string(),
                normalized: None,
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["begin".to_string()],
            },
            ParamGesture {
                phase: ParamGesturePhase::Perform,
                id: "gain".to_string(),
                normalized: Some(0.2),
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["perform-1".to_string()],
            },
            ParamGesture {
                phase: ParamGesturePhase::End,
                id: "gain".to_string(),
                normalized: None,
                gesture_id: Some("drag-1".to_string()),
                request_ids: vec!["end".to_string()],
            },
            ParamGesture {
                phase: ParamGesturePhase::Perform,
                id: "gain".to_string(),
                normalized: Some(0.7),
                gesture_id: Some("late".to_string()),
                request_ids: vec!["perform-2".to_string()],
            },
        ]
    );
}

#[test]
fn formats_and_parses_params() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"param","kind":"request","type":"param.format","id":"1","payload":{"id":"gain","normalized":1.0}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"param","kind":"request","type":"param.parse","id":"2","payload":{"id":"gain","text":"12.0 dB"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":3,"lane":"param","kind":"request","type":"param.format","id":"3","payload":{"id":"mode","normalized":0.5}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":4,"lane":"param","kind":"request","type":"param.parse","id":"4","payload":{"id":"mode","text":"Fuzz"}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent[0].payload, Some(json!("12.000 dB")));
    assert_eq!(runtime.transport.sent[1].payload, Some(json!(1.0)));
    assert_eq!(runtime.transport.sent[2].payload, Some(json!("Drive")));
    assert_eq!(runtime.transport.sent[3].payload, Some(json!(1.0)));
}

#[test]
fn state_set_config_requires_current_base_revision() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"state","kind":"request","type":"state.setConfig","id":"state-1","payload":{"baseRevision":0,"key":"theme","value":"dark"}}"#)
        .unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    let snapshot = response.payload.as_ref().unwrap();
    assert_eq!(snapshot["revision"], 1);
    assert_eq!(snapshot["configRevision"], 1);
    assert_eq!(snapshot["config"]["theme"], "dark");
}

#[test]
fn stale_state_set_config_returns_conflict() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"state","kind":"request","type":"state.setConfig","id":"state-1","payload":{"baseRevision":0,"key":"theme","value":"dark"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"state","kind":"request","type":"state.setConfig","id":"state-2","payload":{"baseRevision":0,"key":"theme","value":"light"}}"#)
        .unwrap();

    let error = runtime.transport.sent.last().unwrap();
    assert_eq!(error.kind, BridgeKind::Error);
    assert_eq!(
        error.error.as_ref().unwrap().code,
        BridgeErrorCode::StateConflict
    );
    let details = error.error.as_ref().unwrap().details.as_ref().unwrap();
    assert!(validate_bridge_json_value(details).is_ok());
    assert_eq!(details["snapshot"]["config"]["theme"], "dark");
    assert_eq!(details["snapshot"]["configRevision"], 1);
}

#[test]
fn state_set_config_without_base_revision_is_invalid() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"state","kind":"request","type":"state.setConfig","id":"state-1","payload":{"key":"theme","value":"dark"}}"#)
        .unwrap();
    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Error);
    assert_eq!(
        runtime.transport.sent[0].error.as_ref().unwrap().code,
        BridgeErrorCode::ValidationError
    );
}

#[test]
fn state_set_config_payload_shape_is_validated() {
    let cases = [
        (None, "missing config baseRevision"),
        (Some(json!(null)), "missing config baseRevision"),
        (Some(json!({})), "missing config baseRevision"),
        (
            Some(json!({ "baseRevision": null, "key": "theme", "value": "dark" })),
            "baseRevision must be a non-negative integer",
        ),
        (
            Some(json!({ "baseRevision": "0", "key": "theme", "value": "dark" })),
            "baseRevision must be a non-negative integer",
        ),
        (
            Some(json!({ "baseRevision": -1, "key": "theme", "value": "dark" })),
            "baseRevision must be a non-negative integer",
        ),
        (
            Some(json!({ "baseRevision": 0.5, "key": "theme", "value": "dark" })),
            "baseRevision must be a non-negative integer",
        ),
        (
            Some(json!({ "baseRevision": 0, "value": "dark" })),
            "missing config key",
        ),
        (
            Some(json!({ "baseRevision": 0, "key": null, "value": "dark" })),
            "config key must be a string",
        ),
        (
            Some(json!({ "baseRevision": 0, "key": "theme" })),
            "missing config value",
        ),
    ];

    for (index, (payload, message)) in cases.into_iter().enumerate() {
        let mut runtime = runtime();
        let id = format!("config-invalid-{index}");
        receive_request(&mut runtime, "state", "state.setConfig", &id, payload);

        assert_last_validation_error(&runtime, &id, message);
        assert_eq!(runtime.snapshot().revision, 0);
        assert_eq!(runtime.snapshot().config_revision, 0);
        assert_eq!(runtime.state.config_entry_count(), 0);
    }
}

#[test]
fn state_set_ui_state_requires_current_ui_revision() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"state","kind":"request","type":"state.setUiState","id":"ui-1","payload":{"baseRevision":0,"value":{"panel":"advanced"}}}"#)
        .unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    let snapshot = response.payload.as_ref().unwrap();
    assert_eq!(snapshot["revision"], 1);
    assert_eq!(snapshot["uiRevision"], 1);
    assert_eq!(snapshot["uiState"]["panel"], "advanced");
}

#[test]
fn state_writes_emit_state_changed_snapshot_when_subscribed() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"state.changed"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"state","kind":"request","type":"state.setConfig","id":"config","payload":{"baseRevision":0,"key":"theme","value":"dark"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":3,"lane":"state","kind":"request","type":"state.setUiState","id":"ui","payload":{"baseRevision":0,"value":{"panel":"advanced"}}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent.len(), 5);
    assert_eq!(
        runtime.transport.sent[1].reply_to.as_deref(),
        Some("config")
    );
    let config_event = &runtime.transport.sent[2];
    assert_eq!(config_event.kind, BridgeKind::Event);
    assert_eq!(config_event.packet_type, "state.changed");
    assert_eq!(config_event.payload.as_ref().unwrap()["revision"], 1);
    assert_eq!(
        config_event.payload.as_ref().unwrap()["config"]["theme"],
        "dark"
    );

    assert_eq!(runtime.transport.sent[3].reply_to.as_deref(), Some("ui"));
    let ui_event = &runtime.transport.sent[4];
    assert_eq!(ui_event.kind, BridgeKind::Event);
    assert_eq!(ui_event.packet_type, "state.changed");
    assert_eq!(ui_event.payload.as_ref().unwrap()["revision"], 2);
    assert_eq!(
        ui_event.payload.as_ref().unwrap()["uiState"]["panel"],
        "advanced"
    );
}

#[test]
fn host_snapshot_restore_updates_runtime_and_emits_state_changed() {
    let mut runtime = runtime();
    let restored = PluginSnapshot {
        revision: 8,
        params_revision: 2,
        config_revision: 3,
        ui_revision: 4,
        config: json!({ "theme": "dark" }),
        ui_state: json!({ "panel": "advanced" }),
    };

    assert!(
        runtime
            .restore_snapshot_from_host(restored.clone())
            .unwrap()
    );
    assert!(runtime.transport.sent.is_empty());
    assert_eq!(runtime.snapshot(), &restored);

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"state.changed"}}"#)
        .unwrap();
    let next = PluginSnapshot {
        revision: 9,
        params_revision: 2,
        config_revision: 4,
        ui_revision: 4,
        config: json!({ "theme": "light" }),
        ui_state: json!({ "panel": "advanced" }),
    };

    assert!(runtime.restore_snapshot_from_host(next).unwrap());
    let event = runtime.transport.sent.last().unwrap();
    assert_eq!(event.kind, BridgeKind::Event);
    assert_eq!(event.packet_type, "state.changed");
    assert_eq!(event.payload.as_ref().unwrap()["revision"], 9);
    assert_eq!(event.payload.as_ref().unwrap()["config"]["theme"], "light");
    assert!(
        !runtime
            .restore_snapshot_from_host(runtime.snapshot().clone())
            .unwrap()
    );
}

#[test]
fn stale_state_set_ui_state_returns_conflict() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"state","kind":"request","type":"state.setUiState","id":"ui-1","payload":{"baseRevision":0,"value":{"panel":"advanced"}}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"state","kind":"request","type":"state.setUiState","id":"ui-2","payload":{"baseRevision":0,"value":{"panel":"simple"}}}"#)
        .unwrap();

    let error = runtime.transport.sent.last().unwrap();
    assert_eq!(error.kind, BridgeKind::Error);
    assert_eq!(
        error.error.as_ref().unwrap().code,
        BridgeErrorCode::StateConflict
    );
    let details = error.error.as_ref().unwrap().details.as_ref().unwrap();
    assert!(validate_bridge_json_value(details).is_ok());
    assert_eq!(details["snapshot"]["uiState"]["panel"], "advanced");
    assert_eq!(details["snapshot"]["uiRevision"], 1);
}

#[test]
fn state_set_ui_state_without_base_revision_is_invalid() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"state","kind":"request","type":"state.setUiState","id":"ui-1","payload":{"value":{"panel":"advanced"}}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Error);
    assert_eq!(
        runtime.transport.sent[0].error.as_ref().unwrap().code,
        BridgeErrorCode::ValidationError
    );
}

#[test]
fn state_set_ui_state_payload_shape_is_validated() {
    let cases = [
        (None, "missing ui state baseRevision"),
        (Some(json!(null)), "missing ui state baseRevision"),
        (Some(json!({})), "missing ui state baseRevision"),
        (
            Some(json!({ "baseRevision": null, "value": { "panel": "advanced" } })),
            "baseRevision must be a non-negative integer",
        ),
        (
            Some(json!({ "baseRevision": "0", "value": { "panel": "advanced" } })),
            "baseRevision must be a non-negative integer",
        ),
        (
            Some(json!({ "baseRevision": -1, "value": { "panel": "advanced" } })),
            "baseRevision must be a non-negative integer",
        ),
        (
            Some(json!({ "baseRevision": 0.5, "value": { "panel": "advanced" } })),
            "baseRevision must be a non-negative integer",
        ),
        (Some(json!({ "baseRevision": 0 })), "missing ui state value"),
    ];

    for (index, (payload, message)) in cases.into_iter().enumerate() {
        let mut runtime = runtime();
        let id = format!("ui-invalid-{index}");
        receive_request(&mut runtime, "state", "state.setUiState", &id, payload);

        assert_last_validation_error(&runtime, &id, message);
        assert_eq!(runtime.snapshot().revision, 0);
        assert_eq!(runtime.snapshot().ui_revision, 0);
        assert!(runtime.snapshot().ui_state.is_null());
    }
}

#[test]
fn state_writes_accept_null_json_values() {
    let mut runtime = runtime();

    receive_request(
        &mut runtime,
        "state",
        "state.setConfig",
        "config-null",
        Some(json!({
            "baseRevision": 0,
            "key": "theme",
            "value": null,
        })),
    );
    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    assert!(response.payload.as_ref().unwrap()["config"]["theme"].is_null());
    assert_eq!(runtime.snapshot().config_revision, 1);

    receive_request(
        &mut runtime,
        "state",
        "state.setUiState",
        "ui-null",
        Some(json!({
            "baseRevision": 0,
            "value": null,
        })),
    );
    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    assert!(response.payload.as_ref().unwrap()["uiState"].is_null());
    assert_eq!(runtime.snapshot().ui_revision, 1);
}

#[test]
fn state_set_config_rejects_invalid_keys() {
    let cases = [
        ("state-empty", String::new()),
        ("state-long", "x".repeat(MAX_CONFIG_KEY_BYTES + 1)),
        ("state-control", "bad\nkey".to_string()),
    ];

    for (id, key) in cases {
        let mut runtime = runtime();
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": 1,
            "lane": "state",
            "kind": "request",
            "type": "state.setConfig",
            "id": id,
            "payload": {
                "baseRevision": 0,
                "key": key,
                "value": "dark",
            },
        })
        .to_string();

        runtime.receive_json(&packet).unwrap();

        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        assert_eq!(response.reply_to.as_deref(), Some(id));
        assert_eq!(
            response.error.as_ref().unwrap().code,
            BridgeErrorCode::ValidationError
        );
    }
}

#[test]
fn state_config_table_full_returns_backpressure_for_new_keys() {
    let mut runtime = runtime();
    for index in 0..MAX_CONFIG_ENTRIES {
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 1,
            "lane": "state",
            "kind": "request",
            "type": "state.setConfig",
            "id": format!("state-{index}"),
            "payload": {
                "baseRevision": index as u64,
                "key": format!("key-{index}"),
                "value": index,
            },
        })
        .to_string();
        runtime.receive_json(&packet).unwrap();
    }
    assert_eq!(runtime.state.config_entry_count(), MAX_CONFIG_ENTRIES);

    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": MAX_CONFIG_ENTRIES + 1,
        "lane": "state",
        "kind": "request",
        "type": "state.setConfig",
        "id": "state-overflow",
        "payload": {
            "baseRevision": MAX_CONFIG_ENTRIES as u64,
            "key": "key-overflow",
            "value": true,
        },
    })
    .to_string();
    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.reply_to.as_deref(), Some("state-overflow"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::Backpressure);
    assert!(error.retryable);

    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": MAX_CONFIG_ENTRIES + 2,
        "lane": "state",
        "kind": "request",
        "type": "state.setConfig",
        "id": "state-existing",
        "payload": {
            "baseRevision": MAX_CONFIG_ENTRIES as u64,
            "key": "key-0",
            "value": "updated",
        },
    })
    .to_string();
    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    assert_eq!(response.reply_to.as_deref(), Some("state-existing"));
    let snapshot = response.payload.as_ref().unwrap();
    assert_eq!(snapshot["config"]["key-0"], "updated");
    assert_eq!(snapshot["configRevision"], (MAX_CONFIG_ENTRIES as u64) + 1);
    assert_eq!(runtime.state.config_entry_count(), MAX_CONFIG_ENTRIES);
}

#[test]
fn unknown_param_returns_validation_error() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"param","kind":"request","type":"param.begin","id":"1","payload":{"id":"missing"}}"#)
        .unwrap();
    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Error);
    assert_eq!(
        runtime.transport.sent[0].error.as_ref().unwrap().code,
        BridgeErrorCode::ValidationError
    );
}

#[test]
fn read_only_param_gestures_are_rejected() {
    let mut runtime = runtime();
    runtime.params.get_mut("gain").unwrap().flags.read_only = true;

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"param","kind":"request","type":"param.begin","id":"begin","payload":{"id":"gain"}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"param","kind":"request","type":"param.perform","id":"perform","payload":{"id":"gain","normalized":0.5}}"#)
        .unwrap();

    assert_eq!(runtime.pending_param_gesture_count(), 0);
    assert_eq!(runtime.transport.sent.len(), 2);
    for response in &runtime.transport.sent {
        assert_eq!(response.kind, BridgeKind::Error);
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::PermissionDenied);
        assert_eq!(error.message, "parameter is read only");
    }
}

#[test]
fn read_only_params_still_support_format_and_parse() {
    let mut runtime = runtime();
    runtime.params.get_mut("mode").unwrap().flags.read_only = true;

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"param","kind":"request","type":"param.format","id":"format","payload":{"id":"mode","normalized":0.5}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"param","kind":"request","type":"param.parse","id":"parse","payload":{"id":"mode","text":"Fuzz"}}"#)
        .unwrap();

    assert_eq!(runtime.transport.sent[0].kind, BridgeKind::Response);
    assert_eq!(runtime.transport.sent[0].payload, Some(json!("Drive")));
    assert_eq!(runtime.transport.sent[1].kind, BridgeKind::Response);
    assert_eq!(runtime.transport.sent[1].payload, Some(json!(1.0)));
}

#[test]
fn param_payload_shape_is_validated() {
    let cases = [
        (
            "param.begin",
            None,
            "missing parameter id",
            "begin-missing-payload",
        ),
        (
            "param.begin",
            Some(json!({})),
            "missing parameter id",
            "begin-missing-id",
        ),
        (
            "param.begin",
            Some(json!({ "id": null })),
            "parameter id must be a string",
            "begin-null-id",
        ),
        (
            "param.begin",
            Some(json!({ "id": "" })),
            "parameter id must not be empty",
            "begin-empty-id",
        ),
        (
            "param.begin",
            Some(json!({ "id": "bad\nid" })),
            "parameter id must not contain control characters",
            "begin-control-id",
        ),
        (
            "param.perform",
            Some(json!({ "id": "gain" })),
            "missing normalized value",
            "perform-missing-normalized",
        ),
        (
            "param.perform",
            Some(json!({ "id": "gain", "normalized": null })),
            "normalized value must be a finite number",
            "perform-null-normalized",
        ),
        (
            "param.perform",
            Some(json!({ "id": "gain", "normalized": "0.5" })),
            "normalized value must be a finite number",
            "perform-string-normalized",
        ),
        (
            "param.perform",
            Some(json!({ "id": "gain", "normalized": 0.5, "gestureId": null })),
            "gestureId must be a string",
            "perform-null-gesture",
        ),
        (
            "param.perform",
            Some(json!({ "id": "gain", "normalized": 0.5, "gestureId": 7 })),
            "gestureId must be a string",
            "perform-number-gesture",
        ),
        (
            "param.format",
            Some(json!({ "id": "gain" })),
            "missing normalized value",
            "format-missing-normalized",
        ),
        (
            "param.format",
            Some(json!({ "id": "gain", "normalized": false })),
            "normalized value must be a finite number",
            "format-bool-normalized",
        ),
        (
            "param.parse",
            Some(json!({ "id": "gain" })),
            "missing parameter text",
            "parse-missing-text",
        ),
        (
            "param.parse",
            Some(json!({ "id": "gain", "text": 12.0 })),
            "parameter text must be a string",
            "parse-number-text",
        ),
    ];

    for (packet_type, payload, message, id) in cases {
        let mut runtime = runtime();
        receive_request(&mut runtime, "param", packet_type, id, payload);
        assert_last_validation_error(&runtime, id, message);
        assert_eq!(runtime.pending_param_gesture_count(), 0);
    }
}

#[test]
fn param_gesture_id_shape_is_validated() {
    let cases = [
        (
            "empty-gesture",
            String::new(),
            "gestureId must not be empty",
        ),
        (
            "long-gesture",
            "x".repeat(MAX_PARAM_GESTURE_ID_BYTES + 1),
            "gestureId too long",
        ),
        (
            "control-gesture",
            "drag\n1".to_string(),
            "gestureId must not contain control characters",
        ),
    ];

    for (id, gesture_id, message) in cases {
        let mut runtime = runtime();
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": 1,
            "lane": "param",
            "kind": "request",
            "type": "param.perform",
            "id": id,
            "payload": {
                "id": "gain",
                "normalized": 0.5,
                "gestureId": gesture_id,
            },
        })
        .to_string();

        runtime.receive_json(&packet).unwrap();

        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        assert_eq!(response.reply_to.as_deref(), Some(id));
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::ValidationError);
        assert_eq!(error.message, message);
        assert_eq!(runtime.pending_param_gesture_count(), 0);
    }
}

#[test]
fn reliable_events_require_subscription() {
    let mut runtime = runtime();
    runtime
        .emit_event("state.changed", json!({ "revision": 1 }))
        .unwrap();
    assert!(runtime.transport.sent.is_empty());

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"state.changed"}}"#)
        .unwrap();
    runtime
        .emit_event("state.changed", json!({ "revision": 2 }))
        .unwrap();

    let event = runtime.transport.sent.last().unwrap();
    assert_eq!(event.kind, BridgeKind::Event);
    assert_eq!(event.lane, BridgeLane::Event);
    assert_eq!(event.packet_type, "state.changed");
    assert_eq!(event.payload, Some(json!({ "revision": 2 })));
}

#[test]
fn outbound_event_payload_is_sanitized_before_send() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"state.changed"}}"#)
        .unwrap();
    runtime
        .emit_event(
            "state.changed",
            Value::String("x".repeat(vesty_ipc::MAX_BRIDGE_JSON_STRING_BYTES + 1)),
        )
        .unwrap();

    let event = runtime.transport.sent.last().unwrap();
    assert_eq!(event.kind, BridgeKind::Event);
    assert_eq!(
        event.payload,
        Some(json!({
            "dropped": true,
            "reason": "bridge JSON string too long",
        }))
    );
}

#[test]
fn outbound_event_rejects_invalid_packet_type_topic_before_send() {
    let mut runtime = runtime();
    let invalid_topic = "state.changed\u{7}";
    runtime.subscriptions.subscribe(invalid_topic);

    runtime
        .emit_event(invalid_topic, json!({ "revision": 1 }))
        .unwrap();

    assert!(runtime.transport.sent.is_empty());
}

#[test]
fn param_changed_events_use_subscription_filter_and_revision() {
    let mut runtime = runtime();

    assert!(
        runtime
            .emit_param_changed(
                "gain",
                1.2,
                ParamChangeSource::Ui,
                Some("drag-1".to_string())
            )
            .unwrap()
    );
    assert!(runtime.transport.sent.is_empty());
    assert_eq!(runtime.state.snapshot().revision, 1);
    assert_eq!(runtime.state.snapshot().params_revision, 1);

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"param.changed"}}"#)
        .unwrap();
    assert!(
        runtime
            .emit_param_changed(
                "gain",
                0.5,
                ParamChangeSource::Ui,
                Some("drag-2".to_string())
            )
            .unwrap()
    );

    let event = runtime.transport.sent.last().unwrap();
    assert_eq!(event.kind, BridgeKind::Event);
    assert_eq!(event.lane, BridgeLane::Event);
    assert_eq!(event.packet_type, "param.changed");
    assert_eq!(
        event.payload,
        Some(json!({
            "id": "gain",
            "normalized": 0.5,
            "plain": -24.0,
            "display": "-24.000 dB",
            "source": "ui",
            "gestureId": "drag-2",
            "revision": 2,
        }))
    );
    assert_eq!(runtime.state.snapshot().params_revision, 2);

    assert!(
        !runtime
            .emit_param_changed("missing", 0.5, ParamChangeSource::Host, None)
            .unwrap()
    );
}

#[test]
fn diagnostics_snapshot_reports_bridge_and_fault_state() {
    let mut runtime = runtime();
    perform_hello(&mut runtime);
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"command","kind":"request","type":"bridge.readyAck","id":"ack","payload":{"protocolVersion":1}}"#)
        .unwrap();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":3,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"diagnostics.fault"}}"#)
        .unwrap();
    runtime.set_fault_report(Some(PluginFaultReport {
        faulted: true,
        fault_count: 1,
    }));
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":4,"lane":"command","kind":"request","type":"diagnostics.get","id":"diag"}"#)
        .unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    assert_eq!(response.reply_to.as_deref(), Some("diag"));
    let payload = response.payload.as_ref().unwrap();
    assert_eq!(payload["readyAcknowledged"], true);
    assert_eq!(payload["subscriptionCount"], 1);
    assert_eq!(payload["subscriptions"], json!(["diagnostics.fault"]));
    assert_eq!(payload["droppedParamGestures"], 0);
    assert_eq!(payload["fault"]["faulted"], true);
    assert_eq!(payload["fault"]["faultCount"], 1);
}

#[test]
fn fault_report_events_require_subscription() {
    let mut runtime = runtime();
    runtime
        .emit_fault_report(
            "diagnostics.fault",
            PluginFaultReport {
                faulted: true,
                fault_count: 1,
            },
        )
        .unwrap();
    assert!(runtime.transport.sent.is_empty());

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"diagnostics.fault"}}"#)
        .unwrap();
    runtime
        .emit_fault_report(
            "diagnostics.fault",
            PluginFaultReport {
                faulted: true,
                fault_count: 2,
            },
        )
        .unwrap();

    let event = runtime.transport.sent.last().unwrap();
    assert_eq!(event.kind, BridgeKind::Event);
    assert_eq!(event.lane, BridgeLane::Event);
    assert_eq!(event.packet_type, "diagnostics.fault");
    assert_eq!(
        event.payload,
        Some(json!({
            "faulted": true,
            "faultCount": 2,
        }))
    );
}

#[test]
fn rt_log_events_use_log_lane_and_subscription_filter() {
    let mut runtime = runtime();
    runtime
        .emit_rt_log_event(
            "log.rt",
            1,
            RtLogEvent::QueueOverflow {
                queue: QueueId::Other(9),
                dropped: 3,
            },
        )
        .unwrap();
    assert!(runtime.transport.sent.is_empty());

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"log.rt"}}"#)
        .unwrap();
    runtime
        .emit_rt_log_event(
            "log.rt",
            2,
            RtLogEvent::QueueOverflow {
                queue: QueueId::Other(9),
                dropped: 3,
            },
        )
        .unwrap();

    let event = runtime.transport.sent.last().unwrap();
    assert_eq!(event.kind, BridgeKind::Event);
    assert_eq!(event.lane, BridgeLane::Log);
    assert_eq!(event.packet_type, "log.rt");
    assert_eq!(
        event.payload,
        Some(json!({
            "sequence": 2,
            "level": "warn",
            "kind": "queueOverflow",
            "queue": "other",
            "otherQueueId": 9,
            "dropped": 3,
        }))
    );
}

#[test]
fn rt_log_events_reject_invalid_packet_type_topic_before_send() {
    let mut runtime = runtime();
    let invalid_topic = "log.rt\u{7}";
    runtime.subscriptions.subscribe(invalid_topic);

    runtime
        .emit_rt_log_event(invalid_topic, 1, RtLogEvent::Faulted { code: 7 })
        .unwrap();

    assert!(runtime.transport.sent.is_empty());
}

#[test]
fn subscription_topic_length_is_validated() {
    let mut runtime = runtime();
    let topic = "x".repeat(MAX_SUBSCRIPTION_TOPIC_BYTES + 1);
    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": 1,
        "lane": "command",
        "kind": "request",
        "type": "subscription.add",
        "id": "sub-long",
        "payload": { "topic": topic },
    })
    .to_string();

    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(
        response.error.as_ref().unwrap().code,
        BridgeErrorCode::ValidationError
    );
}

#[test]
fn subscription_topics_reject_empty_and_control_characters() {
    let mut runtime = runtime();
    for (index, (packet_type, topic, expected_message)) in [
        (
            "subscription.add",
            "",
            "subscription topic must not be empty",
        ),
        (
            "subscription.remove",
            "meter.main\u{7}",
            "subscription topic must not contain control characters",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 1,
            "lane": "command",
            "kind": "request",
            "type": packet_type,
            "id": format!("sub-invalid-{index}"),
            "payload": { "topic": topic },
        })
        .to_string();

        runtime.receive_json(&packet).unwrap();

        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::ValidationError);
        assert_eq!(error.message, expected_message);
    }

    assert!(runtime.subscriptions.is_empty());
}

#[test]
fn subscription_topics_reject_missing_or_non_string_payload() {
    let mut runtime = runtime();
    for (index, (payload, expected_message)) in [
        (None, "missing subscription topic"),
        (Some(json!({})), "missing subscription topic"),
        (
            Some(json!({ "topic": null })),
            "subscription topic must be a string",
        ),
        (
            Some(json!({ "topic": 7 })),
            "subscription topic must be a string",
        ),
        (
            Some(json!({ "topic": ["state.changed"] })),
            "subscription topic must be a string",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 1,
            "lane": "command",
            "kind": "request",
            "type": "subscription.add",
            "id": format!("sub-invalid-shape-{index}"),
        });
        if let Some(payload) = payload {
            packet["payload"] = payload;
        }

        runtime.receive_json(&packet.to_string()).unwrap();

        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Error);
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, BridgeErrorCode::ValidationError);
        assert_eq!(error.message, expected_message);
    }

    assert!(runtime.subscriptions.is_empty());
}

#[test]
fn builtin_request_payloads_reject_unexpected_fields() {
    for (index, (lane, packet_type, payload)) in [
        (
            "command",
            "bridge.readyAck",
            json!({ "protocolVersion": 1, "protocol_version": 1 }),
        ),
        (
            "command",
            "subscription.add",
            json!({ "topic": "state.changed", "topci": "state.changed" }),
        ),
        (
            "state",
            "state.setConfig",
            json!({ "baseRevision": 0, "key": "theme", "value": "dark", "values": "dark" }),
        ),
        (
            "state",
            "state.setUiState",
            json!({ "baseRevision": 0, "value": { "panel": "main" }, "uiState": {} }),
        ),
        (
            "param",
            "param.begin",
            json!({ "id": "gain", "gesture": "drag" }),
        ),
        (
            "param",
            "param.perform",
            json!({ "id": "gain", "normalized": 0.5, "normalised": 0.5 }),
        ),
        (
            "param",
            "param.format",
            json!({ "id": "gain", "normalized": 0.5, "text": "0.5" }),
        ),
        (
            "param",
            "param.parse",
            json!({ "id": "gain", "text": "0.5 dB", "normalized": 0.5 }),
        ),
        ("state", "snapshot.get", json!({ "revision": 0 })),
        ("command", "diagnostics.get", json!({ "ready": true })),
        ("meter", "meter.flush", json!({ "topic": "meter.main" })),
        ("event", "event.flush", json!({ "since": 0 })),
    ]
    .into_iter()
    .enumerate()
    {
        let mut runtime = runtime();
        if packet_type == "bridge.readyAck" {
            perform_hello(&mut runtime);
        }
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 10,
            "lane": lane,
            "kind": "request",
            "type": packet_type,
            "id": format!("strict-payload-{index}"),
            "payload": payload,
        });

        runtime.receive_json(&packet.to_string()).unwrap();

        assert_last_validation_error(
            &runtime,
            &format!("strict-payload-{index}"),
            "unexpected request payload field",
        );
    }
}

#[test]
fn empty_builtin_request_payloads_reject_non_object_payloads() {
    for (index, (lane, packet_type, payload)) in [
        ("command", "diagnostics.get", json!("now")),
        ("meter", "meter.flush", json!(["meter.main"])),
        ("event", "event.flush", json!(true)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut runtime = runtime();
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 15,
            "lane": lane,
            "kind": "request",
            "type": packet_type,
            "id": format!("non-object-empty-payload-{index}"),
            "payload": payload,
        });

        runtime.receive_json(&packet.to_string()).unwrap();

        assert_last_validation_error(
            &runtime,
            &format!("non-object-empty-payload-{index}"),
            "unexpected request payload",
        );
    }
}

#[test]
fn empty_builtin_request_payloads_allow_absent_null_and_empty_object() {
    for (index, (lane, packet_type, payload, expected_type)) in [
        ("state", "snapshot.get", None, "snapshot.get.response"),
        (
            "command",
            "diagnostics.get",
            Some(Value::Null),
            "diagnostics.get.response",
        ),
        (
            "meter",
            "meter.flush",
            Some(json!({})),
            "meter.flush.response",
        ),
        (
            "event",
            "event.flush",
            Some(json!({})),
            "event.flush.response",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut runtime = runtime();
        let mut packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 20,
            "lane": lane,
            "kind": "request",
            "type": packet_type,
            "id": format!("empty-payload-{index}"),
        });
        if let Some(payload) = payload {
            packet["payload"] = payload;
        }

        runtime.receive_json(&packet.to_string()).unwrap();

        let response = runtime.transport.sent.last().unwrap();
        assert_eq!(response.kind, BridgeKind::Response);
        assert_eq!(response.packet_type, expected_type);
        assert_eq!(
            response.reply_to.as_deref(),
            Some(format!("empty-payload-{index}").as_str())
        );
    }
}

#[test]
fn subscription_table_full_returns_backpressure() {
    let mut runtime = runtime();
    for index in 0..MAX_SUBSCRIPTIONS {
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 1,
            "lane": "command",
            "kind": "request",
            "type": "subscription.add",
            "id": format!("sub-{index}"),
            "payload": { "topic": format!("topic.{index}") },
        })
        .to_string();
        runtime.receive_json(&packet).unwrap();
    }

    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": MAX_SUBSCRIPTIONS + 1,
        "lane": "command",
        "kind": "request",
        "type": "subscription.add",
        "id": "sub-overflow",
        "payload": { "topic": "topic.overflow" },
    })
    .to_string();
    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.reply_to.as_deref(), Some("sub-overflow"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::Backpressure);
    assert!(error.retryable);
}

#[test]
fn pending_param_gesture_queue_full_returns_backpressure() {
    let mut runtime = runtime();
    for index in 0..MAX_PENDING_PARAM_GESTURES {
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 1,
            "lane": "param",
            "kind": "request",
            "type": "param.begin",
            "id": format!("begin-{index}"),
            "payload": { "id": "gain", "gestureId": format!("drag-{index}") },
        })
        .to_string();
        runtime.receive_json(&packet).unwrap();
    }
    assert_eq!(
        runtime.pending_param_gesture_count(),
        MAX_PENDING_PARAM_GESTURES
    );

    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": MAX_PENDING_PARAM_GESTURES + 1,
        "lane": "param",
        "kind": "request",
        "type": "param.perform",
        "id": "perform-overflow",
        "payload": { "id": "gain", "normalized": 0.25 },
    })
    .to_string();
    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Error);
    assert_eq!(response.reply_to.as_deref(), Some("perform-overflow"));
    let error = response.error.as_ref().unwrap();
    assert_eq!(error.code, BridgeErrorCode::Backpressure);
    assert!(error.retryable);
    assert_eq!(
        runtime.pending_param_gesture_count(),
        MAX_PENDING_PARAM_GESTURES
    );
}

#[test]
fn param_end_is_prioritized_when_gesture_queue_is_full() {
    let mut runtime = runtime();
    let first_perform = json!({
        "v": 1,
        "session": "session",
        "seq": 1,
        "lane": "param",
        "kind": "request",
        "type": "param.perform",
        "id": "perform-to-drop",
        "payload": { "id": "gain", "normalized": 0.5 },
    })
    .to_string();
    runtime.receive_json(&first_perform).unwrap();

    for index in 1..MAX_PENDING_PARAM_GESTURES {
        let packet = json!({
            "v": 1,
            "session": "session",
            "seq": index + 1,
            "lane": "param",
            "kind": "request",
            "type": "param.begin",
            "id": format!("begin-{index}"),
            "payload": { "id": "gain", "gestureId": format!("drag-{index}") },
        })
        .to_string();
        runtime.receive_json(&packet).unwrap();
    }

    let packet = json!({
        "v": 1,
        "session": "session",
        "seq": MAX_PENDING_PARAM_GESTURES + 1,
        "lane": "param",
        "kind": "request",
        "type": "param.end",
        "id": "end-priority",
        "payload": { "id": "gain", "gestureId": "drag-1" },
    })
    .to_string();
    runtime.receive_json(&packet).unwrap();

    let response = runtime.transport.sent.last().unwrap();
    assert_eq!(response.kind, BridgeKind::Response);
    assert_eq!(response.reply_to.as_deref(), Some("end-priority"));
    assert_eq!(
        runtime.pending_param_gesture_count(),
        MAX_PENDING_PARAM_GESTURES
    );

    let gestures = runtime.drain_param_gestures();
    assert_eq!(gestures.len(), MAX_PENDING_PARAM_GESTURES);
    assert_eq!(gestures.last().unwrap().phase, ParamGesturePhase::End);
    assert_eq!(
        gestures.last().unwrap().gesture_id.as_deref(),
        Some("drag-1")
    );
    assert_eq!(
        gestures
            .iter()
            .filter(|gesture| gesture.phase == ParamGesturePhase::Perform)
            .count(),
        0
    );

    let diagnostics = runtime.diagnostics_snapshot();
    assert_eq!(diagnostics.dropped_param_gestures, 1);
}

#[test]
fn latest_meter_events_are_coalesced_until_flush() {
    let mut runtime = runtime();
    assert!(!runtime.queue_latest_meter("meter.main", json!({ "peak": 0.1 })));
    assert_eq!(runtime.pending_latest_meter_count(), 0);

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"meter.main"}}"#)
        .unwrap();
    assert!(runtime.queue_latest_meter("meter.main", json!({ "peak": 0.2 })));
    assert!(runtime.queue_latest_meter("meter.main", json!({ "peak": 0.9 })));
    assert_eq!(runtime.pending_latest_meter_count(), 1);

    let sent_before_flush = runtime.transport.sent.len();
    assert_eq!(runtime.flush_latest_meters().unwrap(), 1);
    assert_eq!(runtime.pending_latest_meter_count(), 0);
    assert_eq!(runtime.transport.sent.len(), sent_before_flush + 1);

    let packet = runtime.transport.sent.last().unwrap();
    assert_eq!(packet.kind, BridgeKind::Event);
    assert_eq!(packet.lane, BridgeLane::Meter);
    assert_eq!(packet.packet_type, "meter.main");
    assert_eq!(packet.payload, Some(json!({ "peak": 0.9 })));
    assert_eq!(packet.flags, vec!["latest".to_string()]);
}

#[test]
fn latest_meter_payload_is_sanitized_before_flush() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"meter.main"}}"#)
        .unwrap();
    assert!(runtime.queue_latest_meter(
        "meter.main",
        Value::String("x".repeat(vesty_ipc::MAX_BRIDGE_JSON_STRING_BYTES + 1)),
    ));
    assert_eq!(runtime.flush_latest_meters().unwrap(), 1);

    let packet = runtime.transport.sent.last().unwrap();
    assert_eq!(packet.kind, BridgeKind::Event);
    assert_eq!(packet.lane, BridgeLane::Meter);
    assert_eq!(
        packet.payload,
        Some(json!({
            "dropped": true,
            "reason": "bridge JSON string too long",
        }))
    );
}

#[test]
fn latest_meter_rejects_invalid_packet_type_topic_before_queueing() {
    let mut runtime = runtime();
    let invalid_topic = "meter.main\u{7}";
    runtime.subscriptions.subscribe(invalid_topic);

    assert!(!runtime.queue_latest_meter(invalid_topic, json!({ "peak": 0.5 })));
    assert_eq!(runtime.pending_latest_meter_count(), 0);
    assert!(runtime.transport.sent.is_empty());
}

#[test]
fn latest_meter_flush_skips_invalid_packet_type_topics() {
    let mut runtime = runtime();
    let invalid_topic = "meter.main\u{7}".to_string();
    runtime.subscriptions.subscribe(invalid_topic.clone());
    runtime
        .latest_meters
        .insert(invalid_topic, json!({ "peak": 0.5 }));

    assert_eq!(runtime.pending_latest_meter_count(), 1);
    assert_eq!(runtime.flush_latest_meters().unwrap(), 0);
    assert_eq!(runtime.pending_latest_meter_count(), 0);
    assert!(runtime.transport.sent.is_empty());
}

#[test]
fn latest_meter_frame_payload_uses_active_channels() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"meter.main"}}"#)
        .unwrap();

    let mut frame = MeterFrame::new(99, 12);
    assert!(frame.set_channel(0, 0.75, 0.5));
    assert!(frame.set_channel(1, 0.25, 0.125));
    assert!(runtime.queue_latest_meter_frame("meter.main", &frame));
    assert_eq!(runtime.flush_latest_meters().unwrap(), 1);

    let packet = runtime.transport.sent.last().unwrap();
    assert_eq!(
        packet.payload,
        Some(json!({
            "idHash": 99,
            "sampleOffset": 12,
            "channels": 2,
            "peaks": [0.75, 0.25],
            "rms": [0.5, 0.125],
        }))
    );
}

#[test]
fn unsubscribe_drops_queued_latest_meter() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"command","kind":"request","type":"subscription.add","id":"sub","payload":{"topic":"meter.main"}}"#)
        .unwrap();
    assert!(runtime.queue_latest_meter("meter.main", json!({ "peak": 0.7 })));
    assert_eq!(runtime.pending_latest_meter_count(), 1);

    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":2,"lane":"command","kind":"request","type":"subscription.remove","id":"unsub","payload":{"topic":"meter.main"}}"#)
        .unwrap();
    assert_eq!(runtime.pending_latest_meter_count(), 0);

    let sent_before_flush = runtime.transport.sent.len();
    assert_eq!(runtime.flush_latest_meters().unwrap(), 0);
    assert_eq!(runtime.transport.sent.len(), sent_before_flush);
}

#[test]
fn meter_flush_request_is_supported() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"meter","kind":"request","type":"meter.flush","id":"flush"}"#)
        .unwrap();

    let packet = runtime.transport.sent.last().unwrap();
    assert_eq!(packet.kind, BridgeKind::Response);
    assert_eq!(packet.reply_to, Some("flush".to_string()));
    assert_eq!(packet.payload, Some(json!({ "queued": 0 })));
}

#[test]
fn event_flush_request_is_supported() {
    let mut runtime = runtime();
    runtime
        .receive_json(r#"{"v":1,"session":"session","seq":1,"lane":"event","kind":"request","type":"event.flush","id":"flush"}"#)
        .unwrap();

    let packet = runtime.transport.sent.last().unwrap();
    assert_eq!(packet.kind, BridgeKind::Response);
    assert_eq!(packet.reply_to, Some("flush".to_string()));
    assert_eq!(
        packet.payload,
        Some(json!({
            "pendingMeterTopics": 0,
            "pendingParamGestures": 0,
        }))
    );
}
