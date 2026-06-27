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
    BridgeCapabilities, BridgeErrorCode, BridgeErrorPayload, BridgePacket, BridgeReadyPayload,
    ParamChangeSource, PluginFaultReport, PluginSnapshot,
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

pub(crate) struct VestyProcessor<P: Plugin + Default> {
    plugin: P,
    kernel: UnsafeCell<Option<P::Kernel>>,
    events: UnsafeCell<FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>>,
    meter_producer: UnsafeCell<RtMeterProducer>,
    log_producer: UnsafeCell<RtLogProducer>,
    telemetry_id: u64,
    telemetry_registry: Arc<Vst3TelemetryRegistry>,
    param_ids: Vec<String>,
    vst3_param_ids: Vst3ParamIds,
    sample_rate_bits: AtomicU64,
    max_block_size: AtomicUsize,
    input_arrangement: AtomicU64,
    sidechain_arrangement: AtomicU64,
    output_arrangements: [AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    input_bus_active: AtomicBool,
    sidechain_bus_active: AtomicBool,
    output_bus_active: [AtomicBool; MAX_AUDIO_OUTPUT_BUSES],
    event_input_bus_active: AtomicBool,
    io_mode: AtomicI32,
    processing_active: AtomicBool,
    sample64_scratch: UnsafeCell<Sample64Scratch>,
    connection: SharedConnectionPoint,
    fault: Arc<FaultState>,
}

// SAFETY: VST3 hosts call process lifecycle methods according to the component contract. The
// kernel is stored in UnsafeCell so process can mutate DSP state through &self without taking a
// lock. Concurrent process calls for the same component instance are not supported by VST3 hosts.
unsafe impl<P: Plugin + Default> Sync for VestyProcessor<P> {}

fn default_input_arrangement(kind: vesty_core::PluginKind) -> SpeakerArrangement {
    if kind == vesty_core::PluginKind::Instrument {
        0
    } else {
        SpeakerArr::kStereo
    }
}

fn supports_sidechain<P: Plugin>(plugin: &P) -> bool {
    P::INFO.kind != vesty_core::PluginKind::Instrument && plugin.sidechain_inputs() > 0
}

fn default_sidechain_arrangement<P: Plugin>(plugin: &P) -> SpeakerArrangement {
    if supports_sidechain(plugin) {
        SpeakerArr::kStereo
    } else {
        0
    }
}

fn declared_input_bus_count<P: Plugin>(plugin: &P) -> usize {
    if P::INFO.kind == vesty_core::PluginKind::Instrument {
        0
    } else if supports_sidechain(plugin) {
        2
    } else {
        1
    }
}

fn output_bus_count<P: Plugin>(plugin: &P) -> usize {
    let count = plugin
        .output_buses()
        .iter()
        .filter(|bus| bus.is_valid())
        .take(MAX_AUDIO_OUTPUT_BUSES)
        .count();
    count.max(1)
}

fn output_bus_at<P: Plugin>(plugin: &P, index: usize) -> Option<AudioOutputBus> {
    plugin
        .output_buses()
        .iter()
        .copied()
        .filter(AudioOutputBus::is_valid)
        .take(MAX_AUDIO_OUTPUT_BUSES)
        .nth(index)
        .or_else(|| (index == 0).then_some(vesty_core::DEFAULT_AUDIO_OUTPUT_BUSES[0]))
}

fn output_bus_arrangement(bus: AudioOutputBus) -> SpeakerArrangement {
    match bus.channels {
        1 => SpeakerArr::kMono,
        2 => SpeakerArr::kStereo,
        _ => SpeakerArr::kStereo,
    }
}

fn is_supported_output_bus_arrangement(
    bus: AudioOutputBus,
    arrangement: SpeakerArrangement,
) -> bool {
    matches!(
        (bus.channels, arrangement),
        (1, SpeakerArr::kMono) | (2, SpeakerArr::kStereo)
    )
}

fn arrangement_channel_count(arrangement: SpeakerArrangement) -> Option<i32> {
    match arrangement {
        SpeakerArr::kMono => Some(1),
        SpeakerArr::kStereo => Some(2),
        _ => None,
    }
}

fn is_supported_effect_arrangement(input: SpeakerArrangement, output: SpeakerArrangement) -> bool {
    matches!(
        (input, output),
        (SpeakerArr::kMono, SpeakerArr::kMono)
            | (SpeakerArr::kMono, SpeakerArr::kStereo)
            | (SpeakerArr::kStereo, SpeakerArr::kStereo)
    )
}

fn is_supported_sidechain_arrangement(arrangement: SpeakerArrangement) -> bool {
    matches!(arrangement, SpeakerArr::kMono | SpeakerArr::kStereo)
}

fn is_valid_bus_index<P: Plugin>(
    plugin: &P,
    media_type: MediaType,
    dir: BusDirection,
    index: i32,
) -> bool {
    if index < 0 {
        return false;
    }

    match (media_type as MediaTypes, dir as BusDirections) {
        (MediaTypes_::kAudio, BusDirections_::kInput) => {
            P::INFO.kind != vesty_core::PluginKind::Instrument
                && (index == 0 || (index == 1 && supports_sidechain(plugin)))
        }
        (MediaTypes_::kAudio, BusDirections_::kOutput) => {
            (index as usize) < output_bus_count(plugin)
        }
        (MediaTypes_::kEvent, BusDirections_::kInput) => {
            P::INFO.kind == vesty_core::PluginKind::Instrument && index == 0
        }
        _ => false,
    }
}

fn validate_output_arrangements<P: Plugin>(
    plugin: &P,
    outputs: &[SpeakerArrangement],
    main_input: Option<SpeakerArrangement>,
) -> bool {
    if outputs.len() != output_bus_count(plugin) {
        return false;
    }

    for (index, output) in outputs.iter().copied().enumerate() {
        let Some(bus) = output_bus_at(plugin, index) else {
            return false;
        };
        if index == 0 && P::INFO.kind == vesty_core::PluginKind::AudioEffect {
            let Some(input) = main_input else {
                return false;
            };
            if !is_supported_effect_arrangement(input, output) {
                return false;
            }
        } else if !is_supported_output_bus_arrangement(bus, output) {
            return false;
        }
    }
    true
}

fn visible_program_lists<P: Plugin>(plugin: &P) -> impl Iterator<Item = &'static ProgramList> {
    plugin
        .program_lists()
        .iter()
        .filter(|list| !list.is_empty())
}

fn program_list_by_index<P: Plugin>(plugin: &P, index: int32) -> Option<&'static ProgramList> {
    (index >= 0)
        .then_some(index as usize)
        .and_then(|index| visible_program_lists(plugin).nth(index))
}

fn program_list_by_id<P: Plugin>(plugin: &P, id: ProgramListID) -> Option<&'static ProgramList> {
    visible_program_lists(plugin).find(|list| list.id as ProgramListID == id)
}

fn program_list_by_id_or_root<P: Plugin>(
    plugin: &P,
    list_or_unit_id: int32,
) -> Option<&'static ProgramList> {
    program_list_by_id(plugin, list_or_unit_id).or_else(|| {
        (list_or_unit_id == kRootUnitId)
            .then(|| program_list_by_index(plugin, ROOT_UNIT_PROGRAM_LIST_INDEX as int32))?
    })
}

fn program_selection_by_id<P: Plugin>(
    plugin: &P,
    list_id: ProgramListID,
    program_index: int32,
) -> Option<(u32, usize)> {
    if program_index < 0 {
        return None;
    }
    let list = program_list_by_id(plugin, list_id)?;
    let program_index = program_index as usize;
    list.programs.get(program_index)?;
    Some((list.id, program_index))
}

fn program_selection_by_id_or_root<P: Plugin>(
    plugin: &P,
    list_or_unit_id: int32,
    program_index: int32,
) -> Option<(u32, usize)> {
    if program_index < 0 {
        return None;
    }
    let list = program_list_by_id_or_root(plugin, list_or_unit_id)?;
    let program_index = program_index as usize;
    list.programs.get(program_index)?;
    Some((list.id, program_index))
}

fn program_selection_for_param_value<P: Plugin>(
    plugin: &P,
    spec: &vesty_params::ParamSpec,
    normalized: f64,
) -> Option<(u32, usize)> {
    if !spec.flags.program_change {
        return None;
    }
    let list = visible_program_lists(plugin).next()?;
    let plain = normalized_to_plain(spec, normalized.clamp(0.0, 1.0));
    if !plain.is_finite() {
        return None;
    }
    let program_index = plain.round();
    if program_index < 0.0 || program_index > usize::MAX as f64 {
        return None;
    }
    let program_index = program_index as usize;
    list.programs.get(program_index)?;
    Some((list.id, program_index))
}

fn visible_program_attributes<P: Plugin>(
    plugin: &P,
    list_id: u32,
    program_index: usize,
) -> impl Iterator<Item = &'static ProgramAttribute> {
    plugin
        .program_attributes(list_id, program_index)
        .iter()
        .filter(|attribute| attribute.is_valid())
}

fn visible_program_pitch_names<P: Plugin>(
    plugin: &P,
    list_id: u32,
    program_index: usize,
) -> impl Iterator<Item = &'static ProgramPitchName> {
    plugin
        .program_pitch_names(list_id, program_index)
        .iter()
        .filter(|pitch| pitch.is_valid())
}

fn visible_note_expression_value_types<P: Plugin>(
    plugin: &P,
) -> impl Iterator<Item = &'static NoteExpressionValueType> {
    plugin
        .note_expression_value_types()
        .iter()
        .filter(|expression| expression.is_valid())
}

fn note_expression_value_type_by_index<P: Plugin>(
    plugin: &P,
    index: int32,
) -> Option<&'static NoteExpressionValueType> {
    (index >= 0)
        .then_some(index as usize)
        .and_then(|index| visible_note_expression_value_types(plugin).nth(index))
}

fn note_expression_value_type_by_id<P: Plugin>(
    plugin: &P,
    id: NoteExpressionTypeID,
) -> Option<&'static NoteExpressionValueType> {
    visible_note_expression_value_types(plugin).find(|expression| expression.type_id == id)
}

fn visible_note_expression_physical_ui_mappings<P: Plugin>(
    plugin: &P,
) -> impl Iterator<Item = &'static NoteExpressionPhysicalUiMapping> {
    plugin
        .note_expression_physical_ui_mappings()
        .iter()
        .filter(|mapping| {
            mapping.is_valid()
                && note_expression_value_type_by_id(plugin, mapping.note_expression_type_id)
                    .is_some()
        })
}

fn note_expression_bus_channel_valid<P: Plugin>(bus_index: int32, channel: int16) -> bool {
    P::INFO.kind == vesty_core::PluginKind::Instrument
        && bus_index == 0
        && (-1..=15).contains(&channel)
}

fn note_expression_type_flags(flags: vesty_core::NoteExpressionValueFlags) -> int32 {
    let mut raw = 0;
    if flags.bipolar {
        raw |= NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsBipolar as int32;
    }
    if flags.one_shot {
        raw |= NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsOneShot as int32;
    }
    if flags.absolute {
        raw |= NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsAbsolute as int32;
    }
    raw
}

fn silence_flags_for_channel_count(channel_count: usize) -> u64 {
    match channel_count {
        0 => 0,
        1..64 => (1_u64 << channel_count) - 1,
        _ => u64::MAX,
    }
}

fn setup_block_size(setup: &ProcessSetup) -> Option<usize> {
    if setup.maxSamplesPerBlock <= 0 {
        return None;
    }
    let frames = setup.maxSamplesPerBlock as usize;
    (frames <= MAX_SETUP_BLOCK_SIZE).then_some(frames)
}

fn setup_sample_rate(setup: &ProcessSetup) -> Option<f64> {
    (setup.sampleRate.is_finite() && setup.sampleRate > 0.0).then_some(setup.sampleRate)
}

fn setup_sample_size_supported(setup: &ProcessSetup) -> bool {
    matches!(
        setup.symbolicSampleSize as SymbolicSampleSizes,
        SymbolicSampleSizes_::kSample32 | SymbolicSampleSizes_::kSample64
    )
}

fn process_block_frames(process_data: &ProcessData) -> Result<usize, usize> {
    if process_data.numSamples < 0 {
        return Err(0);
    }

    Ok(process_data.numSamples as usize)
}

#[derive(Clone, Copy)]
struct ProcessOutputLayout<T> {
    channels: [*mut T; MAX_AUDIO_OUTPUT_CHANNELS],
    bus_channels: [usize; MAX_AUDIO_OUTPUT_BUSES],
    bus_count: usize,
    channel_count: usize,
}

impl<T> ProcessOutputLayout<T> {
    fn new() -> Self {
        Self {
            channels: [std::ptr::null_mut(); MAX_AUDIO_OUTPUT_CHANNELS],
            bus_channels: [0; MAX_AUDIO_OUTPUT_BUSES],
            bus_count: 0,
            channel_count: 0,
        }
    }
}

fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    // SAFETY: `[MaybeUninit<T>; N]` may be left uninitialized; callers only read the prefix they
    // explicitly initialized before constructing slices from this stack storage.
    unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }
}

#[derive(Default)]
struct Sample64Scratch {
    inputs: [Vec<f32>; MAX_MAIN_IO_CHANNELS],
    sidechain: [Vec<f32>; MAX_SIDECHAIN_CHANNELS],
    outputs: [Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    capacity: usize,
}

impl Sample64Scratch {
    fn prepare(&mut self, frames: usize) {
        self.capacity = frames;
        for channel in self
            .inputs
            .iter_mut()
            .chain(self.sidechain.iter_mut())
            .chain(self.outputs.iter_mut())
        {
            channel.resize(frames, 0.0);
        }
    }

    fn has_capacity(&self, frames: usize) -> bool {
        frames <= self.capacity
    }
}

unsafe fn copy_f64_to_f32(src: *const Sample64, dst: &mut [f32]) {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let src = slice::from_raw_parts(src, dst.len());
        for (src, dst) in src.iter().zip(dst.iter_mut()) {
            *dst = *src as f32;
        }
    }
}

unsafe fn copy_f32_to_f64(src: &[f32], dst: *mut Sample64) {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let dst = slice::from_raw_parts_mut(dst, src.len());
        for (src, dst) in src.iter().zip(dst.iter_mut()) {
            *dst = f64::from(*src);
        }
    }
}

unsafe fn audio_bus_channels32<'a>(
    bus: &AudioBusBuffers,
    expected_channels: usize,
) -> Option<&'a [*mut Sample32]> {
    // SAFETY: The caller is inside the VST3 process callback and `bus` comes from the host-provided `ProcessData`; null and channel-count checks guard the raw slice creation.
    unsafe {
        if bus.numChannels <= 0
            || bus.numChannels as usize != expected_channels
            || bus.__field0.channelBuffers32.is_null()
        {
            return None;
        }
        // SAFETY: VST3 provides `expected_channels` pointers for the selected sample-size buffer
        // union; the host count was checked before constructing this slice.
        Some(slice::from_raw_parts(
            bus.__field0.channelBuffers32,
            expected_channels,
        ))
    }
}

unsafe fn valid_audio_bus_channels32<'a>(
    bus: &AudioBusBuffers,
    max_channels: usize,
) -> Option<&'a [*mut Sample32]> {
    // SAFETY: Forwarding the caller's host callback contract; this helper immediately bounds and null-checks the returned channel list.
    let channel_count = bus.numChannels as usize;
    if bus.numChannels <= 0 || channel_count > max_channels {
        return None;
    }
    // SAFETY: `channel_count` was checked to be positive and within the fixed realtime input
    // limit before using the host-provided channel pointer array.
    let channels = unsafe { audio_bus_channels32(bus, channel_count)? };
    if channels.iter().any(|channel| channel.is_null()) {
        return None;
    }
    Some(channels)
}

unsafe fn audio_bus_channels64<'a>(
    bus: &AudioBusBuffers,
    expected_channels: usize,
) -> Option<&'a [*mut Sample64]> {
    // SAFETY: The caller is inside the VST3 process callback and `bus` comes from the host-provided `ProcessData`; null and channel-count checks guard the raw slice creation.
    unsafe {
        if bus.numChannels <= 0
            || bus.numChannels as usize != expected_channels
            || bus.__field0.channelBuffers64.is_null()
        {
            return None;
        }
        // SAFETY: VST3 provides `expected_channels` pointers for the selected sample-size buffer
        // union; the host count was checked before constructing this slice.
        Some(slice::from_raw_parts(
            bus.__field0.channelBuffers64,
            expected_channels,
        ))
    }
}

unsafe fn valid_audio_bus_channels64<'a>(
    bus: &AudioBusBuffers,
    max_channels: usize,
) -> Option<&'a [*mut Sample64]> {
    // SAFETY: Forwarding the caller's host callback contract; this helper immediately bounds and null-checks the returned channel list.
    let channel_count = bus.numChannels as usize;
    if bus.numChannels <= 0 || channel_count > max_channels {
        return None;
    }
    // SAFETY: `channel_count` was checked to be positive and within the fixed realtime input
    // limit before using the host-provided channel pointer array.
    let channels = unsafe { audio_bus_channels64(bus, channel_count)? };
    if channels.iter().any(|channel| channel.is_null()) {
        return None;
    }
    Some(channels)
}

unsafe fn input_buses<'a, P: Plugin>(
    plugin: &P,
    process_data: &ProcessData,
) -> Option<&'a [AudioBusBuffers]> {
    if process_data.numInputs == 0 {
        return Some(&[]);
    }
    if process_data.numInputs < 0
        || process_data.inputs.is_null()
        || process_data.numInputs as usize > declared_input_bus_count::<P>(plugin)
    {
        return None;
    }

    // SAFETY: The host-provided input bus count is non-negative, non-zero, bounded by the
    // plugin-declared input bus count, and `inputs` is non-null for this process callback.
    Some(unsafe { slice::from_raw_parts(process_data.inputs, process_data.numInputs as usize) })
}

unsafe fn output_layout32<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
) -> Option<ProcessOutputLayout<Sample32>> {
    // SAFETY: The host owns `ProcessData.outputs` for the duration of the process callback. This
    // helper only copies channel pointers into a fixed-capacity stack layout after bus prefix and
    // channel-count validation against the plugin descriptor.
    unsafe {
        let expected_bus_count = output_bus_count(plugin);
        if process_data.outputs.is_null()
            || process_data.numOutputs <= 0
            || process_data.numOutputs > expected_bus_count as i32
        {
            return None;
        }

        let host_bus_count = process_data.numOutputs as usize;
        let output_buses = slice::from_raw_parts(process_data.outputs, host_bus_count);
        let mut layout = ProcessOutputLayout::new();
        layout.bus_count = host_bus_count;
        for (bus_index, bus) in output_buses.iter().enumerate() {
            if bus.numChannels <= 0 {
                if bus_index == 0 {
                    return None;
                }
                continue;
            }

            let declared = if bus_index == 0 && P::INFO.kind == vesty_core::PluginKind::AudioEffect
            {
                match bus.numChannels {
                    1 | 2 => bus.numChannels as usize,
                    _ => return None,
                }
            } else {
                arrangement_channel_count(output_arrangements[bus_index].load(Ordering::Relaxed))
                    .map(|count| count as usize)
                    .or_else(|| output_bus_at(plugin, bus_index).map(|bus| bus.channels as usize))?
            };
            let channels = audio_bus_channels32(bus, declared)?;
            if layout.channel_count + declared > MAX_AUDIO_OUTPUT_CHANNELS
                || channels[..declared].iter().any(|channel| channel.is_null())
            {
                return None;
            }

            layout.bus_channels[bus_index] = declared;
            for channel in channels.iter().take(declared) {
                layout.channels[layout.channel_count] = *channel;
                layout.channel_count += 1;
            }
        }
        Some(layout)
    }
}

unsafe fn output_layout64<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
) -> Option<ProcessOutputLayout<Sample64>> {
    // SAFETY: The host owns `ProcessData.outputs` for the duration of the process callback. This
    // helper only copies channel pointers into a fixed-capacity stack layout after bus prefix and
    // channel-count validation against the plugin descriptor.
    unsafe {
        let expected_bus_count = output_bus_count(plugin);
        if process_data.outputs.is_null()
            || process_data.numOutputs <= 0
            || process_data.numOutputs > expected_bus_count as i32
        {
            return None;
        }

        let host_bus_count = process_data.numOutputs as usize;
        let output_buses = slice::from_raw_parts(process_data.outputs, host_bus_count);
        let mut layout = ProcessOutputLayout::new();
        layout.bus_count = host_bus_count;
        for (bus_index, bus) in output_buses.iter().enumerate() {
            if bus.numChannels <= 0 {
                if bus_index == 0 {
                    return None;
                }
                continue;
            }

            let declared = if bus_index == 0 && P::INFO.kind == vesty_core::PluginKind::AudioEffect
            {
                match bus.numChannels {
                    1 | 2 => bus.numChannels as usize,
                    _ => return None,
                }
            } else {
                arrangement_channel_count(output_arrangements[bus_index].load(Ordering::Relaxed))
                    .map(|count| count as usize)
                    .or_else(|| output_bus_at(plugin, bus_index).map(|bus| bus.channels as usize))?
            };
            let channels = audio_bus_channels64(bus, declared)?;
            if layout.channel_count + declared > MAX_AUDIO_OUTPUT_CHANNELS
                || channels[..declared].iter().any(|channel| channel.is_null())
            {
                return None;
            }

            layout.bus_channels[bus_index] = declared;
            for channel in channels.iter().take(declared) {
                layout.channels[layout.channel_count] = *channel;
                layout.channel_count += 1;
            }
        }
        Some(layout)
    }
}

unsafe fn input_views32<'a, const N: usize>(
    channels: Option<&[*mut Sample32]>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a [f32]>; N],
) -> &'a [&'a [f32]] {
    let Some(channels) = channels else {
        return &[];
    };
    let count = channels.len().min(N);
    // SAFETY: Channel pointers were null-checked by `valid_audio_bus_channels32`; each view is
    // bounded to the host-provided block size and stored in caller-owned stack storage.
    unsafe {
        for (index, channel) in channels.iter().take(count).enumerate() {
            storage[index].write(slice::from_raw_parts(*channel, frames));
        }
        slice::from_raw_parts(storage.as_ptr() as *const &'a [f32], count)
    }
}

unsafe fn input_views64<'a, const N: usize>(
    channels: Option<&[*mut Sample64]>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a [f64]>; N],
) -> &'a [&'a [f64]] {
    let Some(channels) = channels else {
        return &[];
    };
    let count = channels.len().min(N);
    // SAFETY: Channel pointers were null-checked by `valid_audio_bus_channels64`; each view is
    // bounded to the host-provided block size and stored in caller-owned stack storage.
    unsafe {
        for (index, channel) in channels.iter().take(count).enumerate() {
            storage[index].write(slice::from_raw_parts(*channel, frames));
        }
        slice::from_raw_parts(storage.as_ptr() as *const &'a [f64], count)
    }
}

unsafe fn output_views32<'a>(
    layout: &ProcessOutputLayout<Sample32>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS],
) -> &'a mut [&'a mut [f32]] {
    // SAFETY: `layout` contains distinct, non-null output channel pointers validated from the host
    // bus list. Each channel is converted exactly once into a mutable slice for the process block.
    unsafe {
        for (index, slot) in storage.iter_mut().enumerate().take(layout.channel_count) {
            slot.write(slice::from_raw_parts_mut(layout.channels[index], frames));
        }
        slice::from_raw_parts_mut(
            storage.as_mut_ptr() as *mut &'a mut [f32],
            layout.channel_count,
        )
    }
}

unsafe fn output_views64<'a>(
    layout: &ProcessOutputLayout<Sample64>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a mut [f64]>; MAX_AUDIO_OUTPUT_CHANNELS],
) -> &'a mut [&'a mut [f64]] {
    // SAFETY: `layout` contains distinct, non-null output channel pointers validated from the host
    // bus list. Each channel is converted exactly once into a mutable slice for the process block.
    unsafe {
        for (index, slot) in storage.iter_mut().enumerate().take(layout.channel_count) {
            slot.write(slice::from_raw_parts_mut(layout.channels[index], frames));
        }
        slice::from_raw_parts_mut(
            storage.as_mut_ptr() as *mut &'a mut [f64],
            layout.channel_count,
        )
    }
}

unsafe fn scratch_input_views_from_f64<'a, const N: usize>(
    scratch: &'a mut [Vec<f32>; N],
    channels: Option<&[*mut Sample64]>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a [f32]>; N],
) -> &'a [&'a [f32]] {
    let Some(channels) = channels else {
        return &[];
    };
    let count = channels.len().min(N);
    // SAFETY: Source channel pointers were null-checked by `valid_audio_bus_channels64`; scratch
    // capacity is validated before this helper is called.
    unsafe {
        let base = scratch.as_mut_ptr();
        for (index, channel) in channels.iter().take(count).enumerate() {
            let scratch_channel = &mut *base.add(index);
            copy_f64_to_f32(*channel, &mut scratch_channel[..frames]);
            storage[index].write(&scratch_channel[..frames]);
        }
        slice::from_raw_parts(storage.as_ptr() as *const &'a [f32], count)
    }
}

unsafe fn scratch_output_views<'a>(
    scratch: &'a mut [Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    frames: usize,
    channel_count: usize,
    storage: &'a mut [MaybeUninit<&'a mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS],
) -> &'a mut [&'a mut [f32]] {
    // SAFETY: The first `channel_count` scratch channels are unique Vecs prepared to `frames`
    // capacity before processing; raw indexing avoids holding overlapping borrows of the array.
    unsafe {
        let base = scratch.as_mut_ptr();
        for (index, slot) in storage.iter_mut().enumerate().take(channel_count) {
            let channel = &mut *base.add(index);
            slot.write(&mut channel[..frames]);
        }
        slice::from_raw_parts_mut(storage.as_mut_ptr() as *mut &'a mut [f32], channel_count)
    }
}

unsafe fn set_output_silence_flags<T>(
    process_data: &ProcessData,
    layout: &ProcessOutputLayout<T>,
    silent: bool,
) {
    // SAFETY: `layout.bus_count` was derived from and bounded by the host output bus slice for this
    // process call. This only mutates per-bus flags after all channel slices have been dropped.
    unsafe {
        if process_data.outputs.is_null() {
            return;
        }
        let output_buses = slice::from_raw_parts_mut(process_data.outputs, layout.bus_count);
        for (bus_index, bus) in output_buses.iter_mut().enumerate() {
            bus.silenceFlags = if silent {
                silence_flags_for_channel_count(layout.bus_channels[bus_index])
            } else {
                0
            };
        }
    }
}

unsafe fn silence_process_outputs32<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
    frames: usize,
) {
    // SAFETY: Reuses the same host output layout validation as normal processing before mutating
    // output buffers and silence flags inside the process callback.
    unsafe {
        let Some(output_layout) = output_layout32(plugin, output_arrangements, process_data) else {
            return;
        };
        for channel in output_layout
            .channels
            .iter()
            .take(output_layout.channel_count)
        {
            slice::from_raw_parts_mut(*channel, frames).fill(0.0);
        }
        set_output_silence_flags(process_data, &output_layout, true);
    }
}

unsafe fn silence_process_outputs64<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
    frames: usize,
) {
    // SAFETY: Reuses the same host output layout validation as normal processing before mutating
    // output buffers and silence flags inside the process callback.
    unsafe {
        let Some(output_layout) = output_layout64(plugin, output_arrangements, process_data) else {
            return;
        };
        clear_output_layout64(&output_layout, frames);
        set_output_silence_flags(process_data, &output_layout, true);
    }
}

unsafe fn clear_output_layout64(layout: &ProcessOutputLayout<Sample64>, frames: usize) {
    // SAFETY: `layout` contains non-null output channel pointers validated from the host bus list.
    unsafe {
        for channel in layout.channels.iter().take(layout.channel_count) {
            slice::from_raw_parts_mut(*channel, frames).fill(0.0);
        }
    }
}

unsafe fn copy_output_layout64_to_scratch(
    layout: &ProcessOutputLayout<Sample64>,
    scratch: &mut [Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    frames: usize,
) {
    // SAFETY: `layout` contains non-null output channel pointers and scratch capacity was checked
    // before entering the realtime copy.
    unsafe {
        for (index, channel) in layout
            .channels
            .iter()
            .take(layout.channel_count)
            .enumerate()
        {
            copy_f64_to_f32(*channel, &mut scratch[index][..frames]);
        }
    }
}

unsafe fn copy_scratch_to_output_layout64(
    scratch: &[Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    layout: &ProcessOutputLayout<Sample64>,
    frames: usize,
) {
    // SAFETY: `layout` contains non-null output channel pointers and scratch capacity was checked
    // before entering the realtime copy.
    unsafe {
        for (index, channel) in layout
            .channels
            .iter()
            .take(layout.channel_count)
            .enumerate()
        {
            copy_f32_to_f64(&scratch[index][..frames], *channel);
        }
    }
}

fn restart_flags_for_host_changes(changes: HostChangeFlags) -> int32 {
    let mut flags = 0;
    if changes.contains(HostChangeFlags::IO) {
        flags |= RestartFlags_::kIoChanged;
    }
    if changes.contains(HostChangeFlags::PARAM_VALUES) {
        flags |= RestartFlags_::kParamValuesChanged;
    }
    if changes.contains(HostChangeFlags::LATENCY) {
        flags |= RestartFlags_::kLatencyChanged;
    }
    if changes.contains(HostChangeFlags::PARAM_TITLES) {
        flags |= RestartFlags_::kParamTitlesChanged;
    }
    flags
}

#[derive(Clone, Debug, Default)]
struct Vst3ParamIds {
    host_ids: Vec<ParamID>,
    by_host_id: BTreeMap<ParamID, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Vst3ParamIdCollision {
    host_id: ParamID,
    first_id: String,
    second_id: String,
}

impl Vst3ParamIds {
    fn try_from_specs(specs: &[vesty_params::ParamSpec]) -> Result<Self, Vst3ParamIdCollision> {
        let mut host_ids = Vec::with_capacity(specs.len());
        let mut by_host_id: BTreeMap<ParamID, usize> = BTreeMap::new();
        for (index, spec) in specs.iter().enumerate() {
            let host_id = stable_vst3_param_id(&spec.id);
            host_ids.push(host_id);
            if let Some(first_index) = by_host_id.get(&host_id).copied() {
                return Err(Vst3ParamIdCollision {
                    host_id,
                    first_id: specs[first_index].id.clone(),
                    second_id: spec.id.clone(),
                });
            } else {
                by_host_id.insert(host_id, index);
            }
        }
        Ok(Self {
            host_ids,
            by_host_id,
        })
    }

    fn host_id_for_index(&self, index: usize) -> Option<ParamID> {
        self.host_ids.get(index).copied()
    }

    fn index_for_host_id(&self, host_id: ParamID) -> Option<usize> {
        self.by_host_id.get(&host_id).copied()
    }
}

unsafe fn restart_component_for_host_changes(
    handler: &ComPtr<IComponentHandler>,
    changes: HostChangeFlags,
) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let flags = restart_flags_for_host_changes(changes);
        if flags == 0 {
            kResultOk
        } else {
            handler.restartComponent(flags)
        }
    }
}

impl<P: Plugin + Default> Class for VestyProcessor<P> {
    type Interfaces = (
        IComponent,
        IAudioProcessor,
        IProcessContextRequirements,
        IConnectionPoint,
    );
}

#[derive(Debug)]
pub(crate) enum VestyProcessorInitError {
    ParamSchema,
    ParamIdCollision,
}

impl From<vesty_params::ParamSpecError> for VestyProcessorInitError {
    fn from(_error: vesty_params::ParamSpecError) -> Self {
        Self::ParamSchema
    }
}

impl From<Vst3ParamIdCollision> for VestyProcessorInitError {
    fn from(_error: Vst3ParamIdCollision) -> Self {
        Self::ParamIdCollision
    }
}

impl<P: Plugin + Default> VestyProcessor<P> {
    #[cfg(test)]
    pub(crate) fn with_telemetry_registry(telemetry_registry: Arc<Vst3TelemetryRegistry>) -> Self {
        Self::try_with_telemetry_registry(telemetry_registry)
            .expect("plugin parameter specs should be valid")
    }

    pub(crate) fn try_with_telemetry_registry(
        telemetry_registry: Arc<Vst3TelemetryRegistry>,
    ) -> Result<Self, VestyProcessorInitError> {
        let plugin = P::default();
        let specs = plugin.params().specs();
        vesty_params::validate_param_specs(&specs)?;
        let vst3_param_ids = Vst3ParamIds::try_from_specs(&specs)?;
        let param_ids = specs.into_iter().map(|spec| spec.id).collect();
        let (telemetry_id, meter_producer, log_producer, fault) =
            telemetry_registry.create_channel();
        let sidechain_arrangement = default_sidechain_arrangement(&plugin);
        let output_arrangements = std::array::from_fn(|index| {
            AtomicU64::new(output_bus_at(&plugin, index).map_or(0, output_bus_arrangement))
        });
        let output_bus_active = std::array::from_fn(|index| AtomicBool::new(index == 0));
        Ok(Self {
            plugin,
            kernel: UnsafeCell::new(None),
            events: UnsafeCell::new(FixedEventList::new()),
            meter_producer: UnsafeCell::new(meter_producer),
            log_producer: UnsafeCell::new(log_producer),
            telemetry_id,
            telemetry_registry,
            param_ids,
            vst3_param_ids,
            sample_rate_bits: AtomicU64::new(44_100.0_f64.to_bits()),
            max_block_size: AtomicUsize::new(1024),
            input_arrangement: AtomicU64::new(default_input_arrangement(P::INFO.kind)),
            sidechain_arrangement: AtomicU64::new(sidechain_arrangement),
            output_arrangements,
            input_bus_active: AtomicBool::new(P::INFO.kind != vesty_core::PluginKind::Instrument),
            sidechain_bus_active: AtomicBool::new(false),
            output_bus_active,
            event_input_bus_active: AtomicBool::new(
                P::INFO.kind == vesty_core::PluginKind::Instrument,
            ),
            io_mode: AtomicI32::new(IoModes_::kSimple as IoMode),
            processing_active: AtomicBool::new(true),
            sample64_scratch: UnsafeCell::new(Sample64Scratch::default()),
            connection: Mutex::new(None),
            fault,
        })
    }

    #[cfg(test)]
    pub(crate) fn bus_active_for_test(
        &self,
        media_type: MediaType,
        dir: BusDirection,
        index: i32,
    ) -> Option<bool> {
        if !is_valid_bus_index(&self.plugin, media_type, dir, index) {
            return None;
        }

        match (media_type as MediaTypes, dir as BusDirections, index) {
            (MediaTypes_::kAudio, BusDirections_::kInput, 0) => {
                Some(self.input_bus_active.load(Ordering::Relaxed))
            }
            (MediaTypes_::kAudio, BusDirections_::kInput, 1) => {
                Some(self.sidechain_bus_active.load(Ordering::Relaxed))
            }
            (MediaTypes_::kAudio, BusDirections_::kOutput, index) => self
                .output_bus_active
                .get(index as usize)
                .map(|active| active.load(Ordering::Relaxed)),
            (MediaTypes_::kEvent, BusDirections_::kInput, 0) => {
                Some(self.event_input_bus_active.load(Ordering::Relaxed))
            }
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn io_mode_for_test(&self) -> IoMode {
        self.io_mode.load(Ordering::Relaxed)
    }

    unsafe fn ensure_kernel(&self) {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let slot = &mut *self.kernel.get();
            if slot.is_none() {
                let sample_rate = f64::from_bits(self.sample_rate_bits.load(Ordering::Relaxed));
                let max_block_size = self.max_block_size.load(Ordering::Relaxed);
                let mut kernel = self.plugin.create_kernel(KernelInit {
                    sample_rate,
                    max_block_size,
                });
                kernel.prepare(PrepareContext {
                    sample_rate,
                    max_block_size,
                });
                *slot = Some(kernel);
            }
        }
    }

    unsafe fn collect_parameter_changes(
        &self,
        process_data: &ProcessData,
        events: &mut FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>,
    ) {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(param_changes) = ComRef::from_raw(process_data.inputParameterChanges) else {
                return;
            };

            let param_count = param_changes.getParameterCount();
            for param_index in 0..param_count {
                let Some(param_queue) =
                    ComRef::from_raw(param_changes.getParameterData(param_index))
                else {
                    continue;
                };
                let point_count = param_queue.getPointCount();
                if point_count <= 0 {
                    continue;
                }
                let host_param_id = param_queue.getParameterId();
                let Some(param_index) = self.vst3_param_ids.index_for_host_id(host_param_id) else {
                    continue;
                };
                let Some(id) = self.param_ids.get(param_index) else {
                    continue;
                };

                let mut final_value = None;
                for point_index in 0..point_count {
                    let mut sample_offset = 0;
                    let mut value = 0.0;
                    if param_queue.getPoint(point_index, &mut sample_offset, &mut value)
                        != kResultTrue
                    {
                        continue;
                    }

                    let normalized = value.clamp(0.0, 1.0);
                    final_value = Some(normalized);
                    let _ = events.push(VestyEvent::Param {
                        sample_offset: sample_offset.max(0) as u32,
                        handle: vesty_params::ParamHandle::from_index(param_index),
                        id_hash: host_param_id,
                        normalized,
                    });
                }

                if let Some(value) = final_value {
                    let _ = self.plugin.params().set_normalized(id, value);
                }
            }
        }
    }

    unsafe fn collect_input_events(
        &self,
        process_data: &ProcessData,
        events: &mut FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>,
    ) {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(input_events) = ComRef::from_raw(process_data.inputEvents) else {
                return;
            };

            let event_count = input_events.getEventCount();
            for index in 0..event_count {
                let mut event = MaybeUninit::<Event>::zeroed();
                if input_events.getEvent(index, event.as_mut_ptr()) != kResultOk {
                    continue;
                }

                let event = event.assume_init();
                let sample_offset = event.sampleOffset.max(0) as u32;
                match event.r#type as Event_::EventTypes {
                    Event_::EventTypes_::kNoteOnEvent => {
                        let note = event.__field0.noteOn;
                        let _ = events.push(VestyEvent::NoteOn {
                            sample_offset,
                            channel: clamp_midi_channel_i16(note.channel),
                            key: clamp_midi_key(note.pitch),
                            velocity: note.velocity.clamp(0.0, 1.0),
                            note_id: note.noteId,
                        });
                    }
                    Event_::EventTypes_::kNoteOffEvent => {
                        let note = event.__field0.noteOff;
                        let _ = events.push(VestyEvent::NoteOff {
                            sample_offset,
                            channel: clamp_midi_channel_i16(note.channel),
                            key: clamp_midi_key(note.pitch),
                            velocity: note.velocity.clamp(0.0, 1.0),
                            note_id: note.noteId,
                        });
                    }
                    Event_::EventTypes_::kPolyPressureEvent => {
                        let pressure = event.__field0.polyPressure;
                        let _ = events.push(VestyEvent::PolyPressure {
                            sample_offset,
                            channel: clamp_midi_channel_i16(pressure.channel),
                            key: clamp_midi_key(pressure.pitch),
                            pressure: pressure.pressure.clamp(0.0, 1.0),
                            note_id: pressure.noteId,
                        });
                    }
                    Event_::EventTypes_::kDataEvent => {
                        let data = event.__field0.data;
                        if data.r#type == DataEvent_::DataTypes_::kMidiSysEx as uint32 {
                            let (payload, data_len, truncated) =
                                copy_sysex_data(data.bytes, data.size);
                            let _ = events.push(VestyEvent::SysEx {
                                sample_offset,
                                data_len,
                                data: payload,
                                truncated,
                            });
                        }
                    }
                    Event_::EventTypes_::kNoteExpressionValueEvent => {
                        let expression = event.__field0.noteExpressionValue;
                        let value = if expression.value.is_finite() {
                            expression.value
                        } else {
                            0.0
                        };
                        let _ = events.push(VestyEvent::NoteExpressionValue {
                            sample_offset,
                            type_id: expression.typeId,
                            note_id: expression.noteId,
                            value,
                        });
                    }
                    Event_::EventTypes_::kNoteExpressionIntValueEvent => {
                        let expression = event.__field0.noteExpressionIntValue;
                        let _ = events.push(VestyEvent::NoteExpressionInt {
                            sample_offset,
                            type_id: expression.typeId,
                            note_id: expression.noteId,
                            value: expression.value,
                        });
                    }
                    Event_::EventTypes_::kNoteExpressionTextEvent => {
                        let expression = event.__field0.noteExpressionText;
                        let (text, text_len) =
                            copy_note_expression_text(expression.text, expression.textLen);
                        let _ = events.push(VestyEvent::NoteExpressionText {
                            sample_offset,
                            type_id: expression.typeId,
                            note_id: expression.noteId,
                            text_len,
                            text,
                        });
                    }
                    Event_::EventTypes_::kLegacyMIDICCOutEvent => {
                        let midi = event.__field0.midiCCOut;
                        let channel = clamp_midi_channel_i8(midi.channel);
                        let value = clamp_midi7_i8(midi.value);
                        let value2 = clamp_midi7_i8(midi.value2);
                        match u32::from(midi.controlNumber) {
                            control if control == ControllerNumbers_::kPitchBend => {
                                let _ = events.push(VestyEvent::PitchBend {
                                    sample_offset,
                                    channel,
                                    value: midi_pitch_bend_to_bipolar(value, value2),
                                });
                            }
                            control if control == ControllerNumbers_::kAfterTouch => {
                                let _ = events.push(VestyEvent::ChannelPressure {
                                    sample_offset,
                                    channel,
                                    pressure: midi7_to_unit(value),
                                });
                            }
                            control => {
                                let _ = events.push(VestyEvent::MidiCc {
                                    sample_offset,
                                    channel,
                                    controller: control as u16,
                                    value: midi7_to_unit(value),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    unsafe fn transport(&self, process_data: &ProcessData) -> Transport {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if process_data.processContext.is_null() {
                return Transport::default();
            }

            let context = &*process_data.processContext;
            Transport {
                playing: context.state & ProcessContext_::StatesAndFlags_::kPlaying != 0,
                tempo_bpm: (context.state & ProcessContext_::StatesAndFlags_::kTempoValid != 0)
                    .then_some(context.tempo),
                position_samples: Some(context.projectTimeSamples),
            }
        }
    }

    unsafe fn run_kernel<'a>(
        &'a self,
        inputs: &'a [&'a [f32]],
        sidechain: &'a [&'a [f32]],
        outputs: &'a mut [&'a mut [f32]],
        events: &'a [VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> ProcessResult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let slot = &mut *self.kernel.get();
            let Some(kernel) = slot.as_mut() else {
                for output in outputs.iter_mut() {
                    output.fill(0.0);
                }
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::HostWarning {
                    code: RT_LOG_CODE_PROCESS_WITHOUT_KERNEL,
                    value: events.len() as i64,
                });
                return ProcessResult::Silence;
            };

            let audio = AudioBuffers::new(inputs, outputs);
            let sidechain = SidechainBuffers::new(sidechain);
            let meter_producer = &mut *self.meter_producer.get();
            let mut context =
                VestyProcessContext::new(audio, self.plugin.params(), events, transport)
                    .with_sidechain(sidechain)
                    .with_process_mode(process_mode)
                    .with_meter_sink(meter_producer);
            let fault_count_before = self.fault.fault_count();
            let result = panic_guard(&self.fault, ProcessResult::Silence, || {
                kernel.process(&mut context)
            });
            if self.fault.fault_count() > fault_count_before {
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::Faulted {
                    code: RT_LOG_CODE_PROCESS_PANIC,
                });
            }
            if result == ProcessResult::Silence {
                context.audio_mut().clear_outputs();
            }
            result
        }
    }

    unsafe fn run_kernel_f64<'a>(
        &'a self,
        inputs: &'a [&'a [f64]],
        sidechain: &'a [&'a [f64]],
        outputs: &'a mut [&'a mut [f64]],
        events: &'a [VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> ProcessResult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let slot = &mut *self.kernel.get();
            let Some(kernel) = slot.as_mut() else {
                for output in outputs.iter_mut() {
                    output.fill(0.0);
                }
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::HostWarning {
                    code: RT_LOG_CODE_PROCESS_WITHOUT_KERNEL,
                    value: events.len() as i64,
                });
                return ProcessResult::Silence;
            };

            let audio = AudioBuffers64::new(inputs, outputs);
            let sidechain = SidechainBuffers64::new(sidechain);
            let meter_producer = &mut *self.meter_producer.get();
            let mut context =
                VestyProcessContext64::new(audio, self.plugin.params(), events, transport)
                    .with_sidechain(sidechain)
                    .with_process_mode(process_mode)
                    .with_meter_sink(meter_producer);
            let fault_count_before = self.fault.fault_count();
            let result = panic_guard(&self.fault, ProcessResult::Silence, || {
                kernel.process_f64(&mut context)
            });
            if self.fault.fault_count() > fault_count_before {
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::Faulted {
                    code: RT_LOG_CODE_PROCESS_PANIC,
                });
            }
            if result == ProcessResult::Silence {
                context.audio_mut().clear_outputs();
            }
            result
        }
    }

    unsafe fn process_sample32(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(output_layout) =
                output_layout32(&self.plugin, &self.output_arrangements, process_data)
            else {
                return kResultOk;
            };
            let num_samples = process_data.numSamples.max(0) as usize;
            let Some(input_buses) = input_buses::<P>(&self.plugin, process_data) else {
                silence_process_outputs32(
                    &self.plugin,
                    &self.output_arrangements,
                    process_data,
                    num_samples,
                );
                return kResultOk;
            };
            let input_channels = input_buses
                .first()
                .and_then(|bus| valid_audio_bus_channels32(bus, MAX_MAIN_IO_CHANNELS));
            let sidechain_channels = if supports_sidechain(&self.plugin) {
                input_buses
                    .get(1)
                    .and_then(|bus| valid_audio_bus_channels32(bus, MAX_SIDECHAIN_CHANNELS))
            } else {
                None
            };

            let process_result = {
                let mut input_storage: [MaybeUninit<&[f32]>; MAX_MAIN_IO_CHANNELS] = uninit_array();
                let mut sidechain_storage: [MaybeUninit<&[f32]>; MAX_SIDECHAIN_CHANNELS] =
                    uninit_array();
                let mut output_storage: [MaybeUninit<&mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS] =
                    uninit_array();
                let inputs = input_views32(input_channels, num_samples, &mut input_storage);
                let sidechain =
                    input_views32(sidechain_channels, num_samples, &mut sidechain_storage);
                let outputs = output_views32(&output_layout, num_samples, &mut output_storage);
                self.run_kernel(inputs, sidechain, outputs, events, transport, process_mode)
            };
            set_output_silence_flags(
                process_data,
                &output_layout,
                process_result == ProcessResult::Silence,
            );
            kResultOk
        }
    }

    unsafe fn process_sample64(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: Dispatch stays within the VST3 process callback contract; both callees validate
        // nullable host pointers and bus/channel shape before constructing slices.
        unsafe {
            if P::Kernel::SUPPORTS_F64 {
                return self.process_sample64_native(process_data, events, transport, process_mode);
            }
            self.process_sample64_via_f32_scratch(process_data, events, transport, process_mode)
        }
    }

    unsafe fn process_sample64_native(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(output_layout) =
                output_layout64(&self.plugin, &self.output_arrangements, process_data)
            else {
                return kResultOk;
            };
            let num_samples = process_data.numSamples.max(0) as usize;
            let Some(input_buses) = input_buses::<P>(&self.plugin, process_data) else {
                silence_process_outputs64(
                    &self.plugin,
                    &self.output_arrangements,
                    process_data,
                    num_samples,
                );
                return kResultOk;
            };
            let input_channels = input_buses
                .first()
                .and_then(|bus| valid_audio_bus_channels64(bus, MAX_MAIN_IO_CHANNELS));
            let sidechain_channels = if supports_sidechain(&self.plugin) {
                input_buses
                    .get(1)
                    .and_then(|bus| valid_audio_bus_channels64(bus, MAX_SIDECHAIN_CHANNELS))
            } else {
                None
            };

            let process_result = {
                let mut input_storage: [MaybeUninit<&[f64]>; MAX_MAIN_IO_CHANNELS] = uninit_array();
                let mut sidechain_storage: [MaybeUninit<&[f64]>; MAX_SIDECHAIN_CHANNELS] =
                    uninit_array();
                let mut output_storage: [MaybeUninit<&mut [f64]>; MAX_AUDIO_OUTPUT_CHANNELS] =
                    uninit_array();
                let inputs = input_views64(input_channels, num_samples, &mut input_storage);
                let sidechain =
                    input_views64(sidechain_channels, num_samples, &mut sidechain_storage);
                let outputs = output_views64(&output_layout, num_samples, &mut output_storage);
                self.run_kernel_f64(inputs, sidechain, outputs, events, transport, process_mode)
            };
            set_output_silence_flags(
                process_data,
                &output_layout,
                process_result == ProcessResult::Silence,
            );
            kResultOk
        }
    }

    unsafe fn process_sample64_via_f32_scratch(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(output_layout) =
                output_layout64(&self.plugin, &self.output_arrangements, process_data)
            else {
                return kResultOk;
            };
            let num_samples = process_data.numSamples.max(0) as usize;
            let scratch = &mut *self.sample64_scratch.get();
            if !scratch.has_capacity(num_samples) {
                clear_output_layout64(&output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, true);
                return kResultOk;
            }
            copy_output_layout64_to_scratch(&output_layout, &mut scratch.outputs, num_samples);

            let Some(input_buses) = input_buses::<P>(&self.plugin, process_data) else {
                clear_output_layout64(&output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, true);
                return kResultOk;
            };
            let input_channels = input_buses
                .first()
                .and_then(|bus| valid_audio_bus_channels64(bus, MAX_MAIN_IO_CHANNELS));
            let sidechain_channels = if supports_sidechain(&self.plugin) {
                input_buses
                    .get(1)
                    .and_then(|bus| valid_audio_bus_channels64(bus, MAX_SIDECHAIN_CHANNELS))
            } else {
                None
            };

            let process_result = {
                let mut input_storage: [MaybeUninit<&[f32]>; MAX_MAIN_IO_CHANNELS] = uninit_array();
                let mut sidechain_storage: [MaybeUninit<&[f32]>; MAX_SIDECHAIN_CHANNELS] =
                    uninit_array();
                let mut output_storage: [MaybeUninit<&mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS] =
                    uninit_array();
                let inputs = scratch_input_views_from_f64(
                    &mut scratch.inputs,
                    input_channels,
                    num_samples,
                    &mut input_storage,
                );
                let sidechain = scratch_input_views_from_f64(
                    &mut scratch.sidechain,
                    sidechain_channels,
                    num_samples,
                    &mut sidechain_storage,
                );
                let outputs = scratch_output_views(
                    &mut scratch.outputs,
                    num_samples,
                    output_layout.channel_count,
                    &mut output_storage,
                );
                self.run_kernel(inputs, sidechain, outputs, events, transport, process_mode)
            };

            if process_result == ProcessResult::Silence {
                clear_output_layout64(&output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, true);
            } else {
                copy_scratch_to_output_layout64(&scratch.outputs, &output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, false);
            }
            kResultOk
        }
    }
}

fn vst3_process_mode(process_data: &ProcessData) -> ProcessMode {
    match process_data.processMode {
        mode if mode == ProcessModes_::kOffline as int32 => ProcessMode::Offline,
        mode if mode == ProcessModes_::kPrefetch as int32 => ProcessMode::Prefetch,
        _ => ProcessMode::Realtime,
    }
}

impl<P: Plugin + Default> Drop for VestyProcessor<P> {
    fn drop(&mut self) {
        self.telemetry_registry
            .remove_meter_consumer(self.telemetry_id);
    }
}

impl<P: Plugin + Default> IPluginBaseTrait for VestyProcessor<P> {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl<P: Plugin + Default> IComponentTrait for VestyProcessor<P> {
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        if class_id.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; the nullable output pointer has been checked above.
        unsafe {
            *class_id = controller_cid::<P>();
            kResultOk
        }
    }

    unsafe fn setIoMode(&self, mode: IoMode) -> tresult {
        match mode {
            mode if mode == IoModes_::kSimple as IoMode
                || mode == IoModes_::kAdvanced as IoMode
                || mode == IoModes_::kOfflineProcessing as IoMode =>
            {
                self.io_mode.store(mode, Ordering::Release);
                kResultOk
            }
            _ => kInvalidArgument,
        }
    }

    unsafe fn getBusCount(&self, mediaType: MediaType, dir: BusDirection) -> i32 {
        match mediaType as MediaTypes {
            MediaTypes_::kAudio => match dir as BusDirections {
                BusDirections_::kInput => {
                    if P::INFO.kind == vesty_core::PluginKind::Instrument {
                        0
                    } else if supports_sidechain(&self.plugin) {
                        2
                    } else {
                        1
                    }
                }
                BusDirections_::kOutput => output_bus_count(&self.plugin) as i32,
                _ => 0,
            },
            MediaTypes_::kEvent => match dir as BusDirections {
                BusDirections_::kInput if P::INFO.kind == vesty_core::PluginKind::Instrument => 1,
                _ => 0,
            },
            _ => 0,
        }
    }

    unsafe fn getBusInfo(
        &self,
        mediaType: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if index < 0 || bus.is_null() {
                return kInvalidArgument;
            }
            let bus = &mut *bus;
            match (mediaType as MediaTypes, dir as BusDirections) {
                (MediaTypes_::kAudio, BusDirections_::kInput)
                    if P::INFO.kind != vesty_core::PluginKind::Instrument && index == 0 =>
                {
                    bus.mediaType = MediaTypes_::kAudio as MediaType;
                    bus.direction = BusDirections_::kInput as BusDirection;
                    bus.channelCount =
                        arrangement_channel_count(self.input_arrangement.load(Ordering::Relaxed))
                            .unwrap_or(2);
                    copy_wstring("Input", &mut bus.name);
                    bus.busType = BusTypes_::kMain as BusType;
                    bus.flags = BusInfo_::BusFlags_::kDefaultActive;
                    kResultOk
                }
                (MediaTypes_::kAudio, BusDirections_::kInput)
                    if P::INFO.kind != vesty_core::PluginKind::Instrument
                        && index == 1
                        && supports_sidechain(&self.plugin) =>
                {
                    bus.mediaType = MediaTypes_::kAudio as MediaType;
                    bus.direction = BusDirections_::kInput as BusDirection;
                    bus.channelCount = arrangement_channel_count(
                        self.sidechain_arrangement.load(Ordering::Relaxed),
                    )
                    .unwrap_or(2);
                    copy_wstring("Sidechain", &mut bus.name);
                    bus.busType = BusTypes_::kAux as BusType;
                    bus.flags = 0;
                    kResultOk
                }
                (MediaTypes_::kAudio, BusDirections_::kOutput)
                    if (index as usize) < output_bus_count(&self.plugin) =>
                {
                    let index = index as usize;
                    let Some(output_bus) = output_bus_at(&self.plugin, index) else {
                        return kInvalidArgument;
                    };
                    bus.mediaType = MediaTypes_::kAudio as MediaType;
                    bus.direction = BusDirections_::kOutput as BusDirection;
                    bus.channelCount = arrangement_channel_count(
                        self.output_arrangements[index].load(Ordering::Relaxed),
                    )
                    .unwrap_or(output_bus.channels as i32);
                    copy_wstring(output_bus.name, &mut bus.name);
                    bus.busType = if index == 0 {
                        BusTypes_::kMain
                    } else {
                        BusTypes_::kAux
                    } as BusType;
                    bus.flags = if index == 0 {
                        BusInfo_::BusFlags_::kDefaultActive
                    } else {
                        0
                    };
                    kResultOk
                }
                (MediaTypes_::kEvent, BusDirections_::kInput)
                    if P::INFO.kind == vesty_core::PluginKind::Instrument && index == 0 =>
                {
                    bus.mediaType = MediaTypes_::kEvent as MediaType;
                    bus.direction = BusDirections_::kInput as BusDirection;
                    bus.channelCount = 1;
                    copy_wstring("Event Input", &mut bus.name);
                    bus.busType = BusTypes_::kMain as BusType;
                    bus.flags = BusInfo_::BusFlags_::kDefaultActive;
                    kResultOk
                }
                _ => kInvalidArgument,
            }
        }
    }

    unsafe fn getRoutingInfo(
        &self,
        in_info: *mut RoutingInfo,
        out_info: *mut RoutingInfo,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if in_info.is_null() || out_info.is_null() {
                return kInvalidArgument;
            }

            let input = *in_info;
            let valid_input = if P::INFO.kind == vesty_core::PluginKind::Instrument {
                input.mediaType == MediaTypes_::kEvent as MediaType
                    && input.busIndex == 0
                    && (-1..=15).contains(&input.channel)
            } else {
                match input.busIndex {
                    0 => {
                        let input_channels = arrangement_channel_count(
                            self.input_arrangement.load(Ordering::Relaxed),
                        )
                        .unwrap_or(2);
                        input.mediaType == MediaTypes_::kAudio as MediaType
                            && (input.channel == -1 || (0..input_channels).contains(&input.channel))
                    }
                    1 if supports_sidechain(&self.plugin) => {
                        let sidechain_channels = arrangement_channel_count(
                            self.sidechain_arrangement.load(Ordering::Relaxed),
                        )
                        .unwrap_or(2);
                        input.mediaType == MediaTypes_::kAudio as MediaType
                            && (input.channel == -1
                                || (0..sidechain_channels).contains(&input.channel))
                    }
                    _ => false,
                }
            };

            if !valid_input {
                return kInvalidArgument;
            }

            *out_info = RoutingInfo {
                mediaType: MediaTypes_::kAudio as MediaType,
                busIndex: 0,
                channel: -1,
            };
            kResultOk
        }
    }

    unsafe fn activateBus(
        &self,
        media_type: MediaType,
        dir: BusDirection,
        index: i32,
        state: TBool,
    ) -> tresult {
        if !is_valid_bus_index(&self.plugin, media_type, dir, index) {
            return kInvalidArgument;
        }

        let active = state != 0;
        match (media_type as MediaTypes, dir as BusDirections, index) {
            (MediaTypes_::kAudio, BusDirections_::kInput, 0) => {
                self.input_bus_active.store(active, Ordering::Relaxed);
            }
            (MediaTypes_::kAudio, BusDirections_::kInput, 1) => {
                self.sidechain_bus_active.store(active, Ordering::Relaxed);
            }
            (MediaTypes_::kAudio, BusDirections_::kOutput, index) => {
                self.output_bus_active[index as usize].store(active, Ordering::Relaxed);
            }
            (MediaTypes_::kEvent, BusDirections_::kInput, 0) => {
                self.event_input_bus_active.store(active, Ordering::Relaxed);
            }
            _ => return kInvalidArgument,
        }
        kResultOk
    }

    unsafe fn setActive(&self, state: TBool) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if state != 0 {
                self.ensure_kernel();
            } else {
                *self.kernel.get() = None;
            }
            kResultOk
        }
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            match read_state_stream(state) {
                Ok(state) => {
                    if apply_state(&self.plugin, state).is_ok() {
                        kResultOk
                    } else {
                        kResultFalse
                    }
                }
                Err(()) => kInvalidArgument,
            }
        }
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            match capture_state(&self.plugin) {
                Ok(captured) => match write_state_stream(state, &captured) {
                    Ok(()) => kResultOk,
                    Err(()) => kInvalidArgument,
                },
                Err(_) => kResultFalse,
            }
        }
    }
}

impl<P: Plugin + Default> IAudioProcessorTrait for VestyProcessor<P> {
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let output_bus_count = output_bus_count(&self.plugin);
            if outputs.is_null() || num_outs != output_bus_count as i32 {
                return kInvalidArgument;
            }

            let output_arrangements = slice::from_raw_parts(outputs, output_bus_count);
            if P::INFO.kind == vesty_core::PluginKind::Instrument {
                if num_ins != 0
                    || !validate_output_arrangements::<P>(&self.plugin, output_arrangements, None)
                {
                    return kResultFalse;
                }
                for (index, output) in output_arrangements.iter().copied().enumerate() {
                    self.output_arrangements[index].store(output, Ordering::Relaxed);
                }
                return kResultTrue;
            }

            let supports_sidechain = supports_sidechain(&self.plugin);
            if (!supports_sidechain && num_ins != 1)
                || (supports_sidechain && !(1..=2).contains(&num_ins))
            {
                return kResultFalse;
            }
            if inputs.is_null() {
                return kInvalidArgument;
            }
            let input_arrangements = slice::from_raw_parts(inputs, num_ins as usize);
            let input = input_arrangements[0];
            if !validate_output_arrangements::<P>(&self.plugin, output_arrangements, Some(input)) {
                return kResultFalse;
            }
            if supports_sidechain && num_ins == 2 {
                let sidechain = input_arrangements[1];
                if !is_supported_sidechain_arrangement(sidechain) {
                    return kResultFalse;
                }
                self.sidechain_arrangement
                    .store(sidechain, Ordering::Relaxed);
            }
            self.input_arrangement.store(input, Ordering::Relaxed);
            for (index, output) in output_arrangements.iter().copied().enumerate() {
                self.output_arrangements[index].store(output, Ordering::Relaxed);
            }
            kResultTrue
        }
    }

    unsafe fn getBusArrangement(
        &self,
        dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if index < 0 || arr.is_null() {
                return kInvalidArgument;
            }
            match dir as BusDirections {
                BusDirections_::kInput
                    if P::INFO.kind != vesty_core::PluginKind::Instrument && index == 0 =>
                {
                    *arr = self.input_arrangement.load(Ordering::Relaxed);
                    kResultOk
                }
                BusDirections_::kInput
                    if P::INFO.kind != vesty_core::PluginKind::Instrument
                        && index == 1
                        && supports_sidechain(&self.plugin) =>
                {
                    *arr = self.sidechain_arrangement.load(Ordering::Relaxed);
                    kResultOk
                }
                BusDirections_::kOutput if (index as usize) < output_bus_count(&self.plugin) => {
                    *arr = self.output_arrangements[index as usize].load(Ordering::Relaxed);
                    kResultOk
                }
                _ => kInvalidArgument,
            }
        }
    }

    unsafe fn canProcessSampleSize(&self, symbolic_sample_size: i32) -> tresult {
        match symbolic_sample_size as SymbolicSampleSizes {
            SymbolicSampleSizes_::kSample32 => kResultOk,
            SymbolicSampleSizes_::kSample64 => kResultOk,
            _ => kInvalidArgument,
        }
    }

    unsafe fn getLatencySamples(&self) -> u32 {
        self.plugin.latency_samples()
    }

    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if setup.is_null() {
                return kInvalidArgument;
            }
            let setup = &*setup;
            if !setup_sample_size_supported(setup) {
                return kInvalidArgument;
            }
            let Some(sample_rate) = setup_sample_rate(setup) else {
                return kInvalidArgument;
            };
            let Some(max_block_size) = setup_block_size(setup) else {
                return kInvalidArgument;
            };

            self.sample_rate_bits
                .store(sample_rate.to_bits(), Ordering::Relaxed);
            self.max_block_size.store(max_block_size, Ordering::Relaxed);
            (&mut *self.sample64_scratch.get()).prepare(max_block_size);
            if let Some(kernel) = (&mut *self.kernel.get()).as_mut() {
                kernel.prepare(PrepareContext {
                    sample_rate,
                    max_block_size,
                });
            } else {
                self.ensure_kernel();
            }
            kResultOk
        }
    }

    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        self.processing_active.store(state != 0, Ordering::Release);
        kResultOk
    }

    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if data.is_null() {
                return kInvalidArgument;
            }
            let process_data = &*data;
            let symbolic_sample_size = process_data.symbolicSampleSize as SymbolicSampleSizes;
            if !matches!(
                symbolic_sample_size,
                SymbolicSampleSizes_::kSample32 | SymbolicSampleSizes_::kSample64
            ) {
                return kResultOk;
            }
            let _rt_guard = NoAllocGuard::enter();
            if let Err(frames_to_clear) = process_block_frames(process_data) {
                match symbolic_sample_size {
                    SymbolicSampleSizes_::kSample32 => {
                        silence_process_outputs32(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames_to_clear,
                        );
                    }
                    SymbolicSampleSizes_::kSample64 => {
                        silence_process_outputs64(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames_to_clear,
                        );
                    }
                    _ => {}
                }
                return kResultOk;
            }
            if !self.processing_active.load(Ordering::Acquire) {
                let frames = process_data.numSamples.max(0) as usize;
                match symbolic_sample_size {
                    SymbolicSampleSizes_::kSample32 => {
                        silence_process_outputs32(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames,
                        );
                    }
                    SymbolicSampleSizes_::kSample64 => {
                        silence_process_outputs64(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames,
                        );
                    }
                    _ => {}
                }
                return kResultOk;
            }
            let events = &mut *self.events.get();
            events.clear();
            self.collect_parameter_changes(process_data, events);
            self.collect_input_events(process_data, events);
            sort_events_by_sample_offset(events);
            let transport = self.transport(process_data);
            let process_mode = vst3_process_mode(process_data);

            match symbolic_sample_size {
                SymbolicSampleSizes_::kSample32 => {
                    self.process_sample32(process_data, events.as_slice(), transport, process_mode)
                }
                SymbolicSampleSizes_::kSample64 => {
                    self.process_sample64(process_data, events.as_slice(), transport, process_mode)
                }
                _ => kResultOk,
            }
        }
    }

    unsafe fn getTailSamples(&self) -> u32 {
        self.plugin.tail_samples()
    }
}

impl<P: Plugin + Default> IProcessContextRequirementsTrait for VestyProcessor<P> {
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        0
    }
}

impl<P: Plugin + Default> IConnectionPointTrait for VestyProcessor<P> {
    unsafe fn connect(&self, other: *mut IConnectionPoint) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let result = connect_connection_point(&self.connection, other);
            if result == kResultOk {
                let _ = notify_telemetry_bind(other, self.telemetry_id);
            }
            result
        }
    }

    unsafe fn disconnect(&self, other: *mut IConnectionPoint) -> tresult {
        disconnect_connection_point(&self.connection, other)
    }

    unsafe fn notify(&self, _message: *mut IMessage) -> tresult {
        kResultOk
    }
}

unsafe fn notify_telemetry_bind(other: *mut IConnectionPoint, telemetry_id: u64) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let Some(other) = ComRef::from_raw(other) else {
            return kInvalidArgument;
        };
        let Some(message) = VestyMessage::telemetry_bind(telemetry_id) else {
            return kResultFalse;
        };
        let Some(message) = ComWrapper::new(message).to_com_ptr::<IMessage>() else {
            return kResultFalse;
        };
        other.notify(message.as_ptr())
    }
}

fn connect_connection_point(slot: &SharedConnectionPoint, other: *mut IConnectionPoint) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    let Some(other) = (unsafe { ComRef::from_raw(other) }) else {
        return kInvalidArgument;
    };
    let Ok(mut slot) = slot.lock() else {
        return kResultFalse;
    };
    *slot = Some(other.to_com_ptr());
    kResultOk
}

fn disconnect_connection_point(
    slot: &SharedConnectionPoint,
    other: *mut IConnectionPoint,
) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    let Some(other) = (unsafe { ComRef::from_raw(other) }) else {
        return kInvalidArgument;
    };
    let Ok(mut slot) = slot.lock() else {
        return kResultFalse;
    };
    let Some(current) = slot.as_ref() else {
        return kResultFalse;
    };
    if current.as_ptr() != other.as_ptr() {
        return kResultFalse;
    }
    *slot = None;
    kResultOk
}

type SharedComponentHandler = Arc<Mutex<Option<ComPtr<IComponentHandler>>>>;
#[cfg(feature = "wry-ui")]
type ParamSetter = Arc<dyn Fn(&str, f64) -> HostChangeFlags + Send + Sync>;

#[cfg(feature = "wry-ui")]
#[derive(Clone)]
pub(crate) struct WryBridgeEndpoint {
    ready: BridgeReadyPayload,
    param_ids: Arc<Vec<String>>,
    param_host_ids: Arc<Vec<ParamID>>,
    handler: SharedComponentHandler,
    set_param: ParamSetter,
    bridge_snapshot: SharedBridgeSnapshot,
    param_changes: SharedParamChangeQueue,
    meters: SharedMeterConsumer,
    logs: SharedLogConsumer,
    fault: SharedFaultState,
}

#[cfg(feature = "wry-ui")]
impl WryBridgeEndpoint {
    pub(crate) fn bridge_handler(&self) -> impl Fn(String) -> Vec<BridgePacket> + 'static {
        let endpoint = self.clone();
        let transport = SharedBridgeTransport::default();
        let sent = transport.sent.clone();
        let mut ready = self.ready.clone();
        let initial_generation = endpoint.refresh_ready_snapshot(&mut ready);
        ready.editor_session_id = next_editor_session_id();
        let (runtime, invalid_schema_error) =
            match BridgeRuntime::try_new("pending", ready, transport) {
                Ok(runtime) => (Some(Rc::new(RefCell::new(runtime))), None),
                Err(error) => {
                    let message = error.to_string();
                    bridge_trace(format!("invalid_ready_param_schema: {message}"));
                    (None, Some(message))
                }
            };
        let observed_snapshot_generation = Rc::new(RefCell::new(initial_generation));
        let last_fault_report = Rc::new(RefCell::new(None::<PluginFaultReport>));
        let rt_log_sequence = Rc::new(RefCell::new(1_u64));
        let invalid_error_sequence = Rc::new(RefCell::new(1_u64));
        move |text| {
            bridge_trace(format!("ipc: {}", text));
            if let Some(message) = invalid_schema_error.as_ref() {
                let Ok(packet) = serde_json::from_str::<BridgePacket>(&text) else {
                    bridge_trace("invalid_schema_ipc_parse_error");
                    return Vec::new();
                };
                let mut seq = invalid_error_sequence.borrow_mut();
                let error = BridgeErrorPayload::new(
                    BridgeErrorCode::ValidationError,
                    format!("invalid bridge parameter schema: {message}"),
                    false,
                );
                let packet = packet.error_to(*seq, error);
                *seq = seq.saturating_add(1);
                return vec![packet];
            }

            let Some(runtime) = runtime.as_ref() else {
                bridge_trace("bridge_runtime_unavailable");
                return Vec::new();
            };
            let mut runtime = runtime.borrow_mut();
            endpoint.sync_bridge_snapshot(&mut runtime, &observed_snapshot_generation);
            runtime.set_fault_report(endpoint.fault_report());
            if runtime.receive_json(&text).is_err() {
                bridge_trace("ipc_error");
                return std::mem::take(&mut *sent.borrow_mut());
            }
            for gesture in runtime.drain_param_gestures() {
                // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
                unsafe {
                    let result = endpoint.relay_param_gesture(&gesture);
                    bridge_trace(format!("relay: {:?} result={}", gesture, result));
                    if result == kResultOk && gesture.phase == ParamGesturePhase::Perform {
                        let Some(normalized) = gesture.normalized else {
                            continue;
                        };
                        let changed = runtime.emit_param_changed(
                            &gesture.id,
                            normalized,
                            ParamChangeSource::Ui,
                            gesture.gesture_id.clone(),
                        );
                        if let Err(error) = changed {
                            bridge_trace(format!("param_changed_error: {error}"));
                        }
                    }
                }
            }
            endpoint.flush_param_changes(&mut runtime);
            endpoint.flush_meters(&mut runtime);
            endpoint.flush_logs(&mut runtime, &rt_log_sequence);
            endpoint.flush_fault_report(&mut runtime, &last_fault_report);
            endpoint.store_bridge_snapshot(runtime.snapshot(), &observed_snapshot_generation);
            std::mem::take(&mut *sent.borrow_mut())
        }
    }

    fn refresh_ready_snapshot(&self, ready: &mut BridgeReadyPayload) -> u64 {
        let Ok(slot) = self.bridge_snapshot.lock() else {
            bridge_trace("bridge_snapshot_no_lock");
            return 0;
        };
        ready.snapshot = slot.snapshot.clone();
        slot.generation
    }

    fn sync_bridge_snapshot(
        &self,
        runtime: &mut BridgeRuntime<SharedBridgeTransport>,
        observed_generation: &Rc<RefCell<u64>>,
    ) {
        let Ok(slot) = self.bridge_snapshot.lock() else {
            bridge_trace("bridge_snapshot_no_lock");
            return;
        };
        let generation = slot.generation;
        if generation == *observed_generation.borrow() {
            return;
        }
        let snapshot = slot.snapshot.clone();
        drop(slot);

        if let Err(error) = runtime.restore_snapshot_from_host(snapshot) {
            bridge_trace(format!("bridge_snapshot_restore_error: {error}"));
        }
        *observed_generation.borrow_mut() = generation;
    }

    fn store_bridge_snapshot(
        &self,
        snapshot: &PluginSnapshot,
        observed_generation: &Rc<RefCell<u64>>,
    ) {
        let Ok(mut slot) = self.bridge_snapshot.lock() else {
            bridge_trace("bridge_snapshot_no_lock");
            return;
        };
        if slot.generation != *observed_generation.borrow() {
            bridge_trace("bridge_snapshot_store_skipped_external_generation");
            return;
        }
        if slot.snapshot == *snapshot {
            return;
        }
        slot.snapshot = snapshot.clone();
        slot.generation = slot.generation.saturating_add(1);
        *observed_generation.borrow_mut() = slot.generation;
    }

    fn flush_param_changes(&self, runtime: &mut BridgeRuntime<SharedBridgeTransport>) {
        if !runtime.is_subscribed("param.changed") {
            return;
        }
        let Ok(mut changes) = self.param_changes.lock() else {
            bridge_trace("param_change_no_lock");
            return;
        };
        let pending = std::mem::take(&mut *changes);
        drop(changes);
        for (id, change) in pending {
            if let Err(error) =
                runtime.emit_param_changed(&id, change.normalized, change.source, None)
            {
                bridge_trace(format!("host_param_changed_error: {error}"));
            }
        }
    }

    fn flush_meters(&self, runtime: &mut BridgeRuntime<SharedBridgeTransport>) {
        let Ok(mut meter_slot) = self.meters.lock() else {
            bridge_trace("meter_no_lock");
            return;
        };
        let Some(consumer) = meter_slot.as_mut() else {
            return;
        };
        for _ in 0..METER_QUEUE_CAPACITY {
            let Ok(frame) = consumer.try_pop() else {
                break;
            };
            let _ = runtime.queue_latest_meter_frame(MAIN_METER_TOPIC, &frame);
        }
        match runtime.flush_latest_meters() {
            Ok(sent) if sent > 0 => bridge_trace(format!("meter_flush sent={sent}")),
            Ok(_) => {}
            Err(error) => bridge_trace(format!("meter_flush_error: {error}")),
        }
    }

    fn flush_logs(
        &self,
        runtime: &mut BridgeRuntime<SharedBridgeTransport>,
        rt_log_sequence: &Rc<RefCell<u64>>,
    ) {
        let Ok(mut log_slot) = self.logs.lock() else {
            bridge_trace("log_no_lock");
            return;
        };
        let Some(consumer) = log_slot.as_mut() else {
            return;
        };
        for _ in 0..LOG_QUEUE_CAPACITY {
            let Ok(event) = consumer.try_pop() else {
                break;
            };
            let sequence = {
                let mut next = rt_log_sequence.borrow_mut();
                let sequence = *next;
                *next += 1;
                sequence
            };
            if let Err(error) = runtime.emit_rt_log_event(RT_LOG_TOPIC, sequence, event) {
                bridge_trace(format!("rt_log_error: {error}"));
            }
        }
    }

    fn fault_report(&self) -> Option<PluginFaultReport> {
        let fault = self.fault.lock().ok()?.as_ref()?.report();
        Some(PluginFaultReport {
            faulted: fault.faulted,
            fault_count: fault.fault_count,
        })
    }

    fn flush_fault_report(
        &self,
        runtime: &mut BridgeRuntime<SharedBridgeTransport>,
        last_fault_report: &Rc<RefCell<Option<PluginFaultReport>>>,
    ) {
        let current = self.fault_report();
        if *last_fault_report.borrow() == current {
            return;
        }
        *last_fault_report.borrow_mut() = current.clone();
        let Some(report) = current else {
            return;
        };
        if let Err(error) = runtime.emit_fault_report(FAULT_DIAGNOSTICS_TOPIC, report) {
            bridge_trace(format!("fault_report_error: {error}"));
        }
    }

    unsafe fn relay_param_gesture(&self, gesture: &ParamGesture) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(id) = self
                .param_ids
                .iter()
                .position(|id| id == &gesture.id)
                .and_then(|index| self.param_host_ids.get(index).copied())
            else {
                return kInvalidArgument;
            };
            let Ok(handler) = self.handler.lock() else {
                bridge_trace("relay_no_handler_lock");
                return kResultFalse;
            };
            let Some(handler) = handler.as_ref() else {
                bridge_trace("relay_no_handler");
                return kResultFalse;
            };
            match gesture.phase {
                ParamGesturePhase::Begin => handler.beginEdit(id),
                ParamGesturePhase::Perform => {
                    let normalized = gesture.normalized.unwrap_or(0.0).clamp(0.0, 1.0);
                    let changes = (self.set_param)(&gesture.id, normalized);
                    let result = handler.performEdit(id, normalized);
                    if result == kResultOk {
                        let _ = restart_component_for_host_changes(handler, changes);
                    }
                    result
                }
                ParamGesturePhase::End => handler.endEdit(id),
            }
        }
    }
}

#[cfg(feature = "wry-ui")]
fn bridge_trace(line: impl AsRef<str>) {
    let Ok(path) = std::env::var("VESTY_BRIDGE_TRACE") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{}", line.as_ref());
    }
}

#[cfg(feature = "wry-ui")]
#[derive(Clone, Default)]
struct SharedBridgeTransport {
    sent: Rc<RefCell<Vec<BridgePacket>>>,
}

#[cfg(feature = "wry-ui")]
impl BridgeTransport for SharedBridgeTransport {
    fn send(&mut self, packet: &BridgePacket) -> Result<(), BridgeTransportError> {
        self.sent.borrow_mut().push(packet.clone());
        Ok(())
    }
}

struct VestyPlugView {
    descriptor: UiDescriptor,
    size: UnsafeCell<ViewRect>,
    frame: UnsafeCell<*mut IPlugFrame>,
    #[cfg(feature = "wry-ui")]
    bridge_endpoint: WryBridgeEndpoint,
    #[cfg(feature = "wry-ui")]
    runtime: UnsafeCell<Option<WryEditorRuntime>>,
}

// SAFETY: VST3 editor callbacks are expected on the host UI thread. Interior mutability is used to
// update view size/frame/runtime from &self COM callbacks without locking.
unsafe impl Sync for VestyPlugView {}

impl Class for VestyPlugView {
    type Interfaces = (IPlugView,);
}

impl VestyPlugView {
    fn new(
        descriptor: UiDescriptor,
        #[cfg(feature = "wry-ui")] bridge_endpoint: WryBridgeEndpoint,
    ) -> Self {
        let size = view_rect(descriptor.width, descriptor.height);
        Self {
            descriptor,
            size: UnsafeCell::new(size),
            frame: UnsafeCell::new(std::ptr::null_mut()),
            #[cfg(feature = "wry-ui")]
            bridge_endpoint,
            #[cfg(feature = "wry-ui")]
            runtime: UnsafeCell::new(None),
        }
    }

    unsafe fn platform_supported(platform_type: FIDString) -> bool {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            (cfg!(target_os = "macos") && fid_eq(platform_type, kPlatformTypeNSView))
                || (cfg!(target_os = "windows") && fid_eq(platform_type, kPlatformTypeHWND))
                || (cfg!(any(
                    target_os = "linux",
                    target_os = "dragonfly",
                    target_os = "freebsd",
                    target_os = "netbsd",
                    target_os = "openbsd"
                )) && fid_eq(platform_type, kPlatformTypeX11EmbedWindowID))
        }
    }

    fn clamp_rect(&self, rect: &mut ViewRect) {
        let min_width = self.descriptor.min_width as i32;
        let min_height = self.descriptor.min_height as i32;
        let width = (rect.right - rect.left).max(min_width);
        let height = (rect.bottom - rect.top).max(min_height);
        rect.right = rect.left + width;
        rect.bottom = rect.top + height;
    }

    #[cfg(feature = "wry-ui")]
    unsafe fn native_parent(
        &self,
        parent: *mut c_void,
        platform_type: FIDString,
    ) -> Result<NativeParent, ()> {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            #[cfg(target_os = "macos")]
            if fid_eq(platform_type, kPlatformTypeNSView) {
                return NativeParent::macos_ns_view(parent).map_err(|_| ());
            }

            #[cfg(target_os = "windows")]
            if fid_eq(platform_type, kPlatformTypeHWND) {
                return NativeParent::windows_hwnd(parent as isize).map_err(|_| ());
            }

            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            if fid_eq(platform_type, kPlatformTypeX11EmbedWindowID) {
                return NativeParent::xlib_window(parent as std::ffi::c_ulong).map_err(|_| ());
            }

            Err(())
        }
    }
}

impl IPlugViewTrait for VestyPlugView {
    unsafe fn isPlatformTypeSupported(&self, platform_type: FIDString) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if Self::platform_supported(platform_type) {
                kResultTrue
            } else {
                kResultFalse
            }
        }
    }

    unsafe fn attached(&self, parent: *mut c_void, platform_type: FIDString) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if !Self::platform_supported(platform_type) {
                #[cfg(feature = "wry-ui")]
                bridge_trace("editor_attach_unsupported_platform");
                return kResultFalse;
            }
            if parent.is_null() {
                #[cfg(feature = "wry-ui")]
                bridge_trace("editor_attach_unsupported_parent");
                return kResultFalse;
            }

            #[cfg(feature = "wry-ui")]
            {
                let Ok(parent) = self.native_parent(parent, platform_type) else {
                    bridge_trace("editor_attach_unsupported_parent");
                    return kResultFalse;
                };
                let runtime = &mut *self.runtime.get();
                let endpoint = self.bridge_endpoint.clone();
                let mut next = WryEditorRuntime::with_bridge_handler(endpoint.bridge_handler());
                if let Err(error) = next.attach(parent, &self.descriptor) {
                    bridge_trace(format!("editor_attach_error: {error}"));
                    return kResultFalse;
                }
                *runtime = Some(next);
                kResultTrue
            }

            #[cfg(not(feature = "wry-ui"))]
            {
                let _ = parent;
                kResultTrue
            }
        }
    }

    unsafe fn removed(&self) -> tresult {
        #[cfg(feature = "wry-ui")]
        {
            // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
            unsafe {
                if let Some(runtime) = (&mut *self.runtime.get()).as_mut() {
                    runtime.detach();
                }
                *self.runtime.get() = None;
            }
        }

        #[cfg(not(feature = "wry-ui"))]
        {
            let _ = self;
        }

        kResultOk
    }

    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyDown(&self, _key: char16, _key_code: i16, _modifiers: i16) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyUp(&self, _key: char16, _key_code: i16, _modifiers: i16) -> tresult {
        kResultFalse
    }

    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if size.is_null() {
                return kInvalidArgument;
            }
            *size = *self.size.get();
            kResultOk
        }
    }

    unsafe fn onSize(&self, new_size: *mut ViewRect) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if new_size.is_null() {
                return kInvalidArgument;
            }
            let mut rect = *new_size;
            self.clamp_rect(&mut rect);
            *self.size.get() = rect;

            #[cfg(feature = "wry-ui")]
            if let Some(runtime) = (&mut *self.runtime.get()).as_mut() {
                let _ = runtime.resize(EditorSize {
                    width: (rect.right - rect.left).max(0) as u32,
                    height: (rect.bottom - rect.top).max(0) as u32,
                });
            }

            kResultOk
        }
    }

    unsafe fn onFocus(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setFrame(&self, frame: *mut IPlugFrame) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            *self.frame.get() = frame;
            kResultOk
        }
    }

    unsafe fn canResize(&self) -> tresult {
        if self.descriptor.resizable {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn checkSizeConstraint(&self, rect: *mut ViewRect) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if rect.is_null() {
                return kInvalidArgument;
            }
            self.clamp_rect(&mut *rect);
            kResultOk
        }
    }
}

fn view_rect(width: u32, height: u32) -> ViewRect {
    ViewRect {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    }
}

fn resolve_ui_descriptor(mut descriptor: UiDescriptor) -> UiDescriptor {
    let assets_dir = PathBuf::from(&descriptor.assets_dir);
    if assets_dir.is_relative()
        && let Some(resources) = crate::bundle_resources_path()
    {
        descriptor.assets_dir = resources.join(assets_dir).to_string_lossy().into_owned();
    }
    descriptor
}

pub(crate) struct VestyController<P: Plugin + Default> {
    plugin: Arc<P>,
    specs: Vec<vesty_params::ParamSpec>,
    param_ids: Vst3ParamIds,
    handler: SharedComponentHandler,
    #[cfg(feature = "wry-ui")]
    bridge_snapshot: SharedBridgeSnapshot,
    #[cfg(feature = "wry-ui")]
    param_changes: SharedParamChangeQueue,
    meters: SharedMeterConsumer,
    logs: SharedLogConsumer,
    fault: SharedFaultState,
    telemetry_registry: Arc<Vst3TelemetryRegistry>,
    connection: SharedConnectionPoint,
}

impl<P: Plugin + Default> Class for VestyController<P> {
    type Interfaces = (
        IEditController,
        IConnectionPoint,
        IMidiMapping,
        IUnitInfo,
        IProgramListData,
        INoteExpressionController,
        INoteExpressionPhysicalUIMapping,
    );
}

// SAFETY: The controller itself is COM-owned and hosts may query interfaces across threads.
// Shared mutable handler state is guarded by a mutex and is never used from the audio thread.
unsafe impl<P: Plugin + Default> Sync for VestyController<P> {}

#[derive(Debug)]
pub(crate) enum VestyControllerInitError {
    ParamSchema,
    ParamIdCollision,
}

impl From<vesty_params::ParamSpecError> for VestyControllerInitError {
    fn from(_error: vesty_params::ParamSpecError) -> Self {
        Self::ParamSchema
    }
}

impl From<Vst3ParamIdCollision> for VestyControllerInitError {
    fn from(_error: Vst3ParamIdCollision) -> Self {
        Self::ParamIdCollision
    }
}

impl<P: Plugin + Default> VestyController<P> {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::with_telemetry_registry(Arc::new(Vst3TelemetryRegistry::default()))
    }

    #[cfg(test)]
    pub(crate) fn with_telemetry_registry(telemetry_registry: Arc<Vst3TelemetryRegistry>) -> Self {
        Self::try_with_telemetry_registry(telemetry_registry)
            .expect("plugin parameter specs should be valid")
    }

    pub(crate) fn try_with_telemetry_registry(
        telemetry_registry: Arc<Vst3TelemetryRegistry>,
    ) -> Result<Self, VestyControllerInitError> {
        let plugin = Arc::new(P::default());
        let specs = plugin.params().specs();
        vesty_params::validate_param_specs(&specs)?;
        let param_ids = Vst3ParamIds::try_from_specs(&specs)?;
        Ok(Self {
            plugin,
            specs,
            param_ids,
            handler: Arc::new(Mutex::new(None)),
            #[cfg(feature = "wry-ui")]
            bridge_snapshot: Arc::new(Mutex::new(SharedBridgeSnapshotState::default())),
            #[cfg(feature = "wry-ui")]
            param_changes: Arc::new(Mutex::new(BTreeMap::new())),
            meters: Arc::new(Mutex::new(None)),
            logs: Arc::new(Mutex::new(None)),
            fault: Arc::new(Mutex::new(None)),
            telemetry_registry,
            connection: Mutex::new(None),
        })
    }

    fn bind_telemetry_consumer(&self, telemetry_id: u64) -> tresult {
        let Some(channel) = self.telemetry_registry.take_channel(telemetry_id) else {
            return kResultFalse;
        };
        let Ok(mut slot) = self.meters.lock() else {
            return kResultFalse;
        };
        let Ok(mut log_slot) = self.logs.lock() else {
            return kResultFalse;
        };
        let Ok(mut fault_slot) = self.fault.lock() else {
            return kResultFalse;
        };
        *slot = Some(channel.meter_consumer);
        *log_slot = Some(channel.log_consumer);
        *fault_slot = Some(channel.fault);
        kResultOk
    }

    #[cfg(feature = "wry-ui")]
    fn bind_only_pending_telemetry_consumer(&self) {
        let Ok(mut slot) = self.meters.lock() else {
            return;
        };
        if slot.is_some() {
            return;
        }
        if let Some(channel) = self.telemetry_registry.take_only_channel() {
            let Ok(mut log_slot) = self.logs.lock() else {
                return;
            };
            let Ok(mut fault_slot) = self.fault.lock() else {
                return;
            };
            *slot = Some(channel.meter_consumer);
            *log_slot = Some(channel.log_consumer);
            *fault_slot = Some(channel.fault);
        }
    }

    #[cfg(test)]
    pub(crate) fn drain_meter_frames_for_test(&self) -> Vec<vesty_core::MeterFrame> {
        let mut frames = Vec::new();
        let Ok(mut slot) = self.meters.lock() else {
            return frames;
        };
        let Some(consumer) = slot.as_mut() else {
            return frames;
        };
        while let Ok(frame) = consumer.try_pop() {
            frames.push(frame);
        }
        frames
    }

    #[cfg(test)]
    pub(crate) fn drain_rt_log_events_for_test(&self) -> Vec<RtLogEvent> {
        let mut events = Vec::new();
        let Ok(mut slot) = self.logs.lock() else {
            return events;
        };
        let Some(consumer) = slot.as_mut() else {
            return events;
        };
        while let Ok(event) = consumer.try_pop() {
            events.push(event);
        }
        events
    }

    #[cfg(test)]
    pub(crate) fn param_id_for_test(&self, index: usize) -> Option<ParamID> {
        self.param_ids.host_id_for_index(index)
    }

    #[cfg(feature = "wry-ui")]
    pub(crate) fn bridge_endpoint(&self) -> WryBridgeEndpoint {
        self.bind_only_pending_telemetry_consumer();
        let info = P::INFO;
        let plugin = self.plugin.clone();
        let snapshot = self
            .bridge_snapshot
            .lock()
            .map(|slot| slot.snapshot.clone())
            .unwrap_or_default();
        WryBridgeEndpoint {
            ready: BridgeReadyPayload {
                protocol_version: 1,
                instance_id: info
                    .class_id
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
                editor_session_id: "pending".to_string(),
                dev_mode: cfg!(debug_assertions),
                plugin_name: info.name.to_string(),
                vendor: info.vendor.to_string(),
                capabilities: BridgeCapabilities::v1_default(),
                params: self.specs.clone(),
                snapshot,
            },
            param_ids: Arc::new(self.specs.iter().map(|spec| spec.id.clone()).collect()),
            param_host_ids: Arc::new(self.param_ids.host_ids.clone()),
            handler: self.handler.clone(),
            bridge_snapshot: self.bridge_snapshot.clone(),
            param_changes: self.param_changes.clone(),
            meters: self.meters.clone(),
            logs: self.logs.clone(),
            fault: self.fault.clone(),
            set_param: Arc::new(move |id, normalized| {
                let old = plugin.params().get_normalized(id).unwrap_or(normalized);
                match plugin.params().set_normalized(id, normalized) {
                    Ok(()) => {
                        let new = plugin.params().get_normalized(id).unwrap_or(normalized);
                        plugin.host_changes_for_param(id, old, new)
                    }
                    Err(_) => HostChangeFlags::NONE,
                }
            }),
        }
    }

    #[cfg(feature = "wry-ui")]
    fn bridge_snapshot_value(&self) -> Option<serde_json::Value> {
        let snapshot = self.bridge_snapshot.lock().ok()?.snapshot.clone();
        serde_json::to_value(snapshot).ok()
    }

    #[cfg(feature = "wry-ui")]
    fn restore_bridge_snapshot(&self, value: Option<serde_json::Value>) {
        let snapshot = value
            .and_then(|value| serde_json::from_value::<PluginSnapshot>(value).ok())
            .unwrap_or_default();
        if let Ok(mut slot) = self.bridge_snapshot.lock() {
            slot.snapshot = snapshot;
            slot.generation = slot.generation.saturating_add(1);
        }
    }

    fn set_param_and_host_changes(
        &self,
        spec: &vesty_params::ParamSpec,
        normalized: f64,
    ) -> Result<HostChangeFlags, vesty_params::ParamError> {
        if spec.flags.read_only {
            return Err(vesty_params::ParamError::ReadOnly(spec.id.clone()));
        }
        let old = self
            .plugin
            .params()
            .get_normalized(&spec.id)
            .unwrap_or(spec.default_normalized);
        self.plugin.params().set_normalized(&spec.id, normalized)?;
        let new = self
            .plugin
            .params()
            .get_normalized(&spec.id)
            .unwrap_or(normalized);
        Ok(self.plugin.host_changes_for_param(&spec.id, old, new))
    }

    fn apply_program_change_param(
        &self,
        spec: &vesty_params::ParamSpec,
        normalized: f64,
    ) -> Result<Option<ControllerParamApply>, vesty_params::ParamError> {
        if spec.flags.read_only {
            return Err(vesty_params::ParamError::ReadOnly(spec.id.clone()));
        }
        let Some((list_id, program_index)) =
            program_selection_for_param_value(self.plugin.as_ref(), spec, normalized)
        else {
            return Ok(None);
        };

        let before = self.capture_param_values();
        match self.plugin.apply_program(list_id, program_index) {
            Ok(true) => {
                let selected = plain_to_normalized(spec, program_index as f64);
                let _ = self.plugin.params().set_normalized(&spec.id, selected);
                #[cfg(feature = "wry-ui")]
                let changes = self.host_changes_for_program_delta(&before);
                #[cfg(not(feature = "wry-ui"))]
                let changes = self.host_changes_for_param_delta(&before);
                Ok(Some(ControllerParamApply::Program(changes)))
            }
            Ok(false) => Ok(None),
            Err(_) => Err(vesty_params::ParamError::Unknown(spec.id.clone())),
        }
    }

    fn apply_param_value_and_host_changes(
        &self,
        spec: &vesty_params::ParamSpec,
        normalized: f64,
    ) -> Result<ControllerParamApply, vesty_params::ParamError> {
        if let Some(changes) = self.apply_program_change_param(spec, normalized)? {
            return Ok(changes);
        }
        self.set_param_and_host_changes(spec, normalized)
            .map(ControllerParamApply::Param)
    }

    fn spec_for_host_id(&self, host_id: ParamID) -> Option<(usize, &vesty_params::ParamSpec)> {
        let index = self.param_ids.index_for_host_id(host_id)?;
        let spec = self.specs.get(index)?;
        Some((index, spec))
    }

    fn midi_mapping_param_id(
        &self,
        bus_index: int32,
        channel: int16,
        midi_controller_number: CtrlNumber,
    ) -> Option<ParamID> {
        if bus_index != 0 || channel < 0 || midi_controller_number < 0 {
            return None;
        }
        let channel = channel as u16;
        let controller = midi_controller_number as u16;
        self.specs
            .iter()
            .enumerate()
            .find(|(_, spec)| {
                spec.flags.automatable
                    && !spec.flags.read_only
                    && spec.midi_mappings.iter().any(|mapping| {
                        mapping.controller == controller
                            && mapping
                                .channel
                                .is_none_or(|mapped_channel| mapped_channel == channel)
                    })
            })
            .and_then(|(index, _)| self.param_ids.host_id_for_index(index))
    }

    #[cfg(feature = "wry-ui")]
    fn queue_param_change(&self, id: &str, normalized: f64, source: ParamChangeSource) {
        let Ok(mut changes) = self.param_changes.lock() else {
            return;
        };
        changes.insert(
            id.to_string(),
            QueuedParamChange {
                normalized: normalized.clamp(0.0, 1.0),
                source,
            },
        );
    }

    #[cfg(feature = "wry-ui")]
    fn queue_current_params(&self, source: ParamChangeSource) {
        for spec in &self.specs {
            let normalized = self
                .plugin
                .params()
                .get_normalized(&spec.id)
                .unwrap_or(spec.default_normalized);
            self.queue_param_change(&spec.id, normalized, source.clone());
        }
    }

    fn capture_param_values(&self) -> Vec<f64> {
        self.specs
            .iter()
            .map(|spec| {
                self.plugin
                    .params()
                    .get_normalized(&spec.id)
                    .unwrap_or(spec.default_normalized)
                    .clamp(0.0, 1.0)
            })
            .collect()
    }

    #[cfg(not(feature = "wry-ui"))]
    fn host_changes_for_param_delta(&self, before: &[f64]) -> HostChangeFlags {
        self.host_changes_for_param_delta_with_ui_source(
            before,
            #[cfg(feature = "wry-ui")]
            ParamChangeSource::Host,
        )
    }

    #[cfg(feature = "wry-ui")]
    fn host_changes_for_program_delta(&self, before: &[f64]) -> HostChangeFlags {
        self.host_changes_for_param_delta_with_ui_source(before, ParamChangeSource::Program)
    }

    fn host_changes_for_param_delta_with_ui_source(
        &self,
        before: &[f64],
        #[cfg(feature = "wry-ui")] source: ParamChangeSource,
    ) -> HostChangeFlags {
        let mut changes = HostChangeFlags::NONE;
        for (index, spec) in self.specs.iter().enumerate() {
            let old = before
                .get(index)
                .copied()
                .unwrap_or(spec.default_normalized)
                .clamp(0.0, 1.0);
            let new = self
                .plugin
                .params()
                .get_normalized(&spec.id)
                .unwrap_or(spec.default_normalized)
                .clamp(0.0, 1.0);
            if (old - new).abs() > f64::EPSILON {
                changes |= HostChangeFlags::PARAM_VALUES;
                #[cfg(feature = "wry-ui")]
                self.queue_param_change(&spec.id, new, source.clone());
                changes |= self.plugin.host_changes_for_param(&spec.id, old, new);
            }
        }
        changes
    }

    unsafe fn notify_program_param_delta(&self, before: &[f64]) -> tresult {
        #[cfg(feature = "wry-ui")]
        let changes = self.host_changes_for_program_delta(before);
        #[cfg(not(feature = "wry-ui"))]
        let changes = self.host_changes_for_param_delta(before);
        // SAFETY: Program data is applied on the controller side. Host restart notification is a
        // controller COM call and never runs from process().
        unsafe { self.notify_host_changes(changes) }
    }

    unsafe fn apply_program_data_value(
        &self,
        list_id: u32,
        program_index: usize,
        data: serde_json::Value,
    ) -> tresult {
        if !self.plugin.program_data_supported(list_id) {
            return kResultFalse;
        }
        let before = self.capture_param_values();
        match self.plugin.load_program_data(list_id, program_index, data) {
            Ok(true) => {
                // SAFETY: This helper is only called from controller-side program data callbacks.
                unsafe {
                    let _ = self.notify_program_param_delta(&before);
                }
                kResultOk
            }
            Ok(false) => kNotImplemented,
            Err(_) => kResultFalse,
        }
    }

    unsafe fn apply_program_data_stream(
        &self,
        expected_list_id: u32,
        expected_program_index: usize,
        data: *mut IBStream,
    ) -> tresult {
        // SAFETY: This block isolates host-provided IBStream parsing inside the controller boundary.
        unsafe {
            let Ok(program_data) = read_program_data_stream(data) else {
                return kInvalidArgument;
            };
            if program_data.list_id != expected_list_id
                || program_data.program_index != expected_program_index
            {
                return kInvalidArgument;
            }
            self.apply_program_data_value(
                expected_list_id,
                expected_program_index,
                program_data.data,
            )
        }
    }

    unsafe fn notify_host_changes(&self, changes: HostChangeFlags) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let flags = restart_flags_for_host_changes(changes);
            if flags == 0 {
                return kResultOk;
            }
            let Ok(handler) = self.handler.lock() else {
                return kResultFalse;
            };
            match handler.as_ref() {
                Some(handler) => handler.restartComponent(flags),
                None => kResultFalse,
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn begin_param_edit(&self, id: ParamID) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some((_, spec)) = self.spec_for_host_id(id) else {
                return kInvalidArgument;
            };
            if spec.flags.read_only {
                return kInvalidArgument;
            }
            let Ok(handler) = self.handler.lock() else {
                return kResultFalse;
            };
            match handler.as_ref() {
                Some(handler) => handler.beginEdit(id),
                None => kResultFalse,
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn perform_param_edit(&self, id: ParamID, normalized: f64) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some((_, spec)) = self.spec_for_host_id(id) else {
                return kInvalidArgument;
            };
            let normalized = normalized.clamp(0.0, 1.0);
            let apply = match self.apply_param_value_and_host_changes(spec, normalized) {
                Ok(apply) => apply,
                Err(_) => return kInvalidArgument,
            };
            let changes = apply.host_changes();
            let Ok(handler) = self.handler.lock() else {
                return kResultFalse;
            };
            match handler.as_ref() {
                Some(handler) => {
                    let result = handler.performEdit(id, normalized);
                    if result == kResultOk {
                        let _ = restart_component_for_host_changes(handler, changes);
                    }
                    result
                }
                None => kResultFalse,
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn end_param_edit(&self, id: ParamID) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some((_, spec)) = self.spec_for_host_id(id) else {
                return kInvalidArgument;
            };
            if spec.flags.read_only {
                return kInvalidArgument;
            }
            let Ok(handler) = self.handler.lock() else {
                return kResultFalse;
            };
            match handler.as_ref() {
                Some(handler) => handler.endEdit(id),
                None => kResultFalse,
            }
        }
    }
}

impl<P: Plugin + Default> IMidiMappingTrait for VestyController<P> {
    unsafe fn getMidiControllerAssignment(
        &self,
        busIndex: int32,
        channel: int16,
        midiControllerNumber: CtrlNumber,
        id: *mut ParamID,
    ) -> tresult {
        if id.is_null() {
            return kInvalidArgument;
        }
        let Some(param_id) = self.midi_mapping_param_id(busIndex, channel, midiControllerNumber)
        else {
            return kResultFalse;
        };
        // SAFETY: The host supplied a non-null out pointer for the duration of this callback.
        unsafe {
            *id = param_id;
        }
        kResultTrue
    }
}

impl<P: Plugin + Default> IUnitInfoTrait for VestyController<P> {
    unsafe fn getUnitCount(&self) -> int32 {
        1
    }

    unsafe fn getUnitInfo(&self, unitIndex: int32, info: *mut UnitInfo) -> tresult {
        if unitIndex != ROOT_UNIT_INDEX || info.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: The host supplied a non-null out pointer for the duration of this callback.
        unsafe {
            let info = &mut *info;
            info.id = kRootUnitId;
            info.parentUnitId = kNoParentUnitId;
            info.programListId =
                program_list_by_index(self.plugin.as_ref(), ROOT_UNIT_PROGRAM_LIST_INDEX as int32)
                    .map(|list| list.id as ProgramListID)
                    .unwrap_or(kNoProgramListId);
            copy_wstring(P::INFO.name, &mut info.name);
        }
        kResultOk
    }

    unsafe fn getProgramListCount(&self) -> int32 {
        visible_program_lists(self.plugin.as_ref()).count() as int32
    }

    unsafe fn getProgramListInfo(&self, listIndex: int32, info: *mut ProgramListInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }
        let Some(list) = program_list_by_index(self.plugin.as_ref(), listIndex) else {
            return kInvalidArgument;
        };
        // SAFETY: The host supplied a non-null out pointer for the duration of this callback.
        unsafe {
            let info = &mut *info;
            info.id = list.id as ProgramListID;
            info.programCount = list.programs.len() as int32;
            copy_wstring(list.name, &mut info.name);
        }
        kResultOk
    }

    unsafe fn getProgramName(
        &self,
        listId: ProgramListID,
        programIndex: int32,
        name: *mut String128,
    ) -> tresult {
        if name.is_null() || programIndex < 0 {
            return kInvalidArgument;
        }
        let Some(program) = program_list_by_id(self.plugin.as_ref(), listId)
            .and_then(|list| list.programs.get(programIndex as usize))
        else {
            return kInvalidArgument;
        };
        // SAFETY: The host supplied a non-null out pointer for the duration of this callback.
        unsafe {
            copy_wstring(program.name, &mut *name);
        }
        kResultOk
    }

    unsafe fn getProgramInfo(
        &self,
        listId: ProgramListID,
        programIndex: int32,
        attributeId: FIDString,
        attributeValue: *mut String128,
    ) -> tresult {
        if attributeId.is_null() || attributeValue.is_null() {
            return kInvalidArgument;
        }
        let Some((list_id, program_index)) =
            program_selection_by_id(self.plugin.as_ref(), listId, programIndex)
        else {
            return kInvalidArgument;
        };

        // SAFETY: The host supplied a non-null, nul-terminated attribute id pointer for the
        // duration of this controller callback.
        let attribute_id = unsafe { CStr::from_ptr(attributeId).to_bytes() };
        let Some(attribute) =
            visible_program_attributes(self.plugin.as_ref(), list_id, program_index)
                .find(|attribute| attribute.id.as_bytes() == attribute_id)
        else {
            return kResultFalse;
        };

        // SAFETY: The host supplied a non-null output String128 pointer for this callback.
        unsafe {
            copy_wstring(attribute.value, &mut *attributeValue);
        }
        kResultOk
    }

    unsafe fn hasProgramPitchNames(&self, listId: ProgramListID, programIndex: int32) -> tresult {
        let Some((list_id, program_index)) =
            program_selection_by_id(self.plugin.as_ref(), listId, programIndex)
        else {
            return kInvalidArgument;
        };
        if visible_program_pitch_names(self.plugin.as_ref(), list_id, program_index)
            .next()
            .is_some()
        {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn getProgramPitchName(
        &self,
        listId: ProgramListID,
        programIndex: int32,
        midiPitch: int16,
        name: *mut String128,
    ) -> tresult {
        if name.is_null() || !(0..=127).contains(&midiPitch) {
            return kInvalidArgument;
        }
        let Some((list_id, program_index)) =
            program_selection_by_id(self.plugin.as_ref(), listId, programIndex)
        else {
            return kInvalidArgument;
        };
        let Some(pitch) = visible_program_pitch_names(self.plugin.as_ref(), list_id, program_index)
            .find(|pitch| pitch.midi_pitch == midiPitch)
        else {
            return kResultFalse;
        };
        // SAFETY: The host supplied a non-null output String128 pointer for this callback.
        unsafe {
            copy_wstring(pitch.name, &mut *name);
        }
        kResultOk
    }

    unsafe fn getSelectedUnit(&self) -> UnitID {
        kRootUnitId
    }

    unsafe fn selectUnit(&self, unitId: UnitID) -> tresult {
        if unitId == kRootUnitId {
            kResultOk
        } else {
            kInvalidArgument
        }
    }

    unsafe fn getUnitByBus(
        &self,
        r#type: MediaType,
        dir: BusDirection,
        busIndex: int32,
        channel: int32,
        unitId: *mut UnitID,
    ) -> tresult {
        if unitId.is_null() || busIndex < 0 || channel < -1 {
            return kInvalidArgument;
        }
        let valid = match (r#type as MediaTypes, dir as BusDirections) {
            (MediaTypes_::kAudio, BusDirections_::kOutput) => {
                (busIndex as usize) < output_bus_count(self.plugin.as_ref())
            }
            (MediaTypes_::kAudio, BusDirections_::kInput)
                if P::INFO.kind != vesty_core::PluginKind::Instrument =>
            {
                busIndex == 0 || (busIndex == 1 && supports_sidechain(self.plugin.as_ref()))
            }
            (MediaTypes_::kEvent, BusDirections_::kInput)
                if P::INFO.kind == vesty_core::PluginKind::Instrument =>
            {
                busIndex == 0
            }
            _ => false,
        };
        if !valid {
            return kInvalidArgument;
        }
        // SAFETY: The host supplied a non-null out pointer for the duration of this callback.
        unsafe {
            *unitId = kRootUnitId;
        }
        kResultOk
    }

    unsafe fn setUnitProgramData(
        &self,
        listOrUnitId: int32,
        programIndex: int32,
        data: *mut IBStream,
    ) -> tresult {
        let Some((list_id, program_index)) =
            program_selection_by_id_or_root(self.plugin.as_ref(), listOrUnitId, programIndex)
        else {
            return kInvalidArgument;
        };

        if !data.is_null() {
            // SAFETY: Host-provided program data is parsed and applied on the controller side.
            return unsafe { self.apply_program_data_stream(list_id, program_index, data) };
        }

        let before = self.capture_param_values();
        match self.plugin.apply_program(list_id, program_index) {
            Ok(true) => {
                // SAFETY: This callback is already executing at the VST3 controller boundary.
                unsafe {
                    let _ = self.notify_program_param_delta(&before);
                }
                kResultOk
            }
            Ok(false) => kNotImplemented,
            Err(_) => kResultFalse,
        }
    }
}

impl<P: Plugin + Default> IProgramListDataTrait for VestyController<P> {
    unsafe fn programDataSupported(&self, listId: ProgramListID) -> tresult {
        let Some(list) = program_list_by_id(self.plugin.as_ref(), listId) else {
            return kInvalidArgument;
        };
        if self.plugin.program_data_supported(list.id) {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn getProgramData(
        &self,
        listId: ProgramListID,
        programIndex: int32,
        data: *mut IBStream,
    ) -> tresult {
        if data.is_null() {
            return kInvalidArgument;
        }
        let Some((list_id, program_index)) =
            program_selection_by_id(self.plugin.as_ref(), listId, programIndex)
        else {
            return kInvalidArgument;
        };
        if !self.plugin.program_data_supported(list_id) {
            return kResultFalse;
        }
        let payload = match self.plugin.save_program_data(list_id, program_index) {
            Ok(Some(payload)) => payload,
            Ok(None) => return kNotImplemented,
            Err(_) => return kResultFalse,
        };
        let program_data = Vst3ProgramData {
            version: VST3_PROGRAM_DATA_VERSION,
            list_id,
            program_index,
            data: payload,
        };
        // SAFETY: Host-provided IBStream is non-null and this callback runs on the controller side.
        match unsafe { write_program_data_stream(data, &program_data) } {
            Ok(()) => kResultOk,
            Err(()) => kInvalidArgument,
        }
    }

    unsafe fn setProgramData(
        &self,
        listId: ProgramListID,
        programIndex: int32,
        data: *mut IBStream,
    ) -> tresult {
        if data.is_null() {
            return kInvalidArgument;
        }
        let Some((list_id, program_index)) =
            program_selection_by_id(self.plugin.as_ref(), listId, programIndex)
        else {
            return kInvalidArgument;
        };
        // SAFETY: Host-provided program data is parsed and applied on the controller side.
        unsafe { self.apply_program_data_stream(list_id, program_index, data) }
    }
}

impl<P: Plugin + Default> INoteExpressionControllerTrait for VestyController<P> {
    unsafe fn getNoteExpressionCount(&self, busIndex: int32, channel: int16) -> int32 {
        if !note_expression_bus_channel_valid::<P>(busIndex, channel) {
            return 0;
        }
        visible_note_expression_value_types(self.plugin.as_ref()).count() as int32
    }

    unsafe fn getNoteExpressionInfo(
        &self,
        busIndex: int32,
        channel: int16,
        noteExpressionIndex: int32,
        info: *mut NoteExpressionTypeInfo,
    ) -> tresult {
        if info.is_null() || !note_expression_bus_channel_valid::<P>(busIndex, channel) {
            return kInvalidArgument;
        }
        let Some(expression) =
            note_expression_value_type_by_index(self.plugin.as_ref(), noteExpressionIndex)
        else {
            return kInvalidArgument;
        };
        // SAFETY: The host supplied a non-null out pointer for the duration of this callback.
        unsafe {
            let info = &mut *info;
            info.typeId = expression.type_id;
            copy_wstring(expression.title, &mut info.title);
            copy_wstring(expression.short_title, &mut info.shortTitle);
            copy_wstring(expression.units, &mut info.units);
            info.unitId = kRootUnitId;
            info.valueDesc.defaultValue = expression.default_value;
            info.valueDesc.minimum = expression.minimum;
            info.valueDesc.maximum = expression.maximum;
            info.valueDesc.stepCount = expression.step_count;
            info.associatedParameterId = kNoParamId;
            info.flags = note_expression_type_flags(expression.flags);
        }
        kResultOk
    }

    unsafe fn getNoteExpressionStringByValue(
        &self,
        busIndex: int32,
        channel: int16,
        id: NoteExpressionTypeID,
        valueNormalized: NoteExpressionValue,
        string: *mut String128,
    ) -> tresult {
        if string.is_null()
            || !note_expression_bus_channel_valid::<P>(busIndex, channel)
            || valueNormalized.is_nan()
            || note_expression_value_type_by_id(self.plugin.as_ref(), id).is_none()
        {
            return kInvalidArgument;
        }
        // SAFETY: The host supplied a non-null out pointer for the duration of this callback.
        unsafe {
            copy_wstring(&valueNormalized.clamp(0.0, 1.0).to_string(), &mut *string);
        }
        kResultOk
    }

    unsafe fn getNoteExpressionValueByString(
        &self,
        busIndex: int32,
        channel: int16,
        id: NoteExpressionTypeID,
        string: *const TChar,
        valueNormalized: *mut NoteExpressionValue,
    ) -> tresult {
        if string.is_null()
            || valueNormalized.is_null()
            || !note_expression_bus_channel_valid::<P>(busIndex, channel)
            || note_expression_value_type_by_id(self.plugin.as_ref(), id).is_none()
        {
            return kInvalidArgument;
        }
        // SAFETY: The host supplied non-null pointers for the duration of this callback.
        unsafe {
            let Ok(text) = string128_to_string_lossy(string) else {
                return kInvalidArgument;
            };
            let Ok(value) = text.trim().parse::<f64>() else {
                return kInvalidArgument;
            };
            if !value.is_finite() {
                return kInvalidArgument;
            }
            *valueNormalized = value.clamp(0.0, 1.0);
        }
        kResultOk
    }
}

impl<P: Plugin + Default> INoteExpressionPhysicalUIMappingTrait for VestyController<P> {
    unsafe fn getPhysicalUIMapping(
        &self,
        busIndex: int32,
        channel: int16,
        list: *mut PhysicalUIMapList,
    ) -> tresult {
        if list.is_null() || !note_expression_bus_channel_valid::<P>(busIndex, channel) {
            return kInvalidArgument;
        }

        // SAFETY: The host supplied a non-null list pointer for the duration of this callback.
        unsafe {
            let list = &mut *list;
            if list.count > 0 && list.map.is_null() {
                return kInvalidArgument;
            }

            let capacity = list.count as usize;
            let mut written = 0_usize;
            for mapping in visible_note_expression_physical_ui_mappings(self.plugin.as_ref()) {
                if written >= capacity {
                    break;
                }
                let out = &mut *list.map.add(written);
                out.physicalUITypeID = mapping.physical_ui_type_id;
                out.noteExpressionTypeID = mapping.note_expression_type_id;
                written += 1;
            }
            list.count = written as uint32;
        }

        kResultOk
    }
}

impl<P: Plugin + Default> IPluginBaseTrait for VestyController<P> {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl<P: Plugin + Default> IEditControllerTrait for VestyController<P> {
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            match read_state_stream(state) {
                Ok(state) => {
                    #[cfg(feature = "wry-ui")]
                    let bridge = state.bridge.clone();
                    if apply_state(self.plugin.as_ref(), state).is_ok() {
                        #[cfg(feature = "wry-ui")]
                        self.restore_bridge_snapshot(bridge);
                        #[cfg(feature = "wry-ui")]
                        self.queue_current_params(ParamChangeSource::State);
                        kResultOk
                    } else {
                        kResultFalse
                    }
                }
                Err(()) => kInvalidArgument,
            }
        }
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            match read_state_stream(state) {
                Ok(state) => {
                    #[cfg(feature = "wry-ui")]
                    let bridge = state.bridge.clone();
                    if apply_state(self.plugin.as_ref(), state).is_ok() {
                        #[cfg(feature = "wry-ui")]
                        self.restore_bridge_snapshot(bridge);
                        #[cfg(feature = "wry-ui")]
                        self.queue_current_params(ParamChangeSource::State);
                        kResultOk
                    } else {
                        kResultFalse
                    }
                }
                Err(()) => kInvalidArgument,
            }
        }
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            #[cfg(feature = "wry-ui")]
            let captured =
                capture_state_with_bridge(self.plugin.as_ref(), self.bridge_snapshot_value());
            #[cfg(not(feature = "wry-ui"))]
            let captured = capture_state(self.plugin.as_ref());
            match captured {
                Ok(captured) => match write_state_stream(state, &captured) {
                    Ok(()) => kResultOk,
                    Err(()) => kInvalidArgument,
                },
                Err(_) => kResultFalse,
            }
        }
    }

    unsafe fn getParameterCount(&self) -> i32 {
        self.specs.len() as i32
    }

    unsafe fn getParameterInfo(&self, param_index: i32, info: *mut ParameterInfo) -> tresult {
        if param_index < 0 || info.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; the nullable output pointer has been checked above.
        unsafe {
            let Some(spec) = self.specs.get(param_index as usize) else {
                return kInvalidArgument;
            };
            let info = &mut *info;
            let Some(host_id) = self.param_ids.host_id_for_index(param_index as usize) else {
                return kInvalidArgument;
            };
            info.id = host_id;
            copy_wstring(&spec.name, &mut info.title);
            copy_wstring(&spec.name, &mut info.shortTitle);
            copy_wstring(spec.unit.as_deref().unwrap_or(""), &mut info.units);
            info.stepCount = spec.step_count.unwrap_or(0) as i32;
            info.defaultNormalizedValue = spec.default_normalized;
            info.unitId = 0;
            info.flags = if spec.flags.automatable && !spec.flags.read_only {
                ParameterInfo_::ParameterFlags_::kCanAutomate
            } else {
                0
            };
            if spec.flags.bypass {
                info.flags |= ParameterInfo_::ParameterFlags_::kIsBypass;
            }
            if spec.flags.program_change {
                info.flags |= ParameterInfo_::ParameterFlags_::kIsProgramChange;
            }
            kResultOk
        }
    }

    unsafe fn getParamStringByValue(
        &self,
        id: u32,
        value_normalized: f64,
        string: *mut String128,
    ) -> tresult {
        if string.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; the nullable output pointer has been checked above.
        unsafe {
            let Some((_, spec)) = self.spec_for_host_id(id) else {
                return kInvalidArgument;
            };
            copy_wstring(
                &format_normalized_value(spec, value_normalized),
                &mut *string,
            );
            kResultOk
        }
    }

    unsafe fn getParamValueByString(
        &self,
        id: u32,
        string: *mut TChar,
        value_normalized: *mut f64,
    ) -> tresult {
        if string.is_null() || value_normalized.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; nullable host pointers have been checked above.
        unsafe {
            let Some((_, spec)) = self.spec_for_host_id(id) else {
                return kInvalidArgument;
            };
            let Ok(text) = string128_to_string_lossy(string as *const TChar) else {
                return kInvalidArgument;
            };
            let Some(value) = parse_normalized_value(spec, &text) else {
                return kInvalidArgument;
            };
            *value_normalized = value;
            kResultOk
        }
    }

    unsafe fn normalizedParamToPlain(&self, id: u32, value_normalized: f64) -> f64 {
        self.spec_for_host_id(id)
            .map(|(_, spec)| spec)
            .map(|spec| normalized_to_plain(spec, value_normalized))
            .unwrap_or(0.0)
    }

    unsafe fn plainParamToNormalized(&self, id: u32, plain_value: f64) -> f64 {
        self.spec_for_host_id(id)
            .map(|(_, spec)| spec)
            .map(|spec| plain_to_normalized(spec, plain_value))
            .unwrap_or(0.0)
    }

    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        let Some((_, spec)) = self.spec_for_host_id(id) else {
            return 0.0;
        };
        self.plugin
            .params()
            .get_normalized(&spec.id)
            .unwrap_or(spec.default_normalized)
    }

    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some((_, spec)) = self.spec_for_host_id(id) else {
                return kInvalidArgument;
            };
            match self.apply_param_value_and_host_changes(spec, value.clamp(0.0, 1.0)) {
                Ok(apply) => {
                    #[cfg(feature = "wry-ui")]
                    {
                        if !apply.is_program() {
                            let normalized = self
                                .plugin
                                .params()
                                .get_normalized(&spec.id)
                                .unwrap_or(value.clamp(0.0, 1.0));
                            self.queue_param_change(&spec.id, normalized, ParamChangeSource::Host);
                        }
                    }
                    let changes = apply.host_changes();
                    let _ = self.notify_host_changes(changes);
                    kResultOk
                }
                Err(_) => kInvalidArgument,
            }
        }
    }

    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            #[cfg(feature = "wry-ui")]
            bridge_trace(format!("setComponentHandler: null={}", handler.is_null()));
            let Ok(mut slot) = self.handler.lock() else {
                return kResultFalse;
            };
            *slot = if handler.is_null() {
                None
            } else {
                <IComponentHandler as vst3::com_scrape_types::Unknown>::add_ref(handler);
                ComPtr::from_raw(handler)
            };
            kResultOk
        }
    }

    unsafe fn createView(&self, name: *const c_char) -> *mut IPlugView {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if !name.is_null() && !fid_eq(name, c"editor".as_ptr() as FIDString) {
                return std::ptr::null_mut();
            }
            let Some(descriptor) = self.plugin.ui().map(resolve_ui_descriptor) else {
                return std::ptr::null_mut();
            };
            #[cfg(feature = "wry-ui")]
            let view = VestyPlugView::new(descriptor, self.bridge_endpoint());
            #[cfg(not(feature = "wry-ui"))]
            let view = VestyPlugView::new(descriptor);
            ComWrapper::new(view)
                .to_com_ptr::<IPlugView>()
                .map(ComPtr::into_raw)
                .unwrap_or(std::ptr::null_mut())
        }
    }
}

impl<P: Plugin + Default> IConnectionPointTrait for VestyController<P> {
    unsafe fn connect(&self, other: *mut IConnectionPoint) -> tresult {
        connect_connection_point(&self.connection, other)
    }

    unsafe fn disconnect(&self, other: *mut IConnectionPoint) -> tresult {
        disconnect_connection_point(&self.connection, other)
    }

    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if let Some(telemetry_id) = telemetry_id_from_message(message) {
                return self.bind_telemetry_consumer(telemetry_id);
            }
            kResultOk
        }
    }
}

unsafe fn telemetry_id_from_message(message: *mut IMessage) -> Option<u64> {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let message = ComRef::from_raw(message)?;
        let message_id = message.getMessageID();
        if message_id.is_null()
            || CStr::from_ptr(message_id).to_bytes_with_nul() != TELEMETRY_BIND_MESSAGE_ID
        {
            return None;
        }
        let attributes = ComRef::from_raw(message.getAttributes())?;
        let mut telemetry_id = 0;
        if attributes.getInt(TELEMETRY_ID_ATTR.as_ptr() as IAttrID, &mut telemetry_id) != kResultOk
        {
            return None;
        }
        (telemetry_id > 0).then_some(telemetry_id as u64)
    }
}

struct VestyFactory<P: Plugin + Default> {
    telemetry_registry: Arc<Vst3TelemetryRegistry>,
    _marker: PhantomData<P>,
}

impl<P: Plugin + Default> Class for VestyFactory<P> {
    type Interfaces = (IPluginFactory,);
}

impl<P: Plugin + Default> IPluginFactoryTrait for VestyFactory<P> {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; the nullable output pointer has been checked above.
        unsafe {
            let info = &mut *info;
            copy_cstring(P::INFO.vendor, &mut info.vendor);
            copy_cstring(P::INFO.url, &mut info.url);
            copy_cstring(P::INFO.email, &mut info.email);
            info.flags = PFactoryInfo_::FactoryFlags_::kUnicode as int32;
            kResultOk
        }
    }

    unsafe fn countClasses(&self) -> i32 {
        2
    }

    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        if info.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; the nullable output pointer has been checked above.
        unsafe {
            let info = &mut *info;
            match index {
                0 => {
                    info.cid = processor_cid::<P>();
                    info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as int32;
                    copy_cstring("Audio Module Class", &mut info.category);
                    copy_cstring(P::INFO.name, &mut info.name);
                    kResultOk
                }
                1 => {
                    info.cid = controller_cid::<P>();
                    info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as int32;
                    copy_cstring("Component Controller Class", &mut info.category);
                    copy_cstring(P::INFO.name, &mut info.name);
                    kResultOk
                }
                _ => kInvalidArgument,
            }
        }
    }

    unsafe fn createInstance(
        &self,
        cid: FIDString,
        iid: FIDString,
        obj: *mut *mut c_void,
    ) -> tresult {
        if obj.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: `obj` was checked for null above and points to the caller-provided instance output slot.
        unsafe {
            *obj = std::ptr::null_mut();
        }
        if cid.is_null() || iid.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; nullable host pointers have been checked above and the output pointer has been initialized to null for failure paths.
        unsafe {
            let cid = *(cid as *const TUID);
            let instance = if cid == processor_cid::<P>() {
                let Ok(processor) = VestyProcessor::<P>::try_with_telemetry_registry(
                    self.telemetry_registry.clone(),
                ) else {
                    return kResultFalse;
                };
                ComWrapper::new(processor).to_com_ptr::<FUnknown>()
            } else if cid == controller_cid::<P>() {
                let Ok(controller) = VestyController::<P>::try_with_telemetry_registry(
                    self.telemetry_registry.clone(),
                ) else {
                    return kResultFalse;
                };
                ComWrapper::new(controller).to_com_ptr::<FUnknown>()
            } else {
                None
            };

            if let Some(instance) = instance {
                let ptr = instance.as_ptr();
                ((*(*ptr).vtbl).queryInterface)(ptr, iid as *mut TUID, obj)
            } else if cid == processor_cid::<P>() || cid == controller_cid::<P>() {
                kResultFalse
            } else {
                kInvalidArgument
            }
        }
    }
}

pub fn create_plugin_factory<P>() -> *mut IPluginFactory
where
    P: Plugin + Default,
{
    ComWrapper::new(VestyFactory::<P> {
        telemetry_registry: Arc::new(Vst3TelemetryRegistry::default()),
        _marker: PhantomData,
    })
    .to_com_ptr::<IPluginFactory>()
    .map(ComPtr::into_raw)
    .unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vesty_params::{ParamHandle, ParamSpec};

    #[test]
    fn vst3_state_migration_accepts_v1_and_rejects_future_versions() {
        let state = Vst3State {
            version: VST3_STATE_VERSION,
            params: vec![ParamState {
                id: "gain".to_string(),
                normalized: 0.5,
            }],
            custom: None,
            bridge: None,
        };
        assert_eq!(migrate_vst3_state(state).unwrap().params[0].id, "gain");

        let future = Vst3State {
            version: VST3_STATE_VERSION + 1,
            params: Vec::new(),
            custom: None,
            bridge: None,
        };
        let error = migrate_vst3_state(future).unwrap_err().to_string();
        assert!(error.contains("unsupported VST3 state version"));
    }

    #[test]
    fn stable_vst3_param_ids_are_derived_from_string_ids() {
        let gain = ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5);
        let mode = ParamSpec::choice("mode", "Mode", ["Clean", "Drive"], 0);
        let first = Vst3ParamIds::try_from_specs(&[gain.clone(), mode.clone()]).unwrap();
        let second = Vst3ParamIds::try_from_specs(&[mode, gain]).unwrap();

        let gain_host_id = stable_vst3_param_id("gain");
        let mode_host_id = stable_vst3_param_id("mode");
        assert_ne!(gain_host_id, 0);
        assert_ne!(gain_host_id, mode_host_id);
        assert_eq!(first.host_id_for_index(0), Some(gain_host_id));
        assert_eq!(first.index_for_host_id(gain_host_id), Some(0));
        assert_eq!(second.host_id_for_index(1), Some(gain_host_id));
        assert_eq!(second.index_for_host_id(gain_host_id), Some(1));
    }

    #[test]
    fn stable_vst3_param_id_registry_rejects_collisions() {
        let first = ParamSpec::float("gain", "Gain", 0.0, 1.0, 0.5);
        let second = ParamSpec::float("gain", "Duplicate Gain", 0.0, 1.0, 0.5);
        let error = Vst3ParamIds::try_from_specs(&[first, second]).unwrap_err();
        assert_eq!(error.host_id, stable_vst3_param_id("gain"));
        assert_eq!(error.first_id, "gain");
        assert_eq!(error.second_id, "gain");
    }

    #[test]
    fn attribute_list_roundtrips_int_float_string_and_binary() {
        let list = VestyAttributeList::default();

        let mut int_value = 0;
        let mut float_value = 0.0;
        let string_value: Vec<TChar> = "Vesty"
            .encode_utf16()
            .map(|unit| unit as TChar)
            .chain(std::iter::once(0))
            .collect();
        let binary_value = [1_u8, 3, 5, 8, 13];

        // SAFETY: The test passes valid nul-terminated attribute IDs, output pointers, and
        // UTF-16 string/binary buffers for the duration of each direct COM trait call.
        unsafe {
            assert_eq!(list.setInt(c"int".as_ptr() as IAttrID, 42), kResultOk);
            assert_eq!(
                list.getInt(c"int".as_ptr() as IAttrID, &mut int_value),
                kResultOk
            );
            assert_eq!(int_value, 42);

            assert_eq!(
                list.setFloat(c"float".as_ptr() as IAttrID, 0.625),
                kResultOk
            );
            assert_eq!(
                list.getFloat(c"float".as_ptr() as IAttrID, &mut float_value),
                kResultOk
            );
            assert_eq!(float_value, 0.625);

            assert_eq!(
                list.setString(c"string".as_ptr() as IAttrID, string_value.as_ptr()),
                kResultOk
            );
            let mut string_out = [0 as TChar; 16];
            assert_eq!(
                list.getString(
                    c"string".as_ptr() as IAttrID,
                    string_out.as_mut_ptr(),
                    (string_out.len() * size_of::<TChar>()) as uint32,
                ),
                kResultOk
            );
            assert_eq!(&string_out[..string_value.len()], string_value.as_slice());

            let mut tiny_string_out = [99 as TChar; 3];
            assert_eq!(
                list.getString(
                    c"string".as_ptr() as IAttrID,
                    tiny_string_out.as_mut_ptr(),
                    (tiny_string_out.len() * size_of::<TChar>()) as uint32,
                ),
                kResultFalse
            );
            assert_eq!(tiny_string_out.last().copied(), Some(0));

            assert_eq!(
                list.setBinary(
                    c"binary".as_ptr() as IAttrID,
                    binary_value.as_ptr() as *const c_void,
                    binary_value.len() as uint32,
                ),
                kResultOk
            );
            let mut data = std::ptr::null();
            let mut size = 0;
            assert_eq!(
                list.getBinary(c"binary".as_ptr() as IAttrID, &mut data, &mut size),
                kResultOk
            );
            assert_eq!(size as usize, binary_value.len());
            assert_eq!(
                slice::from_raw_parts(data as *const u8, size as usize),
                binary_value
            );
        }
    }

    #[test]
    fn attribute_list_rejects_invalid_pointers_and_missing_values() {
        let list = VestyAttributeList::default();
        let string_value: Vec<TChar> = "Vesty"
            .encode_utf16()
            .map(|unit| unit as TChar)
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: The test intentionally passes null pointers to verify defensive COM boundary
        // handling; non-null inputs are valid for the duration of each direct trait call.
        unsafe {
            assert_eq!(list.setInt(std::ptr::null(), 42), kInvalidArgument);
            assert_eq!(
                list.getInt(c"missing".as_ptr() as IAttrID, std::ptr::null_mut()),
                kInvalidArgument
            );
            let mut int_value = 0;
            assert_eq!(
                list.getInt(c"missing".as_ptr() as IAttrID, &mut int_value),
                kInvalidArgument
            );

            assert_eq!(list.setFloat(std::ptr::null(), 1.0), kInvalidArgument);
            assert_eq!(
                list.getFloat(c"missing".as_ptr() as IAttrID, std::ptr::null_mut()),
                kInvalidArgument
            );
            let mut float_value = 0.0;
            assert_eq!(
                list.getFloat(c"missing".as_ptr() as IAttrID, &mut float_value),
                kInvalidArgument
            );

            assert_eq!(
                list.setString(c"string".as_ptr() as IAttrID, std::ptr::null()),
                kInvalidArgument
            );
            assert_eq!(
                list.setString(std::ptr::null(), string_value.as_ptr()),
                kInvalidArgument
            );
            let mut string_out = [0 as TChar; 4];
            assert_eq!(
                list.getString(
                    c"missing".as_ptr() as IAttrID,
                    string_out.as_mut_ptr(),
                    (string_out.len() * size_of::<TChar>()) as uint32,
                ),
                kInvalidArgument
            );
            assert_eq!(
                list.getString(
                    c"missing".as_ptr() as IAttrID,
                    std::ptr::null_mut(),
                    (string_out.len() * size_of::<TChar>()) as uint32,
                ),
                kInvalidArgument
            );
            assert_eq!(
                list.getString(c"missing".as_ptr() as IAttrID, string_out.as_mut_ptr(), 1,),
                kInvalidArgument
            );

            assert_eq!(
                list.setBinary(c"binary".as_ptr() as IAttrID, std::ptr::null(), 1),
                kInvalidArgument
            );
            assert_eq!(
                list.setBinary(c"empty".as_ptr() as IAttrID, std::ptr::null(), 0),
                kResultOk
            );
            let mut data = std::ptr::null();
            let mut size = uint32::MAX;
            assert_eq!(
                list.getBinary(c"empty".as_ptr() as IAttrID, &mut data, &mut size),
                kResultOk
            );
            assert_eq!(size, 0);
            assert!(data.is_null());
            assert_eq!(
                list.getBinary(c"missing".as_ptr() as IAttrID, &mut data, &mut size),
                kInvalidArgument
            );
            assert_eq!(
                list.getBinary(
                    c"empty".as_ptr() as IAttrID,
                    std::ptr::null_mut(),
                    &mut size,
                ),
                kInvalidArgument
            );
            assert_eq!(
                list.getBinary(
                    c"empty".as_ptr() as IAttrID,
                    &mut data,
                    std::ptr::null_mut(),
                ),
                kInvalidArgument
            );
        }
    }

    #[test]
    fn events_are_sorted_by_sample_offset_stably() {
        let mut events = FixedEventList::<VestyEvent, MAX_BLOCK_EVENTS>::new();
        let gain = ParamHandle::from_index(0);
        events
            .push(VestyEvent::Param {
                sample_offset: 16,
                handle: gain,
                id_hash: 1,
                normalized: 0.25,
            })
            .unwrap();
        events
            .push(VestyEvent::NoteOn {
                sample_offset: 4,
                channel: 0,
                key: 60,
                velocity: 1.0,
                note_id: -1,
            })
            .unwrap();
        events
            .push(VestyEvent::Param {
                sample_offset: 4,
                handle: gain,
                id_hash: 1,
                normalized: 0.5,
            })
            .unwrap();
        events
            .push(VestyEvent::NoteOff {
                sample_offset: 32,
                channel: 0,
                key: 60,
                velocity: 0.0,
                note_id: -1,
            })
            .unwrap();

        sort_events_by_sample_offset(&mut events);

        assert_eq!(
            events.as_slice(),
            &[
                VestyEvent::NoteOn {
                    sample_offset: 4,
                    channel: 0,
                    key: 60,
                    velocity: 1.0,
                    note_id: -1,
                },
                VestyEvent::Param {
                    sample_offset: 4,
                    handle: gain,
                    id_hash: 1,
                    normalized: 0.5,
                },
                VestyEvent::Param {
                    sample_offset: 16,
                    handle: gain,
                    id_hash: 1,
                    normalized: 0.25,
                },
                VestyEvent::NoteOff {
                    sample_offset: 32,
                    channel: 0,
                    key: 60,
                    velocity: 0.0,
                    note_id: -1,
                },
            ]
        );
    }

    #[test]
    fn sysex_copy_uses_fixed_buffer_and_reports_truncation() {
        let mut long = [0_u8; MAX_SYSEX_BYTES + 4];
        for (index, byte) in long.iter_mut().enumerate() {
            *byte = (index & 0x7f) as u8;
        }

        // SAFETY: Test data points to a stack array that remains alive for the duration of the copy.
        let (data, data_len, truncated) =
            unsafe { copy_sysex_data(long.as_ptr(), long.len() as u32) };
        assert_eq!(data_len as usize, MAX_SYSEX_BYTES);
        assert!(truncated);
        assert_eq!(&data[..8], &long[..8]);
        assert_eq!(
            &data[MAX_SYSEX_BYTES - 4..],
            &long[MAX_SYSEX_BYTES - 4..MAX_SYSEX_BYTES]
        );

        // SAFETY: Null pointer with a non-zero declared size is treated as an empty truncated event.
        let (data, data_len, truncated) = unsafe { copy_sysex_data(std::ptr::null(), 4) };
        assert_eq!(data_len, 0);
        assert!(truncated);
        assert!(data.iter().all(|byte| *byte == 0));
    }
}
