use super::*;
use vesty_params::{FloatParam, ParamCollection, ParamError, ParamHandle, ParamSpec};

struct Params {
    gain: FloatParam,
}

impl ParamCollection for Params {
    fn specs(&self) -> Vec<ParamSpec> {
        vec![self.gain.spec()]
    }

    fn get_normalized(&self, id: &str) -> Option<f64> {
        (id == "gain").then(|| self.gain.normalized())
    }

    fn set_normalized(&self, id: &str, normalized: f64) -> Result<(), ParamError> {
        if id == "gain" {
            self.gain.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(id.to_string()))
        }
    }

    fn get_normalized_by_handle(&self, handle: ParamHandle) -> Option<f64> {
        (handle.index() == 0).then(|| self.gain.normalized())
    }

    fn set_normalized_by_handle(
        &self,
        handle: ParamHandle,
        normalized: f64,
    ) -> Result<(), ParamError> {
        if handle.index() == 0 {
            self.gain.set_normalized(normalized);
            Ok(())
        } else {
            Err(ParamError::Unknown(format!("handle:{}", handle.index())))
        }
    }
}

#[test]
fn copies_audio_with_gain() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
    };
    let input = [1.0, -0.5, 0.25];
    let mut output = [0.0; 3];
    let inputs: [&[f32]; 1] = [&input];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let audio = AudioBuffers::new(&inputs, &mut outputs);
    let mut context = ProcessContext::new(audio, &params, &[], Transport::default());
    let gain = context.param_handle("gain").unwrap();
    assert_eq!(gain.index(), 0);
    assert_eq!(context.param_normalized(gain), Some(1.0));
    context.audio_mut().copy_input_to_output(0, 0.5);
    assert_eq!(output, [0.5, -0.25, 0.125]);
}

#[test]
fn process_context_process_mode_defaults_and_overrides() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
    };
    let input = [0.0; 1];
    let mut output = [0.0; 1];
    let inputs: [&[f32]; 1] = [&input];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let audio = AudioBuffers::new(&inputs, &mut outputs);

    let context = ProcessContext::new(audio, &params, &[], Transport::default());
    assert_eq!(context.process_mode(), ProcessMode::Realtime);

    let input = [0.0; 1];
    let mut output = [0.0; 1];
    let inputs: [&[f32]; 1] = [&input];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let audio = AudioBuffers::new(&inputs, &mut outputs);
    let context = ProcessContext::new(audio, &params, &[], Transport::default())
        .with_process_mode(ProcessMode::Offline);

    assert_eq!(context.process_mode(), ProcessMode::Offline);
}

#[test]
fn event_sample_offset_covers_midi_and_param_events() {
    let gain = ParamHandle::from_index(0);
    let events = [
        Event::NoteOn {
            sample_offset: 1,
            channel: 0,
            key: 60,
            velocity: 1.0,
            note_id: -1,
        },
        Event::NoteOff {
            sample_offset: 2,
            channel: 0,
            key: 60,
            velocity: 0.0,
            note_id: -1,
        },
        Event::PolyPressure {
            sample_offset: 3,
            channel: 0,
            key: 60,
            pressure: 0.5,
            note_id: -1,
        },
        Event::MidiCc {
            sample_offset: 4,
            channel: 0,
            controller: 1,
            value: 0.25,
        },
        Event::PitchBend {
            sample_offset: 5,
            channel: 0,
            value: -0.5,
        },
        Event::ChannelPressure {
            sample_offset: 6,
            channel: 0,
            pressure: 0.75,
        },
        Event::SysEx {
            sample_offset: 7,
            data_len: 3,
            data: {
                let mut data = [0; MAX_SYSEX_BYTES];
                data[..3].copy_from_slice(&[0xF0, 0x7D, 0xF7]);
                data
            },
            truncated: false,
        },
        Event::NoteExpressionValue {
            sample_offset: 8,
            type_id: note_expression::BRIGHTNESS,
            note_id: 12,
            value: 0.625,
        },
        Event::NoteExpressionInt {
            sample_offset: 9,
            type_id: note_expression::CUSTOM_START,
            note_id: 12,
            value: 42,
        },
        Event::NoteExpressionText {
            sample_offset: 10,
            type_id: note_expression::TEXT,
            note_id: 12,
            text_len: 0,
            text: [0; MAX_NOTE_EXPRESSION_TEXT_UNITS],
        },
        Event::Param {
            sample_offset: 11,
            handle: gain,
            id_hash: 1,
            normalized: 0.5,
        },
    ];

    assert_eq!(
        events.map(|event| event.sample_offset()),
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    );
}

#[test]
fn ui_descriptor_builders_override_editor_geometry() {
    let descriptor = UiDescriptor::web_assets("ui")
        .with_dev_url("http://localhost:5173")
        .with_size(777, 333)
        .with_min_size(320, 200)
        .with_resizable(false);

    assert_eq!(descriptor.assets_dir, "ui");
    assert_eq!(descriptor.dev_url.as_deref(), Some("http://localhost:5173"));
    assert_eq!(descriptor.width, 777);
    assert_eq!(descriptor.height, 333);
    assert_eq!(descriptor.min_width, 320);
    assert_eq!(descriptor.min_height, 200);
    assert!(!descriptor.resizable);
}

#[test]
fn copies_audio_with_gain_range() {
    let input = [1.0, 2.0, 3.0, 4.0];
    let mut output = [0.0; 4];
    let inputs: [&[f32]; 1] = [&input];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let mut audio = AudioBuffers::new(&inputs, &mut outputs);

    audio.copy_input_to_output_range(0, 1, 3, 0.5);
    audio.copy_input_to_output_range(0, 3, 99, 2.0);

    assert_eq!(output, [0.0, 1.0, 1.5, 8.0]);
}

#[test]
fn copies_audio64_with_gain_range_set_and_clear() {
    let input_l = [1.0_f64, -2.0, 3.5, -4.5];
    let input_r = [0.25_f64, -1.5, 2.0, 8.0];
    let mut output_l = [9.0_f64; 4];
    let mut output_r = [7.0_f64; 4];

    {
        let inputs: [&[f64]; 2] = [&input_l, &input_r];
        let mut outputs: [&mut [f64]; 2] = [&mut output_l, &mut output_r];
        let mut audio = AudioBuffers64::new(&inputs, &mut outputs);

        assert_eq!(audio.input_channels(), 2);
        assert_eq!(audio.output_channels(), 2);
        assert_eq!(audio.frames(), 4);
        assert_eq!(audio.input_channel(0), Some(&input_l[..]));
        assert_eq!(audio.output_channel(1), Some(&[7.0_f64; 4][..]));

        audio.copy_input_to_output(0, 0.5);
        audio.copy_input_to_output_range(1, 1, 3, -2.0);
        audio.copy_input_to_output_range(1, 3, 99, 0.25);
        audio.set_output_sample(1, 0, 0.125);
        audio.set_output_sample(99, 0, 1.0);
        audio.set_output_sample(0, 99, 1.0);
    }

    assert_eq!(output_l, [0.5, -1.0, 1.75, -2.25]);
    assert_eq!(output_r, [0.125, 3.0, -4.0, 2.0]);

    {
        let inputs: [&[f64]; 0] = [];
        let mut outputs: [&mut [f64]; 2] = [&mut output_l, &mut output_r];
        let mut audio = AudioBuffers64::new(&inputs, &mut outputs);
        audio.clear_outputs();
    }

    assert_eq!(output_l, [0.0; 4]);
    assert_eq!(output_r, [0.0; 4]);
}

#[test]
fn filters_param_automation_by_handle() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
    };
    let gain = ParamHandle::from_index(0);
    let other = ParamHandle::from_index(1);
    let events = [
        Event::NoteOn {
            sample_offset: 0,
            channel: 0,
            key: 60,
            velocity: 1.0,
            note_id: -1,
        },
        Event::Param {
            sample_offset: 4,
            handle: gain,
            id_hash: 7,
            normalized: 0.25,
        },
        Event::Param {
            sample_offset: 8,
            handle: other,
            id_hash: 8,
            normalized: 0.75,
        },
        Event::Param {
            sample_offset: 12,
            handle: gain,
            id_hash: 7,
            normalized: 0.5,
        },
    ];
    let input = [0.0; 16];
    let mut output = [0.0; 16];
    let inputs: [&[f32]; 1] = [&input];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let audio = AudioBuffers::new(&inputs, &mut outputs);
    let context = ProcessContext::new(audio, &params, &events, Transport::default());

    let points = context.param_automation(gain).collect::<Vec<_>>();

    assert_eq!(
        points,
        vec![
            ParamAutomationPoint {
                sample_offset: 4,
                handle: gain,
                id_hash: 7,
                normalized: 0.25,
            },
            ParamAutomationPoint {
                sample_offset: 12,
                handle: gain,
                id_hash: 7,
                normalized: 0.5,
            },
        ]
    );
    assert_eq!(
        context.latest_param_automation(gain),
        Some(ParamAutomationPoint {
            sample_offset: 12,
            handle: gain,
            id_hash: 7,
            normalized: 0.5,
        })
    );
    assert_eq!(
        context.latest_param_automation(ParamHandle::from_index(99)),
        None
    );
}

#[test]
fn builds_param_automation_segments() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 0.25),
    };
    let gain = ParamHandle::from_index(0);
    let events = [
        Event::Param {
            sample_offset: 2,
            handle: gain,
            id_hash: 7,
            normalized: 0.5,
        },
        Event::Param {
            sample_offset: 5,
            handle: gain,
            id_hash: 7,
            normalized: 1.0,
        },
    ];
    let input = [0.0; 8];
    let mut output = [0.0; 8];
    let inputs: [&[f32]; 1] = [&input];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let audio = AudioBuffers::new(&inputs, &mut outputs);
    let context = ProcessContext::new(audio, &params, &events, Transport::default());

    let segments = context
        .param_automation_segments(gain, 0.0)
        .collect::<Vec<_>>();

    assert_eq!(
        segments,
        vec![
            ParamAutomationSegment {
                start_sample: 0,
                end_sample: 2,
                normalized: 0.25,
            },
            ParamAutomationSegment {
                start_sample: 2,
                end_sample: 5,
                normalized: 0.5,
            },
            ParamAutomationSegment {
                start_sample: 5,
                end_sample: 8,
                normalized: 1.0,
            },
        ]
    );
    assert_eq!(segments[0].len(), 2);
    assert!(!segments[0].is_empty());
}

#[test]
fn process_context64_exposes_params_events_transport_mode_and_meters() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 0.25),
    };
    let gain = ParamHandle::from_index(0);
    let other = ParamHandle::from_index(1);
    let events = [
        Event::Param {
            sample_offset: 2,
            handle: gain,
            id_hash: 7,
            normalized: 0.5,
        },
        Event::NoteOn {
            sample_offset: 3,
            channel: 0,
            key: 60,
            velocity: 0.75,
            note_id: 42,
        },
        Event::Param {
            sample_offset: 4,
            handle: other,
            id_hash: 8,
            normalized: 0.875,
        },
        Event::Param {
            sample_offset: 5,
            handle: gain,
            id_hash: 7,
            normalized: 0.75,
        },
    ];
    let input_l = [0.0_f64; 8];
    let input_r = [0.0_f64; 8];
    let mut output_l = [0.0_f64; 8];
    let mut output_r = [0.0_f64; 8];
    let transport = Transport {
        playing: true,
        tempo_bpm: Some(123.0),
        position_samples: Some(4096),
    };
    let mut sink = CollectMeterSink::default();

    {
        let inputs: [&[f64]; 2] = [&input_l, &input_r];
        let mut outputs: [&mut [f64]; 2] = [&mut output_l, &mut output_r];
        let audio = AudioBuffers64::new(&inputs, &mut outputs);
        let mut context = ProcessContext64::new(audio, &params, &events, transport)
            .with_process_mode(ProcessMode::Offline)
            .with_meter_sink(&mut sink);

        assert_eq!(context.process_mode(), ProcessMode::Offline);
        assert_eq!(context.transport(), transport);
        assert_eq!(context.events(), &events);
        assert_eq!(context.param_handle("gain"), Some(gain));
        assert_eq!(context.param_normalized(gain), Some(0.25));
        assert_eq!(
            context.latest_param_automation(gain),
            Some(ParamAutomationPoint {
                sample_offset: 5,
                handle: gain,
                id_hash: 7,
                normalized: 0.75,
            })
        );
        assert_eq!(
            context
                .param_automation_segments(gain, 0.0)
                .collect::<Vec<_>>(),
            vec![
                ParamAutomationSegment {
                    start_sample: 0,
                    end_sample: 2,
                    normalized: 0.25,
                },
                ParamAutomationSegment {
                    start_sample: 2,
                    end_sample: 5,
                    normalized: 0.5,
                },
                ParamAutomationSegment {
                    start_sample: 5,
                    end_sample: 8,
                    normalized: 0.75,
                },
            ]
        );

        {
            let (audio, context_events) = context.audio_mut_and_events();
            assert_eq!(context_events.len(), events.len());
            audio.set_output_sample(0, 0, -0.25);
            audio.set_output_sample(0, 1, 0.75);
            audio.set_output_sample(1, 0, 0.5);
            audio.set_output_sample(1, 1, -0.125);
        }
        assert!(context.emit_output_meter(99, 11));
    }

    assert_eq!(sink.frames.len(), 1);
    let frame = sink.frames[0];
    assert_eq!(frame.id_hash, 99);
    assert_eq!(frame.sample_offset, 11);
    assert_eq!(frame.channel_count(), 2);
    assert_eq!(frame.peaks[0], 0.75);
    assert_eq!(frame.peaks[1], 0.5);
    assert!((frame.rms[0] - 0.279_508_5).abs() < 0.000_001);
    assert!((frame.rms[1] - 0.182_217_5).abs() < 0.000_001);
}

#[test]
fn coalesces_param_automation_segments_at_same_offset() {
    let gain = ParamHandle::from_index(0);
    let events = [
        Event::Param {
            sample_offset: 0,
            handle: gain,
            id_hash: 7,
            normalized: 0.4,
        },
        Event::Param {
            sample_offset: 0,
            handle: gain,
            id_hash: 7,
            normalized: 0.6,
        },
        Event::Param {
            sample_offset: 3,
            handle: gain,
            id_hash: 7,
            normalized: 0.8,
        },
    ];

    let segments = ParamAutomationSegments::new(&events, gain, 0.2, 4).collect::<Vec<_>>();

    assert_eq!(
        segments,
        vec![
            ParamAutomationSegment {
                start_sample: 0,
                end_sample: 3,
                normalized: 0.6,
            },
            ParamAutomationSegment {
                start_sample: 3,
                end_sample: 4,
                normalized: 0.8,
            },
        ]
    );
}

#[test]
fn automation_segments_ignore_other_handles_and_clamp_to_block() {
    let gain = ParamHandle::from_index(0);
    let other = ParamHandle::from_index(1);
    let events = [
        Event::Param {
            sample_offset: 1,
            handle: other,
            id_hash: 8,
            normalized: 0.9,
        },
        Event::Param {
            sample_offset: 2,
            handle: gain,
            id_hash: 7,
            normalized: 0.5,
        },
        Event::Param {
            sample_offset: 99,
            handle: gain,
            id_hash: 7,
            normalized: 1.0,
        },
    ];

    let segments = ParamAutomationSegments::new(&events, gain, 0.25, 4).collect::<Vec<_>>();

    assert_eq!(
        segments,
        vec![
            ParamAutomationSegment {
                start_sample: 0,
                end_sample: 2,
                normalized: 0.25,
            },
            ParamAutomationSegment {
                start_sample: 2,
                end_sample: 4,
                normalized: 0.5,
            },
        ]
    );
}

#[test]
fn automation_segments_cover_block_for_deterministic_point_patterns() {
    let gain = ParamHandle::from_index(0);
    let other = ParamHandle::from_index(1);
    let initial = 0.125;
    let patterns: &[&[(u32, f64)]] = &[
        &[],
        &[(0, 0.25)],
        &[(1, 0.5)],
        &[(0, 0.25), (0, 0.75), (1, 0.5)],
        &[(2, 0.25), (2, 0.875), (5, 0.375), (99, 1.0)],
    ];

    for block_frames in [0, 1, 2, 4, 8, 16, 32] {
        for points in patterns {
            let mut events = Vec::new();
            for (offset, normalized) in *points {
                events.push(Event::Param {
                    sample_offset: *offset,
                    handle: other,
                    id_hash: 8,
                    normalized: 0.99,
                });
                events.push(Event::Param {
                    sample_offset: *offset,
                    handle: gain,
                    id_hash: 7,
                    normalized: *normalized,
                });
            }

            let segments = ParamAutomationSegments::new(&events, gain, initial, block_frames)
                .collect::<Vec<_>>();

            if block_frames == 0 {
                assert!(segments.is_empty());
                continue;
            }

            let mut expected_by_sample = Vec::new();
            for sample in 0..block_frames {
                let mut expected = initial;
                for (offset, normalized) in *points {
                    if *offset <= sample {
                        expected = *normalized;
                    }
                }
                expected_by_sample.push(expected);
            }

            let mut cursor = 0;
            for segment in &segments {
                assert!(!segment.is_empty(), "{block_frames:?} {points:?}");
                assert_eq!(segment.start_sample, cursor, "{block_frames:?} {points:?}");
                assert!(
                    segment.end_sample <= block_frames,
                    "{block_frames:?} {points:?}"
                );
                for sample in segment.start_sample..segment.end_sample {
                    assert_eq!(
                        segment.normalized, expected_by_sample[sample as usize],
                        "{block_frames:?} {points:?} sample={sample}"
                    );
                }
                cursor = segment.end_sample;
            }
            assert_eq!(cursor, block_frames, "{block_frames:?} {points:?}");
        }
    }
}

#[test]
fn automation_segments_cover_block_for_seeded_fuzz_patterns() {
    let gain = ParamHandle::from_index(0);
    let other = ParamHandle::from_index(1);
    let initial = 0.375;
    let mut seed = 0x5645_5354_4946_555a_u64;

    for block_frames in [1_u32, 2, 3, 7, 16, 31, 64, 128] {
        for case_index in 0..32 {
            let mut events = Vec::new();
            let point_count = 1 + (next_fuzz_u32(&mut seed) % 48);
            for point_index in 0..point_count {
                let sample_offset = next_fuzz_u32(&mut seed) % (block_frames + 8);
                let normalized = f64::from(next_fuzz_u32(&mut seed) % 1001) / 1000.0;
                let handle = if next_fuzz_u32(&mut seed).is_multiple_of(3) {
                    other
                } else {
                    gain
                };
                events.push(Event::Param {
                    sample_offset,
                    handle,
                    id_hash: if handle == gain { 7 } else { 8 },
                    normalized,
                });

                if point_index % 7 == 0 {
                    events.push(Event::NoteOn {
                        sample_offset,
                        channel: 0,
                        key: 60,
                        velocity: 0.5,
                        note_id: point_index as i32,
                    });
                }
            }
            events.sort_by_key(Event::sample_offset);

            let segments = ParamAutomationSegments::new(&events, gain, initial, block_frames)
                .collect::<Vec<_>>();

            let mut expected_by_sample = vec![initial; block_frames as usize];
            for event in &events {
                let Event::Param {
                    sample_offset,
                    handle,
                    normalized,
                    ..
                } = *event
                else {
                    continue;
                };
                if handle != gain || sample_offset >= block_frames {
                    continue;
                }
                for sample in sample_offset..block_frames {
                    expected_by_sample[sample as usize] = normalized;
                }
            }

            let mut cursor = 0;
            for segment in &segments {
                assert!(
                    !segment.is_empty(),
                    "block={block_frames} case={case_index} events={events:?}"
                );
                assert_eq!(
                    segment.start_sample, cursor,
                    "block={block_frames} case={case_index} events={events:?}"
                );
                assert!(
                    segment.end_sample <= block_frames,
                    "block={block_frames} case={case_index} events={events:?}"
                );
                for sample in segment.start_sample..segment.end_sample {
                    assert_eq!(
                        segment.normalized, expected_by_sample[sample as usize],
                        "block={block_frames} case={case_index} sample={sample} events={events:?}"
                    );
                }
                cursor = segment.end_sample;
            }
            assert_eq!(
                cursor, block_frames,
                "block={block_frames} case={case_index} events={events:?}"
            );
        }
    }
}

fn next_fuzz_u32(seed: &mut u64) -> u32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 32) as u32
}

#[derive(Default)]
struct CollectMeterSink {
    frames: Vec<MeterFrame>,
}

impl MeterSink for CollectMeterSink {
    fn push_meter(&mut self, frame: MeterFrame) -> bool {
        self.frames.push(frame);
        true
    }
}

#[test]
fn computes_and_emits_output_meter_frames() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
    };
    let mut left = [1.0, -0.5, 0.25, -0.25];
    let mut right = [0.0, 0.25, -0.75, 0.5];
    let inputs: [&[f32]; 0] = [];
    let mut outputs: [&mut [f32]; 2] = [&mut left, &mut right];
    let audio = AudioBuffers::new(&inputs, &mut outputs);
    let mut sink = CollectMeterSink::default();

    {
        let mut context = ProcessContext::new(audio, &params, &[], Transport::default())
            .with_meter_sink(&mut sink);
        assert!(context.emit_output_meter(42, 7));
    }

    assert_eq!(sink.frames.len(), 1);
    let frame = sink.frames[0];
    assert_eq!(frame.id_hash, 42);
    assert_eq!(frame.sample_offset, 7);
    assert_eq!(frame.channel_count(), 2);
    assert_eq!(frame.peaks[0], 1.0);
    assert_eq!(frame.peaks[1], 0.75);
    assert!((frame.rms[0] - 0.586_301_9).abs() < 0.000_001);
    assert!((frame.rms[1] - 0.467_707_2).abs() < 0.000_001);
}

#[test]
fn emitting_meter_without_sink_drops_frame() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
    };
    let mut output = [0.0; 3];
    let inputs: [&[f32]; 0] = [];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let audio = AudioBuffers::new(&inputs, &mut outputs);
    let mut context = ProcessContext::new(audio, &params, &[], Transport::default());
    assert!(!context.emit_output_meter(1, 0));
}

#[test]
fn process_context_exposes_optional_sidechain_buffers() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
    };
    let input = [0.25_f32, 0.5];
    let side_left = [1.0_f32, 0.5];
    let side_right = [0.25_f32, 0.125];
    let mut output = [0.0_f32; 2];
    let inputs: [&[f32]; 1] = [&input];
    let sidechain_inputs: [&[f32]; 2] = [&side_left, &side_right];
    let mut outputs: [&mut [f32]; 1] = [&mut output];
    let audio = AudioBuffers::new(&inputs, &mut outputs);
    let sidechain = SidechainBuffers::new(&sidechain_inputs);
    let context =
        ProcessContext::new(audio, &params, &[], Transport::default()).with_sidechain(sidechain);

    assert_eq!(context.audio().input_channels(), 1);
    assert_eq!(context.sidechain().input_channels(), 2);
    assert_eq!(context.sidechain().frames(), 2);
    assert_eq!(context.sidechain().input_channel(0), Some(&side_left[..]));
    assert_eq!(context.sidechain().input_channel(1), Some(&side_right[..]));
}

#[test]
fn process_context64_exposes_optional_sidechain_buffers() {
    let params = Params {
        gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 1.0),
    };
    let side_left = [1.0_f64, 0.5];
    let side_right = [0.25_f64, 0.125];
    let mut output = [0.0_f64; 2];
    let inputs: [&[f64]; 0] = [];
    let sidechain_inputs: [&[f64]; 2] = [&side_left, &side_right];
    let mut outputs: [&mut [f64]; 1] = [&mut output];
    let audio = AudioBuffers64::new(&inputs, &mut outputs);
    let sidechain = SidechainBuffers64::new(&sidechain_inputs);
    let context =
        ProcessContext64::new(audio, &params, &[], Transport::default()).with_sidechain(sidechain);

    assert_eq!(context.audio().input_channels(), 0);
    assert_eq!(context.sidechain().input_channels(), 2);
    assert_eq!(context.sidechain().frames(), 2);
    assert_eq!(context.sidechain().input_channel(0), Some(&side_left[..]));
    assert_eq!(context.sidechain().input_channel(1), Some(&side_right[..]));
}

#[test]
fn program_list_descriptors_are_static_and_default_empty() {
    static PROGRAMS: &[Program] = &[Program::new("Init"), Program::new("Bright")];
    static LISTS: &[ProgramList] = &[ProgramList::new(7, "Factory", PROGRAMS)];

    assert_eq!(LISTS[0].id, 7);
    assert_eq!(LISTS[0].name, "Factory");
    assert!(!LISTS[0].is_empty());
    assert_eq!(LISTS[0].programs[1].name, "Bright");

    let attribute = ProgramAttribute::new("category", "Lead");
    assert!(attribute.is_valid());
    assert!(!ProgramAttribute::new("", "Lead").is_valid());
    assert!(!ProgramAttribute::new("category\0bad", "Lead").is_valid());
    assert!(!ProgramAttribute::new("category", "Lead\0bad").is_valid());

    let pitch_name = ProgramPitchName::new(60, "C4 Lead");
    assert!(pitch_name.is_valid());
    assert!(!ProgramPitchName::new(-1, "C4 Lead").is_valid());
    assert!(!ProgramPitchName::new(128, "C4 Lead").is_valid());
    assert!(!ProgramPitchName::new(60, "").is_valid());
    assert!(!ProgramPitchName::new(60, "C4\0Lead").is_valid());

    struct EmptyProgramPlugin {
        params: Params,
    }

    struct EmptyProgramKernel;

    impl AudioKernel for EmptyProgramKernel {
        fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
            context.audio_mut().clear_outputs();
            ProcessResult::Silence
        }
    }

    impl Plugin for EmptyProgramPlugin {
        const INFO: PluginInfo = PluginInfo {
            name: "Empty Program",
            vendor: "Vesty",
            url: "",
            email: "",
            version: "0.1.0",
            class_id: *b"empty-program!!!",
            kind: PluginKind::AudioEffect,
        };

        type Params = Params;
        type Kernel = EmptyProgramKernel;

        fn params(&self) -> &Self::Params {
            &self.params
        }

        fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
            EmptyProgramKernel
        }
    }

    let plugin = EmptyProgramPlugin {
        params: Params {
            gain: FloatParam::new("gain", "Gain", 0.0, 1.0, 0.5),
        },
    };
    assert!(plugin.program_lists().is_empty());
    assert_eq!(plugin.apply_program(7, 0), Ok(false));
    assert!(!plugin.program_data_supported(7));
    assert_eq!(plugin.save_program_data(7, 0), Ok(None));
    assert_eq!(
        plugin.load_program_data(7, 0, serde_json::json!({ "gain": 0.5 })),
        Ok(false)
    );
    assert!(plugin.program_attributes(7, 0).is_empty());
    assert!(plugin.program_pitch_names(7, 0).is_empty());
}

#[test]
fn audio_output_bus_descriptor_validates_realtime_safe_shape() {
    assert_eq!(MAX_AUDIO_OUTPUT_BUSES, 4);
    assert_eq!(MAX_AUDIO_OUTPUT_CHANNELS, 8);
    assert!(AudioOutputBus::mono("Aux").is_valid());
    assert!(AudioOutputBus::stereo("Main").is_valid());
    assert!(!AudioOutputBus::new("", 2).is_valid());
    assert!(!AudioOutputBus::new("Bad\nName", 2).is_valid());
    assert!(!AudioOutputBus::new("Wide", 3).is_valid());
    assert_eq!(
        DEFAULT_AUDIO_OUTPUT_BUSES,
        [AudioOutputBus::stereo("Output")]
    );
}

#[test]
fn note_expression_value_type_descriptor_validates_static_metadata() {
    let brightness =
        NoteExpressionValueType::new(note_expression::BRIGHTNESS, "Brightness", "Bright")
            .with_range(0.0, 1.0, 0.5)
            .with_flags(NoteExpressionValueFlags::ABSOLUTE);
    assert!(brightness.is_valid());
    assert_eq!(brightness.units, "");
    assert_eq!(brightness.step_count, 0);

    let invalid = NoteExpressionValueType::new(note_expression::INVALID, "Bad\nExpression", "Bad")
        .with_units("bad")
        .with_range(0.0, 1.0, f64::NAN)
        .with_step_count(-1);
    assert!(!invalid.is_valid());

    let mapping =
        NoteExpressionPhysicalUiMapping::new(physical_ui::PRESSURE, note_expression::BRIGHTNESS);
    assert!(mapping.is_valid());

    let invalid_mapping =
        NoteExpressionPhysicalUiMapping::new(physical_ui::INVALID, note_expression::BRIGHTNESS);
    assert!(!invalid_mapping.is_valid());
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct CustomState {
    mode: String,
}

#[derive(Default)]
struct StatefulPlugin {
    mode: std::sync::Mutex<String>,
}

impl PluginState for StatefulPlugin {
    type State = CustomState;

    fn save_state(&self) -> Self::State {
        CustomState {
            mode: self.mode.lock().unwrap().clone(),
        }
    }

    fn load_state(&self, state: Self::State) -> Result<(), StateError> {
        *self.mode.lock().unwrap() = state.mode;
        Ok(())
    }
}

#[test]
fn plugin_state_helpers_roundtrip_json() {
    let plugin = StatefulPlugin::default();
    *plugin.mode.lock().unwrap() = "wide".to_string();

    let value = save_plugin_state(&plugin).unwrap();
    assert_eq!(value["mode"], "wide");

    let restored = StatefulPlugin::default();
    load_plugin_state(&restored, value).unwrap();
    assert_eq!(restored.mode.lock().unwrap().as_str(), "wide");
}

#[test]
fn host_profiles_cover_release_matrix() {
    let profiles = host_profiles();
    assert_eq!(profiles.len(), 5);
    for expected in [
        "REAPER",
        "Cubase/Nuendo",
        "Bitwig Studio",
        "Ableton Live",
        "Studio One",
    ] {
        let profile = profiles
            .iter()
            .find(|profile| profile.name == expected)
            .unwrap_or_else(|| panic!("missing host profile: {expected}"));
        assert_eq!(profile.required_smoke_checks, RELEASE_SMOKE_CHECKS);
        assert!(!profile.quirks.is_empty());
    }
}

#[test]
fn host_profile_lookup_accepts_aliases() {
    assert_eq!(
        find_host_profile("Cubase").map(|profile| profile.id),
        Some("cubase-nuendo")
    );
    assert_eq!(
        find_host_profile("bitwig studio").map(|profile| profile.id),
        Some("bitwig")
    );
    assert!(find_host_profile("unknown").is_none());
}
