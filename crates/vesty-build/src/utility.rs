use serde::Serialize;

use crate::BuildError;

pub(crate) fn sanitize_bundle_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|char| char.is_ascii_alphanumeric() || *char == '_' || *char == '-')
        .collect();
    if sanitized.is_empty() {
        "VestyPlugin".to_string()
    } else {
        sanitized
    }
}

pub(crate) fn fallback_bundle_id(executable: &str) -> String {
    let mut segment = String::new();
    for byte in executable.bytes() {
        let byte = byte.to_ascii_lowercase();
        if byte.is_ascii_alphanumeric() || byte == b'-' {
            segment.push(byte as char);
        } else if byte == b'_' {
            segment.push('-');
        }
    }
    let segment = segment.trim_matches('-');
    if segment.is_empty() {
        "dev.vesty.plugin".to_string()
    } else {
        format!("dev.vesty.{segment}")
    }
}

pub(crate) fn serde_json_pretty<T: Serialize>(value: &T) -> Result<String, BuildError> {
    serde_json::to_string_pretty(value).map_err(BuildError::JsonSerialize)
}
