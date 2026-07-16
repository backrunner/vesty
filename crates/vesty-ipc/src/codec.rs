use thiserror::Error;

use crate::{
    BridgeLane, BridgePacket, MAX_BRIDGE_PACKET_FLAG_BYTES, MAX_BRIDGE_PACKET_FLAGS,
    MAX_BRIDGE_PACKET_ID_BYTES, MAX_BRIDGE_PACKET_SEQ, MAX_BRIDGE_PACKET_TYPE_BYTES,
    MAX_BRIDGE_SESSION_BYTES, MAX_COMMAND_MESSAGE_BYTES, MAX_STATE_MESSAGE_BYTES,
};

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
