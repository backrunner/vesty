use vesty_params::ParamCollection;

use crate::{
    AudioOutputBus, DEFAULT_AUDIO_OUTPUT_BUSES, HostChangeFlags, KernelInit, PluginInfo,
    NoteExpressionPhysicalUiMapping, NoteExpressionValueType, PrepareContext, ProcessContext,
    ProcessContext64, ProcessResult, ProgramAttribute, ProgramList, ProgramPitchName, StateError,
    UiDescriptor,
};

pub trait AudioKernel: Send + 'static {
    const SUPPORTS_F64: bool = false;

    fn prepare(&mut self, _context: PrepareContext) {}
    fn reset(&mut self) {}
    fn suspend(&mut self) {}
    fn resume(&mut self) {}
    fn process(&mut self, context: &mut ProcessContext<'_>) -> ProcessResult;
    fn process_f64(&mut self, context: &mut ProcessContext64<'_>) -> ProcessResult {
        let _ = context;
        ProcessResult::Silence
    }
}

pub use AudioKernel as InstrumentKernel;

pub trait Plugin: Send + Sync + 'static {
    const INFO: PluginInfo;

    type Params: ParamCollection + Send + Sync + 'static;
    type Kernel: AudioKernel;

    fn params(&self) -> &Self::Params;
    fn create_kernel(&self, init: KernelInit) -> Self::Kernel;

    fn ui(&self) -> Option<UiDescriptor> {
        None
    }

    fn latency_samples(&self) -> u32 {
        0
    }

    fn tail_samples(&self) -> u32 {
        0
    }

    fn sidechain_inputs(&self) -> u32 {
        0
    }

    fn output_buses(&self) -> &'static [AudioOutputBus] {
        &DEFAULT_AUDIO_OUTPUT_BUSES
    }

    fn program_lists(&self) -> &'static [ProgramList] {
        &[]
    }

    fn apply_program(&self, list_id: u32, program_index: usize) -> Result<bool, StateError> {
        let _ = (list_id, program_index);
        Ok(false)
    }

    fn program_data_supported(&self, list_id: u32) -> bool {
        let _ = list_id;
        false
    }

    fn save_program_data(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> Result<Option<serde_json::Value>, StateError> {
        let _ = (list_id, program_index);
        Ok(None)
    }

    fn load_program_data(
        &self,
        list_id: u32,
        program_index: usize,
        data: serde_json::Value,
    ) -> Result<bool, StateError> {
        let _ = (list_id, program_index, data);
        Ok(false)
    }

    fn program_attributes(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramAttribute] {
        let _ = (list_id, program_index);
        &[]
    }

    fn program_pitch_names(
        &self,
        list_id: u32,
        program_index: usize,
    ) -> &'static [ProgramPitchName] {
        let _ = (list_id, program_index);
        &[]
    }

    fn note_expression_value_types(&self) -> &'static [NoteExpressionValueType] {
        &[]
    }

    fn note_expression_physical_ui_mappings(&self) -> &'static [NoteExpressionPhysicalUiMapping] {
        &[]
    }

    fn host_changes_for_param(
        &self,
        _id: &str,
        _old_normalized: f64,
        _new_normalized: f64,
    ) -> HostChangeFlags {
        HostChangeFlags::NONE
    }

    fn save_custom_state(&self) -> Result<Option<serde_json::Value>, StateError> {
        Ok(None)
    }

    fn load_custom_state(&self, state: Option<serde_json::Value>) -> Result<(), StateError> {
        let _ = state;
        Ok(())
    }
}
