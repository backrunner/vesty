use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use ts_rs::{Config as TsConfig, TS};
use vesty_params::{ParamFlags, ParamKind, ParamMidiMapping, ParamSpec};

pub const MAX_COMMAND_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_STATE_MESSAGE_BYTES: usize = 256 * 1024;
pub const BRIDGE_PROTOCOL_VERSION: u16 = 1;
pub const MAX_BRIDGE_SESSION_BYTES: usize = 128;
pub const MAX_BRIDGE_PACKET_ID_BYTES: usize = 128;
pub const MAX_BRIDGE_PACKET_TYPE_BYTES: usize = 128;
pub const MAX_BRIDGE_PACKET_SEQ: u64 = 9_007_199_254_740_991;
pub const MAX_BRIDGE_PACKET_FLAGS: usize = 16;
pub const MAX_BRIDGE_PACKET_FLAG_BYTES: usize = 64;
pub const MAX_BRIDGE_ERROR_MESSAGE_BYTES: usize = 2048;
pub const MAX_BRIDGE_JSON_DEPTH: usize = 64;
pub const MAX_BRIDGE_JSON_ARRAY_ITEMS: usize = 65_536;
pub const MAX_BRIDGE_JSON_OBJECT_KEYS: usize = 16_384;
pub const MAX_BRIDGE_JSON_NODES: usize = 262_144;
pub const MAX_BRIDGE_JSON_STRING_BYTES: usize = 262_144;
pub const MAX_HELLO_PROTOCOL_VERSIONS: usize = 16;
pub const MAX_HELLO_JS_PACKAGE_VERSION_BYTES: usize = 64;
pub const MAX_HELLO_PAGE_URL_BYTES: usize = 2048;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/BridgeLane.ts")]
#[serde(rename_all = "camelCase")]
pub enum BridgeLane {
    Lifecycle,
    Command,
    Param,
    State,
    Event,
    Meter,
    Log,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/BridgeKind.ts")]
#[serde(rename_all = "camelCase")]
pub enum BridgeKind {
    Request,
    Response,
    Event,
    Ack,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/BridgePacket.ts")]
#[serde(rename_all = "camelCase")]
pub struct BridgePacket {
    pub v: u16,
    pub session: String,
    pub seq: u64,
    pub lane: BridgeLane,
    pub kind: BridgeKind,
    #[serde(rename = "type")]
    pub packet_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<BridgeErrorPayload>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
}

impl BridgePacket {
    pub fn request(
        session: impl Into<String>,
        seq: u64,
        lane: BridgeLane,
        packet_type: impl Into<String>,
    ) -> Self {
        Self {
            v: 1,
            session: session.into(),
            seq,
            lane,
            kind: BridgeKind::Request,
            packet_type: packet_type.into(),
            id: None,
            reply_to: None,
            payload: None,
            error: None,
            flags: Vec::new(),
        }
    }

    pub fn response_to(&self, seq: u64, payload: Option<Value>) -> Self {
        Self {
            v: self.v,
            session: self.session.clone(),
            seq,
            lane: self.lane.clone(),
            kind: BridgeKind::Response,
            packet_type: reply_packet_type(&self.packet_type, "response"),
            id: None,
            reply_to: sanitized_reply_to(self.id.as_deref()),
            payload: payload.map(sanitize_bridge_json_value),
            error: None,
            flags: Vec::new(),
        }
    }

    pub fn error_to(&self, seq: u64, mut error: BridgeErrorPayload) -> Self {
        error.sanitize_details();
        Self {
            v: self.v,
            session: self.session.clone(),
            seq,
            lane: self.lane.clone(),
            kind: BridgeKind::Error,
            packet_type: reply_packet_type(&self.packet_type, "error"),
            id: None,
            reply_to: sanitized_reply_to(self.id.as_deref()),
            payload: None,
            error: Some(error),
            flags: Vec::new(),
        }
    }
}

fn sanitized_reply_to(id: Option<&str>) -> Option<String> {
    id.filter(|value| validate_bridge_packet_id(value).is_ok())
        .map(str::to_string)
}

fn reply_packet_type(packet_type: &str, suffix: &str) -> String {
    if validate_packet_type(packet_type).is_ok() {
        format!("{packet_type}.{suffix}")
    } else {
        format!("bridge.invalidType.{suffix}")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/BridgeErrorCode.ts")]
#[serde(rename_all = "snake_case")]
pub enum BridgeErrorCode {
    ParseError,
    UnsupportedVersion,
    UnsupportedType,
    ValidationError,
    PermissionDenied,
    Timeout,
    Backpressure,
    HostRejected,
    PluginFaulted,
    StateConflict,
    InternalError,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/BridgeErrorPayload.ts")]
#[serde(rename_all = "camelCase")]
pub struct BridgeErrorPayload {
    pub code: BridgeErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub details: Option<Value>,
    pub retryable: bool,
}

impl BridgeErrorPayload {
    pub fn new(code: BridgeErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: sanitize_bridge_error_message(message),
            details: None,
            retryable,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.set_details(details);
        self
    }

    pub fn set_details(&mut self, details: Value) {
        self.details = Some(sanitize_bridge_json_value(details));
    }

    pub fn sanitize_details(&mut self) {
        if let Some(details) = self.details.take() {
            self.details = Some(sanitize_bridge_json_value(details));
        }
    }
}

pub fn sanitize_bridge_error_message(message: impl Into<String>) -> String {
    let mut message = message.into();
    if message.chars().any(char::is_control) {
        message = message
            .chars()
            .map(|ch| if ch.is_control() { ' ' } else { ch })
            .collect();
    }
    if message.len() > MAX_BRIDGE_ERROR_MESSAGE_BYTES {
        let mut end = MAX_BRIDGE_ERROR_MESSAGE_BYTES;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    message
}

pub fn validate_bridge_error_message(message: &str) -> Result<(), &'static str> {
    if message.len() > MAX_BRIDGE_ERROR_MESSAGE_BYTES {
        return Err("bridge error message too long");
    }
    if message.chars().any(char::is_control) {
        return Err("bridge error message must not contain control characters");
    }
    Ok(())
}

pub fn validate_bridge_json_value(value: &Value) -> Result<(), &'static str> {
    fn visit(value: &Value, depth: usize, nodes: &mut usize) -> Result<(), &'static str> {
        *nodes += 1;
        if *nodes > MAX_BRIDGE_JSON_NODES {
            return Err("bridge JSON value has too many nodes");
        }
        if depth > MAX_BRIDGE_JSON_DEPTH {
            return Err("bridge JSON value exceeds maximum depth");
        }
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
            Value::String(text) => {
                if text.len() > MAX_BRIDGE_JSON_STRING_BYTES {
                    Err("bridge JSON string too long")
                } else {
                    Ok(())
                }
            }
            Value::Array(items) => {
                if items.len() > MAX_BRIDGE_JSON_ARRAY_ITEMS {
                    return Err("bridge JSON array too long");
                }
                for item in items {
                    visit(item, depth + 1, nodes)?;
                }
                Ok(())
            }
            Value::Object(object) => {
                if object.len() > MAX_BRIDGE_JSON_OBJECT_KEYS {
                    return Err("bridge JSON object has too many keys");
                }
                for (key, item) in object {
                    if key.len() > MAX_BRIDGE_JSON_STRING_BYTES {
                        return Err("bridge JSON object key too long");
                    }
                    visit(item, depth + 1, nodes)?;
                }
                Ok(())
            }
        }
    }

    let mut nodes = 0;
    visit(value, 0, &mut nodes)
}

pub fn sanitize_bridge_json_value(value: Value) -> Value {
    match validate_bridge_json_value(&value) {
        Ok(()) => value,
        Err(reason) => serde_json::json!({
            "dropped": true,
            "reason": reason,
        }),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/BridgeReadyPayload.ts")]
#[serde(rename_all = "camelCase")]
pub struct BridgeReadyPayload {
    pub protocol_version: u16,
    pub instance_id: String,
    pub editor_session_id: String,
    pub dev_mode: bool,
    pub plugin_name: String,
    pub vendor: String,
    pub capabilities: BridgeCapabilities,
    pub params: Vec<ParamSpec>,
    pub param_values: Vec<ParamValueSnapshot>,
    pub snapshot: PluginSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/ParamValueSnapshot.ts")]
#[serde(rename_all = "camelCase")]
pub struct ParamValueSnapshot {
    pub id: String,
    pub normalized: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/BridgeCapabilities.ts")]
#[serde(rename_all = "camelCase")]
pub struct BridgeCapabilities {
    pub param_gestures: bool,
    pub param_format_parse: bool,
    pub state_config: bool,
    pub subscriptions: bool,
    pub meter_stream: bool,
    pub reliable_events: bool,
    pub diagnostics: bool,
}

impl BridgeCapabilities {
    pub fn v1_default() -> Self {
        Self {
            param_gestures: true,
            param_format_parse: true,
            state_config: true,
            subscriptions: true,
            meter_stream: true,
            reliable_events: true,
            diagnostics: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/BridgeHelloPayload.ts")]
#[serde(rename_all = "camelCase")]
pub struct BridgeHelloPayload {
    pub supported_protocol_versions: Vec<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub js_package_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub page_url: Option<String>,
}

impl BridgeHelloPayload {
    pub fn supports_protocol(&self, protocol_version: u16) -> bool {
        self.supported_protocol_versions.contains(&protocol_version)
    }

    pub fn has_valid_shape(&self) -> bool {
        !self.supported_protocol_versions.is_empty()
            && self.supported_protocol_versions.len() <= MAX_HELLO_PROTOCOL_VERSIONS
            && self
                .js_package_version
                .as_ref()
                .is_none_or(|value| value.len() <= MAX_HELLO_JS_PACKAGE_VERSION_BYTES)
            && self
                .page_url
                .as_ref()
                .is_none_or(|value| value.len() <= MAX_HELLO_PAGE_URL_BYTES)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/PluginSnapshot.ts")]
#[serde(rename_all = "camelCase")]
pub struct PluginSnapshot {
    pub revision: u64,
    pub params_revision: u64,
    pub config_revision: u64,
    pub ui_revision: u64,
    pub config: Value,
    pub ui_state: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/PluginFaultReport.ts")]
#[serde(rename_all = "camelCase")]
pub struct PluginFaultReport {
    pub faulted: bool,
    pub fault_count: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/BridgeDiagnosticsSnapshot.ts")]
#[serde(rename_all = "camelCase")]
pub struct BridgeDiagnosticsSnapshot {
    pub ready_acknowledged: bool,
    pub subscription_count: usize,
    pub subscriptions: Vec<String>,
    pub pending_param_gestures: usize,
    pub dropped_param_gestures: u64,
    pub pending_meter_topics: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fault: Option<PluginFaultReport>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/RtLogLevel.ts")]
#[serde(rename_all = "camelCase")]
pub enum RtLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/RtLogKind.ts")]
#[serde(rename_all = "camelCase")]
pub enum RtLogKind {
    QueueOverflow,
    Faulted,
    HostWarning,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/RtLogQueue.ts")]
#[serde(rename_all = "camelCase")]
pub enum RtLogQueue {
    Events,
    Params,
    Meter,
    Log,
    Bridge,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/RtLogRecord.ts")]
#[serde(rename_all = "camelCase")]
pub struct RtLogRecord {
    pub sequence: u64,
    pub level: RtLogLevel,
    pub kind: RtLogKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub queue: Option<RtLogQueue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub other_queue_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dropped: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub code: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value_a: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value_b: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/ParamChangedEvent.ts")]
#[serde(rename_all = "camelCase")]
pub struct ParamChangedEvent {
    pub id: String,
    pub normalized: f64,
    pub plain: Option<f64>,
    pub display: Option<String>,
    pub source: ParamChangeSource,
    pub gesture_id: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/ParamChangeSource.ts")]
#[serde(rename_all = "camelCase")]
pub enum ParamChangeSource {
    Host,
    Ui,
    State,
    Program,
}

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("failed to parse bridge packet: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("protocol binding export failed: {message}")]
    TypeExport { message: String },
    #[error("protocol binding filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("bridge message too large: {len} bytes exceeds {max} byte limit")]
    MessageTooLarge { len: usize, max: usize },
}

pub fn max_message_bytes_for_lane(lane: &BridgeLane) -> usize {
    match lane {
        BridgeLane::State => MAX_STATE_MESSAGE_BYTES,
        _ => MAX_COMMAND_MESSAGE_BYTES,
    }
}

pub fn validate_packet_type(packet_type: &str) -> Result<(), &'static str> {
    if packet_type.is_empty() {
        return Err("request type must not be empty");
    }
    if packet_type.len() > MAX_BRIDGE_PACKET_TYPE_BYTES {
        return Err("request type too long");
    }
    if packet_type.chars().any(char::is_control) {
        return Err("request type must not contain control characters");
    }
    Ok(())
}

pub fn validate_bridge_packet_id(id: &str) -> Result<(), &'static str> {
    if id.is_empty() {
        return Err("request id must not be empty");
    }
    if id.len() > MAX_BRIDGE_PACKET_ID_BYTES {
        return Err("request id too long");
    }
    if id.chars().any(char::is_control) {
        return Err("request id must not contain control characters");
    }
    Ok(())
}

pub fn validate_bridge_packet_flags(flags: &[String]) -> Result<(), &'static str> {
    if flags.len() > MAX_BRIDGE_PACKET_FLAGS {
        return Err("bridge packet has too many flags");
    }
    for flag in flags {
        if flag.is_empty() {
            return Err("bridge packet flag must not be empty");
        }
        if flag.len() > MAX_BRIDGE_PACKET_FLAG_BYTES {
            return Err("bridge packet flag too long");
        }
        if flag.chars().any(char::is_control) {
            return Err("bridge packet flag must not contain control characters");
        }
    }
    Ok(())
}

pub fn validate_bridge_packet_seq(seq: u64) -> Result<(), &'static str> {
    if seq > MAX_BRIDGE_PACKET_SEQ {
        return Err("bridge packet seq exceeds JavaScript safe integer range");
    }
    Ok(())
}

pub fn advance_bridge_packet_seq(seq: u64) -> u64 {
    if seq >= MAX_BRIDGE_PACKET_SEQ {
        1
    } else {
        seq + 1
    }
}

pub fn validate_bridge_session(session: &str) -> Result<(), &'static str> {
    if session.is_empty() {
        return Err("bridge session must not be empty");
    }
    if session.len() > MAX_BRIDGE_SESSION_BYTES {
        return Err("bridge session too long");
    }
    if session.chars().any(char::is_control) {
        return Err("bridge session must not contain control characters");
    }
    Ok(())
}

pub fn parse_packet(text: &str) -> Result<BridgePacket, IpcError> {
    if text.len() > MAX_STATE_MESSAGE_BYTES {
        return Err(IpcError::MessageTooLarge {
            len: text.len(),
            max: MAX_STATE_MESSAGE_BYTES,
        });
    }
    Ok(serde_json::from_str(text)?)
}

pub fn serialize_packet(packet: &BridgePacket) -> Result<String, IpcError> {
    Ok(serde_json::to_string(packet)?)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolExportReport {
    pub typescript_dir: PathBuf,
    pub json_schema_dir: PathBuf,
    pub typescript_files: usize,
    pub json_schema_files: usize,
}

pub fn export_protocol_bindings(
    out_dir: impl AsRef<Path>,
) -> Result<ProtocolExportReport, IpcError> {
    let out_dir = out_dir.as_ref();
    let typescript_dir = out_dir.join("typescript");
    let json_schema_dir = out_dir.join("json-schema");
    fs::create_dir_all(&typescript_dir)?;
    fs::create_dir_all(&json_schema_dir)?;

    let ts_config = TsConfig::new()
        .with_out_dir(&typescript_dir)
        .with_large_int("number");

    export_ts::<BridgeLane>(&ts_config)?;
    export_ts::<BridgeKind>(&ts_config)?;
    export_ts::<BridgePacket>(&ts_config)?;
    export_ts::<BridgeErrorCode>(&ts_config)?;
    export_ts::<BridgeErrorPayload>(&ts_config)?;
    export_ts::<BridgeReadyPayload>(&ts_config)?;
    export_ts::<BridgeCapabilities>(&ts_config)?;
    export_ts::<BridgeHelloPayload>(&ts_config)?;
    export_ts::<PluginSnapshot>(&ts_config)?;
    export_ts::<PluginFaultReport>(&ts_config)?;
    export_ts::<BridgeDiagnosticsSnapshot>(&ts_config)?;
    export_ts::<RtLogLevel>(&ts_config)?;
    export_ts::<RtLogKind>(&ts_config)?;
    export_ts::<RtLogQueue>(&ts_config)?;
    export_ts::<RtLogRecord>(&ts_config)?;
    export_ts::<ParamChangedEvent>(&ts_config)?;
    export_ts::<ParamChangeSource>(&ts_config)?;
    export_ts::<ParamKind>(&ts_config)?;
    export_ts::<ParamFlags>(&ts_config)?;
    export_ts::<ParamMidiMapping>(&ts_config)?;
    export_ts::<ParamSpec>(&ts_config)?;
    export_ts::<ParamValueSnapshot>(&ts_config)?;

    write_json_schema::<BridgePacket>(&json_schema_dir, "BridgePacket.schema.json")?;
    write_json_schema::<BridgeReadyPayload>(&json_schema_dir, "BridgeReadyPayload.schema.json")?;
    write_json_schema::<BridgeHelloPayload>(&json_schema_dir, "BridgeHelloPayload.schema.json")?;
    write_json_schema::<BridgeDiagnosticsSnapshot>(
        &json_schema_dir,
        "BridgeDiagnosticsSnapshot.schema.json",
    )?;
    write_json_schema::<RtLogRecord>(&json_schema_dir, "RtLogRecord.schema.json")?;
    write_json_schema::<ParamChangedEvent>(&json_schema_dir, "ParamChangedEvent.schema.json")?;
    write_json_schema::<ParamSpec>(&json_schema_dir, "ParamSpec.schema.json")?;

    Ok(ProtocolExportReport {
        typescript_files: count_files_with_extension(&typescript_dir, "ts")?,
        json_schema_files: count_files_with_extension(&json_schema_dir, "json")?,
        typescript_dir,
        json_schema_dir,
    })
}

fn export_ts<T: TS + 'static>(config: &TsConfig) -> Result<(), IpcError> {
    T::export_all(config).map_err(|source| IpcError::TypeExport {
        message: source.to_string(),
    })
}

fn write_json_schema<T: JsonSchema>(dir: &Path, filename: &str) -> Result<(), IpcError> {
    let schema = schemars::schema_for!(T);
    let text = serde_json::to_string_pretty(&schema)?;
    fs::write(dir.join(filename), text)?;
    Ok(())
}

fn count_files_with_extension(dir: &Path, extension: &str) -> Result<usize, IpcError> {
    let mut count = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            count += count_files_with_extension(&path, extension)?;
        } else if path.extension().is_some_and(|value| value == extension) {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_packet() {
        let text = r#"{"v":1,"session":"s","seq":1,"lane":"command","kind":"request","type":"bridge.hello"}"#;
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
            validate_bridge_packet_flags(&["x".repeat(MAX_BRIDGE_PACKET_FLAG_BYTES + 1)])
                .unwrap_err(),
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
            validate_bridge_error_message(&"x".repeat(MAX_BRIDGE_ERROR_MESSAGE_BYTES + 1))
                .unwrap_err(),
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
            fs::read_to_string(report.typescript_dir.join("protocol").join("ParamSpec.ts"))
                .unwrap();
        assert!(param_spec.contains("defaultNormalized: number"));
        assert!(param_spec.contains("stepCount: number | null"));
        assert!(!param_spec.contains("default_normalized"));
        assert!(!param_spec.contains("step_count"));

        let param_kind =
            fs::read_to_string(report.typescript_dir.join("protocol").join("ParamKind.ts"))
                .unwrap();
        assert!(param_kind.contains("\"float\""));
        assert!(param_kind.contains("\"bool\""));
        assert!(param_kind.contains("\"choice\""));
    }
}
