use super::*;

struct AliasingKernel;

macro_rules! process_aliasing {
    ($context:ident) => {{
        // Clearing outputs must not alter either input bus, even with in-place host buffers.
        $context.audio_mut().clear_outputs();
        for frame in 0..$context.audio().frames() {
            let value = $context.audio().input_channel(0).unwrap()[frame]
                + $context.sidechain().input_channel(0).unwrap()[frame];
            $context.audio_mut().set_output_sample(0, frame, value);
            $context
                .audio_mut()
                .set_output_sample(1, frame, value + 1.0);
            assert_eq!($context.audio().output_channel(0).unwrap()[frame], value);
        }
        ProcessResult::Continue
    }};
}

impl AudioKernel for AliasingKernel {
    const SUPPORTS_F64: bool = true;

    fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        process_aliasing!(context)
    }

    fn process_f64(&mut self, context: &mut vesty_core::ProcessContext64<'_>) -> ProcessResult {
        process_aliasing!(context)
    }
}

#[derive(Default)]
struct AliasingPlugin {
    params: TestParams,
}

impl Plugin for AliasingPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Aliasing",
        vendor: "Vesty",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: *b"aliasing-test!!!",
        kind: PluginKind::AudioEffect,
    };

    type Params = TestParams;
    type Kernel = AliasingKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        AliasingKernel
    }

    fn sidechain_inputs(&self) -> u32 {
        1
    }
}

macro_rules! check_aliasing {
    ($sample:ty, $buffers:ident, $size:ident) => {{
        let wrapper = ComWrapper::new(
            crate::bindings_impl::VestyProcessor::<AliasingPlugin>::with_telemetry_registry(
                std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
            ),
        );
        let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
        let mut setup = ProcessSetup {
            processMode: ProcessModes_::kRealtime as int32,
            symbolicSampleSize: SymbolicSampleSizes_::$size as int32,
            maxSamplesPerBlock: 4,
            sampleRate: 48_000.0,
        };
        assert_eq!(processor.setupProcessing(&mut setup), kResultOk);
        let burst = ComWrapper::new(FakeEventList::new(
            (0..1025)
                .map(|index| Event {
                    busIndex: 0,
                    sampleOffset: index % 4,
                    ppqPosition: 0.0,
                    flags: 0,
                    r#type: Event_::EventTypes_::kNoteOnEvent as u16,
                    __field0: Event__type0 {
                        noteOn: NoteOnEvent {
                            channel: 0,
                            pitch: 60,
                            velocity: 1.0,
                            noteId: index,
                            tuning: 0.0,
                            length: 0,
                        },
                    },
                })
                .collect(),
        ));
        let burst_ptr = burst.to_com_ptr::<IEventList>().unwrap();

        for offset in [0, 1] {
            // Exercise main-input, sidechain, and output-only overlap independently.
            for aliased_bus in 0..3 {
                for duplicate_outputs in [false, true] {
                    for dense in [false, true] {
                        let mut main = [1.0 as $sample; 4];
                        let mut sidechain = [2.0 as $sample; 5];
                        let mut shared = [3.0 as $sample; 5];
                        let mut right = [0.0 as $sample; 4];
                        let mut main_channels = [if aliased_bus == 0 {
                            shared.as_mut_ptr()
                        } else {
                            main.as_mut_ptr()
                        }];
                        let mut sidechain_channels = [if aliased_bus == 1 {
                            shared.as_mut_ptr()
                        } else {
                            sidechain.as_mut_ptr()
                        }];
                        let output = shared.as_mut_ptr().add(offset);
                        let mut output_channels = [
                            output,
                            if duplicate_outputs {
                                output
                            } else {
                                right.as_mut_ptr()
                            },
                        ];
                        let mut inputs = [
                            AudioBusBuffers {
                                numChannels: 1,
                                silenceFlags: 0,
                                __field0: AudioBusBuffers__type0 {
                                    $buffers: main_channels.as_mut_ptr(),
                                },
                            },
                            AudioBusBuffers {
                                numChannels: 1,
                                silenceFlags: 0,
                                __field0: AudioBusBuffers__type0 {
                                    $buffers: sidechain_channels.as_mut_ptr(),
                                },
                            },
                        ];
                        let mut outputs = AudioBusBuffers {
                            numChannels: 2,
                            silenceFlags: 0,
                            __field0: AudioBusBuffers__type0 {
                                $buffers: output_channels.as_mut_ptr(),
                            },
                        };
                        let mut data = ProcessData {
                            processMode: ProcessModes_::kRealtime as int32,
                            symbolicSampleSize: SymbolicSampleSizes_::$size as int32,
                            numSamples: 4,
                            numInputs: 2,
                            numOutputs: 1,
                            inputs: inputs.as_mut_ptr(),
                            outputs: &mut outputs,
                            inputParameterChanges: ptr::null_mut(),
                            outputParameterChanges: ptr::null_mut(),
                            inputEvents: if dense {
                                burst_ptr.as_ptr()
                            } else {
                                ptr::null_mut()
                            },
                            outputEvents: ptr::null_mut(),
                            processContext: ptr::null_mut(),
                        };

                        reset_rt_allocation_count();
                        assert_eq!(processor.process(&mut data), kResultOk);
                        assert_eq!(rt_allocation_count(), 0);
                        let expected = match aliased_bus {
                            0 => 5.0,
                            1 => 4.0,
                            _ => 3.0,
                        } as $sample;
                        if duplicate_outputs {
                            assert_eq!(&shared[offset..offset + 4], &[expected + 1.0; 4]);
                        } else {
                            assert_eq!(&shared[offset..offset + 4], &[expected; 4]);
                            assert_eq!(right, [expected + 1.0; 4]);
                        }
                        assert_eq!(outputs.silenceFlags, 0);

                        // A host block exceeding prepared scratch capacity must fail silently,
                        // without allocating or entering the kernel with aliased views.
                        if offset == 0 && aliased_bus == 0 && duplicate_outputs {
                            shared.fill(3.0);
                            data.numSamples = 5;
                            reset_rt_allocation_count();
                            assert_eq!(processor.process(&mut data), kResultOk);
                            assert_eq!(rt_allocation_count(), 0);
                            assert_eq!(shared, [0.0; 5]);
                            assert_eq!(outputs.silenceFlags, 3);
                        }
                    }
                }
            }
        }
    }};
}

#[test]
fn native_processing_is_safe_with_overlapping_host_buffers() {
    // SAFETY: All host channel pointers refer to live arrays large enough for the process block.
    // The adapter must support overlapping channels without exposing aliased Rust references.
    unsafe {
        check_aliasing!(f32, channelBuffers32, kSample32);
        check_aliasing!(f64, channelBuffers64, kSample64);
    }
}
