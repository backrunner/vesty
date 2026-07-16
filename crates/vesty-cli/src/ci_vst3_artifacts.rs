use super::*;

pub(super) fn is_vst3_sdk_generated_bindings_scaffold_candidate(
    path: &Utf8Path,
    text: &str,
) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

pub(super) fn is_vst3_sdk_generated_bindings_abi_candidate(path: &Utf8Path, text: &str) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated-abi.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

pub(super) fn is_vst3_sdk_generated_bindings_abi_seed_candidate(
    path: &Utf8Path,
    text: &str,
) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated-abi-seed.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

pub(super) fn is_vst3_sdk_generated_bindings_interface_skeleton_candidate(
    path: &Utf8Path,
    text: &str,
) -> bool {
    if text.contains(vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR) {
        return true;
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("generated-interface-skeleton.rs"))
    {
        let lower = path.as_str().to_ascii_lowercase();
        return lower.contains("vst3-sdk")
            || lower.contains("vst3_sdk")
            || lower.contains("vesty-vst3-sdk");
    }
    false
}

pub(super) fn validate_vst3_sdk_generated_bindings_abi_text(text: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR) {
        errors.push(format!(
            "missing ABI layout generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_ABI_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"abi-layout\";") {
        errors.push("missing ABI layout status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors.push("ABI layout plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push("ABI layout surface status must be ready-for-binding-emitter".to_string());
    }
    if !text.contains("pub const ABI_LAYOUT_GENERATED: bool = true;") {
        errors.push("ABI layout must explicitly mark ABI_LAYOUT_GENERATED true".to_string());
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("ABI layout must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI layout must keep `BINDINGS_GENERATED` false".to_string());
    }
    if text.contains("FULL_COM_BINDINGS_GENERATED: bool = true") {
        errors.push("ABI layout must not claim full COM bindings are generated".to_string());
    }
    if !text.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI layout must keep `FULL_COM_BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push("ABI layout must be generated from a complete header manifest".to_string());
    }
    for required in [
        "#[repr(C)]",
        "pub struct TUID",
        "pub struct FUnknownVTable",
        "pub struct FUnknown",
        "pub struct ViewRect",
        "pub struct ProgramListInfo",
        "pub struct UnitInfo",
        "pub struct NoteExpressionValueDescription",
        "pub struct NoteExpressionTypeInfo",
        "pub struct PhysicalUIMap",
        "pub struct PhysicalUIMapList",
        "pub struct AbiLayoutRecord",
        "pub struct AbiFieldOffset",
        "pub type FUnknownQueryInterface = unsafe extern \"system\" fn(",
        "pub type ParamID = u32;",
        "pub type ParamValue = f64;",
        "pub type TChar = u16;",
        "pub type String128 = [TChar; STRING128_CODE_UNITS];",
        "pub type UnitID = int32;",
        "pub type ProgramListID = int32;",
        "pub type NoteExpressionTypeID = uint32;",
        "pub type NoteExpressionValue = f64;",
        "pub type PhysicalUITypeID = uint32;",
        "pub type Sample32 = f32;",
        "pub type Sample64 = f64;",
        "pub const ABI_LAYOUT_TYPES: &[&str] = &[",
        "pub const STRING128_CODE_UNITS: usize = 128;",
        "pub const PROGRAM_LIST_INFO_FIELD_COUNT: usize = 3;",
        "pub const UNIT_INFO_FIELD_COUNT: usize = 4;",
        "pub const NOTE_EXPRESSION_TYPE_INFO_FIELD_COUNT: usize = 8;",
        "pub const PHYSICAL_UI_MAP_FIELD_COUNT: usize = 2;",
        "pub const ABI_LAYOUT_RECORDS: &[AbiLayoutRecord] = &[",
        "pub const ABI_FIELD_OFFSETS: &[AbiFieldOffset] = &[",
        "type_name: \"ProgramListInfo\", size: std::mem::size_of::<ProgramListInfo>()",
        "type_name: \"NoteExpressionTypeInfo\", size: std::mem::size_of::<NoteExpressionTypeInfo>()",
        "owner: \"ProgramListInfo\", field: \"programCount\", offset: std::mem::offset_of!(ProgramListInfo, programCount)",
        "owner: \"NoteExpressionTypeInfo\", field: \"valueDesc\", offset: std::mem::offset_of!(NoteExpressionTypeInfo, valueDesc)",
        "owner: \"PhysicalUIMapList\", field: \"map\", offset: std::mem::offset_of!(PhysicalUIMapList, map)",
        "pub const kResultOk: TResult = 0;",
        "pub const kInvalidArgument: TResult = 2;",
        "pub const kNotImplemented: TResult = 3;",
        "pub const kRootUnitId: UnitID = 0;",
        "pub const kNoParentUnitId: UnitID = -1;",
        "pub const kNoProgramListId: ProgramListID = -1;",
        "pub const kPlatformTypeHWND: PlatformType = \"HWND\";",
        "pub const kPlatformTypeNSView: PlatformType = \"NSView\";",
        "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";",
    ] {
        if !text.contains(required) {
            errors.push(format!("missing ABI layout item `{required}`"));
        }
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    for symbol in [
        "FUnknown",
        "IPlugView",
        "IMidiMapping",
        "ProcessData",
        "UnitInfo",
        "ProgramListInfo",
        "NoteExpressionTypeInfo",
        "PhysicalUIMap",
    ] {
        if !text.contains(symbol) {
            errors.push(format!("missing binding surface symbol `{symbol}`"));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings ABI layout: {}",
            errors.join("; ")
        ))
    }
}

pub(super) fn validate_vst3_sdk_generated_bindings_abi_seed_text(text: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR) {
        errors.push(format!(
            "missing ABI seed generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_ABI_SEED_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"abi-seed\";") {
        errors.push("missing ABI seed status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors.push("ABI seed plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push("ABI seed surface status must be ready-for-binding-emitter".to_string());
    }
    if !text.contains("pub const ABI_SEED_GENERATED: bool = true;") {
        errors.push("ABI seed must explicitly mark ABI_SEED_GENERATED true".to_string());
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("ABI seed must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI seed must keep `BINDINGS_GENERATED` false".to_string());
    }
    if text.contains("FULL_COM_BINDINGS_GENERATED: bool = true") {
        errors.push("ABI seed must not claim full COM bindings are generated".to_string());
    }
    if !text.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;") {
        errors.push("ABI seed must keep `FULL_COM_BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push("ABI seed must be generated from a complete header manifest".to_string());
    }
    for required in [
        "pub type TResult = i32;",
        "pub type ParamID = u32;",
        "pub type ParamValue = f64;",
        "pub type TChar = u16;",
        "pub type TUID = [std::os::raw::c_char; 16];",
        "pub const kResultOk: TResult = 0;",
        "pub const kInvalidArgument: TResult = 2;",
        "pub const kNotImplemented: TResult = 3;",
        "pub const kPlatformTypeHWND: PlatformType = \"HWND\";",
        "pub const kPlatformTypeNSView: PlatformType = \"NSView\";",
        "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";",
    ] {
        if !text.contains(required) {
            errors.push(format!("missing ABI seed item `{required}`"));
        }
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    for symbol in ["IPlugView", "IMidiMapping"] {
        if !text.contains(symbol) {
            errors.push(format!("missing binding surface symbol `{symbol}`"));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings ABI seed: {}",
            errors.join("; ")
        ))
    }
}

pub(super) fn validate_vst3_sdk_generated_bindings_interface_skeleton_text(
    text: &str,
) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR) {
        errors.push(format!(
            "missing interface skeleton generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"interface-skeleton\";") {
        errors.push("missing interface skeleton status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors
            .push("interface skeleton plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push(
            "interface skeleton surface status must be ready-for-binding-emitter".to_string(),
        );
    }
    if !text.contains("pub const INTERFACE_SKELETON_GENERATED: bool = true;") {
        errors.push(
            "interface skeleton must explicitly mark INTERFACE_SKELETON_GENERATED true".to_string(),
        );
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("interface skeleton must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("interface skeleton must keep `BINDINGS_GENERATED` false".to_string());
    }
    if text.contains("FULL_COM_BINDINGS_GENERATED: bool = true") {
        errors
            .push("interface skeleton must not claim full COM bindings are generated".to_string());
    }
    if !text.contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;") {
        errors.push("interface skeleton must keep `FULL_COM_BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push(
            "interface skeleton must be generated from a complete header manifest".to_string(),
        );
    }
    for required in [
        "#[repr(C)]",
        "pub struct FUnknownVTable",
        "pub struct FUnknown",
        "pub struct IPlugViewVTable",
        "pub struct IPlugView",
        "pub struct IEditControllerVTable",
        "pub struct IMidiMappingVTable",
        "pub struct INoteExpressionControllerVTable",
        "pub struct INoteExpressionPhysicalUIMappingVTable",
        "pub struct IUnitInfoVTable",
        "pub struct IProgramListDataVTable",
        "pub struct InterfaceMethod",
        "pub struct InterfaceVTableSlot",
        "pub struct InterfaceCallbackType",
        "pub struct InterfaceVTableFieldOffset",
        "pub struct InterfaceId",
        "pub struct QueryInterfaceEntry",
        "pub struct ComObjectInterface",
        "pub struct ComObjectIdentityPlan",
        "pub struct ComObjectQueryInterfaceDispatchEntry",
        "pub struct FactoryExportPlan",
        "pub struct FactoryClassPlan",
        "pub struct ModuleExportPlan",
        "pub struct BinaryExportSymbolPlan",
        "pub struct BinaryExportInspectionToolPlan",
        "pub slot: usize,",
        "pub signature: &'static str,",
        "pub local_slot: usize,",
        "pub global_slot: usize,",
        "pub callback_type: &'static str,",
        "pub object: &'static str,",
        "pub iid_const: &'static str,",
        "pub uid_words: [u32; 4],",
        "pub inherits_funknown: bool,",
        "pub required: bool,",
        "pub root_interface: &'static str,",
        "pub root_iid_const: &'static str,",
        "pub funknown_identity: &'static str,",
        "pub unknown_iid_result: &'static str,",
        "pub returns_same_identity: bool,",
        "pub add_ref_on_success: bool,",
        "pub factory_object: &'static str,",
        "pub factory_interface: &'static str,",
        "pub factory_iid_const: &'static str,",
        "pub class_count: usize,",
        "pub class_kind: &'static str,",
        "pub class_index: usize,",
        "pub category: &'static str,",
        "pub name_source: &'static str,",
        "pub cid_source: &'static str,",
        "pub cid_policy: &'static str,",
        "pub create_instance_root_iid_const: &'static str,",
        "pub unknown_cid_result: &'static str,",
        "pub construction_failure_result: &'static str,",
        "pub requested_iid_dispatch: &'static str,",
        "pub symbol: &'static str,",
        "pub platforms: &'static str,",
        "pub generated_callable: bool,",
        "pub binary_format: &'static str,",
        "pub tool_symbol: &'static str,",
        "pub inspection_tool: &'static str,",
        "pub verified_by_generated_bindings: bool,",
        "pub program: &'static str,",
        "pub args: &'static [&'static str],",
        "pub const INTERFACE_METHOD_COUNT: usize = ",
        "pub const INTERFACE_VTABLE_SLOT_COUNT: usize = ",
        "pub const INTERFACE_VTABLE_FIELD_COUNT: usize = ",
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_COUNT: usize = ",
        "pub const INTERFACE_CALLBACK_TYPE_COUNT: usize = ",
        "pub const INTERFACE_ID_COUNT: usize = ",
        "pub const QUERY_INTERFACE_ENTRY_COUNT: usize = ",
        "pub const COM_OBJECT_COUNT: usize = ",
        "pub const COM_OBJECT_INTERFACE_COUNT: usize = ",
        "pub const COM_OBJECT_IDENTITY_PLAN_COUNT: usize = ",
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRY_COUNT: usize = ",
        "pub const FACTORY_EXPORT_PLAN_COUNT: usize = 1;",
        "pub const FACTORY_CLASS_PLAN_COUNT: usize = ",
        "pub const MODULE_EXPORT_PLAN_COUNT: usize = ",
        "pub const BINARY_EXPORT_SYMBOL_PLAN_COUNT: usize = ",
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = ",
        "pub const INTERFACE_METHOD_SLOT_SCOPE: &str = \"per-interface-order-audit\";",
        "pub const INTERFACE_METHOD_SIGNATURE_SCOPE: &str = \"signature-intent-audit\";",
        "pub const INTERFACE_VTABLE_SLOT_SCOPE: &str = \"per-interface-local-vtable-seed-audit\";",
        "pub const INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE: &str = \"com-vtable-global-slot-seed-audit\";",
        "pub const INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE: &str = \"pure-vtable-slot-lookup-seed-audit\";",
        "pub const INTERFACE_VTABLE_FIELD_SCOPE: &str = \"repr-c-vtable-callback-field-layout-seed-audit\";",
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_SCOPE: &str = \"repr-c-vtable-callback-field-offset-fingerprint-audit\";",
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_LOOKUP_SCOPE: &str = \"pure-vtable-field-offset-lookup-seed-audit\";",
        "pub const INTERFACE_CALLBACK_TYPE_SCOPE: &str = \"callback-type-alias-seed-audit\";",
        "pub const INTERFACE_ID_SCOPE: &str = \"upstream-vst3-interface-iid-audit\";",
        "pub const QUERY_INTERFACE_ENTRY_SCOPE: &str = \"query-interface-dispatch-plan-audit\";",
        "pub const QUERY_INTERFACE_IID_LOOKUP_SCOPE: &str = \"pure-iid-dispatch-lookup-seed-audit\";",
        "pub const COM_OBJECT_INTERFACE_SCOPE: &str = \"vesty-com-object-interface-exposure-plan-audit\";",
        "pub const COM_OBJECT_IDENTITY_PLAN_SCOPE: &str = \"vesty-com-object-funknown-identity-plan-audit\";",
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_SCOPE: &str = \"vesty-com-object-query-interface-dispatch-plan-audit\";",
        "pub const FACTORY_EXPORT_PLAN_SCOPE: &str = \"vesty-factory-export-plan-audit\";",
        "pub const FACTORY_CLASS_PLAN_SCOPE: &str = \"vesty-factory-class-plan-audit\";",
        "pub const MODULE_EXPORT_PLAN_SCOPE: &str = \"vesty-module-export-plan-audit\";",
        "pub const BINARY_EXPORT_SYMBOL_PLAN_SCOPE: &str = \"vesty-binary-export-symbol-plan-audit\";",
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE: &str = \"vesty-binary-export-inspection-tool-plan-audit\";",
        "pub const BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED: bool = true;",
        "pub const BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED: bool = false;",
        "pub const fn iid_from_words(a: u32, b: u32, c: u32, d: u32) -> TUID",
        "pub const FUNKNOWN_IID: TUID = iid_from_words(0x00000000, 0x00000000, 0xC0000000, 0x00000046);",
        "pub const IAUDIOPROCESSOR_IID: TUID = iid_from_words(0x42043F99, 0xB7DA453C, 0xA569E79D, 0x9AAEC33D);",
        "pub const IEDITCONTROLLER_IID: TUID = iid_from_words(0xDCD7BBE3, 0x7742448D, 0xA874AACC, 0x979C759E);",
        "pub const IMIDIMAPPING_IID: TUID = iid_from_words(0xDF0FF9F7, 0x49B74669, 0xB63AB732, 0x7ADBF5E5);",
        "pub const IUNITINFO_IID: TUID = iid_from_words(0x3D4BD6B5, 0x913A4FD2, 0xA886E768, 0xA5EB92C1);",
        "pub const IPROGRAMLISTDATA_IID: TUID = iid_from_words(0x8683B01F, 0x7B354F70, 0xA2651DEC, 0x353AF4FF);",
        "pub const INOTEEXPRESSIONCONTROLLER_IID: TUID = iid_from_words(0xB7F8F859, 0x41234872, 0x91169581, 0x4F3721A3);",
        "pub fn interface_id_for_iid(iid: &TUID) -> Option<&'static InterfaceId>",
        "pub fn query_interface_entry_by_interface(interface: &str) -> Option<&'static QueryInterfaceEntry>",
        "pub fn query_interface_entry_for_iid(iid: &TUID) -> Option<&'static QueryInterfaceEntry>",
        "pub const INTERFACE_IDS: &[InterfaceId] = &[",
        "pub const QUERY_INTERFACE_ENTRIES: &[QueryInterfaceEntry] = &[",
        "pub const COM_OBJECTS: &[&str] = &[",
        "pub const COM_OBJECT_INTERFACES: &[ComObjectInterface] = &[",
        "pub const COM_OBJECT_IDENTITY_PLANS: &[ComObjectIdentityPlan] = &[",
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES: &[ComObjectQueryInterfaceDispatchEntry] = &[",
        "pub fn com_object_query_interface_dispatch_by_interface(",
        "pub fn com_object_query_interface_dispatch_for_iid(",
        "pub const FACTORY_EXPORT_PLAN: FactoryExportPlan = ",
        "pub const VESTYPROCESSOR_FACTORY_CLASS_PLAN: FactoryClassPlan = ",
        "pub const VESTYCONTROLLER_FACTORY_CLASS_PLAN: FactoryClassPlan = ",
        "pub const FACTORY_CLASS_PLANS: &[FactoryClassPlan] = &[",
        "pub const GETPLUGINFACTORY_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const WINDOWS_INITDLL_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const MACOS_BUNDLEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const LINUX_MODULEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ",
        "pub const MODULE_EXPORT_PLANS: &[ModuleExportPlan] = &[",
        "pub const WINDOWS_GETPLUGINFACTORY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = ",
        "pub const MACOS_BUNDLEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = ",
        "pub const LINUX_MODULEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = ",
        "pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[",
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[",
        "pub fn binary_export_inspection_tools(",
        "pub fn binary_export_symbol_plan_by_platform_and_symbol(",
        "pub fn required_binary_export_symbol_count(platform: &str) -> usize",
        "pub fn first_missing_binary_export_symbol(",
        "pub fn binary_export_required_symbols_present(",
        "pub const VESTYPROCESSOR_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYCONTROLLER_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYPLUGVIEW_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYFACTORY_INTERFACES: &[ComObjectInterface] = &[",
        "pub const VESTYPROCESSOR_IDENTITY_PLAN: ComObjectIdentityPlan = ",
        "pub const VESTYPROCESSOR_QUERY_INTERFACE_DISPATCH: &[ComObjectQueryInterfaceDispatchEntry] = &[",
        "pub const INTERFACE_METHODS: &[InterfaceMethod] = &[",
        "pub const INTERFACE_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[",
        "pub fn interface_vtable_slot_by_interface_and_method(",
        "pub fn interface_vtable_slot_by_interface_and_global_slot(",
        "pub const INTERFACE_CALLBACK_TYPES: &[InterfaceCallbackType] = &[",
        "pub const INTERFACE_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[",
        "pub fn interface_vtable_field_offset_by_interface_and_field(",
        "pub const IAUDIOPROCESSOR_METHODS: &[InterfaceMethod] = &[",
        "pub const IAUDIOPROCESSOR_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[",
        "pub const IAUDIOPROCESSOR_CALLBACK_TYPES: &[InterfaceCallbackType] = &[",
        "pub const IAUDIOPROCESSOR_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[",
        "pub const IEDITCONTROLLER_METHODS: &[InterfaceMethod] = &[",
        "pub const IUNITINFO_METHODS: &[InterfaceMethod] = &[",
        "pub const IUNITINFO_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[",
        "pub const IUNITINFO_CALLBACK_TYPES: &[InterfaceCallbackType] = &[",
        "pub const IUNITINFO_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[",
        "pub const IPROGRAMLISTDATA_METHODS: &[InterfaceMethod] = &[",
        "pub const INOTEEXPRESSIONCONTROLLER_METHODS: &[InterfaceMethod] = &[",
        "pub type IAudioProcessorProcess = unsafe extern \"system\" fn(",
        "pub process: IAudioProcessorProcess,",
        "offset: std::mem::offset_of!(IAudioProcessorVTable, process)",
        "local_slot: 6, global_slot: 9, interface: \"IAudioProcessor\", method: \"process\", field: \"process\", callback_type: \"IAudioProcessorProcess\"",
        "pub type IUnitInfoGetProgramListInfo = unsafe extern \"system\" fn(",
        "pub getProgramListInfo: IUnitInfoGetProgramListInfo,",
        "offset: std::mem::offset_of!(IUnitInfoVTable, getProgramListInfo)",
        "local_slot: 3, global_slot: 6, interface: \"IUnitInfo\", method: \"getProgramListInfo\", field: \"getProgramListInfo\", callback_type: \"IUnitInfoGetProgramListInfo\"",
        "pub type IProgramListDataGetProgramData = unsafe extern \"system\" fn(",
        "pub getProgramData: IProgramListDataGetProgramData,",
        "offset: std::mem::offset_of!(IProgramListDataVTable, getProgramData)",
        "local_slot: 1, global_slot: 4, interface: \"IProgramListData\", method: \"getProgramData\", field: \"getProgramData\", callback_type: \"IProgramListDataGetProgramData\"",
        "pub type INoteExpressionControllerGetNoteExpressionInfo = unsafe extern \"system\" fn(",
        "pub getNoteExpressionInfo: INoteExpressionControllerGetNoteExpressionInfo,",
        "offset: std::mem::offset_of!(INoteExpressionControllerVTable, getNoteExpressionInfo)",
        "local_slot: 1, global_slot: 4, interface: \"INoteExpressionController\", method: \"getNoteExpressionInfo\", field: \"getNoteExpressionInfo\", callback_type: \"INoteExpressionControllerGetNoteExpressionInfo\"",
        "pub const INTERFACE_SKELETON_TYPES: &[&str] = &[",
        "pub const kResultOk: TResult = 0;",
        "pub const kInvalidArgument: TResult = 2;",
        "pub const kNotImplemented: TResult = 3;",
        "pub const kNoInterface: TResult",
    ] {
        if !text.contains(required) {
            errors.push(format!("missing interface skeleton item `{required}`"));
        }
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    if !text.contains(&format!(
        "pub const BINARY_EXPORT_SYMBOL_PLAN_COUNT: usize = {};",
        vesty_vst3_sys::binary_export_symbol_plans().len()
    )) {
        errors.push("binary export symbol plan count does not match vesty-vst3-sys".to_string());
    }
    if !text.contains(&format!(
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = {};",
        vesty_vst3_sys::binary_export_inspection_tool_plans().len()
    )) {
        errors.push(
            "binary export inspection tool plan count does not match vesty-vst3-sys".to_string(),
        );
    }
    if let Some(symbol_array) = rust_array_body(
        text,
        "pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[",
    ) {
        let actual_count = count_text_occurrences(symbol_array, "BinaryExportSymbolPlan {");
        let expected_count = vesty_vst3_sys::binary_export_symbol_plans().len();
        if actual_count != expected_count {
            errors.push(format!(
                "binary export symbol plan array contains {actual_count} record(s), expected {expected_count}"
            ));
        }
        for plan in vesty_vst3_sys::binary_export_symbol_plans() {
            let expected = format!(
                "BinaryExportSymbolPlan {{ platform: {}, binary_format: {}, symbol: {}, tool_symbol: {}, inspection_tool: {}",
                rust_string_literal(plan.platform),
                rust_string_literal(plan.binary_format),
                rust_string_literal(plan.symbol),
                rust_string_literal(plan.tool_symbol),
                rust_string_literal(plan.inspection_tool),
            );
            match count_text_occurrences(symbol_array, &expected) {
                1 => {}
                0 => errors.push(format!(
                    "missing vesty-vst3-sys binary export symbol plan `{}/{}`",
                    plan.platform, plan.tool_symbol
                )),
                count => errors.push(format!(
                    "vesty-vst3-sys binary export symbol plan `{}/{}` appears {count} time(s), expected exactly once",
                    plan.platform, plan.tool_symbol
                )),
            }
        }
    } else {
        errors.push("missing binary export symbol plan array body".to_string());
    }
    if let Some(tool_array) = rust_array_body(
        text,
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[",
    ) {
        let actual_count = count_text_occurrences(tool_array, "BinaryExportInspectionToolPlan {");
        let expected_count = vesty_vst3_sys::binary_export_inspection_tool_plans().len();
        if actual_count != expected_count {
            errors.push(format!(
                "binary export inspection tool plan array contains {actual_count} record(s), expected {expected_count}"
            ));
        }
        for tool in vesty_vst3_sys::binary_export_inspection_tool_plans() {
            let args = tool
                .args
                .iter()
                .map(|arg| rust_string_literal(arg))
                .collect::<Vec<_>>()
                .join(", ");
            let expected = format!(
                "BinaryExportInspectionToolPlan {{ platform: {}, program: {}, args: &[{}] }}",
                rust_string_literal(tool.platform),
                rust_string_literal(tool.program),
                args
            );
            match count_text_occurrences(tool_array, &expected) {
                1 => {}
                0 => errors.push(format!(
                    "missing vesty-vst3-sys binary export inspection tool plan `{}/{}`",
                    tool.platform, tool.program
                )),
                count => errors.push(format!(
                    "vesty-vst3-sys binary export inspection tool plan `{}/{}` appears {count} time(s), expected exactly once",
                    tool.platform, tool.program
                )),
            }
        }
    } else {
        errors.push("missing binary export inspection tool plan array body".to_string());
    }
    for symbol in [
        "FUnknown",
        "IPlugView",
        "IEditController",
        "IMidiMapping",
        "INoteExpressionController",
        "IUnitInfo",
        "IProgramListData",
    ] {
        if !text.contains(symbol) {
            errors.push(format!("missing interface skeleton symbol `{symbol}`"));
        }
    }
    for required_metadata in [
        "slot: 0, interface: \"IAudioProcessor\", name: \"setBusArrangements\"",
        "slot: 6, interface: \"IAudioProcessor\", name: \"process\"",
        "local_slot: 6, global_slot: 9, interface: \"IAudioProcessor\", method: \"process\", field: \"process\", callback_type: \"IAudioProcessorProcess\"",
        "local_slot: 3, global_slot: 6, interface: \"IUnitInfo\", method: \"getProgramListInfo\", field: \"getProgramListInfo\", callback_type: \"IUnitInfoGetProgramListInfo\"",
        "local_slot: 1, global_slot: 4, interface: \"IProgramListData\", method: \"getProgramData\", field: \"getProgramData\", callback_type: \"IProgramListDataGetProgramData\"",
        "local_slot: 1, global_slot: 4, interface: \"INoteExpressionController\", method: \"getNoteExpressionInfo\", field: \"getNoteExpressionInfo\", callback_type: \"INoteExpressionControllerGetNoteExpressionInfo\"",
        "name: \"process\", purpose: \"realtime audio/MIDI/process callback\", realtime: true, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\"",
        "name: \"getParameterInfo\", purpose: \"return host parameter metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IEditController, param_index: int32, info: *mut ParameterInfo) -> TResult\"",
        "name: \"getMidiControllerAssignment\", purpose: \"map MIDI controller to host parameter id\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IMidiMapping, bus_index: int32, channel: int16, midi_controller_number: CtrlNumber, id: *mut ParamID) -> TResult\"",
        "name: \"getNoteExpressionInfo\", purpose: \"return Note Expression type metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\"",
        "name: \"getPhysicalUIMapping\", purpose: \"return physical UI mapping for Note Expression\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionPhysicalUIMapping, bus_index: int32, channel: int16, list: *mut PhysicalUIMapList) -> TResult\"",
        "name: \"getProgramListInfo\", purpose: \"return program-list metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\"",
        "name: \"getProgramData\", purpose: \"save program data to stream\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\"",
        "InterfaceId { interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", uid_words: [0x42043F99, 0xB7DA453C, 0xA569E79D, 0x9AAEC33D], source: \"upstream-vst3-0.3.0/src/bindings.rs\" }",
        "QueryInterfaceEntry { interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", inherits_funknown: true, implementation: \"planned-dispatch-entry-no-callable-glue\" }",
        "InterfaceId { interface: \"IUnitInfo\", iid_const: \"IUNITINFO_IID\", uid_words: [0x3D4BD6B5, 0x913A4FD2, 0xA886E768, 0xA5EB92C1], source: \"upstream-vst3-0.3.0/src/bindings.rs\" }",
        "QueryInterfaceEntry { interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", inherits_funknown: true, implementation: \"planned-dispatch-entry-no-callable-glue\" }",
        "ComObjectInterface { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyProcessor\", interface: \"IProcessContextRequirements\", iid_const: \"IPROCESSCONTEXTREQUIREMENTS_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"IEditController\", iid_const: \"IEDITCONTROLLER_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"IUnitInfo\", iid_const: \"IUNITINFO_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyController\", interface: \"INoteExpressionController\", iid_const: \"INOTEEXPRESSIONCONTROLLER_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyPlugView\", interface: \"IPlugView\", iid_const: \"IPLUGVIEW_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectInterface { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }",
        "ComObjectIdentityPlan { object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }",
        "ComObjectIdentityPlan { object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"FUnknown\", iid_const: \"FUNKNOWN_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyController\", interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", root_interface: \"IEditController\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "ComObjectQueryInterfaceDispatchEntry { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", root_interface: \"IPluginFactory\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }",
        "FactoryExportPlan { factory_object: \"VestyFactory\", factory_interface: \"IPluginFactory\", factory_iid_const: \"IPLUGINFACTORY_IID\", class_count: 2, count_classes_result: \"2\", get_factory_info_source: \"PluginInfo vendor/url/email + kUnicode\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }",
        "FactoryClassPlan { class_kind: \"processor\", class_index: 0, class_object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", category: \"Audio Module Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id\", cid_policy: \"processor-cid-is-plugin-class-id\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyProcessor\", create_instance_root_interface: \"IComponent\", create_instance_root_iid_const: \"ICOMPONENT_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }",
        "FactoryClassPlan { class_kind: \"controller\", class_index: 1, class_object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", category: \"Component Controller Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id[15].wrapping_add(1)\", cid_policy: \"controller-cid-last-byte-wrapping-add-1\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyController\", create_instance_root_interface: \"IEditController\", create_instance_root_iid_const: \"IEDITCONTROLLER_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }",
        "ModuleExportPlan { symbol: \"GetPluginFactory\", platforms: \"windows,macos,linux\", signature: \"extern \\\"system\\\" fn() -> *mut IPluginFactory\", purpose: \"return VST3 plugin factory pointer\", implementation: \"vesty_vst3::create_plugin_factory::<Plugin>()\", return_policy: \"returns owned COM factory pointer for host discovery\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"InitDll\", platforms: \"windows\", signature: \"extern \\\"system\\\" fn() -> bool\", purpose: \"Windows VST3 module initialization entry\", implementation: \"return true\", return_policy: \"host may continue loading the module\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"bundleEntry\", platforms: \"macos\", signature: \"extern \\\"system\\\" fn(bundle_ref: *mut c_void) -> bool\", purpose: \"macOS VST3 bundle initialization entry\", implementation: \"vesty_vst3::set_macos_bundle_ref(bundle_ref); return true\", return_policy: \"bundle resources path is captured when possible\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"BundleEntry\", platforms: \"macos\", signature: \"extern \\\"system\\\" fn(bundle_ref: *mut c_void) -> bool\", purpose: \"macOS compatibility initialization alias\", implementation: \"delegate to bundleEntry(bundle_ref)\", return_policy: \"keeps uppercase host lookup compatibility\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "ModuleExportPlan { symbol: \"ModuleEntry\", platforms: \"linux\", signature: \"extern \\\"system\\\" fn(library_handle: *mut c_void) -> bool\", purpose: \"Linux VST3 module initialization entry\", implementation: \"return true\", return_policy: \"host may continue loading the module\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "BinaryExportSymbolPlan { platform: \"windows-x64\", binary_format: \"PE/COFF\", symbol: \"GetPluginFactory\", tool_symbol: \"GetPluginFactory\", inspection_tool: \"dumpbin /exports or llvm-objdump -p\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "BinaryExportSymbolPlan { platform: \"macos\", binary_format: \"Mach-O\", symbol: \"bundleEntry\", tool_symbol: \"_bundleEntry\", inspection_tool: \"nm -gU or llvm-nm -gU\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
        "BinaryExportSymbolPlan { platform: \"linux-x64\", binary_format: \"ELF\", symbol: \"ModuleEntry\", tool_symbol: \"ModuleEntry\", inspection_tool: \"nm -D --defined-only or llvm-nm -D --defined-only\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }",
    ] {
        if !text.contains(required_metadata) {
            errors.push(format!(
                "missing interface skeleton metadata `{required_metadata}`"
            ));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings interface skeleton: {}",
            errors.join("; ")
        ))
    }
}

pub(super) fn rust_array_body<'a>(text: &'a str, declaration: &str) -> Option<&'a str> {
    let after_declaration = text.get(text.find(declaration)? + declaration.len()..)?;
    let end = after_declaration.find("\n];")?;
    Some(&after_declaration[..end])
}

pub(super) fn count_text_occurrences(text: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    text.match_indices(needle).count()
}

pub(super) fn validate_vst3_sdk_generated_bindings_scaffold_text(text: &str) -> Result<(), String> {
    let mut errors = Vec::new();
    if !text.contains(vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR) {
        errors.push(format!(
            "missing scaffold generator `{}`",
            vesty_vst3_sys::GENERATED_BINDINGS_SCAFFOLD_GENERATOR
        ));
    }
    if !text.contains("pub const STATUS: &str = \"metadata-scaffold\";") {
        errors.push("missing metadata scaffold status".to_string());
    }
    if !text.contains(&format!(
        "pub const PLAN_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_PLAN_GENERATOR
    )) {
        errors.push("missing generated-bindings plan generator marker".to_string());
    }
    if !text.contains("pub const PLAN_STATUS: &str = \"ready-for-binding-generator\";") {
        errors.push("scaffold plan status must be ready-for-binding-generator".to_string());
    }
    if !text.contains(&format!(
        "pub const SURFACE_GENERATOR: &str = \"{}\";",
        vesty_vst3_sys::GENERATED_BINDINGS_SURFACE_GENERATOR
    )) {
        errors.push("missing generated-bindings surface generator marker".to_string());
    }
    if !text.contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";") {
        errors.push("scaffold surface status must be ready-for-binding-emitter".to_string());
    }
    if text.contains("BINDINGS_GENERATED: bool = true") {
        errors.push("scaffold must not claim SDK bindings are generated".to_string());
    }
    if !text.contains("pub const BINDINGS_GENERATED: bool = false;") {
        errors.push("scaffold must keep `BINDINGS_GENERATED` false".to_string());
    }
    if !text.contains(&format!(
        "pub const STEINBERG_VST3_SDK_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::STEINBERG_VST3_SDK_BASELINE
    )) {
        errors.push("missing Steinberg SDK baseline".to_string());
    }
    if !text.contains(&format!(
        "pub const UPSTREAM_VST3_CRATE_BASELINE: &str = \"{}\";",
        vesty_vst3_sys::UPSTREAM_VST3_CRATE_BASELINE
    )) {
        errors.push("missing upstream vst3 crate baseline".to_string());
    }
    if !text.contains("pub const MISSING_HEADER_COUNT: usize = 0;") {
        errors.push(
            "metadata scaffold must be generated from a complete header manifest".to_string(),
        );
    }
    if !text.contains("pub const SURFACE_SYMBOL_COUNT: usize = ") {
        errors.push("missing surface symbol count".to_string());
    }
    if !text.contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol]") {
        errors.push("missing binding surface symbol list".to_string());
    }
    for symbol in ["IPlugView", "IMidiMapping"] {
        if !text.contains(symbol) {
            errors.push(format!("missing binding surface symbol `{symbol}`"));
        }
    }
    for header in vesty_vst3_sys::REQUIRED_GENERATED_HEADER_INPUTS {
        if !text.contains(header) {
            errors.push(format!("missing required header input `{header}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "invalid VST3 SDK generated bindings scaffold: {}",
            errors.join("; ")
        ))
    }
}
