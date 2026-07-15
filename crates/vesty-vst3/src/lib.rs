#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use vesty_core::{Plugin, PluginInfo};
pub use vesty_vst3_sys as sys;

#[cfg(feature = "vst3-bindings")]
mod bindings_impl;

#[cfg(feature = "vst3-bindings")]
pub use bindings_impl::{create_plugin_factory, raw};

#[derive(Debug, Error)]
pub enum Vst3AdapterError {
    #[error("plugin instance is faulted")]
    Faulted,
    #[error("panic crossed VST3 callback boundary")]
    Panic,
}

#[derive(Debug, Default)]
pub struct FaultState {
    faulted: AtomicBool,
    fault_count: AtomicU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaultReport {
    pub faulted: bool,
    pub fault_count: u64,
}

impl FaultState {
    pub fn is_faulted(&self) -> bool {
        self.faulted.load(Ordering::Acquire)
    }

    pub fn fault_count(&self) -> u64 {
        self.fault_count.load(Ordering::Acquire)
    }

    pub fn report(&self) -> FaultReport {
        FaultReport {
            faulted: self.is_faulted(),
            fault_count: self.fault_count(),
        }
    }

    pub fn mark_faulted(&self) {
        if self
            .faulted
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.fault_count.fetch_add(1, Ordering::AcqRel);
        }
    }
}

pub fn panic_guard<T>(fault: &FaultState, fallback: T, f: impl FnOnce() -> T) -> T {
    if fault.is_faulted() {
        return fallback;
    }

    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            fault.mark_faulted();
            fallback
        }
    }
}

pub fn abi_guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

#[derive(Clone, Copy, Debug)]
pub struct Vst3BundleMetadata {
    pub info: PluginInfo,
    pub processor_class_id: [u8; 16],
    pub controller_class_id: [u8; 16],
}

impl Vst3BundleMetadata {
    pub fn for_plugin<P: Plugin>() -> Self {
        let info = P::INFO;
        let mut controller_class_id = info.class_id;
        controller_class_id[15] = controller_class_id[15].wrapping_add(1);
        Self {
            info,
            processor_class_id: info.class_id,
            controller_class_id,
        }
    }
}

#[cfg(feature = "vst3-bindings")]
pub fn bindings_enabled() -> bool {
    let _ = std::any::TypeId::of::<vst3::Steinberg::tresult>();
    true
}

#[cfg(not(feature = "vst3-bindings"))]
pub fn bindings_enabled() -> bool {
    false
}

pub fn binding_baseline() -> sys::BindingBaseline {
    sys::BINDING_BASELINE
}

#[cfg(target_os = "macos")]
static MACOS_BUNDLE_RESOURCES: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFBundleCopyResourcesDirectoryURL(
        bundle: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
    fn CFURLGetFileSystemRepresentation(
        url: *const std::ffi::c_void,
        resolve_against_base: u8,
        buffer: *mut u8,
        max_buf_len: isize,
    ) -> u8;
    fn CFRelease(value: *const std::ffi::c_void);
}

#[cfg(target_os = "macos")]
pub fn set_macos_bundle_ref(bundle_ref: *mut std::ffi::c_void) {
    if bundle_ref.is_null() || MACOS_BUNDLE_RESOURCES.get().is_some() {
        return;
    }

    // SAFETY: Steinberg calls BundleEntry with a valid CFBundleRef; null/CF returns are checked and the copied CFURL is released.
    unsafe {
        let url = CFBundleCopyResourcesDirectoryURL(bundle_ref.cast_const());
        if url.is_null() {
            return;
        }

        let mut buffer = [0_u8; 4096];
        let ok =
            CFURLGetFileSystemRepresentation(url, 1, buffer.as_mut_ptr(), buffer.len() as isize);
        CFRelease(url);
        if ok == 0 {
            return;
        }

        let len = buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(buffer.len());
        if len == 0 {
            return;
        }

        let path = PathBuf::from(String::from_utf8_lossy(&buffer[..len]).into_owned());
        let _ = MACOS_BUNDLE_RESOURCES.set(path);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_macos_bundle_ref(_bundle_ref: *mut std::ffi::c_void) {}

pub fn bundle_resources_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        MACOS_BUNDLE_RESOURCES.get().cloned()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
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
                program: ChoiceParam::new(
                    "program",
                    "Program",
                    ["Init", "Bright Lead", "Soft Pad"],
                    0,
                ),
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
        vesty_core::NoteExpressionValueType::new(
            vesty_core::note_expression::TUNING,
            "Tuning",
            "Tune",
        )
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
    mod bindings_tests {
        use super::*;
        use std::cell::{Cell, RefCell};
        use std::ffi::{c_char, c_void};
        use std::mem::MaybeUninit;
        use std::ptr;
        use std::sync::atomic::{
            AtomicBool as TestAtomicBool, AtomicUsize as TestAtomicUsize, Ordering as TestOrdering,
        };
        use std::sync::{LazyLock, Mutex};
        use vesty_core::PrepareContext;
        use vesty_core::{Event as CoreEvent, ProcessMode, Transport};
        use vesty_rt::NoAllocGuard;
        use vst3::{
            Class, ComPtr, ComWrapper,
            Steinberg::{Vst::*, *},
        };

        fn tuid(bytes: [u8; 16]) -> TUID {
            bytes.map(|byte| byte as c_char)
        }

        fn test_param_id(id: &str) -> ParamID {
            vesty_params::stable_vst3_param_id(id)
        }

        fn controller_param_id(controller: &ComPtr<IEditController>, index: int32) -> ParamID {
            let mut info = MaybeUninit::<ParameterInfo>::zeroed();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                assert_eq!(
                    controller.getParameterInfo(index, info.as_mut_ptr()),
                    kResultOk
                );
                info.assume_init().id
            }
        }

        fn string128_to_string(value: &String128) -> String {
            let len = value
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(value.len());
            String::from_utf16(&value[..len]).expect("test UTF-16 string")
        }

        fn wide_cstring(value: &str) -> Vec<TChar> {
            value
                .encode_utf16()
                .map(|unit| unit as TChar)
                .chain(std::iter::once(0))
                .collect()
        }

        #[cfg(target_os = "macos")]
        fn supported_platform_type_for_current_os() -> FIDString {
            kPlatformTypeNSView
        }

        #[cfg(target_os = "windows")]
        fn supported_platform_type_for_current_os() -> FIDString {
            kPlatformTypeHWND
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        fn supported_platform_type_for_current_os() -> FIDString {
            kPlatformTypeX11EmbedWindowID
        }

        #[derive(Default)]
        struct MemoryStream {
            bytes: RefCell<Vec<u8>>,
            cursor: Cell<usize>,
        }

        impl MemoryStream {
            fn with_bytes(bytes: Vec<u8>) -> Self {
                Self {
                    bytes: RefCell::new(bytes),
                    cursor: Cell::new(0),
                }
            }

            fn bytes(&self) -> Vec<u8> {
                self.bytes.borrow().clone()
            }
        }

        fn raw_state_bytes(value: serde_json::Value) -> Vec<u8> {
            let mut bytes = b"VESTY_STATE_V1\n".to_vec();
            bytes.extend_from_slice(serde_json::to_string(&value).unwrap().as_bytes());
            bytes
        }

        fn raw_program_data_bytes(
            list_id: u32,
            program_index: usize,
            data: serde_json::Value,
        ) -> Vec<u8> {
            raw_program_data_bytes_with_version(1, list_id, program_index, data)
        }

        fn raw_program_data_bytes_with_version(
            version: u32,
            list_id: u32,
            program_index: usize,
            data: serde_json::Value,
        ) -> Vec<u8> {
            let mut bytes = b"VESTY_PROGRAM_DATA_V1\n".to_vec();
            bytes.extend_from_slice(
                serde_json::to_string(&serde_json::json!({
                    "version": version,
                    "listId": list_id,
                    "programIndex": program_index,
                    "data": data,
                }))
                .unwrap()
                .as_bytes(),
            );
            bytes
        }

        fn program_data_json(bytes: &[u8]) -> serde_json::Value {
            let json = bytes
                .strip_prefix(b"VESTY_PROGRAM_DATA_V1\n")
                .expect("program data magic");
            serde_json::from_slice(json).expect("program data json")
        }

        impl Class for MemoryStream {
            type Interfaces = (IBStream,);
        }

        impl IBStreamTrait for MemoryStream {
            unsafe fn read(
                &self,
                buffer: *mut c_void,
                num_bytes: int32,
                num_bytes_read: *mut int32,
            ) -> tresult {
                if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
                    return kInvalidArgument;
                }

                let cursor = self.cursor.get();
                let bytes = self.bytes.borrow();
                let read = (num_bytes as usize).min(bytes.len().saturating_sub(cursor));
                if read > 0 {
                    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                    unsafe {
                        ptr::copy_nonoverlapping(bytes[cursor..].as_ptr(), buffer as *mut u8, read);
                    }
                }
                self.cursor.set(cursor + read);
                if !num_bytes_read.is_null() {
                    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                    unsafe {
                        *num_bytes_read = read as int32;
                    }
                }
                kResultOk
            }

            unsafe fn write(
                &self,
                buffer: *mut c_void,
                num_bytes: int32,
                num_bytes_written: *mut int32,
            ) -> tresult {
                if num_bytes < 0 || (buffer.is_null() && num_bytes > 0) {
                    return kInvalidArgument;
                }

                let len = num_bytes as usize;
                let cursor = self.cursor.get();
                let mut bytes = self.bytes.borrow_mut();
                if bytes.len() < cursor + len {
                    bytes.resize(cursor + len, 0);
                }
                if len > 0 {
                    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                    unsafe {
                        ptr::copy_nonoverlapping(
                            buffer as *const u8,
                            bytes[cursor..].as_mut_ptr(),
                            len,
                        );
                    }
                }
                self.cursor.set(cursor + len);
                if !num_bytes_written.is_null() {
                    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                    unsafe {
                        *num_bytes_written = len as int32;
                    }
                }
                kResultOk
            }

            unsafe fn seek(&self, pos: int64, mode: int32, result: *mut int64) -> tresult {
                let len = self.bytes.borrow().len() as int64;
                #[allow(clippy::unnecessary_cast)]
                let seek_set = IBStream_::IStreamSeekMode_::kIBSeekSet as int32;
                #[allow(clippy::unnecessary_cast)]
                let seek_current = IBStream_::IStreamSeekMode_::kIBSeekCur as int32;
                #[allow(clippy::unnecessary_cast)]
                let seek_end = IBStream_::IStreamSeekMode_::kIBSeekEnd as int32;
                let base = match mode {
                    value if value == seek_set => 0,
                    value if value == seek_current => self.cursor.get() as int64,
                    value if value == seek_end => len,
                    _ => return kInvalidArgument,
                };
                let Some(next) = base.checked_add(pos) else {
                    return kInvalidArgument;
                };
                if next < 0 {
                    return kInvalidArgument;
                }
                self.cursor.set(next as usize);
                if !result.is_null() {
                    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                    unsafe {
                        *result = next;
                    }
                }
                kResultOk
            }

            unsafe fn tell(&self, pos: *mut int64) -> tresult {
                if pos.is_null() {
                    return kInvalidArgument;
                }
                // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                unsafe {
                    *pos = self.cursor.get() as int64;
                }
                kResultOk
            }
        }

        struct FakeParamValueQueue {
            id: ParamID,
            points: RefCell<Vec<(int32, ParamValue)>>,
        }

        impl FakeParamValueQueue {
            fn new(id: ParamID, points: Vec<(int32, ParamValue)>) -> Self {
                Self {
                    id,
                    points: RefCell::new(points),
                }
            }
        }

        impl Class for FakeParamValueQueue {
            type Interfaces = (IParamValueQueue,);
        }

        impl IParamValueQueueTrait for FakeParamValueQueue {
            unsafe fn getParameterId(&self) -> ParamID {
                self.id
            }

            unsafe fn getPointCount(&self) -> int32 {
                self.points.borrow().len() as int32
            }

            unsafe fn getPoint(
                &self,
                index: int32,
                sample_offset: *mut int32,
                value: *mut ParamValue,
            ) -> tresult {
                if sample_offset.is_null() || value.is_null() {
                    return kInvalidArgument;
                }
                let Some((sample, param_value)) = self.points.borrow().get(index as usize).copied()
                else {
                    return kInvalidArgument;
                };
                // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                unsafe {
                    *sample_offset = sample;
                    *value = param_value;
                }
                kResultTrue
            }

            unsafe fn addPoint(
                &self,
                sample_offset: int32,
                value: ParamValue,
                index: *mut int32,
            ) -> tresult {
                let mut points = self.points.borrow_mut();
                points.push((sample_offset, value));
                if !index.is_null() {
                    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                    unsafe {
                        *index = (points.len() - 1) as int32;
                    }
                }
                kResultOk
            }
        }

        struct FakeParameterChanges {
            queues: Vec<ComPtr<IParamValueQueue>>,
        }

        impl Class for FakeParameterChanges {
            type Interfaces = (IParameterChanges,);
        }

        impl IParameterChangesTrait for FakeParameterChanges {
            unsafe fn getParameterCount(&self) -> int32 {
                self.queues.len() as int32
            }

            unsafe fn getParameterData(&self, index: int32) -> *mut IParamValueQueue {
                self.queues
                    .get(index as usize)
                    .map_or(ptr::null_mut(), ComPtr::as_ptr)
            }

            unsafe fn addParameterData(
                &self,
                _id: *const ParamID,
                _index: *mut int32,
            ) -> *mut IParamValueQueue {
                ptr::null_mut()
            }
        }

        struct FakeEventList {
            events: Vec<Event>,
            added: RefCell<Vec<Event>>,
        }

        impl FakeEventList {
            fn new(events: Vec<Event>) -> Self {
                Self {
                    events,
                    added: RefCell::new(Vec::new()),
                }
            }
        }

        impl Class for FakeEventList {
            type Interfaces = (IEventList,);
        }

        impl IEventListTrait for FakeEventList {
            unsafe fn getEventCount(&self) -> int32 {
                self.events.len() as int32
            }

            unsafe fn getEvent(&self, index: int32, e: *mut Event) -> tresult {
                if e.is_null() {
                    return kInvalidArgument;
                }
                let Some(event) = self.events.get(index as usize).copied() else {
                    return kInvalidArgument;
                };
                // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                unsafe {
                    *e = event;
                }
                kResultOk
            }

            unsafe fn addEvent(&self, e: *mut Event) -> tresult {
                if e.is_null() {
                    return kInvalidArgument;
                }
                // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
                unsafe {
                    self.added.borrow_mut().push(*e);
                }
                kResultOk
            }
        }

        #[derive(Clone, Debug, PartialEq)]
        enum HandlerCall {
            Begin(ParamID),
            Perform(ParamID, ParamValue),
            End(ParamID),
            Restart(int32),
        }

        struct FakeComponentHandler {
            calls: RefCell<Vec<HandlerCall>>,
            perform_result: Cell<tresult>,
        }

        impl Default for FakeComponentHandler {
            fn default() -> Self {
                Self {
                    calls: RefCell::new(Vec::new()),
                    perform_result: Cell::new(kResultOk),
                }
            }
        }

        impl FakeComponentHandler {
            fn calls(&self) -> Vec<HandlerCall> {
                self.calls.borrow().clone()
            }

            fn rejecting_perform() -> Self {
                Self {
                    perform_result: Cell::new(kResultFalse),
                    ..Self::default()
                }
            }
        }

        impl Class for FakeComponentHandler {
            type Interfaces = (IComponentHandler,);
        }

        impl IComponentHandlerTrait for FakeComponentHandler {
            unsafe fn beginEdit(&self, id: ParamID) -> tresult {
                self.calls.borrow_mut().push(HandlerCall::Begin(id));
                kResultOk
            }

            unsafe fn performEdit(&self, id: ParamID, value_normalized: ParamValue) -> tresult {
                self.calls
                    .borrow_mut()
                    .push(HandlerCall::Perform(id, value_normalized));
                self.perform_result.get()
            }

            unsafe fn endEdit(&self, id: ParamID) -> tresult {
                self.calls.borrow_mut().push(HandlerCall::End(id));
                kResultOk
            }

            unsafe fn restartComponent(&self, flags: int32) -> tresult {
                self.calls.borrow_mut().push(HandlerCall::Restart(flags));
                kResultOk
            }
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        struct CapturedProcess {
            events: Vec<CoreEvent>,
            transport: Transport,
            process_mode: ProcessMode,
            param_value: Option<f64>,
            no_alloc_active: bool,
        }

        static CAPTURED_PROCESS: LazyLock<Mutex<CapturedProcess>> =
            LazyLock::new(|| Mutex::new(CapturedProcess::default()));

        struct CaptureKernel;

        impl AudioKernel for CaptureKernel {
            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                let initial_gain = context.params().get_normalized("gain").unwrap_or(0.5);
                let mut captured = CAPTURED_PROCESS.lock().unwrap();
                captured.events = context.events().to_vec();
                captured.transport = context.transport();
                captured.process_mode = context.process_mode();
                captured.param_value = Some(initial_gain);
                captured.no_alloc_active = NoAllocGuard::is_active();
                drop(captured);

                let frames = context.audio().frames().min(u32::MAX as usize) as u32;
                let handle = vesty_params::ParamHandle::from_index(0);
                let output_channels = context.audio().output_channels();
                let (audio, events) = context.audio_mut_and_events();
                for segment in
                    vesty_core::ParamAutomationSegments::new(events, handle, initial_gain, frames)
                {
                    for frame in segment.start_sample..segment.end_sample {
                        for channel in 0..output_channels {
                            audio.set_output_sample(
                                channel,
                                frame as usize,
                                segment.normalized as f32,
                            );
                        }
                    }
                }
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct CapturePlugin {
            params: TestParams,
        }

        impl Plugin for CapturePlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Capture",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"process-test-000",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = CaptureKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                CaptureKernel
            }
        }

        struct ProgramAutomationParams {
            program: ChoiceParam,
        }

        impl Default for ProgramAutomationParams {
            fn default() -> Self {
                Self {
                    program: ChoiceParam::new("program", "Program", ["Init", "Lead", "Pad"], 0),
                }
            }
        }

        impl ParamCollection for ProgramAutomationParams {
            fn specs(&self) -> Vec<ParamSpec> {
                vec![self.program.spec().as_program_change()]
            }

            fn get_normalized(&self, id: &str) -> Option<f64> {
                if id == self.program.id() {
                    Some(self.program.normalized())
                } else {
                    None
                }
            }

            fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
                if id == self.program.id() {
                    self.program.set_normalized(normalized);
                    Ok(())
                } else {
                    Err(ParamError::Unknown(id.to_string()))
                }
            }

            fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
                (handle.index() == 0).then(|| self.program.normalized())
            }

            fn set_normalized_by_handle(
                &self,
                handle: ParamHandle,
                normalized: f64,
            ) -> Result<(), ParamError> {
                if handle.index() == 0 {
                    self.program.set_normalized(normalized);
                    Ok(())
                } else {
                    Err(ParamError::Unknown(format!("handle:{}", handle.index())))
                }
            }
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        struct ProgramAutomationCapture {
            events: Vec<CoreEvent>,
            param_value: Option<f64>,
            no_alloc_active: bool,
        }

        static PROGRAM_AUTOMATION_CAPTURE: LazyLock<Mutex<ProgramAutomationCapture>> =
            LazyLock::new(|| Mutex::new(ProgramAutomationCapture::default()));
        static PROGRAM_AUTOMATION_APPLY_CALLS: TestAtomicUsize = TestAtomicUsize::new(0);
        static PROGRAM_AUTOMATION_LOAD_CALLS: TestAtomicUsize = TestAtomicUsize::new(0);

        struct ProgramAutomationKernel;

        impl AudioKernel for ProgramAutomationKernel {
            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                let mut captured = PROGRAM_AUTOMATION_CAPTURE.lock().unwrap();
                captured.events = context.events().to_vec();
                captured.param_value = context.params().get_normalized("program");
                captured.no_alloc_active = NoAllocGuard::is_active();
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct ProgramAutomationPlugin {
            params: ProgramAutomationParams,
        }

        static PROGRAM_AUTOMATION_PROGRAMS: &[vesty_core::Program] = &[
            vesty_core::Program::new("Init"),
            vesty_core::Program::new("Lead"),
            vesty_core::Program::new("Pad"),
        ];
        static PROGRAM_AUTOMATION_PROGRAM_LISTS: &[vesty_core::ProgramList] =
            &[vesty_core::ProgramList::new(
                91,
                "Program Automation",
                PROGRAM_AUTOMATION_PROGRAMS,
            )];

        impl Plugin for ProgramAutomationPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Program Automation",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"program-auto-001",
                kind: PluginKind::AudioEffect,
            };

            type Params = ProgramAutomationParams;
            type Kernel = ProgramAutomationKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                ProgramAutomationKernel
            }

            fn program_lists(&self) -> &'static [vesty_core::ProgramList] {
                PROGRAM_AUTOMATION_PROGRAM_LISTS
            }

            fn apply_program(
                &self,
                _list_id: u32,
                _program_index: usize,
            ) -> Result<bool, StateError> {
                PROGRAM_AUTOMATION_APPLY_CALLS.fetch_add(1, TestOrdering::Relaxed);
                self.params
                    .set_normalized("program", 0.0)
                    .map_err(|error| StateError::custom(error.to_string()))?;
                Ok(true)
            }

            fn load_program_data(
                &self,
                _list_id: u32,
                _program_index: usize,
                _data: serde_json::Value,
            ) -> Result<bool, StateError> {
                PROGRAM_AUTOMATION_LOAD_CALLS.fetch_add(1, TestOrdering::Relaxed);
                Ok(true)
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq)]
        struct PrepareMatrixRecord {
            init_sample_rate: f64,
            init_max_block_size: usize,
            prepare_sample_rate: f64,
            prepare_max_block_size: usize,
            process_frames: usize,
            no_alloc_active: bool,
        }

        static PREPARE_MATRIX_RECORDS: LazyLock<Mutex<Vec<PrepareMatrixRecord>>> =
            LazyLock::new(|| Mutex::new(Vec::new()));
        static PREPARE_MATRIX_KERNEL_CREATIONS: TestAtomicUsize = TestAtomicUsize::new(0);
        static PREPARE_MATRIX_RESETS: TestAtomicUsize = TestAtomicUsize::new(0);
        static PREPARE_MATRIX_SUSPENDS: TestAtomicUsize = TestAtomicUsize::new(0);
        static PREPARE_MATRIX_RESUMES: TestAtomicUsize = TestAtomicUsize::new(0);
        static PREPARE_MATRIX_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

        struct PrepareMatrixKernel {
            init: KernelInit,
            prepare: PrepareContext,
        }

        impl AudioKernel for PrepareMatrixKernel {
            fn prepare(&mut self, context: PrepareContext) {
                self.prepare = context;
            }

            fn reset(&mut self) {
                PREPARE_MATRIX_RESETS.fetch_add(1, TestOrdering::Relaxed);
            }

            fn suspend(&mut self) {
                PREPARE_MATRIX_SUSPENDS.fetch_add(1, TestOrdering::Relaxed);
            }

            fn resume(&mut self) {
                PREPARE_MATRIX_RESUMES.fetch_add(1, TestOrdering::Relaxed);
            }

            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                PREPARE_MATRIX_RECORDS
                    .lock()
                    .unwrap()
                    .push(PrepareMatrixRecord {
                        init_sample_rate: self.init.sample_rate,
                        init_max_block_size: self.init.max_block_size,
                        prepare_sample_rate: self.prepare.sample_rate,
                        prepare_max_block_size: self.prepare.max_block_size,
                        process_frames: context.audio().frames(),
                        no_alloc_active: NoAllocGuard::is_active(),
                    });
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct PrepareMatrixPlugin {
            params: TestParams,
        }

        impl Plugin for PrepareMatrixPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Prepare Matrix",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"prepare-matrix!!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = PrepareMatrixKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, init: KernelInit) -> Self::Kernel {
                PREPARE_MATRIX_KERNEL_CREATIONS.fetch_add(1, TestOrdering::Relaxed);
                PrepareMatrixKernel {
                    init,
                    prepare: PrepareContext {
                        sample_rate: 0.0,
                        max_block_size: 0,
                    },
                }
            }
        }

        static NO_ALLOC_KERNEL_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
        static NO_ALLOC_GUARD_SEEN: TestAtomicBool = TestAtomicBool::new(false);
        static NO_ALLOC_INPUT_CHANNELS: TestAtomicUsize = TestAtomicUsize::new(usize::MAX);
        static NO_ALLOC_PLUGIN_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

        struct NoAllocKernel;

        impl AudioKernel for NoAllocKernel {
            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                NO_ALLOC_KERNEL_ENTERED.store(true, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(NoAllocGuard::is_active(), TestOrdering::Relaxed);
                let _frames = context.audio().frames();
                NO_ALLOC_INPUT_CHANNELS
                    .store(context.audio().input_channels(), TestOrdering::Relaxed);
                let _gain = context.param_normalized(ParamHandle::from_index(0));
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct NoAllocPlugin {
            params: TestParams,
        }

        impl Plugin for NoAllocPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "No Alloc",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"no-alloc-test!!!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = NoAllocKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                NoAllocKernel
            }
        }

        static NATIVE_F64_F32_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
        static NATIVE_F64_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
        static NATIVE_F64_GUARD_SEEN: TestAtomicBool = TestAtomicBool::new(false);
        static NATIVE_F64_FRAMES: TestAtomicUsize = TestAtomicUsize::new(0);
        static NATIVE_F64_PLUGIN_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
        const NATIVE_F64_LEFT_BIAS: f64 = 1.0e-12;
        const NATIVE_F64_RIGHT_BIAS: f64 = -2.0e-12;

        struct NativeF64Kernel;

        impl AudioKernel for NativeF64Kernel {
            const SUPPORTS_F64: bool = true;

            fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                NATIVE_F64_F32_ENTERED.store(true, TestOrdering::Relaxed);
                ProcessResult::Continue
            }

            fn process_f64(
                &mut self,
                context: &mut vesty_core::ProcessContext64<'_>,
            ) -> ProcessResult {
                NATIVE_F64_ENTERED.store(true, TestOrdering::Relaxed);
                NATIVE_F64_GUARD_SEEN.store(NoAllocGuard::is_active(), TestOrdering::Relaxed);
                let frames = context.audio().frames();
                NATIVE_F64_FRAMES.store(frames, TestOrdering::Relaxed);

                for frame in 0..frames {
                    let left = context
                        .audio()
                        .input_channel(0)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let right = context
                        .audio()
                        .input_channel(1)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    context.audio_mut().set_output_sample(
                        0,
                        frame,
                        left * 0.5 + NATIVE_F64_LEFT_BIAS,
                    );
                    context.audio_mut().set_output_sample(
                        1,
                        frame,
                        right * -0.25 + NATIVE_F64_RIGHT_BIAS,
                    );
                }

                let _ = context.emit_output_meter(88, 0);
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct NativeF64Plugin {
            params: TestParams,
        }

        impl Plugin for NativeF64Plugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Native F64",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"native-f64-test!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = NativeF64Kernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                NativeF64Kernel
            }
        }

        static NATIVE_F64_SIDECHAIN_F32_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
        static NATIVE_F64_SIDECHAIN_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
        static NATIVE_F64_SIDECHAIN_GUARD_SEEN: TestAtomicBool = TestAtomicBool::new(false);

        struct NativeF64SidechainKernel;

        impl AudioKernel for NativeF64SidechainKernel {
            const SUPPORTS_F64: bool = true;

            fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                NATIVE_F64_SIDECHAIN_F32_ENTERED.store(true, TestOrdering::Relaxed);
                ProcessResult::Continue
            }

            fn process_f64(
                &mut self,
                context: &mut vesty_core::ProcessContext64<'_>,
            ) -> ProcessResult {
                NATIVE_F64_SIDECHAIN_ENTERED.store(true, TestOrdering::Relaxed);
                NATIVE_F64_SIDECHAIN_GUARD_SEEN
                    .store(NoAllocGuard::is_active(), TestOrdering::Relaxed);
                let frames = context.audio().frames();
                assert_eq!(context.sidechain().input_channels(), 2);
                for frame in 0..frames {
                    let main_l = context
                        .audio()
                        .input_channel(0)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let main_r = context
                        .audio()
                        .input_channel(1)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let side_l = context
                        .sidechain()
                        .input_channel(0)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let side_r = context
                        .sidechain()
                        .input_channel(1)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    context
                        .audio_mut()
                        .set_output_sample(0, frame, side_l + main_l * 0.001);
                    context
                        .audio_mut()
                        .set_output_sample(1, frame, side_r + main_r * 0.002);
                }
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct NativeF64SidechainPlugin {
            params: TestParams,
        }

        impl Plugin for NativeF64SidechainPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Native F64 Sidechain",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"native-f64-side!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = NativeF64SidechainKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                NativeF64SidechainKernel
            }

            fn sidechain_inputs(&self) -> u32 {
                1
            }
        }

        static PANIC_KERNEL_CALLS: TestAtomicUsize = TestAtomicUsize::new(0);
        static PANIC_PLUGIN_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

        struct PanicKernel;

        impl AudioKernel for PanicKernel {
            fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                PANIC_KERNEL_CALLS.fetch_add(1, TestOrdering::Relaxed);
                panic!("panic plugin");
            }
        }

        struct SilenceKernel;

        impl AudioKernel for SilenceKernel {
            fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                ProcessResult::Silence
            }
        }

        struct MeterKernel;

        impl AudioKernel for MeterKernel {
            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                context.audio_mut().set_output_sample(0, 0, -0.25);
                context.audio_mut().set_output_sample(0, 1, 0.75);
                context.audio_mut().set_output_sample(1, 0, 0.5);
                context.audio_mut().set_output_sample(1, 1, -0.125);
                let _ = context.emit_output_meter(77, 3);
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct MeterPlugin {
            params: TestParams,
        }

        impl Plugin for MeterPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Meter",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"meter-test-000!!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = MeterKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                MeterKernel
            }
        }

        #[derive(Default)]
        struct PanicPlugin {
            params: TestParams,
        }

        impl Plugin for PanicPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Panic",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"panic-test-000!!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = PanicKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                PanicKernel
            }
        }

        struct DefaultPanicPlugin;

        impl Default for DefaultPanicPlugin {
            fn default() -> Self {
                panic!("default panic")
            }
        }

        impl Plugin for DefaultPanicPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Default Panic",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"default-panic-01",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = SilenceKernel;

            fn params(&self) -> &Self::Params {
                unreachable!("default panic plugin is never constructed")
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                SilenceKernel
            }
        }

        #[derive(Default)]
        struct CallbackPanicPlugin {
            params: TestParams,
        }

        impl Plugin for CallbackPanicPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Callback Panic",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"callback-panic-1",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = SilenceKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                SilenceKernel
            }

            fn ui(&self) -> Option<UiDescriptor> {
                panic!("ui panic")
            }

            fn latency_samples(&self) -> u32 {
                panic!("latency panic")
            }

            fn save_custom_state(&self) -> Result<Option<serde_json::Value>, StateError> {
                panic!("state panic")
            }
        }

        #[derive(Default)]
        struct SilencePlugin {
            params: TestParams,
        }

        impl Plugin for SilencePlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Silence",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"silence-test-000",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = SilenceKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                SilenceKernel
            }
        }

        #[derive(Default)]
        struct InstrumentPlugin {
            params: TestParams,
        }

        impl Plugin for InstrumentPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Instrument",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"instrument-test!",
                kind: PluginKind::Instrument,
            };

            type Params = TestParams;
            type Kernel = Kernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                Kernel
            }
        }

        static MULTI_OUTPUT_INSTRUMENT_BUSES: &[AudioOutputBus] = &[
            AudioOutputBus::stereo("Main"),
            AudioOutputBus::stereo("Aux 1"),
        ];

        struct MultiOutputInstrumentKernel;

        impl AudioKernel for MultiOutputInstrumentKernel {
            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                let frames = context.audio().frames();
                for frame in 0..frames {
                    context.audio_mut().set_output_sample(0, frame, 0.10);
                    context.audio_mut().set_output_sample(1, frame, 0.20);
                    context.audio_mut().set_output_sample(2, frame, 0.30);
                    context.audio_mut().set_output_sample(3, frame, 0.40);
                }
                ProcessResult::Continue
            }
        }

        static MULTI_OUTPUT_NATIVE_F64_F32_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
        static MULTI_OUTPUT_NATIVE_F64_ENTERED: TestAtomicBool = TestAtomicBool::new(false);

        struct MultiOutputNativeF64InstrumentKernel;

        impl AudioKernel for MultiOutputNativeF64InstrumentKernel {
            const SUPPORTS_F64: bool = true;

            fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                MULTI_OUTPUT_NATIVE_F64_F32_ENTERED.store(true, TestOrdering::Relaxed);
                ProcessResult::Continue
            }

            fn process_f64(
                &mut self,
                context: &mut vesty_core::ProcessContext64<'_>,
            ) -> ProcessResult {
                MULTI_OUTPUT_NATIVE_F64_ENTERED.store(true, TestOrdering::Relaxed);
                let frames = context.audio().frames();
                for frame in 0..frames {
                    context.audio_mut().set_output_sample(0, frame, 1.10);
                    context.audio_mut().set_output_sample(1, frame, 1.20);
                    context.audio_mut().set_output_sample(2, frame, 1.30);
                    context.audio_mut().set_output_sample(3, frame, 1.40);
                }
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct MultiOutputInstrumentPlugin {
            params: TestParams,
        }

        impl Plugin for MultiOutputInstrumentPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Multi Output Instrument",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"multi-output-ins",
                kind: PluginKind::Instrument,
            };

            type Params = TestParams;
            type Kernel = MultiOutputInstrumentKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                MultiOutputInstrumentKernel
            }

            fn output_buses(&self) -> &'static [AudioOutputBus] {
                MULTI_OUTPUT_INSTRUMENT_BUSES
            }
        }

        #[derive(Default)]
        struct MultiOutputNativeF64InstrumentPlugin {
            params: TestParams,
        }

        impl Plugin for MultiOutputNativeF64InstrumentPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Multi Output Native F64 Instrument",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"multi-f64-out!!!",
                kind: PluginKind::Instrument,
            };

            type Params = TestParams;
            type Kernel = MultiOutputNativeF64InstrumentKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                MultiOutputNativeF64InstrumentKernel
            }

            fn output_buses(&self) -> &'static [AudioOutputBus] {
                MULTI_OUTPUT_INSTRUMENT_BUSES
            }
        }

        struct SidechainKernel;

        impl AudioKernel for SidechainKernel {
            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                let frames = context.audio().frames();
                assert_eq!(context.sidechain().input_channels(), 2);
                for frame in 0..frames {
                    let main_l = context
                        .audio()
                        .input_channel(0)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let main_r = context
                        .audio()
                        .input_channel(1)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let side_l = context
                        .sidechain()
                        .input_channel(0)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let side_r = context
                        .sidechain()
                        .input_channel(1)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    context
                        .audio_mut()
                        .set_output_sample(0, frame, side_l + main_l * 0.01);
                    context
                        .audio_mut()
                        .set_output_sample(1, frame, side_r + main_r * 0.01);
                }
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct SidechainPlugin {
            params: TestParams,
        }

        impl Plugin for SidechainPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Sidechain",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"sidechain-test!!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = SidechainKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                SidechainKernel
            }

            fn sidechain_inputs(&self) -> u32 {
                1
            }
        }

        struct OptionalSidechainKernel;

        impl AudioKernel for OptionalSidechainKernel {
            fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
                let frames = context.audio().frames();
                for frame in 0..frames {
                    let main_l = context
                        .audio()
                        .input_channel(0)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let main_r = context
                        .audio()
                        .input_channel(1)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let side_l = context
                        .sidechain()
                        .input_channel(0)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    let side_r = context
                        .sidechain()
                        .input_channel(1)
                        .and_then(|channel| channel.get(frame))
                        .copied()
                        .unwrap_or(0.0);
                    context
                        .audio_mut()
                        .set_output_sample(0, frame, side_l + main_l * 0.01);
                    context
                        .audio_mut()
                        .set_output_sample(1, frame, side_r + main_r * 0.01);
                }
                ProcessResult::Continue
            }
        }

        #[derive(Default)]
        struct OptionalSidechainPlugin {
            params: TestParams,
        }

        impl Plugin for OptionalSidechainPlugin {
            const INFO: PluginInfo = PluginInfo {
                name: "Optional Sidechain",
                vendor: "Vesty",
                url: "",
                email: "",
                version: "0.1.0",
                class_id: *b"sidechain-loose!",
                kind: PluginKind::AudioEffect,
            };

            type Params = TestParams;
            type Kernel = OptionalSidechainKernel;

            fn params(&self) -> &Self::Params {
                &self.params
            }

            fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
                OptionalSidechainKernel
            }

            fn sidechain_inputs(&self) -> u32 {
                1
            }
        }

        #[test]
        fn factory_reports_processor_and_controller_classes() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                assert_eq!(factory.countClasses(), 2);

                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let mut processor = MaybeUninit::<PClassInfo>::zeroed();
                assert_eq!(factory.getClassInfo(0, processor.as_mut_ptr()), kResultOk);
                let processor = processor.assume_init();
                assert_eq!(processor.cid, tuid(metadata.processor_class_id));

                let mut controller = MaybeUninit::<PClassInfo>::zeroed();
                assert_eq!(factory.getClassInfo(1, controller.as_mut_ptr()), kResultOk);
                let controller = controller.assume_init();
                assert_eq!(controller.cid, tuid(metadata.controller_class_id));
            }
        }

        #[test]
        fn factory_rejects_null_pointers_and_clears_failed_instance_output() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid host pointers.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);

                assert_eq!(factory.getFactoryInfo(ptr::null_mut()), kInvalidArgument);
                assert_eq!(factory.getClassInfo(0, ptr::null_mut()), kInvalidArgument);

                let stale = ptr::dangling_mut::<c_void>();
                let mut instance = stale;
                assert_eq!(
                    factory.createInstance(ptr::null(), IComponent_iid.as_ptr(), &mut instance,),
                    kInvalidArgument
                );
                assert!(instance.is_null());

                instance = stale;
                assert_eq!(
                    factory.createInstance(processor_cid.as_ptr(), ptr::null(), &mut instance),
                    kInvalidArgument
                );
                assert!(instance.is_null());

                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IComponent_iid.as_ptr(),
                        ptr::null_mut(),
                    ),
                    kInvalidArgument
                );

                let unknown_cid = [42 as c_char; 16];
                instance = stale;
                assert_eq!(
                    factory.createInstance(
                        unknown_cid.as_ptr(),
                        IComponent_iid.as_ptr(),
                        &mut instance,
                    ),
                    kInvalidArgument
                );
                assert!(instance.is_null());
            }
        }

        #[test]
        fn factory_creates_component_and_controller_instances() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();

                let processor_cid = tuid(metadata.processor_class_id);
                let mut component: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IComponent_iid.as_ptr(),
                        &mut component,
                    ),
                    kResultOk
                );
                let component = ComPtr::<IComponent>::from_raw(component as *mut IComponent)
                    .expect("component");
                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                    ),
                    1
                );

                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                assert_eq!(controller.getParameterCount(), 2);

                let mut midi_mapping: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IMidiMapping_iid.as_ptr(),
                        &mut midi_mapping,
                    ),
                    kResultOk
                );
                let _midi_mapping =
                    ComPtr::<IMidiMapping>::from_raw(midi_mapping as *mut IMidiMapping)
                        .expect("midi mapping");

                let mut info = MaybeUninit::<ParameterInfo>::zeroed();
                assert_eq!(controller.getParameterInfo(0, info.as_mut_ptr()), kResultOk);
                let info = info.assume_init();
                assert_eq!(info.id, test_param_id("gain"));
                assert_eq!(
                    info.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
                    ParameterInfo_::ParameterFlags_::kCanAutomate
                );
            }
        }

        #[test]
        fn controller_exposes_opt_in_midi_mapping() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut mapping: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IMidiMapping_iid.as_ptr(),
                        &mut mapping,
                    ),
                    kResultOk
                );
                let mapping = ComPtr::<IMidiMapping>::from_raw(mapping as *mut IMidiMapping)
                    .expect("midi mapping");

                let mut id = ParamID::MAX;
                assert_eq!(
                    mapping.getMidiControllerAssignment(0, 9, 7, &mut id),
                    kResultTrue
                );
                assert_eq!(id, test_param_id("gain"));

                id = ParamID::MAX;
                assert_eq!(
                    mapping.getMidiControllerAssignment(0, 1, 74, &mut id),
                    kResultFalse
                );
                assert_eq!(id, ParamID::MAX);

                assert_eq!(
                    mapping.getMidiControllerAssignment(0, 2, 74, &mut id),
                    kResultTrue
                );
                assert_eq!(id, test_param_id("cutoff"));

                assert_eq!(
                    mapping.getMidiControllerAssignment(
                        0,
                        1,
                        vesty_params::midi::PITCH_BEND as CtrlNumber,
                        &mut id,
                    ),
                    kResultTrue
                );
                assert_eq!(id, test_param_id("pitch"));

                assert_eq!(
                    mapping.getMidiControllerAssignment(0, 0, 10, &mut id),
                    kResultFalse
                );
                assert_eq!(
                    mapping.getMidiControllerAssignment(1, 9, 7, &mut id),
                    kResultFalse
                );
                assert_eq!(
                    mapping.getMidiControllerAssignment(0, -1, 7, &mut id),
                    kResultFalse
                );
                assert_eq!(
                    mapping.getMidiControllerAssignment(0, 0, -1, &mut id),
                    kResultFalse
                );
                assert_eq!(
                    mapping.getMidiControllerAssignment(0, 0, 7, ptr::null_mut()),
                    kInvalidArgument
                );
            }
        }

        #[test]
        fn controller_exposes_program_list_metadata() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut unit_info: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IUnitInfo_iid.as_ptr(),
                        &mut unit_info,
                    ),
                    kResultOk
                );
                let unit_info =
                    ComPtr::<IUnitInfo>::from_raw(unit_info as *mut IUnitInfo).expect("unit info");
                let controller = unit_info
                    .cast::<IEditController>()
                    .expect("same controller edit interface");
                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );
                let gain_id = test_param_id("gain");
                let cutoff_id = test_param_id("cutoff");
                let pitch_id = test_param_id("pitch");

                assert_eq!(unit_info.getUnitCount(), 1);
                let mut root = MaybeUninit::<UnitInfo>::zeroed();
                assert_eq!(unit_info.getUnitInfo(0, root.as_mut_ptr()), kResultOk);
                let root = root.assume_init();
                assert_eq!(root.id, kRootUnitId);
                assert_eq!(root.parentUnitId, kNoParentUnitId);
                assert_eq!(root.programListId, 77);
                assert_eq!(string128_to_string(&root.name), "Midi Mapped");
                assert_eq!(unit_info.getSelectedUnit(), kRootUnitId);
                assert_eq!(unit_info.selectUnit(kRootUnitId), kResultOk);
                assert_eq!(unit_info.selectUnit(99), kInvalidArgument);

                assert_eq!(unit_info.getProgramListCount(), 1);
                let mut list = MaybeUninit::<ProgramListInfo>::zeroed();
                assert_eq!(
                    unit_info.getProgramListInfo(0, list.as_mut_ptr()),
                    kResultOk
                );
                let list = list.assume_init();
                assert_eq!(list.id, 77);
                assert_eq!(list.programCount, 3);
                assert_eq!(string128_to_string(&list.name), "Factory Programs");

                let mut name = [0; 128];
                assert_eq!(unit_info.getProgramName(77, 1, &mut name), kResultOk);
                assert_eq!(string128_to_string(&name), "Bright Lead");
                assert_eq!(unit_info.getProgramName(77, 3, &mut name), kInvalidArgument);
                assert_eq!(
                    unit_info.getProgramName(999, 0, &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramInfo(77, 1, c"category".as_ptr() as FIDString, &mut name),
                    kResultOk
                );
                assert_eq!(string128_to_string(&name), "Lead");
                assert_eq!(
                    unit_info.getProgramInfo(77, 1, c"mood".as_ptr() as FIDString, &mut name),
                    kResultOk
                );
                assert_eq!(string128_to_string(&name), "Bright");
                assert_eq!(
                    unit_info.getProgramInfo(77, 1, c"unknown".as_ptr() as FIDString, &mut name),
                    kResultFalse
                );
                assert_eq!(
                    unit_info.getProgramInfo(
                        77,
                        1,
                        c"invalid_value".as_ptr() as FIDString,
                        &mut name
                    ),
                    kResultFalse
                );
                assert_eq!(
                    unit_info.getProgramInfo(77, 0, c"category".as_ptr() as FIDString, &mut name),
                    kResultFalse
                );
                assert_eq!(
                    unit_info.getProgramInfo(77, 3, c"category".as_ptr() as FIDString, &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramInfo(999, 1, c"category".as_ptr() as FIDString, &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramInfo(77, 1, ptr::null(), &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramInfo(
                        77,
                        1,
                        c"category".as_ptr() as FIDString,
                        ptr::null_mut()
                    ),
                    kInvalidArgument
                );

                assert_eq!(unit_info.hasProgramPitchNames(77, 0), kResultFalse);
                assert_eq!(unit_info.hasProgramPitchNames(77, 1), kResultTrue);
                assert_eq!(unit_info.hasProgramPitchNames(77, 3), kInvalidArgument);
                assert_eq!(unit_info.hasProgramPitchNames(999, 1), kInvalidArgument);
                assert_eq!(
                    unit_info.getProgramPitchName(77, 1, 60, &mut name),
                    kResultOk
                );
                assert_eq!(string128_to_string(&name), "C4 Lead");
                assert_eq!(
                    unit_info.getProgramPitchName(77, 1, 64, &mut name),
                    kResultOk
                );
                assert_eq!(string128_to_string(&name), "E4 Lead");
                assert_eq!(
                    unit_info.getProgramPitchName(77, 1, 63, &mut name),
                    kResultFalse
                );
                assert_eq!(
                    unit_info.getProgramPitchName(77, 1, 62, &mut name),
                    kResultFalse
                );
                assert_eq!(
                    unit_info.getProgramPitchName(77, 0, 60, &mut name),
                    kResultFalse
                );
                assert_eq!(
                    unit_info.getProgramPitchName(77, 3, 60, &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramPitchName(999, 1, 60, &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramPitchName(77, 1, -1, &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramPitchName(77, 1, 128, &mut name),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.getProgramPitchName(77, 1, 60, ptr::null_mut()),
                    kInvalidArgument
                );

                let mut unit_id = -99;
                assert_eq!(
                    unit_info.getUnitByBus(
                        MediaTypes_::kEvent as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                        -1,
                        &mut unit_id,
                    ),
                    kResultOk
                );
                assert_eq!(unit_id, kRootUnitId);
                assert_eq!(
                    unit_info.getUnitByBus(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                        -1,
                        &mut unit_id,
                    ),
                    kInvalidArgument
                );

                assert_eq!(
                    unit_info.setUnitProgramData(77, 1, ptr::null_mut()),
                    kResultOk
                );
                assert!((controller.getParamNormalized(gain_id) - 0.8).abs() < 0.000_001);
                assert!((controller.getParamNormalized(cutoff_id) - 0.9).abs() < 0.000_001);
                assert!((controller.getParamNormalized(pitch_id) - 0.65).abs() < 0.000_001);
                assert_eq!(
                    handler.calls(),
                    vec![HandlerCall::Restart(RestartFlags_::kParamValuesChanged)]
                );

                assert_eq!(
                    unit_info.setUnitProgramData(kRootUnitId, 2, ptr::null_mut()),
                    kResultOk
                );
                assert!((controller.getParamNormalized(gain_id) - 0.3).abs() < 0.000_001);
                assert!((controller.getParamNormalized(cutoff_id) - 0.35).abs() < 0.000_001);
                assert!((controller.getParamNormalized(pitch_id) - 0.4).abs() < 0.000_001);
                assert_eq!(
                    unit_info.setUnitProgramData(999, 0, ptr::null_mut()),
                    kInvalidArgument
                );
                assert_eq!(
                    unit_info.setUnitProgramData(77, 99, ptr::null_mut()),
                    kInvalidArgument
                );
            }
        }

        #[test]
        fn controller_supports_program_data_streams() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut program_list_data: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IProgramListData_iid.as_ptr(),
                        &mut program_list_data,
                    ),
                    kResultOk
                );
                let program_list_data = ComPtr::<IProgramListData>::from_raw(
                    program_list_data as *mut IProgramListData,
                )
                .expect("program list data");
                let controller = program_list_data
                    .cast::<IEditController>()
                    .expect("same controller edit interface");
                let unit_info = program_list_data
                    .cast::<IUnitInfo>()
                    .expect("same controller unit info interface");
                let gain_id = test_param_id("gain");
                let cutoff_id = test_param_id("cutoff");
                let pitch_id = test_param_id("pitch");
                let assert_param = |id: ParamID, expected: f64| {
                    assert!((controller.getParamNormalized(id) - expected).abs() < 0.000_001);
                };

                assert_eq!(program_list_data.programDataSupported(77), kResultTrue);
                assert_eq!(
                    program_list_data.programDataSupported(999),
                    kInvalidArgument
                );

                assert_eq!(controller.setParamNormalized(gain_id, 0.42), kResultOk);
                assert_eq!(controller.setParamNormalized(cutoff_id, 0.84), kResultOk);
                assert_eq!(controller.setParamNormalized(pitch_id, 0.21), kResultOk);

                let saved = ComWrapper::new(MemoryStream::default());
                let saved_ptr = saved.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.getProgramData(77, 1, saved_ptr.as_ptr()),
                    kResultOk
                );
                let saved_bytes = saved.bytes();
                let saved_json = program_data_json(&saved_bytes);
                assert_eq!(saved_json["version"], 1);
                assert_eq!(saved_json["listId"], 77);
                assert_eq!(saved_json["programIndex"], 1);
                assert!((saved_json["data"]["gain"].as_f64().unwrap() - 0.42).abs() < 0.000_001);
                assert!((saved_json["data"]["cutoff"].as_f64().unwrap() - 0.84).abs() < 0.000_001);
                assert!((saved_json["data"]["pitch"].as_f64().unwrap() - 0.21).abs() < 0.000_001);

                assert_eq!(controller.setParamNormalized(gain_id, 0.1), kResultOk);
                assert_eq!(controller.setParamNormalized(cutoff_id, 0.2), kResultOk);
                assert_eq!(controller.setParamNormalized(pitch_id, 0.3), kResultOk);
                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );
                let input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes.clone()));
                let input_ptr = input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.setProgramData(77, 1, input_ptr.as_ptr()),
                    kResultOk
                );
                assert_param(gain_id, 0.42);
                assert_param(cutoff_id, 0.84);
                assert_param(pitch_id, 0.21);
                assert_eq!(
                    handler.calls(),
                    vec![HandlerCall::Restart(RestartFlags_::kParamValuesChanged)]
                );

                assert_eq!(controller.setParamNormalized(gain_id, 0.11), kResultOk);
                assert_eq!(controller.setParamNormalized(cutoff_id, 0.22), kResultOk);
                assert_eq!(controller.setParamNormalized(pitch_id, 0.33), kResultOk);
                let unit_input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes.clone()));
                let unit_input_ptr = unit_input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    unit_info.setUnitProgramData(77, 1, unit_input_ptr.as_ptr()),
                    kResultOk
                );
                assert_param(gain_id, 0.42);
                assert_param(cutoff_id, 0.84);
                assert_param(pitch_id, 0.21);

                assert_eq!(controller.setParamNormalized(gain_id, 0.14), kResultOk);
                assert_eq!(controller.setParamNormalized(cutoff_id, 0.24), kResultOk);
                assert_eq!(controller.setParamNormalized(pitch_id, 0.34), kResultOk);
                let root_input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes.clone()));
                let root_input_ptr = root_input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    unit_info.setUnitProgramData(kRootUnitId, 1, root_input_ptr.as_ptr()),
                    kResultOk
                );
                assert_param(gain_id, 0.42);
                assert_param(cutoff_id, 0.84);
                assert_param(pitch_id, 0.21);

                let out_of_range = ComWrapper::new(MemoryStream::default());
                let out_of_range_ptr = out_of_range.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.getProgramData(77, 99, out_of_range_ptr.as_ptr()),
                    kInvalidArgument
                );
                let unknown_list = ComWrapper::new(MemoryStream::default());
                let unknown_list_ptr = unknown_list.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.getProgramData(999, 0, unknown_list_ptr.as_ptr()),
                    kInvalidArgument
                );
                assert_eq!(
                    program_list_data.getProgramData(77, 1, ptr::null_mut()),
                    kInvalidArgument
                );
                assert_eq!(
                    program_list_data.setProgramData(77, 1, ptr::null_mut()),
                    kInvalidArgument
                );

                let bad_magic = ComWrapper::new(MemoryStream::with_bytes(b"bad".to_vec()));
                let bad_magic_ptr = bad_magic.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.setProgramData(77, 1, bad_magic_ptr.as_ptr()),
                    kInvalidArgument
                );

                let future_version = ComWrapper::new(MemoryStream::with_bytes(
                    raw_program_data_bytes_with_version(
                        2,
                        77,
                        1,
                        serde_json::json!({
                            "gain": 0.42,
                            "cutoff": 0.84,
                            "pitch": 0.21,
                        }),
                    ),
                ));
                let future_version_ptr = future_version.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.setProgramData(77, 1, future_version_ptr.as_ptr()),
                    kInvalidArgument
                );

                let list_mismatch =
                    ComWrapper::new(MemoryStream::with_bytes(raw_program_data_bytes(
                        78,
                        1,
                        serde_json::json!({
                            "gain": 0.42,
                            "cutoff": 0.84,
                            "pitch": 0.21,
                        }),
                    )));
                let list_mismatch_ptr = list_mismatch.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.setProgramData(77, 1, list_mismatch_ptr.as_ptr()),
                    kInvalidArgument
                );

                let program_mismatch =
                    ComWrapper::new(MemoryStream::with_bytes(raw_program_data_bytes(
                        77,
                        2,
                        serde_json::json!({
                            "gain": 0.42,
                            "cutoff": 0.84,
                            "pitch": 0.21,
                        }),
                    )));
                let program_mismatch_ptr = program_mismatch.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.setProgramData(77, 1, program_mismatch_ptr.as_ptr()),
                    kInvalidArgument
                );

                let missing_data = ComWrapper::new(MemoryStream::with_bytes(
                    raw_program_data_bytes(77, 1, serde_json::json!({ "gain": 0.42 })),
                ));
                let missing_data_ptr = missing_data.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    program_list_data.setProgramData(77, 1, missing_data_ptr.as_ptr()),
                    kResultFalse
                );
            }
        }

        #[test]
        fn controller_program_change_param_selects_visible_program() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<MidiMappedPlugin>::new();
                let gain_id = test_param_id("gain");
                let cutoff_id = test_param_id("cutoff");
                let pitch_id = test_param_id("pitch");
                let program_id = test_param_id("program");
                let assert_param = |id: ParamID, expected: f64| {
                    assert!((controller.getParamNormalized(id) - expected).abs() < 0.000_001);
                };

                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );

                assert_eq!(controller.setParamNormalized(gain_id, 0.11), kResultOk);
                assert_eq!(controller.setParamNormalized(cutoff_id, 0.22), kResultOk);
                assert_eq!(controller.setParamNormalized(pitch_id, 0.33), kResultOk);

                assert_eq!(controller.setParamNormalized(program_id, 0.5), kResultOk);
                assert_param(gain_id, 0.8);
                assert_param(cutoff_id, 0.9);
                assert_param(pitch_id, 0.65);
                assert_param(program_id, 0.5);
                assert_eq!(
                    handler.calls(),
                    vec![HandlerCall::Restart(RestartFlags_::kParamValuesChanged)]
                );
            }
        }

        #[test]
        fn controller_program_change_param_gesture_selects_visible_program() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<MidiMappedPlugin>::new();
                let gain_id = test_param_id("gain");
                let cutoff_id = test_param_id("cutoff");
                let pitch_id = test_param_id("pitch");
                let program_id = test_param_id("program");
                let assert_param = |id: ParamID, expected: f64| {
                    assert!((controller.getParamNormalized(id) - expected).abs() < 0.000_001);
                };

                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );

                assert_eq!(controller.perform_param_edit(program_id, 1.0), kResultOk);
                assert_param(gain_id, 0.3);
                assert_param(cutoff_id, 0.35);
                assert_param(pitch_id, 0.4);
                assert_param(program_id, 1.0);
                assert_eq!(
                    handler.calls(),
                    vec![
                        HandlerCall::Perform(program_id, 1.0),
                        HandlerCall::Restart(RestartFlags_::kParamValuesChanged),
                    ]
                );
            }
        }

        #[test]
        fn controller_exposes_note_expression_metadata() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MidiMappedPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<MidiMappedPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut note_expression: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        INoteExpressionController_iid.as_ptr(),
                        &mut note_expression,
                    ),
                    kResultOk
                );
                let mut physical_mapping: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        INoteExpressionPhysicalUIMapping_iid.as_ptr(),
                        &mut physical_mapping,
                    ),
                    kResultOk
                );
                let note_expression = ComPtr::<INoteExpressionController>::from_raw(
                    note_expression as *mut INoteExpressionController,
                )
                .expect("note expression controller");
                let physical_mapping = ComPtr::<INoteExpressionPhysicalUIMapping>::from_raw(
                    physical_mapping as *mut INoteExpressionPhysicalUIMapping,
                )
                .expect("note expression physical ui mapping");

                assert_eq!(note_expression.getNoteExpressionCount(0, -1), 2);
                assert_eq!(note_expression.getNoteExpressionCount(0, 15), 2);
                assert_eq!(note_expression.getNoteExpressionCount(1, 0), 0);
                assert_eq!(note_expression.getNoteExpressionCount(0, 16), 0);

                let mut info = MaybeUninit::<NoteExpressionTypeInfo>::zeroed();
                assert_eq!(
                    note_expression.getNoteExpressionInfo(0, 0, 0, info.as_mut_ptr()),
                    kResultOk
                );
                let expression_info = info.assume_init();
                assert_eq!(
                    expression_info.typeId,
                    NoteExpressionTypeIDs_::kBrightnessTypeID
                );
                assert_eq!(string128_to_string(&expression_info.title), "Brightness");
                assert_eq!(string128_to_string(&expression_info.shortTitle), "Bright");
                assert_eq!(expression_info.valueDesc.defaultValue, 0.5);
                assert_eq!(expression_info.valueDesc.minimum, 0.0);
                assert_eq!(expression_info.valueDesc.maximum, 1.0);
                let absolute_flag =
                    NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsAbsolute as int32;
                assert_eq!(expression_info.flags & absolute_flag, absolute_flag);
                assert_eq!(expression_info.associatedParameterId, kNoParamId);

                assert_eq!(
                    note_expression.getNoteExpressionInfo(0, 0, 2, info.as_mut_ptr()),
                    kInvalidArgument
                );
                assert_eq!(
                    note_expression.getNoteExpressionInfo(1, 0, 0, info.as_mut_ptr()),
                    kInvalidArgument
                );
                assert_eq!(
                    note_expression.getNoteExpressionInfo(0, 0, 0, ptr::null_mut()),
                    kInvalidArgument
                );

                let mut text = [0; 128];
                assert_eq!(
                    note_expression.getNoteExpressionStringByValue(
                        0,
                        0,
                        NoteExpressionTypeIDs_::kBrightnessTypeID,
                        0.625,
                        &mut text,
                    ),
                    kResultOk
                );
                assert_eq!(string128_to_string(&text), "0.625");

                let parse_input = wide_cstring("0.875");
                let mut parsed = 0.0;
                assert_eq!(
                    note_expression.getNoteExpressionValueByString(
                        0,
                        0,
                        NoteExpressionTypeIDs_::kBrightnessTypeID,
                        parse_input.as_ptr(),
                        &mut parsed,
                    ),
                    kResultOk
                );
                assert_eq!(parsed, 0.875);

                let mut bounded_parse_input = [b' ' as u16; 128];
                for (index, unit) in "0.5".encode_utf16().enumerate() {
                    bounded_parse_input[index] = unit;
                }
                parsed = 0.0;
                assert_eq!(
                    note_expression.getNoteExpressionValueByString(
                        0,
                        0,
                        NoteExpressionTypeIDs_::kBrightnessTypeID,
                        bounded_parse_input.as_ptr(),
                        &mut parsed,
                    ),
                    kResultOk
                );
                assert_eq!(parsed, 0.5);
                assert_eq!(
                    note_expression.getNoteExpressionStringByValue(
                        0,
                        0,
                        NoteExpressionTypeIDs_::kInvalidTypeID,
                        0.5,
                        &mut text,
                    ),
                    kInvalidArgument
                );
                assert_eq!(
                    note_expression.getNoteExpressionValueByString(
                        0,
                        0,
                        NoteExpressionTypeIDs_::kInvalidTypeID,
                        parse_input.as_ptr(),
                        &mut parsed,
                    ),
                    kInvalidArgument
                );

                let mut maps = [PhysicalUIMap {
                    physicalUITypeID: PhysicalUITypeIDs_::kInvalidPUITypeID as PhysicalUITypeID,
                    noteExpressionTypeID: NoteExpressionTypeIDs_::kInvalidTypeID,
                }; 3];
                let mut list = PhysicalUIMapList {
                    count: maps.len() as uint32,
                    map: maps.as_mut_ptr(),
                };
                assert_eq!(
                    physical_mapping.getPhysicalUIMapping(0, -1, &mut list),
                    kResultOk
                );
                assert_eq!(list.count, 2);
                assert_eq!(
                    maps[0].physicalUITypeID,
                    PhysicalUITypeIDs_::kPUIPressure as PhysicalUITypeID
                );
                assert_eq!(
                    maps[0].noteExpressionTypeID,
                    NoteExpressionTypeIDs_::kBrightnessTypeID
                );
                assert_eq!(
                    maps[1].physicalUITypeID,
                    PhysicalUITypeIDs_::kPUIYMovement as PhysicalUITypeID
                );
                assert_eq!(
                    maps[1].noteExpressionTypeID,
                    NoteExpressionTypeIDs_::kTuningTypeID
                );

                let mut one_map = [PhysicalUIMap {
                    physicalUITypeID: PhysicalUITypeIDs_::kInvalidPUITypeID as PhysicalUITypeID,
                    noteExpressionTypeID: NoteExpressionTypeIDs_::kInvalidTypeID,
                }; 1];
                let mut one_list = PhysicalUIMapList {
                    count: one_map.len() as uint32,
                    map: one_map.as_mut_ptr(),
                };
                assert_eq!(
                    physical_mapping.getPhysicalUIMapping(0, 0, &mut one_list),
                    kResultOk
                );
                assert_eq!(one_list.count, 1);
                assert_eq!(
                    one_map[0].physicalUITypeID,
                    PhysicalUITypeIDs_::kPUIPressure as PhysicalUITypeID
                );
                assert_eq!(
                    physical_mapping.getPhysicalUIMapping(1, 0, &mut list),
                    kInvalidArgument
                );
                assert_eq!(
                    physical_mapping.getPhysicalUIMapping(0, 16, &mut list),
                    kInvalidArgument
                );
                assert_eq!(
                    physical_mapping.getPhysicalUIMapping(0, 0, ptr::null_mut()),
                    kInvalidArgument
                );
                let mut invalid_list = PhysicalUIMapList {
                    count: 1,
                    map: ptr::null_mut(),
                };
                assert_eq!(
                    physical_mapping.getPhysicalUIMapping(0, 0, &mut invalid_list),
                    kInvalidArgument
                );
            }
        }

        #[test]
        fn factory_rejects_processor_and_controller_with_invalid_param_schema() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    InvalidParamSchemaPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<InvalidParamSchemaPlugin>();

                let processor_cid = tuid(metadata.processor_class_id);
                let mut component: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IComponent_iid.as_ptr(),
                        &mut component,
                    ),
                    kResultFalse
                );
                assert!(component.is_null());

                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultFalse
                );
                assert!(controller.is_null());
            }
        }

        #[test]
        fn controller_maps_bypass_and_read_only_parameter_flags() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<FlagPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<FlagPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                assert_eq!(controller.getParameterCount(), 3);

                let mut bypass = MaybeUninit::<ParameterInfo>::zeroed();
                assert_eq!(
                    controller.getParameterInfo(0, bypass.as_mut_ptr()),
                    kResultOk
                );
                let bypass = bypass.assume_init();
                assert_eq!(
                    bypass.flags & ParameterInfo_::ParameterFlags_::kIsBypass,
                    ParameterInfo_::ParameterFlags_::kIsBypass
                );
                assert_eq!(
                    bypass.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
                    ParameterInfo_::ParameterFlags_::kCanAutomate
                );

                let mut meter = MaybeUninit::<ParameterInfo>::zeroed();
                assert_eq!(
                    controller.getParameterInfo(1, meter.as_mut_ptr()),
                    kResultOk
                );
                let meter = meter.assume_init();
                assert_eq!(meter.flags & ParameterInfo_::ParameterFlags_::kIsBypass, 0);
                assert_eq!(
                    meter.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
                    0
                );
                assert_eq!(
                    controller.setParamNormalized(meter.id, 0.75),
                    kInvalidArgument
                );
                assert_eq!(controller.getParamNormalized(meter.id), 0.0);

                let mut program = MaybeUninit::<ParameterInfo>::zeroed();
                assert_eq!(
                    controller.getParameterInfo(2, program.as_mut_ptr()),
                    kResultOk
                );
                let program = program.assume_init();
                assert_eq!(
                    program.flags & ParameterInfo_::ParameterFlags_::kIsProgramChange,
                    ParameterInfo_::ParameterFlags_::kIsProgramChange
                );
                assert_eq!(
                    program.flags & ParameterInfo_::ParameterFlags_::kCanAutomate,
                    ParameterInfo_::ParameterFlags_::kCanAutomate
                );
                assert_eq!(controller.setParamNormalized(program.id, 1.0), kResultOk);
                assert_eq!(controller.getParamNormalized(program.id), 1.0);
            }
        }

        #[test]
        fn processor_negotiates_mvp_effect_bus_arrangements() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    TestPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let component = processor_wrapper.to_com_ptr::<IComponent>().unwrap();
                assert_eq!(
                    processor.canProcessSampleSize(SymbolicSampleSizes_::kSample32 as int32),
                    kResultOk
                );
                assert_eq!(
                    processor.canProcessSampleSize(SymbolicSampleSizes_::kSample64 as int32),
                    kResultOk
                );

                for (mut input, mut output, expected_input_channels, expected_output_channels) in [
                    (SpeakerArr::kMono, SpeakerArr::kMono, 1, 1),
                    (SpeakerArr::kMono, SpeakerArr::kStereo, 1, 2),
                    (SpeakerArr::kStereo, SpeakerArr::kStereo, 2, 2),
                ] {
                    assert_eq!(
                        processor.setBusArrangements(&mut input, 1, &mut output, 1),
                        kResultTrue
                    );

                    let mut current_input = 0;
                    let mut current_output = 0;
                    assert_eq!(
                        processor.getBusArrangement(
                            BusDirections_::kInput as BusDirection,
                            0,
                            &mut current_input,
                        ),
                        kResultOk
                    );
                    assert_eq!(
                        processor.getBusArrangement(
                            BusDirections_::kOutput as BusDirection,
                            0,
                            &mut current_output,
                        ),
                        kResultOk
                    );
                    assert_eq!(current_input, input);
                    assert_eq!(current_output, output);

                    let mut input_info = MaybeUninit::<BusInfo>::zeroed();
                    assert_eq!(
                        component.getBusInfo(
                            MediaTypes_::kAudio as MediaType,
                            BusDirections_::kInput as BusDirection,
                            0,
                            input_info.as_mut_ptr(),
                        ),
                        kResultOk
                    );
                    let input_info = input_info.assume_init();
                    assert_eq!(input_info.channelCount, expected_input_channels);

                    let mut output_info = MaybeUninit::<BusInfo>::zeroed();
                    assert_eq!(
                        component.getBusInfo(
                            MediaTypes_::kAudio as MediaType,
                            BusDirections_::kOutput as BusDirection,
                            0,
                            output_info.as_mut_ptr(),
                        ),
                        kResultOk
                    );
                    let output_info = output_info.assume_init();
                    assert_eq!(output_info.channelCount, expected_output_channels);

                    let mut input_route = RoutingInfo {
                        mediaType: MediaTypes_::kAudio as MediaType,
                        busIndex: 0,
                        channel: if expected_input_channels > 1 { 1 } else { 0 },
                    };
                    let mut output_route = RoutingInfo {
                        mediaType: MediaTypes_::kEvent as MediaType,
                        busIndex: 99,
                        channel: 99,
                    };
                    assert_eq!(
                        component.getRoutingInfo(&mut input_route, &mut output_route),
                        kResultOk
                    );
                    assert_eq!(output_route.mediaType, MediaTypes_::kAudio as MediaType);
                    assert_eq!(output_route.busIndex, 0);
                    assert_eq!(output_route.channel, -1);
                }

                let mut invalid_route = RoutingInfo {
                    mediaType: MediaTypes_::kEvent as MediaType,
                    busIndex: 0,
                    channel: 0,
                };
                let mut output_route = RoutingInfo {
                    mediaType: MediaTypes_::kAudio as MediaType,
                    busIndex: 0,
                    channel: 0,
                };
                assert_eq!(
                    component.getRoutingInfo(&mut invalid_route, &mut output_route),
                    kInvalidArgument
                );
                invalid_route = RoutingInfo {
                    mediaType: MediaTypes_::kAudio as MediaType,
                    busIndex: 0,
                    channel: 2,
                };
                assert_eq!(
                    component.getRoutingInfo(&mut invalid_route, &mut output_route),
                    kInvalidArgument
                );
                assert_eq!(
                    component.getRoutingInfo(ptr::null_mut(), &mut output_route),
                    kInvalidArgument
                );
                assert_eq!(
                    component.getRoutingInfo(&mut invalid_route, ptr::null_mut()),
                    kInvalidArgument
                );

                let mut stereo = SpeakerArr::kStereo;
                let mut mono = SpeakerArr::kMono;
                assert_eq!(
                    processor.setBusArrangements(&mut stereo, 1, &mut mono, 1),
                    kResultFalse
                );
                let mut surround = SpeakerArr::kStereoSurround;
                assert_eq!(
                    processor.setBusArrangements(&mut stereo, 1, &mut surround, 1),
                    kResultFalse
                );
            }
        }

        #[test]
        fn processor_exposes_optional_sidechain_input_bus() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    SidechainPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let component = processor_wrapper.to_com_ptr::<IComponent>().unwrap();

                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                    ),
                    2
                );
                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                    ),
                    1
                );

                let mut main_info = MaybeUninit::<BusInfo>::zeroed();
                assert_eq!(
                    component.getBusInfo(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                        main_info.as_mut_ptr(),
                    ),
                    kResultOk
                );
                let main_info = main_info.assume_init();
                assert_eq!(main_info.channelCount, 2);
                assert_eq!(main_info.busType, BusTypes_::kMain as BusType);
                let default_active = crate::bindings_impl::DEFAULT_ACTIVE_BUS_FLAG;
                assert_eq!(main_info.flags & default_active, default_active);

                let mut sidechain_info = MaybeUninit::<BusInfo>::zeroed();
                assert_eq!(
                    component.getBusInfo(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        1,
                        sidechain_info.as_mut_ptr(),
                    ),
                    kResultOk
                );
                let sidechain_info = sidechain_info.assume_init();
                assert_eq!(sidechain_info.channelCount, 2);
                assert_eq!(sidechain_info.busType, BusTypes_::kAux as BusType);
                assert_eq!(sidechain_info.flags & default_active, 0);
                let mut invalid_info = MaybeUninit::<BusInfo>::zeroed();
                assert_eq!(
                    component.getBusInfo(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        2,
                        invalid_info.as_mut_ptr(),
                    ),
                    kInvalidArgument
                );

                let mut input = [SpeakerArr::kStereo, SpeakerArr::kMono];
                let mut output = SpeakerArr::kStereo;
                assert_eq!(
                    processor.setBusArrangements(input.as_mut_ptr(), 2, &mut output, 1),
                    kResultTrue
                );
                let mut current_main = 0;
                let mut current_sidechain = 0;
                assert_eq!(
                    processor.getBusArrangement(
                        BusDirections_::kInput as BusDirection,
                        0,
                        &mut current_main,
                    ),
                    kResultOk
                );
                assert_eq!(
                    processor.getBusArrangement(
                        BusDirections_::kInput as BusDirection,
                        1,
                        &mut current_sidechain,
                    ),
                    kResultOk
                );
                assert_eq!(current_main, SpeakerArr::kStereo);
                assert_eq!(current_sidechain, SpeakerArr::kMono);

                let mut input = [SpeakerArr::kStereo, SpeakerArr::kStereoSurround];
                assert_eq!(
                    processor.setBusArrangements(input.as_mut_ptr(), 2, &mut output, 1),
                    kResultFalse
                );

                let mut route = RoutingInfo {
                    mediaType: MediaTypes_::kAudio as MediaType,
                    busIndex: 1,
                    channel: 0,
                };
                let mut output_route = RoutingInfo {
                    mediaType: MediaTypes_::kEvent as MediaType,
                    busIndex: 99,
                    channel: 99,
                };
                assert_eq!(
                    component.getRoutingInfo(&mut route, &mut output_route),
                    kResultOk
                );
                assert_eq!(output_route.mediaType, MediaTypes_::kAudio as MediaType);
                assert_eq!(output_route.busIndex, 0);
                assert_eq!(output_route.channel, -1);

                route.channel = 1;
                assert_eq!(
                    component.getRoutingInfo(&mut route, &mut output_route),
                    kInvalidArgument
                );
            }
        }

        #[test]
        fn processor_negotiates_instrument_event_input_and_stereo_output() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    InstrumentPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let component = processor_wrapper.to_com_ptr::<IComponent>().unwrap();

                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                    ),
                    0
                );
                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kEvent as MediaType,
                        BusDirections_::kInput as BusDirection,
                    ),
                    1
                );
                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                    ),
                    1
                );

                let mut output = SpeakerArr::kStereo;
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, &mut output, 1),
                    kResultTrue
                );
                let mut current_output = 0;
                assert_eq!(
                    processor.getBusArrangement(
                        BusDirections_::kOutput as BusDirection,
                        0,
                        &mut current_output,
                    ),
                    kResultOk
                );
                assert_eq!(current_output, SpeakerArr::kStereo);

                let mut output_info = MaybeUninit::<BusInfo>::zeroed();
                assert_eq!(
                    component.getBusInfo(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        0,
                        output_info.as_mut_ptr(),
                    ),
                    kResultOk
                );
                let output_info = output_info.assume_init();
                assert_eq!(output_info.channelCount, 2);

                let mut input_route = RoutingInfo {
                    mediaType: MediaTypes_::kEvent as MediaType,
                    busIndex: 0,
                    channel: -1,
                };
                let mut output_route = RoutingInfo {
                    mediaType: MediaTypes_::kEvent as MediaType,
                    busIndex: 99,
                    channel: 99,
                };
                assert_eq!(
                    component.getRoutingInfo(&mut input_route, &mut output_route),
                    kResultOk
                );
                assert_eq!(output_route.mediaType, MediaTypes_::kAudio as MediaType);
                assert_eq!(output_route.busIndex, 0);
                assert_eq!(output_route.channel, -1);

                input_route.channel = 15;
                assert_eq!(
                    component.getRoutingInfo(&mut input_route, &mut output_route),
                    kResultOk
                );
                input_route.channel = 16;
                assert_eq!(
                    component.getRoutingInfo(&mut input_route, &mut output_route),
                    kInvalidArgument
                );
                input_route = RoutingInfo {
                    mediaType: MediaTypes_::kAudio as MediaType,
                    busIndex: 0,
                    channel: 0,
                };
                assert_eq!(
                    component.getRoutingInfo(&mut input_route, &mut output_route),
                    kInvalidArgument
                );

                let mut mono_output = SpeakerArr::kMono;
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, &mut mono_output, 1),
                    kResultFalse
                );
                let mut input = SpeakerArr::kMono;
                let mut stereo_output = SpeakerArr::kStereo;
                assert_eq!(
                    processor.setBusArrangements(&mut input, 1, &mut stereo_output, 1),
                    kResultFalse
                );
            }
        }

        #[test]
        fn component_get_controller_class_id_rejects_null_and_writes_cid() {
            // SAFETY: Test code calls the VST3 component trait entrypoint directly with a null output pointer and a valid stack output pointer.
            unsafe {
                let processor =
                    crate::bindings_impl::VestyProcessor::<TestPlugin>::with_telemetry_registry(
                        std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                    );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::getControllerClassId(
                        &processor,
                        ptr::null_mut(),
                    ),
                    kInvalidArgument
                );

                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let mut cid = [0; 16];
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::getControllerClassId(
                        &processor,
                        &mut cid,
                    ),
                    kResultOk
                );
                assert_eq!(cid, tuid(metadata.controller_class_id));
            }
        }

        #[test]
        fn component_set_io_mode_validates_standard_modes_and_tracks_state() {
            // SAFETY: Test code calls the VST3 component trait entrypoint directly with primitive IO mode identifiers.
            unsafe {
                let processor =
                    crate::bindings_impl::VestyProcessor::<TestPlugin>::with_telemetry_registry(
                        std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                    );
                assert_eq!(processor.io_mode_for_test(), IoModes_::kSimple as IoMode);

                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::setIoMode(
                        &processor,
                        IoModes_::kAdvanced as IoMode,
                    ),
                    kResultOk
                );
                assert_eq!(processor.io_mode_for_test(), IoModes_::kAdvanced as IoMode);

                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::setIoMode(
                        &processor,
                        IoModes_::kOfflineProcessing as IoMode,
                    ),
                    kResultOk
                );
                assert_eq!(
                    processor.io_mode_for_test(),
                    IoModes_::kOfflineProcessing as IoMode
                );

                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::setIoMode(
                        &processor,
                        99,
                    ),
                    kInvalidArgument
                );
                assert_eq!(
                    processor.io_mode_for_test(),
                    IoModes_::kOfflineProcessing as IoMode
                );

                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::setIoMode(
                        &processor,
                        IoModes_::kSimple as IoMode,
                    ),
                    kResultOk
                );
                assert_eq!(processor.io_mode_for_test(), IoModes_::kSimple as IoMode);
            }
        }

        #[test]
        fn component_activate_bus_validates_declared_buses_and_tracks_state() {
            // SAFETY: Test code calls the VST3 component trait entrypoint directly with primitive bus identifiers.
            unsafe {
                let effect =
                    crate::bindings_impl::VestyProcessor::<TestPlugin>::with_telemetry_registry(
                        std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                    );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::activateBus(
                        &effect,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                        0,
                    ),
                    kResultOk
                );
                assert_eq!(
                    effect.bus_active_for_test(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                    ),
                    Some(false)
                );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::activateBus(
                        &effect,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        0,
                        1,
                    ),
                    kResultOk
                );
                assert_eq!(
                    effect.bus_active_for_test(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        0,
                    ),
                    Some(true)
                );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::activateBus(
                        &effect,
                        MediaTypes_::kEvent as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                        1,
                    ),
                    kInvalidArgument
                );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<TestPlugin> as IComponentTrait>::activateBus(
                        &effect,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        1,
                        1,
                    ),
                    kInvalidArgument
                );

                let sidechain =
                    crate::bindings_impl::VestyProcessor::<SidechainPlugin>::with_telemetry_registry(
                        std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                    );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<SidechainPlugin> as IComponentTrait>::activateBus(
                        &sidechain,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        1,
                        1,
                    ),
                    kResultOk
                );
                assert_eq!(
                    sidechain.bus_active_for_test(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        1,
                    ),
                    Some(true)
                );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<SidechainPlugin> as IComponentTrait>::activateBus(
                        &sidechain,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        2,
                        1,
                    ),
                    kInvalidArgument
                );

                let instrument =
                    crate::bindings_impl::VestyProcessor::<InstrumentPlugin>::with_telemetry_registry(
                        std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                    );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<InstrumentPlugin> as IComponentTrait>::activateBus(
                        &instrument,
                        MediaTypes_::kEvent as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                        0,
                    ),
                    kResultOk
                );
                assert_eq!(
                    instrument.bus_active_for_test(
                        MediaTypes_::kEvent as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                    ),
                    Some(false)
                );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<InstrumentPlugin> as IComponentTrait>::activateBus(
                        &instrument,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                        0,
                        1,
                    ),
                    kInvalidArgument
                );

                let multi_output = crate::bindings_impl::VestyProcessor::<
                    MultiOutputInstrumentPlugin,
                >::with_telemetry_registry(std::sync::Arc::new(
                    crate::bindings_impl::Vst3TelemetryRegistry::default(),
                ));
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<MultiOutputInstrumentPlugin> as IComponentTrait>::activateBus(
                        &multi_output,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        1,
                        1,
                    ),
                    kResultOk
                );
                assert_eq!(
                    multi_output.bus_active_for_test(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        1,
                    ),
                    Some(true)
                );
                assert_eq!(
                    <crate::bindings_impl::VestyProcessor<MultiOutputInstrumentPlugin> as IComponentTrait>::activateBus(
                        &multi_output,
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        2,
                        1,
                    ),
                    kInvalidArgument
                );
            }
        }

        #[test]
        fn processor_supports_multi_output_instrument_buses_and_sample32_process() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    MultiOutputInstrumentPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let component = processor_wrapper.to_com_ptr::<IComponent>().unwrap();

                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kInput as BusDirection,
                    ),
                    0
                );
                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kEvent as MediaType,
                        BusDirections_::kInput as BusDirection,
                    ),
                    1
                );
                assert_eq!(
                    component.getBusCount(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                    ),
                    2
                );

                let mut main_info = MaybeUninit::<BusInfo>::zeroed();
                assert_eq!(
                    component.getBusInfo(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        0,
                        main_info.as_mut_ptr(),
                    ),
                    kResultOk
                );
                let main_info = main_info.assume_init();
                assert_eq!(main_info.channelCount, 2);
                assert_eq!(main_info.busType, BusTypes_::kMain as BusType);
                let default_active = crate::bindings_impl::DEFAULT_ACTIVE_BUS_FLAG;
                assert_eq!(main_info.flags & default_active, default_active);
                assert_eq!(string128_to_string(&main_info.name), "Main");

                let mut aux_info = MaybeUninit::<BusInfo>::zeroed();
                assert_eq!(
                    component.getBusInfo(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        1,
                        aux_info.as_mut_ptr(),
                    ),
                    kResultOk
                );
                let aux_info = aux_info.assume_init();
                assert_eq!(aux_info.channelCount, 2);
                assert_eq!(aux_info.busType, BusTypes_::kAux as BusType);
                assert_eq!(aux_info.flags & default_active, 0);
                assert_eq!(string128_to_string(&aux_info.name), "Aux 1");

                let mut invalid_info = MaybeUninit::<BusInfo>::zeroed();
                assert_eq!(
                    component.getBusInfo(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        2,
                        invalid_info.as_mut_ptr(),
                    ),
                    kInvalidArgument
                );

                let mut outputs = [SpeakerArr::kStereo, SpeakerArr::kStereo];
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, outputs.as_mut_ptr(), 2),
                    kResultTrue
                );
                let mut current_main = 0;
                let mut current_aux = 0;
                assert_eq!(
                    processor.getBusArrangement(
                        BusDirections_::kOutput as BusDirection,
                        0,
                        &mut current_main,
                    ),
                    kResultOk
                );
                assert_eq!(
                    processor.getBusArrangement(
                        BusDirections_::kOutput as BusDirection,
                        1,
                        &mut current_aux,
                    ),
                    kResultOk
                );
                assert_eq!(current_main, SpeakerArr::kStereo);
                assert_eq!(current_aux, SpeakerArr::kStereo);

                let mut missing_aux = [SpeakerArr::kStereo];
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, missing_aux.as_mut_ptr(), 1),
                    kInvalidArgument
                );

                let mut mono_aux = [SpeakerArr::kStereo, SpeakerArr::kMono];
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, mono_aux.as_mut_ptr(), 2),
                    kResultFalse
                );
                let mut input = SpeakerArr::kStereo;
                assert_eq!(
                    processor.setBusArrangements(&mut input, 1, outputs.as_mut_ptr(), 2),
                    kResultFalse
                );

                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    MultiOutputInstrumentPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<MultiOutputInstrumentPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IUnitInfo_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let unit_info =
                    ComPtr::<IUnitInfo>::from_raw(controller as *mut IUnitInfo).expect("unit info");
                let mut unit_id = -99;
                assert_eq!(
                    unit_info.getUnitByBus(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        1,
                        -1,
                        &mut unit_id,
                    ),
                    kResultOk
                );
                assert_eq!(unit_id, kRootUnitId);
                assert_eq!(
                    unit_info.getUnitByBus(
                        MediaTypes_::kAudio as MediaType,
                        BusDirections_::kOutput as BusDirection,
                        2,
                        -1,
                        &mut unit_id,
                    ),
                    kInvalidArgument
                );

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut main_l = [0.0_f32; 4];
                let mut main_r = [0.0_f32; 4];
                let mut aux_l = [0.0_f32; 4];
                let mut aux_r = [0.0_f32; 4];
                let mut main_channels = [main_l.as_mut_ptr(), main_r.as_mut_ptr()];
                let mut aux_channels = [aux_l.as_mut_ptr(), aux_r.as_mut_ptr()];
                let mut output_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: aux_channels.as_mut_ptr(),
                        },
                    },
                ];
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 2,
                    inputs: ptr::null_mut(),
                    outputs: output_buses.as_mut_ptr(),
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(main_l, [0.10; 4]);
                assert_eq!(main_r, [0.20; 4]);
                assert_eq!(aux_l, [0.30; 4]);
                assert_eq!(aux_r, [0.40; 4]);
                assert_eq!(output_buses[0].silenceFlags, 0);
                assert_eq!(output_buses[1].silenceFlags, 0);
            }
        }

        #[test]
        fn processor_runs_multi_output_instrument_when_aux_output_is_not_provided() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    MultiOutputInstrumentPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let mut outputs = [SpeakerArr::kStereo, SpeakerArr::kStereo];
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, outputs.as_mut_ptr(), 2),
                    kResultTrue
                );

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut main_l = [0.0_f32; 4];
                let mut main_r = [0.0_f32; 4];
                let mut main_channels = [main_l.as_mut_ptr(), main_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: main_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(main_l, [0.10; 4]);
                assert_eq!(main_r, [0.20; 4]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_runs_multi_output_instrument_with_empty_inactive_aux_output() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    MultiOutputInstrumentPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let mut outputs = [SpeakerArr::kStereo, SpeakerArr::kStereo];
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, outputs.as_mut_ptr(), 2),
                    kResultTrue
                );

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut main_l = [0.0_f32; 4];
                let mut main_r = [0.0_f32; 4];
                let mut main_channels = [main_l.as_mut_ptr(), main_r.as_mut_ptr()];
                let mut output_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 0,
                        silenceFlags: u64::MAX,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: ptr::null_mut(),
                        },
                    },
                ];
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 2,
                    inputs: ptr::null_mut(),
                    outputs: output_buses.as_mut_ptr(),
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(main_l, [0.10; 4]);
                assert_eq!(main_r, [0.20; 4]);
                assert_eq!(output_buses[0].silenceFlags, 0);
                assert_eq!(output_buses[1].silenceFlags, 0);
            }
        }

        #[test]
        fn processor_routes_multi_output_instrument_through_sample64_scratch_fallback() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    MultiOutputInstrumentPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let mut outputs = [SpeakerArr::kStereo, SpeakerArr::kStereo];
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, outputs.as_mut_ptr(), 2),
                    kResultTrue
                );

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut main_l = [9.0_f64; 4];
                let mut main_r = [9.0_f64; 4];
                let mut aux_l = [9.0_f64; 4];
                let mut aux_r = [9.0_f64; 4];
                let mut main_channels = [main_l.as_mut_ptr(), main_r.as_mut_ptr()];
                let mut aux_channels = [aux_l.as_mut_ptr(), aux_r.as_mut_ptr()];
                let mut output_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: aux_channels.as_mut_ptr(),
                        },
                    },
                ];
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 2,
                    inputs: ptr::null_mut(),
                    outputs: output_buses.as_mut_ptr(),
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(main_l, [f64::from(0.10_f32); 4]);
                assert_eq!(main_r, [f64::from(0.20_f32); 4]);
                assert_eq!(aux_l, [f64::from(0.30_f32); 4]);
                assert_eq!(aux_r, [f64::from(0.40_f32); 4]);
                assert_eq!(output_buses[0].silenceFlags, 0);
                assert_eq!(output_buses[1].silenceFlags, 0);
            }
        }

        #[test]
        fn processor_routes_multi_output_instrument_through_native_sample64_process() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let processor_wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                    MultiOutputNativeF64InstrumentPlugin,
                >::with_telemetry_registry(
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
                ));
                let processor = processor_wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
                let mut outputs = [SpeakerArr::kStereo, SpeakerArr::kStereo];
                assert_eq!(
                    processor.setBusArrangements(ptr::null_mut(), 0, outputs.as_mut_ptr(), 2),
                    kResultTrue
                );

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut main_l = [0.0_f64; 4];
                let mut main_r = [0.0_f64; 4];
                let mut aux_l = [0.0_f64; 4];
                let mut aux_r = [0.0_f64; 4];
                let mut main_channels = [main_l.as_mut_ptr(), main_r.as_mut_ptr()];
                let mut aux_channels = [aux_l.as_mut_ptr(), aux_r.as_mut_ptr()];
                let mut output_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: aux_channels.as_mut_ptr(),
                        },
                    },
                ];
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 2,
                    inputs: ptr::null_mut(),
                    outputs: output_buses.as_mut_ptr(),
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                MULTI_OUTPUT_NATIVE_F64_F32_ENTERED.store(false, TestOrdering::Relaxed);
                MULTI_OUTPUT_NATIVE_F64_ENTERED.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert!(!MULTI_OUTPUT_NATIVE_F64_F32_ENTERED.load(TestOrdering::Relaxed));
                assert!(MULTI_OUTPUT_NATIVE_F64_ENTERED.load(TestOrdering::Relaxed));
                assert_eq!(main_l, [1.10; 4]);
                assert_eq!(main_r, [1.20; 4]);
                assert_eq!(aux_l, [1.30; 4]);
                assert_eq!(aux_r, [1.40; 4]);
                assert_eq!(output_buses[0].silenceFlags, 0);
                assert_eq!(output_buses[1].silenceFlags, 0);
            }
        }

        #[test]
        fn controller_parameter_callbacks_reject_null_outputs() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid host pointers.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                let mode_id = controller_param_id(&controller, 1);

                assert_eq!(
                    controller.getParameterInfo(0, ptr::null_mut()),
                    kInvalidArgument
                );
                let mut info = MaybeUninit::<ParameterInfo>::zeroed();
                assert_eq!(
                    controller.getParameterInfo(-1, info.as_mut_ptr()),
                    kInvalidArgument
                );
                assert_eq!(
                    controller.getParamStringByValue(mode_id, 0.5, ptr::null_mut()),
                    kInvalidArgument
                );

                let mut input: Vec<u16> =
                    "Drive".encode_utf16().chain(std::iter::once(0)).collect();
                let mut normalized = 0.25;
                assert_eq!(
                    controller.getParamValueByString(mode_id, ptr::null_mut(), &mut normalized),
                    kInvalidArgument
                );
                assert_eq!(normalized, 0.25);
                assert_eq!(
                    controller.getParamValueByString(mode_id, input.as_mut_ptr(), ptr::null_mut(),),
                    kInvalidArgument
                );
            }
        }

        #[test]
        fn controller_formats_and_parses_choice_params() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                let mode_id = controller_param_id(&controller, 1);

                let mut text = [0_u16; 128];
                assert_eq!(
                    controller.getParamStringByValue(mode_id, 0.5, &mut text),
                    kResultOk
                );
                let len = text
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(text.len());
                assert_eq!(String::from_utf16(&text[..len]).unwrap(), "Drive");

                let mut input: Vec<u16> = "Fuzz".encode_utf16().chain(std::iter::once(0)).collect();
                let mut normalized = 0.0;
                assert_eq!(
                    controller.getParamValueByString(mode_id, input.as_mut_ptr(), &mut normalized),
                    kResultOk
                );
                assert_eq!(normalized, 1.0);

                let mut bounded_input = [b' ' as u16; 128];
                for (index, unit) in "Fuzz".encode_utf16().enumerate() {
                    bounded_input[index] = unit;
                }
                normalized = 0.0;
                assert_eq!(
                    controller.getParamValueByString(
                        mode_id,
                        bounded_input.as_mut_ptr(),
                        &mut normalized,
                    ),
                    kResultOk
                );
                assert_eq!(normalized, 1.0);
            }
        }

        #[test]
        fn processor_reports_latency_and_tail_samples() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");
                assert_eq!(processor.getLatencySamples(), 128);
                assert_eq!(processor.getTailSamples(), 4096);
            }
        }

        #[test]
        fn processor_and_controller_support_connection_points() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();

                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IConnectionPoint_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IConnectionPoint>::from_raw(processor as *mut IConnectionPoint)
                        .expect("processor connection point");

                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IConnectionPoint_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IConnectionPoint>::from_raw(controller as *mut IConnectionPoint)
                        .expect("controller connection point");

                assert_eq!(processor.connect(controller.as_ptr()), kResultOk);
                assert_eq!(controller.connect(processor.as_ptr()), kResultOk);
                assert_eq!(processor.notify(ptr::null_mut()), kResultOk);
                assert_eq!(controller.notify(ptr::null_mut()), kResultOk);
                assert_eq!(processor.disconnect(controller.as_ptr()), kResultOk);
                assert_eq!(controller.disconnect(processor.as_ptr()), kResultOk);
                assert_eq!(processor.disconnect(controller.as_ptr()), kResultFalse);
                assert_eq!(processor.connect(ptr::null_mut()), kInvalidArgument);
            }
        }

        #[test]
        fn component_and_controller_state_roundtrip_through_ibstream() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();

                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                let gain_id = controller_param_id(&controller, 0);
                assert_eq!(controller.setParamNormalized(gain_id, 0.25), kResultOk);

                let controller_state = ComWrapper::new(MemoryStream::default());
                let controller_state_ptr = controller_state.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    controller.getState(controller_state_ptr.as_ptr()),
                    kResultOk
                );
                assert!(!controller_state.bytes().is_empty());

                let processor_cid = tuid(metadata.processor_class_id);
                let mut component: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IComponent_iid.as_ptr(),
                        &mut component,
                    ),
                    kResultOk
                );
                let component = ComPtr::<IComponent>::from_raw(component as *mut IComponent)
                    .expect("component");
                let component_input = ComWrapper::new(MemoryStream::with_bytes(raw_state_bytes(
                    serde_json::json!({
                        "version": 1,
                        "params": [{ "id": "gain", "normalized": 0.25 }],
                        "custom": { "label": "from-stream" }
                    }),
                )));
                let component_input_ptr = component_input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(component.setState(component_input_ptr.as_ptr()), kResultOk);

                let component_state = ComWrapper::new(MemoryStream::default());
                let component_state_ptr = component_state.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(component.getState(component_state_ptr.as_ptr()), kResultOk);
                let component_state_bytes = component_state.bytes();
                let component_state_text = String::from_utf8_lossy(&component_state_bytes);
                assert!(component_state_text.contains(r#""custom""#));
                assert!(component_state_text.contains("from-stream"));

                let mut restored: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut restored,
                    ),
                    kResultOk
                );
                let restored =
                    ComPtr::<IEditController>::from_raw(restored as *mut IEditController)
                        .expect("restored controller");
                let restored_input =
                    ComWrapper::new(MemoryStream::with_bytes(component_state_bytes));
                let restored_input_ptr = restored_input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    restored.setComponentState(restored_input_ptr.as_ptr()),
                    kResultOk
                );
                let restored_gain_id = controller_param_id(&restored, 0);
                assert_eq!(restored.getParamNormalized(restored_gain_id), 0.25);

                let mut restored_controller_state: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut restored_controller_state,
                    ),
                    kResultOk
                );
                let restored_controller_state = ComPtr::<IEditController>::from_raw(
                    restored_controller_state as *mut IEditController,
                )
                .expect("controller state restore");
                let controller_input =
                    ComWrapper::new(MemoryStream::with_bytes(controller_state.bytes()));
                let controller_input_ptr = controller_input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    restored_controller_state.setState(controller_input_ptr.as_ptr()),
                    kResultOk
                );
                let restored_controller_gain_id =
                    controller_param_id(&restored_controller_state, 0);
                assert_eq!(
                    restored_controller_state.getParamNormalized(restored_controller_gain_id),
                    0.25
                );
            }
        }

        #[test]
        fn component_rejects_unsupported_state_version() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut component: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IComponent_iid.as_ptr(),
                        &mut component,
                    ),
                    kResultOk
                );
                let component = ComPtr::<IComponent>::from_raw(component as *mut IComponent)
                    .expect("component");
                let component_input = ComWrapper::new(MemoryStream::with_bytes(raw_state_bytes(
                    serde_json::json!({
                        "version": 2,
                        "params": [{ "id": "gain", "normalized": 0.25 }],
                    }),
                )));
                let component_input_ptr = component_input.to_com_ptr::<IBStream>().unwrap();

                assert_eq!(
                    component.setState(component_input_ptr.as_ptr()),
                    kInvalidArgument
                );
            }
        }

        #[test]
        fn controller_rejects_invalid_custom_state_without_param_partial_restore() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                let gain_id = controller_param_id(&controller, 0);
                assert_eq!(controller.getParamNormalized(gain_id), 0.5);

                let state = ComWrapper::new(MemoryStream::with_bytes(raw_state_bytes(
                    serde_json::json!({
                        "version": 1,
                        "params": [{ "id": "gain", "normalized": 0.25 }],
                        "custom": { "missing": "label" },
                    }),
                )));
                let state_ptr = state.to_com_ptr::<IBStream>().unwrap();

                assert_eq!(controller.setState(state_ptr.as_ptr()), kResultFalse);
                assert_eq!(controller.getParamNormalized(gain_id), 0.5);
            }
        }

        #[test]
        fn processor_translates_automation_midi_and_transport() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                *CAPTURED_PROCESS.lock().unwrap() = CapturedProcess::default();

                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<CapturePlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<CapturePlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 8,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let gain_id = test_param_id("gain");
                let queue =
                    ComWrapper::new(FakeParamValueQueue::new(gain_id, vec![(2, 0.2), (6, 0.8)]));
                let changes = ComWrapper::new(FakeParameterChanges {
                    queues: vec![queue.to_com_ptr::<IParamValueQueue>().unwrap()],
                });
                let changes_ptr = changes.to_com_ptr::<IParameterChanges>().unwrap();

                let note_on = Event {
                    busIndex: 0,
                    sampleOffset: 3,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteOnEvent as u16,
                    __field0: Event__type0 {
                        noteOn: NoteOnEvent {
                            channel: 1,
                            pitch: 64,
                            tuning: 0.0,
                            velocity: 0.7,
                            length: 0,
                            noteId: 42,
                        },
                    },
                };
                let note_off = Event {
                    busIndex: 0,
                    sampleOffset: 7,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteOffEvent as u16,
                    __field0: Event__type0 {
                        noteOff: NoteOffEvent {
                            channel: 1,
                            pitch: 64,
                            velocity: 0.1,
                            noteId: 42,
                            tuning: 0.0,
                        },
                    },
                };
                let poly_pressure = Event {
                    busIndex: 0,
                    sampleOffset: 4,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kPolyPressureEvent as u16,
                    __field0: Event__type0 {
                        polyPressure: PolyPressureEvent {
                            channel: 1,
                            pitch: 64,
                            pressure: 0.8,
                            noteId: 42,
                        },
                    },
                };
                let note_expression = Event {
                    busIndex: 0,
                    sampleOffset: 5,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteExpressionValueEvent as u16,
                    __field0: Event__type0 {
                        noteExpressionValue: NoteExpressionValueEvent {
                            typeId: NoteExpressionTypeIDs_::kBrightnessTypeID,
                            noteId: 42,
                            value: 0.625,
                        },
                    },
                };
                let note_expression_int = Event {
                    busIndex: 0,
                    sampleOffset: 5,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteExpressionIntValueEvent as u16,
                    __field0: Event__type0 {
                        noteExpressionIntValue: NoteExpressionIntValueEvent {
                            typeId: NoteExpressionTypeIDs_::kCustomStart,
                            noteId: 42,
                            value: 123,
                        },
                    },
                };
                let note_expression_text_value = wide_cstring("ah");
                let note_expression_text = Event {
                    busIndex: 0,
                    sampleOffset: 5,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteExpressionTextEvent as u16,
                    __field0: Event__type0 {
                        noteExpressionText: NoteExpressionTextEvent {
                            typeId: NoteExpressionTypeIDs_::kTextTypeID,
                            noteId: 42,
                            textLen: 2,
                            text: note_expression_text_value.as_ptr(),
                        },
                    },
                };
                let sysex_bytes = [0xF0_u8, 0x7D, 0x01, 0xF7];
                let sysex = Event {
                    busIndex: 0,
                    sampleOffset: 5,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kDataEvent as u16,
                    __field0: Event__type0 {
                        data: DataEvent {
                            size: sysex_bytes.len() as uint32,
                            r#type: DataEvent_::DataTypes_::kMidiSysEx as uint32,
                            bytes: sysex_bytes.as_ptr(),
                        },
                    },
                };
                let mod_wheel = Event {
                    busIndex: 0,
                    sampleOffset: 5,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kLegacyMIDICCOutEvent as u16,
                    __field0: Event__type0 {
                        midiCCOut: LegacyMIDICCOutEvent {
                            controlNumber: ControllerNumbers_::kCtrlModWheel as u8,
                            channel: 1,
                            value: 64,
                            value2: 0,
                        },
                    },
                };
                let pitch_bend = Event {
                    busIndex: 0,
                    sampleOffset: 1,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kLegacyMIDICCOutEvent as u16,
                    __field0: Event__type0 {
                        midiCCOut: LegacyMIDICCOutEvent {
                            controlNumber: ControllerNumbers_::kPitchBend as u8,
                            channel: 1,
                            value: 0,
                            value2: 64,
                        },
                    },
                };
                let channel_pressure = Event {
                    busIndex: 0,
                    sampleOffset: 6,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kLegacyMIDICCOutEvent as u16,
                    __field0: Event__type0 {
                        midiCCOut: LegacyMIDICCOutEvent {
                            controlNumber: ControllerNumbers_::kAfterTouch as u8,
                            channel: 1,
                            value: 96,
                            value2: 0,
                        },
                    },
                };
                let events = ComWrapper::new(FakeEventList::new(vec![
                    note_on,
                    note_off,
                    poly_pressure,
                    note_expression,
                    note_expression_int,
                    note_expression_text,
                    sysex,
                    mod_wheel,
                    pitch_bend,
                    channel_pressure,
                ]));
                let events_ptr = events.to_com_ptr::<IEventList>().unwrap();

                let input_l = [0.0_f32; 8];
                let input_r = [0.0_f32; 8];
                let mut output_l = [1.0_f32; 8];
                let mut output_r = [1.0_f32; 8];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample32,
                    input_r.as_ptr() as *mut Sample32,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };

                let mut context =
                    MaybeUninit::<vst3::Steinberg::Vst::ProcessContext>::zeroed().assume_init();
                context.state = crate::bindings_impl::PROCESS_CONTEXT_PLAYING_FLAG
                    | crate::bindings_impl::PROCESS_CONTEXT_TEMPO_VALID_FLAG;
                context.tempo = 132.5;
                context.projectTimeSamples = 2048;

                let mut data = ProcessData {
                    processMode: ProcessModes_::kOffline as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 8,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: changes_ptr.as_ptr(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: events_ptr.as_ptr(),
                    outputEvents: ptr::null_mut(),
                    processContext: &mut context,
                };

                assert_eq!(processor.process(&mut data), kResultOk);

                let captured = CAPTURED_PROCESS.lock().unwrap().clone();
                assert_eq!(
                    captured.events,
                    vec![
                        CoreEvent::PitchBend {
                            sample_offset: 1,
                            channel: 1,
                            value: 0.0,
                        },
                        CoreEvent::Param {
                            sample_offset: 2,
                            handle: vesty_params::ParamHandle::from_index(0),
                            id_hash: gain_id,
                            normalized: 0.2,
                        },
                        CoreEvent::NoteOn {
                            sample_offset: 3,
                            channel: 1,
                            key: 64,
                            velocity: 0.7,
                            note_id: 42,
                        },
                        CoreEvent::PolyPressure {
                            sample_offset: 4,
                            channel: 1,
                            key: 64,
                            pressure: 0.8,
                            note_id: 42,
                        },
                        CoreEvent::NoteExpressionValue {
                            sample_offset: 5,
                            type_id: vesty_core::note_expression::BRIGHTNESS,
                            note_id: 42,
                            value: 0.625,
                        },
                        CoreEvent::NoteExpressionInt {
                            sample_offset: 5,
                            type_id: vesty_core::note_expression::CUSTOM_START,
                            note_id: 42,
                            value: 123,
                        },
                        CoreEvent::NoteExpressionText {
                            sample_offset: 5,
                            type_id: vesty_core::note_expression::TEXT,
                            note_id: 42,
                            text_len: 2,
                            text: {
                                let mut text = [0; vesty_core::MAX_NOTE_EXPRESSION_TEXT_UNITS];
                                text[0] = 'a' as u16;
                                text[1] = 'h' as u16;
                                text
                            },
                        },
                        CoreEvent::SysEx {
                            sample_offset: 5,
                            data_len: 4,
                            data: {
                                let mut data = [0; vesty_core::MAX_SYSEX_BYTES];
                                data[..4].copy_from_slice(&[0xF0, 0x7D, 0x01, 0xF7]);
                                data
                            },
                            truncated: false,
                        },
                        CoreEvent::MidiCc {
                            sample_offset: 5,
                            channel: 1,
                            controller: ControllerNumbers_::kCtrlModWheel as u16,
                            value: 64.0 / 127.0,
                        },
                        CoreEvent::Param {
                            sample_offset: 6,
                            handle: vesty_params::ParamHandle::from_index(0),
                            id_hash: gain_id,
                            normalized: 0.8,
                        },
                        CoreEvent::ChannelPressure {
                            sample_offset: 6,
                            channel: 1,
                            pressure: 96.0 / 127.0,
                        },
                        CoreEvent::NoteOff {
                            sample_offset: 7,
                            channel: 1,
                            key: 64,
                            velocity: 0.1,
                            note_id: 42,
                        },
                    ]
                );
                let param_value = captured.param_value.expect("captured gain value");
                assert!((param_value - 0.5).abs() < 0.000_001);
                assert!(captured.no_alloc_active);
                assert_eq!(output_l, [0.5, 0.5, 0.2, 0.2, 0.2, 0.2, 0.8, 0.8]);
                assert_eq!(output_r, output_l);
                assert_eq!(
                    captured.transport,
                    Transport {
                        playing: true,
                        tempo_bpm: Some(132.5),
                        position_samples: Some(2048),
                    }
                );
                assert_eq!(captured.process_mode, ProcessMode::Offline);
            }
        }

        #[test]
        fn processor_treats_program_change_automation_as_realtime_safe_param_event() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                *PROGRAM_AUTOMATION_CAPTURE.lock().unwrap() = ProgramAutomationCapture::default();
                PROGRAM_AUTOMATION_APPLY_CALLS.store(0, TestOrdering::Relaxed);
                PROGRAM_AUTOMATION_LOAD_CALLS.store(0, TestOrdering::Relaxed);

                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    ProgramAutomationPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<ProgramAutomationPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 8,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let program_id = test_param_id("program");
                let queue = ComWrapper::new(FakeParamValueQueue::new(
                    program_id,
                    vec![(1, 0.5), (5, 1.0)],
                ));
                let changes = ComWrapper::new(FakeParameterChanges {
                    queues: vec![queue.to_com_ptr::<IParamValueQueue>().unwrap()],
                });
                let changes_ptr = changes.to_com_ptr::<IParameterChanges>().unwrap();

                let input_l = [0.0_f32; 8];
                let input_r = [0.0_f32; 8];
                let mut output_l = [1.0_f32; 8];
                let mut output_r = [1.0_f32; 8];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample32,
                    input_r.as_ptr() as *mut Sample32,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 8,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: changes_ptr.as_ptr(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);

                let captured = PROGRAM_AUTOMATION_CAPTURE.lock().unwrap().clone();
                assert_eq!(
                    captured.events,
                    vec![
                        CoreEvent::Param {
                            sample_offset: 1,
                            handle: vesty_params::ParamHandle::from_index(0),
                            id_hash: program_id,
                            normalized: 0.5,
                        },
                        CoreEvent::Param {
                            sample_offset: 5,
                            handle: vesty_params::ParamHandle::from_index(0),
                            id_hash: program_id,
                            normalized: 1.0,
                        },
                    ]
                );
                let param_value = captured.param_value.expect("captured program value");
                assert!((param_value - 0.0).abs() < 0.000_001);
                assert!(captured.no_alloc_active);
                assert_eq!(
                    PROGRAM_AUTOMATION_APPLY_CALLS.load(TestOrdering::Relaxed),
                    0
                );
                assert_eq!(PROGRAM_AUTOMATION_LOAD_CALLS.load(TestOrdering::Relaxed), 0);
            }
        }

        #[test]
        fn processor_drives_reset_suspend_and_resume_lifecycle() {
            let _lock = PREPARE_MATRIX_TEST_LOCK.lock().unwrap();
            PREPARE_MATRIX_KERNEL_CREATIONS.store(0, TestOrdering::Relaxed);
            PREPARE_MATRIX_RESETS.store(0, TestOrdering::Relaxed);
            PREPARE_MATRIX_SUSPENDS.store(0, TestOrdering::Relaxed);
            PREPARE_MATRIX_RESUMES.store(0, TestOrdering::Relaxed);

            let wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                PrepareMatrixPlugin,
            >::with_telemetry_registry(
                std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
            ));
            let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
            let component = wrapper.to_com_ptr::<IComponent>().unwrap();
            let mut setup = ProcessSetup {
                processMode: ProcessModes_::kRealtime as int32,
                symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                maxSamplesPerBlock: 64,
                sampleRate: 48_000.0,
            };

            // SAFETY: Test code invokes lifecycle callbacks on a locally owned COM wrapper.
            unsafe {
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);
                assert_eq!(processor.setProcessing(0), kResultOk);
                assert_eq!(processor.setProcessing(1), kResultOk);
                assert_eq!(component.setActive(0), kResultOk);
                assert_eq!(component.setActive(1), kResultOk);
            }

            assert_eq!(
                PREPARE_MATRIX_KERNEL_CREATIONS.load(TestOrdering::Relaxed),
                2
            );
            assert_eq!(PREPARE_MATRIX_RESETS.load(TestOrdering::Relaxed), 3);
            assert_eq!(PREPARE_MATRIX_SUSPENDS.load(TestOrdering::Relaxed), 1);
            assert_eq!(PREPARE_MATRIX_RESUMES.load(TestOrdering::Relaxed), 1);
        }

        #[test]
        fn processor_prepare_tracks_sample_rate_and_block_size_matrix() {
            let _lock = PREPARE_MATRIX_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let cases = [
                    (32_usize, 44_100.0),
                    (64_usize, 48_000.0),
                    (128_usize, 96_000.0),
                    (1024_usize, 192_000.0),
                ];
                {
                    let mut records = PREPARE_MATRIX_RECORDS.lock().unwrap();
                    records.clear();
                    records.reserve(cases.len());
                }
                PREPARE_MATRIX_KERNEL_CREATIONS.store(0, TestOrdering::Relaxed);

                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    PrepareMatrixPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<PrepareMatrixPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                for (frames, sample_rate) in cases {
                    let mut setup = ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        maxSamplesPerBlock: frames as int32,
                        sampleRate: sample_rate,
                    };
                    assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                    let input_l = vec![0.0_f32; frames];
                    let input_r = vec![0.0_f32; frames];
                    let mut output_l = vec![0.0_f32; frames];
                    let mut output_r = vec![0.0_f32; frames];
                    let mut input_channels = [
                        input_l.as_ptr() as *mut Sample32,
                        input_r.as_ptr() as *mut Sample32,
                    ];
                    let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                    let mut input_bus = AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: input_channels.as_mut_ptr(),
                        },
                    };
                    let mut output_bus = AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: output_channels.as_mut_ptr(),
                        },
                    };
                    let mut data = ProcessData {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        numSamples: frames as int32,
                        numInputs: 1,
                        numOutputs: 1,
                        inputs: &mut input_bus,
                        outputs: &mut output_bus,
                        inputParameterChanges: ptr::null_mut(),
                        outputParameterChanges: ptr::null_mut(),
                        inputEvents: ptr::null_mut(),
                        outputEvents: ptr::null_mut(),
                        processContext: ptr::null_mut(),
                    };

                    assert_eq!(processor.process(&mut data), kResultOk);
                }

                let records = PREPARE_MATRIX_RECORDS.lock().unwrap().clone();
                assert_eq!(records.len(), cases.len());
                assert_eq!(
                    PREPARE_MATRIX_KERNEL_CREATIONS.load(TestOrdering::Relaxed),
                    1
                );
                for (record, (frames, sample_rate)) in records.iter().zip(cases) {
                    assert_eq!(record.init_sample_rate, cases[0].1);
                    assert_eq!(record.init_max_block_size, cases[0].0);
                    assert_eq!(record.prepare_sample_rate, sample_rate);
                    assert_eq!(record.prepare_max_block_size, frames);
                    assert_eq!(record.process_frames, frames);
                    assert!(record.no_alloc_active);
                }
            }
        }

        #[test]
        fn processor_setup_processing_rejects_invalid_host_setup_without_creating_kernel() {
            let _lock = PREPARE_MATRIX_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                PREPARE_MATRIX_RECORDS.lock().unwrap().clear();
                PREPARE_MATRIX_KERNEL_CREATIONS.store(0, TestOrdering::Relaxed);

                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    PrepareMatrixPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<PrepareMatrixPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                assert_eq!(processor.setupProcessing(ptr::null_mut()), kInvalidArgument);

                let mut invalid_setups = [
                    ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: 999,
                        maxSamplesPerBlock: 64,
                        sampleRate: 48_000.0,
                    },
                    ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        maxSamplesPerBlock: 0,
                        sampleRate: 48_000.0,
                    },
                    ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        maxSamplesPerBlock: -1,
                        sampleRate: 48_000.0,
                    },
                    ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        maxSamplesPerBlock: (1 << 20) + 1,
                        sampleRate: 48_000.0,
                    },
                    ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        maxSamplesPerBlock: 64,
                        sampleRate: 0.0,
                    },
                    ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        maxSamplesPerBlock: 64,
                        sampleRate: f64::NAN,
                    },
                    ProcessSetup {
                        processMode: ProcessModes_::kRealtime as int32,
                        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                        maxSamplesPerBlock: 64,
                        sampleRate: f64::INFINITY,
                    },
                ];

                for setup in &mut invalid_setups {
                    assert_eq!(processor.setupProcessing(setup), kInvalidArgument);
                }

                assert_eq!(
                    PREPARE_MATRIX_KERNEL_CREATIONS.load(TestOrdering::Relaxed),
                    0
                );
                assert!(PREPARE_MATRIX_RECORDS.lock().unwrap().is_empty());
            }
        }

        #[test]
        fn processor_routes_sidechain_bus_to_process_context_sample32() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<SidechainPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<SidechainPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let main_l = [100.0_f32, 200.0, 300.0, 400.0];
                let main_r = [-100.0_f32, -200.0, -300.0, -400.0];
                let side_l = [1.0_f32, 2.0, 3.0, 4.0];
                let side_r = [10.0_f32, 20.0, 30.0, 40.0];
                let mut output_l = [0.0_f32; 4];
                let mut output_r = [0.0_f32; 4];
                let mut main_channels = [
                    main_l.as_ptr() as *mut Sample32,
                    main_r.as_ptr() as *mut Sample32,
                ];
                let mut sidechain_channels = [
                    side_l.as_ptr() as *mut Sample32,
                    side_r.as_ptr() as *mut Sample32,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: sidechain_channels.as_mut_ptr(),
                        },
                    },
                ];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 2,
                    numOutputs: 1,
                    inputs: input_buses.as_mut_ptr(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert_eq!(output_l, [2.0, 4.0, 6.0, 8.0]);
                assert_eq!(output_r, [9.0, 18.0, 27.0, 36.0]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_routes_sidechain_bus_through_sample64_scratch_fallback() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<SidechainPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<SidechainPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let main_l = [100.0_f64, 200.0, 300.0, 400.0];
                let main_r = [-100.0_f64, -200.0, -300.0, -400.0];
                let side_l = [1.0_f64, 2.0, 3.0, 4.0];
                let side_r = [10.0_f64, 20.0, 30.0, 40.0];
                let mut output_l = [0.0_f64; 4];
                let mut output_r = [0.0_f64; 4];
                let mut main_channels = [
                    main_l.as_ptr() as *mut Sample64,
                    main_r.as_ptr() as *mut Sample64,
                ];
                let mut sidechain_channels = [
                    side_l.as_ptr() as *mut Sample64,
                    side_r.as_ptr() as *mut Sample64,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: sidechain_channels.as_mut_ptr(),
                        },
                    },
                ];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 4,
                    numInputs: 2,
                    numOutputs: 1,
                    inputs: input_buses.as_mut_ptr(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert_eq!(output_l, [2.0, 4.0, 6.0, 8.0]);
                assert_eq!(output_r, [9.0, 18.0, 27.0, 36.0]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_runs_optional_sidechain_effect_with_main_input_only() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    OptionalSidechainPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<OptionalSidechainPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let main_l = [100.0_f32, 200.0, 300.0, 400.0];
                let main_r = [-100.0_f32, -200.0, -300.0, -400.0];
                let mut output_l = [0.0_f32; 4];
                let mut output_r = [0.0_f32; 4];
                let mut main_channels = [
                    main_l.as_ptr() as *mut Sample32,
                    main_r.as_ptr() as *mut Sample32,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: main_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.0, 2.0, 3.0, 4.0]);
                assert_eq!(output_r, [-1.0, -2.0, -3.0, -4.0]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_runs_optional_sidechain_effect_with_empty_inactive_sidechain_input() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    OptionalSidechainPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<OptionalSidechainPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let main_l = [100.0_f32, 200.0, 300.0, 400.0];
                let main_r = [-100.0_f32, -200.0, -300.0, -400.0];
                let mut output_l = [0.0_f32; 4];
                let mut output_r = [0.0_f32; 4];
                let mut main_channels = [
                    main_l.as_ptr() as *mut Sample32,
                    main_r.as_ptr() as *mut Sample32,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 0,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: ptr::null_mut(),
                        },
                    },
                ];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 2,
                    numOutputs: 1,
                    inputs: input_buses.as_mut_ptr(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.0, 2.0, 3.0, 4.0]);
                assert_eq!(output_r, [-1.0, -2.0, -3.0, -4.0]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_runs_optional_sidechain_effect_sample64_with_main_input_only() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    OptionalSidechainPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<OptionalSidechainPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let main_l = [100.0_f64, 200.0, 300.0, 400.0];
                let main_r = [-100.0_f64, -200.0, -300.0, -400.0];
                let mut output_l = [0.0_f64; 4];
                let mut output_r = [0.0_f64; 4];
                let mut main_channels = [
                    main_l.as_ptr() as *mut Sample64,
                    main_r.as_ptr() as *mut Sample64,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: main_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 4,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.0, 2.0, 3.0, 4.0]);
                assert_eq!(output_r, [-1.0, -2.0, -3.0, -4.0]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_runs_optional_sidechain_effect_sample64_with_empty_inactive_sidechain_input() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    OptionalSidechainPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<OptionalSidechainPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let main_l = [100.0_f64, 200.0, 300.0, 400.0];
                let main_r = [-100.0_f64, -200.0, -300.0, -400.0];
                let mut output_l = [0.0_f64; 4];
                let mut output_r = [0.0_f64; 4];
                let mut main_channels = [
                    main_l.as_ptr() as *mut Sample64,
                    main_r.as_ptr() as *mut Sample64,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 0,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: ptr::null_mut(),
                        },
                    },
                ];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 4,
                    numInputs: 2,
                    numOutputs: 1,
                    inputs: input_buses.as_mut_ptr(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                reset_rt_allocation_count();
                let _guard = NoAllocGuard::enter();
                assert_eq!(processor.process(&mut data), kResultOk);
                drop(_guard);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.0, 2.0, 3.0, 4.0]);
                assert_eq!(output_r, [-1.0, -2.0, -3.0, -4.0]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_process_does_not_allocate_inside_rt_guard_under_automation_and_midi() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 8,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let input_l = [0.0_f32; 8];
                let input_r = [0.0_f32; 8];
                let mut output_l = [0.0_f32; 8];
                let mut output_r = [0.0_f32; 8];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample32,
                    input_r.as_ptr() as *mut Sample32,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };

                let queue = ComWrapper::new(FakeParamValueQueue::new(
                    test_param_id("gain"),
                    vec![(0, 0.1), (3, 0.6), (7, 0.9)],
                ));
                let changes = ComWrapper::new(FakeParameterChanges {
                    queues: vec![queue.to_com_ptr::<IParamValueQueue>().unwrap()],
                });
                let changes_ptr = changes.to_com_ptr::<IParameterChanges>().unwrap();

                let note_on = Event {
                    busIndex: 0,
                    sampleOffset: 1,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteOnEvent as u16,
                    __field0: Event__type0 {
                        noteOn: NoteOnEvent {
                            channel: 0,
                            pitch: 60,
                            tuning: 0.0,
                            velocity: 0.8,
                            length: 0,
                            noteId: 11,
                        },
                    },
                };
                let note_off = Event {
                    busIndex: 0,
                    sampleOffset: 6,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteOffEvent as u16,
                    __field0: Event__type0 {
                        noteOff: NoteOffEvent {
                            channel: 0,
                            pitch: 60,
                            velocity: 0.2,
                            noteId: 11,
                            tuning: 0.0,
                        },
                    },
                };
                let events = ComWrapper::new(FakeEventList::new(vec![note_on, note_off]));
                let events_ptr = events.to_com_ptr::<IEventList>().unwrap();

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 8,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: changes_ptr.as_ptr(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: events_ptr.as_ptr(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
            }
        }

        #[test]
        fn processor_rejects_oversized_input_bus_count_without_entering_kernel() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid process input bus shape.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let input_l = [0.0_f32; 4];
                let input_r = [0.0_f32; 4];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample32,
                    input_r.as_ptr() as *mut Sample32,
                ];
                let mut input_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: input_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers32: input_channels.as_mut_ptr(),
                        },
                    },
                ];
                let mut output_l = [1.0_f32; 4];
                let mut output_r = [-1.0_f32; 4];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 2,
                    numOutputs: 1,
                    inputs: input_buses.as_mut_ptr(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();
                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(!NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [0.0; 4]);
                assert_eq!(output_r, [0.0; 4]);
                assert_eq!(output_bus.silenceFlags, 0b11);
            }
        }

        #[test]
        fn processor_treats_oversized_input_channel_count_as_empty_input() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid host channel-count metadata.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let input_l = [0.0_f32; 4];
                let input_r = [0.0_f32; 4];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample32,
                    input_r.as_ptr() as *mut Sample32,
                ];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 3,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_l = [1.0_f32; 4];
                let mut output_r = [-1.0_f32; 4];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                NO_ALLOC_INPUT_CHANNELS.store(usize::MAX, TestOrdering::Relaxed);
                reset_rt_allocation_count();
                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(NO_ALLOC_INPUT_CHANNELS.load(TestOrdering::Relaxed), 0);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.0; 4]);
                assert_eq!(output_r, [-1.0; 4]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_rejects_oversized_output_channel_count_without_entering_kernel() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid host channel-count metadata.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut output_l = [1.0_f32; 4];
                let mut output_r = [-1.0_f32; 4];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 3,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                NO_ALLOC_INPUT_CHANNELS.store(usize::MAX, TestOrdering::Relaxed);
                reset_rt_allocation_count();
                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(!NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(
                    NO_ALLOC_INPUT_CHANNELS.load(TestOrdering::Relaxed),
                    usize::MAX
                );
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.0; 4]);
                assert_eq!(output_r, [-1.0; 4]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_rejects_negative_process_block_size_without_entering_kernel() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid process block size.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut output_l = [1.0_f32; 6];
                let mut output_r = [-1.0_f32; 6];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: -1,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();
                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(!NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.0; 6]);
                assert_eq!(output_r, [-1.0; 6]);
                assert_eq!(output_bus.silenceFlags, 0b11);
            }
        }

        #[test]
        fn processor_processes_sample64_without_realtime_allocation() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 8,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let input_l = [0.0_f64; 8];
                let input_r = [0.0_f64; 8];
                let mut output_l = [0.0_f64; 8];
                let mut output_r = [0.0_f64; 8];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample64,
                    input_r.as_ptr() as *mut Sample64,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 8,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_set_processing_false_silences_without_entering_kernel() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut output_l = [1.0_f32; 4];
                let mut output_r = [-1.0_f32; 4];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.setProcessing(0), kResultOk);
                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();
                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(!NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [0.0; 4]);
                assert_eq!(output_r, [0.0; 4]);
                assert_eq!(output_bus.silenceFlags, 0b11);

                output_l = [2.0_f32; 4];
                output_r = [-2.0_f32; 4];
                output_bus.silenceFlags = 0b11;
                assert_eq!(processor.setProcessing(1), kResultOk);
                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();
                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [2.0; 4]);
                assert_eq!(output_r, [-2.0; 4]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_processes_sample64_with_native_f64_kernel_without_scratch_fallback() {
            let _lock = NATIVE_F64_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NativeF64Plugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NativeF64Plugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let input_l = [1.0_f64, -0.25, f64::from(f32::MAX) + 1.0, 0.5, -0.75, 2.0];
                let input_r = [0.125_f64, -2.0, 4.0, -0.5, 0.25, 8.0];
                let mut output_l = [0.0_f64; 6];
                let mut output_r = [0.0_f64; 6];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample64,
                    input_r.as_ptr() as *mut Sample64,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };

                NATIVE_F64_F32_ENTERED.store(false, TestOrdering::Relaxed);
                NATIVE_F64_ENTERED.store(false, TestOrdering::Relaxed);
                NATIVE_F64_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                NATIVE_F64_FRAMES.store(0, TestOrdering::Relaxed);
                reset_rt_allocation_count();

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 6,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NATIVE_F64_F32_ENTERED.load(TestOrdering::Relaxed));
                assert!(NATIVE_F64_ENTERED.load(TestOrdering::Relaxed));
                assert!(NATIVE_F64_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(NATIVE_F64_FRAMES.load(TestOrdering::Relaxed), 6);
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_bus.silenceFlags, 0);
                for frame in 0..6 {
                    assert_eq!(output_l[frame], input_l[frame] * 0.5 + NATIVE_F64_LEFT_BIAS);
                    assert_eq!(
                        output_r[frame],
                        input_r[frame] * -0.25 + NATIVE_F64_RIGHT_BIAS
                    );
                }
            }
        }

        #[test]
        fn processor_routes_sidechain_bus_through_native_sample64_process() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    NativeF64SidechainPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NativeF64SidechainPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let main_l = [100.0_f64, 200.0, 300.0, 400.0];
                let main_r = [-100.0_f64, -200.0, -300.0, -400.0];
                let side_l = [1.0_f64, 2.0, 3.0, 4.0];
                let side_r = [10.0_f64, 20.0, 30.0, 40.0];
                let mut output_l = [0.0_f64; 4];
                let mut output_r = [0.0_f64; 4];
                let mut main_channels = [
                    main_l.as_ptr() as *mut Sample64,
                    main_r.as_ptr() as *mut Sample64,
                ];
                let mut sidechain_channels = [
                    side_l.as_ptr() as *mut Sample64,
                    side_r.as_ptr() as *mut Sample64,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_buses = [
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: main_channels.as_mut_ptr(),
                        },
                    },
                    AudioBusBuffers {
                        numChannels: 2,
                        silenceFlags: 0,
                        __field0: AudioBusBuffers__type0 {
                            channelBuffers64: sidechain_channels.as_mut_ptr(),
                        },
                    },
                ];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };

                NATIVE_F64_SIDECHAIN_F32_ENTERED.store(false, TestOrdering::Relaxed);
                NATIVE_F64_SIDECHAIN_ENTERED.store(false, TestOrdering::Relaxed);
                NATIVE_F64_SIDECHAIN_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 4,
                    numInputs: 2,
                    numOutputs: 1,
                    inputs: input_buses.as_mut_ptr(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NATIVE_F64_SIDECHAIN_F32_ENTERED.load(TestOrdering::Relaxed));
                assert!(NATIVE_F64_SIDECHAIN_ENTERED.load(TestOrdering::Relaxed));
                assert!(NATIVE_F64_SIDECHAIN_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert_eq!(output_l, [1.1, 2.2, 3.3, 4.4]);
                assert_eq!(output_r, [9.8, 19.6, 29.4, 39.2]);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_sample64_over_capacity_silences_without_realtime_allocation() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut output_l = [1.0_f64; 8];
                let mut output_r = [-1.0_f64; 8];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 8,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(!NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert!(output_l.iter().all(|sample| *sample == 0.0));
                assert!(output_r.iter().all(|sample| *sample == 0.0));
                assert_eq!(output_bus.silenceFlags, 0b11);
            }
        }

        #[test]
        fn processor_sample64_without_setup_silences_without_creating_kernel() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut output_l = [1.0_f64; 8];
                let mut output_r = [-1.0_f64; 8];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 8,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(!NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert!(output_l.iter().all(|sample| *sample == 0.0));
                assert!(output_r.iter().all(|sample| *sample == 0.0));
                assert_eq!(output_bus.silenceFlags, 0b11);
            }
        }

        #[test]
        fn processor_process_without_setup_silences_without_creating_kernel() {
            let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<NoAllocPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut output_l = [1.0_f32; 8];
                let mut output_r = [1.0_f32; 8];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };

                NO_ALLOC_KERNEL_ENTERED.store(false, TestOrdering::Relaxed);
                NO_ALLOC_GUARD_SEEN.store(false, TestOrdering::Relaxed);
                reset_rt_allocation_count();

                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 8,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert!(!NO_ALLOC_KERNEL_ENTERED.load(TestOrdering::Relaxed));
                assert!(!NO_ALLOC_GUARD_SEEN.load(TestOrdering::Relaxed));
                assert_eq!(rt_allocation_count(), 0);
                assert!(output_l.iter().all(|sample| *sample == 0.0));
                assert!(output_r.iter().all(|sample| *sample == 0.0));
                assert_eq!(output_bus.silenceFlags, 0b11);
            }
        }

        #[test]
        fn processor_updates_output_silence_flags() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<SilencePlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<SilencePlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("silence processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut output_l = [1.0_f32; 4];
                let mut output_r = [-1.0_f32; 4];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert_eq!(output_bus.silenceFlags, 0b11);
                assert!(output_l.iter().all(|sample| *sample == 0.0));
                assert!(output_r.iter().all(|sample| *sample == 0.0));

                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("continue processor");
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut output_l = [0.25_f32; 4];
                let mut output_r = [0.5_f32; 4];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0b11,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert_eq!(output_bus.silenceFlags, 0);
            }
        }

        #[test]
        fn processor_panic_faults_and_silences_subsequent_blocks() {
            let _guard = PANIC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<PanicPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<PanicPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let mut output = [1.0_f32; 4];
                let mut output_channels = [output.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 1,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                PANIC_KERNEL_CALLS.store(0, TestOrdering::Relaxed);
                assert_eq!(processor.process(&mut data), kResultOk);
                assert_eq!(output, [0.0; 4]);
                assert_eq!(PANIC_KERNEL_CALLS.load(TestOrdering::Relaxed), 1);

                output.fill(0.25);
                assert_eq!(processor.process(&mut data), kResultOk);
                assert_eq!(output, [0.0; 4]);
                assert_eq!(PANIC_KERNEL_CALLS.load(TestOrdering::Relaxed), 1);
            }
        }

        #[test]
        fn factory_catches_plugin_default_panics() {
            // SAFETY: Test code invokes the generated COM callback to verify panic containment.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    DefaultPanicPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<DefaultPanicPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();

                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultFalse
                );
                assert!(processor.is_null());
            }
        }

        #[test]
        fn processor_and_controller_callbacks_catch_plugin_hook_panics() {
            // SAFETY: Test code invokes generated COM callbacks with a plugin whose hooks panic.
            unsafe {
                let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<
                    CallbackPanicPlugin,
                >())
                .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<CallbackPanicPlugin>();

                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");
                assert_eq!(processor.getLatencySamples(), 0);

                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                assert!(
                    controller
                        .createView(c"editor".as_ptr() as *const c_char)
                        .is_null()
                );

                let state = ComWrapper::new(MemoryStream::default());
                let state_ptr = state.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(controller.getState(state_ptr.as_ptr()), kResultFalse);
            }
        }

        #[test]
        fn processor_panic_emits_rt_log_event_to_controller() {
            let _guard = PANIC_PLUGIN_TEST_LOCK.lock().unwrap();
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let registry =
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default());
                let processor = ComWrapper::new(
                    crate::bindings_impl::VestyProcessor::<PanicPlugin>::with_telemetry_registry(
                        registry.clone(),
                    ),
                );
                let controller = ComWrapper::new(crate::bindings_impl::VestyController::<
                    PanicPlugin,
                >::with_telemetry_registry(
                    registry
                ));
                let processor_connection = processor.to_com_ptr::<IConnectionPoint>().unwrap();
                let controller_connection = controller.to_com_ptr::<IConnectionPoint>().unwrap();
                assert_eq!(
                    processor_connection.connect(controller_connection.as_ptr()),
                    kResultOk
                );

                let audio_processor = processor.to_com_ptr::<IAudioProcessor>().unwrap();
                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(audio_processor.setupProcessing(&mut setup), kResultOk);

                let mut output = [1.0_f32; 4];
                let mut output_channels = [output.as_mut_ptr()];
                let mut output_bus = AudioBusBuffers {
                    numChannels: 1,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 0,
                    numOutputs: 1,
                    inputs: ptr::null_mut(),
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                PANIC_KERNEL_CALLS.store(0, TestOrdering::Relaxed);
                assert_eq!(audio_processor.process(&mut data), kResultOk);

                let events = controller.drain_rt_log_events_for_test();
                assert_eq!(
                    events,
                    vec![vesty_rt::RtLogEvent::Faulted {
                        code: crate::bindings_impl::RT_LOG_CODE_PROCESS_PANIC
                    }]
                );

                assert_eq!(audio_processor.process(&mut data), kResultOk);
                assert!(controller.drain_rt_log_events_for_test().is_empty());
            }
        }

        #[test]
        fn processor_processes_sample64_and_writes_host_outputs() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MeterPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<MeterPlugin>();
                let processor_cid = tuid(metadata.processor_class_id);
                let mut processor: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        processor_cid.as_ptr(),
                        IAudioProcessor_iid.as_ptr(),
                        &mut processor,
                    ),
                    kResultOk
                );
                let processor =
                    ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
                        .expect("processor");

                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

                let input_l = [0.0_f64; 4];
                let input_r = [0.0_f64; 4];
                let mut output_l = [0.0_f64; 4];
                let mut output_r = [0.0_f64; 4];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample64,
                    input_r.as_ptr() as *mut Sample64,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers64: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample64 as int32,
                    numSamples: 4,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(processor.process(&mut data), kResultOk);
                assert_eq!(output_bus.silenceFlags, 0);
                assert_eq!(output_l[0], -0.25);
                assert_eq!(output_l[1], 0.75);
                assert_eq!(output_r[0], 0.5);
                assert_eq!(output_r[1], -0.125);
            }
        }

        #[test]
        fn processor_meter_frames_are_bound_to_controller() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let registry =
                    std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default());
                let processor = ComWrapper::new(
                    crate::bindings_impl::VestyProcessor::<MeterPlugin>::with_telemetry_registry(
                        registry.clone(),
                    ),
                );
                let controller = ComWrapper::new(crate::bindings_impl::VestyController::<
                    MeterPlugin,
                >::with_telemetry_registry(
                    registry
                ));
                let processor_connection = processor.to_com_ptr::<IConnectionPoint>().unwrap();
                let controller_connection = controller.to_com_ptr::<IConnectionPoint>().unwrap();
                assert_eq!(
                    processor_connection.connect(controller_connection.as_ptr()),
                    kResultOk
                );

                let audio_processor = processor.to_com_ptr::<IAudioProcessor>().unwrap();
                let mut setup = ProcessSetup {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    maxSamplesPerBlock: 4,
                    sampleRate: 48_000.0,
                };
                assert_eq!(audio_processor.setupProcessing(&mut setup), kResultOk);

                let input_l = [0.0_f32; 4];
                let input_r = [0.0_f32; 4];
                let mut output_l = [0.0_f32; 4];
                let mut output_r = [0.0_f32; 4];
                let mut input_channels = [
                    input_l.as_ptr() as *mut Sample32,
                    input_r.as_ptr() as *mut Sample32,
                ];
                let mut output_channels = [output_l.as_mut_ptr(), output_r.as_mut_ptr()];
                let mut input_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: input_channels.as_mut_ptr(),
                    },
                };
                let mut output_bus = AudioBusBuffers {
                    numChannels: 2,
                    silenceFlags: 0,
                    __field0: AudioBusBuffers__type0 {
                        channelBuffers32: output_channels.as_mut_ptr(),
                    },
                };
                let mut data = ProcessData {
                    processMode: ProcessModes_::kRealtime as int32,
                    symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
                    numSamples: 4,
                    numInputs: 1,
                    numOutputs: 1,
                    inputs: &mut input_bus,
                    outputs: &mut output_bus,
                    inputParameterChanges: ptr::null_mut(),
                    outputParameterChanges: ptr::null_mut(),
                    inputEvents: ptr::null_mut(),
                    outputEvents: ptr::null_mut(),
                    processContext: ptr::null_mut(),
                };

                assert_eq!(audio_processor.process(&mut data), kResultOk);

                let frames = controller.drain_meter_frames_for_test();
                assert_eq!(frames.len(), 1);
                assert_eq!(frames[0].id_hash, 77);
                assert_eq!(frames[0].sample_offset, 3);
                assert_eq!(frames[0].channel_count(), 2);
                assert_eq!(frames[0].peaks[0], 0.75);
                assert_eq!(frames[0].peaks[1], 0.5);
            }
        }

        #[test]
        fn controller_relays_param_gestures_to_component_handler() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
                assert_eq!(controller.begin_param_edit(gain_id), kResultFalse);
                assert_eq!(controller.perform_param_edit(gain_id, 0.5), kResultFalse);
                assert_eq!(controller.end_param_edit(gain_id), kResultFalse);
                assert_eq!(controller.begin_param_edit(99), kInvalidArgument);

                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );

                assert_eq!(controller.begin_param_edit(gain_id), kResultOk);
                assert_eq!(controller.perform_param_edit(gain_id, 1.5), kResultOk);
                assert_eq!(controller.end_param_edit(gain_id), kResultOk);
                assert_eq!(controller.getParamNormalized(gain_id), 1.0);
                assert_eq!(
                    handler.calls(),
                    vec![
                        HandlerCall::Begin(gain_id),
                        HandlerCall::Perform(gain_id, 1.0),
                        HandlerCall::End(gain_id),
                    ]
                );

                assert_eq!(controller.setComponentHandler(ptr::null_mut()), kResultOk);
                assert_eq!(controller.begin_param_edit(gain_id), kResultFalse);
            }
        }

        #[test]
        fn controller_rejects_read_only_param_gestures() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<FlagPlugin>::new();
                let meter_id = controller.param_id_for_test(1).expect("meter ParamID");
                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );

                assert_eq!(controller.begin_param_edit(meter_id), kInvalidArgument);
                assert_eq!(
                    controller.perform_param_edit(meter_id, 0.75),
                    kInvalidArgument
                );
                assert_eq!(controller.end_param_edit(meter_id), kInvalidArgument);
                assert_eq!(controller.getParamNormalized(meter_id), 0.0);
                assert!(handler.calls().is_empty());
            }
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn controller_wry_bridge_returns_native_ipc_validation_errors() {
            let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
            let endpoint = controller.bridge_endpoint();
            let bridge = endpoint.bridge_handler();

            let hello = serde_json::json!({
                "v": 1,
                "session": "pending",
                "seq": 1,
                "lane": "command",
                "kind": "request",
                "type": "bridge.hello",
                "id": "hello",
                "payload": {
                    "supportedProtocolVersions": [1],
                    "jsPackageVersion": "test",
                    "pageUrl": "vesty://assets/index.html",
                },
            })
            .to_string();
            let hello_packets = bridge(hello);
            let session = hello_packets[0]
                .payload
                .as_ref()
                .and_then(|payload| payload.get("editorSessionId"))
                .and_then(serde_json::Value::as_str)
                .expect("editor session")
                .to_string();

            let non_string_type = serde_json::json!({
                "v": 1,
                "session": session,
                "seq": 2,
                "lane": "command",
                "kind": "request",
                "type": 42,
                "id": "bad-type",
            })
            .to_string();
            let parse_packets = bridge(non_string_type);
            assert_eq!(parse_packets.len(), 1);
            assert_eq!(parse_packets[0].packet_type, "bridge.parseError.error");
            assert_eq!(parse_packets[0].reply_to.as_deref(), Some("bad-type"));
            let parse_error = parse_packets[0].error.as_ref().unwrap();
            assert_eq!(parse_error.code, BridgeErrorCode::ParseError);
            assert_eq!(parse_error.message, "failed to parse bridge packet");
            assert!(!parse_error.retryable);

            let control_type = serde_json::json!({
                "v": 1,
                "session": session,
                "seq": 3,
                "lane": "command",
                "kind": "request",
                "type": "bridge.hello\u{7}",
                "id": "control-type",
                "payload": {
                    "supportedProtocolVersions": [1],
                    "jsPackageVersion": "test",
                    "pageUrl": "vesty://assets/index.html",
                },
            })
            .to_string();
            let validation_packets = bridge(control_type);
            assert_eq!(validation_packets.len(), 1);
            assert_eq!(
                validation_packets[0].packet_type,
                "bridge.invalidType.error"
            );
            assert_eq!(
                validation_packets[0].reply_to.as_deref(),
                Some("control-type")
            );
            let validation_error = validation_packets[0].error.as_ref().unwrap();
            assert_eq!(validation_error.code, BridgeErrorCode::ValidationError);
            assert_eq!(
                validation_error.message,
                "request type must not contain control characters"
            );
            assert!(!validation_error.retryable);
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn controller_wry_bridge_reports_host_rejection_without_mutating_param() {
            // SAFETY: Test code wires a fake component handler into the controller COM callback.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
                let handler = ComWrapper::new(FakeComponentHandler::rejecting_perform());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );
                let endpoint = controller.bridge_endpoint();
                let bridge = endpoint.bridge_handler();

                let hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "hello",
                    "payload": {
                        "supportedProtocolVersions": [1],
                        "jsPackageVersion": "test",
                        "pageUrl": "vesty://assets/index.html",
                    },
                })
                .to_string();
                let hello_packets = bridge(hello);
                assert_eq!(
                    hello_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
                    0.5
                );
                let session = hello_packets[0]
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("editorSessionId"))
                    .and_then(serde_json::Value::as_str)
                    .expect("editor session");

                let perform = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 2,
                    "lane": "param",
                    "kind": "request",
                    "type": "param.perform",
                    "id": "perform-rejected",
                    "payload": { "id": "gain", "normalized": 0.75 },
                })
                .to_string();
                let packets = bridge(perform);

                assert_eq!(packets.len(), 1);
                assert_eq!(packets[0].kind, BridgeKind::Error);
                assert_eq!(packets[0].reply_to.as_deref(), Some("perform-rejected"));
                assert_eq!(
                    packets[0].error.as_ref().map(|error| error.code.clone()),
                    Some(BridgeErrorCode::HostRejected)
                );
                assert_eq!(controller.getParamNormalized(gain_id), 0.5);
                assert_eq!(handler.calls(), vec![HandlerCall::Perform(gain_id, 0.75)]);
            }
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn controller_wry_bridge_emits_param_changed_after_ui_perform() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );

                let endpoint = controller.bridge_endpoint();
                let bridge = endpoint.bridge_handler();

                let hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "hello",
                    "payload": {
                        "supportedProtocolVersions": [1],
                        "jsPackageVersion": "test",
                        "pageUrl": "vesty://assets/index.html",
                    },
                })
                .to_string();
                let hello_packets = bridge(hello);
                assert_eq!(
                    hello_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
                    0.5
                );
                let session = hello_packets[0]
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("editorSessionId"))
                    .and_then(serde_json::Value::as_str)
                    .expect("editor session")
                    .to_string();

                let subscribe = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 2,
                    "lane": "command",
                    "kind": "request",
                    "type": "subscription.add",
                    "id": "sub-param",
                    "payload": { "topic": "param.changed" },
                })
                .to_string();
                let subscribe_packets = bridge(subscribe);
                assert_eq!(subscribe_packets.len(), 1);
                assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-param"));

                let begin = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 3,
                    "lane": "param",
                    "kind": "request",
                    "type": "param.begin",
                    "id": "begin",
                    "payload": { "id": "gain", "gestureId": "drag-1" },
                })
                .to_string();
                let begin_packets = bridge(begin);
                assert_eq!(begin_packets.len(), 1);
                assert_eq!(begin_packets[0].reply_to.as_deref(), Some("begin"));

                let perform = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 4,
                    "lane": "param",
                    "kind": "request",
                    "type": "param.perform",
                    "id": "perform",
                    "payload": {
                        "id": "gain",
                        "normalized": 0.75,
                        "gestureId": "drag-1",
                    },
                })
                .to_string();
                let perform_packets = bridge(perform);
                assert_eq!(perform_packets.len(), 2);
                assert_eq!(perform_packets[0].reply_to.as_deref(), Some("perform"));

                let event = perform_packets
                    .iter()
                    .find(|packet| packet.packet_type == "param.changed")
                    .expect("param changed event");
                assert_eq!(event.payload.as_ref().unwrap()["id"], "gain");
                assert_eq!(event.payload.as_ref().unwrap()["normalized"], 0.75);
                assert_eq!(event.payload.as_ref().unwrap()["plain"], 1.5);
                assert_eq!(event.payload.as_ref().unwrap()["display"], "1.500");
                assert_eq!(event.payload.as_ref().unwrap()["source"], "ui");
                assert_eq!(event.payload.as_ref().unwrap()["gestureId"], "drag-1");
                assert_eq!(event.payload.as_ref().unwrap()["revision"], 1);

                assert_eq!(
                    handler.calls(),
                    vec![
                        HandlerCall::Begin(gain_id),
                        HandlerCall::Perform(gain_id, 0.75)
                    ]
                );
            }
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn controller_wry_bridge_emits_host_param_changes_on_event_flush() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
                assert_eq!(controller.setParamNormalized(gain_id, 0.25), kResultOk);

                let endpoint = controller.bridge_endpoint();
                let bridge = endpoint.bridge_handler();

                let hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "hello",
                    "payload": {
                        "supportedProtocolVersions": [1],
                        "jsPackageVersion": "test",
                        "pageUrl": "vesty://assets/index.html",
                    },
                })
                .to_string();
                let hello_packets = bridge(hello);
                assert_eq!(
                    hello_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
                    0.25
                );
                let session = hello_packets[0]
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("editorSessionId"))
                    .and_then(serde_json::Value::as_str)
                    .expect("editor session")
                    .to_string();

                let subscribe = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 2,
                    "lane": "command",
                    "kind": "request",
                    "type": "subscription.add",
                    "id": "sub-param",
                    "payload": { "topic": "param.changed" },
                })
                .to_string();
                let subscribe_packets = bridge(subscribe);
                assert_eq!(subscribe_packets.len(), 2);
                assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-param"));
                let catch_up = subscribe_packets
                    .iter()
                    .find(|packet| packet.packet_type == "param.changed")
                    .expect("catch-up param changed event");
                assert_eq!(catch_up.payload.as_ref().unwrap()["id"], "gain");
                assert_eq!(catch_up.payload.as_ref().unwrap()["normalized"], 0.25);
                assert_eq!(catch_up.payload.as_ref().unwrap()["plain"], 0.5);
                assert_eq!(catch_up.payload.as_ref().unwrap()["source"], "host");

                assert_eq!(controller.setParamNormalized(gain_id, 0.5), kResultOk);
                let flush = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 3,
                    "lane": "event",
                    "kind": "request",
                    "type": "event.flush",
                    "id": "flush",
                })
                .to_string();
                let flush_packets = bridge(flush);
                assert_eq!(flush_packets.len(), 2);
                assert_eq!(flush_packets[0].reply_to.as_deref(), Some("flush"));

                let event = flush_packets
                    .iter()
                    .find(|packet| packet.packet_type == "param.changed")
                    .expect("flushed param changed event");
                assert_eq!(event.payload.as_ref().unwrap()["id"], "gain");
                assert_eq!(event.payload.as_ref().unwrap()["normalized"], 0.5);
                assert_eq!(event.payload.as_ref().unwrap()["plain"], 1.0);
                assert_eq!(event.payload.as_ref().unwrap()["display"], "1.000");
                assert_eq!(event.payload.as_ref().unwrap()["source"], "host");
                assert_eq!(
                    event.payload.as_ref().unwrap()["gestureId"],
                    serde_json::Value::Null
                );

                let state_input = ComWrapper::new(MemoryStream::with_bytes(raw_state_bytes(
                    serde_json::json!({
                        "version": 1,
                        "params": [{ "id": "gain", "normalized": 0.75 }],
                    }),
                )));
                let state_input_ptr = state_input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(controller.setState(state_input_ptr.as_ptr()), kResultOk);
                let flush_state = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 4,
                    "lane": "event",
                    "kind": "request",
                    "type": "event.flush",
                    "id": "flush-state",
                })
                .to_string();
                let state_packets = bridge(flush_state);
                assert_eq!(state_packets[0].reply_to.as_deref(), Some("flush-state"));
                let state_event = state_packets
                    .iter()
                    .find(|packet| {
                        packet.packet_type == "param.changed"
                            && packet
                                .payload
                                .as_ref()
                                .is_some_and(|payload| payload["id"] == "gain")
                    })
                    .expect("state param changed event");
                assert_eq!(state_event.payload.as_ref().unwrap()["normalized"], 0.75);
                assert_eq!(state_event.payload.as_ref().unwrap()["source"], "state");

                let reopened = controller.bridge_endpoint().bridge_handler();
                let reopened_hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "reopened-hello",
                    "payload": { "supportedProtocolVersions": [1] },
                })
                .to_string();
                let reopened_packets = reopened(reopened_hello);
                assert_eq!(
                    reopened_packets[0].payload.as_ref().unwrap()["paramValues"][0]["normalized"],
                    0.75
                );
            }
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn controller_wry_bridge_marks_program_param_changes() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<MidiMappedPlugin>::new();
                let gain_id = test_param_id("gain");
                let cutoff_id = test_param_id("cutoff");
                let pitch_id = test_param_id("pitch");

                let endpoint = controller.bridge_endpoint();
                let bridge = endpoint.bridge_handler();

                let hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "hello",
                    "payload": {
                        "supportedProtocolVersions": [1],
                        "jsPackageVersion": "test",
                        "pageUrl": "vesty://assets/index.html",
                    },
                })
                .to_string();
                let hello_packets = bridge(hello);
                let session = hello_packets[0]
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("editorSessionId"))
                    .and_then(serde_json::Value::as_str)
                    .expect("editor session")
                    .to_string();

                let subscribe = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 2,
                    "lane": "command",
                    "kind": "request",
                    "type": "subscription.add",
                    "id": "sub-param",
                    "payload": { "topic": "param.changed" },
                })
                .to_string();
                let subscribe_packets = bridge(subscribe);
                assert_eq!(subscribe_packets.len(), 1);
                assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-param"));

                assert_eq!(
                    controller.setUnitProgramData(77, 1, ptr::null_mut()),
                    kResultOk
                );
                let flush_program = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 3,
                    "lane": "event",
                    "kind": "request",
                    "type": "event.flush",
                    "id": "flush-program",
                })
                .to_string();
                let program_packets = bridge(flush_program);
                assert_eq!(
                    program_packets[0].reply_to.as_deref(),
                    Some("flush-program")
                );
                let program_event = program_packets
                    .iter()
                    .find(|packet| {
                        packet.packet_type == "param.changed"
                            && packet
                                .payload
                                .as_ref()
                                .is_some_and(|payload| payload["id"] == "gain")
                    })
                    .expect("program param changed event");
                let program_normalized = program_event.payload.as_ref().unwrap()["normalized"]
                    .as_f64()
                    .expect("program normalized");
                assert!((program_normalized - 0.8).abs() < 0.000_001);
                assert_eq!(program_event.payload.as_ref().unwrap()["source"], "program");
                assert_eq!(
                    program_event.payload.as_ref().unwrap()["gestureId"],
                    serde_json::Value::Null
                );

                assert_eq!(controller.setParamNormalized(gain_id, 0.11), kResultOk);
                assert_eq!(controller.setParamNormalized(cutoff_id, 0.22), kResultOk);
                assert_eq!(controller.setParamNormalized(pitch_id, 0.33), kResultOk);
                let drain_host_changes = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 4,
                    "lane": "event",
                    "kind": "request",
                    "type": "event.flush",
                    "id": "drain-host",
                })
                .to_string();
                let drain_packets = bridge(drain_host_changes);
                assert_eq!(drain_packets[0].reply_to.as_deref(), Some("drain-host"));

                let program_id = test_param_id("program");
                assert_eq!(controller.setParamNormalized(program_id, 1.0), kResultOk);
                let flush_program_param = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 5,
                    "lane": "event",
                    "kind": "request",
                    "type": "event.flush",
                    "id": "flush-program-param",
                })
                .to_string();
                let program_param_packets = bridge(flush_program_param);
                assert_eq!(
                    program_param_packets[0].reply_to.as_deref(),
                    Some("flush-program-param")
                );
                let program_param_event = program_param_packets
                    .iter()
                    .find(|packet| {
                        packet.packet_type == "param.changed"
                            && packet
                                .payload
                                .as_ref()
                                .is_some_and(|payload| payload["id"] == "gain")
                    })
                    .expect("program-change param changed event");
                let program_param_normalized =
                    program_param_event.payload.as_ref().unwrap()["normalized"]
                        .as_f64()
                        .expect("program-change normalized");
                assert!((program_param_normalized - 0.3).abs() < 0.000_001);
                assert_eq!(
                    program_param_event.payload.as_ref().unwrap()["source"],
                    "program"
                );

                let input = ComWrapper::new(MemoryStream::with_bytes(raw_program_data_bytes(
                    77,
                    1,
                    serde_json::json!({
                        "gain": 0.42,
                        "cutoff": 0.84,
                        "pitch": 0.21,
                    }),
                )));
                let input_ptr = input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(
                    controller.setProgramData(77, 1, input_ptr.as_ptr()),
                    kResultOk
                );
                let flush_program_data = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 6,
                    "lane": "event",
                    "kind": "request",
                    "type": "event.flush",
                    "id": "flush-program-data",
                })
                .to_string();
                let program_data_packets = bridge(flush_program_data);
                assert_eq!(
                    program_data_packets[0].reply_to.as_deref(),
                    Some("flush-program-data")
                );
                let program_data_event = program_data_packets
                    .iter()
                    .find(|packet| {
                        packet.packet_type == "param.changed"
                            && packet
                                .payload
                                .as_ref()
                                .is_some_and(|payload| payload["id"] == "gain")
                    })
                    .expect("program data param changed event");
                let program_data_normalized =
                    program_data_event.payload.as_ref().unwrap()["normalized"]
                        .as_f64()
                        .expect("program data normalized");
                assert!((program_data_normalized - 0.42).abs() < 0.000_001);
                assert_eq!(
                    program_data_event.payload.as_ref().unwrap()["source"],
                    "program"
                );
            }
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn controller_wry_bridge_state_roundtrips_through_vst3_state() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let endpoint = controller.bridge_endpoint();
                let bridge = endpoint.bridge_handler();

                let hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "hello",
                    "payload": {
                        "supportedProtocolVersions": [1],
                        "jsPackageVersion": "test",
                        "pageUrl": "vesty://assets/index.html",
                    },
                })
                .to_string();
                let hello_packets = bridge(hello);
                let session = hello_packets[0]
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("editorSessionId"))
                    .and_then(serde_json::Value::as_str)
                    .expect("editor session")
                    .to_string();

                let set_config = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 2,
                    "lane": "state",
                    "kind": "request",
                    "type": "state.setConfig",
                    "id": "set-config",
                    "payload": {
                        "baseRevision": 0,
                        "key": "theme",
                        "value": "dark",
                    },
                })
                .to_string();
                let config_packets = bridge(set_config);
                assert_eq!(config_packets[0].reply_to.as_deref(), Some("set-config"));

                let set_ui = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 3,
                    "lane": "state",
                    "kind": "request",
                    "type": "state.setUiState",
                    "id": "set-ui",
                    "payload": {
                        "baseRevision": 0,
                        "value": { "panel": "advanced" },
                    },
                })
                .to_string();
                let ui_packets = bridge(set_ui);
                assert_eq!(ui_packets[0].reply_to.as_deref(), Some("set-ui"));

                let saved = ComWrapper::new(MemoryStream::default());
                let saved_ptr = saved.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(controller.getState(saved_ptr.as_ptr()), kResultOk);
                let saved_bytes = saved.bytes();
                let saved_text = String::from_utf8_lossy(&saved_bytes);
                assert!(saved_text.contains(r#""bridge""#));
                assert!(saved_text.contains(r#""uiState""#));
                assert!(saved_text.contains("advanced"));

                let restored = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let input = ComWrapper::new(MemoryStream::with_bytes(saved_bytes));
                let input_ptr = input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(restored.setState(input_ptr.as_ptr()), kResultOk);

                let restored_endpoint = restored.bridge_endpoint();
                let restored_bridge = restored_endpoint.bridge_handler();
                let restored_hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "hello",
                    "payload": {
                        "supportedProtocolVersions": [1],
                        "jsPackageVersion": "test",
                        "pageUrl": "vesty://assets/index.html",
                    },
                })
                .to_string();
                let restored_packets = restored_bridge(restored_hello);
                let snapshot = restored_packets[0]
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("snapshot"))
                    .expect("ready snapshot");
                assert_eq!(snapshot["revision"], 2);
                assert_eq!(snapshot["configRevision"], 1);
                assert_eq!(snapshot["uiRevision"], 1);
                assert_eq!(snapshot["config"]["theme"], "dark");
                assert_eq!(snapshot["uiState"]["panel"], "advanced");
            }
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn controller_wry_bridge_syncs_state_restore_to_active_ui_runtime() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let endpoint = controller.bridge_endpoint();
                let bridge = endpoint.bridge_handler();

                let hello = serde_json::json!({
                    "v": 1,
                    "session": "pending",
                    "seq": 1,
                    "lane": "command",
                    "kind": "request",
                    "type": "bridge.hello",
                    "id": "hello",
                    "payload": {
                        "supportedProtocolVersions": [1],
                        "jsPackageVersion": "test",
                        "pageUrl": "vesty://assets/index.html",
                    },
                })
                .to_string();
                let hello_packets = bridge(hello);
                let session = hello_packets[0]
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("editorSessionId"))
                    .and_then(serde_json::Value::as_str)
                    .expect("editor session")
                    .to_string();

                let subscribe = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 2,
                    "lane": "command",
                    "kind": "request",
                    "type": "subscription.add",
                    "id": "sub-state",
                    "payload": { "topic": "state.changed" },
                })
                .to_string();
                let subscribe_packets = bridge(subscribe);
                assert_eq!(subscribe_packets.len(), 1);
                assert_eq!(subscribe_packets[0].reply_to.as_deref(), Some("sub-state"));

                let state_input = ComWrapper::new(MemoryStream::with_bytes(raw_state_bytes(
                    serde_json::json!({
                        "version": 1,
                        "params": [{ "id": "gain", "normalized": 0.75 }],
                        "bridge": {
                            "revision": 11,
                            "paramsRevision": 2,
                            "configRevision": 5,
                            "uiRevision": 4,
                            "config": { "theme": "light", "scale": 1.25 },
                            "uiState": { "panel": "compact" },
                        },
                    }),
                )));
                let state_input_ptr = state_input.to_com_ptr::<IBStream>().unwrap();
                assert_eq!(controller.setState(state_input_ptr.as_ptr()), kResultOk);

                let flush = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 3,
                    "lane": "event",
                    "kind": "request",
                    "type": "event.flush",
                    "id": "flush",
                })
                .to_string();
                let flush_packets = bridge(flush);
                let state_event = flush_packets
                    .iter()
                    .find(|packet| packet.packet_type == "state.changed")
                    .expect("state restore event");
                let payload = state_event.payload.as_ref().unwrap();
                assert_eq!(payload["revision"], 11);
                assert_eq!(payload["configRevision"], 5);
                assert_eq!(payload["uiRevision"], 4);
                assert_eq!(payload["config"]["theme"], "light");
                assert_eq!(payload["config"]["scale"], 1.25);
                assert_eq!(payload["uiState"]["panel"], "compact");
                assert!(
                    flush_packets
                        .iter()
                        .any(|packet| packet.reply_to.as_deref() == Some("flush"))
                );

                let snapshot_get = serde_json::json!({
                    "v": 1,
                    "session": session,
                    "seq": 4,
                    "lane": "state",
                    "kind": "request",
                    "type": "snapshot.get",
                    "id": "snapshot",
                })
                .to_string();
                let snapshot_packets = bridge(snapshot_get);
                let snapshot = snapshot_packets[0].payload.as_ref().unwrap();
                assert_eq!(snapshot["revision"], 11);
                assert_eq!(snapshot["config"]["theme"], "light");
                assert_eq!(snapshot["uiState"]["panel"], "compact");
            }
        }

        #[test]
        fn controller_notifies_host_when_latency_affecting_param_changes() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let gain_id = controller.param_id_for_test(0).expect("gain ParamID");
                let mode_id = controller.param_id_for_test(1).expect("mode ParamID");
                let handler = ComWrapper::new(FakeComponentHandler::default());
                let handler_ptr = handler.to_com_ptr::<IComponentHandler>().unwrap();
                assert_eq!(
                    controller.setComponentHandler(handler_ptr.as_ptr()),
                    kResultOk
                );

                assert_eq!(controller.setParamNormalized(gain_id, 0.25), kResultOk);
                assert!(handler.calls().is_empty());

                assert_eq!(controller.setParamNormalized(mode_id, 1.0), kResultOk);
                assert_eq!(
                    handler.calls(),
                    vec![HandlerCall::Restart(RestartFlags_::kLatencyChanged)]
                );

                assert_eq!(controller.perform_param_edit(mode_id, 0.5), kResultOk);
                assert_eq!(
                    handler.calls(),
                    vec![
                        HandlerCall::Restart(RestartFlags_::kLatencyChanged),
                        HandlerCall::Perform(mode_id, 0.5),
                        HandlerCall::Restart(RestartFlags_::kLatencyChanged),
                    ]
                );
            }
        }

        #[test]
        fn controller_creates_editor_view() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");

                let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
                let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

                let mut rect = MaybeUninit::<ViewRect>::zeroed();
                assert_eq!(view.getSize(rect.as_mut_ptr()), kResultOk);
                let rect = rect.assume_init();
                assert_eq!(rect.right, 777);
                assert_eq!(rect.bottom, 333);

                let mut too_small = ViewRect {
                    left: 0,
                    top: 0,
                    right: 10,
                    bottom: 10,
                };
                assert_eq!(view.checkSizeConstraint(&mut too_small), kResultOk);
                assert_eq!(too_small.right, 320);
                assert_eq!(too_small.bottom, 200);

                assert_eq!(view.canResize(), kResultTrue);
                assert_eq!(view.getSize(ptr::null_mut()), kInvalidArgument);
                assert_eq!(view.checkSizeConstraint(ptr::null_mut()), kInvalidArgument);
                assert_eq!(view.onSize(ptr::null_mut()), kInvalidArgument);

                let mut resized = ViewRect {
                    left: 4,
                    top: 8,
                    right: 120,
                    bottom: 90,
                };
                assert_eq!(view.onSize(&mut resized), kResultOk);

                let mut current = MaybeUninit::<ViewRect>::zeroed();
                assert_eq!(view.getSize(current.as_mut_ptr()), kResultOk);
                let current = current.assume_init();
                assert_eq!(current.left, 4);
                assert_eq!(current.top, 8);
                assert_eq!(current.right, 324);
                assert_eq!(current.bottom, 208);

                let mut large = ViewRect {
                    left: 1,
                    top: 2,
                    right: 901,
                    bottom: 602,
                };
                assert_eq!(view.onSize(&mut large), kResultOk);
                let mut current = MaybeUninit::<ViewRect>::zeroed();
                assert_eq!(view.getSize(current.as_mut_ptr()), kResultOk);
                let current = current.assume_init();
                assert_eq!(current.left, 1);
                assert_eq!(current.top, 2);
                assert_eq!(current.right, 901);
                assert_eq!(current.bottom, 602);

                assert_eq!(view.removed(), kResultOk);
                assert_eq!(view.removed(), kResultOk);
            }
        }

        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        #[test]
        fn editor_view_rejects_null_platform_and_parent_handles() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with intentionally invalid host handles.
            unsafe {
                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);
                let mut controller: *mut c_void = ptr::null_mut();
                assert_eq!(
                    factory.createInstance(
                        controller_cid.as_ptr(),
                        IEditController_iid.as_ptr(),
                        &mut controller,
                    ),
                    kResultOk
                );
                let controller =
                    ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                        .expect("controller");
                let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
                let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

                assert_eq!(view.isPlatformTypeSupported(ptr::null()), kResultFalse);
                assert_eq!(
                    view.attached(ptr::null_mut(), supported_platform_type_for_current_os()),
                    kResultFalse
                );
            }
        }

        #[test]
        fn editor_open_close_resize_fake_host_stress() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                const CYCLES: usize = 128;
                const WIDTHS: [i32; 6] = [1, 120, 319, 320, 777, 1200];
                const HEIGHTS: [i32; 6] = [1, 80, 199, 200, 333, 900];

                let factory =
                    ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
                        .expect("factory");
                let metadata = Vst3BundleMetadata::for_plugin::<TestPlugin>();
                let controller_cid = tuid(metadata.controller_class_id);

                for cycle in 0..CYCLES {
                    let mut controller: *mut c_void = ptr::null_mut();
                    assert_eq!(
                        factory.createInstance(
                            controller_cid.as_ptr(),
                            IEditController_iid.as_ptr(),
                            &mut controller,
                        ),
                        kResultOk,
                        "cycle {cycle}: create controller"
                    );
                    let controller =
                        ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
                            .expect("controller");

                    assert!(
                        controller
                            .createView(c"not-editor".as_ptr() as *const c_char)
                            .is_null(),
                        "cycle {cycle}: unknown view name is rejected"
                    );

                    let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
                    let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

                    let mut initial = MaybeUninit::<ViewRect>::zeroed();
                    assert_eq!(
                        view.getSize(initial.as_mut_ptr()),
                        kResultOk,
                        "cycle {cycle}: get initial size"
                    );
                    let initial = initial.assume_init();
                    assert_eq!(initial.left, 0, "cycle {cycle}: initial left");
                    assert_eq!(initial.top, 0, "cycle {cycle}: initial top");
                    assert_eq!(initial.right, 777, "cycle {cycle}: initial width");
                    assert_eq!(initial.bottom, 333, "cycle {cycle}: initial height");
                    assert_eq!(view.canResize(), kResultTrue, "cycle {cycle}: resizable");

                    let left = (cycle % 11) as i32;
                    let top = (cycle % 7) as i32;
                    let requested_width = WIDTHS[cycle % WIDTHS.len()];
                    let requested_height = HEIGHTS[(cycle * 5) % HEIGHTS.len()];
                    let expected_width = requested_width.max(320);
                    let expected_height = requested_height.max(200);

                    let mut constrained = ViewRect {
                        left,
                        top,
                        right: left + requested_width,
                        bottom: top + requested_height,
                    };
                    assert_eq!(
                        view.checkSizeConstraint(&mut constrained),
                        kResultOk,
                        "cycle {cycle}: check size constraint"
                    );
                    assert_eq!(
                        constrained.right - constrained.left,
                        expected_width,
                        "cycle {cycle}: constrained width"
                    );
                    assert_eq!(
                        constrained.bottom - constrained.top,
                        expected_height,
                        "cycle {cycle}: constrained height"
                    );

                    let mut resized = ViewRect {
                        left,
                        top,
                        right: left + requested_width,
                        bottom: top + requested_height,
                    };
                    assert_eq!(
                        view.onSize(&mut resized),
                        kResultOk,
                        "cycle {cycle}: resize"
                    );

                    let mut current = MaybeUninit::<ViewRect>::zeroed();
                    assert_eq!(
                        view.getSize(current.as_mut_ptr()),
                        kResultOk,
                        "cycle {cycle}: get current size"
                    );
                    let current = current.assume_init();
                    assert_eq!(current.left, left, "cycle {cycle}: current left");
                    assert_eq!(current.top, top, "cycle {cycle}: current top");
                    assert_eq!(
                        current.right - current.left,
                        expected_width,
                        "cycle {cycle}: current width"
                    );
                    assert_eq!(
                        current.bottom - current.top,
                        expected_height,
                        "cycle {cycle}: current height"
                    );

                    assert_eq!(view.removed(), kResultOk, "cycle {cycle}: first remove");
                    assert_eq!(view.removed(), kResultOk, "cycle {cycle}: second remove");
                }
            }
        }

        #[cfg(feature = "wry-ui")]
        #[test]
        fn editor_attach_failure_is_traced() {
            // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
            unsafe {
                let temp = tempfile::tempdir().unwrap();
                let trace = temp.path().join("bridge-trace.log");
                std::env::set_var("VESTY_BRIDGE_TRACE", &trace);

                let controller = crate::bindings_impl::VestyController::<TestPlugin>::new();
                let view_ptr = controller.createView(c"editor".as_ptr() as *const c_char);
                let view = ComPtr::<IPlugView>::from_raw(view_ptr).expect("editor view");

                assert_eq!(
                    view.attached(
                        ptr::null_mut(),
                        c"unsupported-platform".as_ptr() as FIDString
                    ),
                    kResultFalse
                );

                std::env::remove_var("VESTY_BRIDGE_TRACE");
                let text = std::fs::read_to_string(trace).unwrap();
                assert!(text.contains("editor_attach_unsupported_platform"));
            }
        }
    }
}
