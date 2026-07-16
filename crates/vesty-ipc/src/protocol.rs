use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use vesty_params::ParamSpec;

use crate::{
    MAX_HELLO_JS_PACKAGE_VERSION_BYTES, MAX_HELLO_PAGE_URL_BYTES, MAX_HELLO_PROTOCOL_VERSIONS,
};

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
