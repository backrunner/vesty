use super::processor::*;
use super::*;

type SharedComponentHandler = Arc<Mutex<Option<ComPtr<IComponentHandler>>>>;
#[cfg(feature = "wry-ui")]
type ParamSetter = Arc<dyn Fn(&str, f64) -> HostChangeFlags + Send + Sync>;
#[cfg(feature = "wry-ui")]
type ParamValueGetter = Arc<dyn Fn() -> Vec<ParamValueSnapshot> + Send + Sync>;

#[cfg(feature = "wry-ui")]
#[derive(Clone)]
pub(crate) struct WryBridgeEndpoint {
    ready: BridgeReadyPayload,
    param_ids: Arc<Vec<String>>,
    param_host_ids: Arc<Vec<ParamID>>,
    handler: SharedComponentHandler,
    set_param: ParamSetter,
    get_param_values: ParamValueGetter,
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
            runtime.set_ready_param_values((endpoint.get_param_values)());
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
                    if result != kResultOk {
                        reject_param_gesture_responses(&sent, &gesture, result);
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
                    let result = handler.performEdit(id, normalized);
                    if result == kResultOk {
                        let changes = (self.set_param)(&gesture.id, normalized);
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
fn reject_param_gesture_responses(
    sent: &Rc<RefCell<Vec<BridgePacket>>>,
    gesture: &ParamGesture,
    host_result: tresult,
) {
    let packet_type = match gesture.phase {
        ParamGesturePhase::Begin => "param.begin.error",
        ParamGesturePhase::Perform => "param.perform.error",
        ParamGesturePhase::End => "param.end.error",
    };
    let error = BridgeErrorPayload::new(
        BridgeErrorCode::HostRejected,
        "host rejected parameter edit",
        true,
    )
    .with_details(serde_json::json!({ "result": host_result }));
    for packet in sent.borrow_mut().iter_mut() {
        if packet
            .reply_to
            .as_ref()
            .is_some_and(|reply_to| gesture.request_ids.contains(reply_to))
        {
            packet.kind = BridgeKind::Error;
            packet.packet_type = packet_type.to_string();
            packet.payload = None;
            packet.error = Some(error.clone());
        }
    }
}

#[cfg(feature = "wry-ui")]
fn current_param_values(
    params: &dyn ParamCollection,
    specs: &[vesty_params::ParamSpec],
) -> Vec<ParamValueSnapshot> {
    specs
        .iter()
        .map(|spec| ParamValueSnapshot {
            id: spec.id.clone(),
            normalized: params
                .get_normalized(&spec.id)
                .unwrap_or(spec.default_normalized)
                .clamp(0.0, 1.0),
        })
        .collect()
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

#[vesty_macros::vst3_panic_boundary]
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
        let value_plugin = self.plugin.clone();
        let value_specs = self.specs.clone();
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
                param_values: current_param_values(self.plugin.params(), &self.specs),
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
            get_param_values: Arc::new(move || {
                current_param_values(value_plugin.params(), &value_specs)
            }),
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

#[vesty_macros::vst3_panic_boundary]
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

#[vesty_macros::vst3_panic_boundary]
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

#[vesty_macros::vst3_panic_boundary]
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

#[vesty_macros::vst3_panic_boundary]
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

#[vesty_macros::vst3_panic_boundary]
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

#[vesty_macros::vst3_panic_boundary]
impl<P: Plugin + Default> IPluginBaseTrait for VestyController<P> {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

#[vesty_macros::vst3_panic_boundary]
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

#[vesty_macros::vst3_panic_boundary]
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
