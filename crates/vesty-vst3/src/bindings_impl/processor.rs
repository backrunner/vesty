use super::*;

pub(crate) struct VestyProcessor<P: Plugin + Default> {
    plugin: P,
    kernel: UnsafeCell<Option<P::Kernel>>,
    events: UnsafeCell<FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>>,
    final_param_values: UnsafeCell<Vec<Option<f64>>>,
    meter_producer: UnsafeCell<RtMeterProducer>,
    log_producer: UnsafeCell<RtLogProducer>,
    telemetry_id: u64,
    telemetry_registry: Arc<Vst3TelemetryRegistry>,
    vst3_param_ids: Vst3ParamIds,
    sample_rate_bits: AtomicU64,
    max_block_size: AtomicUsize,
    input_arrangement: AtomicU64,
    sidechain_arrangement: AtomicU64,
    output_arrangements: [AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    input_bus_active: AtomicBool,
    sidechain_bus_active: AtomicBool,
    output_bus_active: [AtomicBool; MAX_AUDIO_OUTPUT_BUSES],
    event_input_bus_active: AtomicBool,
    io_mode: AtomicI32,
    processing_active: AtomicBool,
    sample64_scratch: UnsafeCell<Sample64Scratch>,
    connection: SharedConnectionPoint,
    fault: Arc<FaultState>,
}

// SAFETY: VST3 hosts call process lifecycle methods according to the component contract. The
// kernel is stored in UnsafeCell so process can mutate DSP state through &self without taking a
// lock. Concurrent process calls for the same component instance are not supported by VST3 hosts.
unsafe impl<P: Plugin + Default> Sync for VestyProcessor<P> {}

pub(super) fn default_input_arrangement(kind: vesty_core::PluginKind) -> SpeakerArrangement {
    if kind == vesty_core::PluginKind::Instrument {
        0
    } else {
        SpeakerArr::kStereo
    }
}

pub(super) fn supports_sidechain<P: Plugin>(plugin: &P) -> bool {
    P::INFO.kind != vesty_core::PluginKind::Instrument && plugin.sidechain_inputs() > 0
}

pub(super) fn default_sidechain_arrangement<P: Plugin>(plugin: &P) -> SpeakerArrangement {
    if supports_sidechain(plugin) {
        SpeakerArr::kStereo
    } else {
        0
    }
}

pub(super) fn declared_input_bus_count<P: Plugin>(plugin: &P) -> usize {
    if P::INFO.kind == vesty_core::PluginKind::Instrument {
        0
    } else if supports_sidechain(plugin) {
        2
    } else {
        1
    }
}

pub(super) fn output_bus_count<P: Plugin>(plugin: &P) -> usize {
    let count = plugin
        .output_buses()
        .iter()
        .filter(|bus| bus.is_valid())
        .take(MAX_AUDIO_OUTPUT_BUSES)
        .count();
    count.max(1)
}

pub(super) fn output_bus_at<P: Plugin>(plugin: &P, index: usize) -> Option<AudioOutputBus> {
    plugin
        .output_buses()
        .iter()
        .copied()
        .filter(AudioOutputBus::is_valid)
        .take(MAX_AUDIO_OUTPUT_BUSES)
        .nth(index)
        .or_else(|| (index == 0).then_some(vesty_core::DEFAULT_AUDIO_OUTPUT_BUSES[0]))
}

pub(super) fn output_bus_arrangement(bus: AudioOutputBus) -> SpeakerArrangement {
    match bus.channels {
        1 => SpeakerArr::kMono,
        2 => SpeakerArr::kStereo,
        _ => SpeakerArr::kStereo,
    }
}

pub(super) fn is_supported_output_bus_arrangement(
    bus: AudioOutputBus,
    arrangement: SpeakerArrangement,
) -> bool {
    matches!(
        (bus.channels, arrangement),
        (1, SpeakerArr::kMono) | (2, SpeakerArr::kStereo)
    )
}

pub(super) fn arrangement_channel_count(arrangement: SpeakerArrangement) -> Option<i32> {
    match arrangement {
        SpeakerArr::kMono => Some(1),
        SpeakerArr::kStereo => Some(2),
        _ => None,
    }
}

pub(super) fn is_supported_effect_arrangement(
    input: SpeakerArrangement,
    output: SpeakerArrangement,
) -> bool {
    matches!(
        (input, output),
        (SpeakerArr::kMono, SpeakerArr::kMono)
            | (SpeakerArr::kMono, SpeakerArr::kStereo)
            | (SpeakerArr::kStereo, SpeakerArr::kStereo)
    )
}

pub(super) fn is_supported_sidechain_arrangement(arrangement: SpeakerArrangement) -> bool {
    matches!(arrangement, SpeakerArr::kMono | SpeakerArr::kStereo)
}

pub(super) fn is_valid_bus_index<P: Plugin>(
    plugin: &P,
    media_type: MediaType,
    dir: BusDirection,
    index: i32,
) -> bool {
    if index < 0 {
        return false;
    }

    match (media_type as MediaTypes, dir as BusDirections) {
        (MediaTypes_::kAudio, BusDirections_::kInput) => {
            P::INFO.kind != vesty_core::PluginKind::Instrument
                && (index == 0 || (index == 1 && supports_sidechain(plugin)))
        }
        (MediaTypes_::kAudio, BusDirections_::kOutput) => {
            (index as usize) < output_bus_count(plugin)
        }
        (MediaTypes_::kEvent, BusDirections_::kInput) => {
            P::INFO.kind == vesty_core::PluginKind::Instrument && index == 0
        }
        _ => false,
    }
}

pub(super) fn validate_output_arrangements<P: Plugin>(
    plugin: &P,
    outputs: &[SpeakerArrangement],
    main_input: Option<SpeakerArrangement>,
) -> bool {
    if outputs.len() != output_bus_count(plugin) {
        return false;
    }

    for (index, output) in outputs.iter().copied().enumerate() {
        let Some(bus) = output_bus_at(plugin, index) else {
            return false;
        };
        if index == 0 && P::INFO.kind == vesty_core::PluginKind::AudioEffect {
            let Some(input) = main_input else {
                return false;
            };
            if !is_supported_effect_arrangement(input, output) {
                return false;
            }
        } else if !is_supported_output_bus_arrangement(bus, output) {
            return false;
        }
    }
    true
}

pub(super) fn visible_program_lists<P: Plugin>(
    plugin: &P,
) -> impl Iterator<Item = &'static ProgramList> {
    plugin
        .program_lists()
        .iter()
        .filter(|list| !list.is_empty())
}

pub(super) fn program_list_by_index<P: Plugin>(
    plugin: &P,
    index: int32,
) -> Option<&'static ProgramList> {
    (index >= 0)
        .then_some(index as usize)
        .and_then(|index| visible_program_lists(plugin).nth(index))
}

pub(super) fn program_list_by_id<P: Plugin>(
    plugin: &P,
    id: ProgramListID,
) -> Option<&'static ProgramList> {
    visible_program_lists(plugin).find(|list| list.id as ProgramListID == id)
}

pub(super) fn program_list_by_id_or_root<P: Plugin>(
    plugin: &P,
    list_or_unit_id: int32,
) -> Option<&'static ProgramList> {
    program_list_by_id(plugin, list_or_unit_id).or_else(|| {
        (list_or_unit_id == kRootUnitId)
            .then(|| program_list_by_index(plugin, ROOT_UNIT_PROGRAM_LIST_INDEX as int32))?
    })
}

pub(super) fn program_selection_by_id<P: Plugin>(
    plugin: &P,
    list_id: ProgramListID,
    program_index: int32,
) -> Option<(u32, usize)> {
    if program_index < 0 {
        return None;
    }
    let list = program_list_by_id(plugin, list_id)?;
    let program_index = program_index as usize;
    list.programs.get(program_index)?;
    Some((list.id, program_index))
}

pub(super) fn program_selection_by_id_or_root<P: Plugin>(
    plugin: &P,
    list_or_unit_id: int32,
    program_index: int32,
) -> Option<(u32, usize)> {
    if program_index < 0 {
        return None;
    }
    let list = program_list_by_id_or_root(plugin, list_or_unit_id)?;
    let program_index = program_index as usize;
    list.programs.get(program_index)?;
    Some((list.id, program_index))
}

pub(super) fn program_selection_for_param_value<P: Plugin>(
    plugin: &P,
    spec: &vesty_params::ParamSpec,
    normalized: f64,
) -> Option<(u32, usize)> {
    if !spec.flags.program_change {
        return None;
    }
    let list = visible_program_lists(plugin).next()?;
    let plain = normalized_to_plain(spec, normalized.clamp(0.0, 1.0));
    if !plain.is_finite() {
        return None;
    }
    let program_index = plain.round();
    if program_index < 0.0 || program_index > usize::MAX as f64 {
        return None;
    }
    let program_index = program_index as usize;
    list.programs.get(program_index)?;
    Some((list.id, program_index))
}

pub(super) fn visible_program_attributes<P: Plugin>(
    plugin: &P,
    list_id: u32,
    program_index: usize,
) -> impl Iterator<Item = &'static ProgramAttribute> {
    plugin
        .program_attributes(list_id, program_index)
        .iter()
        .filter(|attribute| attribute.is_valid())
}

pub(super) fn visible_program_pitch_names<P: Plugin>(
    plugin: &P,
    list_id: u32,
    program_index: usize,
) -> impl Iterator<Item = &'static ProgramPitchName> {
    plugin
        .program_pitch_names(list_id, program_index)
        .iter()
        .filter(|pitch| pitch.is_valid())
}

pub(super) fn visible_note_expression_value_types<P: Plugin>(
    plugin: &P,
) -> impl Iterator<Item = &'static NoteExpressionValueType> {
    plugin
        .note_expression_value_types()
        .iter()
        .filter(|expression| expression.is_valid())
}

pub(super) fn note_expression_value_type_by_index<P: Plugin>(
    plugin: &P,
    index: int32,
) -> Option<&'static NoteExpressionValueType> {
    (index >= 0)
        .then_some(index as usize)
        .and_then(|index| visible_note_expression_value_types(plugin).nth(index))
}

pub(super) fn note_expression_value_type_by_id<P: Plugin>(
    plugin: &P,
    id: NoteExpressionTypeID,
) -> Option<&'static NoteExpressionValueType> {
    visible_note_expression_value_types(plugin).find(|expression| expression.type_id == id)
}

pub(super) fn visible_note_expression_physical_ui_mappings<P: Plugin>(
    plugin: &P,
) -> impl Iterator<Item = &'static NoteExpressionPhysicalUiMapping> {
    plugin
        .note_expression_physical_ui_mappings()
        .iter()
        .filter(|mapping| {
            mapping.is_valid()
                && note_expression_value_type_by_id(plugin, mapping.note_expression_type_id)
                    .is_some()
        })
}

pub(super) fn note_expression_bus_channel_valid<P: Plugin>(
    bus_index: int32,
    channel: int16,
) -> bool {
    P::INFO.kind == vesty_core::PluginKind::Instrument
        && bus_index == 0
        && (-1..=15).contains(&channel)
}

pub(super) fn note_expression_type_flags(flags: vesty_core::NoteExpressionValueFlags) -> int32 {
    let mut raw = 0;
    if flags.bipolar {
        raw |= NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsBipolar as int32;
    }
    if flags.one_shot {
        raw |= NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsOneShot as int32;
    }
    if flags.absolute {
        raw |= NoteExpressionTypeInfo_::NoteExpressionTypeFlags_::kIsAbsolute as int32;
    }
    raw
}

pub(super) fn silence_flags_for_channel_count(channel_count: usize) -> u64 {
    match channel_count {
        0 => 0,
        1..64 => (1_u64 << channel_count) - 1,
        _ => u64::MAX,
    }
}

pub(super) fn setup_block_size(setup: &ProcessSetup) -> Option<usize> {
    if setup.maxSamplesPerBlock <= 0 {
        return None;
    }
    let frames = setup.maxSamplesPerBlock as usize;
    (frames <= MAX_SETUP_BLOCK_SIZE).then_some(frames)
}

pub(super) fn setup_sample_rate(setup: &ProcessSetup) -> Option<f64> {
    (setup.sampleRate.is_finite() && setup.sampleRate > 0.0).then_some(setup.sampleRate)
}

pub(super) fn setup_sample_size_supported(setup: &ProcessSetup) -> bool {
    matches!(
        setup.symbolicSampleSize as SymbolicSampleSizes,
        SymbolicSampleSizes_::kSample32 | SymbolicSampleSizes_::kSample64
    )
}

pub(super) fn process_block_frames(process_data: &ProcessData) -> Result<usize, usize> {
    if process_data.numSamples < 0 {
        return Err(0);
    }

    Ok(process_data.numSamples as usize)
}

#[derive(Clone, Copy)]
pub(super) struct ProcessOutputLayout<T> {
    channels: [*mut T; MAX_AUDIO_OUTPUT_CHANNELS],
    bus_channels: [usize; MAX_AUDIO_OUTPUT_BUSES],
    bus_count: usize,
    channel_count: usize,
}

impl<T> ProcessOutputLayout<T> {
    fn new() -> Self {
        Self {
            channels: [std::ptr::null_mut(); MAX_AUDIO_OUTPUT_CHANNELS],
            bus_channels: [0; MAX_AUDIO_OUTPUT_BUSES],
            bus_count: 0,
            channel_count: 0,
        }
    }
}

pub(super) fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    // SAFETY: `[MaybeUninit<T>; N]` may be left uninitialized; callers only read the prefix they
    // explicitly initialized before constructing slices from this stack storage.
    unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }
}

#[derive(Default)]
pub(super) struct Sample64Scratch {
    inputs: [Vec<f32>; MAX_MAIN_IO_CHANNELS],
    sidechain: [Vec<f32>; MAX_SIDECHAIN_CHANNELS],
    outputs: [Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    capacity: usize,
}

impl Sample64Scratch {
    fn prepare(&mut self, frames: usize) {
        self.capacity = frames;
        for channel in self
            .inputs
            .iter_mut()
            .chain(self.sidechain.iter_mut())
            .chain(self.outputs.iter_mut())
        {
            channel.resize(frames, 0.0);
        }
    }

    fn has_capacity(&self, frames: usize) -> bool {
        frames <= self.capacity
    }
}

unsafe fn copy_f64_to_f32(src: *const Sample64, dst: &mut [f32]) {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let src = slice::from_raw_parts(src, dst.len());
        for (src, dst) in src.iter().zip(dst.iter_mut()) {
            *dst = *src as f32;
        }
    }
}

unsafe fn copy_f32_to_f64(src: &[f32], dst: *mut Sample64) {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let dst = slice::from_raw_parts_mut(dst, src.len());
        for (src, dst) in src.iter().zip(dst.iter_mut()) {
            *dst = f64::from(*src);
        }
    }
}

unsafe fn audio_bus_channels32<'a>(
    bus: &AudioBusBuffers,
    expected_channels: usize,
) -> Option<&'a [*mut Sample32]> {
    // SAFETY: The caller is inside the VST3 process callback and `bus` comes from the host-provided `ProcessData`; null and channel-count checks guard the raw slice creation.
    unsafe {
        if bus.numChannels <= 0
            || bus.numChannels as usize != expected_channels
            || bus.__field0.channelBuffers32.is_null()
        {
            return None;
        }
        // SAFETY: VST3 provides `expected_channels` pointers for the selected sample-size buffer
        // union; the host count was checked before constructing this slice.
        Some(slice::from_raw_parts(
            bus.__field0.channelBuffers32,
            expected_channels,
        ))
    }
}

unsafe fn valid_audio_bus_channels32<'a>(
    bus: &AudioBusBuffers,
    max_channels: usize,
) -> Option<&'a [*mut Sample32]> {
    // SAFETY: Forwarding the caller's host callback contract; this helper immediately bounds and null-checks the returned channel list.
    let channel_count = bus.numChannels as usize;
    if bus.numChannels <= 0 || channel_count > max_channels {
        return None;
    }
    // SAFETY: `channel_count` was checked to be positive and within the fixed realtime input
    // limit before using the host-provided channel pointer array.
    let channels = unsafe { audio_bus_channels32(bus, channel_count)? };
    if channels.iter().any(|channel| channel.is_null()) {
        return None;
    }
    Some(channels)
}

unsafe fn audio_bus_channels64<'a>(
    bus: &AudioBusBuffers,
    expected_channels: usize,
) -> Option<&'a [*mut Sample64]> {
    // SAFETY: The caller is inside the VST3 process callback and `bus` comes from the host-provided `ProcessData`; null and channel-count checks guard the raw slice creation.
    unsafe {
        if bus.numChannels <= 0
            || bus.numChannels as usize != expected_channels
            || bus.__field0.channelBuffers64.is_null()
        {
            return None;
        }
        // SAFETY: VST3 provides `expected_channels` pointers for the selected sample-size buffer
        // union; the host count was checked before constructing this slice.
        Some(slice::from_raw_parts(
            bus.__field0.channelBuffers64,
            expected_channels,
        ))
    }
}

unsafe fn valid_audio_bus_channels64<'a>(
    bus: &AudioBusBuffers,
    max_channels: usize,
) -> Option<&'a [*mut Sample64]> {
    // SAFETY: Forwarding the caller's host callback contract; this helper immediately bounds and null-checks the returned channel list.
    let channel_count = bus.numChannels as usize;
    if bus.numChannels <= 0 || channel_count > max_channels {
        return None;
    }
    // SAFETY: `channel_count` was checked to be positive and within the fixed realtime input
    // limit before using the host-provided channel pointer array.
    let channels = unsafe { audio_bus_channels64(bus, channel_count)? };
    if channels.iter().any(|channel| channel.is_null()) {
        return None;
    }
    Some(channels)
}

unsafe fn input_buses<'a, P: Plugin>(
    plugin: &P,
    process_data: &ProcessData,
) -> Option<&'a [AudioBusBuffers]> {
    if process_data.numInputs == 0 {
        return Some(&[]);
    }
    if process_data.numInputs < 0
        || process_data.inputs.is_null()
        || process_data.numInputs as usize > declared_input_bus_count::<P>(plugin)
    {
        return None;
    }

    // SAFETY: The host-provided input bus count is non-negative, non-zero, bounded by the
    // plugin-declared input bus count, and `inputs` is non-null for this process callback.
    Some(unsafe { slice::from_raw_parts(process_data.inputs, process_data.numInputs as usize) })
}

unsafe fn output_layout32<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
) -> Option<ProcessOutputLayout<Sample32>> {
    // SAFETY: The host owns `ProcessData.outputs` for the duration of the process callback. This
    // helper only copies channel pointers into a fixed-capacity stack layout after bus prefix and
    // channel-count validation against the plugin descriptor.
    unsafe {
        let expected_bus_count = output_bus_count(plugin);
        if process_data.outputs.is_null()
            || process_data.numOutputs <= 0
            || process_data.numOutputs > expected_bus_count as i32
        {
            return None;
        }

        let host_bus_count = process_data.numOutputs as usize;
        let output_buses = slice::from_raw_parts(process_data.outputs, host_bus_count);
        let mut layout = ProcessOutputLayout::new();
        layout.bus_count = host_bus_count;
        for (bus_index, bus) in output_buses.iter().enumerate() {
            if bus.numChannels <= 0 {
                if bus_index == 0 {
                    return None;
                }
                continue;
            }

            let declared = if bus_index == 0 && P::INFO.kind == vesty_core::PluginKind::AudioEffect
            {
                match bus.numChannels {
                    1 | 2 => bus.numChannels as usize,
                    _ => return None,
                }
            } else {
                arrangement_channel_count(output_arrangements[bus_index].load(Ordering::Relaxed))
                    .map(|count| count as usize)
                    .or_else(|| output_bus_at(plugin, bus_index).map(|bus| bus.channels as usize))?
            };
            let channels = audio_bus_channels32(bus, declared)?;
            if layout.channel_count + declared > MAX_AUDIO_OUTPUT_CHANNELS
                || channels[..declared].iter().any(|channel| channel.is_null())
            {
                return None;
            }

            layout.bus_channels[bus_index] = declared;
            for channel in channels.iter().take(declared) {
                layout.channels[layout.channel_count] = *channel;
                layout.channel_count += 1;
            }
        }
        Some(layout)
    }
}

unsafe fn output_layout64<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
) -> Option<ProcessOutputLayout<Sample64>> {
    // SAFETY: The host owns `ProcessData.outputs` for the duration of the process callback. This
    // helper only copies channel pointers into a fixed-capacity stack layout after bus prefix and
    // channel-count validation against the plugin descriptor.
    unsafe {
        let expected_bus_count = output_bus_count(plugin);
        if process_data.outputs.is_null()
            || process_data.numOutputs <= 0
            || process_data.numOutputs > expected_bus_count as i32
        {
            return None;
        }

        let host_bus_count = process_data.numOutputs as usize;
        let output_buses = slice::from_raw_parts(process_data.outputs, host_bus_count);
        let mut layout = ProcessOutputLayout::new();
        layout.bus_count = host_bus_count;
        for (bus_index, bus) in output_buses.iter().enumerate() {
            if bus.numChannels <= 0 {
                if bus_index == 0 {
                    return None;
                }
                continue;
            }

            let declared = if bus_index == 0 && P::INFO.kind == vesty_core::PluginKind::AudioEffect
            {
                match bus.numChannels {
                    1 | 2 => bus.numChannels as usize,
                    _ => return None,
                }
            } else {
                arrangement_channel_count(output_arrangements[bus_index].load(Ordering::Relaxed))
                    .map(|count| count as usize)
                    .or_else(|| output_bus_at(plugin, bus_index).map(|bus| bus.channels as usize))?
            };
            let channels = audio_bus_channels64(bus, declared)?;
            if layout.channel_count + declared > MAX_AUDIO_OUTPUT_CHANNELS
                || channels[..declared].iter().any(|channel| channel.is_null())
            {
                return None;
            }

            layout.bus_channels[bus_index] = declared;
            for channel in channels.iter().take(declared) {
                layout.channels[layout.channel_count] = *channel;
                layout.channel_count += 1;
            }
        }
        Some(layout)
    }
}

unsafe fn input_views32<'a, const N: usize>(
    channels: Option<&[*mut Sample32]>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a [f32]>; N],
) -> &'a [&'a [f32]] {
    let Some(channels) = channels else {
        return &[];
    };
    let count = channels.len().min(N);
    // SAFETY: Channel pointers were null-checked by `valid_audio_bus_channels32`; each view is
    // bounded to the host-provided block size and stored in caller-owned stack storage.
    unsafe {
        for (index, channel) in channels.iter().take(count).enumerate() {
            storage[index].write(slice::from_raw_parts(*channel, frames));
        }
        slice::from_raw_parts(storage.as_ptr() as *const &'a [f32], count)
    }
}

unsafe fn input_views64<'a, const N: usize>(
    channels: Option<&[*mut Sample64]>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a [f64]>; N],
) -> &'a [&'a [f64]] {
    let Some(channels) = channels else {
        return &[];
    };
    let count = channels.len().min(N);
    // SAFETY: Channel pointers were null-checked by `valid_audio_bus_channels64`; each view is
    // bounded to the host-provided block size and stored in caller-owned stack storage.
    unsafe {
        for (index, channel) in channels.iter().take(count).enumerate() {
            storage[index].write(slice::from_raw_parts(*channel, frames));
        }
        slice::from_raw_parts(storage.as_ptr() as *const &'a [f64], count)
    }
}

unsafe fn output_views32<'a>(
    layout: &ProcessOutputLayout<Sample32>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS],
) -> &'a mut [&'a mut [f32]] {
    // SAFETY: `layout` contains distinct, non-null output channel pointers validated from the host
    // bus list. Each channel is converted exactly once into a mutable slice for the process block.
    unsafe {
        for (index, slot) in storage.iter_mut().enumerate().take(layout.channel_count) {
            slot.write(slice::from_raw_parts_mut(layout.channels[index], frames));
        }
        slice::from_raw_parts_mut(
            storage.as_mut_ptr() as *mut &'a mut [f32],
            layout.channel_count,
        )
    }
}

unsafe fn output_views64<'a>(
    layout: &ProcessOutputLayout<Sample64>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a mut [f64]>; MAX_AUDIO_OUTPUT_CHANNELS],
) -> &'a mut [&'a mut [f64]] {
    // SAFETY: `layout` contains distinct, non-null output channel pointers validated from the host
    // bus list. Each channel is converted exactly once into a mutable slice for the process block.
    unsafe {
        for (index, slot) in storage.iter_mut().enumerate().take(layout.channel_count) {
            slot.write(slice::from_raw_parts_mut(layout.channels[index], frames));
        }
        slice::from_raw_parts_mut(
            storage.as_mut_ptr() as *mut &'a mut [f64],
            layout.channel_count,
        )
    }
}

unsafe fn scratch_input_views_from_f64<'a, const N: usize>(
    scratch: &'a mut [Vec<f32>; N],
    channels: Option<&[*mut Sample64]>,
    frames: usize,
    storage: &'a mut [MaybeUninit<&'a [f32]>; N],
) -> &'a [&'a [f32]] {
    let Some(channels) = channels else {
        return &[];
    };
    let count = channels.len().min(N);
    // SAFETY: Source channel pointers were null-checked by `valid_audio_bus_channels64`; scratch
    // capacity is validated before this helper is called.
    unsafe {
        let base = scratch.as_mut_ptr();
        for (index, channel) in channels.iter().take(count).enumerate() {
            let scratch_channel = &mut *base.add(index);
            copy_f64_to_f32(*channel, &mut scratch_channel[..frames]);
            storage[index].write(&scratch_channel[..frames]);
        }
        slice::from_raw_parts(storage.as_ptr() as *const &'a [f32], count)
    }
}

unsafe fn scratch_output_views<'a>(
    scratch: &'a mut [Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    frames: usize,
    channel_count: usize,
    storage: &'a mut [MaybeUninit<&'a mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS],
) -> &'a mut [&'a mut [f32]] {
    // SAFETY: The first `channel_count` scratch channels are unique Vecs prepared to `frames`
    // capacity before processing; raw indexing avoids holding overlapping borrows of the array.
    unsafe {
        let base = scratch.as_mut_ptr();
        for (index, slot) in storage.iter_mut().enumerate().take(channel_count) {
            let channel = &mut *base.add(index);
            slot.write(&mut channel[..frames]);
        }
        slice::from_raw_parts_mut(storage.as_mut_ptr() as *mut &'a mut [f32], channel_count)
    }
}

unsafe fn set_output_silence_flags<T>(
    process_data: &ProcessData,
    layout: &ProcessOutputLayout<T>,
    silent: bool,
) {
    // SAFETY: `layout.bus_count` was derived from and bounded by the host output bus slice for this
    // process call. This only mutates per-bus flags after all channel slices have been dropped.
    unsafe {
        if process_data.outputs.is_null() {
            return;
        }
        let output_buses = slice::from_raw_parts_mut(process_data.outputs, layout.bus_count);
        for (bus_index, bus) in output_buses.iter_mut().enumerate() {
            bus.silenceFlags = if silent {
                silence_flags_for_channel_count(layout.bus_channels[bus_index])
            } else {
                0
            };
        }
    }
}

unsafe fn silence_process_outputs32<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
    frames: usize,
) {
    // SAFETY: Reuses the same host output layout validation as normal processing before mutating
    // output buffers and silence flags inside the process callback.
    unsafe {
        let Some(output_layout) = output_layout32(plugin, output_arrangements, process_data) else {
            return;
        };
        for channel in output_layout
            .channels
            .iter()
            .take(output_layout.channel_count)
        {
            slice::from_raw_parts_mut(*channel, frames).fill(0.0);
        }
        set_output_silence_flags(process_data, &output_layout, true);
    }
}

unsafe fn silence_process_outputs64<P: Plugin>(
    plugin: &P,
    output_arrangements: &[AtomicU64; MAX_AUDIO_OUTPUT_BUSES],
    process_data: &ProcessData,
    frames: usize,
) {
    // SAFETY: Reuses the same host output layout validation as normal processing before mutating
    // output buffers and silence flags inside the process callback.
    unsafe {
        let Some(output_layout) = output_layout64(plugin, output_arrangements, process_data) else {
            return;
        };
        clear_output_layout64(&output_layout, frames);
        set_output_silence_flags(process_data, &output_layout, true);
    }
}

unsafe fn clear_output_layout64(layout: &ProcessOutputLayout<Sample64>, frames: usize) {
    // SAFETY: `layout` contains non-null output channel pointers validated from the host bus list.
    unsafe {
        for channel in layout.channels.iter().take(layout.channel_count) {
            slice::from_raw_parts_mut(*channel, frames).fill(0.0);
        }
    }
}

unsafe fn copy_output_layout64_to_scratch(
    layout: &ProcessOutputLayout<Sample64>,
    scratch: &mut [Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    frames: usize,
) {
    // SAFETY: `layout` contains non-null output channel pointers and scratch capacity was checked
    // before entering the realtime copy.
    unsafe {
        for (index, channel) in layout
            .channels
            .iter()
            .take(layout.channel_count)
            .enumerate()
        {
            copy_f64_to_f32(*channel, &mut scratch[index][..frames]);
        }
    }
}

unsafe fn copy_scratch_to_output_layout64(
    scratch: &[Vec<f32>; MAX_AUDIO_OUTPUT_CHANNELS],
    layout: &ProcessOutputLayout<Sample64>,
    frames: usize,
) {
    // SAFETY: `layout` contains non-null output channel pointers and scratch capacity was checked
    // before entering the realtime copy.
    unsafe {
        for (index, channel) in layout
            .channels
            .iter()
            .take(layout.channel_count)
            .enumerate()
        {
            copy_f32_to_f64(&scratch[index][..frames], *channel);
        }
    }
}

pub(super) fn restart_flags_for_host_changes(changes: HostChangeFlags) -> int32 {
    let mut flags = 0;
    if changes.contains(HostChangeFlags::IO) {
        flags |= RestartFlags_::kIoChanged;
    }
    if changes.contains(HostChangeFlags::PARAM_VALUES) {
        flags |= RestartFlags_::kParamValuesChanged;
    }
    if changes.contains(HostChangeFlags::LATENCY) {
        flags |= RestartFlags_::kLatencyChanged;
    }
    if changes.contains(HostChangeFlags::PARAM_TITLES) {
        flags |= RestartFlags_::kParamTitlesChanged;
    }
    flags
}

#[derive(Clone, Debug, Default)]
pub(super) struct Vst3ParamIds {
    pub(super) host_ids: Vec<ParamID>,
    by_host_id: BTreeMap<ParamID, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Vst3ParamIdCollision {
    pub(super) host_id: ParamID,
    pub(super) first_id: String,
    pub(super) second_id: String,
}

impl Vst3ParamIds {
    pub(super) fn try_from_specs(
        specs: &[vesty_params::ParamSpec],
    ) -> Result<Self, Vst3ParamIdCollision> {
        let mut host_ids = Vec::with_capacity(specs.len());
        let mut by_host_id: BTreeMap<ParamID, usize> = BTreeMap::new();
        for (index, spec) in specs.iter().enumerate() {
            let host_id = stable_vst3_param_id(&spec.id);
            host_ids.push(host_id);
            if let Some(first_index) = by_host_id.get(&host_id).copied() {
                return Err(Vst3ParamIdCollision {
                    host_id,
                    first_id: specs[first_index].id.clone(),
                    second_id: spec.id.clone(),
                });
            } else {
                by_host_id.insert(host_id, index);
            }
        }
        Ok(Self {
            host_ids,
            by_host_id,
        })
    }

    pub(super) fn host_id_for_index(&self, index: usize) -> Option<ParamID> {
        self.host_ids.get(index).copied()
    }

    pub(super) fn index_for_host_id(&self, host_id: ParamID) -> Option<usize> {
        self.by_host_id.get(&host_id).copied()
    }
}

pub(super) unsafe fn restart_component_for_host_changes(
    handler: &ComPtr<IComponentHandler>,
    changes: HostChangeFlags,
) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let flags = restart_flags_for_host_changes(changes);
        if flags == 0 {
            kResultOk
        } else {
            handler.restartComponent(flags)
        }
    }
}

impl<P: Plugin + Default> Class for VestyProcessor<P> {
    type Interfaces = (
        IComponent,
        IAudioProcessor,
        IProcessContextRequirements,
        IConnectionPoint,
    );
}

#[derive(Debug)]
pub(crate) enum VestyProcessorInitError {
    ParamSchema,
    ParamIdCollision,
}

impl From<vesty_params::ParamSpecError> for VestyProcessorInitError {
    fn from(_error: vesty_params::ParamSpecError) -> Self {
        Self::ParamSchema
    }
}

impl From<Vst3ParamIdCollision> for VestyProcessorInitError {
    fn from(_error: Vst3ParamIdCollision) -> Self {
        Self::ParamIdCollision
    }
}

impl<P: Plugin + Default> VestyProcessor<P> {
    #[cfg(test)]
    pub(crate) fn with_telemetry_registry(telemetry_registry: Arc<Vst3TelemetryRegistry>) -> Self {
        Self::try_with_telemetry_registry(telemetry_registry)
            .expect("plugin parameter specs should be valid")
    }

    pub(crate) fn try_with_telemetry_registry(
        telemetry_registry: Arc<Vst3TelemetryRegistry>,
    ) -> Result<Self, VestyProcessorInitError> {
        let plugin = P::default();
        let specs = plugin.params().specs();
        vesty_params::validate_param_specs(&specs)?;
        let vst3_param_ids = Vst3ParamIds::try_from_specs(&specs)?;
        let param_count = specs.len();
        let (telemetry_id, meter_producer, log_producer, fault) =
            telemetry_registry.create_channel();
        let sidechain_arrangement = default_sidechain_arrangement(&plugin);
        let output_arrangements = std::array::from_fn(|index| {
            AtomicU64::new(output_bus_at(&plugin, index).map_or(0, output_bus_arrangement))
        });
        let output_bus_active = std::array::from_fn(|index| AtomicBool::new(index == 0));
        Ok(Self {
            plugin,
            kernel: UnsafeCell::new(None),
            events: UnsafeCell::new(FixedEventList::new()),
            final_param_values: UnsafeCell::new(vec![None; param_count]),
            meter_producer: UnsafeCell::new(meter_producer),
            log_producer: UnsafeCell::new(log_producer),
            telemetry_id,
            telemetry_registry,
            vst3_param_ids,
            sample_rate_bits: AtomicU64::new(44_100.0_f64.to_bits()),
            max_block_size: AtomicUsize::new(1024),
            input_arrangement: AtomicU64::new(default_input_arrangement(P::INFO.kind)),
            sidechain_arrangement: AtomicU64::new(sidechain_arrangement),
            output_arrangements,
            input_bus_active: AtomicBool::new(P::INFO.kind != vesty_core::PluginKind::Instrument),
            sidechain_bus_active: AtomicBool::new(false),
            output_bus_active,
            event_input_bus_active: AtomicBool::new(
                P::INFO.kind == vesty_core::PluginKind::Instrument,
            ),
            io_mode: AtomicI32::new(IoModes_::kSimple as IoMode),
            processing_active: AtomicBool::new(true),
            sample64_scratch: UnsafeCell::new(Sample64Scratch::default()),
            connection: Mutex::new(None),
            fault,
        })
    }

    #[cfg(test)]
    pub(crate) fn bus_active_for_test(
        &self,
        media_type: MediaType,
        dir: BusDirection,
        index: i32,
    ) -> Option<bool> {
        if !is_valid_bus_index(&self.plugin, media_type, dir, index) {
            return None;
        }

        match (media_type as MediaTypes, dir as BusDirections, index) {
            (MediaTypes_::kAudio, BusDirections_::kInput, 0) => {
                Some(self.input_bus_active.load(Ordering::Relaxed))
            }
            (MediaTypes_::kAudio, BusDirections_::kInput, 1) => {
                Some(self.sidechain_bus_active.load(Ordering::Relaxed))
            }
            (MediaTypes_::kAudio, BusDirections_::kOutput, index) => self
                .output_bus_active
                .get(index as usize)
                .map(|active| active.load(Ordering::Relaxed)),
            (MediaTypes_::kEvent, BusDirections_::kInput, 0) => {
                Some(self.event_input_bus_active.load(Ordering::Relaxed))
            }
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn io_mode_for_test(&self) -> IoMode {
        self.io_mode.load(Ordering::Relaxed)
    }

    unsafe fn ensure_kernel(&self) {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let slot = &mut *self.kernel.get();
            if slot.is_none() {
                let sample_rate = f64::from_bits(self.sample_rate_bits.load(Ordering::Relaxed));
                let max_block_size = self.max_block_size.load(Ordering::Relaxed);
                let mut kernel = self.plugin.create_kernel(KernelInit {
                    sample_rate,
                    max_block_size,
                });
                kernel.prepare(PrepareContext {
                    sample_rate,
                    max_block_size,
                });
                *slot = Some(kernel);
            }
        }
    }

    unsafe fn collect_parameter_changes(
        &self,
        process_data: &ProcessData,
        events: &mut FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>,
        final_values: &mut [Option<f64>],
    ) {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(param_changes) = ComRef::from_raw(process_data.inputParameterChanges) else {
                return;
            };

            let param_count = param_changes.getParameterCount();
            for param_index in 0..param_count {
                let Some(param_queue) =
                    ComRef::from_raw(param_changes.getParameterData(param_index))
                else {
                    continue;
                };
                let point_count = param_queue.getPointCount();
                if point_count <= 0 {
                    continue;
                }
                let host_param_id = param_queue.getParameterId();
                let Some(param_index) = self.vst3_param_ids.index_for_host_id(host_param_id) else {
                    continue;
                };
                let Some(final_value) = final_values.get_mut(param_index) else {
                    continue;
                };

                for point_index in 0..point_count {
                    let mut sample_offset = 0;
                    let mut value = 0.0;
                    if param_queue.getPoint(point_index, &mut sample_offset, &mut value)
                        != kResultTrue
                    {
                        continue;
                    }

                    let normalized = value.clamp(0.0, 1.0);
                    *final_value = Some(normalized);
                    let _ = events.push(VestyEvent::Param {
                        sample_offset: sample_offset.max(0) as u32,
                        handle: vesty_params::ParamHandle::from_index(param_index),
                        id_hash: host_param_id,
                        normalized,
                    });
                }
            }
        }
    }

    fn apply_final_parameter_values(&self, final_values: &[Option<f64>]) {
        for (index, value) in final_values.iter().copied().enumerate() {
            let Some(value) = value else {
                continue;
            };
            let _ = self
                .plugin
                .params()
                .set_normalized_by_handle(vesty_params::ParamHandle::from_index(index), value);
        }
    }

    unsafe fn collect_input_events(
        &self,
        process_data: &ProcessData,
        events: &mut FixedEventList<VestyEvent, MAX_BLOCK_EVENTS>,
    ) {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(input_events) = ComRef::from_raw(process_data.inputEvents) else {
                return;
            };

            let event_count = input_events.getEventCount();
            for index in 0..event_count {
                let mut event = MaybeUninit::<Event>::zeroed();
                if input_events.getEvent(index, event.as_mut_ptr()) != kResultOk {
                    continue;
                }

                let event = event.assume_init();
                let sample_offset = event.sampleOffset.max(0) as u32;
                match event.r#type as Event_::EventTypes {
                    Event_::EventTypes_::kNoteOnEvent => {
                        let note = event.__field0.noteOn;
                        let _ = events.push(VestyEvent::NoteOn {
                            sample_offset,
                            channel: clamp_midi_channel_i16(note.channel),
                            key: clamp_midi_key(note.pitch),
                            velocity: note.velocity.clamp(0.0, 1.0),
                            note_id: note.noteId,
                        });
                    }
                    Event_::EventTypes_::kNoteOffEvent => {
                        let note = event.__field0.noteOff;
                        let _ = events.push(VestyEvent::NoteOff {
                            sample_offset,
                            channel: clamp_midi_channel_i16(note.channel),
                            key: clamp_midi_key(note.pitch),
                            velocity: note.velocity.clamp(0.0, 1.0),
                            note_id: note.noteId,
                        });
                    }
                    Event_::EventTypes_::kPolyPressureEvent => {
                        let pressure = event.__field0.polyPressure;
                        let _ = events.push(VestyEvent::PolyPressure {
                            sample_offset,
                            channel: clamp_midi_channel_i16(pressure.channel),
                            key: clamp_midi_key(pressure.pitch),
                            pressure: pressure.pressure.clamp(0.0, 1.0),
                            note_id: pressure.noteId,
                        });
                    }
                    Event_::EventTypes_::kDataEvent => {
                        let data = event.__field0.data;
                        if data.r#type == DataEvent_::DataTypes_::kMidiSysEx as uint32 {
                            let (payload, data_len, truncated) =
                                copy_sysex_data(data.bytes, data.size);
                            let _ = events.push(VestyEvent::SysEx {
                                sample_offset,
                                data_len,
                                data: payload,
                                truncated,
                            });
                        }
                    }
                    Event_::EventTypes_::kNoteExpressionValueEvent => {
                        let expression = event.__field0.noteExpressionValue;
                        let value = if expression.value.is_finite() {
                            expression.value
                        } else {
                            0.0
                        };
                        let _ = events.push(VestyEvent::NoteExpressionValue {
                            sample_offset,
                            type_id: expression.typeId,
                            note_id: expression.noteId,
                            value,
                        });
                    }
                    Event_::EventTypes_::kNoteExpressionIntValueEvent => {
                        let expression = event.__field0.noteExpressionIntValue;
                        let _ = events.push(VestyEvent::NoteExpressionInt {
                            sample_offset,
                            type_id: expression.typeId,
                            note_id: expression.noteId,
                            value: expression.value,
                        });
                    }
                    Event_::EventTypes_::kNoteExpressionTextEvent => {
                        let expression = event.__field0.noteExpressionText;
                        let (text, text_len) =
                            copy_note_expression_text(expression.text, expression.textLen);
                        let _ = events.push(VestyEvent::NoteExpressionText {
                            sample_offset,
                            type_id: expression.typeId,
                            note_id: expression.noteId,
                            text_len,
                            text,
                        });
                    }
                    Event_::EventTypes_::kLegacyMIDICCOutEvent => {
                        let midi = event.__field0.midiCCOut;
                        let channel = clamp_midi_channel_i8(midi.channel);
                        let value = clamp_midi7_i8(midi.value);
                        let value2 = clamp_midi7_i8(midi.value2);
                        match u32::from(midi.controlNumber) {
                            LEGACY_PITCH_BEND_CONTROL => {
                                let _ = events.push(VestyEvent::PitchBend {
                                    sample_offset,
                                    channel,
                                    value: midi_pitch_bend_to_bipolar(value, value2),
                                });
                            }
                            LEGACY_AFTERTOUCH_CONTROL => {
                                let _ = events.push(VestyEvent::ChannelPressure {
                                    sample_offset,
                                    channel,
                                    pressure: midi7_to_unit(value),
                                });
                            }
                            control => {
                                let _ = events.push(VestyEvent::MidiCc {
                                    sample_offset,
                                    channel,
                                    controller: control as u16,
                                    value: midi7_to_unit(value),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    unsafe fn transport(&self, process_data: &ProcessData) -> Transport {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if process_data.processContext.is_null() {
                return Transport::default();
            }

            let context = &*process_data.processContext;
            Transport {
                playing: context.state & PROCESS_CONTEXT_PLAYING_FLAG != 0,
                tempo_bpm: (context.state & PROCESS_CONTEXT_TEMPO_VALID_FLAG != 0)
                    .then_some(context.tempo),
                position_samples: Some(context.projectTimeSamples),
            }
        }
    }

    unsafe fn run_kernel<'a>(
        &'a self,
        inputs: &'a [&'a [f32]],
        sidechain: &'a [&'a [f32]],
        outputs: &'a mut [&'a mut [f32]],
        events: &'a [VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> ProcessResult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let slot = &mut *self.kernel.get();
            let Some(kernel) = slot.as_mut() else {
                for output in outputs.iter_mut() {
                    output.fill(0.0);
                }
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::HostWarning {
                    code: RT_LOG_CODE_PROCESS_WITHOUT_KERNEL,
                    value: events.len() as i64,
                });
                return ProcessResult::Silence;
            };

            let audio = AudioBuffers::new(inputs, outputs);
            let sidechain = SidechainBuffers::new(sidechain);
            let meter_producer = &mut *self.meter_producer.get();
            let mut context =
                VestyProcessContext::new(audio, self.plugin.params(), events, transport)
                    .with_sidechain(sidechain)
                    .with_process_mode(process_mode)
                    .with_meter_sink(meter_producer);
            let fault_count_before = self.fault.fault_count();
            let result = panic_guard(&self.fault, ProcessResult::Silence, || {
                kernel.process(&mut context)
            });
            if self.fault.fault_count() > fault_count_before {
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::Faulted {
                    code: RT_LOG_CODE_PROCESS_PANIC,
                });
            }
            if result == ProcessResult::Silence {
                context.audio_mut().clear_outputs();
            }
            result
        }
    }

    unsafe fn run_kernel_f64<'a>(
        &'a self,
        inputs: &'a [&'a [f64]],
        sidechain: &'a [&'a [f64]],
        outputs: &'a mut [&'a mut [f64]],
        events: &'a [VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> ProcessResult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let slot = &mut *self.kernel.get();
            let Some(kernel) = slot.as_mut() else {
                for output in outputs.iter_mut() {
                    output.fill(0.0);
                }
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::HostWarning {
                    code: RT_LOG_CODE_PROCESS_WITHOUT_KERNEL,
                    value: events.len() as i64,
                });
                return ProcessResult::Silence;
            };

            let audio = AudioBuffers64::new(inputs, outputs);
            let sidechain = SidechainBuffers64::new(sidechain);
            let meter_producer = &mut *self.meter_producer.get();
            let mut context =
                VestyProcessContext64::new(audio, self.plugin.params(), events, transport)
                    .with_sidechain(sidechain)
                    .with_process_mode(process_mode)
                    .with_meter_sink(meter_producer);
            let fault_count_before = self.fault.fault_count();
            let result = panic_guard(&self.fault, ProcessResult::Silence, || {
                kernel.process_f64(&mut context)
            });
            if self.fault.fault_count() > fault_count_before {
                let log_producer = &mut *self.log_producer.get();
                let _ = log_producer.try_push(RtLogEvent::Faulted {
                    code: RT_LOG_CODE_PROCESS_PANIC,
                });
            }
            if result == ProcessResult::Silence {
                context.audio_mut().clear_outputs();
            }
            result
        }
    }

    unsafe fn process_sample32(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(output_layout) =
                output_layout32(&self.plugin, &self.output_arrangements, process_data)
            else {
                return kResultOk;
            };
            let num_samples = process_data.numSamples.max(0) as usize;
            let Some(input_buses) = input_buses::<P>(&self.plugin, process_data) else {
                silence_process_outputs32(
                    &self.plugin,
                    &self.output_arrangements,
                    process_data,
                    num_samples,
                );
                return kResultOk;
            };
            let input_channels = input_buses
                .first()
                .and_then(|bus| valid_audio_bus_channels32(bus, MAX_MAIN_IO_CHANNELS));
            let sidechain_channels = if supports_sidechain(&self.plugin) {
                input_buses
                    .get(1)
                    .and_then(|bus| valid_audio_bus_channels32(bus, MAX_SIDECHAIN_CHANNELS))
            } else {
                None
            };

            let process_result = {
                let mut input_storage: [MaybeUninit<&[f32]>; MAX_MAIN_IO_CHANNELS] = uninit_array();
                let mut sidechain_storage: [MaybeUninit<&[f32]>; MAX_SIDECHAIN_CHANNELS] =
                    uninit_array();
                let mut output_storage: [MaybeUninit<&mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS] =
                    uninit_array();
                let inputs = input_views32(input_channels, num_samples, &mut input_storage);
                let sidechain =
                    input_views32(sidechain_channels, num_samples, &mut sidechain_storage);
                let outputs = output_views32(&output_layout, num_samples, &mut output_storage);
                self.run_kernel(inputs, sidechain, outputs, events, transport, process_mode)
            };
            set_output_silence_flags(
                process_data,
                &output_layout,
                process_result == ProcessResult::Silence,
            );
            kResultOk
        }
    }

    unsafe fn process_sample64(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: Dispatch stays within the VST3 process callback contract; both callees validate
        // nullable host pointers and bus/channel shape before constructing slices.
        unsafe {
            if P::Kernel::SUPPORTS_F64 {
                return self.process_sample64_native(process_data, events, transport, process_mode);
            }
            self.process_sample64_via_f32_scratch(process_data, events, transport, process_mode)
        }
    }

    unsafe fn process_sample64_native(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(output_layout) =
                output_layout64(&self.plugin, &self.output_arrangements, process_data)
            else {
                return kResultOk;
            };
            let num_samples = process_data.numSamples.max(0) as usize;
            let Some(input_buses) = input_buses::<P>(&self.plugin, process_data) else {
                silence_process_outputs64(
                    &self.plugin,
                    &self.output_arrangements,
                    process_data,
                    num_samples,
                );
                return kResultOk;
            };
            let input_channels = input_buses
                .first()
                .and_then(|bus| valid_audio_bus_channels64(bus, MAX_MAIN_IO_CHANNELS));
            let sidechain_channels = if supports_sidechain(&self.plugin) {
                input_buses
                    .get(1)
                    .and_then(|bus| valid_audio_bus_channels64(bus, MAX_SIDECHAIN_CHANNELS))
            } else {
                None
            };

            let process_result = {
                let mut input_storage: [MaybeUninit<&[f64]>; MAX_MAIN_IO_CHANNELS] = uninit_array();
                let mut sidechain_storage: [MaybeUninit<&[f64]>; MAX_SIDECHAIN_CHANNELS] =
                    uninit_array();
                let mut output_storage: [MaybeUninit<&mut [f64]>; MAX_AUDIO_OUTPUT_CHANNELS] =
                    uninit_array();
                let inputs = input_views64(input_channels, num_samples, &mut input_storage);
                let sidechain =
                    input_views64(sidechain_channels, num_samples, &mut sidechain_storage);
                let outputs = output_views64(&output_layout, num_samples, &mut output_storage);
                self.run_kernel_f64(inputs, sidechain, outputs, events, transport, process_mode)
            };
            set_output_silence_flags(
                process_data,
                &output_layout,
                process_result == ProcessResult::Silence,
            );
            kResultOk
        }
    }

    unsafe fn process_sample64_via_f32_scratch(
        &self,
        process_data: &ProcessData,
        events: &[VestyEvent],
        transport: Transport,
        process_mode: ProcessMode,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let Some(output_layout) =
                output_layout64(&self.plugin, &self.output_arrangements, process_data)
            else {
                return kResultOk;
            };
            let num_samples = process_data.numSamples.max(0) as usize;
            let scratch = &mut *self.sample64_scratch.get();
            if !scratch.has_capacity(num_samples) {
                clear_output_layout64(&output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, true);
                return kResultOk;
            }
            copy_output_layout64_to_scratch(&output_layout, &mut scratch.outputs, num_samples);

            let Some(input_buses) = input_buses::<P>(&self.plugin, process_data) else {
                clear_output_layout64(&output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, true);
                return kResultOk;
            };
            let input_channels = input_buses
                .first()
                .and_then(|bus| valid_audio_bus_channels64(bus, MAX_MAIN_IO_CHANNELS));
            let sidechain_channels = if supports_sidechain(&self.plugin) {
                input_buses
                    .get(1)
                    .and_then(|bus| valid_audio_bus_channels64(bus, MAX_SIDECHAIN_CHANNELS))
            } else {
                None
            };

            let process_result = {
                let mut input_storage: [MaybeUninit<&[f32]>; MAX_MAIN_IO_CHANNELS] = uninit_array();
                let mut sidechain_storage: [MaybeUninit<&[f32]>; MAX_SIDECHAIN_CHANNELS] =
                    uninit_array();
                let mut output_storage: [MaybeUninit<&mut [f32]>; MAX_AUDIO_OUTPUT_CHANNELS] =
                    uninit_array();
                let inputs = scratch_input_views_from_f64(
                    &mut scratch.inputs,
                    input_channels,
                    num_samples,
                    &mut input_storage,
                );
                let sidechain = scratch_input_views_from_f64(
                    &mut scratch.sidechain,
                    sidechain_channels,
                    num_samples,
                    &mut sidechain_storage,
                );
                let outputs = scratch_output_views(
                    &mut scratch.outputs,
                    num_samples,
                    output_layout.channel_count,
                    &mut output_storage,
                );
                self.run_kernel(inputs, sidechain, outputs, events, transport, process_mode)
            };

            if process_result == ProcessResult::Silence {
                clear_output_layout64(&output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, true);
            } else {
                copy_scratch_to_output_layout64(&scratch.outputs, &output_layout, num_samples);
                set_output_silence_flags(process_data, &output_layout, false);
            }
            kResultOk
        }
    }
}

pub(super) fn vst3_process_mode(process_data: &ProcessData) -> ProcessMode {
    match process_data.processMode {
        mode if mode == ProcessModes_::kOffline as int32 => ProcessMode::Offline,
        mode if mode == ProcessModes_::kPrefetch as int32 => ProcessMode::Prefetch,
        _ => ProcessMode::Realtime,
    }
}

impl<P: Plugin + Default> Drop for VestyProcessor<P> {
    fn drop(&mut self) {
        self.telemetry_registry
            .remove_meter_consumer(self.telemetry_id);
    }
}

#[vesty_macros::vst3_panic_boundary]
impl<P: Plugin + Default> IPluginBaseTrait for VestyProcessor<P> {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

#[vesty_macros::vst3_panic_boundary]
impl<P: Plugin + Default> IComponentTrait for VestyProcessor<P> {
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        if class_id.is_null() {
            return kInvalidArgument;
        }

        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; the nullable output pointer has been checked above.
        unsafe {
            *class_id = controller_cid::<P>();
            kResultOk
        }
    }

    unsafe fn setIoMode(&self, mode: IoMode) -> tresult {
        match mode {
            mode if mode == IoModes_::kSimple as IoMode
                || mode == IoModes_::kAdvanced as IoMode
                || mode == IoModes_::kOfflineProcessing as IoMode =>
            {
                self.io_mode.store(mode, Ordering::Release);
                kResultOk
            }
            _ => kInvalidArgument,
        }
    }

    unsafe fn getBusCount(&self, mediaType: MediaType, dir: BusDirection) -> i32 {
        match mediaType as MediaTypes {
            MediaTypes_::kAudio => match dir as BusDirections {
                BusDirections_::kInput => {
                    if P::INFO.kind == vesty_core::PluginKind::Instrument {
                        0
                    } else if supports_sidechain(&self.plugin) {
                        2
                    } else {
                        1
                    }
                }
                BusDirections_::kOutput => output_bus_count(&self.plugin) as i32,
                _ => 0,
            },
            MediaTypes_::kEvent => match dir as BusDirections {
                BusDirections_::kInput if P::INFO.kind == vesty_core::PluginKind::Instrument => 1,
                _ => 0,
            },
            _ => 0,
        }
    }

    unsafe fn getBusInfo(
        &self,
        mediaType: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if index < 0 || bus.is_null() {
                return kInvalidArgument;
            }
            let bus = &mut *bus;
            match (mediaType as MediaTypes, dir as BusDirections) {
                (MediaTypes_::kAudio, BusDirections_::kInput)
                    if P::INFO.kind != vesty_core::PluginKind::Instrument && index == 0 =>
                {
                    bus.mediaType = MediaTypes_::kAudio as MediaType;
                    bus.direction = BusDirections_::kInput as BusDirection;
                    bus.channelCount =
                        arrangement_channel_count(self.input_arrangement.load(Ordering::Relaxed))
                            .unwrap_or(2);
                    copy_wstring("Input", &mut bus.name);
                    bus.busType = BusTypes_::kMain as BusType;
                    bus.flags = DEFAULT_ACTIVE_BUS_FLAG;
                    kResultOk
                }
                (MediaTypes_::kAudio, BusDirections_::kInput)
                    if P::INFO.kind != vesty_core::PluginKind::Instrument
                        && index == 1
                        && supports_sidechain(&self.plugin) =>
                {
                    bus.mediaType = MediaTypes_::kAudio as MediaType;
                    bus.direction = BusDirections_::kInput as BusDirection;
                    bus.channelCount = arrangement_channel_count(
                        self.sidechain_arrangement.load(Ordering::Relaxed),
                    )
                    .unwrap_or(2);
                    copy_wstring("Sidechain", &mut bus.name);
                    bus.busType = BusTypes_::kAux as BusType;
                    bus.flags = 0;
                    kResultOk
                }
                (MediaTypes_::kAudio, BusDirections_::kOutput)
                    if (index as usize) < output_bus_count(&self.plugin) =>
                {
                    let index = index as usize;
                    let Some(output_bus) = output_bus_at(&self.plugin, index) else {
                        return kInvalidArgument;
                    };
                    bus.mediaType = MediaTypes_::kAudio as MediaType;
                    bus.direction = BusDirections_::kOutput as BusDirection;
                    bus.channelCount = arrangement_channel_count(
                        self.output_arrangements[index].load(Ordering::Relaxed),
                    )
                    .unwrap_or(output_bus.channels as i32);
                    copy_wstring(output_bus.name, &mut bus.name);
                    bus.busType = if index == 0 {
                        BusTypes_::kMain
                    } else {
                        BusTypes_::kAux
                    } as BusType;
                    bus.flags = if index == 0 {
                        DEFAULT_ACTIVE_BUS_FLAG
                    } else {
                        0
                    };
                    kResultOk
                }
                (MediaTypes_::kEvent, BusDirections_::kInput)
                    if P::INFO.kind == vesty_core::PluginKind::Instrument && index == 0 =>
                {
                    bus.mediaType = MediaTypes_::kEvent as MediaType;
                    bus.direction = BusDirections_::kInput as BusDirection;
                    bus.channelCount = 1;
                    copy_wstring("Event Input", &mut bus.name);
                    bus.busType = BusTypes_::kMain as BusType;
                    bus.flags = DEFAULT_ACTIVE_BUS_FLAG;
                    kResultOk
                }
                _ => kInvalidArgument,
            }
        }
    }

    unsafe fn getRoutingInfo(
        &self,
        in_info: *mut RoutingInfo,
        out_info: *mut RoutingInfo,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if in_info.is_null() || out_info.is_null() {
                return kInvalidArgument;
            }

            let input = *in_info;
            let valid_input = if P::INFO.kind == vesty_core::PluginKind::Instrument {
                input.mediaType == MediaTypes_::kEvent as MediaType
                    && input.busIndex == 0
                    && (-1..=15).contains(&input.channel)
            } else {
                match input.busIndex {
                    0 => {
                        let input_channels = arrangement_channel_count(
                            self.input_arrangement.load(Ordering::Relaxed),
                        )
                        .unwrap_or(2);
                        input.mediaType == MediaTypes_::kAudio as MediaType
                            && (input.channel == -1 || (0..input_channels).contains(&input.channel))
                    }
                    1 if supports_sidechain(&self.plugin) => {
                        let sidechain_channels = arrangement_channel_count(
                            self.sidechain_arrangement.load(Ordering::Relaxed),
                        )
                        .unwrap_or(2);
                        input.mediaType == MediaTypes_::kAudio as MediaType
                            && (input.channel == -1
                                || (0..sidechain_channels).contains(&input.channel))
                    }
                    _ => false,
                }
            };

            if !valid_input {
                return kInvalidArgument;
            }

            *out_info = RoutingInfo {
                mediaType: MediaTypes_::kAudio as MediaType,
                busIndex: 0,
                channel: -1,
            };
            kResultOk
        }
    }

    unsafe fn activateBus(
        &self,
        media_type: MediaType,
        dir: BusDirection,
        index: i32,
        state: TBool,
    ) -> tresult {
        if !is_valid_bus_index(&self.plugin, media_type, dir, index) {
            return kInvalidArgument;
        }

        let active = state != 0;
        match (media_type as MediaTypes, dir as BusDirections, index) {
            (MediaTypes_::kAudio, BusDirections_::kInput, 0) => {
                self.input_bus_active.store(active, Ordering::Relaxed);
            }
            (MediaTypes_::kAudio, BusDirections_::kInput, 1) => {
                self.sidechain_bus_active.store(active, Ordering::Relaxed);
            }
            (MediaTypes_::kAudio, BusDirections_::kOutput, index) => {
                self.output_bus_active[index as usize].store(active, Ordering::Relaxed);
            }
            (MediaTypes_::kEvent, BusDirections_::kInput, 0) => {
                self.event_input_bus_active.store(active, Ordering::Relaxed);
            }
            _ => return kInvalidArgument,
        }
        kResultOk
    }

    unsafe fn setActive(&self, state: TBool) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if state != 0 {
                self.ensure_kernel();
                if let Some(kernel) = (&mut *self.kernel.get()).as_mut() {
                    kernel.reset();
                }
            } else {
                if let Some(kernel) = (&mut *self.kernel.get()).as_mut() {
                    kernel.reset();
                }
                *self.kernel.get() = None;
            }
            kResultOk
        }
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            match read_state_stream(state) {
                Ok(state) => {
                    if apply_state(&self.plugin, state).is_ok() {
                        kResultOk
                    } else {
                        kResultFalse
                    }
                }
                Err(()) => kInvalidArgument,
            }
        }
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            match capture_state(&self.plugin) {
                Ok(captured) => match write_state_stream(state, &captured) {
                    Ok(()) => kResultOk,
                    Err(()) => kInvalidArgument,
                },
                Err(_) => kResultFalse,
            }
        }
    }
}

#[vesty_macros::vst3_panic_boundary]
impl<P: Plugin + Default> IAudioProcessorTrait for VestyProcessor<P> {
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let output_bus_count = output_bus_count(&self.plugin);
            if outputs.is_null() || num_outs != output_bus_count as i32 {
                return kInvalidArgument;
            }

            let output_arrangements = slice::from_raw_parts(outputs, output_bus_count);
            if P::INFO.kind == vesty_core::PluginKind::Instrument {
                if num_ins != 0
                    || !validate_output_arrangements::<P>(&self.plugin, output_arrangements, None)
                {
                    return kResultFalse;
                }
                for (index, output) in output_arrangements.iter().copied().enumerate() {
                    self.output_arrangements[index].store(output, Ordering::Relaxed);
                }
                return kResultTrue;
            }

            let supports_sidechain = supports_sidechain(&self.plugin);
            if (!supports_sidechain && num_ins != 1)
                || (supports_sidechain && !(1..=2).contains(&num_ins))
            {
                return kResultFalse;
            }
            if inputs.is_null() {
                return kInvalidArgument;
            }
            let input_arrangements = slice::from_raw_parts(inputs, num_ins as usize);
            let input = input_arrangements[0];
            if !validate_output_arrangements::<P>(&self.plugin, output_arrangements, Some(input)) {
                return kResultFalse;
            }
            if supports_sidechain && num_ins == 2 {
                let sidechain = input_arrangements[1];
                if !is_supported_sidechain_arrangement(sidechain) {
                    return kResultFalse;
                }
                self.sidechain_arrangement
                    .store(sidechain, Ordering::Relaxed);
            }
            self.input_arrangement.store(input, Ordering::Relaxed);
            for (index, output) in output_arrangements.iter().copied().enumerate() {
                self.output_arrangements[index].store(output, Ordering::Relaxed);
            }
            kResultTrue
        }
    }

    unsafe fn getBusArrangement(
        &self,
        dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if index < 0 || arr.is_null() {
                return kInvalidArgument;
            }
            match dir as BusDirections {
                BusDirections_::kInput
                    if P::INFO.kind != vesty_core::PluginKind::Instrument && index == 0 =>
                {
                    *arr = self.input_arrangement.load(Ordering::Relaxed);
                    kResultOk
                }
                BusDirections_::kInput
                    if P::INFO.kind != vesty_core::PluginKind::Instrument
                        && index == 1
                        && supports_sidechain(&self.plugin) =>
                {
                    *arr = self.sidechain_arrangement.load(Ordering::Relaxed);
                    kResultOk
                }
                BusDirections_::kOutput if (index as usize) < output_bus_count(&self.plugin) => {
                    *arr = self.output_arrangements[index as usize].load(Ordering::Relaxed);
                    kResultOk
                }
                _ => kInvalidArgument,
            }
        }
    }

    unsafe fn canProcessSampleSize(&self, symbolic_sample_size: i32) -> tresult {
        match symbolic_sample_size as SymbolicSampleSizes {
            SymbolicSampleSizes_::kSample32 => kResultOk,
            SymbolicSampleSizes_::kSample64 => kResultOk,
            _ => kInvalidArgument,
        }
    }

    unsafe fn getLatencySamples(&self) -> u32 {
        self.plugin.latency_samples()
    }

    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if setup.is_null() {
                return kInvalidArgument;
            }
            let setup = &*setup;
            if !setup_sample_size_supported(setup) {
                return kInvalidArgument;
            }
            let Some(sample_rate) = setup_sample_rate(setup) else {
                return kInvalidArgument;
            };
            let Some(max_block_size) = setup_block_size(setup) else {
                return kInvalidArgument;
            };

            self.sample_rate_bits
                .store(sample_rate.to_bits(), Ordering::Relaxed);
            self.max_block_size.store(max_block_size, Ordering::Relaxed);
            (&mut *self.sample64_scratch.get()).prepare(max_block_size);
            if let Some(kernel) = (&mut *self.kernel.get()).as_mut() {
                kernel.prepare(PrepareContext {
                    sample_rate,
                    max_block_size,
                });
                kernel.reset();
            } else {
                self.ensure_kernel();
                if let Some(kernel) = (&mut *self.kernel.get()).as_mut() {
                    kernel.reset();
                }
            }
            kResultOk
        }
    }

    unsafe fn setProcessing(&self, state: TBool) -> tresult {
        // SAFETY: Lifecycle callbacks are serialized by the VST3 host outside the process call.
        unsafe {
            let active = state != 0;
            if active {
                self.ensure_kernel();
                if let Some(kernel) = (&mut *self.kernel.get()).as_mut() {
                    kernel.resume();
                }
            } else if let Some(kernel) = (&mut *self.kernel.get()).as_mut() {
                kernel.suspend();
            }
            self.processing_active.store(active, Ordering::Release);
            kResultOk
        }
    }

    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            if data.is_null() {
                return kInvalidArgument;
            }
            let process_data = &*data;
            let symbolic_sample_size = process_data.symbolicSampleSize as SymbolicSampleSizes;
            if !matches!(
                symbolic_sample_size,
                SymbolicSampleSizes_::kSample32 | SymbolicSampleSizes_::kSample64
            ) {
                return kResultOk;
            }
            let _rt_guard = NoAllocGuard::enter();
            if let Err(frames_to_clear) = process_block_frames(process_data) {
                match symbolic_sample_size {
                    SymbolicSampleSizes_::kSample32 => {
                        silence_process_outputs32(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames_to_clear,
                        );
                    }
                    SymbolicSampleSizes_::kSample64 => {
                        silence_process_outputs64(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames_to_clear,
                        );
                    }
                    _ => {}
                }
                return kResultOk;
            }
            if !self.processing_active.load(Ordering::Acquire) {
                let frames = process_data.numSamples.max(0) as usize;
                match symbolic_sample_size {
                    SymbolicSampleSizes_::kSample32 => {
                        silence_process_outputs32(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames,
                        );
                    }
                    SymbolicSampleSizes_::kSample64 => {
                        silence_process_outputs64(
                            &self.plugin,
                            &self.output_arrangements,
                            process_data,
                            frames,
                        );
                    }
                    _ => {}
                }
                return kResultOk;
            }
            let events = &mut *self.events.get();
            events.clear();
            let final_param_values = &mut *self.final_param_values.get();
            final_param_values.fill(None);
            self.collect_parameter_changes(process_data, events, final_param_values);
            self.collect_input_events(process_data, events);
            sort_events_by_sample_offset(events);
            let transport = self.transport(process_data);
            let process_mode = vst3_process_mode(process_data);

            let result = match symbolic_sample_size {
                SymbolicSampleSizes_::kSample32 => {
                    self.process_sample32(process_data, events.as_slice(), transport, process_mode)
                }
                SymbolicSampleSizes_::kSample64 => {
                    self.process_sample64(process_data, events.as_slice(), transport, process_mode)
                }
                _ => kResultOk,
            };
            self.apply_final_parameter_values(final_param_values);
            result
        }
    }

    unsafe fn getTailSamples(&self) -> u32 {
        self.plugin.tail_samples()
    }
}

#[vesty_macros::vst3_panic_boundary]
impl<P: Plugin + Default> IProcessContextRequirementsTrait for VestyProcessor<P> {
    unsafe fn getProcessContextRequirements(&self) -> u32 {
        0
    }
}

#[vesty_macros::vst3_panic_boundary]
impl<P: Plugin + Default> IConnectionPointTrait for VestyProcessor<P> {
    unsafe fn connect(&self, other: *mut IConnectionPoint) -> tresult {
        // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
        unsafe {
            let result = connect_connection_point(&self.connection, other);
            if result == kResultOk {
                let _ = notify_telemetry_bind(other, self.telemetry_id);
            }
            result
        }
    }

    unsafe fn disconnect(&self, other: *mut IConnectionPoint) -> tresult {
        disconnect_connection_point(&self.connection, other)
    }

    unsafe fn notify(&self, _message: *mut IMessage) -> tresult {
        kResultOk
    }
}

unsafe fn notify_telemetry_bind(other: *mut IConnectionPoint, telemetry_id: u64) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    unsafe {
        let Some(other) = ComRef::from_raw(other) else {
            return kInvalidArgument;
        };
        let Some(message) = VestyMessage::telemetry_bind(telemetry_id) else {
            return kResultFalse;
        };
        let Some(message) = ComWrapper::new(message).to_com_ptr::<IMessage>() else {
            return kResultFalse;
        };
        other.notify(message.as_ptr())
    }
}

pub(super) fn connect_connection_point(
    slot: &SharedConnectionPoint,
    other: *mut IConnectionPoint,
) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    let Some(other) = (unsafe { ComRef::from_raw(other) }) else {
        return kInvalidArgument;
    };
    let Ok(mut slot) = slot.lock() else {
        return kResultFalse;
    };
    *slot = Some(other.to_com_ptr());
    kResultOk
}

pub(super) fn disconnect_connection_point(
    slot: &SharedConnectionPoint,
    other: *mut IConnectionPoint,
) -> tresult {
    // SAFETY: This block isolates raw host pointers/COM calls inside the VST3 adapter boundary; callers uphold the enclosing unsafe callback contract and nullable pointers are checked before use.
    let Some(other) = (unsafe { ComRef::from_raw(other) }) else {
        return kInvalidArgument;
    };
    let Ok(mut slot) = slot.lock() else {
        return kResultFalse;
    };
    let Some(current) = slot.as_ref() else {
        return kResultFalse;
    };
    if current.as_ptr() != other.as_ptr() {
        return kResultFalse;
    }
    *slot = None;
    kResultOk
}
