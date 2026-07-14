use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use vesty_params::{ParamCollection, ParamHandle};

pub const MAX_METER_CHANNELS: usize = 8;
pub const MAX_NOTE_EXPRESSION_TEXT_UNITS: usize = 64;
pub const MAX_SYSEX_BYTES: usize = 256;
pub const MAX_AUDIO_OUTPUT_BUSES: usize = 4;
pub const MAX_AUDIO_OUTPUT_CHANNELS: usize = 8;
pub const RELEASE_SMOKE_CHECKS: &[&str] = &[
    "scan",
    "load",
    "ui",
    "ui_host_param",
    "meter_stream",
    "automation",
    "buffer_sample_rate_change",
    "save_restore",
    "offline_render",
];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginKind {
    AudioEffect,
    Instrument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: &'static str,
    pub vendor: &'static str,
    pub url: &'static str,
    pub email: &'static str,
    pub version: &'static str,
    pub class_id: [u8; 16],
    pub kind: PluginKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AudioOutputBus {
    pub name: &'static str,
    pub channels: u16,
}

impl AudioOutputBus {
    pub const fn new(name: &'static str, channels: u16) -> Self {
        Self { name, channels }
    }

    pub const fn mono(name: &'static str) -> Self {
        Self::new(name, 1)
    }

    pub const fn stereo(name: &'static str) -> Self {
        Self::new(name, 2)
    }

    pub fn is_valid(&self) -> bool {
        !self.name.is_empty()
            && !contains_control_chars(self.name)
            && matches!(self.channels, 1 | 2)
    }
}

pub static DEFAULT_AUDIO_OUTPUT_BUSES: [AudioOutputBus; 1] = [AudioOutputBus::stereo("Output")];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostQuirkSeverity {
    Info,
    Warning,
    Required,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostQuirkArea {
    Scanning,
    Editor,
    Automation,
    State,
    Render,
    Meter,
    Platform,
    Packaging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HostQuirk {
    pub area: HostQuirkArea,
    pub severity: HostQuirkSeverity,
    pub summary: &'static str,
    pub mitigation: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct HostProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub platforms: &'static [&'static str],
    pub notes: &'static [&'static str],
    pub quirks: &'static [HostQuirk],
    pub required_smoke_checks: &'static [&'static str],
}

const REAPER_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Scanning,
        severity: HostQuirkSeverity::Info,
        summary: "REAPER keeps plugin scan/cache state per installation and architecture.",
        mitigation: "Collect scan evidence from the target architecture cache or force a rescan before release smoke.",
    },
    HostQuirk {
        area: HostQuirkArea::Editor,
        severity: HostQuirkSeverity::Required,
        summary: "Editor open/close and parameter relay must be proven with host-side evidence.",
        mitigation: "Use UI trace plus host parameter watch evidence for `ui` and `ui_host_param` checks.",
    },
];

const CUBASE_NUENDO_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Packaging,
        severity: HostQuirkSeverity::Required,
        summary: "Treat Steinberg hosts as the strict reference path for VST3 metadata and validator behavior.",
        mitigation: "Run Steinberg validator and collect Cubase/Nuendo scan/load/UI/automation/buffer-size/sample-rate/save/offline render evidence.",
    },
    HostQuirk {
        area: HostQuirkArea::Automation,
        severity: HostQuirkSeverity::Warning,
        summary: "Latency or IO affecting parameter edits must notify the host with the correct restart flags.",
        mitigation: "Keep `HostChangeFlags` tests and collect host automation evidence for latency-affecting parameters.",
    },
];

const BITWIG_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Platform,
        severity: HostQuirkSeverity::Warning,
        summary: "Linux UI support is scoped to X11 for the MVP; Wayland remains experimental.",
        mitigation: "Collect Bitwig Linux smoke on X11 with WebKitGTK installed and mark Wayland separately.",
    },
    HostQuirk {
        area: HostQuirkArea::Meter,
        severity: HostQuirkSeverity::Required,
        summary: "Meter/analyzer streams are latest-wins and must not block audio processing.",
        mitigation: "Collect `meter_stream` evidence while the UI is open and automation/offline render evidence separately.",
    },
];

const ABLETON_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::Editor,
        severity: HostQuirkSeverity::Required,
        summary: "Floating editor lifecycle and WebView attach/detach behavior need host smoke evidence.",
        mitigation: "Open/close the editor repeatedly and record UI plus UI-to-host parameter relay logs.",
    },
    HostQuirk {
        area: HostQuirkArea::Render,
        severity: HostQuirkSeverity::Required,
        summary: "Offline render must be validated independently from realtime playback.",
        mitigation: "Collect an `offline_render` marker from a real Ableton Live render pass.",
    },
];

const STUDIO_ONE_QUIRKS: &[HostQuirk] = &[
    HostQuirk {
        area: HostQuirkArea::State,
        severity: HostQuirkSeverity::Required,
        summary: "Project save/restore must preserve params, custom state and UI config revision.",
        mitigation: "Collect `save_restore` evidence after closing and reopening a project with all examples loaded.",
    },
    HostQuirk {
        area: HostQuirkArea::Automation,
        severity: HostQuirkSeverity::Required,
        summary: "Begin/perform/end edit ordering must be verified from the host automation path.",
        mitigation: "Record parameter automation and confirm host-side parameter movement evidence.",
    },
];

const HOST_PROFILES: &[HostProfile] = &[
    HostProfile {
        id: "reaper",
        name: "REAPER",
        aliases: &["reaper"],
        platforms: &["macos", "windows", "linux"],
        notes: &["Current local evidence covers REAPER on macOS arm64 only."],
        quirks: REAPER_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "cubase-nuendo",
        name: "Cubase/Nuendo",
        aliases: &["cubase", "nuendo", "steinberg"],
        platforms: &["macos", "windows"],
        notes: &["Required release host; evidence is currently external/manual."],
        quirks: CUBASE_NUENDO_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "bitwig",
        name: "Bitwig Studio",
        aliases: &["bitwig", "bitwig-studio"],
        platforms: &["macos", "windows", "linux-x11"],
        notes: &["Wayland support is experimental until a separate smoke path exists."],
        quirks: BITWIG_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "ableton-live",
        name: "Ableton Live",
        aliases: &["ableton", "live", "ableton-live"],
        platforms: &["macos", "windows"],
        notes: &["Evidence is currently external/manual."],
        quirks: ABLETON_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
    HostProfile {
        id: "studio-one",
        name: "Studio One",
        aliases: &["studio-one", "studio one", "presonus"],
        platforms: &["macos", "windows"],
        notes: &["Evidence is currently external/manual."],
        quirks: STUDIO_ONE_QUIRKS,
        required_smoke_checks: RELEASE_SMOKE_CHECKS,
    },
];

pub fn host_profiles() -> &'static [HostProfile] {
    HOST_PROFILES
}

pub fn find_host_profile(query: &str) -> Option<&'static HostProfile> {
    let query = normalize_host_query(query);
    HOST_PROFILES.iter().find(|profile| {
        normalize_host_query(profile.id) == query
            || normalize_host_query(profile.name) == query
            || profile
                .aliases
                .iter()
                .any(|alias| normalize_host_query(alias) == query)
    })
}

fn normalize_host_query(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StateError {
    #[error("state serialization failed: {0}")]
    Serialize(String),
    #[error("state deserialization failed: {0}")]
    Deserialize(String),
    #[error("custom state error: {0}")]
    Custom(String),
}

impl StateError {
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }
}

pub trait PluginState {
    type State: Serialize + DeserializeOwned + 'static;

    fn save_state(&self) -> Self::State;
    fn load_state(&self, state: Self::State) -> Result<(), StateError>;
}

pub fn save_plugin_state<P: PluginState>(plugin: &P) -> Result<serde_json::Value, StateError> {
    serde_json::to_value(plugin.save_state())
        .map_err(|error| StateError::Serialize(error.to_string()))
}

pub fn load_plugin_state<P: PluginState>(
    plugin: &P,
    value: serde_json::Value,
) -> Result<(), StateError> {
    let state = serde_json::from_value(value)
        .map_err(|error| StateError::Deserialize(error.to_string()))?;
    plugin.load_state(state)
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

fn contains_control_chars(value: &str) -> bool {
    value.chars().any(char::is_control)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiDescriptor {
    pub assets_dir: String,
    pub dev_url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub resizable: bool,
}

impl UiDescriptor {
    pub fn web_assets(assets_dir: impl Into<String>) -> Self {
        Self {
            assets_dir: assets_dir.into(),
            dev_url: None,
            width: 900,
            height: 560,
            min_width: 640,
            min_height: 420,
            resizable: true,
        }
    }

    pub fn with_dev_url(mut self, dev_url: impl Into<String>) -> Self {
        self.dev_url = Some(dev_url.into());
        self
    }

    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    #[must_use]
    pub fn with_min_size(mut self, min_width: u32, min_height: u32) -> Self {
        self.min_width = min_width;
        self.min_height = min_height;
        self
    }

    #[must_use]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KernelInit {
    pub sample_rate: f64,
    pub max_block_size: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct PrepareContext {
    pub sample_rate: f64,
    pub max_block_size: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Transport {
    pub playing: bool,
    pub tempo_bpm: Option<f64>,
    pub position_samples: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProcessMode {
    #[default]
    Realtime,
    Prefetch,
    Offline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    NoteOn {
        sample_offset: u32,
        channel: u16,
        key: u8,
        velocity: f32,
        note_id: i32,
    },
    NoteOff {
        sample_offset: u32,
        channel: u16,
        key: u8,
        velocity: f32,
        note_id: i32,
    },
    PolyPressure {
        sample_offset: u32,
        channel: u16,
        key: u8,
        pressure: f32,
        note_id: i32,
    },
    MidiCc {
        sample_offset: u32,
        channel: u16,
        controller: u16,
        value: f32,
    },
    PitchBend {
        sample_offset: u32,
        channel: u16,
        value: f32,
    },
    ChannelPressure {
        sample_offset: u32,
        channel: u16,
        pressure: f32,
    },
    SysEx {
        sample_offset: u32,
        data_len: u16,
        data: [u8; MAX_SYSEX_BYTES],
        truncated: bool,
    },
    NoteExpressionValue {
        sample_offset: u32,
        type_id: u32,
        note_id: i32,
        value: f64,
    },
    NoteExpressionInt {
        sample_offset: u32,
        type_id: u32,
        note_id: i32,
        value: u64,
    },
    NoteExpressionText {
        sample_offset: u32,
        type_id: u32,
        note_id: i32,
        text_len: u8,
        text: [u16; MAX_NOTE_EXPRESSION_TEXT_UNITS],
    },
    Param {
        sample_offset: u32,
        handle: ParamHandle,
        id_hash: u32,
        normalized: f64,
    },
}

impl Event {
    pub const fn sample_offset(&self) -> u32 {
        match *self {
            Self::NoteOn { sample_offset, .. }
            | Self::NoteOff { sample_offset, .. }
            | Self::PolyPressure { sample_offset, .. }
            | Self::MidiCc { sample_offset, .. }
            | Self::PitchBend { sample_offset, .. }
            | Self::ChannelPressure { sample_offset, .. }
            | Self::SysEx { sample_offset, .. }
            | Self::NoteExpressionValue { sample_offset, .. }
            | Self::NoteExpressionInt { sample_offset, .. }
            | Self::NoteExpressionText { sample_offset, .. }
            | Self::Param { sample_offset, .. } => sample_offset,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamAutomationPoint {
    pub sample_offset: u32,
    pub handle: ParamHandle,
    pub id_hash: u32,
    pub normalized: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamAutomationSegment {
    pub start_sample: u32,
    pub end_sample: u32,
    pub normalized: f64,
}

impl ParamAutomationSegment {
    pub fn len(&self) -> u32 {
        self.end_sample.saturating_sub(self.start_sample)
    }

    pub fn is_empty(&self) -> bool {
        self.start_sample >= self.end_sample
    }
}

#[derive(Clone, Debug)]
pub struct ParamAutomationSegments<'a> {
    events: &'a [Event],
    handle: ParamHandle,
    next_event_index: usize,
    cursor: u32,
    block_end: u32,
    current: f64,
    pending: Option<ParamAutomationPoint>,
}

impl<'a> ParamAutomationSegments<'a> {
    pub fn new(
        events: &'a [Event],
        handle: ParamHandle,
        initial_normalized: f64,
        block_frames: u32,
    ) -> Self {
        Self {
            events,
            handle,
            next_event_index: 0,
            cursor: 0,
            block_end: block_frames,
            current: initial_normalized,
            pending: None,
        }
    }

    fn next_point(&mut self) -> Option<ParamAutomationPoint> {
        while let Some(event) = self.events.get(self.next_event_index) {
            self.next_event_index += 1;
            if let Event::Param {
                sample_offset,
                handle,
                id_hash,
                normalized,
            } = *event
                && handle == self.handle
            {
                return Some(ParamAutomationPoint {
                    sample_offset,
                    handle,
                    id_hash,
                    normalized,
                });
            }
        }
        None
    }
}

impl Iterator for ParamAutomationSegments<'_> {
    type Item = ParamAutomationSegment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.block_end {
            return None;
        }

        if let Some(point) = self.pending.take() {
            self.current = point.normalized;
            self.cursor = point.sample_offset.min(self.block_end);
        }

        while let Some(point) = self.next_point() {
            let point_offset = point.sample_offset.min(self.block_end);
            if point_offset > self.cursor {
                let segment = ParamAutomationSegment {
                    start_sample: self.cursor,
                    end_sample: point_offset,
                    normalized: self.current,
                };
                self.pending = Some(ParamAutomationPoint {
                    sample_offset: point_offset,
                    ..point
                });
                self.cursor = point_offset;
                return Some(segment);
            }
            self.current = point.normalized;
            self.cursor = point_offset;
        }

        let segment = ParamAutomationSegment {
            start_sample: self.cursor,
            end_sample: self.block_end,
            normalized: self.current,
        };
        self.cursor = self.block_end;
        Some(segment)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeterFrame {
    pub id_hash: u32,
    pub sample_offset: u32,
    pub channels: u8,
    pub peaks: [f32; MAX_METER_CHANNELS],
    pub rms: [f32; MAX_METER_CHANNELS],
}

impl MeterFrame {
    pub fn new(id_hash: u32, sample_offset: u32) -> Self {
        Self {
            id_hash,
            sample_offset,
            channels: 0,
            peaks: [0.0; MAX_METER_CHANNELS],
            rms: [0.0; MAX_METER_CHANNELS],
        }
    }

    pub fn set_channel(&mut self, channel: usize, peak: f32, rms: f32) -> bool {
        if channel >= MAX_METER_CHANNELS {
            return false;
        }
        self.peaks[channel] = finite_or_zero(peak.abs());
        self.rms[channel] = finite_or_zero(rms.abs());
        self.channels = self.channels.max((channel + 1) as u8);
        true
    }

    pub fn channel_count(&self) -> usize {
        (self.channels as usize).min(MAX_METER_CHANNELS)
    }

    pub fn from_outputs(id_hash: u32, sample_offset: u32, audio: &AudioBuffers<'_>) -> Self {
        let mut frame = Self::new(id_hash, sample_offset);
        let channels = audio.output_channels().min(MAX_METER_CHANNELS);
        for channel_index in 0..channels {
            let Some(channel) = audio.output_channel(channel_index) else {
                continue;
            };
            let mut peak = 0.0_f32;
            let mut sum_squares = 0.0_f32;
            for sample in channel {
                let sample = sample.abs();
                peak = peak.max(sample);
                sum_squares += sample * sample;
            }
            let rms = if channel.is_empty() {
                0.0
            } else {
                (sum_squares / channel.len() as f32).sqrt()
            };
            frame.set_channel(channel_index, peak, rms);
        }
        frame
    }

    pub fn from_outputs_f64(id_hash: u32, sample_offset: u32, audio: &AudioBuffers64<'_>) -> Self {
        let mut frame = Self::new(id_hash, sample_offset);
        let channels = audio.output_channels().min(MAX_METER_CHANNELS);
        for channel_index in 0..channels {
            let Some(channel) = audio.output_channel(channel_index) else {
                continue;
            };
            let mut peak = 0.0_f64;
            let mut sum_squares = 0.0_f64;
            for sample in channel {
                let sample = sample.abs();
                peak = peak.max(sample);
                sum_squares += sample * sample;
            }
            let rms = if channel.is_empty() {
                0.0
            } else {
                (sum_squares / channel.len() as f64).sqrt()
            };
            frame.set_channel(channel_index, peak as f32, rms as f32);
        }
        frame
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

pub trait MeterSink {
    fn push_meter(&mut self, frame: MeterFrame) -> bool;
}

#[derive(Debug)]
pub struct AudioBuffers<'a> {
    inputs: &'a [&'a [f32]],
    outputs: &'a mut [&'a mut [f32]],
}

impl<'a> AudioBuffers<'a> {
    pub fn new(inputs: &'a [&'a [f32]], outputs: &'a mut [&'a mut [f32]]) -> Self {
        Self { inputs, outputs }
    }

    pub fn input_channels(&self) -> usize {
        self.inputs.len()
    }

    pub fn output_channels(&self) -> usize {
        self.outputs.len()
    }

    pub fn frames(&self) -> usize {
        self.outputs
            .first()
            .map(|channel| channel.len())
            .unwrap_or(0)
    }

    pub fn input_channel(&self, channel: usize) -> Option<&[f32]> {
        self.inputs.get(channel).copied()
    }

    pub fn output_channel(&self, channel: usize) -> Option<&[f32]> {
        self.outputs.get(channel).map(|channel| &**channel)
    }

    pub fn clear_outputs(&mut self) {
        for output in self.outputs.iter_mut() {
            output.fill(0.0);
        }
    }

    pub fn copy_input_to_output(&mut self, channel: usize, gain: f32) {
        let Some(input) = self.inputs.get(channel).copied() else {
            return;
        };
        let Some(output) = self.outputs.get_mut(channel) else {
            return;
        };
        let frames = input.len().min(output.len());
        for index in 0..frames {
            output[index] = input[index] * gain;
        }
    }

    pub fn copy_input_to_output_range(
        &mut self,
        channel: usize,
        start: usize,
        end: usize,
        gain: f32,
    ) {
        let Some(input) = self.inputs.get(channel).copied() else {
            return;
        };
        let Some(output) = self.outputs.get_mut(channel) else {
            return;
        };
        let frames = input.len().min(output.len());
        let start = start.min(frames);
        let end = end.min(frames);
        if start >= end {
            return;
        }
        for index in start..end {
            output[index] = input[index] * gain;
        }
    }

    pub fn set_output_sample(&mut self, channel: usize, frame: usize, sample: f32) {
        let Some(output) = self.outputs.get_mut(channel) else {
            return;
        };
        if let Some(slot) = output.get_mut(frame) {
            *slot = sample;
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SidechainBuffers<'a> {
    inputs: &'a [&'a [f32]],
}

impl<'a> SidechainBuffers<'a> {
    pub fn new(inputs: &'a [&'a [f32]]) -> Self {
        Self { inputs }
    }

    pub fn input_channels(&self) -> usize {
        self.inputs.len()
    }

    pub fn frames(&self) -> usize {
        self.inputs
            .first()
            .map(|channel| channel.len())
            .unwrap_or(0)
    }

    pub fn input_channel(&self, channel: usize) -> Option<&[f32]> {
        self.inputs.get(channel).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}

#[derive(Debug)]
pub struct AudioBuffers64<'a> {
    inputs: &'a [&'a [f64]],
    outputs: &'a mut [&'a mut [f64]],
}

impl<'a> AudioBuffers64<'a> {
    pub fn new(inputs: &'a [&'a [f64]], outputs: &'a mut [&'a mut [f64]]) -> Self {
        Self { inputs, outputs }
    }

    pub fn input_channels(&self) -> usize {
        self.inputs.len()
    }

    pub fn output_channels(&self) -> usize {
        self.outputs.len()
    }

    pub fn frames(&self) -> usize {
        self.outputs
            .first()
            .map(|channel| channel.len())
            .unwrap_or(0)
    }

    pub fn input_channel(&self, channel: usize) -> Option<&[f64]> {
        self.inputs.get(channel).copied()
    }

    pub fn output_channel(&self, channel: usize) -> Option<&[f64]> {
        self.outputs.get(channel).map(|channel| &**channel)
    }

    pub fn clear_outputs(&mut self) {
        for output in self.outputs.iter_mut() {
            output.fill(0.0);
        }
    }

    pub fn copy_input_to_output(&mut self, channel: usize, gain: f64) {
        let Some(input) = self.inputs.get(channel).copied() else {
            return;
        };
        let Some(output) = self.outputs.get_mut(channel) else {
            return;
        };
        let frames = input.len().min(output.len());
        for index in 0..frames {
            output[index] = input[index] * gain;
        }
    }

    pub fn copy_input_to_output_range(
        &mut self,
        channel: usize,
        start: usize,
        end: usize,
        gain: f64,
    ) {
        let Some(input) = self.inputs.get(channel).copied() else {
            return;
        };
        let Some(output) = self.outputs.get_mut(channel) else {
            return;
        };
        let frames = input.len().min(output.len());
        let start = start.min(frames);
        let end = end.min(frames);
        if start >= end {
            return;
        }
        for index in start..end {
            output[index] = input[index] * gain;
        }
    }

    pub fn set_output_sample(&mut self, channel: usize, frame: usize, sample: f64) {
        let Some(output) = self.outputs.get_mut(channel) else {
            return;
        };
        if let Some(slot) = output.get_mut(frame) {
            *slot = sample;
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SidechainBuffers64<'a> {
    inputs: &'a [&'a [f64]],
}

impl<'a> SidechainBuffers64<'a> {
    pub fn new(inputs: &'a [&'a [f64]]) -> Self {
        Self { inputs }
    }

    pub fn input_channels(&self) -> usize {
        self.inputs.len()
    }

    pub fn frames(&self) -> usize {
        self.inputs
            .first()
            .map(|channel| channel.len())
            .unwrap_or(0)
    }

    pub fn input_channel(&self, channel: usize) -> Option<&[f64]> {
        self.inputs.get(channel).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}

pub struct ProcessContext<'a> {
    audio: AudioBuffers<'a>,
    sidechain: SidechainBuffers<'a>,
    params: &'a dyn ParamCollection,
    events: &'a [Event],
    transport: Transport,
    process_mode: ProcessMode,
    meters: Option<&'a mut dyn MeterSink>,
}

impl<'a> ProcessContext<'a> {
    pub fn new(
        audio: AudioBuffers<'a>,
        params: &'a dyn ParamCollection,
        events: &'a [Event],
        transport: Transport,
    ) -> Self {
        Self {
            audio,
            sidechain: SidechainBuffers::default(),
            params,
            events,
            transport,
            process_mode: ProcessMode::Realtime,
            meters: None,
        }
    }

    pub fn with_process_mode(mut self, process_mode: ProcessMode) -> Self {
        self.process_mode = process_mode;
        self
    }

    pub fn with_meter_sink(mut self, meters: &'a mut dyn MeterSink) -> Self {
        self.meters = Some(meters);
        self
    }

    pub fn with_sidechain(mut self, sidechain: SidechainBuffers<'a>) -> Self {
        self.sidechain = sidechain;
        self
    }

    pub fn audio(&self) -> &AudioBuffers<'a> {
        &self.audio
    }

    pub fn audio_mut(&mut self) -> &mut AudioBuffers<'a> {
        &mut self.audio
    }

    pub fn audio_mut_and_events(&mut self) -> (&mut AudioBuffers<'a>, &'a [Event]) {
        (&mut self.audio, self.events)
    }

    pub fn sidechain(&self) -> SidechainBuffers<'a> {
        self.sidechain
    }

    pub fn params(&self) -> &dyn ParamCollection {
        self.params
    }

    pub fn param_handle(&self, id: &str) -> Option<ParamHandle> {
        self.params.resolve(id)
    }

    pub fn param_normalized(&self, handle: ParamHandle) -> Option<f64> {
        self.params.get_normalized_by_handle(handle)
    }

    pub fn events(&self) -> &[Event] {
        self.events
    }

    pub fn param_automation(
        &self,
        handle: ParamHandle,
    ) -> impl Iterator<Item = ParamAutomationPoint> + '_ {
        self.events.iter().filter_map(move |event| match *event {
            Event::Param {
                sample_offset,
                handle: event_handle,
                id_hash,
                normalized,
            } if event_handle == handle => Some(ParamAutomationPoint {
                sample_offset,
                handle: event_handle,
                id_hash,
                normalized,
            }),
            _ => None,
        })
    }

    pub fn latest_param_automation(&self, handle: ParamHandle) -> Option<ParamAutomationPoint> {
        self.param_automation(handle).last()
    }

    pub fn param_automation_segments(
        &self,
        handle: ParamHandle,
        default_normalized: f64,
    ) -> ParamAutomationSegments<'_> {
        let initial = self.param_normalized(handle).unwrap_or(default_normalized);
        let block_frames = self.audio.frames().min(u32::MAX as usize) as u32;
        ParamAutomationSegments::new(self.events, handle, initial, block_frames)
    }

    pub fn transport(&self) -> Transport {
        self.transport
    }

    pub fn process_mode(&self) -> ProcessMode {
        self.process_mode
    }

    pub fn emit_meter(&mut self, frame: MeterFrame) -> bool {
        self.meters
            .as_deref_mut()
            .is_some_and(|sink| sink.push_meter(frame))
    }

    pub fn emit_output_meter(&mut self, id_hash: u32, sample_offset: u32) -> bool {
        let frame = MeterFrame::from_outputs(id_hash, sample_offset, &self.audio);
        self.emit_meter(frame)
    }
}

pub struct ProcessContext64<'a> {
    audio: AudioBuffers64<'a>,
    sidechain: SidechainBuffers64<'a>,
    params: &'a dyn ParamCollection,
    events: &'a [Event],
    transport: Transport,
    process_mode: ProcessMode,
    meters: Option<&'a mut dyn MeterSink>,
}

impl<'a> ProcessContext64<'a> {
    pub fn new(
        audio: AudioBuffers64<'a>,
        params: &'a dyn ParamCollection,
        events: &'a [Event],
        transport: Transport,
    ) -> Self {
        Self {
            audio,
            sidechain: SidechainBuffers64::default(),
            params,
            events,
            transport,
            process_mode: ProcessMode::Realtime,
            meters: None,
        }
    }

    pub fn with_process_mode(mut self, process_mode: ProcessMode) -> Self {
        self.process_mode = process_mode;
        self
    }

    pub fn with_meter_sink(mut self, meters: &'a mut dyn MeterSink) -> Self {
        self.meters = Some(meters);
        self
    }

    pub fn with_sidechain(mut self, sidechain: SidechainBuffers64<'a>) -> Self {
        self.sidechain = sidechain;
        self
    }

    pub fn audio(&self) -> &AudioBuffers64<'a> {
        &self.audio
    }

    pub fn audio_mut(&mut self) -> &mut AudioBuffers64<'a> {
        &mut self.audio
    }

    pub fn audio_mut_and_events(&mut self) -> (&mut AudioBuffers64<'a>, &'a [Event]) {
        (&mut self.audio, self.events)
    }

    pub fn sidechain(&self) -> SidechainBuffers64<'a> {
        self.sidechain
    }

    pub fn params(&self) -> &dyn ParamCollection {
        self.params
    }

    pub fn param_handle(&self, id: &str) -> Option<ParamHandle> {
        self.params.resolve(id)
    }

    pub fn param_normalized(&self, handle: ParamHandle) -> Option<f64> {
        self.params.get_normalized_by_handle(handle)
    }

    pub fn events(&self) -> &[Event] {
        self.events
    }

    pub fn param_automation(
        &self,
        handle: ParamHandle,
    ) -> impl Iterator<Item = ParamAutomationPoint> + '_ {
        self.events.iter().filter_map(move |event| match *event {
            Event::Param {
                sample_offset,
                handle: event_handle,
                id_hash,
                normalized,
            } if event_handle == handle => Some(ParamAutomationPoint {
                sample_offset,
                handle: event_handle,
                id_hash,
                normalized,
            }),
            _ => None,
        })
    }

    pub fn latest_param_automation(&self, handle: ParamHandle) -> Option<ParamAutomationPoint> {
        self.param_automation(handle).last()
    }

    pub fn param_automation_segments(
        &self,
        handle: ParamHandle,
        default_normalized: f64,
    ) -> ParamAutomationSegments<'_> {
        let initial = self.param_normalized(handle).unwrap_or(default_normalized);
        let block_frames = self.audio.frames().min(u32::MAX as usize) as u32;
        ParamAutomationSegments::new(self.events, handle, initial, block_frames)
    }

    pub fn transport(&self) -> Transport {
        self.transport
    }

    pub fn process_mode(&self) -> ProcessMode {
        self.process_mode
    }

    pub fn emit_meter(&mut self, frame: MeterFrame) -> bool {
        self.meters
            .as_deref_mut()
            .is_some_and(|sink| sink.push_meter(frame))
    }

    pub fn emit_output_meter(&mut self, id_hash: u32, sample_offset: u32) -> bool {
        let frame = MeterFrame::from_outputs_f64(id_hash, sample_offset, &self.audio);
        self.emit_meter(frame)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessResult {
    Continue,
    Silence,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostChangeFlags(u32);

impl HostChangeFlags {
    pub const NONE: Self = Self(0);
    pub const IO: Self = Self(1 << 0);
    pub const PARAM_VALUES: Self = Self(1 << 1);
    pub const LATENCY: Self = Self(1 << 2);
    pub const PARAM_TITLES: Self = Self(1 << 3);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for HostChangeFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for HostChangeFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

pub trait AudioKernel: Send + 'static {
    const SUPPORTS_F64: bool = false;

    fn prepare(&mut self, _context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
    fn process_f64(&mut self, context: &mut ProcessContext64<'_>) -> ProcessResult {
        let _ = context;
        ProcessResult::Silence
    }
}

pub use AudioKernel as InstrumentKernel;

pub trait Plugin: Send + Sync + 'static {
    const INFO: PluginInfo;

    type Params: ParamCollection + Send + Sync + 'static;
    type Kernel: AudioKernel;

    fn params(&self) -> &Self::Params;
    fn create_kernel(&self, init: KernelInit) -> Self::Kernel;

    fn ui(&self) -> Option<UiDescriptor> {
        None
    }

    fn latency_samples(&self) -> u32 {
        0
    }

    fn tail_samples(&self) -> u32 {
        0
    }

    fn sidechain_inputs(&self) -> u32 {
        0
    }

    fn output_buses(&self) -> &'static [AudioOutputBus] {
        &DEFAULT_AUDIO_OUTPUT_BUSES
    }

    fn program_lists(&self) -> &'static [ProgramList] {
        &[]
    }

    fn apply_program(&self, list_id: u32, program_index: usize) -> Result<bool, StateError> {
        let _ = (list_id, program_index);
        Ok(false)
    }

    fn program_data_supported(&self, list_id: u32) -> bool {
        let _ = list_id;
        false
    }

    fn save_program_data(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> Result<Option<serde_json::Value>, StateError> {
        let _ = (list_id, program_index);
        Ok(None)
    }

    fn load_program_data(
        &self,
        list_id: u32,
        program_index: usize,
        data: serde_json::Value,
    ) -> Result<bool, StateError> {
        let _ = (list_id, program_index, data);
        Ok(false)
    }

    fn program_attributes(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramAttribute] {
        let _ = (list_id, program_index);
        &[]
    }

    fn program_pitch_names(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramPitchName] {
        let _ = (list_id, program_index);
        &[]
    }

    fn note_expression_value_types(&self) -> &'static [NoteExpressionValueType] {
        &[]
    }

    fn note_expression_physical_ui_mappings(&self) -> &'static [NoteExpressionPhysicalUiMapping] {
        &[]
    }

    fn host_changes_for_param(
        &self,
        _id: &str,
        _old_normalized: f64,
        _new_normalized: f64,
    ) -> HostChangeFlags {
        HostChangeFlags::NONE
    }

    fn save_custom_state(&self) -> Result<Option<serde_json::Value>, StateError> {
        Ok(None)
    }

    fn load_custom_state(&self, state: Option<serde_json::Value>) -> Result<(), StateError> {
        let _ = state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vesty_params::{FloatParam, ParamError, ParamSpec};

    struct Params {
        gain: FloatParam,
    }

    impl ParamCollection for Params {
        fn specs(&self) -> Vec<ParamSpec> {
            vec![self.gain.spec()]
        }

        fn get_normalized(&self, id: &str) -> Option<f64> {
            (id == "gain").then(|| self.gain.normalized())
        }

        fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
            if id == "gain" {
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
    fn copies_audio_with_gain() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
        };
        let input = [1.0, -0.5, 0.25];
        let mut output = [0.0; 3];
        let inputs: [&[f32]; 1] = [&input];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let audio = AudioBuffers::new(&inputs, &mut outputs);
        let mut context = ProcessContext::new(audio, &params, &[], Transport::default());
        let gain = context.param_handle("gain").unwrap();
        assert_eq!(gain.index(), 0);
        assert_eq!(context.param_normalized(gain), Some(1.0));
        context.audio_mut().copy_input_to_output(0, 0.5);
        assert_eq!(output, [0.5, -0.25, 0.125]);
    }

    #[test]
    fn process_context_process_mode_defaults_and_overrides() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
        };
        let input = [0.0; 1];
        let mut output = [0.0; 1];
        let inputs: [&[f32]; 1] = [&input];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let audio = AudioBuffers::new(&inputs, &mut outputs);

        let context = ProcessContext::new(audio, &params, &[], Transport::default());
        assert_eq!(context.process_mode(), ProcessMode::Realtime);

        let input = [0.0; 1];
        let mut output = [0.0; 1];
        let inputs: [&[f32]; 1] = [&input];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let audio = AudioBuffers::new(&inputs, &mut outputs);
        let context = ProcessContext::new(audio, &params, &[], Transport::default())
            .with_process_mode(ProcessMode::Offline);

        assert_eq!(context.process_mode(), ProcessMode::Offline);
    }

    #[test]
    fn event_sample_offset_covers_midi_and_param_events() {
        let gain = ParamHandle::from_index(0);
        let events = [
            Event::NoteOn {
                sample_offset: 1,
                channel: 0,
                key: 60,
                velocity: 1.0,
                note_id: -1,
            },
            Event::NoteOff {
                sample_offset: 2,
                channel: 0,
                key: 60,
                velocity: 0.0,
                note_id: -1,
            },
            Event::PolyPressure {
                sample_offset: 3,
                channel: 0,
                key: 60,
                pressure: 0.5,
                note_id: -1,
            },
            Event::MidiCc {
                sample_offset: 4,
                channel: 0,
                controller: 1,
                value: 0.25,
            },
            Event::PitchBend {
                sample_offset: 5,
                channel: 0,
                value: -0.5,
            },
            Event::ChannelPressure {
                sample_offset: 6,
                channel: 0,
                pressure: 0.75,
            },
            Event::SysEx {
                sample_offset: 7,
                data_len: 3,
                data: {
                    let mut data = [0; MAX_SYSEX_BYTES];
                    data[..3].copy_from_slice(&[0xF0, 0x7D, 0xF7]);
                    data
                },
                truncated: false,
            },
            Event::NoteExpressionValue {
                sample_offset: 8,
                type_id: note_expression::BRIGHTNESS,
                note_id: 12,
                value: 0.625,
            },
            Event::NoteExpressionInt {
                sample_offset: 9,
                type_id: note_expression::CUSTOM_START,
                note_id: 12,
                value: 42,
            },
            Event::NoteExpressionText {
                sample_offset: 10,
                type_id: note_expression::TEXT,
                note_id: 12,
                text_len: 0,
                text: [0; MAX_NOTE_EXPRESSION_TEXT_UNITS],
            },
            Event::Param {
                sample_offset: 11,
                handle: gain,
                id_hash: 1,
                normalized: 0.5,
            },
        ];

        assert_eq!(
            events.map(|event| event.sample_offset()),
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
        );
    }

    #[test]
    fn ui_descriptor_builders_override_editor_geometry() {
        let descriptor = UiDescriptor::web_assets("ui")
            .with_dev_url("http://localhost:5173")
            .with_size(777, 333)
            .with_min_size(320, 200)
            .with_resizable(false);

        assert_eq!(descriptor.assets_dir, "ui");
        assert_eq!(descriptor.dev_url.as_deref(), Some("http://localhost:5173"));
        assert_eq!(descriptor.width, 777);
        assert_eq!(descriptor.height, 333);
        assert_eq!(descriptor.min_width, 320);
        assert_eq!(descriptor.min_height, 200);
        assert!(!descriptor.resizable);
    }

    #[test]
    fn copies_audio_with_gain_range() {
        let input = [1.0, 2.0, 3.0, 4.0];
        let mut output = [0.0; 4];
        let inputs: [&[f32]; 1] = [&input];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let mut audio = AudioBuffers::new(&inputs, &mut outputs);

        audio.copy_input_to_output_range(0, 1, 3, 0.5);
        audio.copy_input_to_output_range(0, 3, 99, 2.0);

        assert_eq!(output, [0.0, 1.0, 1.5, 8.0]);
    }

    #[test]
    fn copies_audio64_with_gain_range_set_and_clear() {
        let input_l = [1.0_f64, -2.0, 3.5, -4.5];
        let input_r = [0.25_f64, -1.5, 2.0, 8.0];
        let mut output_l = [9.0_f64; 4];
        let mut output_r = [7.0_f64; 4];

        {
            let inputs: [&[f64]; 2] = [&input_l, &input_r];
            let mut outputs: [&mut [f64]; 2] = [&mut output_l, &mut output_r];
            let mut audio = AudioBuffers64::new(&inputs, &mut outputs);

            assert_eq!(audio.input_channels(), 2);
            assert_eq!(audio.output_channels(), 2);
            assert_eq!(audio.frames(), 4);
            assert_eq!(audio.input_channel(0), Some(&input_l[..]));
            assert_eq!(audio.output_channel(1), Some(&[7.0_f64; 4][..]));

            audio.copy_input_to_output(0, 0.5);
            audio.copy_input_to_output_range(1, 1, 3, -2.0);
            audio.copy_input_to_output_range(1, 3, 99, 0.25);
            audio.set_output_sample(1, 0, 0.125);
            audio.set_output_sample(99, 0, 1.0);
            audio.set_output_sample(0, 99, 1.0);
        }

        assert_eq!(output_l, [0.5, -1.0, 1.75, -2.25]);
        assert_eq!(output_r, [0.125, 3.0, -4.0, 2.0]);

        {
            let inputs: [&[f64]; 0] = [];
            let mut outputs: [&mut [f64]; 2] = [&mut output_l, &mut output_r];
            let mut audio = AudioBuffers64::new(&inputs, &mut outputs);
            audio.clear_outputs();
        }

        assert_eq!(output_l, [0.0; 4]);
        assert_eq!(output_r, [0.0; 4]);
    }

    #[test]
    fn filters_param_automation_by_handle() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
        };
        let gain = ParamHandle::from_index(0);
        let other = ParamHandle::from_index(1);
        let events = [
            Event::NoteOn {
                sample_offset: 0,
                channel: 0,
                key: 60,
                velocity: 1.0,
                note_id: -1,
            },
            Event::Param {
                sample_offset: 4,
                handle: gain,
                id_hash: 7,
                normalized: 0.25,
            },
            Event::Param {
                sample_offset: 8,
                handle: other,
                id_hash: 8,
                normalized: 0.75,
            },
            Event::Param {
                sample_offset: 12,
                handle: gain,
                id_hash: 7,
                normalized: 0.5,
            },
        ];
        let input = [0.0; 16];
        let mut output = [0.0; 16];
        let inputs: [&[f32]; 1] = [&input];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let audio = AudioBuffers::new(&inputs, &mut outputs);
        let context = ProcessContext::new(audio, &params, &events, Transport::default());

        let points = context.param_automation(gain).collect::<Vec<_>>();

        assert_eq!(
            points,
            vec![
                ParamAutomationPoint {
                    sample_offset: 4,
                    handle: gain,
                    id_hash: 7,
                    normalized: 0.25,
                },
                ParamAutomationPoint {
                    sample_offset: 12,
                    handle: gain,
                    id_hash: 7,
                    normalized: 0.5,
                },
            ]
        );
        assert_eq!(
            context.latest_param_automation(gain),
            Some(ParamAutomationPoint {
                sample_offset: 12,
                handle: gain,
                id_hash: 7,
                normalized: 0.5,
            })
        );
        assert_eq!(
            context.latest_param_automation(ParamHandle::from_index(99)),
            None
        );
    }

    #[test]
    fn builds_param_automation_segments() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 0.25),
        };
        let gain = ParamHandle::from_index(0);
        let events = [
            Event::Param {
                sample_offset: 2,
                handle: gain,
                id_hash: 7,
                normalized: 0.5,
            },
            Event::Param {
                sample_offset: 5,
                handle: gain,
                id_hash: 7,
                normalized: 1.0,
            },
        ];
        let input = [0.0; 8];
        let mut output = [0.0; 8];
        let inputs: [&[f32]; 1] = [&input];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let audio = AudioBuffers::new(&inputs, &mut outputs);
        let context = ProcessContext::new(audio, &params, &events, Transport::default());

        let segments = context
            .param_automation_segments(gain, 0.0)
            .collect::<Vec<_>>();

        assert_eq!(
            segments,
            vec![
                ParamAutomationSegment {
                    start_sample: 0,
                    end_sample: 2,
                    normalized: 0.25,
                },
                ParamAutomationSegment {
                    start_sample: 2,
                    end_sample: 5,
                    normalized: 0.5,
                },
                ParamAutomationSegment {
                    start_sample: 5,
                    end_sample: 8,
                    normalized: 1.0,
                },
            ]
        );
        assert_eq!(segments[0].len(), 2);
        assert!(!segments[0].is_empty());
    }

    #[test]
    fn process_context64_exposes_params_events_transport_mode_and_meters() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 0.25),
        };
        let gain = ParamHandle::from_index(0);
        let other = ParamHandle::from_index(1);
        let events = [
            Event::Param {
                sample_offset: 2,
                handle: gain,
                id_hash: 7,
                normalized: 0.5,
            },
            Event::NoteOn {
                sample_offset: 3,
                channel: 0,
                key: 60,
                velocity: 0.75,
                note_id: 42,
            },
            Event::Param {
                sample_offset: 4,
                handle: other,
                id_hash: 8,
                normalized: 0.875,
            },
            Event::Param {
                sample_offset: 5,
                handle: gain,
                id_hash: 7,
                normalized: 0.75,
            },
        ];
        let input_l = [0.0_f64; 8];
        let input_r = [0.0_f64; 8];
        let mut output_l = [0.0_f64; 8];
        let mut output_r = [0.0_f64; 8];
        let transport = Transport {
            playing: true,
            tempo_bpm: Some(123.0),
            position_samples: Some(4096),
        };
        let mut sink = CollectMeterSink::default();

        {
            let inputs: [&[f64]; 2] = [&input_l, &input_r];
            let mut outputs: [&mut [f64]; 2] = [&mut output_l, &mut output_r];
            let audio = AudioBuffers64::new(&inputs, &mut outputs);
            let mut context = ProcessContext64::new(audio, &params, &events, transport)
                .with_process_mode(ProcessMode::Offline)
                .with_meter_sink(&mut sink);

            assert_eq!(context.process_mode(), ProcessMode::Offline);
            assert_eq!(context.transport(), transport);
            assert_eq!(context.events(), &events);
            assert_eq!(context.param_handle("gain"), Some(gain));
            assert_eq!(context.param_normalized(gain), Some(0.25));
            assert_eq!(
                context.latest_param_automation(gain),
                Some(ParamAutomationPoint {
                    sample_offset: 5,
                    handle: gain,
                    id_hash: 7,
                    normalized: 0.75,
                })
            );
            assert_eq!(
                context
                    .param_automation_segments(gain, 0.0)
                    .collect::<Vec<_>>(),
                vec![
                    ParamAutomationSegment {
                        start_sample: 0,
                        end_sample: 2,
                        normalized: 0.25,
                    },
                    ParamAutomationSegment {
                        start_sample: 2,
                        end_sample: 5,
                        normalized: 0.5,
                    },
                    ParamAutomationSegment {
                        start_sample: 5,
                        end_sample: 8,
                        normalized: 0.75,
                    },
                ]
            );

            {
                let (audio, context_events) = context.audio_mut_and_events();
                assert_eq!(context_events.len(), events.len());
                audio.set_output_sample(0, 0, -0.25);
                audio.set_output_sample(0, 1, 0.75);
                audio.set_output_sample(1, 0, 0.5);
                audio.set_output_sample(1, 1, -0.125);
            }
            assert!(context.emit_output_meter(99, 11));
        }

        assert_eq!(sink.frames.len(), 1);
        let frame = sink.frames[0];
        assert_eq!(frame.id_hash, 99);
        assert_eq!(frame.sample_offset, 11);
        assert_eq!(frame.channel_count(), 2);
        assert_eq!(frame.peaks[0], 0.75);
        assert_eq!(frame.peaks[1], 0.5);
        assert!((frame.rms[0] - 0.279_508_5).abs() < 0.000_001);
        assert!((frame.rms[1] - 0.182_217_5).abs() < 0.000_001);
    }

    #[test]
    fn coalesces_param_automation_segments_at_same_offset() {
        let gain = ParamHandle::from_index(0);
        let events = [
            Event::Param {
                sample_offset: 0,
                handle: gain,
                id_hash: 7,
                normalized: 0.4,
            },
            Event::Param {
                sample_offset: 0,
                handle: gain,
                id_hash: 7,
                normalized: 0.6,
            },
            Event::Param {
                sample_offset: 3,
                handle: gain,
                id_hash: 7,
                normalized: 0.8,
            },
        ];

        let segments = ParamAutomationSegments::new(&events, gain, 0.2, 4).collect::<Vec<_>>();

        assert_eq!(
            segments,
            vec![
                ParamAutomationSegment {
                    start_sample: 0,
                    end_sample: 3,
                    normalized: 0.6,
                },
                ParamAutomationSegment {
                    start_sample: 3,
                    end_sample: 4,
                    normalized: 0.8,
                },
            ]
        );
    }

    #[test]
    fn automation_segments_ignore_other_handles_and_clamp_to_block() {
        let gain = ParamHandle::from_index(0);
        let other = ParamHandle::from_index(1);
        let events = [
            Event::Param {
                sample_offset: 1,
                handle: other,
                id_hash: 8,
                normalized: 0.9,
            },
            Event::Param {
                sample_offset: 2,
                handle: gain,
                id_hash: 7,
                normalized: 0.5,
            },
            Event::Param {
                sample_offset: 99,
                handle: gain,
                id_hash: 7,
                normalized: 1.0,
            },
        ];

        let segments = ParamAutomationSegments::new(&events, gain, 0.25, 4).collect::<Vec<_>>();

        assert_eq!(
            segments,
            vec![
                ParamAutomationSegment {
                    start_sample: 0,
                    end_sample: 2,
                    normalized: 0.25,
                },
                ParamAutomationSegment {
                    start_sample: 2,
                    end_sample: 4,
                    normalized: 0.5,
                },
            ]
        );
    }

    #[test]
    fn automation_segments_cover_block_for_deterministic_point_patterns() {
        let gain = ParamHandle::from_index(0);
        let other = ParamHandle::from_index(1);
        let initial = 0.125;
        let patterns: &[&[(u32, f64)]] = &[
            &[],
            &[(0, 0.25)],
            &[(1, 0.5)],
            &[(0, 0.25), (0, 0.75), (1, 0.5)],
            &[(2, 0.25), (2, 0.875), (5, 0.375), (99, 1.0)],
        ];

        for block_frames in [0, 1, 2, 4, 8, 16, 32] {
            for points in patterns {
                let mut events = Vec::new();
                for (offset, normalized) in *points {
                    events.push(Event::Param {
                        sample_offset: *offset,
                        handle: other,
                        id_hash: 8,
                        normalized: 0.99,
                    });
                    events.push(Event::Param {
                        sample_offset: *offset,
                        handle: gain,
                        id_hash: 7,
                        normalized: *normalized,
                    });
                }

                let segments = ParamAutomationSegments::new(&events, gain, initial, block_frames)
                    .collect::<Vec<_>>();

                if block_frames == 0 {
                    assert!(segments.is_empty());
                    continue;
                }

                let mut expected_by_sample = Vec::new();
                for sample in 0..block_frames {
                    let mut expected = initial;
                    for (offset, normalized) in *points {
                        if *offset <= sample {
                            expected = *normalized;
                        }
                    }
                    expected_by_sample.push(expected);
                }

                let mut cursor = 0;
                for segment in &segments {
                    assert!(!segment.is_empty(), "{block_frames:?} {points:?}");
                    assert_eq!(segment.start_sample, cursor, "{block_frames:?} {points:?}");
                    assert!(
                        segment.end_sample <= block_frames,
                        "{block_frames:?} {points:?}"
                    );
                    for sample in segment.start_sample..segment.end_sample {
                        assert_eq!(
                            segment.normalized, expected_by_sample[sample as usize],
                            "{block_frames:?} {points:?} sample={sample}"
                        );
                    }
                    cursor = segment.end_sample;
                }
                assert_eq!(cursor, block_frames, "{block_frames:?} {points:?}");
            }
        }
    }

    #[test]
    fn automation_segments_cover_block_for_seeded_fuzz_patterns() {
        let gain = ParamHandle::from_index(0);
        let other = ParamHandle::from_index(1);
        let initial = 0.375;
        let mut seed = 0x5645_5354_4946_555a_u64;

        for block_frames in [1_u32, 2, 3, 7, 16, 31, 64, 128] {
            for case_index in 0..32 {
                let mut events = Vec::new();
                let point_count = 1 + (next_fuzz_u32(&mut seed) % 48);
                for point_index in 0..point_count {
                    let sample_offset = next_fuzz_u32(&mut seed) % (block_frames + 8);
                    let normalized = f64::from(next_fuzz_u32(&mut seed) % 1001) / 1000.0;
                    let handle = if next_fuzz_u32(&mut seed).is_multiple_of(3) {
                        other
                    } else {
                        gain
                    };
                    events.push(Event::Param {
                        sample_offset,
                        handle,
                        id_hash: if handle == gain { 7 } else { 8 },
                        normalized,
                    });

                    if point_index % 7 == 0 {
                        events.push(Event::NoteOn {
                            sample_offset,
                            channel: 0,
                            key: 60,
                            velocity: 0.5,
                            note_id: point_index as i32,
                        });
                    }
                }
                events.sort_by_key(Event::sample_offset);

                let segments = ParamAutomationSegments::new(&events, gain, initial, block_frames)
                    .collect::<Vec<_>>();

                let mut expected_by_sample = vec![initial; block_frames as usize];
                for event in &events {
                    let Event::Param {
                        sample_offset,
                        handle,
                        normalized,
                        ..
                    } = *event
                    else {
                        continue;
                    };
                    if handle != gain || sample_offset >= block_frames {
                        continue;
                    }
                    for sample in sample_offset..block_frames {
                        expected_by_sample[sample as usize] = normalized;
                    }
                }

                let mut cursor = 0;
                for segment in &segments {
                    assert!(
                        !segment.is_empty(),
                        "block={block_frames} case={case_index} events={events:?}"
                    );
                    assert_eq!(
                        segment.start_sample, cursor,
                        "block={block_frames} case={case_index} events={events:?}"
                    );
                    assert!(
                        segment.end_sample <= block_frames,
                        "block={block_frames} case={case_index} events={events:?}"
                    );
                    for sample in segment.start_sample..segment.end_sample {
                        assert_eq!(
                            segment.normalized, expected_by_sample[sample as usize],
                            "block={block_frames} case={case_index} sample={sample} events={events:?}"
                        );
                    }
                    cursor = segment.end_sample;
                }
                assert_eq!(
                    cursor, block_frames,
                    "block={block_frames} case={case_index} events={events:?}"
                );
            }
        }
    }

    fn next_fuzz_u32(seed: &mut u64) -> u32 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (*seed >> 32) as u32
    }

    #[derive(Default)]
    struct CollectMeterSink {
        frames: Vec<MeterFrame>,
    }

    impl MeterSink for CollectMeterSink {
        fn push_meter(&mut self, frame: MeterFrame) -> bool {
            self.frames.push(frame);
            true
        }
    }

    #[test]
    fn computes_and_emits_output_meter_frames() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
        };
        let mut left = [1.0, -0.5, 0.25, -0.25];
        let mut right = [0.0, 0.25, -0.75, 0.5];
        let inputs: [&[f32]; 0] = [];
        let mut outputs: [&mut [f32]; 2] = [&mut left, &mut right];
        let audio = AudioBuffers::new(&inputs, &mut outputs);
        let mut sink = CollectMeterSink::default();

        {
            let mut context = ProcessContext::new(audio, &params, &[], Transport::default())
                .with_meter_sink(&mut sink);
            assert!(context.emit_output_meter(42, 7));
        }

        assert_eq!(sink.frames.len(), 1);
        let frame = sink.frames[0];
        assert_eq!(frame.id_hash, 42);
        assert_eq!(frame.sample_offset, 7);
        assert_eq!(frame.channel_count(), 2);
        assert_eq!(frame.peaks[0], 1.0);
        assert_eq!(frame.peaks[1], 0.75);
        assert!((frame.rms[0] - 0.586_301_9).abs() < 0.000_001);
        assert!((frame.rms[1] - 0.467_707_2).abs() < 0.000_001);
    }

    #[test]
    fn emitting_meter_without_sink_drops_frame() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
        };
        let mut output = [0.0; 3];
        let inputs: [&[f32]; 0] = [];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let audio = AudioBuffers::new(&inputs, &mut outputs);
        let mut context = ProcessContext::new(audio, &params, &[], Transport::default());
        assert!(!context.emit_output_meter(1, 0));
    }

    #[test]
    fn process_context_exposes_optional_sidechain_buffers() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
        };
        let input = [0.25_f32, 0.5];
        let side_left = [1.0_f32, 0.5];
        let side_right = [0.25_f32, 0.125];
        let mut output = [0.0_f32; 2];
        let inputs: [&[f32]; 1] = [&input];
        let sidechain_inputs: [&[f32]; 2] = [&side_left, &side_right];
        let mut outputs: [&mut [f32]; 1] = [&mut output];
        let audio = AudioBuffers::new(&inputs, &mut outputs);
        let sidechain = SidechainBuffers::new(&sidechain_inputs);
        let context = ProcessContext::new(audio, &params, &[], Transport::default())
            .with_sidechain(sidechain);

        assert_eq!(context.audio().input_channels(), 1);
        assert_eq!(context.sidechain().input_channels(), 2);
        assert_eq!(context.sidechain().frames(), 2);
        assert_eq!(context.sidechain().input_channel(0), Some(&side_left[..]));
        assert_eq!(context.sidechain().input_channel(1), Some(&side_right[..]));
    }

    #[test]
    fn process_context64_exposes_optional_sidechain_buffers() {
        let params = Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
        };
        let side_left = [1.0_f64, 0.5];
        let side_right = [0.25_f64, 0.125];
        let mut output = [0.0_f64; 2];
        let inputs: [&[f64]; 0] = [];
        let sidechain_inputs: [&[f64]; 2] = [&side_left, &side_right];
        let mut outputs: [&mut [f64]; 1] = [&mut output];
        let audio = AudioBuffers64::new(&inputs, &mut outputs);
        let sidechain = SidechainBuffers64::new(&sidechain_inputs);
        let context = ProcessContext64::new(audio, &params, &[], Transport::default())
            .with_sidechain(sidechain);

        assert_eq!(context.audio().input_channels(), 0);
        assert_eq!(context.sidechain().input_channels(), 2);
        assert_eq!(context.sidechain().frames(), 2);
        assert_eq!(context.sidechain().input_channel(0), Some(&side_left[..]));
        assert_eq!(context.sidechain().input_channel(1), Some(&side_right[..]));
    }

    #[test]
    fn program_list_descriptors_are_static_and_default_empty() {
        static PROGRAMS: &[Program] = &[Program::new("Init"), Program::new("Bright")];
        static LISTS: &[ProgramList] = &[ProgramList::new(7, "Factory", PROGRAMS)];

        assert_eq!(LISTS[0].id, 7);
        assert_eq!(LISTS[0].name, "Factory");
        assert!(!LISTS[0].is_empty());
        assert_eq!(LISTS[0].programs[1].name, "Bright");

        let attribute = ProgramAttribute::new("category", "Lead");
        assert!(attribute.is_valid());
        assert!(!ProgramAttribute::new("", "Lead").is_valid());
        assert!(!ProgramAttribute::new("category\0bad", "Lead").is_valid());
        assert!(!ProgramAttribute::new("category", "Lead\0bad").is_valid());

        let pitch_name = ProgramPitchName::new(60, "C4 Lead");
        assert!(pitch_name.is_valid());
        assert!(!ProgramPitchName::new(-1, "C4 Lead").is_valid());
        assert!(!ProgramPitchName::new(128, "C4 Lead").is_valid());
        assert!(!ProgramPitchName::new(60, "").is_valid());
        assert!(!ProgramPitchName::new(60, "C4\0Lead").is_valid());

        struct EmptyProgramPlugin {
            params: Params,
        }

        struct EmptyProgramKernel;

        impl AudioKernel for EmptyProgramKernel {
            fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
                context.audio_mut().clear_outputs();
                ProcessResult::Silence
            }
        }

        impl Plugin for EmptyProgramPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Empty Program",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"empty-program!!!",
                kind: PluginKind::AudioEffect,
            };

            type Params = Params;
            type Kernel = EmptyProgramKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                EmptyProgramKernel
            }
        }

        let plugin = EmptyProgramPlugin {
            params: Params {
                gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 0.5),
            },
        };
        assert!(plugin.program_lists().is_empty());
        assert_eq!(plugin.apply_program(7, 0), Ok(false));
        assert!(!plugin.program_data_supported(7));
        assert_eq!(plugin.save_program_data(7, 0), Ok(None));
        assert_eq!(
            plugin.load_program_data(7, 0, serde_json::json!({ "gain": 0.5 })),
            Ok(false)
        );
        assert!(plugin.program_attributes(7, 0).is_empty());
        assert!(plugin.program_pitch_names(7, 0).is_empty());
    }

    #[test]
    fn audio_output_bus_descriptor_validates_realtime_safe_shape() {
        assert_eq!(MAX_AUDIO_OUTPUT_BUSES, 4);
        assert_eq!(MAX_AUDIO_OUTPUT_CHANNELS, 8);
        assert!(AudioOutputBus::mono("Aux").is_valid());
        assert!(AudioOutputBus::stereo("Main").is_valid());
        assert!(!AudioOutputBus::new("", 2).is_valid());
        assert!(!AudioOutputBus::new("Bad\nName", 2).is_valid());
        assert!(!AudioOutputBus::new("Wide", 3).is_valid());
        assert_eq!(
            DEFAULT_AUDIO_OUTPUT_BUSES,
            [AudioOutputBus::stereo("Output")]
        );
    }

    #[test]
    fn note_expression_value_type_descriptor_validates_static_metadata() {
        let brightness =
            NoteExpressionValueType::new(note_expression::BRIGHTNESS, "Brightness", "Bright")
                .with_range(0.0, 1.0, 0.5)
                .with_flags(NoteExpressionValueFlags::ABSOLUTE);
        assert!(brightness.is_valid());
        assert_eq!(brightness.units, "");
        assert_eq!(brightness.step_count, 0);

        let invalid =
            NoteExpressionValueType::new(note_expression::INVALID, "Bad\nExpression", "Bad")
                .with_units("bad")
                .with_range(0.0, 1.0, f64::NAN)
                .with_step_count(-1);
        assert!(!invalid.is_valid());

        let mapping = NoteExpressionPhysicalUiMapping::new(
            physical_ui::PRESSURE,
            note_expression::BRIGHTNESS,
        );
        assert!(mapping.is_valid());

        let invalid_mapping =
            NoteExpressionPhysicalUiMapping::new(physical_ui::INVALID, note_expression::BRIGHTNESS);
        assert!(!invalid_mapping.is_valid());
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    struct CustomState {
        mode: String,
    }

    #[derive(Default)]
    struct StatefulPlugin {
        mode: std::sync::Mutex<String>,
    }

    impl PluginState for StatefulPlugin {
        type State = CustomState;

        fn save_state(&self) -> Self::State {
            CustomState {
                mode: self.mode.lock().unwrap().clone(),
            }
        }

        fn load_state(&self, state: Self::State) -> Result<(), StateError> {
            *self.mode.lock().unwrap() = state.mode;
            Ok(())
        }
    }

    #[test]
    fn plugin_state_helpers_roundtrip_json() {
        let plugin = StatefulPlugin::default();
        *plugin.mode.lock().unwrap() = "wide".to_string();

        let value = save_plugin_state(&plugin).unwrap();
        assert_eq!(value["mode"], "wide");

        let restored = StatefulPlugin::default();
        load_plugin_state(&restored, value).unwrap();
        assert_eq!(restored.mode.lock().unwrap().as_str(), "wide");
    }

    #[test]
    fn host_profiles_cover_release_matrix() {
        let profiles = host_profiles();
        assert_eq!(profiles.len(), 5);
        for expected in [
            "REAPER",
            "Cubase/Nuendo",
            "Bitwig Studio",
            "Ableton Live",
            "Studio One",
        ] {
            let profile = profiles
                .iter()
                .find(|profile| profile.name == expected)
                .unwrap_or_else(|| panic!("missing host profile: {expected}"));
            assert_eq!(profile.required_smoke_checks, RELEASE_SMOKE_CHECKS);
            assert!(!profile.quirks.is_empty());
        }
    }

    #[test]
    fn host_profile_lookup_accepts_aliases() {
        assert_eq!(
            find_host_profile("Cubase").map(|profile| profile.id),
            Some("cubase-nuendo")
        );
        assert_eq!(
            find_host_profile("bitwig studio").map(|profile| profile.id),
            Some("bitwig")
        );
        assert!(find_host_profile("unknown").is_none());
    }
}
