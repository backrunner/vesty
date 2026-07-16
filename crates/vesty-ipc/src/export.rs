use schemars::JsonSchema;
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::{Config as TsConfig, TS};
use vesty_params::{ParamFlags, ParamKind, ParamMidiMapping, ParamSpec};

use crate::{
    BridgeCapabilities, BridgeDiagnosticsSnapshot, BridgeErrorCode, BridgeErrorPayload,
    BridgeHelloPayload, BridgeKind, BridgeLane, BridgePacket, BridgeReadyPayload, IpcError,
    ParamChangeSource, ParamChangedEvent, ParamValueSnapshot, PluginFaultReport, PluginSnapshot,
    RtLogKind, RtLogLevel, RtLogQueue, RtLogRecord,
};

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
