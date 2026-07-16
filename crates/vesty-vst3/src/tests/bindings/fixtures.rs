use super::*;

pub(super) fn tuid(bytes: [u8; 16]) -> TUID {
    bytes.map(|byte| byte as c_char)
}

pub(super) fn test_param_id(id: &str) -> ParamID {
    vesty_params::stable_vst3_param_id(id)
}

pub(super) fn controller_param_id(controller: &ComPtr<IEditController>, index: int32) -> ParamID {
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

pub(super) fn string128_to_string(value: &String128) -> String {
    let len = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16(&value[..len]).expect("test UTF-16 string")
}

pub(super) fn wide_cstring(value: &str) -> Vec<TChar> {
    value
        .encode_utf16()
        .map(|unit| unit as TChar)
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "macos")]
pub(super) fn supported_platform_type_for_current_os() -> FIDString {
    kPlatformTypeNSView
}

#[cfg(target_os = "windows")]
pub(super) fn supported_platform_type_for_current_os() -> FIDString {
    kPlatformTypeHWND
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(super) fn supported_platform_type_for_current_os() -> FIDString {
    kPlatformTypeX11EmbedWindowID
}

#[derive(Default)]
pub(super) struct MemoryStream {
    bytes: RefCell<Vec<u8>>,
    cursor: Cell<usize>,
}

impl MemoryStream {
    pub(super) fn with_bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes: RefCell::new(bytes),
            cursor: Cell::new(0),
        }
    }

    pub(super) fn bytes(&self) -> Vec<u8> {
        self.bytes.borrow().clone()
    }
}

pub(super) fn raw_state_bytes(value: serde_json::Value) -> Vec<u8> {
    let mut bytes = b"VESTY_STATE_V1\n".to_vec();
    bytes.extend_from_slice(serde_json::to_string(&value).unwrap().as_bytes());
    bytes
}

pub(super) fn raw_program_data_bytes(
    list_id: u32,
    program_index: usize,
    data: serde_json::Value,
) -> Vec<u8> {
    raw_program_data_bytes_with_version(1, list_id, program_index, data)
}

pub(super) fn raw_program_data_bytes_with_version(
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

pub(super) fn program_data_json(bytes: &[u8]) -> serde_json::Value {
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
                ptr::copy_nonoverlapping(buffer as *const u8, bytes[cursor..].as_mut_ptr(), len);
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

pub(super) struct FakeParamValueQueue {
    id: ParamID,
    points: RefCell<Vec<(int32, ParamValue)>>,
}

impl FakeParamValueQueue {
    pub(super) fn new(id: ParamID, points: Vec<(int32, ParamValue)>) -> Self {
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
        let Some((sample, param_value)) = self.points.borrow().get(index as usize).copied() else {
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

pub(super) struct FakeParameterChanges {
    pub(super) queues: Vec<ComPtr<IParamValueQueue>>,
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

pub(super) struct FakeEventList {
    events: Vec<Event>,
    added: RefCell<Vec<Event>>,
}

impl FakeEventList {
    pub(super) fn new(events: Vec<Event>) -> Self {
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
pub(super) enum HandlerCall {
    Begin(ParamID),
    Perform(ParamID, ParamValue),
    End(ParamID),
    Restart(int32),
}

pub(super) struct FakeComponentHandler {
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
    pub(super) fn calls(&self) -> Vec<HandlerCall> {
        self.calls.borrow().clone()
    }

    #[cfg(feature = "wry-ui")]
    pub(super) fn rejecting_perform() -> Self {
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
pub(super) struct CapturedProcess {
    pub(super) events: Vec<CoreEvent>,
    pub(super) transport: Transport,
    pub(super) process_mode: ProcessMode,
    pub(super) param_value: Option<f64>,
    pub(super) no_alloc_active: bool,
}

pub(super) static CAPTURED_PROCESS: LazyLock<Mutex<CapturedProcess>> =
    LazyLock::new(|| Mutex::new(CapturedProcess::default()));

pub(super) struct CaptureKernel;

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
                    audio.set_output_sample(channel, frame as usize, segment.normalized as f32);
                }
            }
        }
        ProcessResult::Continue
    }
}

#[derive(Default)]
pub(super) struct CapturePlugin {
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

pub(super) struct ProgramAutomationParams {
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
pub(super) struct ProgramAutomationCapture {
    pub(super) events: Vec<CoreEvent>,
    pub(super) param_value: Option<f64>,
    pub(super) no_alloc_active: bool,
}

pub(super) static PROGRAM_AUTOMATION_CAPTURE: LazyLock<Mutex<ProgramAutomationCapture>> =
    LazyLock::new(|| Mutex::new(ProgramAutomationCapture::default()));
pub(super) static PROGRAM_AUTOMATION_APPLY_CALLS: TestAtomicUsize = TestAtomicUsize::new(0);
pub(super) static PROGRAM_AUTOMATION_LOAD_CALLS: TestAtomicUsize = TestAtomicUsize::new(0);

pub(super) struct ProgramAutomationKernel;

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
pub(super) struct ProgramAutomationPlugin {
    params: ProgramAutomationParams,
}

pub(super) static PROGRAM_AUTOMATION_PROGRAMS: &[vesty_core::Program] = &[
    vesty_core::Program::new("Init"),
    vesty_core::Program::new("Lead"),
    vesty_core::Program::new("Pad"),
];
pub(super) static PROGRAM_AUTOMATION_PROGRAM_LISTS: &[vesty_core::ProgramList] =
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

    fn apply_program(&self, _list_id: u32, _program_index: usize) -> Result<bool, StateError> {
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
pub(super) struct PrepareMatrixRecord {
    pub(super) init_sample_rate: f64,
    pub(super) init_max_block_size: usize,
    pub(super) prepare_sample_rate: f64,
    pub(super) prepare_max_block_size: usize,
    pub(super) process_frames: usize,
    pub(super) no_alloc_active: bool,
}

pub(super) static PREPARE_MATRIX_RECORDS: LazyLock<Mutex<Vec<PrepareMatrixRecord>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
pub(super) static PREPARE_MATRIX_KERNEL_CREATIONS: TestAtomicUsize = TestAtomicUsize::new(0);
pub(super) static PREPARE_MATRIX_RESETS: TestAtomicUsize = TestAtomicUsize::new(0);
pub(super) static PREPARE_MATRIX_SUSPENDS: TestAtomicUsize = TestAtomicUsize::new(0);
pub(super) static PREPARE_MATRIX_RESUMES: TestAtomicUsize = TestAtomicUsize::new(0);
pub(super) static PREPARE_MATRIX_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(super) struct PrepareMatrixKernel {
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
pub(super) struct PrepareMatrixPlugin {
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

pub(super) static NO_ALLOC_KERNEL_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static NO_ALLOC_GUARD_SEEN: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static NO_ALLOC_INPUT_CHANNELS: TestAtomicUsize = TestAtomicUsize::new(usize::MAX);
pub(super) static NO_ALLOC_PLUGIN_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(super) struct NoAllocKernel;

impl AudioKernel for NoAllocKernel {
    fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        NO_ALLOC_KERNEL_ENTERED.store(true, TestOrdering::Relaxed);
        NO_ALLOC_GUARD_SEEN.store(NoAllocGuard::is_active(), TestOrdering::Relaxed);
        let _frames = context.audio().frames();
        NO_ALLOC_INPUT_CHANNELS.store(context.audio().input_channels(), TestOrdering::Relaxed);
        let _gain = context.param_normalized(ParamHandle::from_index(0));
        ProcessResult::Continue
    }
}

#[derive(Default)]
pub(super) struct NoAllocPlugin {
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

pub(super) static NATIVE_F64_F32_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static NATIVE_F64_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static NATIVE_F64_GUARD_SEEN: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static NATIVE_F64_FRAMES: TestAtomicUsize = TestAtomicUsize::new(0);
pub(super) static NATIVE_F64_PLUGIN_TEST_LOCK: LazyLock<Mutex<()>> =
    LazyLock::new(|| Mutex::new(()));
pub(super) const NATIVE_F64_LEFT_BIAS: f64 = 1.0e-12;
pub(super) const NATIVE_F64_RIGHT_BIAS: f64 = -2.0e-12;

pub(super) struct NativeF64Kernel;

impl AudioKernel for NativeF64Kernel {
    const SUPPORTS_F64: bool = true;

    fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        NATIVE_F64_F32_ENTERED.store(true, TestOrdering::Relaxed);
        ProcessResult::Continue
    }

    fn process_f64(&mut self, context: &mut vesty_core::ProcessContext64<'_>) -> ProcessResult {
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
            context
                .audio_mut()
                .set_output_sample(0, frame, left * 0.5 + NATIVE_F64_LEFT_BIAS);
            context
                .audio_mut()
                .set_output_sample(1, frame, right * -0.25 + NATIVE_F64_RIGHT_BIAS);
        }

        let _ = context.emit_output_meter(88, 0);
        ProcessResult::Continue
    }
}

#[derive(Default)]
pub(super) struct NativeF64Plugin {
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

pub(super) static NATIVE_F64_SIDECHAIN_F32_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static NATIVE_F64_SIDECHAIN_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static NATIVE_F64_SIDECHAIN_GUARD_SEEN: TestAtomicBool = TestAtomicBool::new(false);

pub(super) struct NativeF64SidechainKernel;

impl AudioKernel for NativeF64SidechainKernel {
    const SUPPORTS_F64: bool = true;

    fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        NATIVE_F64_SIDECHAIN_F32_ENTERED.store(true, TestOrdering::Relaxed);
        ProcessResult::Continue
    }

    fn process_f64(&mut self, context: &mut vesty_core::ProcessContext64<'_>) -> ProcessResult {
        NATIVE_F64_SIDECHAIN_ENTERED.store(true, TestOrdering::Relaxed);
        NATIVE_F64_SIDECHAIN_GUARD_SEEN.store(NoAllocGuard::is_active(), TestOrdering::Relaxed);
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
pub(super) struct NativeF64SidechainPlugin {
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

pub(super) static PANIC_KERNEL_CALLS: TestAtomicUsize = TestAtomicUsize::new(0);
pub(super) static PANIC_PLUGIN_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(super) struct PanicKernel;

impl AudioKernel for PanicKernel {
    fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        PANIC_KERNEL_CALLS.fetch_add(1, TestOrdering::Relaxed);
        panic!("panic plugin");
    }
}

pub(super) struct SilenceKernel;

impl AudioKernel for SilenceKernel {
    fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        ProcessResult::Silence
    }
}

pub(super) struct MeterKernel;

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
pub(super) struct MeterPlugin {
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
pub(super) struct PanicPlugin {
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

pub(super) struct DefaultPanicPlugin;

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
pub(super) struct CallbackPanicPlugin {
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
pub(super) struct SilencePlugin {
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
pub(super) struct InstrumentPlugin {
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

pub(super) static MULTI_OUTPUT_INSTRUMENT_BUSES: &[AudioOutputBus] = &[
    AudioOutputBus::stereo("Main"),
    AudioOutputBus::stereo("Aux 1"),
];

pub(super) struct MultiOutputInstrumentKernel;

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

pub(super) static MULTI_OUTPUT_NATIVE_F64_F32_ENTERED: TestAtomicBool = TestAtomicBool::new(false);
pub(super) static MULTI_OUTPUT_NATIVE_F64_ENTERED: TestAtomicBool = TestAtomicBool::new(false);

pub(super) struct MultiOutputNativeF64InstrumentKernel;

impl AudioKernel for MultiOutputNativeF64InstrumentKernel {
    const SUPPORTS_F64: bool = true;

    fn process(&mut self, _context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        MULTI_OUTPUT_NATIVE_F64_F32_ENTERED.store(true, TestOrdering::Relaxed);
        ProcessResult::Continue
    }

    fn process_f64(&mut self, context: &mut vesty_core::ProcessContext64<'_>) -> ProcessResult {
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
pub(super) struct MultiOutputInstrumentPlugin {
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
pub(super) struct MultiOutputNativeF64InstrumentPlugin {
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

pub(super) struct SidechainKernel;

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
pub(super) struct SidechainPlugin {
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

pub(super) struct OptionalSidechainKernel;

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
pub(super) struct OptionalSidechainPlugin {
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
