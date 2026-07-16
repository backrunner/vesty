use super::*;

#[test]
fn processor_negotiates_mvp_effect_bus_arrangements() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let processor_wrapper = ComWrapper::new(
            crate::bindings_impl::VestyProcessor::<TestPlugin>::with_telemetry_registry(
                std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
            ),
        );
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
        let processor = crate::bindings_impl::VestyProcessor::<TestPlugin>::with_telemetry_registry(
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
        let processor = crate::bindings_impl::VestyProcessor::<TestPlugin>::with_telemetry_registry(
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
                &processor, 99,
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
        let effect = crate::bindings_impl::VestyProcessor::<TestPlugin>::with_telemetry_registry(
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
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
        let controller = ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
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

        let mut input: Vec<u16> = "Drive".encode_utf16().chain(std::iter::once(0)).collect();
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
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
        let controller = ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
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
            controller.getParamValueByString(mode_id, bounded_input.as_mut_ptr(), &mut normalized,),
            kResultOk
        );
        assert_eq!(normalized, 1.0);
    }
}

#[test]
fn processor_reports_latency_and_tail_samples() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
            .expect("processor");
        assert_eq!(processor.getLatencySamples(), 128);
        assert_eq!(processor.getTailSamples(), 4096);
    }
}

#[test]
fn processor_and_controller_support_connection_points() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
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
        let processor = ComPtr::<IConnectionPoint>::from_raw(processor as *mut IConnectionPoint)
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
        let controller = ComPtr::<IConnectionPoint>::from_raw(controller as *mut IConnectionPoint)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
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
        let controller = ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
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
        let component =
            ComPtr::<IComponent>::from_raw(component as *mut IComponent).expect("component");
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
        let restored = ComPtr::<IEditController>::from_raw(restored as *mut IEditController)
            .expect("restored controller");
        let restored_input = ComWrapper::new(MemoryStream::with_bytes(component_state_bytes));
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
        let restored_controller_state =
            ComPtr::<IEditController>::from_raw(restored_controller_state as *mut IEditController)
                .expect("controller state restore");
        let controller_input = ComWrapper::new(MemoryStream::with_bytes(controller_state.bytes()));
        let controller_input_ptr = controller_input.to_com_ptr::<IBStream>().unwrap();
        assert_eq!(
            restored_controller_state.setState(controller_input_ptr.as_ptr()),
            kResultOk
        );
        let restored_controller_gain_id = controller_param_id(&restored_controller_state, 0);
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
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
        let component =
            ComPtr::<IComponent>::from_raw(component as *mut IComponent).expect("component");
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<TestPlugin>())
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
        let controller = ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
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
