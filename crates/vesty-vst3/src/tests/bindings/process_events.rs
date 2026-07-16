use super::*;

#[test]
fn processor_translates_automation_midi_and_transport() {
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        *CAPTURED_PROCESS.lock().unwrap() = CapturedProcess::default();

        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<CapturePlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
            .expect("processor");

        let mut setup = ProcessSetup {
            processMode: ProcessModes_::kRealtime as int32,
            symbolicSampleSize: SymbolicSampleSizes_::kSample32 as int32,
            maxSamplesPerBlock: 8,
            sampleRate: 48_000.0,
        };
        assert_eq!(processor.setupProcessing(&mut setup), kResultOk);

        let gain_id = test_param_id("gain");
        let queue = ComWrapper::new(FakeParamValueQueue::new(gain_id, vec![(2, 0.2), (6, 0.8)]));
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

        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<ProgramAutomationPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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

    let wrapper = ComWrapper::new(
        crate::bindings_impl::VestyProcessor::<PrepareMatrixPlugin>::with_telemetry_registry(
            std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
        ),
    );
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

        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<PrepareMatrixPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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

        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<PrepareMatrixPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<OptionalSidechainPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<OptionalSidechainPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<OptionalSidechainPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<OptionalSidechainPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
