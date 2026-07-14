pub use vesty_bridge as bridge;
pub use vesty_core::*;
pub use vesty_ipc as ipc;
pub use vesty_macros::Params;
pub use vesty_params as params;
pub use vesty_rt as rt;
pub use vesty_ui as ui;

#[cfg(feature = "webview-wry")]
pub use vesty_ui_wry as ui_wry;

#[cfg(feature = "vst3")]
pub use vesty_vst3 as vst3;

pub mod prelude {
    pub use crate::params::{
        BoolParam, ChoiceParam, FloatParam, ParamCollection, ParamError, ParamHandle, ParamSpec,
    };
    pub use crate::rt::{QueueId, RtLogEvent, RtLogLevel};
    pub use crate::{
        AudioBuffers, AudioBuffers64, AudioKernel, AudioOutputBus, Event, HostChangeFlags,
        HostProfile, HostQuirk, HostQuirkArea, HostQuirkSeverity, InstrumentKernel, KernelInit,
        MAX_AUDIO_OUTPUT_BUSES, MAX_AUDIO_OUTPUT_CHANNELS, MAX_METER_CHANNELS, MAX_SYSEX_BYTES,
        MeterFrame, MeterSink, NoteExpressionPhysicalUiMapping, NoteExpressionValueFlags,
        NoteExpressionValueType, ParamAutomationPoint, ParamAutomationSegment,
        ParamAutomationSegments, Params, Plugin, PluginInfo, PluginKind, PluginState,
        PrepareContext, ProcessContext, ProcessContext64, ProcessMode, ProcessResult, Program,
        ProgramAttribute, ProgramList, ProgramPitchName, RELEASE_SMOKE_CHECKS, StateError,
        Transport, UiDescriptor, export_vst3, find_host_profile, host_profiles, load_plugin_state,
        note_expression, physical_ui, save_plugin_state,
    };
}

#[macro_export]
macro_rules! export_vst3 {
    ($plugin:ty) => {
        #[doc(hidden)]
        pub fn vesty_plugin_info() -> $crate::PluginInfo {
            <$plugin as $crate::Plugin>::INFO
        }

        #[cfg(target_os = "windows")]
        #[unsafe(no_mangle)]
        pub extern "system" fn InitDll() -> bool {
            $crate::vst3::abi_guard(false, || true)
        }

        #[cfg(target_os = "windows")]
        #[unsafe(no_mangle)]
        pub extern "system" fn ExitDll() -> bool {
            $crate::vst3::abi_guard(false, || true)
        }

        #[cfg(target_os = "macos")]
        #[unsafe(no_mangle)]
        pub extern "system" fn bundleEntry(bundle_ref: *mut ::std::ffi::c_void) -> bool {
            $crate::vst3::abi_guard(false, || {
                $crate::vst3::set_macos_bundle_ref(bundle_ref);
                true
            })
        }

        #[cfg(target_os = "macos")]
        #[unsafe(no_mangle)]
        pub extern "system" fn bundleExit() -> bool {
            $crate::vst3::abi_guard(false, || true)
        }

        #[cfg(target_os = "macos")]
        #[unsafe(no_mangle)]
        pub extern "system" fn BundleEntry(bundle_ref: *mut ::std::ffi::c_void) -> bool {
            $crate::vst3::abi_guard(false, || bundleEntry(bundle_ref))
        }

        #[cfg(target_os = "macos")]
        #[unsafe(no_mangle)]
        pub extern "system" fn BundleExit() -> bool {
            $crate::vst3::abi_guard(false, || bundleExit())
        }

        #[cfg(target_os = "linux")]
        #[unsafe(no_mangle)]
        pub extern "system" fn ModuleEntry(_library_handle: *mut ::std::ffi::c_void) -> bool {
            $crate::vst3::abi_guard(false, || true)
        }

        #[cfg(target_os = "linux")]
        #[unsafe(no_mangle)]
        pub extern "system" fn ModuleExit() -> bool {
            $crate::vst3::abi_guard(false, || true)
        }

        #[unsafe(no_mangle)]
        pub extern "system" fn GetPluginFactory() -> *mut $crate::vst3::raw::IPluginFactory {
            $crate::vst3::abi_guard(::std::ptr::null_mut(), || {
                $crate::vst3::create_plugin_factory::<$plugin>()
            })
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::ParamCollection;

    #[derive(Params)]
    struct DerivedParams {
        gain: params::FloatParam,
        bypass: params::BoolParam,
        mode: params::ChoiceParam,
        #[param(skip)]
        label: String,
    }

    impl Default for DerivedParams {
        fn default() -> Self {
            Self {
                gain: params::FloatParam::new("gain", "Gain", 0.0, 2.0, 1.0),
                bypass: params::BoolParam::bypass("bypass", "Bypass", false),
                mode: params::ChoiceParam::new("mode", "Mode", ["Clean", "Drive", "Fuzz"], 1),
                label: "ignored".to_string(),
            }
        }
    }

    #[test]
    fn params_derive_implements_param_collection() {
        let params = DerivedParams::default();
        let specs = params.specs();
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].id, "gain");
        assert_eq!(specs[1].id, "bypass");
        assert_eq!(specs[2].id, "mode");

        params.set_normalized("gain", 0.25).unwrap();
        params.set_normalized("bypass", 1.0).unwrap();
        params.set_normalized("mode", 0.75).unwrap();
        assert_eq!(params.get_normalized("gain"), Some(0.25));
        assert_eq!(params.get_normalized("bypass"), Some(1.0));
        assert_eq!(params.get_normalized("mode"), Some(1.0));

        let gain = params.resolve("gain").unwrap();
        let bypass = params.resolve("bypass").unwrap();
        let mode = params.resolve("mode").unwrap();
        assert_eq!(gain.index(), 0);
        assert_eq!(bypass.index(), 1);
        assert_eq!(mode.index(), 2);
        params.set_normalized_by_handle(gain, 0.5).unwrap();
        assert_eq!(params.get_normalized_by_handle(gain), Some(0.5));
        assert_eq!(
            params.get_normalized_by_handle(params::ParamHandle::from_index(99)),
            None
        );
        assert!(matches!(
            params.set_normalized_by_handle(params::ParamHandle::from_index(99), 0.0),
            Err(params::ParamError::Unknown(id)) if id == "handle:99"
        ));
        assert!(matches!(
            params.set_normalized("missing", 0.0),
            Err(params::ParamError::Unknown(id)) if id == "missing"
        ));
        assert_eq!(params.label, "ignored");
    }

    #[derive(Params)]
    struct AttributeParams {
        #[param(id = "wet")]
        mix: params::FloatParam,
        #[param(bypass)]
        soft_bypass: params::BoolParam,
    }

    impl Default for AttributeParams {
        fn default() -> Self {
            Self {
                mix: params::FloatParam::new("internal_mix", "Mix", 0.0, 100.0, 50.0),
                soft_bypass: params::BoolParam::new("soft_bypass", "Soft Bypass", false),
            }
        }
    }

    #[test]
    fn params_derive_supports_id_and_bypass_attributes() {
        let params = AttributeParams::default();
        let specs = params.specs();

        assert_eq!(specs[0].id, "wet");
        assert_eq!(specs[0].name, "Mix");
        assert!(!specs[0].flags.bypass);
        assert_eq!(specs[1].id, "soft_bypass");
        assert!(specs[1].flags.bypass);

        assert!(params.resolve("internal_mix").is_none());
        let wet = params.resolve("wet").unwrap();
        assert_eq!(wet.index(), 0);
        params.set_normalized("wet", 0.25).unwrap();
        assert_eq!(params.get_normalized("wet"), Some(0.25));
        assert_eq!(params.get_normalized("internal_mix"), None);

        let soft_bypass = params.resolve("soft_bypass").unwrap();
        params.set_normalized_by_handle(soft_bypass, 1.0).unwrap();
        assert_eq!(params.get_normalized("soft_bypass"), Some(1.0));
    }

    #[test]
    fn prelude_exports_process_mode() {
        use crate::prelude::*;

        assert_eq!(ProcessMode::Offline, crate::ProcessMode::Offline);
    }
}
