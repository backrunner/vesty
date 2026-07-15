use vesty::prelude::*;

const PROGRAM_LIST_ID: u32 = 1;
const PROGRAM_LABELS: [&str; 3] = ["Init", "Bright Lead", "Soft Bass"];
const SYSEX_EXPERIMENTAL_ID: u8 = 0x7D;
const DEFAULT_BRIGHTNESS: f32 = 0.5;
const DEFAULT_TUNING_CENTS: f32 = 0.0;
const TUNING_RANGE_CENTS: f64 = 200.0;

static SYNTH_PROGRAMS: &[Program] = &[
    Program::new("Init"),
    Program::new("Bright Lead"),
    Program::new("Soft Bass"),
];
static SYNTH_PROGRAM_LISTS: &[ProgramList] =
    &[ProgramList::new(PROGRAM_LIST_ID, "Factory", SYNTH_PROGRAMS)];
static INIT_ATTRIBUTES: &[ProgramAttribute] = &[ProgramAttribute::new("category", "Lead")];
static BRIGHT_ATTRIBUTES: &[ProgramAttribute] = &[
    ProgramAttribute::new("category", "Lead"),
    ProgramAttribute::new("character", "Bright"),
];
static BASS_ATTRIBUTES: &[ProgramAttribute] = &[
    ProgramAttribute::new("category", "Bass"),
    ProgramAttribute::new("character", "Soft"),
];
static COMMON_PITCH_NAMES: &[ProgramPitchName] = &[
    ProgramPitchName::new(36, "Bass C"),
    ProgramPitchName::new(60, "Middle C"),
    ProgramPitchName::new(72, "High C"),
];
static NOTE_EXPRESSION_TYPES: &[NoteExpressionValueType] = &[
    NoteExpressionValueType::new(note_expression::BRIGHTNESS, "Brightness", "Bright")
        .with_range(0.0, 1.0, DEFAULT_BRIGHTNESS as f64)
        .with_flags(NoteExpressionValueFlags::ABSOLUTE),
    NoteExpressionValueType::new(note_expression::TUNING, "Tuning", "Tune")
        .with_units("cent")
        .with_range(0.0, 1.0, 0.5)
        .with_flags(NoteExpressionValueFlags::ABSOLUTE_BIPOLAR),
];
static NOTE_EXPRESSION_MAPPINGS: &[NoteExpressionPhysicalUiMapping] = &[
    NoteExpressionPhysicalUiMapping::new(physical_ui::PRESSURE, note_expression::BRIGHTNESS),
    NoteExpressionPhysicalUiMapping::new(physical_ui::X_MOVEMENT, note_expression::TUNING),
];

#[derive(Params)]
pub struct SynthParams {
    pub level: FloatParam,
    pub program: ChoiceParam,
}

impl Default for SynthParams {
    fn default() -> Self {
        Self {
            level: FloatParam::new("level", "Level", 0.0, 1.0, 0.5),
            program: ChoiceParam::new("program", "Program", PROGRAM_LABELS, 0)
                .with_midi_mapping(vesty::params::midi::PROGRAM_CHANGE, None)
                .as_program_change(),
        }
    }
}

#[derive(Default)]
pub struct SynthPlugin {
    params: SynthParams,
}

impl Plugin for SynthPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Vesty MIDI Synth",
        vendor: "Vesty",
        url: "https://github.com/backrunner/vesty",
        email: "",
        version: "0.1.0",
        class_id: *b"VESTYSYNTH000001",
        kind: PluginKind::Instrument,
    };

    type Params = SynthParams;
    type Kernel = SynthKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, init: KernelInit) -> Self::Kernel {
        SynthKernel {
            sample_rate: init.sample_rate,
            phase: 0.0,
            active_note: None,
            brightness: DEFAULT_BRIGHTNESS,
            tuning_cents: DEFAULT_TUNING_CENTS,
            sysex_level_override: None,
            level: self.params.resolve_or_invalid("level"),
        }
    }

    fn program_lists(&self) -> &'static [ProgramList] {
        SYNTH_PROGRAM_LISTS
    }

    fn apply_program(&self, list_id: u32, program_index: usize) -> Result<bool, StateError> {
        if list_id != PROGRAM_LIST_ID || program_index >= SYNTH_PROGRAMS.len() {
            return Err(StateError::custom("unknown synth program"));
        }

        self.params
            .program
            .set_normalized(normalized_for_program_index(program_index));
        self.params
            .level
            .set_normalized(program_level(program_index).unwrap_or(0.5));
        Ok(true)
    }

    fn program_data_supported(&self, list_id: u32) -> bool {
        list_id == PROGRAM_LIST_ID
    }

    fn save_program_data(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> Result<Option<serde_json::Value>, StateError> {
        if list_id != PROGRAM_LIST_ID || program_index >= SYNTH_PROGRAMS.len() {
            return Err(StateError::custom("unknown synth program"));
        }

        let current_program = self.params.program.index();
        let level = if current_program == program_index {
            self.params.level.normalized()
        } else {
            program_level(program_index).unwrap_or(0.5)
        };

        Ok(Some(serde_json::json!({
            "programIndex": program_index,
            "programName": SYNTH_PROGRAMS[program_index].name,
            "levelNormalized": level,
        })))
    }

    fn load_program_data(
        &self,
        list_id: u32,
        program_index: usize,
        data: serde_json::Value,
    ) -> Result<bool, StateError> {
        if list_id != PROGRAM_LIST_ID || program_index >= SYNTH_PROGRAMS.len() {
            return Err(StateError::custom("unknown synth program"));
        }

        if let Some(encoded_index) = data.get("programIndex").and_then(serde_json::Value::as_u64)
            && encoded_index as usize != program_index
        {
            return Err(StateError::custom("program data index mismatch"));
        }

        let level = data
            .get("levelNormalized")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| StateError::custom("program data missing levelNormalized"))?;
        if !level.is_finite() {
            return Err(StateError::custom(
                "program data levelNormalized must be finite",
            ));
        }

        self.params
            .program
            .set_normalized(normalized_for_program_index(program_index));
        self.params.level.set_normalized(level.clamp(0.0, 1.0));
        Ok(true)
    }

    fn program_attributes(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramAttribute] {
        if list_id != PROGRAM_LIST_ID {
            return &[];
        }

        match program_index {
            0 => INIT_ATTRIBUTES,
            1 => BRIGHT_ATTRIBUTES,
            2 => BASS_ATTRIBUTES,
            _ => &[],
        }
    }

    fn program_pitch_names(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramPitchName] {
        if list_id == PROGRAM_LIST_ID && program_index < SYNTH_PROGRAMS.len() {
            COMMON_PITCH_NAMES
        } else {
            &[]
        }
    }

    fn note_expression_value_types(&self) -> &'static [NoteExpressionValueType] {
        NOTE_EXPRESSION_TYPES
    }

    fn note_expression_physical_ui_mappings(&self) -> &'static [NoteExpressionPhysicalUiMapping] {
        NOTE_EXPRESSION_MAPPINGS
    }
}

fn program_level(program_index: usize) -> Option<f64> {
    match program_index {
        0 => Some(0.5),
        1 => Some(0.82),
        2 => Some(0.36),
        _ => None,
    }
}

fn normalized_for_program_index(program_index: usize) -> f64 {
    if SYNTH_PROGRAMS.len() <= 1 {
        0.0
    } else {
        program_index.min(SYNTH_PROGRAMS.len() - 1) as f64 / (SYNTH_PROGRAMS.len() as f64 - 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveNote {
    key: u8,
    note_id: i32,
}

pub struct SynthKernel {
    sample_rate: f64,
    phase: f32,
    active_note: Option<ActiveNote>,
    brightness: f32,
    tuning_cents: f32,
    sysex_level_override: Option<f32>,
    level: ParamHandle,
}

impl AudioKernel for SynthKernel {
    fn reset(&mut self) {
        self.phase = 0.0;
        self.active_note = None;
        self.brightness = DEFAULT_BRIGHTNESS;
        self.tuning_cents = DEFAULT_TUNING_CENTS;
        self.sysex_level_override = None;
    }

    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        for event in context.events() {
            match event {
                Event::NoteOn {
                    key,
                    velocity,
                    note_id,
                    ..
                } if *velocity > 0.0 => {
                    self.active_note = Some(ActiveNote {
                        key: *key,
                        note_id: *note_id,
                    });
                    self.brightness = DEFAULT_BRIGHTNESS;
                    self.tuning_cents = DEFAULT_TUNING_CENTS;
                }
                Event::NoteOff { key, note_id, .. }
                    if note_off_matches_active(self.active_note, *key, *note_id) =>
                {
                    self.active_note = None
                }
                Event::SysEx {
                    data_len,
                    data,
                    truncated,
                    ..
                } => {
                    if let Some(level) = sysex_level_override(data, *data_len, *truncated) {
                        self.sysex_level_override = Some(level);
                    }
                }
                Event::NoteExpressionValue {
                    type_id,
                    note_id,
                    value,
                    ..
                } if note_expression_targets_active(self.active_note, *note_id)
                    && value.is_finite() =>
                {
                    match *type_id {
                        note_expression::BRIGHTNESS => {
                            self.brightness = (*value).clamp(0.0, 1.0) as f32;
                        }
                        note_expression::TUNING => {
                            self.tuning_cents =
                                (((*value).clamp(0.0, 1.0) - 0.5) * TUNING_RANGE_CENTS) as f32;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        context.audio_mut().clear_outputs();
        let Some(note) = self.active_note else {
            return ProcessResult::Continue;
        };

        let initial_level = context.param_normalized(self.level).unwrap_or(0.5);
        let frequency = 440.0_f32
            * 2.0_f32.powf((note.key as f32 - 69.0) / 12.0)
            * 2.0_f32.powf(self.tuning_cents / 1200.0);
        let increment = frequency / self.sample_rate as f32;
        let brightness = self.brightness.clamp(0.0, 1.0);
        let frames = context.audio().frames().min(u32::MAX as usize) as u32;
        let outputs = context.audio().output_channels();
        let (audio, events) = context.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.level, initial_level, frames) {
            let normalized_level = self
                .sysex_level_override
                .unwrap_or(segment.normalized as f32);
            let gain = normalized_level * 0.1;
            for frame in segment.start_sample as usize..segment.end_sample as usize {
                self.phase = (self.phase + increment) % 1.0;
                let fundamental = (self.phase * std::f32::consts::TAU).sin();
                let harmonic = (self.phase * std::f32::consts::TAU * 2.0).sin();
                let sample =
                    (fundamental * (1.0 - brightness * 0.35) + harmonic * brightness * 0.35) * gain;
                for channel in 0..outputs {
                    audio.set_output_sample(channel, frame, sample);
                }
            }
        }

        ProcessResult::Continue
    }
}

fn note_off_matches_active(active: Option<ActiveNote>, key: u8, note_id: i32) -> bool {
    active.is_some_and(|note| note.key == key && (note.note_id == note_id || note_id < 0))
}

fn note_expression_targets_active(active: Option<ActiveNote>, note_id: i32) -> bool {
    active.is_some_and(|note| note.note_id == note_id || note_id < 0)
}

fn sysex_level_override(
    data: &[u8; MAX_SYSEX_BYTES],
    data_len: u16,
    truncated: bool,
) -> Option<f32> {
    if truncated || data_len != 4 {
        return None;
    }
    if data[0] == 0xF0 && data[1] == SYSEX_EXPERIMENTAL_ID && data[2] <= 127 && data[3] == 0xF7 {
        Some(data[2] as f32 / 127.0)
    } else {
        None
    }
}

vesty::export_vst3!(SynthPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_program_metadata_and_program_change_param() {
        let plugin = SynthPlugin::default();
        let specs = plugin.params().specs();
        let program = specs.iter().find(|spec| spec.id == "program").unwrap();

        assert!(program.flags.program_change);
        assert_eq!(program.step_count, Some(2));
        assert_eq!(plugin.program_lists()[0].programs[1].name, "Bright Lead");
        assert_eq!(
            plugin.program_attributes(PROGRAM_LIST_ID, 2)[0].value,
            "Bass"
        );
        assert_eq!(
            plugin.program_pitch_names(PROGRAM_LIST_ID, 1)[1].name,
            "Middle C"
        );

        let expressions = plugin.note_expression_value_types();
        assert_eq!(expressions.len(), 2);
        assert_eq!(expressions[0].type_id, note_expression::BRIGHTNESS);
        assert_eq!(expressions[1].type_id, note_expression::TUNING);
        assert!(expressions.iter().all(NoteExpressionValueType::is_valid));

        let mappings = plugin.note_expression_physical_ui_mappings();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].physical_ui_type_id, physical_ui::PRESSURE);
        assert_eq!(
            mappings[0].note_expression_type_id,
            note_expression::BRIGHTNESS
        );
        assert!(
            mappings
                .iter()
                .all(NoteExpressionPhysicalUiMapping::is_valid)
        );
    }

    #[test]
    fn applies_and_roundtrips_program_data_on_controller_side() {
        let plugin = SynthPlugin::default();

        assert_eq!(plugin.apply_program(PROGRAM_LIST_ID, 1), Ok(true));
        assert_eq!(plugin.params.program.index(), 1);
        assert!((plugin.params.level.normalized() - 0.82).abs() < 0.000_001);

        plugin.params.level.set_normalized(0.91);
        let saved = plugin
            .save_program_data(PROGRAM_LIST_ID, 1)
            .unwrap()
            .unwrap();
        assert_eq!(saved["programName"], "Bright Lead");
        let saved_level = saved["levelNormalized"].as_f64().unwrap();
        assert!((saved_level - 0.91).abs() < 0.000_001);

        assert_eq!(
            plugin.load_program_data(
                PROGRAM_LIST_ID,
                2,
                serde_json::json!({
                    "programIndex": 2,
                    "levelNormalized": 0.24,
                }),
            ),
            Ok(true)
        );
        assert_eq!(plugin.params.program.index(), 2);
        assert!((plugin.params.level.normalized() - 0.24).abs() < 0.000_001);

        assert!(
            plugin
                .load_program_data(
                    PROGRAM_LIST_ID,
                    2,
                    serde_json::json!({
                        "programIndex": 1,
                        "levelNormalized": 0.3,
                    }),
                )
                .is_err()
        );
    }

    #[test]
    fn consumes_sysex_and_note_expression_without_touching_controller_state() {
        let plugin = SynthPlugin::default();
        let mut kernel = plugin.create_kernel(KernelInit {
            sample_rate: 48_000.0,
            max_block_size: 32,
        });
        let mut sysex = [0_u8; MAX_SYSEX_BYTES];
        sysex[..4].copy_from_slice(&[0xF0, SYSEX_EXPERIMENTAL_ID, 96, 0xF7]);
        let events = [
            Event::NoteOn {
                sample_offset: 0,
                channel: 0,
                key: 69,
                velocity: 1.0,
                note_id: 17,
            },
            Event::SysEx {
                sample_offset: 1,
                data_len: 4,
                data: sysex,
                truncated: false,
            },
            Event::NoteExpressionValue {
                sample_offset: 2,
                type_id: note_expression::BRIGHTNESS,
                note_id: 17,
                value: 0.8,
            },
            Event::NoteExpressionValue {
                sample_offset: 3,
                type_id: note_expression::TUNING,
                note_id: 17,
                value: 0.75,
            },
        ];
        let input_channels: [&[f32]; 0] = [];
        let mut left = [0.0_f32; 32];
        let mut right = [0.0_f32; 32];
        {
            let mut outputs: [&mut [f32]; 2] = [&mut left, &mut right];
            let audio = AudioBuffers::new(&input_channels, &mut outputs);
            let mut context =
                ProcessContext::new(audio, plugin.params(), &events, Transport::default());

            assert_eq!(kernel.process(&mut context), ProcessResult::Continue);
        }

        assert_eq!(
            kernel.active_note,
            Some(ActiveNote {
                key: 69,
                note_id: 17,
            })
        );
        assert!((kernel.brightness - 0.8).abs() < 0.000_001);
        assert!((kernel.tuning_cents - 50.0).abs() < 0.000_001);
        assert!((kernel.sysex_level_override.unwrap() - (96.0 / 127.0)).abs() < 0.000_001);
        assert!(left.iter().any(|sample| *sample != 0.0));
        assert_eq!(left, right);
        assert_eq!(plugin.params.program.index(), 0);
        assert!((plugin.params.level.normalized() - 0.5).abs() < 0.000_001);
    }

    #[test]
    fn ignores_unmatched_note_expression_and_invalid_sysex() {
        let plugin = SynthPlugin::default();
        let mut kernel = plugin.create_kernel(KernelInit {
            sample_rate: 48_000.0,
            max_block_size: 16,
        });
        let mut sysex = [0_u8; MAX_SYSEX_BYTES];
        sysex[..4].copy_from_slice(&[0xF0, SYSEX_EXPERIMENTAL_ID, 120, 0xF7]);
        let events = [
            Event::NoteOn {
                sample_offset: 0,
                channel: 0,
                key: 60,
                velocity: 1.0,
                note_id: 5,
            },
            Event::SysEx {
                sample_offset: 1,
                data_len: 4,
                data: sysex,
                truncated: true,
            },
            Event::NoteExpressionValue {
                sample_offset: 2,
                type_id: note_expression::BRIGHTNESS,
                note_id: 99,
                value: 1.0,
            },
            Event::NoteExpressionValue {
                sample_offset: 3,
                type_id: note_expression::TUNING,
                note_id: 99,
                value: 1.0,
            },
            Event::NoteOff {
                sample_offset: 4,
                channel: 0,
                key: 60,
                velocity: 0.0,
                note_id: 99,
            },
        ];
        let input_channels: [&[f32]; 0] = [];
        let mut left = [0.0_f32; 16];
        let mut right = [0.0_f32; 16];
        {
            let mut outputs: [&mut [f32]; 2] = [&mut left, &mut right];
            let audio = AudioBuffers::new(&input_channels, &mut outputs);
            let mut context =
                ProcessContext::new(audio, plugin.params(), &events, Transport::default());

            assert_eq!(kernel.process(&mut context), ProcessResult::Continue);
        }

        assert_eq!(
            kernel.active_note,
            Some(ActiveNote {
                key: 60,
                note_id: 5,
            })
        );
        assert_eq!(kernel.brightness, DEFAULT_BRIGHTNESS);
        assert_eq!(kernel.tuning_cents, DEFAULT_TUNING_CENTS);
        assert_eq!(kernel.sysex_level_override, None);
    }
}
