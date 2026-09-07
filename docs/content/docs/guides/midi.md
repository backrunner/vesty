---
title: MIDI and event timing
description: Build instruments that handle note identity, automation, and dense event blocks correctly.
order: 3
---

Vesty delivers VST3 events as typed `Event` values in `ProcessContext` and `ProcessContext64`. Start from the [MIDI synth example](https://github.com/backrunner/vesty/tree/main/examples/midi-synth), a monophonic instrument with tests for timing, expression, and SysEx.

## Declare your inputs

An instrument declares `PluginKind::Instrument` and an event input bus in its plugin bus layout. A plugin receives only the event types and buses it supports. VST3 represents mapped MIDI controllers through parameter changes: declare mappings with `with_midi_mapping` on your parameter, rather than expecting every host to forward raw MIDI CC bytes. See [Parameters](/docs/guides/parameters) and the example's program parameter.

Channel numbers are zero-based (`0..=15`), keys are `0..=127`, and note events carry a host `note_id`. For polyphonic voice tracking, prefer the note ID when the host supplies one; define a channel/key fallback for events without a usable ID. Match the channel as well as the key when releasing a voice.

## Render between events

Offsets are relative to the audio range in the **current process call**. Do not apply all events before rendering the entire buffer.

For example, with NoteOn at 8 and NoteOff at 24 in a 32-frame context:

| Range | Work |
| --- | --- |
| Samples 0–7 | Render the previous voice state |
| Before sample 8 | Apply NoteOn |
| Samples 8–23 | Render the active voice |
| Before sample 24 | Apply NoteOff |
| Samples 24–31 | Render the released voice state |

Use this loop structure in your kernel; `render` and `apply` below represent your own allocation-free DSP functions:

```text
cursor = 0
for event in context.events():
    render(cursor .. event.sample_offset())
    apply(event)
    cursor = event.sample_offset()
render(cursor .. context.audio().frames())
```

Consume events at identical offsets in their provided order. Parameter events precede note-list events at equal offsets; order within each source is stable. Keep oscillator and envelope state across calls, and update sample-rate-dependent state in `prepare()`.

Treat zero-velocity NoteOn as a release where MIDI semantics require it. The example implements this. The adapter filters invalid bus/channel/key/offset data and non-finite NoteOn velocity or pressure; a NoteOff with non-finite velocity is preserved with velocity zero so the release is not lost.

## Dense blocks and zero-frame calls

**512 is the batch cache size, not an event limit for the host block.** When a host sends more events, Vesty delivers successive batches using preallocated storage. A single host block can therefore produce multiple kernel calls.

When more than 512 events share one sample, earlier batches at that position can have zero audio frames. Always consume their events. Skip only audio generation; do not return before processing events. This rule also applies to native `process_f64`.

A host's zero-frame parameter flush is different: the adapter updates parameter state without invoking the audio kernel, including while processing is stopped.

The overflow path rescans host lists for each batch. Memory stays bounded, but CPU cost grows with event volume and batch count. Avoid generating unnecessary automation points and profile dense arrangements in your target DAWs. Removing the event cutoff does not remove the callback deadline.

## Payload limits

| Payload | Current contract |
| --- | --- |
| Events per cache batch | 512; remaining events continue in later batches |
| SysEx | Up to 256 bytes per event; inspect `data_len` and `truncated` |
| Note-expression text | Up to 64 UTF-16 code units; inspect `text_len` |

Do not interpret a truncated SysEx prefix as a complete message. The example rejects truncated messages before validating its experimental manufacturer ID and framing. Larger SysEx payloads require a separate API/storage design; batching does not enlarge an individual event's payload.

## Verify your instrument

Test the same event sequence with different block sizes and compare the rendered samples. Include multiple channels, repeated notes, zero-velocity NoteOn, expression, automation, and a final NoteOff. Also test zero-frame event contexts and bursts above 512 events at one timestamp.

The repository's adapter regression suite covers 4,800 mixed events, 2,050 events at one timestamp, f32 and f64 paths, allocation checks, and overlapping audio buffers. Run it with:

```bash
cargo test -p vesty-vst3 --features vst3-bindings
cargo test -p vesty-example-midi-synth
```

Local tests supplement [real DAW release evidence](/docs/tooling/release-evidence). Verify note release, automation playback, transport changes, and editor open/close behavior in your target hosts.
