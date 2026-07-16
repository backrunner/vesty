use super::*;

#[cfg(feature = "wry-ui")]
#[test]
fn controller_wry_bridge_returns_native_ipc_validation_errors() {
    let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
    let endpoint = controller.bridge_endpoint();
    let bridge = endpoint.bridge_handler();

    let hello = serde_json::json!({
        "v": 1,
        "session": "pending",
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
    })
    .to_string();
    let hello_packets = bridge(hello);
    let session = hello_packets[0]
        .payload
        .as_ref()
        .and_then(|payload| payload.get("editorSessionId"))
        .and_then(serde_json::Value::as_str)
        .expect("editor session")
        .to_string();

    let non_string_type = serde_json::json!({
        "v": 1,
        "session": session,
        "seq": 2,
        "lane": "command",
        "kind": "request",
        "type": 42,
        "id": "bad-type",
    })
    .to_string();
    let parse_packets = bridge(non_string_type);
    assert_eq!(parse_packets.len(), 1);
    assert_eq!(parse_packets[0].packet_type, "bridge.parseError.error");
    assert_eq!(parse_packets[0].reply_to.as_deref(), Some("bad-type"));
    let parse_error = parse_packets[0].error.as_ref().unwrap();
    assert_eq!(parse_error.code, BridgeErrorCode::ParseError);
    assert_eq!(parse_error.message, "failed to parse bridge packet");
    assert!(!parse_error.retryable);

    let control_type = serde_json::json!({
        "v": 1,
        "session": session,
        "seq": 3,
        "lane": "command",
        "kind": "request",
        "type": "bridge.hello\u{7}",
        "id": "control-type",
        "payload": {
            "supportedProtocolVersions": [1],
            "jsPackageVersion": "test",
            "pageUrl": "vesty://assets/index.html",
        },
    })
    .to_string();
    let validation_packets = bridge(control_type);
    assert_eq!(validation_packets.len(), 1);
    assert_eq!(
        validation_packets[0].packet_type,
        "bridge.invalidType.error"
    );
    assert_eq!(
        validation_packets[0].reply_to.as_deref(),
        Some("control-type")
    );
    let validation_error = validation_packets[0].error.as_ref().unwrap();
    assert_eq!(validation_error.code, BridgeErrorCode::ValidationError);
    assert_eq!(
        validation_error.message,
        "request type must not contain control characters"
    );
    assert!(!validation_error.retryable);
}

#[cfg(feature = "wry-ui")]
#[test]
fn controller_wry_bridge_reports_host_rejection_without_mutating_param() {
    // SAFETY: Test code wires a fake component handler into the controller COM callback.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
        let handler = ComWrapper::new(FakeComponentHandler::rejecting_perform());
        let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
        assert_eq!(
            controller.setComponentHandler(handler_ptr.as_ptr()),
            kResultOk
        );
        let endpoint = controller.bridge_endpoint();
        let bridge = endpoint.bridge_handler();

        let hello = serde_json::json!({
            "v": 1,
            "session": "pending",
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
        })
        .to_string();
        let hello_packets = bridge(hello);
        assert_eq!(
            hello_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
            0.5
        );
        let session = hello_packets[0]
            .payload
            .as_ref()
            .and_then(|payload| payload.get("editorSessionId"))
            .and_then(serde_json::Value::as_str)
            .expect("editor session");

        let perform = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 2,
            "lane": "param",
            "kind": "request",
            "type": "param.perform",
            "id": "perform-rejected",
            "payload": { "id": "gain", "normalized": 0.75 },
        })
        .to_string();
        let packets = bridge(perform);

        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].kind, BridgeKind::Error);
        assert_eq!(packets[0].reply_to.as_deref(), Some("perform-rejected"));
        assert_eq!(
            packets[0].error.as_ref().map(|error| error.code.clone()),
            Some(BridgeErrorCode::HostRejected)
        );
        assert_eq!(controller.getParamNormalized(gain_id), 0.5);
        assert_eq!(handler.calls(), vec![HandlerCall::Perform(gain_id, 0.75)]);
    }
}

#[cfg(feature = "wry-ui")]
#[test]
fn controller_wry_bridge_emits_param_changed_after_ui_perform() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
        let handler = ComWrapper::new(FakeComponentHandler::default());
        let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
        assert_eq!(
            controller.setComponentHandler(handler_ptr.as_ptr()),
            kResultOk
        );

        let endpoint = controller.bridge_endpoint();
        let bridge = endpoint.bridge_handler();

        let hello = serde_json::json!({
            "v": 1,
            "session": "pending",
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
        })
        .to_string();
        let hello_packets = bridge(hello);
        assert_eq!(
            hello_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
            0.5
        );
        let session = hello_packets[0]
            .payload
            .as_ref()
            .and_then(|payload| payload.get("editorSessionId"))
            .and_then(serde_json::Value::as_str)
            .expect("editor session")
            .to_string();

        let subscribe = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 2,
            "lane": "command",
            "kind": "request",
            "type": "subscription.add",
            "id": "sub-param",
            "payload": { "topic": "param.changed" },
        })
        .to_string();
        let subscribe_packets = bridge(subscribe);
        assert_eq!(subscribe_packets.len(), 1);
        assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-param"));

        let begin = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 3,
            "lane": "param",
            "kind": "request",
            "type": "param.begin",
            "id": "begin",
            "payload": { "id": "gain", "gestureId": "drag-1" },
        })
        .to_string();
        let begin_packets = bridge(begin);
        assert_eq!(begin_packets.len(), 1);
        assert_eq!(begin_packets[0].reply_to.as_deref(), Some("begin"));

        let perform = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 4,
            "lane": "param",
            "kind": "request",
            "type": "param.perform",
            "id": "perform",
            "payload": {
                "id": "gain",
                "normalized": 0.75,
                "gestureId": "drag-1",
            },
        })
        .to_string();
        let perform_packets = bridge(perform);
        assert_eq!(perform_packets.len(), 2);
        assert_eq!(perform_packets[0].reply_to.as_deref(), Some("perform"));

        let event = perform_packets
            .iter()
            .find(|packet| packet.packet_type == "param.changed")
            .expect("param changed event");
        assert_eq!(event.payload.as_ref().unwrap()["id"], "gain");
        assert_eq!(event.payload.as_ref().unwrap()["normalized"], 0.75);
        assert_eq!(event.payload.as_ref().unwrap()["plain"], 1.5);
        assert_eq!(event.payload.as_ref().unwrap()["display"], "1.500");
        assert_eq!(event.payload.as_ref().unwrap()["source"], "ui");
        assert_eq!(event.payload.as_ref().unwrap()["gestureId"], "drag-1");
        assert_eq!(event.payload.as_ref().unwrap()["revision"], 1);

        assert_eq!(
            handler.calls(),
            vec![
                HandlerCall::Begin(gain_id),
                HandlerCall::Perform(gain_id, 0.75)
            ]
        );
    }
}

#[cfg(feature = "wry-ui")]
#[test]
fn controller_wry_bridge_emits_host_param_changes_on_event_flush() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
        assert_eq!(controller.setParamNormalized(gain_id, 0.25), kResultOk);

        let endpoint = controller.bridge_endpoint();
        let bridge = endpoint.bridge_handler();

        let hello = serde_json::json!({
            "v": 1,
            "session": "pending",
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
        })
        .to_string();
        let hello_packets = bridge(hello);
        assert_eq!(
            hello_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
            0.25
        );
        let session = hello_packets[0]
            .payload
            .as_ref()
            .and_then(|payload| payload.get("editorSessionId"))
            .and_then(serde_json::Value::as_str)
            .expect("editor session")
            .to_string();

        let subscribe = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 2,
            "lane": "command",
            "kind": "request",
            "type": "subscription.add",
            "id": "sub-param",
            "payload": { "topic": "param.changed" },
        })
        .to_string();
        let subscribe_packets = bridge(subscribe);
        assert_eq!(subscribe_packets.len(), 2);
        assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-param"));
        let catch_up = subscribe_packets
            .iter()
            .find(|packet| packet.packet_type == "param.changed")
            .expect("catch-up param changed event");
        assert_eq!(catch_up.payload.as_ref().unwrap()["id"], "gain");
        assert_eq!(catch_up.payload.as_ref().unwrap()["normalized"], 0.25);
        assert_eq!(catch_up.payload.as_ref().unwrap()["plain"], 0.5);
        assert_eq!(catch_up.payload.as_ref().unwrap()["source"], "host");

        assert_eq!(controller.setParamNormalized(gain_id, 0.5), kResultOk);
        let flush = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 3,
            "lane": "event",
            "kind": "request",
            "type": "event.flush",
            "id": "flush",
        })
        .to_string();
        let flush_packets = bridge(flush);
        assert_eq!(flush_packets.len(), 2);
        assert_eq!(flush_packets[0].reply_to.as_deref(), Some("flush"));

        let event = flush_packets
            .iter()
            .find(|packet| packet.packet_type == "param.changed")
            .expect("flushed param changed event");
        assert_eq!(event.payload.as_ref().unwrap()["id"], "gain");
        assert_eq!(event.payload.as_ref().unwrap()["normalized"], 0.5);
        assert_eq!(event.payload.as_ref().unwrap()["plain"], 1.0);
        assert_eq!(event.payload.as_ref().unwrap()["display"], "1.000");
        assert_eq!(event.payload.as_ref().unwrap()["source"], "host");
        assert_eq!(
            event.payload.as_ref().unwrap()["gestureId"],
            serde_json::Value::Null
        );

        let state_input = ComWrapper::new(MemoryStream::with_bytes(raw_state_bytes(
            serde_json::json!({
                "version": 1,
                "params": [{ "id": "gain", "normalized": 0.75 }],
            }),
        )));
        let state_input_ptr = state_input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(controller.setState(state_input_ptr.as_ptr()), kResultOk);
        let flush_state = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 4,
            "lane": "event",
            "kind": "request",
            "type": "event.flush",
            "id": "flush-state",
        })
        .to_string();
        let state_packets = bridge(flush_state);
        assert_eq!(state_packets[0].reply_to.as_deref(), Some("flush-state"));
        let state_event = state_packets
            .iter()
            .find(|packet| {
                packet.packet_type == "param.changed"
                    && packet
                        .payload
                        .as_ref()
                        .is_some_and(|payload| payload["id"] == "gain")
            })
            .expect("state param changed event");
        assert_eq!(state_event.payload.as_ref().unwrap()["normalized"], 0.75);
        assert_eq!(state_event.payload.as_ref().unwrap()["source"], "state");

        let reopened = controller.bridge_endpoint().bridge_handler();
        let reopened_hello = serde_json::json!({
            "v": 1,
            "session": "pending",
            "seq": 1,
            "lane": "command",
            "kind": "request",
            "type": "bridge.hello",
            "id": "reopened-hello",
            "payload": { "supportedProtocolVersions": [1] },
        })
        .to_string();
        let reopened_packets = reopened(reopened_hello);
        assert_eq!(
            reopened_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
            0.75
        );
    }
}

#[cfg(feature = "wry-ui")]
#[test]
fn controller_wry_bridge_marks_program_param_changes() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<MidiMappedPlugin>::new();
        let gain_id = test_param_id("gain");
        let cutoff_id = test_param_id("cutoff");
        let pitch_id = test_param_id("pitch");

        let endpoint = controller.bridge_endpoint();
        let bridge = endpoint.bridge_handler();

        let hello = serde_json::json!({
            "v": 1,
            "session": "pending",
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
        })
        .to_string();
        let hello_packets = bridge(hello);
        let session = hello_packets[0]
            .payload
            .as_ref()
            .and_then(|payload| payload.get("editorSessionId"))
            .and_then(serde_json::Value::as_str)
            .expect("editor session")
            .to_string();

        let subscribe = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 2,
            "lane": "command",
            "kind": "request",
            "type": "subscription.add",
            "id": "sub-param",
            "payload": { "topic": "param.changed" },
        })
        .to_string();
        let subscribe_packets = bridge(subscribe);
        assert_eq!(subscribe_packets.len(), 1);
        assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-param"));

        assert_eq!(
            controller.setUnitProgramData(77, 1, ptr::null_mut()),
            kResultOk
        );
        let flush_program = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 3,
            "lane": "event",
            "kind": "request",
            "type": "event.flush",
            "id": "flush-program",
        })
        .to_string();
        let program_packets = bridge(flush_program);
        assert_eq!(
            program_packets[0].reply_to.as_deref(),
            Some("flush-program")
        );
        let program_event = program_packets
            .iter()
            .find(|packet| {
                packet.packet_type == "param.changed"
                    && packet
                        .payload
                        .as_ref()
                        .is_some_and(|payload| payload["id"] == "gain")
            })
            .expect("program param changed event");
        let program_normalized = program_event.payload.as_ref().unwrap()["normalized"]
            .as_f64()
            .expect("program normalized");
        assert!((program_normalized - 0.8).abs() < 0.000_001);
        assert_eq!(program_event.payload.as_ref().unwrap()["source"], "program");
        assert_eq!(
            program_event.payload.as_ref().unwrap()["gestureId"],
            serde_json::Value::Null
        );

        assert_eq!(controller.setParamNormalized(gain_id, 0.11), kResultOk);
        assert_eq!(controller.setParamNormalized(cutoff_id, 0.22), kResultOk);
        assert_eq!(controller.setParamNormalized(pitch_id, 0.33), kResultOk);
        let drain_host_changes = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 4,
            "lane": "event",
            "kind": "request",
            "type": "event.flush",
            "id": "drain-host",
        })
        .to_string();
        let drain_packets = bridge(drain_host_changes);
        assert_eq!(drain_packets[0].reply_to.as_deref(), Some("drain-host"));

        let program_id = test_param_id("program");
        assert_eq!(controller.setParamNormalized(program_id, 1.0), kResultOk);
        let flush_program_param = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 5,
            "lane": "event",
            "kind": "request",
            "type": "event.flush",
            "id": "flush-program-param",
        })
        .to_string();
        let program_param_packets = bridge(flush_program_param);
        assert_eq!(
            program_param_packets[0].reply_to.as_deref(),
            Some("flush-program-param")
        );
        let program_param_event = program_param_packets
            .iter()
            .find(|packet| {
                packet.packet_type == "param.changed"
                    && packet
                        .payload
                        .as_ref()
                        .is_some_and(|payload| payload["id"] == "gain")
            })
            .expect("program-change param changed event");
        let program_param_normalized = program_param_event.payload.as_ref().unwrap()["normalized"]
            .as_f64()
            .expect("program-change normalized");
        assert!((program_param_normalized - 0.3).abs() < 0.000_001);
        assert_eq!(
            program_param_event.payload.as_ref().unwrap()["source"],
            "program"
        );

        let input = ComWrapper::new(MemoryStream::with_bytes(raw_program_data_bytes(
            77,
            1,
            serde_json::json!({
                "gain": 0.42,
                "cutoff": 0.84,
                "pitch": 0.21,
            }),
        )));
        let input_ptr = input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            controller.setProgramData(77, 1, input_ptr.as_ptr()),
            kResultOk
        );
        let flush_program_data = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 6,
            "lane": "event",
            "kind": "request",
            "type": "event.flush",
            "id": "flush-program-data",
        })
        .to_string();
        let program_data_packets = bridge(flush_program_data);
        assert_eq!(
            program_data_packets[0].reply_to.as_deref(),
            Some("flush-program-data")
        );
        let program_data_event = program_data_packets
            .iter()
            .find(|packet| {
                packet.packet_type == "param.changed"
                    && packet
                        .payload
                        .as_ref()
                        .is_some_and(|payload| payload["id"] == "gain")
            })
            .expect("program data param changed event");
        let program_data_normalized = program_data_event.payload.as_ref().unwrap()["normalized"]
            .as_f64()
            .expect("program data normalized");
        assert!((program_data_normalized - 0.42).abs() < 0.000_001);
        assert_eq!(
            program_data_event.payload.as_ref().unwrap()["source"],
            "program"
        );
    }
}

#[cfg(feature = "wry-ui")]
#[test]
fn controller_wry_bridge_state_roundtrips_through_vst3_state() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let endpoint = controller.bridge_endpoint();
        let bridge = endpoint.bridge_handler();

        let hello = serde_json::json!({
            "v": 1,
            "session": "pending",
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
        })
        .to_string();
        let hello_packets = bridge(hello);
        let session = hello_packets[0]
            .payload
            .as_ref()
            .and_then(|payload| payload.get("editorSessionId"))
            .and_then(serde_json::Value::as_str)
            .expect("editor session")
            .to_string();

        let set_config = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 2,
            "lane": "state",
            "kind": "request",
            "type": "state.setConfig",
            "id": "set-config",
            "payload": {
                "baseRevision": 0,
                "key": "theme",
                "value": "dark",
            },
        })
        .to_string();
        let config_packets = bridge(set_config);
        assert_eq!(config_packets[0].reply_to.as_deref(), Some("set-config"));

        let set_ui = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 3,
            "lane": "state",
            "kind": "request",
            "type": "state.setUiState",
            "id": "set-ui",
            "payload": {
                "baseRevision": 0,
                "value": { "panel": "advanced" },
            },
        })
        .to_string();
        let ui_packets = bridge(set_ui);
        assert_eq!(ui_packets[0].reply_to.as_deref(), Some("set-ui"));

        let saved = ComWrapper::new(MemoryStream::default());
        let saved_ptr = saved.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(controller.getState(saved_ptr.as_ptr()), kResultOk);
        let saved_bytes = saved.bytes();
        let saved_text = String::from_utf8_lossy(&saved_bytes);
        assert!(saved_text.contains(r#""bridge""#));
        assert!(saved_text.contains(r#""uiState""#));
        assert!(saved_text.contains("advanced"));

        let restored = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes));
        let input_ptr = input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(restored.setState(input_ptr.as_ptr()), kResultOk);

        let restored_endpoint = restored.bridge_endpoint();
        let restored_bridge = restored_endpoint.bridge_handler();
        let restored_hello = serde_json::json!({
            "v": 1,
            "session": "pending",
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
        })
        .to_string();
        let restored_packets = restored_bridge(restored_hello);
        let snapshot = restored_packets[0]
            .payload
            .as_ref()
            .and_then(|payload| payload.get("snapshot"))
            .expect("ready snapshot");
        assert_eq!(snapshot["revision"], 2);
        assert_eq!(snapshot["configRevision"], 1);
        assert_eq!(snapshot["uiRevision"], 1);
        assert_eq!(snapshot["config"]["theme"], "dark");
        assert_eq!(snapshot["uiState"]["panel"], "advanced");
    }
}

#[cfg(feature = "wry-ui")]
#[test]
fn controller_wry_bridge_syncs_state_restore_to_active_ui_runtime() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
        let endpoint = controller.bridge_endpoint();
        let bridge = endpoint.bridge_handler();

        let hello = serde_json::json!({
            "v": 1,
            "session": "pending",
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
        })
        .to_string();
        let hello_packets = bridge(hello);
        let session = hello_packets[0]
            .payload
            .as_ref()
            .and_then(|payload| payload.get("editorSessionId"))
            .and_then(serde_json::Value::as_str)
            .expect("editor session")
            .to_string();

        let subscribe = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 2,
            "lane": "command",
            "kind": "request",
            "type": "subscription.add",
            "id": "sub-state",
            "payload": { "topic": "state.changed" },
        })
        .to_string();
        let subscribe_packets = bridge(subscribe);
        assert_eq!(subscribe_packets.len(), 1);
        assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-state"));

        let state_input = ComWrapper::new(MemoryStream::with_bytes(raw_state_bytes(
            serde_json::json!({
                "version": 1,
                "params": [{ "id": "gain", "normalized": 0.75 }],
                "bridge": {
                    "revision": 11,
                    "paramsRevision": 2,
                    "configRevision": 5,
                    "uiRevision": 4,
                    "config": { "theme": "light", "scale": 1.25 },
                    "uiState": { "panel": "compact" },
                },
            }),
        )));
        let state_input_ptr = state_input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(controller.setState(state_input_ptr.as_ptr()), kResultOk);

        let flush = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 3,
            "lane": "event",
            "kind": "request",
            "type": "event.flush",
            "id": "flush",
        })
        .to_string();
        let flush_packets = bridge(flush);
        let state_event = flush_packets
            .iter()
            .find(|packet| packet.packet_type == "state.changed")
            .expect("state restore event");
        let payload = state_event.payload.as_ref().unwrap();
        assert_eq!(payload["revision"], 11);
        assert_eq!(payload["configRevision"], 5);
        assert_eq!(payload["uiRevision"], 4);
        assert_eq!(payload["config"]["theme"], "light");
        assert_eq!(payload["config"]["scale"], 1.25);
        assert_eq!(payload["uiState"]["panel"], "compact");
        assert!(
            flush_packets
                .iter()
                .any(|packet| packet.reply_to.as_deref() == Some("flush"))
        );

        let snapshot_get = serde_json::json!({
            "v": 1,
            "session": session,
            "seq": 4,
            "lane": "state",
            "kind": "request",
            "type": "snapshot.get",
            "id": "snapshot",
        })
        .to_string();
        let snapshot_packets = bridge(snapshot_get);
        let snapshot = snapshot_packets[0].payload.as_ref().unwrap();
        assert_eq!(snapshot["revision"], 11);
        assert_eq!(snapshot["config"]["theme"], "light");
        assert_eq!(snapshot["uiState"]["panel"], "compact");
    }
}
