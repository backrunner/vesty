use super::*;
use serde_json::Value;
use std::fs;

#[test]
fn parses_packet() {
    let text =
        r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello"}"#;
    let packet = parse_packet(text).unwrap();
    assert_eq!(packet.packet_type, "bridge.hello");
    assert_eq!(packet.lane, BridgeLane::Command);
}

#[test]
fn bridge_packet_allows_unknown_top_level_fields_for_protocol_extension() {
    let text = r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello","futureField":{"ok":true}}"#;
    let packet = parse_packet(text).unwrap();

    assert_eq!(packet.packet_type, "bridge.hello");
    assert_eq!(packet.session, "s");
}

#[test]
fn bridge_ready_payload_allows_unknown_fields_for_protocol_extension() {
    let payload: BridgeReadyPayload = serde_json::from_value(serde_json::json!({
        "protocolVersion": BRIDGE_PROTOCOL_VERSION,
        "instanceId": "instance",
        "editorSessionId": "editor",
        "devMode": true,
        "pluginName": "Test",
        "vendor": "Vesty",
        "hostExtensions": { "canPinEditor": true },
        "capabilities": {
            "paramGestures": true,
            "paramFormatParse": true,
            "stateConfig": true,
            "subscriptions": true,
            "meterStream": true,
            "reliableEvents": true,
            "diagnostics": true,
            "vendorDiagnostics": true
        },
        "params": [
            {
                "id": "gain",
                "name": "Gain",
                "kind": { "float": { "min": -60.0, "max": 12.0, "displayHint": "decibel" } },
                "defaultNormalized": 0.5,
                "unit": "dB",
                "stepCount": null,
                "flags": {
                    "automatable": true,
                    "bypass": false,
                    "readOnly": false,
                    "programChange": false,
                    "vendorVisible": true
                },
                "midiMappings": [
                    { "controller": 7, "channel": null, "source": "host" }
                ]
            }
        ],
        "paramValues": [
            { "id": "gain", "normalized": 0.25, "source": "host" }
        ],
        "snapshot": {
            "revision": 1,
            "paramsRevision": 2,
            "configRevision": 3,
            "uiRevision": 4,
            "config": {},
            "uiState": {},
            "vendorState": { "theme": "dark" }
        }
    }))
    .unwrap();

    assert_eq!(payload.protocol_version, BRIDGE_PROTOCOL_VERSION);
    assert_eq!(payload.editor_session_id, "editor");
    assert!(payload.capabilities.diagnostics);
    assert_eq!(payload.params[0].id.as_str(), "gain");
    assert_eq!(payload.params[0].midi_mappings[0].controller, 7);
    assert_eq!(
        payload.param_values,
        vec![ParamValueSnapshot {
            id: "gain".to_owned(),
            normalized: 0.25,
        }]
    );
    assert_eq!(payload.snapshot.revision, 1);
}

#[test]
fn bridge_hello_payload_allows_unknown_fields_for_protocol_extension() {
    let payload: BridgeHelloPayload = serde_json::from_value(serde_json::json!({
        "supportedProtocolVersions": [BRIDGE_PROTOCOL_VERSION],
        "jsPackageVersion": "0.1.0",
        "pageUrl": "vesty://assets/index.html",
        "jsCapabilities": {
            "supportsDocking": true
        }
    }))
    .unwrap();

    assert!(payload.supports_protocol(BRIDGE_PROTOCOL_VERSION));
    assert_eq!(payload.js_package_version.as_deref(), Some("0.1.0"));
    assert!(payload.has_valid_shape());
}

#[test]
fn validates_packet_type_and_sanitizes_invalid_reply_type() {
    assert_eq!(
        validate_packet_type("").unwrap_err(),
        "request type must not be empty"
    );
    assert_eq!(
        validate_packet_type(&"x".repeat(MAX_BRIDGE_PACKET_TYPE_BYTES + 1)).unwrap_err(),
        "request type too long"
    );
    assert_eq!(
        validate_packet_type("bad\u{7}type").unwrap_err(),
        "request type must not contain control characters"
    );
    assert!(validate_packet_type("bridge.hello").is_ok());

    let packet = BridgePacket::request("s", 1, BridgeLane::Command, "bad\u{7}type");
    let response = packet.error_to(
        2,
        BridgeErrorPayload::new(BridgeErrorCode::ValidationError, "bad type", false),
    );

    assert_eq!(response.packet_type, "bridge.invalidType.error");
    assert_eq!(response.reply_to, None);
    assert!(!response.packet_type.contains('\u{7}'));
}

#[test]
fn validates_bridge_packet_id_and_sanitizes_invalid_reply_id() {
    assert_eq!(
        validate_bridge_packet_id("").unwrap_err(),
        "request id must not be empty"
    );
    assert_eq!(
        validate_bridge_packet_id(&"x".repeat(MAX_BRIDGE_PACKET_ID_BYTES + 1)).unwrap_err(),
        "request id too long"
    );
    assert_eq!(
        validate_bridge_packet_id("bad\u{7}id").unwrap_err(),
        "request id must not contain control characters"
    );
    assert!(validate_bridge_packet_id("bridge-request-1").is_ok());

    let mut packet = BridgePacket::request("s", 1, BridgeLane::Command, "bridge.hello");
    packet.id = Some("bad\u{7}id".to_string());
    let response = packet.response_to(2, None);
    assert_eq!(response.reply_to, None);

    packet.id = Some("x".repeat(MAX_BRIDGE_PACKET_ID_BYTES + 1));
    let response = packet.error_to(
        3,
        BridgeErrorPayload::new(BridgeErrorCode::ValidationError, "bad request", false),
    );
    assert_eq!(response.reply_to, None);

    packet.id = Some("ok-id".to_string());
    let response = packet.error_to(
        4,
        BridgeErrorPayload::new(BridgeErrorCode::ValidationError, "bad request", false),
    );
    assert_eq!(response.reply_to.as_deref(), Some("ok-id"));
}

#[test]
fn validates_bridge_packet_flags() {
    assert!(validate_bridge_packet_flags(&[]).is_ok());
    assert!(validate_bridge_packet_flags(&["latest".to_string()]).is_ok());
    assert_eq!(
        validate_bridge_packet_flags(&vec!["flag".to_string(); MAX_BRIDGE_PACKET_FLAGS + 1])
            .unwrap_err(),
        "bridge packet has too many flags"
    );
    assert_eq!(
        validate_bridge_packet_flags(&["".to_string()]).unwrap_err(),
        "bridge packet flag must not be empty"
    );
    assert_eq!(
        validate_bridge_packet_flags(&["x".repeat(MAX_BRIDGE_PACKET_FLAG_BYTES + 1)]).unwrap_err(),
        "bridge packet flag too long"
    );
    assert_eq!(
        validate_bridge_packet_flags(&["bad\u{7}flag".to_string()]).unwrap_err(),
        "bridge packet flag must not contain control characters"
    );
}

#[test]
fn validates_and_advances_bridge_packet_seq() {
    assert!(validate_bridge_packet_seq(0).is_ok());
    assert!(validate_bridge_packet_seq(1).is_ok());
    assert!(validate_bridge_packet_seq(MAX_BRIDGE_PACKET_SEQ).is_ok());
    assert_eq!(
        validate_bridge_packet_seq(MAX_BRIDGE_PACKET_SEQ + 1).unwrap_err(),
        "bridge packet seq exceeds JavaScript safe integer range"
    );
    assert_eq!(advance_bridge_packet_seq(1), 2);
    assert_eq!(
        advance_bridge_packet_seq(MAX_BRIDGE_PACKET_SEQ - 1),
        MAX_BRIDGE_PACKET_SEQ
    );
    assert_eq!(advance_bridge_packet_seq(MAX_BRIDGE_PACKET_SEQ), 1);
    assert_eq!(advance_bridge_packet_seq(MAX_BRIDGE_PACKET_SEQ + 1), 1);
}

#[test]
fn validates_bridge_error_message() {
    assert!(validate_bridge_error_message("").is_ok());
    assert!(validate_bridge_error_message("failed to parse bridge packet").is_ok());
    assert_eq!(
        validate_bridge_error_message(&"x".repeat(MAX_BRIDGE_ERROR_MESSAGE_BYTES + 1)).unwrap_err(),
        "bridge error message too long"
    );
    assert_eq!(
        validate_bridge_error_message("bad\u{7}error").unwrap_err(),
        "bridge error message must not contain control characters"
    );
}

#[test]
fn bridge_error_payload_new_sanitizes_message() {
    let clean = BridgeErrorPayload::new(
        BridgeErrorCode::ValidationError,
        "failed to parse bridge packet",
        false,
    );
    assert_eq!(clean.message, "failed to parse bridge packet");
    assert!(validate_bridge_error_message(&clean.message).is_ok());

    let control = BridgeErrorPayload::new(
        BridgeErrorCode::ValidationError,
        "bad\u{7}error\nmessage",
        false,
    );
    assert_eq!(control.message, "bad error message");
    assert!(validate_bridge_error_message(&control.message).is_ok());

    let long = BridgeErrorPayload::new(
        BridgeErrorCode::InternalError,
        "x".repeat(MAX_BRIDGE_ERROR_MESSAGE_BYTES + 1),
        true,
    );
    assert_eq!(long.message.len(), MAX_BRIDGE_ERROR_MESSAGE_BYTES);
    assert!(validate_bridge_error_message(&long.message).is_ok());

    let multibyte = BridgeErrorPayload::new(
        BridgeErrorCode::InternalError,
        format!("{}界", "x".repeat(MAX_BRIDGE_ERROR_MESSAGE_BYTES - 1)),
        true,
    );
    assert_eq!(multibyte.message.len(), MAX_BRIDGE_ERROR_MESSAGE_BYTES - 1);
    assert!(multibyte.message.ends_with('x'));
    assert!(validate_bridge_error_message(&multibyte.message).is_ok());
}

#[test]
fn validates_and_sanitizes_bridge_json_values() {
    let valid = serde_json::json!({
        "snapshot": {
            "revision": 1,
            "uiState": {
                "panel": "advanced"
            }
        }
    });
    assert!(validate_bridge_json_value(&valid).is_ok());
    assert_eq!(sanitize_bridge_json_value(valid.clone()), valid);

    let long_string = Value::String("x".repeat(MAX_BRIDGE_JSON_STRING_BYTES + 1));
    assert_eq!(
        validate_bridge_json_value(&long_string).unwrap_err(),
        "bridge JSON string too long"
    );
    assert_eq!(
        sanitize_bridge_json_value(long_string),
        serde_json::json!({
            "dropped": true,
            "reason": "bridge JSON string too long",
        })
    );

    let mut too_deep = Value::Null;
    for _ in 0..=MAX_BRIDGE_JSON_DEPTH {
        too_deep = Value::Array(vec![too_deep]);
    }
    assert_eq!(
        validate_bridge_json_value(&too_deep).unwrap_err(),
        "bridge JSON value exceeds maximum depth"
    );
}

#[test]
fn bridge_packet_error_to_sanitizes_error_details() {
    let packet = BridgePacket::request("s", 1, BridgeLane::State, "state.setConfig");
    let valid_details = serde_json::json!({ "snapshot": { "revision": 1 } });
    let response = packet.error_to(
        2,
        BridgeErrorPayload::new(BridgeErrorCode::StateConflict, "conflict", true)
            .with_details(valid_details.clone()),
    );
    assert_eq!(
        response.error.as_ref().unwrap().details.as_ref(),
        Some(&valid_details)
    );

    let mut error =
        BridgeErrorPayload::new(BridgeErrorCode::InternalError, "too much detail", true);
    error.details = Some(Value::String("x".repeat(MAX_BRIDGE_JSON_STRING_BYTES + 1)));
    let response = packet.error_to(3, error);
    assert_eq!(
        response.error.as_ref().unwrap().details.as_ref(),
        Some(&serde_json::json!({
            "dropped": true,
            "reason": "bridge JSON string too long",
        }))
    );
}

#[test]
fn bridge_packet_response_to_sanitizes_payload() {
    let packet = BridgePacket::request("s", 1, BridgeLane::State, "snapshot.get");
    let valid_payload = serde_json::json!({ "snapshot": { "revision": 1 } });
    let response = packet.response_to(2, Some(valid_payload.clone()));
    assert_eq!(response.payload.as_ref(), Some(&valid_payload));

    let response = packet.response_to(
        3,
        Some(Value::String("x".repeat(MAX_BRIDGE_JSON_STRING_BYTES + 1))),
    );
    assert_eq!(
        response.payload.as_ref(),
        Some(&serde_json::json!({
            "dropped": true,
            "reason": "bridge JSON string too long",
        }))
    );
}

#[test]
fn validates_hello_payload_shape() {
    let payload = BridgeHelloPayload {
        supported_protocol_versions: vec![BRIDGE_PROTOCOL_VERSION],
        js_package_version: Some("0.1.0".to_string()),
        page_url: Some("vesty://assets/index.html".to_string()),
    };
    assert!(payload.supports_protocol(BRIDGE_PROTOCOL_VERSION));
    assert!(payload.has_valid_shape());

    let empty_versions = BridgeHelloPayload {
        supported_protocol_versions: Vec::new(),
        js_package_version: None,
        page_url: None,
    };
    assert!(!empty_versions.has_valid_shape());

    let long_page_url = BridgeHelloPayload {
        supported_protocol_versions: vec![BRIDGE_PROTOCOL_VERSION],
        js_package_version: None,
        page_url: Some("x".repeat(MAX_HELLO_PAGE_URL_BYTES + 1)),
    };
    assert!(!long_page_url.has_valid_shape());
}

#[test]
fn rejects_messages_over_absolute_limit() {
    let text = " ".repeat(MAX_STATE_MESSAGE_BYTES + 1);
    assert!(matches!(
        parse_packet(&text),
        Err(IpcError::MessageTooLarge {
            len,
            max: MAX_STATE_MESSAGE_BYTES
        }) if len == MAX_STATE_MESSAGE_BYTES + 1
    ));
}

#[test]
fn exports_protocol_types_and_json_schema() {
    let temp = tempfile::tempdir().unwrap();
    let report = export_protocol_bindings(temp.path()).unwrap();

    assert!(report.typescript_files >= 16);
    assert_eq!(report.json_schema_files, 7);
    assert!(
        report
            .json_schema_dir
            .join("BridgeReadyPayload.schema.json")
            .is_file()
    );

    let packet = fs::read_to_string(
        report
            .typescript_dir
            .join("protocol")
            .join("BridgePacket.ts"),
    )
    .unwrap();
    assert!(packet.contains("seq: number"));
    assert!(packet.contains("id?: string"));
    assert!(packet.contains("payload?: JsonValue"));
    assert!(packet.contains("flags?: Array<string>"));

    let ready = fs::read_to_string(
        report
            .typescript_dir
            .join("protocol")
            .join("BridgeReadyPayload.ts"),
    )
    .unwrap();
    assert!(ready.contains("params: Array<ParamSpec>"));
    assert!(ready.contains("paramValues: Array<ParamValueSnapshot>"));

    let param_value = fs::read_to_string(
        report
            .typescript_dir
            .join("protocol")
            .join("ParamValueSnapshot.ts"),
    )
    .unwrap();
    assert!(param_value.contains("id: string"));
    assert!(param_value.contains("normalized: number"));

    let ready_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            report
                .json_schema_dir
                .join("BridgeReadyPayload.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        ready_schema.get("additionalProperties"),
        None,
        "BridgeReadyPayload top-level schema should stay extension-friendly"
    );

    let hello_schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            report
                .json_schema_dir
                .join("BridgeHelloPayload.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        hello_schema.get("additionalProperties"),
        None,
        "BridgeHelloPayload top-level schema should stay extension-friendly"
    );

    let param_spec =
        fs::read_to_string(report.typescript_dir.join("protocol").join("ParamSpec.ts")).unwrap();
    assert!(param_spec.contains("defaultNormalized: number"));
    assert!(param_spec.contains("stepCount: number | null"));
    assert!(!param_spec.contains("default_normalized"));
    assert!(!param_spec.contains("step_count"));

    let param_kind =
        fs::read_to_string(report.typescript_dir.join("protocol").join("ParamKind.ts")).unwrap();
    assert!(param_kind.contains("\"float\""));
    assert!(param_kind.contains("\"bool\""));
    assert!(param_kind.contains("\"choice\""));
}
