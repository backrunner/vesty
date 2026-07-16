use super::*;

struct VestyFactory<P: Plugin + Default> {
    telemetry_registry: Arc<Vst3TelemetryRegistry>,
    _marker: PhantomData<P>,
}

impl<P: Plugin + Default> Class for VestyFactory<P> {
    type Interfaces = (IPluginFactory,);
}

#[vesty_macros::vst3_panic_boundary]
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
