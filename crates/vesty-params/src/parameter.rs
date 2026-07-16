use std::sync::atomic::{AtomicU32, Ordering};

use crate::{
    ParamError, ParamHandle, ParamKind, ParamSpec, choice_index_from_normalized,
    normalized_for_choice_index, normalized_to_plain,
};

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
