use serde::{Deserialize, Serialize};
use vesty_params::{ParamCollection, ParamHandle};

use crate::{MAX_METER_CHANNELS, MAX_NOTE_EXPRESSION_TEXT_UNITS, MAX_SYSEX_BYTES};

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
