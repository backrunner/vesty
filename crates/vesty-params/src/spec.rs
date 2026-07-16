use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::{ParamId, normalized_for_choice_index};

#[derive(Clone, Debug, Serialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/ParamKind.ts")]
#[serde(rename_all = "camelCase")]
pub enum ParamKind {
    Float { min: f64, max: f64 },
    Bool,
    Choice { values: Vec<String> },
}

impl<'de> Deserialize<'de> for ParamKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        enum WireParamKind {
            #[serde(alias = "Float")]
            Float { min: f64, max: f64 },
            #[serde(alias = "Bool")]
            Bool,
            #[serde(alias = "Choice")]
            Choice { values: Vec<String> },
        }

        Ok(match WireParamKind::deserialize(deserializer)? {
            WireParamKind::Float { min, max } => ParamKind::Float { min, max },
            WireParamKind::Bool => ParamKind::Bool,
            WireParamKind::Choice { values } => ParamKind::Choice { values },
        })
    }
}

#[derive(Clone, Debug, Default, Serialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/ParamFlags.ts")]
#[serde(rename_all = "camelCase")]
pub struct ParamFlags {
    pub automatable: bool,
    pub bypass: bool,
    pub read_only: bool,
    pub program_change: bool,
}

impl<'de> Deserialize<'de> for ParamFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WireParamFlags {
            automatable: bool,
            bypass: bool,
            #[serde(alias = "read_only")]
            read_only: bool,
            #[serde(default, alias = "program_change")]
            program_change: bool,
        }

        let flags = WireParamFlags::deserialize(deserializer)?;
        Ok(Self {
            automatable: flags.automatable,
            bypass: flags.bypass,
            read_only: flags.read_only,
            program_change: flags.program_change,
        })
    }
}

pub mod midi {
    pub const CHANNEL_PRESSURE: u16 = 128;
    pub const PITCH_BEND: u16 = 129;
    pub const PROGRAM_CHANGE: u16 = 130;
    pub const POLY_PRESSURE: u16 = 131;
    pub const QUARTER_FRAME: u16 = 132;
    pub const SONG_SELECT: u16 = 133;
    pub const SONG_POINTER: u16 = 134;
    pub const CABLE_SELECT: u16 = 135;
    pub const TUNE_REQUEST: u16 = 136;
    pub const CLOCK_START: u16 = 137;
    pub const CLOCK_CONTINUE: u16 = 138;
    pub const CLOCK_STOP: u16 = 139;
    pub const ACTIVE_SENSING: u16 = 140;
}

pub const MAX_MIDI_CONTROLLER: u16 = 140;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, TS, PartialEq, Eq)]
#[ts(export_to = "protocol/ParamMidiMapping.ts")]
#[serde(rename_all = "camelCase")]
pub struct ParamMidiMapping {
    pub controller: u16,
    pub channel: Option<u16>,
}

#[derive(Clone, Debug, Serialize, JsonSchema, TS, PartialEq)]
#[ts(export_to = "protocol/ParamSpec.ts")]
#[serde(rename_all = "camelCase")]
pub struct ParamSpec {
    pub id: ParamId,
    pub name: String,
    pub kind: ParamKind,
    pub default_normalized: f64,
    pub unit: Option<String>,
    pub step_count: Option<u32>,
    pub flags: ParamFlags,
    pub midi_mappings: Vec<ParamMidiMapping>,
}

impl<'de> Deserialize<'de> for ParamSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WireParamSpec {
            id: ParamId,
            name: String,
            kind: ParamKind,
            #[serde(alias = "default_normalized")]
            default_normalized: f64,
            unit: Option<String>,
            #[serde(alias = "step_count")]
            step_count: Option<u32>,
            flags: ParamFlags,
            #[serde(default, alias = "midi_mappings")]
            midi_mappings: Vec<ParamMidiMapping>,
        }

        let spec = WireParamSpec::deserialize(deserializer)?;
        Ok(Self {
            id: spec.id,
            name: spec.name,
            kind: spec.kind,
            default_normalized: spec.default_normalized,
            unit: spec.unit,
            step_count: spec.step_count,
            flags: spec.flags,
            midi_mappings: spec.midi_mappings,
        })
    }
}

impl ParamSpec {
    pub fn float(
        id: impl Into<String>,
        name: impl Into<String>,
        min: f64,
        max: f64,
        default: f64,
    ) -> Self {
        let normalized = if (max - min).abs() <= f64::EPSILON {
            0.0
        } else {
            ((default - min) / (max - min)).clamp(0.0, 1.0)
        };

        Self {
            id: id.into(),
            name: name.into(),
            kind: ParamKind::Float { min, max },
            default_normalized: normalized,
            unit: None,
            step_count: None,
            flags: ParamFlags {
                automatable: true,
                bypass: false,
                read_only: false,
                program_change: false,
            },
            midi_mappings: Vec::new(),
        }
    }

    pub fn bool(id: impl Into<String>, name: impl Into<String>, default: bool) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: ParamKind::Bool,
            default_normalized: if default { 1.0 } else { 0.0 },
            unit: None,
            step_count: Some(1),
            flags: ParamFlags {
                automatable: true,
                bypass: false,
                read_only: false,
                program_change: false,
            },
            midi_mappings: Vec::new(),
        }
    }

    pub fn choice<V, S>(
        id: impl Into<String>,
        name: impl Into<String>,
        values: V,
        default_index: usize,
    ) -> Self
    where
        V: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let values: Vec<String> = values.into_iter().map(Into::into).collect();
        let step_count = values.len().saturating_sub(1) as u32;
        let default_normalized = normalized_for_choice_index(values.len(), default_index);

        Self {
            id: id.into(),
            name: name.into(),
            kind: ParamKind::Choice { values },
            default_normalized,
            unit: None,
            step_count: Some(step_count),
            flags: ParamFlags {
                automatable: true,
                bypass: false,
                read_only: false,
                program_change: false,
            },
            midi_mappings: Vec::new(),
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn with_automatable(mut self, automatable: bool) -> Self {
        self.flags.automatable = automatable;
        self
    }

    pub fn as_bypass(mut self) -> Self {
        self.flags.bypass = true;
        self
    }

    pub fn as_read_only(mut self) -> Self {
        self.flags.read_only = true;
        self.flags.automatable = false;
        self
    }

    pub fn as_program_change(mut self) -> Self {
        self.flags.program_change = true;
        self.flags.automatable = true;
        self
    }

    pub fn with_midi_mapping(mut self, controller: u16, channel: Option<u16>) -> Self {
        self.midi_mappings.push(ParamMidiMapping {
            controller,
            channel,
        });
        self
    }

    pub fn with_midi_cc(self, controller: u16) -> Self {
        self.with_midi_mapping(controller, None)
    }

    pub fn with_channel_midi_cc(self, controller: u16, channel: u16) -> Self {
        self.with_midi_mapping(controller, Some(channel))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParamError {
    #[error("unknown parameter '{0}'")]
    Unknown(String),
    #[error("parameter '{0}' is read only")]
    ReadOnly(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParamSpecError {
    #[error("parameter id at index {index} must not be empty")]
    EmptyId { index: usize },
    #[error("duplicate parameter id '{id}'")]
    DuplicateId { id: String },
    #[error("parameter '{id}' name must not be empty")]
    EmptyName { id: String },
    #[error("parameter '{id}' {field} must not contain control characters")]
    ControlCharacter { id: String, field: &'static str },
    #[error("parameter '{id}' default normalized value must be finite and within 0.0..=1.0")]
    InvalidDefaultNormalized { id: String },
    #[error("parameter '{id}' float range must be finite and min < max")]
    InvalidFloatRange { id: String },
    #[error("parameter '{id}' choice label at index {index} must not be empty")]
    EmptyChoiceLabel { id: String, index: usize },
    #[error("parameter '{id}' choice label at index {index} must not contain control characters")]
    ChoiceLabelControlCharacter { id: String, index: usize },
    #[error("parameter '{id}' is read-only and must not be automatable")]
    ReadOnlyAutomatable { id: String },
    #[error("parameter '{id}' MIDI mapping controller {controller} is out of range")]
    InvalidMidiController { id: String, controller: u16 },
    #[error("parameter '{id}' MIDI mapping channel {channel} is out of range")]
    InvalidMidiChannel { id: String, channel: u16 },
    #[error("parameter '{id}' has a duplicate MIDI mapping for controller {controller}")]
    DuplicateMidiMapping { id: String, controller: u16 },
}
