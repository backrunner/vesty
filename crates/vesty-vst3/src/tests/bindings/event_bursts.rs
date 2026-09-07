use super::*;

static NOTES: TestAtomicUsize = TestAtomicUsize::new(0);
static PARAMS: TestAtomicUsize = TestAtomicUsize::new(0);
static ZERO_FRAMES: TestAtomicUsize = TestAtomicUsize::new(0);
static PANIC_AT_NOTE: TestAtomicUsize = TestAtomicUsize::new(usize::MAX);
static BURST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
struct BurstKernel<const NATIVE: bool> {
    notes: usize,
    params: usize,
    active: bool,
    last_offset: i64,
}

macro_rules! consume_burst {
    ($kernel:ident, $context:ident, $sample:ty) => {{
        let frames = $context.audio().frames();
        if frames == 0 {
            ZERO_FRAMES.fetch_add(1, TestOrdering::Relaxed);
        }
        let mut level = $context
            .param_normalized(ParamHandle::from_index(0))
            .unwrap();
        let position = $context.transport().position_samples.unwrap();
        let (audio, events) = $context.audio_mut_and_events();
        let mut cursor = 0;
        for event in events {
            assert_ne!(
                $kernel.notes,
                PANIC_AT_NOTE.load(TestOrdering::Relaxed),
                "intentional late-batch panic"
            );
            let offset = event.sample_offset() as usize;
            assert!(offset < frames || (frames == 0 && offset == 0));
            let absolute = position + offset as i64;
            assert!(absolute >= $kernel.last_offset);
            $kernel.last_offset = absolute;
            for frame in cursor..offset {
                for channel in 0..audio.output_channels() {
                    audio.set_output_sample(
                        channel,
                        frame,
                        if $kernel.active {
                            level as $sample
                        } else {
                            0.0
                        },
                    );
                }
            }
            cursor = offset;
            match event {
                CoreEvent::NoteOn { note_id, .. } => {
                    assert_eq!(*note_id, $kernel.notes as i32);
                    $kernel.notes += 1;
                    $kernel.active = true;
                }
                CoreEvent::NoteOff { note_id, .. } => {
                    assert_eq!(*note_id, $kernel.notes as i32);
                    $kernel.notes += 1;
                    $kernel.active = false;
                }
                CoreEvent::Param { normalized, .. } => {
                    assert_eq!(*normalized, ($kernel.params % 4) as f64 / 4.0);
                    $kernel.params += 1;
                    level = *normalized;
                }
                _ => {}
            }
        }
        for frame in cursor..frames {
            for channel in 0..audio.output_channels() {
                audio.set_output_sample(
                    channel,
                    frame,
                    if $kernel.active {
                        level as $sample
                    } else {
                        0.0
                    },
                );
            }
        }
        NOTES.store($kernel.notes, TestOrdering::Relaxed);
        PARAMS.store($kernel.params, TestOrdering::Relaxed);
        ProcessResult::Continue
    }};
}

impl<const NATIVE: bool> AudioKernel for BurstKernel<NATIVE> {
    const SUPPORTS_F64: bool = NATIVE;
    fn process(&mut self, context: &mut vesty_core::ProcessContext<'_>) -> ProcessResult {
        consume_burst!(self, context, f32)
    }
    fn process_f64(&mut self, context: &mut vesty_core::ProcessContext64<'_>) -> ProcessResult {
        consume_burst!(self, context, f64)
    }
}

#[derive(Default)]
struct BurstPlugin<const NATIVE: bool> {
    params: TestParams,
}

impl<const NATIVE: bool> Plugin for BurstPlugin<NATIVE> {
    const INFO: PluginInfo = PluginInfo {
        name: "Event burst",
        vendor: "Vesty",
        url: "",
        email: "",
        version: "0.1.0",
        class_id: *b"event-burst-test",
        kind: PluginKind::Instrument,
    };
    type Params = TestParams;
    type Kernel = BurstKernel<NATIVE>;
    fn params(&self) -> &Self::Params {
        &self.params
    }
    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        BurstKernel::default()
    }
}

fn burst_events(frames: usize, per_frame: usize) -> Vec<Event> {
    let total = frames * per_frame;
    let mut events = Vec::new();
    // Reverse the sample groups to exercise host input that is not pre-sorted.
    for frame in (0..frames).rev() {
        for index in 0..per_frame {
            let id = frame * per_frame + index;
            let off = id + 1 == total;
            events.push(Event {
                busIndex: 0,
                sampleOffset: frame as i32,
                ppqPosition: 0.0,
                flags: 0,
                r#type: if off {
                    Event_::EventTypes_::kNoteOffEvent
                } else {
                    Event_::EventTypes_::kNoteOnEvent
                } as u16,
                __field0: if off {
                    Event__type0 {
                        noteOff: NoteOffEvent {
                            channel: 0,
                            pitch: 60,
                            velocity: 0.0,
                            noteId: id as i32,
                            tuning: 0.0,
                        },
                    }
                } else {
                    Event__type0 {
                        noteOn: NoteOnEvent {
                            channel: 0,
                            pitch: 60,
                            velocity: 1.0,
                            noteId: id as i32,
                            tuning: 0.0,
                            length: 0,
                        },
                    }
                },
            });
        }
    }
    events
}

macro_rules! check_burst {
    ($sample:ty, $buffers:ident, $size:ident, $native:literal) => {{
        check_burst!($sample, $buffers, $size, $native, false);
    }};
    ($sample:ty, $buffers:ident, $size:ident, $native:literal, $panic:literal) => {{
        for (frames, per_frame) in [(8, 128), (4, 600), (1, 1025)] {
            NOTES.store(0, TestOrdering::Relaxed);
            PARAMS.store(0, TestOrdering::Relaxed);
            ZERO_FRAMES.store(0, TestOrdering::Relaxed);
            PANIC_AT_NOTE.store(if $panic { 700 } else { usize::MAX }, TestOrdering::Relaxed);
            let wrapper = ComWrapper::new(crate::bindings_impl::VestyProcessor::<
                BurstPlugin<$native>,
            >::with_telemetry_registry(
                std::sync::Arc::new(crate::bindings_impl::Vst3TelemetryRegistry::default()),
            ));
            let processor = wrapper.to_com_ptr::<IAudioProcessor>().unwrap();
            let mut setup = ProcessSetup {
                processMode: ProcessModes_::kRealtime as int32,
                symbolicSampleSize: SymbolicSampleSizes_::$size as int32,
                maxSamplesPerBlock: frames as i32,
                sampleRate: 48_000.0,
            };
            assert_eq!(processor.setupProcessing(&mut setup), kResultOk);
            let points = (0..frames * per_frame)
                .map(|index| ((index / per_frame) as i32, (index % 4) as f64 / 4.0))
                .collect();
            let queue = ComWrapper::new(FakeParamValueQueue::new(test_param_id("gain"), points));
            let changes = ComWrapper::new(FakeParameterChanges {
                queues: vec![queue.to_com_ptr::<IParamValueQueue>().unwrap()],
            });
            let changes_ptr = changes.to_com_ptr::<IParameterChanges>().unwrap();
            let events = ComWrapper::new(FakeEventList::new(burst_events(frames, per_frame)));
            let events_ptr = events.to_com_ptr::<IEventList>().unwrap();
            let mut left = vec![-99.0 as $sample; frames];
            let mut right = vec![-99.0 as $sample; frames];
            let mut channels = [left.as_mut_ptr(), right.as_mut_ptr()];
            let mut outputs = AudioBusBuffers {
                numChannels: 2,
                silenceFlags: 0,
                __field0: AudioBusBuffers__type0 {
                    $buffers: channels.as_mut_ptr(),
                },
            };
            let mut context =
                MaybeUninit::<vst3::Steinberg::Vst::ProcessContext>::zeroed().assume_init();
            context.projectTimeSamples = 2048;
            let mut data = ProcessData {
                processMode: ProcessModes_::kRealtime as int32,
                symbolicSampleSize: SymbolicSampleSizes_::$size as int32,
                numSamples: frames as i32,
                numInputs: 0,
                numOutputs: 1,
                inputs: ptr::null_mut(),
                outputs: &mut outputs,
                inputParameterChanges: changes_ptr.as_ptr(),
                outputParameterChanges: ptr::null_mut(),
                inputEvents: events_ptr.as_ptr(),
                outputEvents: ptr::null_mut(),
                processContext: &mut context,
            };
            reset_rt_allocation_count();
            assert_eq!(processor.process(&mut data), kResultOk);
            if $panic {
                assert!(left.iter().all(|value| *value == 0.0));
                assert!(right.iter().all(|value| *value == 0.0));
                assert_eq!(outputs.silenceFlags, 3);
                PANIC_AT_NOTE.store(usize::MAX, TestOrdering::Relaxed);
                continue;
            }
            assert_eq!(rt_allocation_count(), 0);
            assert_eq!(NOTES.load(TestOrdering::Relaxed), frames * per_frame);
            assert_eq!(PARAMS.load(TestOrdering::Relaxed), frames * per_frame);
            assert_eq!(left, right);
            assert_eq!(left[frames - 1], 0.0);
            for (frame, value) in left[..frames - 1].iter().enumerate() {
                assert_eq!(*value, (((frame + 1) * per_frame - 1) % 4) as $sample / 4.0);
            }
            if per_frame > 512 {
                assert!(ZERO_FRAMES.load(TestOrdering::Relaxed) > 0);
            }
            // The final NoteOff must also remain effective in the following host block.
            data.inputEvents = ptr::null_mut();
            data.inputParameterChanges = ptr::null_mut();
            assert_eq!(processor.process(&mut data), kResultOk);
            assert!(left.iter().all(|value| *value == 0.0));
        }
    }};
}

#[test]
fn dense_events_do_not_drop_notes_or_automation_in_any_sample_path() {
    let _lock = BURST_LOCK.lock().unwrap();
    // SAFETY: All COM fixtures, channel buffers, and process context remain live through callbacks.
    unsafe {
        check_burst!(f32, channelBuffers32, kSample32, true);
        check_burst!(f64, channelBuffers64, kSample64, true);
        check_burst!(f64, channelBuffers64, kSample64, false);
    }
}

#[test]
fn a_late_batch_panic_silences_the_entire_host_block() {
    let _lock = BURST_LOCK.lock().unwrap();
    // SAFETY: Locally owned COM fixtures and buffers remain valid while the kernel panic is caught.
    unsafe {
        check_burst!(f32, channelBuffers32, kSample32, true, true);
        check_burst!(f64, channelBuffers64, kSample64, true, true);
        check_burst!(f64, channelBuffers64, kSample64, false, true);
    }
}
