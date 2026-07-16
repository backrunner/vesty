use super::*;
use vesty_ipc::{
    BridgeErrorCode, BridgeKind, BridgeLane, BridgePacket, MAX_COMMAND_MESSAGE_BYTES,
    MAX_STATE_MESSAGE_BYTES,
};

#[cfg(feature = "wry-backend")]
use crate::assets::*;
#[cfg(feature = "wry-backend")]
use sha2::{Digest, Sha256};
#[cfg(feature = "wry-backend")]
use std::rc::Rc;

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

    let packets =
        with_suppressed_panic_hook(|| call_ipc_handler_guarded(&handler, "{not json".to_string()));

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
        script
            .contains("assertBridgeSessionValue(session, \"editorSessionId\", readyPayloadError)")
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
    assert!(script.contains("if (readyPayloadCache) return Promise.resolve(readyPayloadCache)"));
    assert!(script.contains("if (!readyPromise)"));
    assert!(script.contains("readyPayloadCache = payload"));
    assert!(script.contains("async setParam(id, normalized, gestureId)"));
    assert!(script.contains("async setConfig"));
    assert!(script.contains("async setUiState"));
    assert!(script.contains("async beginParamEdit"));
    assert!(script.contains("async beginParamEdit(id, gestureId)"));
    assert!(script.contains("\"param.begin\", \"param\", paramGesturePayload(id, gestureId)"));
    assert!(script.contains("async performParamEdit"));
    assert!(
        script.contains(
            "\"param.perform\", \"param\", paramPerformPayload(id, normalized, gestureId)"
        )
    );
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
    assert!(script.contains("assertReadyParamValues"));
    assert!(script.contains("assertParamSpec"));
    assert!(script.contains("assertParamMidiMappings"));
    assert!(script.contains("duplicate parameter id"));
    assert!(script.contains("duplicate current parameter value"));
    assert!(script.contains("paramValues must contain one value for every parameter"));
    assert!(script.contains("references an unknown parameter"));
    assert!(script.contains("${name}.normalized"));
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
    std::os::unix::fs::symlink(root.join("assets/target.js"), root.join("assets/link.js")).unwrap();
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
