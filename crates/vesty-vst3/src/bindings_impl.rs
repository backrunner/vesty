#![allow(non_snake_case)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]
use crate::{FaultState, panic_guard};
use serde::{Deserialize, Serialize};
#[cfg(feature = "wry-ui")]
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_void};
#[cfg(feature = "wry-ui")]
use std::io::Write;
use std::marker::PhantomData;
use std::mem::{MaybeUninit, size_of};
use std::path::PathBuf;
#[cfg(feature = "wry-ui")]
use std::rc::Rc;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(feature = "wry-ui")]
use vesty_bridge::{
    BridgeRuntime, BridgeTransport, BridgeTransportError, ParamGesture, ParamGesturePhase,
};
use vesty_core::{
    AudioBuffers, AudioBuffers64, AudioKernel, AudioOutputBus, Event as VestyEvent,
    HostChangeFlags, KernelInit, MAX_AUDIO_OUTPUT_BUSES, MAX_AUDIO_OUTPUT_CHANNELS,
    MAX_NOTE_EXPRESSION_TEXT_UNITS, MAX_SYSEX_BYTES, NoteExpressionPhysicalUiMapping,
    NoteExpressionValueType, Plugin, PrepareContext, ProcessContext as VestyProcessContext,
    ProcessContext64 as VestyProcessContext64, ProcessMode, ProcessResult, ProgramAttribute,
    ProgramList, ProgramPitchName, SidechainBuffers, SidechainBuffers64, StateError, Transport,
    UiDescriptor,
};
#[cfg(feature = "wry-ui")]
use vesty_ipc::{
    BridgeCapabilities, BridgeErrorCode, BridgeErrorPayload, BridgeKind, BridgePacket,
    BridgeReadyPayload, ParamChangeSource, ParamValueSnapshot, PluginFaultReport, PluginSnapshot,
};
use vesty_params::{
    ParamCollection, format_normalized_value, normalized_to_plain, parse_normalized_value,
    plain_to_normalized, stable_vst3_param_id,
};
use vesty_rt::{
    FixedEventList, NoAllocGuard, RtLogConsumer, RtLogEvent, RtLogProducer, RtMeterConsumer,
    RtMeterProducer, log_spsc, meter_spsc,
};
#[cfg(feature = "wry-ui")]
use vesty_ui::{EditorRuntime, EditorSize};
#[cfg(feature = "wry-ui")]
use vesty_ui_wry::{NativeParent, WryEditorRuntime};
use vst3::{Class, ComPtr, ComRef, ComWrapper, Steinberg::Vst::*, Steinberg::*};

const MAX_BLOCK_EVENTS: usize = 512;
const MAX_SETUP_BLOCK_SIZE: usize = 1 << 20;
const MAX_MAIN_IO_CHANNELS: usize = 2;
const MAX_SIDECHAIN_CHANNELS: usize = 2;
const METER_QUEUE_CAPACITY: usize = 256;
const LOG_QUEUE_CAPACITY: usize = 256;
pub(crate) const RT_LOG_CODE_PROCESS_PANIC: u32 = 1;
pub(crate) const RT_LOG_CODE_PROCESS_WITHOUT_KERNEL: u32 = 2;
#[cfg(feature = "wry-ui")]
const MAIN_METER_TOPIC: &str = "meter.main";
#[cfg(feature = "wry-ui")]
const FAULT_DIAGNOSTICS_TOPIC: &str = "diagnostics.fault";
#[cfg(feature = "wry-ui")]
const RT_LOG_TOPIC: &str = "log.rt";
const STATE_MAGIC: &[u8] = b"VESTY_STATE_V1\n";
const VST3_STATE_VERSION: u32 = 1;
const PROGRAM_DATA_MAGIC: &[u8] = b"VESTY_PROGRAM_DATA_V1\n";
const VST3_PROGRAM_DATA_VERSION: u32 = 1;
const MAX_STREAM_BYTES: usize = 16 * 1024 * 1024;
const TELEMETRY_BIND_MESSAGE_ID: &[u8] = b"vesty.telemetry.bind\0";
const TELEMETRY_ID_ATTR: &[u8] = b"telemetryId\0";
const ROOT_UNIT_INDEX: int32 = 0;
const ROOT_UNIT_PROGRAM_LIST_INDEX: usize = 0;
#[allow(clippy::unnecessary_cast)]
const LEGACY_PITCH_BEND_CONTROL: u32 = ControllerNumbers_::kPitchBend as u32;
#[allow(clippy::unnecessary_cast)]
const LEGACY_AFTERTOUCH_CONTROL: u32 = ControllerNumbers_::kAfterTouch as u32;
#[allow(clippy::unnecessary_cast)]
pub(crate) const PROCESS_CONTEXT_PLAYING_FLAG: u32 =
    ProcessContext_::StatesAndFlags_::kPlaying as u32;
#[allow(clippy::unnecessary_cast)]
pub(crate) const PROCESS_CONTEXT_TEMPO_VALID_FLAG: u32 =
    ProcessContext_::StatesAndFlags_::kTempoValid as u32;
#[allow(clippy::unnecessary_cast)]
pub(crate) const DEFAULT_ACTIVE_BUS_FLAG: u32 = BusInfo_::BusFlags_::kDefaultActive as u32;
#[cfg(feature = "wry-ui")]
static NEXT_EDITOR_SESSION_ID: AtomicU64 = AtomicU64::new(1);

type SharedConnectionPoint = Mutex<Option<ComPtr<IConnectionPoint>>>;
type SharedMeterConsumer = Arc<Mutex<Option<RtMeterConsumer>>>;
type SharedLogConsumer = Arc<Mutex<Option<RtLogConsumer>>>;
type SharedFaultState = Arc<Mutex<Option<Arc<FaultState>>>>;
#[cfg(feature = "wry-ui")]
type SharedParamChangeQueue = Arc<Mutex<BTreeMap<String, QueuedParamChange>>>;
#[cfg(feature = "wry-ui")]
type SharedBridgeSnapshot = Arc<Mutex<SharedBridgeSnapshotState>>;

#[cfg(feature = "wry-ui")]
#[derive(Clone, Debug)]
struct QueuedParamChange {
    normalized: f64,
    source: ParamChangeSource,
}

#[cfg(feature = "wry-ui")]
#[derive(Clone, Debug, Default)]
struct SharedBridgeSnapshotState {
    snapshot: PluginSnapshot,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControllerParamApply {
    Param(HostChangeFlags),
    Program(HostChangeFlags),
}

impl ControllerParamApply {
    fn host_changes(self) -> HostChangeFlags {
        match self {
            Self::Param(changes) | Self::Program(changes) => changes,
        }
    }

    #[cfg(feature = "wry-ui")]
    fn is_program(self) -> bool {
        matches!(self, Self::Program(_))
    }
}

#[cfg(feature = "wry-ui")]
fn next_editor_session_id() -> String {
    let id = NEXT_EDITOR_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    format!("editor-{id}")
}

fn event_sample_offset(event: &VestyEvent) -> u32 {
    event.sample_offset()
}

fn sort_events_by_sample_offset(events: &mut FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>) {
    let items = events.as_mut_slice();
    for index in 1..items.len() {
        let mut cursor = index;
        while cursor > 0
            && event_sample_offset(&items[cursor - 1]) > event_sample_offset(&items[cursor])
        {
            items.swap(cursor - 1, cursor);
            cursor -= 1;
        }
    }
}

fn clamp_midi_channel_i16(channel: i16) -> u16 {
    channel.clamp(0, 15) as u16
}

fn clamp_midi_channel_i8(channel: i8) -> u16 {
    channel.clamp(0, 15) as u16
}

fn clamp_midi_key(key: i16) -> u8 {
    key.clamp(0, 127) as u8
}

fn clamp_midi7_i8(value: i8) -> u8 {
    value.clamp(0, 127) as u8
}

fn midi7_to_unit(value: u8) -> f32 {
    f32::from(value) / 127.0
}

fn midi_pitch_bend_to_bipolar(lsb: u8, msb: u8) -> f32 {
    let raw = u16::from(lsb.min(127)) | (u16::from(msb.min(127)) << 7);
    ((f32::from(raw) - 8192.0) / 8192.0).clamp(-1.0, 1.0)
}

unsafe fn copy_note_expression_text(
    text: *const TChar,
    text_len: u32,
) -> ([u16; MAX_NOTE_EXPRESSION_TEXT_UNITS], u8) {
    let mut out = [0_u16; MAX_NOTE_EXPRESSION_TEXT_UNITS];
    if text.is_null() || text_len == 0 {
        return (out, 0);
    }
    let len = (text_len as usize).min(MAX_NOTE_EXPRESSION_TEXT_UNITS);
    // SAFETY: The VST3 event declares `textLen` UTF-16 code units at `text`; the copy is clamped into Vesty's fixed-size realtime-safe buffer.
    let input = unsafe { slice::from_raw_parts(text, len) };
    out[..len].copy_from_slice(input);
    (out, len as u8)
}

unsafe fn copy_sysex_data(bytes: *const uint8, size: u32) -> ([u8; MAX_SYSEX_BYTES], u16, bool) {
    let mut out = [0_u8; MAX_SYSEX_BYTES];
    if bytes.is_null() || size == 0 {
        return (out, 0, size > 0);
    }
    let requested = size as usize;
    let len = requested.min(MAX_SYSEX_BYTES);
    // SAFETY: The VST3 data event declares `size` bytes at `bytes`; the copy is clamped into Vesty's fixed-size realtime-safe buffer before plugin code observes the event.
    let input = unsafe { slice::from_raw_parts(bytes, len) };
    out[..len].copy_from_slice(input);
    (out, len as u16, requested > MAX_SYSEX_BYTES)
}

#[derive(Default)]
pub(crate) struct Vst3TelemetryRegistry {
    next_id: AtomicU64,
    channels: Mutex<BTreeMap<u64, Vst3TelemetryChannel>>,
}

struct Vst3TelemetryChannel {
    meter_consumer: RtMeterConsumer,
    log_consumer: RtLogConsumer,
    fault: Arc<FaultState>,
}

impl Vst3TelemetryRegistry {
    fn create_channel(&self) -> (u64, RtMeterProducer, RtLogProducer, Arc<FaultState>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (meter_producer, meter_consumer) = meter_spsc(METER_QUEUE_CAPACITY);
        let (log_producer, log_consumer) = log_spsc(LOG_QUEUE_CAPACITY);
        let fault = Arc::new(FaultState::default());
        if let Ok(mut channels) = self.channels.lock() {
            channels.insert(
                id,
                Vst3TelemetryChannel {
                    meter_consumer,
                    log_consumer,
                    fault: fault.clone(),
                },
            );
        }
        (id, meter_producer, log_producer, fault)
    }

    fn take_channel(&self, id: u64) -> Option<Vst3TelemetryChannel> {
        self.channels.lock().ok()?.remove(&id)
    }

    #[cfg(feature = "wry-ui")]
    fn take_only_channel(&self) -> Option<Vst3TelemetryChannel> {
        let mut channels = self.channels.lock().ok()?;
        if channels.len() != 1 {
            return None;
        }
        let id = *channels.keys().next()?;
        channels.remove(&id)
    }

    fn remove_meter_consumer(&self, id: u64) {
        if let Ok(mut channels) = self.channels.lock() {
            channels.remove(&id);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Vst3State {
    version: u32,
    params: Vec<ParamState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    custom: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bridge: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ParamState {
    id: String,
    normalized: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Vst3ProgramData {
    version: u32,
    list_id: u32,
    program_index: usize,
    data: serde_json::Value,
}

#[derive(Default)]
struct VestyAttributeList {
    ints: Mutex<BTreeMap<String, int64>>,
    floats: Mutex<BTreeMap<String, f64>>,
    strings: Mutex<BTreeMap<String, Vec<TChar>>>,
    binaries: Mutex<BTreeMap<String, Vec<u8>>>,
}

// SAFETY: Attribute access is guarded by a mutex and is only used on non-realtime VST3 callback
// paths while binding processor/controller telemetry channels.
unsafe impl Sync for VestyAttributeList {}

impl VestyAttributeList {
    fn with_int(id: &[u8], value: int64) -> Self {
        let list = Self::default();
        if let Some(key) = attr_key(id.as_ptr() as IAttrID)
            && let Ok(mut ints) = list.ints.lock()
        {
            ints.insert(key, value);
        }
        list
    }
}

impl Class for VestyAttributeList {
    type Interfaces = (IAttributeList,);
}

#[vesty_macros::vst3_panic_boundary]
impl IAttributeListTrait for VestyAttributeList {
    unsafe fn setInt(&self, id: IAttrID, value: int64) -> tresult {
        let Some(key) = attr_key(id) else {
            return kInvalidArgument;
        };
        let Ok(mut ints) = self.ints.lock() else {
            return kResultFalse;
        };
        ints.insert(key, value);
        kResultOk
    }

    unsafe fn getInt(&self, id: IAttrID, value: *mut int64) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if value.is_null() {
                return kInvalidArgument;
            }
            let Some(key) = attr_key(id) else {
                return kInvalidArgument;
            };
            let Ok(ints) = self.ints.lock() else {
                return kResultFalse;
            };
            let Some(stored) = ints.get(&key).copied() else {
                return kInvalidArgument;
            };
            *value = stored;
            kResultOk
        }
    }

    unsafe fn setFloat(&self, id: IAttrID, value: f64) -> tresult {
        let Some(key) = attr_key(id) else {
            return kInvalidArgument;
        };
        let Ok(mut floats) = self.floats.lock() else {
            return kResultFalse;
        };
        floats.insert(key, value);
        kResultOk
    }

    unsafe fn getFloat(&self, id: IAttrID, value: *mut f64) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if value.is_null() {
                return kInvalidArgument;
            }
            let Some(key) = attr_key(id) else {
                return kInvalidArgument;
            };
            let Ok(floats) = self.floats.lock() else {
                return kResultFalse;
            };
            let Some(stored) = floats.get(&key).copied() else {
                return kInvalidArgument;
            };
            *value = stored;
            kResultOk
        }
    }

    unsafe fn setString(&self, id: IAttrID, string: *const TChar) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if string.is_null() {
                return kInvalidArgument;
            }
            let Some(key) = attr_key(id) else {
                return kInvalidArgument;
            };
            let len = len_wstring(string);
            let value = slice::from_raw_parts(string, len + 1).to_vec();
            let Ok(mut strings) = self.strings.lock() else {
                return kResultFalse;
            };
            strings.insert(key, value);
            kResultOk
        }
    }

    unsafe fn getString(&self, id: IAttrID, string: *mut TChar, size_in_bytes: uint32) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if string.is_null() {
                return kInvalidArgument;
            }
            let char_capacity = size_in_bytes as usize / size_of::<TChar>();
            if char_capacity == 0 {
                return kInvalidArgument;
            }
            let Some(key) = attr_key(id) else {
                return kInvalidArgument;
            };
            let Ok(strings) = self.strings.lock() else {
                return kResultFalse;
            };
            let Some(stored) = strings.get(&key) else {
                return kInvalidArgument;
            };
            let output = slice::from_raw_parts_mut(string, char_capacity);
            if stored.len() <= output.len() {
                output[..stored.len()].copy_from_slice(stored);
                kResultOk
            } else {
                let copy_len = output.len();
                output.copy_from_slice(&stored[..copy_len]);
                if let Some(last) = output.last_mut() {
                    *last = 0;
                }
                kResultFalse
            }
        }
    }

    unsafe fn setBinary(&self, id: IAttrID, data: *const c_void, size_in_bytes: uint32) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(key) = attr_key(id) else {
                return kInvalidArgument;
            };
            if size_in_bytes > 0 && data.is_null() {
                return kInvalidArgument;
            }
            let value = if size_in_bytes == 0 {
                Vec::new()
            } else {
                slice::from_raw_parts(data as *const u8, size_in_bytes as usize).to_vec()
            };
            let Ok(mut binaries) = self.binaries.lock() else {
                return kResultFalse;
            };
            binaries.insert(key, value);
            kResultOk
        }
    }

    unsafe fn getBinary(
        &self,
        id: IAttrID,
        data: *mut *const c_void,
        size_in_bytes: *mut uint32,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if data.is_null() || size_in_bytes.is_null() {
                return kInvalidArgument;
            }
            let Some(key) = attr_key(id) else {
                return kInvalidArgument;
            };
            let Ok(binaries) = self.binaries.lock() else {
                return kResultFalse;
            };
            let Some(stored) = binaries.get(&key) else {
                return kInvalidArgument;
            };
            *data = if stored.is_empty() {
                std::ptr::null()
            } else {
                stored.as_ptr() as *const c_void
            };
            *size_in_bytes = stored.len().try_into().unwrap_or(uint32::MAX);
            kResultOk
        }
    }
}

struct VestyMessage {
    message_id: Mutex<CString>,
    attributes: ComPtr<IAttributeList>,
}

// SAFETY: Message fields are immutable after construction except for the optional VST3
// setMessageID callback, which is mutex-protected and not used from the audio thread.
unsafe impl Sync for VestyMessage {}

impl VestyMessage {
    fn telemetry_bind(telemetry_id: u64) -> Option<Self> {
        let attributes = ComWrapper::new(VestyAttributeList::with_int(
            TELEMETRY_ID_ATTR,
            telemetry_id as int64,
        ))
        .to_com_ptr::<IAttributeList>()?;
        let message_id =
            CString::new(&TELEMETRY_BIND_MESSAGE_ID[..TELEMETRY_BIND_MESSAGE_ID.len() - 1]).ok()?;
        Some(Self {
            message_id: Mutex::new(message_id),
            attributes,
        })
    }
}

impl Class for VestyMessage {
    type Interfaces = (IMessage,);
}

#[vesty_macros::vst3_panic_boundary]
impl IMessageTrait for VestyMessage {
    unsafe fn getMessageID(&self) -> FIDString {
        self.message_id
            .lock()
            .map(|message_id| message_id.as_ptr())
            .unwrap_or(std::ptr::null())
    }

    unsafe fn setMessageID(&self, id: FIDString) {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if id.is_null() {
                return;
            }
            if let Ok(mut message_id) = self.message_id.lock() {
                *message_id = CStr::from_ptr(id).to_owned();
            }
        }
    }

    unsafe fn getAttributes(&self) -> *mut IAttributeList {
        self.attributes.as_ptr()
    }
}

fn attr_key(id: IAttrID) -> Option<String> {
    if id.is_null() {
        return None;
    }
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    Some(unsafe { CStr::from_ptr(id) }.to_string_lossy().into_owned())
}

pub mod raw {
    pub use vst3::Steinberg::IPluginFactory;
}

fn copy_cstring(src: &str, dst: &mut [c_char]) {
    let c_string = CString::new(src).unwrap_or_default();
    let bytes = c_string.as_bytes_with_nul();
    for (src, dst) in bytes.iter().zip(dst.iter_mut()) {
        *dst = *src as c_char;
    }
    if bytes.len() > dst.len()
        && let Some(last) = dst.last_mut()
    {
        *last = 0;
    }
}

fn copy_wstring(src: &str, dst: &mut [TChar]) {
    let mut len = 0;
    for (src, dst) in src.encode_utf16().zip(dst.iter_mut()) {
        *dst = src as TChar;
        len += 1;
    }
    if len < dst.len() {
        dst[len] = 0;
    } else if let Some(last) = dst.last_mut() {
        *last = 0;
    }
}

unsafe fn len_wstring(string: *const TChar) -> usize {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let mut len = 0;
        while *string.add(len) != 0 {
            len += 1;
        }
        len
    }
}

unsafe fn string128_to_string_lossy(string: *const TChar) -> Result<String, ()> {
    if string.is_null() {
        return Err(());
    }

    // SAFETY: VST3 String128 inputs are bounded to 128 UTF-16 code units. The scan stops at the
    // first NUL inside that bound or consumes the full fixed-size buffer.
    unsafe {
        let input = slice::from_raw_parts(string, 128);
        let len = input.iter().position(|unit| *unit == 0).unwrap_or(128);
        String::from_utf16(&input[..len]).map_err(|_| ())
    }
}

unsafe fn fid_eq(value: FIDString, expected: FIDString) -> bool {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        if value.is_null() || expected.is_null() {
            return false;
        }
        CStr::from_ptr(value).to_bytes() == CStr::from_ptr(expected).to_bytes()
    }
}

fn tuid_from_bytes(bytes: [u8; 16]) -> TUID {
    bytes.map(|byte| byte as c_char)
}

fn processor_cid<P: Plugin>() -> TUID {
    tuid_from_bytes(P::INFO.class_id)
}

fn controller_cid<P: Plugin>() -> TUID {
    let mut class_id = P::INFO.class_id;
    class_id[15] = class_id[15].wrapping_add(1);
    tuid_from_bytes(class_id)
}

pub(crate) fn capture_state<P: Plugin>(plugin: &P) -> Result<Vst3State, StateError> {
    capture_state_with_bridge(plugin, None)
}

pub(crate) fn capture_state_with_bridge<P: Plugin>(
    plugin: &P,
    bridge: Option<serde_json::Value>,
) -> Result<Vst3State, StateError> {
    let params = plugin.params();
    let values = params
        .specs()
        .into_iter()
        .filter_map(|spec| {
            params
                .get_normalized(&spec.id)
                .map(|normalized| ParamState {
                    id: spec.id,
                    normalized,
                })
        })
        .collect();
    Ok(Vst3State {
        version: VST3_STATE_VERSION,
        params: values,
        custom: plugin.save_custom_state()?,
        bridge,
    })
}

fn migrate_vst3_state(state: Vst3State) -> Result<Vst3State, StateError> {
    match state.version {
        VST3_STATE_VERSION => Ok(state),
        version => Err(StateError::Deserialize(format!(
            "unsupported VST3 state version: {version}"
        ))),
    }
}

pub(crate) fn apply_state<P: Plugin>(plugin: &P, state: Vst3State) -> Result<(), StateError> {
    let state = migrate_vst3_state(state)?;
    plugin.load_custom_state(state.custom)?;
    let params = plugin.params();
    for param in state.params {
        let _ = params.set_normalized(&param.id, param.normalized);
    }
    Ok(())
}

unsafe fn read_state_stream(stream: *mut IBStream) -> Result<Vst3State, ()> {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let bytes = read_stream_bytes(stream)?;
        let Some(json) = bytes.strip_prefix(STATE_MAGIC) else {
            return Err(());
        };
        let state = serde_json::from_slice(json).map_err(|_| ())?;
        migrate_vst3_state(state).map_err(|_| ())
    }
}

unsafe fn write_state_stream(stream: *mut IBStream, state: &Vst3State) -> Result<(), ()> {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let json = serde_json::to_vec(state).map_err(|_| ())?;
        let mut bytes = Vec::with_capacity(STATE_MAGIC.len() + json.len());
        bytes.extend_from_slice(STATE_MAGIC);
        bytes.extend_from_slice(&json);
        write_stream_bytes(stream, &bytes)
    }
}

unsafe fn read_program_data_stream(stream: *mut IBStream) -> Result<Vst3ProgramData, ()> {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let bytes = read_stream_bytes(stream)?;
        let Some(json) = bytes.strip_prefix(PROGRAM_DATA_MAGIC) else {
            return Err(());
        };
        let data: Vst3ProgramData = serde_json::from_slice(json).map_err(|_| ())?;
        if data.version != VST3_PROGRAM_DATA_VERSION {
            return Err(());
        }
        Ok(data)
    }
}

unsafe fn write_program_data_stream(
    stream: *mut IBStream,
    data: &Vst3ProgramData,
) -> Result<(), ()> {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let json = serde_json::to_vec(data).map_err(|_| ())?;
        let mut bytes = Vec::with_capacity(PROGRAM_DATA_MAGIC.len() + json.len());
        bytes.extend_from_slice(PROGRAM_DATA_MAGIC);
        bytes.extend_from_slice(&json);
        write_stream_bytes(stream, &bytes)
    }
}

unsafe fn read_stream_bytes(stream: *mut IBStream) -> Result<Vec<u8>, ()> {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let Some(stream) = ComRef::from_raw(stream) else {
            return Err(());
        };
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let mut read = 0;
            let result = stream.read(
                chunk.as_mut_ptr() as *mut c_void,
                chunk.len() as i32,
                &mut read,
            );
            if result != kResultOk {
                return Err(());
            }
            if read <= 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read as usize]);
            if bytes.len() > MAX_STREAM_BYTES {
                return Err(());
            }
        }
        Ok(bytes)
    }
}

unsafe fn write_stream_bytes(stream: *mut IBStream, bytes: &[u8]) -> Result<(), ()> {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let Some(stream) = ComRef::from_raw(stream) else {
            return Err(());
        };
        let mut offset = 0;
        while offset < bytes.len() {
            let remaining = bytes.len() - offset;
            let len = remaining.min(i32::MAX as usize) as i32;
            let mut written = 0;
            let result = stream.write(bytes[offset..].as_ptr() as *mut c_void, len, &mut written);
            if result != kResultOk || written <= 0 {
                return Err(());
            }
            offset += written as usize;
        }
        Ok(())
    }
}

mod controller;
mod factory;
mod processor;

pub(crate) use controller::*;
pub use factory::create_plugin_factory;
pub(crate) use processor::*;

#[cfg(test)]
mod tests;
