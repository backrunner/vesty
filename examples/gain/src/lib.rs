use vesty::prelude::*;

#[derive(Params)]
pub struct GainParams {
    pub gain: FloatParam,
    pub bypass: BoolParam,
}

impl Default for GainParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("gain", "Gain", -60.0, 12.0, 0.0).with_unit("dB"),
            bypass: BoolParam::bypass("bypass", "Bypass", false),
        }
    }
}

#[derive(Default)]
pub struct GainPlugin {
    params: GainParams,
}

impl Plugin for GainPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Vesty Gain",
        vendor: "Vesty",
        url: "https://github.com/backrunner/vesty",
        email: "",
        version: "0.1.0",
        class_id: *b"VESTYGAIN0000001",
        kind: PluginKind::AudioEffect,
    };

    type Params = GainParams;
    type Kernel = GainKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        GainKernel {
            gain: self.params.resolve_or_invalid("gain"),
            bypass: self.params.resolve_or_invalid("bypass"),
        }
    }
}

pub struct GainKernel {
    gain: ParamHandle,
    bypass: ParamHandle,
}

impl AudioKernel for GainKernel {
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        let bypass = context.param_normalized(self.bypass).unwrap_or(0.0) >= 0.5;
        let initial_gain = context.param_normalized(self.gain).unwrap_or(0.833_333);
        let frames = context.audio().frames().min(u32::MAX as usize) as u32;
        let channels = context
            .audio()
            .input_channels()
            .min(context.audio().output_channels());
        let (audio, events) = context.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.gain, initial_gain, frames) {
            let gain_db = -60.0 + segment.normalized * 72.0;
            let gain = if bypass {
                1.0
            } else {
                10.0_f32.powf(gain_db as f32 / 20.0)
            };
            for channel in 0..channels {
                audio.copy_input_to_output_range(
                    channel,
                    segment.start_sample as usize,
                    segment.end_sample as usize,
                    gain,
                );
            }
        }
        ProcessResult::Continue
    }
}

vesty::export_vst3!(GainPlugin);
