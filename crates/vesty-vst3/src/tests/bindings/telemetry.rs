use super::*;

#[test]
fn processor_processes_sample64_and_writes_host_outputs() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<MeterPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let registry = std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default());
        let processor = ComWrapper::new(
            crate::bindings_impl::VestyProcessor::<MeterPlugin>::with_telemetry_registry(
                registry.clone(),
            ),
        );
        let controller = ComWrapper::new(
            crate::bindings_impl::VestyController::<MeterPlugin>::with_telemetry_registry(registry),
        );
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
