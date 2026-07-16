use super::*;

#[test]
fn processor_process_does_not_allocate_inside_rt_guard_under_automation_and_midi() {
    let _lock = NO_ALLOC_PLUGIN_TEST_LOCK.lock().unwrap();
    // SAFETY: Test code is exercising fake VST3/COM objects and raw callback entrypoints with fixtures constructed in this module.
    unsafe {
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NativeF64SidechainPlugin>())
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<NoAllocPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<SilencePlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory = ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<PanicPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<DefaultPanicPlugin>())
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
        let factory =
            ComPtr::<IPluginFactory>::from_raw(create_plugin_factory::<CallbackPanicPlugin>())
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
        let processor = ComPtr::<IAudioProcessor>::from_raw(processor as *mut IAudioProcessor)
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
        let controller = ComPtr::<IEditController>::from_raw(controller as *mut IEditController)
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
        let registry = std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default());
        let processor = ComWrapper::new(
            crate::bindings_impl::VestyProcessor::<PanicPlugin>::with_telemetry_registry(
                registry.clone(),
            ),
        );
        let controller = ComWrapper::new(
            crate::bindings_impl::VestyController::<PanicPlugin>::with_telemetry_registry(registry),
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
