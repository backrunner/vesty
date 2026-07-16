use serde_json::Value;
use vesty_ipc::{
    BridgePacket, validate_bridge_json_value, validate_bridge_packet_flags,
    validate_bridge_packet_id, validate_bridge_packet_seq,
};

use crate::{MAX_CONFIG_KEY_BYTES, MAX_PARAM_GESTURE_ID_BYTES, MAX_SUBSCRIPTION_TOPIC_BYTES};

pub(crate) fn payload_required_u64(
    payload: Option<&Value>,
    key: &str,
    missing_message: &'static str,
    type_message: &'static str,
) -> Result<u64, &'static str> {
    let Some(value) = payload.and_then(|payload| payload.get(key)) else {
        return Err(missing_message);
    };
    value.as_u64().ok_or(type_message)
}

pub(crate) fn payload_required_string<'a>(
    payload: Option<&'a Value>,
    key: &str,
    missing_message: &'static str,
    type_message: &'static str,
) -> Result<&'a str, &'static str> {
    let Some(value) = payload.and_then(|payload| payload.get(key)) else {
        return Err(missing_message);
    };
    value.as_str().ok_or(type_message)
}

pub(crate) fn payload_required_value<'a>(
    payload: Option<&'a Value>,
    key: &str,
    missing_message: &'static str,
) -> Result<&'a Value, &'static str> {
    let Some(value) = payload.and_then(|payload| payload.get(key)) else {
        return Err(missing_message);
    };
    Ok(value)
}

pub(crate) fn validate_payload_allowed_fields(
    payload: Option<&Value>,
    allowed: &[&str],
) -> Result<(), &'static str> {
    let Some(Value::Object(object)) = payload else {
        return Ok(());
    };
    if object
        .keys()
        .any(|key| !allowed.iter().any(|allowed| key == allowed))
    {
        return Err("unexpected request payload field");
    }
    Ok(())
}

pub(crate) fn validate_empty_payload(payload: Option<&Value>) -> Result<(), &'static str> {
    match payload {
        None => Ok(()),
        Some(Value::Object(object)) if object.is_empty() => Ok(()),
        Some(Value::Object(_)) => Err("unexpected request payload field"),
        Some(_) => Err("unexpected request payload"),
    }
}

pub(crate) fn config_write_from_payload(
    payload: Option<&Value>,
) -> Result<(u64, String, Value), &'static str> {
    validate_payload_allowed_fields(payload, &["baseRevision", "key", "value"])?;
    let base_revision = payload_required_u64(
        payload,
        "baseRevision",
        "missing config baseRevision",
        "baseRevision must be a non-negative integer",
    )?;
    let key = payload_required_string(
        payload,
        "key",
        "missing config key",
        "config key must be a string",
    )?;
    validate_config_key(key)?;
    let value = payload_required_value(payload, "value", "missing config value")?;
    Ok((base_revision, key.to_string(), value.clone()))
}

pub(crate) fn ui_state_write_from_payload(
    payload: Option<&Value>,
) -> Result<(u64, Value), &'static str> {
    validate_payload_allowed_fields(payload, &["baseRevision", "value"])?;
    let base_revision = payload_required_u64(
        payload,
        "baseRevision",
        "missing ui state baseRevision",
        "baseRevision must be a non-negative integer",
    )?;
    let value = payload_required_value(payload, "value", "missing ui state value")?;
    Ok((base_revision, value.clone()))
}

pub(crate) fn ready_ack_protocol_version(payload: Option<&Value>) -> Result<u64, &'static str> {
    validate_payload_allowed_fields(payload, &["protocolVersion"])?;
    payload_required_u64(
        payload,
        "protocolVersion",
        "missing readyAck protocolVersion",
        "readyAck protocolVersion must be a non-negative integer",
    )
}

pub(crate) fn validate_request_id(packet: &BridgePacket) -> Result<(), &'static str> {
    let Some(id) = packet.id.as_deref() else {
        return Err("request id must be a string");
    };
    validate_bridge_packet_id(id)
}

pub(crate) fn validate_inbound_request_shape(packet: &BridgePacket) -> Result<(), &'static str> {
    validate_request_id(packet)?;
    validate_bridge_packet_seq(packet.seq)?;
    validate_bridge_packet_flags(&packet.flags)?;
    if packet.reply_to.is_some() {
        return Err("request replyTo must not be set");
    }
    if packet.error.is_some() {
        return Err("request error must not be set");
    }
    if let Some(payload) = packet.payload.as_ref() {
        validate_bridge_json_value(payload)?;
    }
    Ok(())
}

pub(crate) fn param_id_from_payload(payload: Option<&Value>) -> Result<&str, &'static str> {
    let id = payload_required_string(
        payload,
        "id",
        "missing parameter id",
        "parameter id must be a string",
    )?;
    if id.is_empty() {
        return Err("parameter id must not be empty");
    }
    if id.chars().any(char::is_control) {
        return Err("parameter id must not contain control characters");
    }
    Ok(id)
}

pub(crate) fn normalized_value_from_payload(payload: Option<&Value>) -> Result<f64, &'static str> {
    let Some(value) = payload.and_then(|payload| payload.get("normalized")) else {
        return Err("missing normalized value");
    };
    let Some(normalized) = value.as_f64() else {
        return Err("normalized value must be a finite number");
    };
    if !normalized.is_finite() {
        return Err("normalized value must be a finite number");
    }
    Ok(normalized)
}

pub(crate) fn param_text_from_payload(payload: Option<&Value>) -> Result<&str, &'static str> {
    payload_required_string(
        payload,
        "text",
        "missing parameter text",
        "parameter text must be a string",
    )
}

pub(crate) fn payload_gesture_id(payload: Option<&Value>) -> Result<Option<String>, &'static str> {
    let Some(value) = payload.and_then(|payload| payload.get("gestureId")) else {
        return Ok(None);
    };
    let Some(gesture_id) = value.as_str() else {
        return Err("gestureId must be a string");
    };
    if gesture_id.is_empty() {
        return Err("gestureId must not be empty");
    }
    if gesture_id.len() > MAX_PARAM_GESTURE_ID_BYTES {
        return Err("gestureId too long");
    }
    if gesture_id.chars().any(char::is_control) {
        return Err("gestureId must not contain control characters");
    }
    Ok(Some(gesture_id.to_string()))
}

pub(crate) fn validate_config_key(key: &str) -> Result<(), &'static str> {
    if key.is_empty() {
        return Err("config key must not be empty");
    }
    if key.len() > MAX_CONFIG_KEY_BYTES {
        return Err("config key too long");
    }
    if key.chars().any(char::is_control) {
        return Err("config key must not contain control characters");
    }
    Ok(())
}

pub(crate) fn validate_subscription_topic(topic: &str) -> Result<(), &'static str> {
    if topic.is_empty() {
        return Err("subscription topic must not be empty");
    }
    if topic.len() > MAX_SUBSCRIPTION_TOPIC_BYTES {
        return Err("subscription topic too long");
    }
    if topic.chars().any(char::is_control) {
        return Err("subscription topic must not contain control characters");
    }
    Ok(())
}

pub(crate) fn subscription_topic_from_payload(
    payload: Option<&Value>,
) -> Result<&str, &'static str> {
    validate_payload_allowed_fields(payload, &["topic"])?;
    let Some(payload) = payload else {
        return Err("missing subscription topic");
    };
    let Some(topic) = payload.get("topic") else {
        return Err("missing subscription topic");
    };
    let Some(topic) = topic.as_str() else {
        return Err("subscription topic must be a string");
    };
    validate_subscription_topic(topic)?;
    Ok(topic)
}
