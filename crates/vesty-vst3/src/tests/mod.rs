use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use vesty_core::{
    AudioKernel, AudioOutputBus, HostChangeFlags, KernelInit, PluginKind, ProcessContext,
    ProcessResult, StateError, UiDescriptor,
};
#[cfg(feature = "wry-ui")]
use vesty_ipc::{BridgeErrorCode, BridgeKind};
use vesty_params::{
    BoolParam, ChoiceParam, FloatParam, ParamCollection, ParamError, ParamHandle, ParamSpec,
};

struct RtCountingAllocator;

thread_local! {
    static RT_ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[global_allocator]
static TEST_ALLOCATOR: RtCountingAllocator = RtCountingAllocator;

// SAFETY: This test allocator delegates every operation to `System` with the original
// allocation arguments while incrementing a thread-local counter for realtime guard tests.
unsafe impl GlobalAlloc for RtCountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if vesty_rt::NoAllocGuard::is_active() {
            RT_ALLOCATION_COUNT.with(|count| count.set(count.get() + 1));
        }
        // SAFETY: Delegates to the system allocator with the layout provided by the caller.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Delegates to System with the original pointer/layout from Rust's allocator contract.
        unsafe {
            System.dealloc(ptr, layout);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if vesty_rt::NoAllocGuard::is_active() {
            RT_ALLOCATION_COUNT.with(|count| count.set(count.get() + 1));
        }
        // SAFETY: Delegates to System with the pointer/layout/size from Rust's allocator contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[cfg(feature = "vst3-bindings")]
fn reset_rt_allocation_count() {
    RT_ALLOCATION_COUNT.with(|count| count.set(0));
}

#[cfg(feature = "vst3-bindings")]
fn rt_allocation_count() -> usize {
    RT_ALLOCATION_COUNT.with(Cell::get)
}

struct TestParams {
    gain: FloatParam,
    mode: ChoiceParam,
}

impl Default for TestParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", 0.0, 2.0, 1.0),
            mode: ChoiceParam::new("mode", "Mode", ["Clean", "Drive", "Fuzz"], 0),
        }
    }
}

impl ParamCollection for TestParams {
    fn specs(&self) -> Vec<ParamSpec> {
        vec![self.gain.spec(), self.mode.spec()]
    }

    fn get_normalized(&self, id: &str) -> Option<f64> {
        if id == self.gain.id() {
            Some(self.gain.normalized())
        } else if id == self.mode.id() {
            Some(self.mode.normalized())
        } else {
            None
        }
    }

    fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
        if id == self.gain.id() {
            self.gain.set_normalized(normalized);
            Ok(())
        } else if id == self.mode.id() {
            self.mode.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(id.to_string()))
        }
    }

    fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
        match handle.index() {
            0 => Some(self.gain.normalized()),
            1 => Some(self.mode.normalized()),
            _ => None,
        }
    }

    fn set_normalized_by_handle(
        &self,
        handle: ParamHandle,
        normalized: f64,
    ) -> Result<(), ParamError> {
        match handle.index() {
            0 => self.gain.set_normalized(normalized),
            1 => self.mode.set_normalized(normalized),
            _ => return Err(ParamError::Unknown(format!("handle:{}", handle.index()))),
        }
        Ok(())
    }
}

struct Kernel;

impl AudioKernel for Kernel {
    fn process(&mut self, _context: &mut ProcessContext<'_>) -> ProcessResult {
        ProcessResult::Continue
    }
}

#[derive(Default)]
struct TestPlugin {
    params: TestParams,
    custom_state: std::sync::Mutex<Option<String>>,
}

impl Plugin for TestPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Test",
        vendor: "Vesty",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: *b"0123456789abcdef",
        kind: PluginKind::AudioEffect,
    };

    type Params = TestParams;
    type Kernel = Kernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        Kernel
    }

    fn ui(&self) -> Option<UiDescriptor> {
        Some(UiDescriptor {
            assets_dir: "ui".to_string(),
            dev_url: None,
            width: 777,
            height: 333,
            min_width: 320,
            min_height: 200,
            resizable: true,
        })
    }

    fn latency_samples(&self) -> u32 {
        128
    }

    fn tail_samples(&self) -> u32 {
        4096
    }

    fn host_changes_for_param(
        &self,
        id: &str,
        old_normalized: f64,
        new_normalized: f64,
    ) -> HostChangeFlags {
        if id == "mode" && (old_normalized - new_normalized).abs() > f64::EPSILON {
            HostChangeFlags::LATENCY
        } else {
            HostChangeFlags::NONE
        }
    }

    fn save_custom_state(&self) -> Result<Option<serde_json::Value>, StateError> {
        let state = self
            .custom_state
            .lock()
            .map_err(|_| StateError::custom("custom state lock poisoned"))?
            .clone();
        Ok(state.map(|label| serde_json::json!({ "label": label })))
    }

    fn load_custom_state(&self, state: Option<serde_json::Value>) -> Result<(), StateError> {
        let Some(state) = state else {
            return Ok(());
        };
        let label = state
            .get("label")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| StateError::Deserialize("missing custom label".to_string()))?
            .to_string();
        *self
            .custom_state
            .lock()
            .map_err(|_| StateError::custom("custom state lock poisoned"))? = Some(label);
        Ok(())
    }
}

#[allow(dead_code)]
struct FlagParams {
    bypass: BoolParam,
    meter: FloatParam,
    program: ChoiceParam,
}

impl Default for FlagParams {
    fn default() -> Self {
        Self {
            bypass: BoolParam::bypass("bypass", "Bypass", false),
            meter: FloatParam::new("meter", "Meter", 0.0, 1.0, 0.0),
            program: ChoiceParam::new("program", "Program", ["Init", "Lead"], 0),
        }
    }
}

impl ParamCollection for FlagParams {
    fn specs(&self) -> Vec<ParamSpec> {
        vec![
            self.bypass.spec(),
            self.meter.spec().as_read_only(),
            self.program.spec().as_program_change(),
        ]
    }

    fn get_normalized(&self, id: &str) -> Option<f64> {
        if id == self.bypass.id() {
            Some(self.bypass.normalized())
        } else if id == self.meter.id() {
            Some(self.meter.normalized())
        } else if id == self.program.id() {
            Some(self.program.normalized())
        } else {
            None
        }
    }

    fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
        if id == self.bypass.id() {
            self.bypass.set_normalized(normalized);
            Ok(())
        } else if id == self.meter.id() {
            Err(ParamError::ReadOnly(id.to_string()))
        } else if id == self.program.id() {
            self.program.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(id.to_string()))
        }
    }

    fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
        match handle.index() {
            0 => Some(self.bypass.normalized()),
            1 => Some(self.meter.normalized()),
            2 => Some(self.program.normalized()),
            _ => None,
        }
    }

    fn set_normalized_by_handle(
        &self,
        handle: ParamHandle,
        normalized: f64,
    ) -> Result<(), ParamError> {
        match handle.index() {
            0 => self.bypass.set_normalized(normalized),
            1 => return Err(ParamError::ReadOnly("meter".to_string())),
            2 => self.program.set_normalized(normalized),
            _ => return Err(ParamError::Unknown(format!("handle:{}", handle.index()))),
        }
        Ok(())
    }
}

#[derive(Default)]
#[allow(dead_code)]
struct FlagPlugin {
    params: FlagParams,
}

impl Plugin for FlagPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Flag Test",
        vendor: "Vesty",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: *b"flag-test-plugin",
        kind: PluginKind::AudioEffect,
    };

    type Params = FlagParams;
    type Kernel = Kernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        Kernel
    }
}

struct MidiMappedParams {
    gain: FloatParam,
    cutoff: FloatParam,
    pitch: FloatParam,
    meter: FloatParam,
    program: ChoiceParam,
}

impl Default for MidiMappedParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 0.5),
            cutoff: FloatParam::new("cutoff", "Cutoff", 20.0, 20_000.0, 1_000.0),
            pitch: FloatParam::new("pitch", "Pitch", -1.0, 1.0, 0.0),
            meter: FloatParam::new("meter", "Meter", 0.0, 1.0, 0.0),
            program: ChoiceParam::new("program", "Program", ["Init", "Bright Lead", "Soft Pad"], 0),
        }
    }
}

impl ParamCollection for MidiMappedParams {
    fn specs(&self) -> Vec<ParamSpec> {
        vec![
            self.gain.spec().with_midi_cc(7),
            self.cutoff.spec().with_channel_midi_cc(74, 2),
            self.pitch
                .spec()
                .with_channel_midi_cc(vesty_params::midi::PITCH_BEND, 1),
            self.meter.spec().with_midi_cc(10).as_read_only(),
            self.program.spec().as_program_change(),
        ]
    }

    fn get_normalized(&self, id: &str) -> Option<f64> {
        if id == self.gain.id() {
            Some(self.gain.normalized())
        } else if id == self.cutoff.id() {
            Some(self.cutoff.normalized())
        } else if id == self.pitch.id() {
            Some(self.pitch.normalized())
        } else if id == self.meter.id() {
            Some(self.meter.normalized())
        } else if id == self.program.id() {
            Some(self.program.normalized())
        } else {
            None
        }
    }

    fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
        if id == self.gain.id() {
            self.gain.set_normalized(normalized);
            Ok(())
        } else if id == self.cutoff.id() {
            self.cutoff.set_normalized(normalized);
            Ok(())
        } else if id == self.pitch.id() {
            self.pitch.set_normalized(normalized);
            Ok(())
        } else if id == self.meter.id() {
            Err(ParamError::ReadOnly(id.to_string()))
        } else if id == self.program.id() {
            self.program.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(id.to_string()))
        }
    }

    fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
        match handle.index() {
            0 => Some(self.gain.normalized()),
            1 => Some(self.cutoff.normalized()),
            2 => Some(self.pitch.normalized()),
            3 => Some(self.meter.normalized()),
            4 => Some(self.program.normalized()),
            _ => None,
        }
    }

    fn set_normalized_by_handle(
        &self,
        handle: ParamHandle,
        normalized: f64,
    ) -> Result<(), ParamError> {
        match handle.index() {
            0 => self.gain.set_normalized(normalized),
            1 => self.cutoff.set_normalized(normalized),
            2 => self.pitch.set_normalized(normalized),
            3 => return Err(ParamError::ReadOnly("meter".to_string())),
            4 => self.program.set_normalized(normalized),
            _ => return Err(ParamError::Unknown(format!("handle:{}", handle.index()))),
        }
        Ok(())
    }
}

#[derive(Default)]
struct MidiMappedPlugin {
    params: MidiMappedParams,
}

static MIDI_MAPPED_PROGRAMS: &[vesty_core::Program] = &[
    vesty_core::Program::new("Init"),
    vesty_core::Program::new("Bright Lead"),
    vesty_core::Program::new("Soft Pad"),
];
static MIDI_MAPPED_PROGRAM_LISTS: &[vesty_core::ProgramList] = &[vesty_core::ProgramList::new(
    77,
    "Factory Programs",
    MIDI_MAPPED_PROGRAMS,
)];
static MIDI_MAPPED_PROGRAM_ATTRIBUTES: &[vesty_core::ProgramAttribute] = &[
    vesty_core::ProgramAttribute::new("category", "Lead"),
    vesty_core::ProgramAttribute::new("mood", "Bright"),
    vesty_core::ProgramAttribute::new("", "Hidden"),
    vesty_core::ProgramAttribute::new("invalid\0id", "Hidden"),
    vesty_core::ProgramAttribute::new("invalid_value", "Hidden\0Value"),
];
static MIDI_MAPPED_PROGRAM_PITCH_NAMES: &[vesty_core::ProgramPitchName] = &[
    vesty_core::ProgramPitchName::new(60, "C4 Lead"),
    vesty_core::ProgramPitchName::new(64, "E4 Lead"),
    vesty_core::ProgramPitchName::new(-1, "Hidden"),
    vesty_core::ProgramPitchName::new(128, "Hidden"),
    vesty_core::ProgramPitchName::new(61, ""),
    vesty_core::ProgramPitchName::new(62, "Hidden\0Name"),
];
static MIDI_MAPPED_NOTE_EXPRESSIONS: &[vesty_core::NoteExpressionValueType] = &[
    vesty_core::NoteExpressionValueType::new(
        vesty_core::note_expression::BRIGHTNESS,
        "Brightness",
        "Bright",
    )
    .with_range(0.0, 1.0, 0.5)
    .with_flags(vesty_core::NoteExpressionValueFlags::ABSOLUTE),
    vesty_core::NoteExpressionValueType::new(vesty_core::note_expression::TUNING, "Tuning", "Tune")
        .with_units("st")
        .with_range(0.0, 1.0, 0.5)
        .with_flags(vesty_core::NoteExpressionValueFlags::ABSOLUTE_BIPOLAR),
];
static MIDI_MAPPED_PHYSICAL_UI_MAPPINGS: &[vesty_core::NoteExpressionPhysicalUiMapping] = &[
    vesty_core::NoteExpressionPhysicalUiMapping::new(
        vesty_core::physical_ui::PRESSURE,
        vesty_core::note_expression::BRIGHTNESS,
    ),
    vesty_core::NoteExpressionPhysicalUiMapping::new(
        vesty_core::physical_ui::Y_MOVEMENT,
        vesty_core::note_expression::TUNING,
    ),
];

impl Plugin for MidiMappedPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Midi Mapped",
        vendor: "Vesty",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: *b"midi-map-plugin!",
        kind: PluginKind::Instrument,
    };

    type Params = MidiMappedParams;
    type Kernel = Kernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        Kernel
    }

    fn program_lists(&self) -> &'static [vesty_core::ProgramList] {
        MIDI_MAPPED_PROGRAM_LISTS
    }

    fn apply_program(&self, list_id: u32, program_index: usize) -> Result<bool, StateError> {
        if list_id != 77 {
            return Ok(false);
        }

        let (gain, cutoff, pitch) = match program_index {
            0 => (0.5, 0.25, 0.5),
            1 => (0.8, 0.9, 0.65),
            2 => (0.3, 0.35, 0.4),
            _ => return Ok(false),
        };
        self.params
            .set_normalized("gain", gain)
            .map_err(|error| StateError::custom(error.to_string()))?;
        self.params
            .set_normalized("cutoff", cutoff)
            .map_err(|error| StateError::custom(error.to_string()))?;
        self.params
            .set_normalized("pitch", pitch)
            .map_err(|error| StateError::custom(error.to_string()))?;
        Ok(true)
    }

    fn program_data_supported(&self, list_id: u32) -> bool {
        list_id == 77
    }

    fn save_program_data(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> Result<Option<serde_json::Value>, StateError> {
        if list_id != 77 || program_index >= MIDI_MAPPED_PROGRAMS.len() {
            return Ok(None);
        }

        Ok(Some(serde_json::json!({
            "gain": self.params.gain.normalized(),
            "cutoff": self.params.cutoff.normalized(),
            "pitch": self.params.pitch.normalized(),
        })))
    }

    fn load_program_data(
        &self,
        list_id: u32,
        program_index: usize,
        data: serde_json::Value,
    ) -> Result<bool, StateError> {
        if list_id != 77 || program_index >= MIDI_MAPPED_PROGRAMS.len() {
            return Ok(false);
        }

        let normalized = |id: &str| {
            data.get(id)
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| StateError::Deserialize(format!("missing {id} program data")))
        };
        self.params
            .set_normalized("gain", normalized("gain")?)
            .map_err(|error| StateError::custom(error.to_string()))?;
        self.params
            .set_normalized("cutoff", normalized("cutoff")?)
            .map_err(|error| StateError::custom(error.to_string()))?;
        self.params
            .set_normalized("pitch", normalized("pitch")?)
            .map_err(|error| StateError::custom(error.to_string()))?;
        Ok(true)
    }

    fn program_attributes(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [vesty_core::ProgramAttribute] {
        if list_id == 77 && program_index == 1 {
            MIDI_MAPPED_PROGRAM_ATTRIBUTES
        } else {
            &[]
        }
    }

    fn program_pitch_names(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [vesty_core::ProgramPitchName] {
        if list_id == 77 && program_index == 1 {
            MIDI_MAPPED_PROGRAM_PITCH_NAMES
        } else {
            &[]
        }
    }

    fn note_expression_value_types(&self) -> &'static [vesty_core::NoteExpressionValueType] {
        MIDI_MAPPED_NOTE_EXPRESSIONS
    }

    fn note_expression_physical_ui_mappings(
        &self,
    ) -> &'static [vesty_core::NoteExpressionPhysicalUiMapping] {
        MIDI_MAPPED_PHYSICAL_UI_MAPPINGS
    }
}

#[allow(dead_code)]
struct InvalidParamSchemaParams {
    first: FloatParam,
    second: FloatParam,
}

impl Default for InvalidParamSchemaParams {
    fn default() -> Self {
        Self {
            first: FloatParam::new("duplicate", "First", 0.0, 1.0, 0.0),
            second: FloatParam::new("duplicate", "Second", 0.0, 1.0, 0.0),
        }
    }
}

impl ParamCollection for InvalidParamSchemaParams {
    fn specs(&self) -> Vec<ParamSpec> {
        vec![self.first.spec(), self.second.spec()]
    }

    fn get_normalized(&self, id: &str) -> Option<f64> {
        if id == self.first.id() {
            Some(self.first.normalized())
        } else {
            None
        }
    }

    fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
        if id == self.first.id() {
            self.first.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(id.to_string()))
        }
    }

    fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
        match handle.index() {
            0 => Some(self.first.normalized()),
            1 => Some(self.second.normalized()),
            _ => None,
        }
    }

    fn set_normalized_by_handle(
        &self,
        handle: ParamHandle,
        normalized: f64,
    ) -> Result<(), ParamError> {
        match handle.index() {
            0 => self.first.set_normalized(normalized),
            1 => self.second.set_normalized(normalized),
            _ => return Err(ParamError::Unknown(format!("handle:{}", handle.index()))),
        }
        Ok(())
    }
}

#[derive(Default)]
#[allow(dead_code)]
struct InvalidParamSchemaPlugin {
    params: InvalidParamSchemaParams,
}

impl Plugin for InvalidParamSchemaPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Invalid Params",
        vendor: "Vesty",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: *b"VESTYINVALIDPARA",
        kind: PluginKind::AudioEffect,
    };

    type Params = InvalidParamSchemaParams;
    type Kernel = Kernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        Kernel
    }
}

#[test]
fn metadata_uses_distinct_controller_class() {
    let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
    assert_ne!(metadata.processor_class_id, metadata.controller_class_id);
}

#[test]
fn binding_baseline_reports_reserved_sys_layer() {
    let baseline = binding_baseline();
    assert_eq!(baseline.steinberg_sdk, sys::STEINBERG_VST3_SDK_BASELINE);
    assert_eq!(
        baseline.upstream_vst3_crate,
        sys::UPSTREAM_VST3_CRATE_BASELINE
    );
    assert_ne!(baseline.backend, sys::BindingBackend::MetadataOnly);
}

#[test]
fn panic_guard_faults() {
    let fault = FaultState::default();
    assert_eq!(
        fault.report(),
        FaultReport {
            faulted: false,
            fault_count: 0
        }
    );
    let value = panic_guard(&fault, 7, || panic!("boom"));
    assert_eq!(value, 7);
    assert!(fault.is_faulted());
    assert_eq!(
        fault.report(),
        FaultReport {
            faulted: true,
            fault_count: 1
        }
    );

    let value = panic_guard(&fault, 9, || 11);
    assert_eq!(value, 9);
    assert_eq!(fault.fault_count(), 1);
}

#[cfg(feature = "vst3-bindings")]
#[test]
fn state_roundtrips_params() {
    let source = TestPlugin::default();
    source.params().set_normalized("gain", 0.25).unwrap();
    *source.custom_state.lock().unwrap() = Some("saved".to_string());
    let state = super::bindings_impl::capture_state(&source).unwrap();

    let restored = TestPlugin::default();
    super::bindings_impl::apply_state(&restored, state).unwrap();
    assert_eq!(restored.params().get_normalized("gain"), Some(0.25));
    assert_eq!(
        restored.custom_state.lock().unwrap().as_deref(),
        Some("saved")
    );
}

#[cfg(feature = "vst3-bindings")]
mod bindings;
