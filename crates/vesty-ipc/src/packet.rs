use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::{validate_bridge_packet_id, validate_packet_type};

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
