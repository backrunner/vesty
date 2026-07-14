use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};
use thiserror::Error;
use ts_rs::TS;

pub type ParamId = String;
pub type Vst3ParamId = u32;

pub const VST3_PARAM_ID_ALGORITHM: &str = "vesty.vst3.param.fnv1a31-positive.v2";

pub fn stable_vst3_param_id(param_id: &str) -> Vst3ParamId {
    const FNV_OFFSET: u32 = 0x811c_9dc5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    for byte in b"vesty.vst3.param:"
        .iter()
        .copied()
        .chain(param_id.as_bytes().iter().copied())
    {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    let positive_id = hash & 0x7fff_ffff;
    if positive_id == 0 { 1 } else { positive_id }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ParamHandle {
    index: usize,
}

impl ParamHandle {
    pub const INVALID_INDEX: usize = usize::MAX;

    pub const fn from_index(index: usize) -> Self {
        Self { index }
    }

    pub const fn invalid() -> Self {
        Self {
            index: Self::INVALID_INDEX,
        }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn is_invalid(self) -> bool {
        self.index == Self::INVALID_INDEX
    }
}

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

pub trait ParamCollection {
    fn specs(&self) -> Vec<ParamSpec>;
    fn get_normalized(&self, id: &str) -> Option<f64>;
    fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError>;

    fn resolve(&self, id: &str) -> Option<ParamHandle> {
        self.specs()
            .iter()
            .position(|spec| spec.id == id)
            .map(ParamHandle::from_index)
    }

    fn resolve_or_invalid(&self, id: &str) -> ParamHandle {
        self.resolve(id).unwrap_or_else(ParamHandle::invalid)
    }

    fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64>;

    fn set_normalized_by_handle(
        &self,
        handle: ParamHandle,
        normalized: f64,
    ) -> Result<(), ParamError>;
}

#[derive(Debug)]
pub struct FloatParam {
    spec: ParamSpec,
    normalized_bits: AtomicU32,
}

impl FloatParam {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        min: f64,
        max: f64,
        default: f64,
    ) -> Self {
        let spec = ParamSpec::float(id, name, min, max, default);
        Self {
            normalized_bits: AtomicU32::new((spec.default_normalized as f32).to_bits()),
            spec,
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.spec = self.spec.with_unit(unit);
        self
    }

    pub fn with_automatable(mut self, automatable: bool) -> Self {
        self.spec = self.spec.with_automatable(automatable);
        self
    }

    pub fn as_read_only(mut self) -> Self {
        self.spec = self.spec.as_read_only();
        self
    }

    pub fn as_program_change(mut self) -> Self {
        self.spec = self.spec.as_program_change();
        self
    }

    pub fn with_midi_mapping(mut self, controller: u16, channel: Option<u16>) -> Self {
        self.spec = self.spec.with_midi_mapping(controller, channel);
        self
    }

    pub fn with_midi_cc(mut self, controller: u16) -> Self {
        self.spec = self.spec.with_midi_cc(controller);
        self
    }

    pub fn with_channel_midi_cc(mut self, controller: u16, channel: u16) -> Self {
        self.spec = self.spec.with_channel_midi_cc(controller, channel);
        self
    }

    pub fn spec(&self) -> ParamSpec {
        self.spec.clone()
    }

    pub fn id(&self) -> &str {
        &self.spec.id
    }

    pub fn normalized(&self) -> f64 {
        f32::from_bits(self.normalized_bits.load(Ordering::Relaxed)) as f64
    }

    pub fn plain(&self) -> f64 {
        normalized_to_plain(&self.spec, self.normalized())
    }

    pub fn set_normalized(&self, normalized: f64) {
        self.normalized_bits.store(
            (normalized.clamp(0.0, 1.0) as f32).to_bits(),
            Ordering::Relaxed,
        );
    }
}

#[derive(Debug)]
pub struct BoolParam {
    spec: ParamSpec,
    normalized_bits: AtomicU32,
}

impl BoolParam {
    pub fn new(id: impl Into<String>, name: impl Into<String>, default: bool) -> Self {
        let spec = ParamSpec::bool(id, name, default);
        Self {
            normalized_bits: AtomicU32::new((spec.default_normalized as f32).to_bits()),
            spec,
        }
    }

    pub fn bypass(id: impl Into<String>, name: impl Into<String>, default: bool) -> Self {
        Self::new(id, name, default).as_bypass()
    }

    pub fn with_automatable(mut self, automatable: bool) -> Self {
        self.spec = self.spec.with_automatable(automatable);
        self
    }

    pub fn as_bypass(mut self) -> Self {
        self.spec = self.spec.as_bypass();
        self
    }

    pub fn as_read_only(mut self) -> Self {
        self.spec = self.spec.as_read_only();
        self
    }

    pub fn as_program_change(mut self) -> Self {
        self.spec = self.spec.as_program_change();
        self
    }

    pub fn with_midi_mapping(mut self, controller: u16, channel: Option<u16>) -> Self {
        self.spec = self.spec.with_midi_mapping(controller, channel);
        self
    }

    pub fn with_midi_cc(mut self, controller: u16) -> Self {
        self.spec = self.spec.with_midi_cc(controller);
        self
    }

    pub fn with_channel_midi_cc(mut self, controller: u16, channel: u16) -> Self {
        self.spec = self.spec.with_channel_midi_cc(controller, channel);
        self
    }

    pub fn spec(&self) -> ParamSpec {
        self.spec.clone()
    }

    pub fn id(&self) -> &str {
        &self.spec.id
    }

    pub fn value(&self) -> bool {
        self.normalized() >= 0.5
    }

    pub fn normalized(&self) -> f64 {
        f32::from_bits(self.normalized_bits.load(Ordering::Relaxed)) as f64
    }

    pub fn set_normalized(&self, normalized: f64) {
        let value = if normalized >= 0.5 { 1.0_f32 } else { 0.0_f32 };
        self.normalized_bits
            .store(value.to_bits(), Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub struct ChoiceParam {
    spec: ParamSpec,
    normalized_bits: AtomicU32,
}

impl ChoiceParam {
    pub fn new<V, S>(
        id: impl Into<String>,
        name: impl Into<String>,
        values: V,
        default_index: usize,
    ) -> Self
    where
        V: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let spec = ParamSpec::choice(id, name, values, default_index);
        Self {
            normalized_bits: AtomicU32::new((spec.default_normalized as f32).to_bits()),
            spec,
        }
    }

    pub fn with_automatable(mut self, automatable: bool) -> Self {
        self.spec = self.spec.with_automatable(automatable);
        self
    }

    pub fn as_read_only(mut self) -> Self {
        self.spec = self.spec.as_read_only();
        self
    }

    pub fn as_program_change(mut self) -> Self {
        self.spec = self.spec.as_program_change();
        self
    }

    pub fn with_midi_mapping(mut self, controller: u16, channel: Option<u16>) -> Self {
        self.spec = self.spec.with_midi_mapping(controller, channel);
        self
    }

    pub fn with_midi_cc(mut self, controller: u16) -> Self {
        self.spec = self.spec.with_midi_cc(controller);
        self
    }

    pub fn with_channel_midi_cc(mut self, controller: u16, channel: u16) -> Self {
        self.spec = self.spec.with_channel_midi_cc(controller, channel);
        self
    }

    pub fn spec(&self) -> ParamSpec {
        self.spec.clone()
    }

    pub fn id(&self) -> &str {
        &self.spec.id
    }

    pub fn normalized(&self) -> f64 {
        f32::from_bits(self.normalized_bits.load(Ordering::Relaxed)) as f64
    }

    pub fn index(&self) -> usize {
        let values_len = match &self.spec.kind {
            ParamKind::Choice { values } => values.len(),
            _ => 0,
        };
        choice_index_from_normalized(values_len, self.normalized())
    }

    pub fn value(&self) -> Option<&str> {
        let ParamKind::Choice { values } = &self.spec.kind else {
            return None;
        };
        values.get(self.index()).map(String::as_str)
    }

    pub fn set_normalized(&self, normalized: f64) {
        let values_len = match &self.spec.kind {
            ParamKind::Choice { values } => values.len(),
            _ => 0,
        };
        let index = choice_index_from_normalized(values_len, normalized);
        let normalized = normalized_for_choice_index(values_len, index) as f32;
        self.normalized_bits
            .store(normalized.to_bits(), Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Default)]
pub struct ParamRegistry {
    specs: Vec<ParamSpec>,
}

impl ParamRegistry {
    pub fn new(specs: Vec<ParamSpec>) -> Self {
        Self { specs }
    }

    pub fn try_new(specs: Vec<ParamSpec>) -> Result<Self, ParamSpecError> {
        validate_param_specs(&specs)?;
        Ok(Self { specs })
    }

    pub fn specs(&self) -> &[ParamSpec] {
        &self.specs
    }

    pub fn find(&self, id: &str) -> Option<&ParamSpec> {
        self.specs.iter().find(|spec| spec.id == id)
    }
}

pub fn validate_param_specs(specs: &[ParamSpec]) -> Result<(), ParamSpecError> {
    let mut ids = BTreeSet::new();
    for (index, spec) in specs.iter().enumerate() {
        if spec.id.trim().is_empty() {
            return Err(ParamSpecError::EmptyId { index });
        }
        if contains_control(&spec.id) {
            return Err(ParamSpecError::ControlCharacter {
                id: spec.id.clone(),
                field: "id",
            });
        }
        if !ids.insert(spec.id.clone()) {
            return Err(ParamSpecError::DuplicateId {
                id: spec.id.clone(),
            });
        }
        if spec.name.trim().is_empty() {
            return Err(ParamSpecError::EmptyName {
                id: spec.id.clone(),
            });
        }
        if contains_control(&spec.name) {
            return Err(ParamSpecError::ControlCharacter {
                id: spec.id.clone(),
                field: "name",
            });
        }
        if let Some(unit) = &spec.unit
            && contains_control(unit)
        {
            return Err(ParamSpecError::ControlCharacter {
                id: spec.id.clone(),
                field: "unit",
            });
        }
        if !spec.default_normalized.is_finite() || !(0.0..=1.0).contains(&spec.default_normalized) {
            return Err(ParamSpecError::InvalidDefaultNormalized {
                id: spec.id.clone(),
            });
        }
        if spec.flags.read_only && spec.flags.automatable {
            return Err(ParamSpecError::ReadOnlyAutomatable {
                id: spec.id.clone(),
            });
        }
        let mut midi_mappings = BTreeSet::new();
        for mapping in &spec.midi_mappings {
            if mapping.controller > MAX_MIDI_CONTROLLER {
                return Err(ParamSpecError::InvalidMidiController {
                    id: spec.id.clone(),
                    controller: mapping.controller,
                });
            }
            if let Some(channel) = mapping.channel
                && channel > 15
            {
                return Err(ParamSpecError::InvalidMidiChannel {
                    id: spec.id.clone(),
                    channel,
                });
            }
            if !midi_mappings.insert((mapping.controller, mapping.channel)) {
                return Err(ParamSpecError::DuplicateMidiMapping {
                    id: spec.id.clone(),
                    controller: mapping.controller,
                });
            }
        }
        match &spec.kind {
            ParamKind::Float { min, max } => {
                if !min.is_finite() || !max.is_finite() || min >= max {
                    return Err(ParamSpecError::InvalidFloatRange {
                        id: spec.id.clone(),
                    });
                }
            }
            ParamKind::Bool => {}
            ParamKind::Choice { values } => {
                for (choice_index, value) in values.iter().enumerate() {
                    if value.trim().is_empty() {
                        return Err(ParamSpecError::EmptyChoiceLabel {
                            id: spec.id.clone(),
                            index: choice_index,
                        });
                    }
                    if contains_control(value) {
                        return Err(ParamSpecError::ChoiceLabelControlCharacter {
                            id: spec.id.clone(),
                            index: choice_index,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn contains_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

pub fn normalized_to_plain(spec: &ParamSpec, normalized: f64) -> f64 {
    match spec.kind {
        ParamKind::Float { min, max } => min + normalized.clamp(0.0, 1.0) * (max - min),
        ParamKind::Bool => {
            if normalized >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ParamKind::Choice { ref values } => {
            if values.len() <= 1 {
                0.0
            } else {
                choice_index_from_normalized(values.len(), normalized) as f64
            }
        }
    }
}

pub fn plain_to_normalized(spec: &ParamSpec, plain: f64) -> f64 {
    match spec.kind {
        ParamKind::Float { min, max } => {
            if (max - min).abs() <= f64::EPSILON {
                0.0
            } else {
                ((plain - min) / (max - min)).clamp(0.0, 1.0)
            }
        }
        ParamKind::Bool => {
            if plain >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ParamKind::Choice { ref values } => {
            if values.len() <= 1 {
                0.0
            } else {
                (plain.round() / (values.len() as f64 - 1.0)).clamp(0.0, 1.0)
            }
        }
    }
}

pub fn format_normalized_value(spec: &ParamSpec, normalized: f64) -> String {
    if let ParamKind::Choice { values } = &spec.kind {
        let index = choice_index_from_normalized(values.len(), normalized);
        return values
            .get(index)
            .cloned()
            .unwrap_or_else(|| index.to_string());
    }

    let plain = normalized_to_plain(spec, normalized);
    match &spec.unit {
        Some(unit) if !unit.is_empty() => format!("{plain:.3} {unit}"),
        _ => format!("{plain:.3}"),
    }
}

pub fn parse_normalized_value(spec: &ParamSpec, text: &str) -> Option<f64> {
    let text = text.trim();
    if let ParamKind::Choice { values } = &spec.kind
        && let Some((index, _)) = values
            .iter()
            .enumerate()
            .find(|(_, value)| value.eq_ignore_ascii_case(text))
    {
        return Some(normalized_for_choice_index(values.len(), index));
    }

    let text = spec
        .unit
        .as_deref()
        .and_then(|unit| text.strip_suffix(unit))
        .unwrap_or(text)
        .trim();
    let plain = text.parse::<f64>().ok()?;
    Some(plain_to_normalized(spec, plain))
}

fn choice_index_from_normalized(values_len: usize, normalized: f64) -> usize {
    if values_len <= 1 {
        0
    } else {
        (normalized.clamp(0.0, 1.0) * (values_len as f64 - 1.0)).round() as usize
    }
}

fn normalized_for_choice_index(values_len: usize, index: usize) -> f64 {
    if values_len <= 1 {
        0.0
    } else {
        index.min(values_len - 1) as f64 / (values_len as f64 - 1.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LinearSmoother {
    current: f32,
    target: f32,
    step: f32,
    remaining: u32,
}

impl LinearSmoother {
    pub fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            step: 0.0,
            remaining: 0,
        }
    }

    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.step = 0.0;
        self.remaining = 0;
    }

    pub fn set_target(&mut self, target: f32, samples: u32) {
        self.target = target;
        self.remaining = samples;
        self.step = if samples == 0 {
            self.current = target;
            0.0
        } else {
            (target - self.current) / samples as f32
        };
    }

    pub fn next_value(&mut self) -> f32 {
        if self.remaining == 0 {
            self.current = self.target;
            return self.current;
        }

        self.current += self.step;
        self.remaining -= 1;
        if self.remaining == 0 {
            self.current = self.target;
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ManualParams {
        gain: FloatParam,
    }

    impl ParamCollection for ManualParams {
        fn specs(&self) -> Vec<ParamSpec> {
            vec![self.gain.spec()]
        }

        fn get_normalized(&self, id: &str) -> Option<f64> {
            (id == self.gain.id()).then(|| self.gain.normalized())
        }

        fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
            if id == self.gain.id() {
                self.gain.set_normalized(normalized);
                Ok(())
            } else {
                Err(ParamError::Unknown(id.to_string()))
            }
        }

        fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
            (handle.index() == 0).then(|| self.gain.normalized())
        }

        fn set_normalized_by_handle(
            &self,
            handle: ParamHandle,
            normalized: f64,
        ) -> Result<(), ParamError> {
            if handle.index() == 0 {
                self.gain.set_normalized(normalized);
                Ok(())
            } else {
                Err(ParamError::Unknown(format!("handle:{}", handle.index())))
            }
        }
    }

    #[test]
    fn converts_float_values() {
        let spec = ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0);
        assert!((plain_to_normalized(&spec, 0.0) - 0.833_333_333).abs() < 0.0001);
        assert!((normalized_to_plain(&spec, 1.0) - 12.0).abs() < 0.0001);
    }

    #[test]
    fn derives_stable_vst3_param_ids_from_string_ids() {
        assert_eq!(
            VST3_PARAM_ID_ALGORITHM,
            "vesty.vst3.param.fnv1a31-positive.v2"
        );
        assert_eq!(stable_vst3_param_id("gain"), 1_983_572_582);
        assert_eq!(stable_vst3_param_id("mix"), 2_093_438_089);
        assert_eq!(stable_vst3_param_id("level"), 1_889_771_317);
        assert!(stable_vst3_param_id("mix") <= i32::MAX as u32);
        assert!(stable_vst3_param_id("level") <= i32::MAX as u32);
        assert_ne!(stable_vst3_param_id("gain"), stable_vst3_param_id("mode"));
        assert_eq!(stable_vst3_param_id("gain"), stable_vst3_param_id("gain"));
    }

    #[test]
    fn param_handles_resolve_and_access_values() {
        let params = ManualParams {
            gain: FloatParam::new("gain", "Gain", 0.0, 2.0, 1.0),
        };
        let handle = params.resolve("gain").unwrap();
        assert_eq!(handle.index(), 0);
        assert_eq!(params.get_normalized_by_handle(handle), Some(0.5));

        params.set_normalized_by_handle(handle, 0.25).unwrap();
        assert_eq!(params.get_normalized("gain"), Some(0.25));
        assert_eq!(
            params.get_normalized_by_handle(ParamHandle::from_index(1)),
            None
        );
        assert!(matches!(
            params.set_normalized_by_handle(ParamHandle::from_index(1), 0.0),
            Err(ParamError::Unknown(id)) if id == "handle:1"
        ));
    }

    #[test]
    fn invalid_param_handle_is_safe_fallback() {
        let params = ManualParams {
            gain: FloatParam::new("gain", "Gain", 0.0, 2.0, 1.0),
        };

        let handle = params.resolve_or_invalid("missing");

        assert!(handle.is_invalid());
        assert_eq!(handle.index(), ParamHandle::INVALID_INDEX);
        assert_eq!(params.get_normalized_by_handle(handle), None);
        let unknown_handle = format!("handle:{}", ParamHandle::INVALID_INDEX);
        assert!(matches!(
            params.set_normalized_by_handle(handle, 0.0),
            Err(ParamError::Unknown(id)) if id == unknown_handle
        ));
    }

    #[test]
    fn converts_choice_values() {
        let spec = ParamSpec::choice("mode", "Mode", ["Clean", "Drive", "Fuzz"], 1);
        assert_eq!(spec.step_count, Some(2));
        assert!((spec.default_normalized - 0.5).abs() < f64::EPSILON);
        assert_eq!(normalized_to_plain(&spec, 0.0), 0.0);
        assert_eq!(normalized_to_plain(&spec, 0.49), 1.0);
        assert_eq!(normalized_to_plain(&spec, 1.0), 2.0);
        assert_eq!(plain_to_normalized(&spec, 2.0), 1.0);
    }

    #[test]
    fn param_spec_wire_schema_is_camel_case_with_legacy_aliases() {
        let spec = ParamSpec::choice("mode", "Mode", ["Clean", "Drive"], 1);
        let value = serde_json::to_value(&spec).unwrap();
        assert_eq!(value["kind"]["choice"]["values"][1], "Drive");
        assert_eq!(value["defaultNormalized"], 1.0);
        assert_eq!(value["stepCount"], 1);
        assert_eq!(value["flags"]["readOnly"], false);
        assert_eq!(value["flags"]["programChange"], false);
        assert_eq!(value["midiMappings"], serde_json::json!([]));
        assert!(value.get("default_normalized").is_none());
        assert!(value.get("step_count").is_none());
        assert!(value.get("midi_mappings").is_none());
        assert!(value["flags"].get("read_only").is_none());
        assert!(value["flags"].get("program_change").is_none());

        let legacy = serde_json::json!({
            "id": "mode",
            "name": "Mode",
            "kind": { "Choice": { "values": ["Clean", "Drive"] } },
            "default_normalized": 1.0,
            "unit": null,
            "step_count": 1,
            "flags": {
                "automatable": true,
                "bypass": false,
                "read_only": false
            }
        });
        let decoded: ParamSpec = serde_json::from_value(legacy).unwrap();
        assert_eq!(decoded, spec);

        let program_change: ParamSpec = serde_json::from_value(serde_json::json!({
            "id": "program",
            "name": "Program",
            "kind": { "choice": { "values": ["Init", "Lead"] } },
            "defaultNormalized": 0.0,
            "unit": null,
            "stepCount": 1,
            "flags": {
                "automatable": true,
                "bypass": false,
                "readOnly": false,
                "programChange": true
            },
            "midiMappings": []
        }))
        .unwrap();
        assert!(program_change.flags.program_change);

        let legacy_bool = serde_json::json!("Bool");
        assert_eq!(
            serde_json::from_value::<ParamKind>(legacy_bool).unwrap(),
            ParamKind::Bool
        );
    }

    #[test]
    fn param_flag_builders_mark_bypass_read_only_and_non_automatable() {
        let read_only_spec = ParamSpec::float("meter", "Meter", 0.0, 1.0, 0.0).as_read_only();
        assert!(read_only_spec.flags.read_only);
        assert!(!read_only_spec.flags.automatable);

        let non_automatable_spec =
            ParamSpec::choice("quality", "Quality", ["Eco", "High"], 1).with_automatable(false);
        assert!(!non_automatable_spec.flags.automatable);
        assert!(!non_automatable_spec.flags.read_only);

        let float = FloatParam::new("meter", "Meter", 0.0, 1.0, 0.0)
            .with_unit("dB")
            .as_read_only();
        let float_spec = float.spec();
        assert_eq!(float_spec.unit.as_deref(), Some("dB"));
        assert!(float_spec.flags.read_only);
        assert!(!float_spec.flags.automatable);

        let bypass = BoolParam::new("bypass", "Bypass", false).as_bypass();
        assert!(bypass.spec().flags.bypass);
        assert!(
            BoolParam::bypass("hard_bypass", "Hard Bypass", false)
                .spec()
                .flags
                .bypass
        );

        let choice =
            ChoiceParam::new("mode", "Mode", ["Clean", "Drive"], 0).with_automatable(false);
        assert!(!choice.spec().flags.automatable);

        let program = ChoiceParam::new("program", "Program", ["Init", "Lead"], 0)
            .with_automatable(false)
            .as_program_change();
        assert!(program.spec().flags.program_change);
        assert!(program.spec().flags.automatable);
    }

    #[test]
    fn param_midi_mapping_builders_and_validation() {
        let spec = ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5)
            .with_midi_cc(7)
            .with_channel_midi_cc(midi::PITCH_BEND, 2);
        assert_eq!(
            spec.midi_mappings,
            vec![
                ParamMidiMapping {
                    controller: 7,
                    channel: None
                },
                ParamMidiMapping {
                    controller: midi::PITCH_BEND,
                    channel: Some(2)
                }
            ]
        );
        validate_param_specs(&[spec]).unwrap();

        let float = FloatParam::new("cutoff", "Cutoff", 20.0, 20_000.0, 1_000.0).with_midi_cc(74);
        assert_eq!(float.spec().midi_mappings[0].controller, 74);

        let bool_param = BoolParam::new("hold", "Hold", false).with_channel_midi_cc(64, 1);
        assert_eq!(bool_param.spec().midi_mappings[0].channel, Some(1));

        let choice_param = ChoiceParam::new("program", "Program", ["A", "B"], 0)
            .with_midi_cc(midi::PROGRAM_CHANGE);
        assert_eq!(
            choice_param.spec().midi_mappings[0].controller,
            midi::PROGRAM_CHANGE
        );
    }

    #[test]
    fn validates_param_specs_for_host_and_bridge_schema() {
        let specs = vec![
            ParamSpec::float("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
            ParamSpec::bool("bypass", "Bypass", false).as_bypass(),
            ParamSpec::choice("mode", "Mode", ["Clean", "Drive"], 0),
        ];
        validate_param_specs(&specs).unwrap();
        assert!(ParamRegistry::try_new(specs).is_ok());

        let duplicate = vec![
            ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5),
            ParamSpec::bool("gain", "Gain Toggle", false),
        ];
        assert!(matches!(
            validate_param_specs(&duplicate),
            Err(ParamSpecError::DuplicateId { id }) if id == "gain"
        ));

        let empty_id = vec![ParamSpec::float(" ", "Gain", 0.0, 1.0, 0.5)];
        assert!(matches!(
            validate_param_specs(&empty_id),
            Err(ParamSpecError::EmptyId { index: 0 })
        ));

        let mut control = ParamSpec::float("gain", "Gain\nBad", 0.0, 1.0, 0.5);
        assert!(matches!(
            validate_param_specs(&[control.clone()]),
            Err(ParamSpecError::ControlCharacter { field: "name", .. })
        ));
        control.name = "Gain".to_string();
        control.unit = Some("dB\nBad".to_string());
        assert!(matches!(
            validate_param_specs(&[control]),
            Err(ParamSpecError::ControlCharacter { field: "unit", .. })
        ));

        let invalid_default = ParamSpec {
            default_normalized: 1.1,
            ..ParamSpec::bool("toggle", "Toggle", false)
        };
        assert!(matches!(
            validate_param_specs(&[invalid_default]),
            Err(ParamSpecError::InvalidDefaultNormalized { .. })
        ));

        let invalid_range = ParamSpec::float("flat", "Flat", 1.0, 1.0, 1.0);
        assert!(matches!(
            validate_param_specs(&[invalid_range]),
            Err(ParamSpecError::InvalidFloatRange { .. })
        ));

        let empty_choice = ParamSpec::choice("mode", "Mode", ["Clean", ""], 0);
        assert!(matches!(
            validate_param_specs(&[empty_choice]),
            Err(ParamSpecError::EmptyChoiceLabel { index: 1, .. })
        ));

        let read_only_automatable = ParamSpec {
            flags: ParamFlags {
                automatable: true,
                bypass: false,
                read_only: true,
                program_change: false,
            },
            ..ParamSpec::float("meter", "Meter", 0.0, 1.0, 0.0)
        };
        assert!(matches!(
            validate_param_specs(&[read_only_automatable]),
            Err(ParamSpecError::ReadOnlyAutomatable { id }) if id == "meter"
        ));

        let invalid_midi_controller =
            ParamSpec::float("mapped", "Mapped", 0.0, 1.0, 0.0).with_midi_cc(999);
        assert!(matches!(
            validate_param_specs(&[invalid_midi_controller]),
            Err(ParamSpecError::InvalidMidiController {
                controller: 999,
                ..
            })
        ));

        let invalid_midi_channel =
            ParamSpec::float("mapped", "Mapped", 0.0, 1.0, 0.0).with_channel_midi_cc(7, 16);
        assert!(matches!(
            validate_param_specs(&[invalid_midi_channel]),
            Err(ParamSpecError::InvalidMidiChannel { channel: 16, .. })
        ));

        let duplicate_midi_mapping = ParamSpec::float("mapped", "Mapped", 0.0, 1.0, 0.0)
            .with_midi_cc(7)
            .with_midi_cc(7);
        assert!(matches!(
            validate_param_specs(&[duplicate_midi_mapping]),
            Err(ParamSpecError::DuplicateMidiMapping { controller: 7, .. })
        ));
    }

    #[test]
    fn formats_and_parses_choice_labels() {
        let spec = ParamSpec::choice("mode", "Mode", ["Clean", "Drive", "Fuzz"], 0);
        assert_eq!(format_normalized_value(&spec, 0.0), "Clean");
        assert_eq!(format_normalized_value(&spec, 0.51), "Drive");
        assert_eq!(format_normalized_value(&spec, 1.0), "Fuzz");
        assert_eq!(parse_normalized_value(&spec, "Drive"), Some(0.5));
        assert_eq!(parse_normalized_value(&spec, "fuzz"), Some(1.0));
        assert_eq!(parse_normalized_value(&spec, "0"), Some(0.0));
        assert_eq!(parse_normalized_value(&spec, "missing"), None);
    }

    #[test]
    fn choice_param_snaps_to_nearest_index() {
        let param = ChoiceParam::new("mode", "Mode", ["Clean", "Drive", "Fuzz"], 99);
        assert_eq!(param.index(), 2);
        assert_eq!(param.value(), Some("Fuzz"));
        assert_eq!(param.normalized(), 1.0);

        param.set_normalized(0.1);
        assert_eq!(param.index(), 0);
        assert_eq!(param.value(), Some("Clean"));
        assert_eq!(param.normalized(), 0.0);

        param.set_normalized(0.74);
        assert_eq!(param.index(), 1);
        assert_eq!(param.value(), Some("Drive"));
        assert_eq!(param.normalized(), 0.5);

        param.set_normalized(0.75);
        assert_eq!(param.index(), 2);
        assert_eq!(param.value(), Some("Fuzz"));
        assert_eq!(param.normalized(), 1.0);
    }

    #[test]
    fn empty_choice_param_is_safe() {
        let param = ChoiceParam::new("empty", "Empty", std::iter::empty::<&str>(), 4);
        assert_eq!(param.index(), 0);
        assert_eq!(param.value(), None);
        assert_eq!(param.normalized(), 0.0);
        assert_eq!(param.spec().step_count, Some(0));
    }

    #[test]
    fn smoother_reaches_target() {
        let mut smoother = LinearSmoother::new(0.0);
        smoother.set_target(1.0, 4);
        let values: Vec<f32> = (0..4).map(|_| smoother.next_value()).collect();
        assert_eq!(values, vec![0.25, 0.5, 0.75, 1.0]);
    }
}
