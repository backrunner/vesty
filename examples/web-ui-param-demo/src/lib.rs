use vesty::prelude::*;

#[derive(Params)]
pub struct WebParams {
    pub mix: FloatParam,
}

impl Default for WebParams {
    fn default() -> Self {
        Self {
            mix: FloatParam::new("mix", "Mix", 0.0, 100.0, 50.0).with_unit("%"),
        }
    }
}

#[derive(Default)]
pub struct WebPlugin {
    params: WebParams,
}

impl Plugin for WebPlugin {
    const INFO: PluginInfo = PluginInfo {
        name: "Vesty Web UI Demo",
        vendor: "Vesty",
        url: "https://github.com/orchiliao/vesty",
        email: "",
        version: "0.1.0",
        class_id: *b"VESTYWEBUI00001X",
        kind: PluginKind::AudioEffect,
    };

    type Params = WebParams;
    type Kernel = WebKernel;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn create_kernel(&self, _init: KernelInit) -> Self::Kernel {
        WebKernel {
            mix: self.params.resolve_or_invalid("mix"),
        }
    }

    fn ui(&self) -> Option<UiDescriptor> {
        Some(UiDescriptor::web_assets("ui").with_dev_url("http://localhost:5173"))
    }
}

pub struct WebKernel {
    mix: ParamHandle,
}

impl AudioKernel for WebKernel {
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult {
        let initial_mix = context.param_normalized(self.mix).unwrap_or(0.5);
        let frames = context.audio().frames().min(u32::MAX as usize) as u32;
        let channels = context
            .audio()
            .input_channels()
            .min(context.audio().output_channels());
        let (audio, events) = context.audio_mut_and_events();

        for segment in ParamAutomationSegments::new(events, self.mix, initial_mix, frames) {
            let mix = segment.normalized as f32;
            for channel in 0..channels {
                audio.copy_input_to_output_range(
                    channel,
                    segment.start_sample as usize,
                    segment.end_sample as usize,
                    mix,
                );
            }
        }
        let sample_offset = context.audio().frames().saturating_sub(1) as u32;
        let _ = context.emit_output_meter(0, sample_offset);
        ProcessResult::Continue
    }
}

vesty::export_vst3!(WebPlugin);
