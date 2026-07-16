use std::collections::BTreeSet;

use crate::{MAX_MIDI_CONTROLLER, ParamKind, ParamSpec, ParamSpecError};

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
