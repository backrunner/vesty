use serde::Serialize;

pub mod note_expression {
    pub const VOLUME: u32 = 0;
    pub const PAN: u32 = 1;
    pub const TUNING: u32 = 2;
    pub const VIBRATO: u32 = 3;
    pub const EXPRESSION: u32 = 4;
    pub const BRIGHTNESS: u32 = 5;
    pub const TEXT: u32 = 6;
    pub const PHONEME: u32 = 7;
    pub const CUSTOM_START: u32 = 100_000;
    pub const CUSTOM_END: u32 = 200_000;
    pub const INVALID: u32 = u32::MAX;
}

pub mod physical_ui {
    pub const X_MOVEMENT: u32 = 0;
    pub const Y_MOVEMENT: u32 = 1;
    pub const PRESSURE: u32 = 2;
    pub const INVALID: u32 = u32::MAX;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Program {
    pub name: &'static str,
}

impl Program {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProgramAttribute {
    pub id: &'static str,
    pub value: &'static str,
}

impl ProgramAttribute {
    pub const fn new(id: &'static str, value: &'static str) -> Self {
        Self { id, value }
    }

    pub fn is_valid(&self) -> bool {
        !self.id.is_empty()
            && !self.id.as_bytes().contains(&0)
            && !self.value.as_bytes().contains(&0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProgramPitchName {
    pub midi_pitch: i16,
    pub name: &'static str,
}

impl ProgramPitchName {
    pub const fn new(midi_pitch: i16, name: &'static str) -> Self {
        Self { midi_pitch, name }
    }

    pub fn is_valid(&self) -> bool {
        (0..=127).contains(&self.midi_pitch)
            && !self.name.is_empty()
            && !self.name.as_bytes().contains(&0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProgramList {
    pub id: u32,
    pub name: &'static str,
    pub programs: &'static [Program],
}

impl ProgramList {
    pub const fn new(id: u32, name: &'static str, programs: &'static [Program]) -> Self {
        Self { id, name, programs }
    }

    pub fn is_empty(&self) -> bool {
        self.programs.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct NoteExpressionValueFlags {
    pub bipolar: bool,
    pub one_shot: bool,
    pub absolute: bool,
}

impl NoteExpressionValueFlags {
    pub const NONE: Self = Self {
        bipolar: false,
        one_shot: false,
        absolute: false,
    };

    pub const BIPOLAR: Self = Self {
        bipolar: true,
        one_shot: false,
        absolute: false,
    };

    pub const ABSOLUTE: Self = Self {
        bipolar: false,
        one_shot: false,
        absolute: true,
    };

    pub const ABSOLUTE_BIPOLAR: Self = Self {
        bipolar: true,
        one_shot: false,
        absolute: true,
    };

    pub const ONE_SHOT: Self = Self {
        bipolar: false,
        one_shot: true,
        absolute: false,
    };
}

impl Default for NoteExpressionValueFlags {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct NoteExpressionValueType {
    pub type_id: u32,
    pub title: &'static str,
    pub short_title: &'static str,
    pub units: &'static str,
    pub default_value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub step_count: i32,
    pub flags: NoteExpressionValueFlags,
}

impl NoteExpressionValueType {
    pub const fn new(type_id: u32, title: &'static str, short_title: &'static str) -> Self {
        Self {
            type_id,
            title,
            short_title,
            units: "",
            default_value: 0.0,
            minimum: 0.0,
            maximum: 1.0,
            step_count: 0,
            flags: NoteExpressionValueFlags::NONE,
        }
    }

    #[must_use]
    pub const fn with_units(mut self, units: &'static str) -> Self {
        self.units = units;
        self
    }

    #[must_use]
    pub const fn with_range(mut self, minimum: f64, maximum: f64, default_value: f64) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self.default_value = default_value;
        self
    }

    #[must_use]
    pub const fn with_step_count(mut self, step_count: i32) -> Self {
        self.step_count = step_count;
        self
    }

    #[must_use]
    pub const fn with_flags(mut self, flags: NoteExpressionValueFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.type_id != note_expression::INVALID
            && !self.title.is_empty()
            && self.default_value.is_finite()
            && self.minimum.is_finite()
            && self.maximum.is_finite()
            && self.minimum <= self.default_value
            && self.default_value <= self.maximum
            && self.minimum < self.maximum
            && self.step_count >= 0
            && !contains_control_chars(self.title)
            && !contains_control_chars(self.short_title)
            && !contains_control_chars(self.units)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct NoteExpressionPhysicalUiMapping {
    pub physical_ui_type_id: u32,
    pub note_expression_type_id: u32,
}

impl NoteExpressionPhysicalUiMapping {
    pub const fn new(physical_ui_type_id: u32, note_expression_type_id: u32) -> Self {
        Self {
            physical_ui_type_id,
            note_expression_type_id,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.physical_ui_type_id != physical_ui::INVALID
            && self.physical_ui_type_id <= physical_ui::PRESSURE
            && self.note_expression_type_id != note_expression::INVALID
    }
}

pub(crate) fn contains_control_chars(value: &str) -> bool {
    value.chars().any(char::is_control)
}
