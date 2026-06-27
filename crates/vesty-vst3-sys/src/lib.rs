//! Binding selection metadata for Vesty's VST3 adapter.
//!
//! The MVP uses the upstream `vst3` crate as the active Rust binding source.
//! This crate reserves the `vesty-vst3-sys` layer required by the architecture so
//! Vesty can add generated bindings from Steinberg headers without changing the
//! safe `vesty-vst3` API surface.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const STEINBERG_VST3_SDK_BASELINE: &str = "v3.8.0_build_66";
pub const UPSTREAM_VST3_CRATE_BASELINE: &str = "0.3.0";
pub const VST3_SDK_DIR_ENV: &str = "VESTY_VST3_SDK_DIR";
pub const SDK_HEADER_MANIFEST_VERSION: u32 = 1;
pub const SDK_HEADER_MANIFEST_GENERATOR: &str = "vesty-vst3-sys.sdk-header-input-manifest.v1";
pub const GENERATED_BINDINGS_PLAN_VERSION: u32 = 1;
pub const GENERATED_BINDINGS_PLAN_GENERATOR: &str = "vesty-vst3-sys.generated-bindings-plan.v1";
pub const GENERATED_BINDINGS_SURFACE_VERSION: u32 = 2;
pub const GENERATED_BINDINGS_SURFACE_GENERATOR: &str =
    "vesty-vst3-sys.generated-bindings-surface.v1";
pub const GENERATED_BINDINGS_SCAFFOLD_GENERATOR: &str =
    "vesty-vst3-sys.generated-bindings-scaffold.v1";
pub const GENERATED_BINDINGS_ABI_SEED_GENERATOR: &str =
    "vesty-vst3-sys.generated-bindings-abi-seed.v1";
pub const GENERATED_BINDINGS_ABI_GENERATOR: &str = "vesty-vst3-sys.generated-bindings-abi.v1";
pub const GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR: &str =
    "vesty-vst3-sys.generated-bindings-interface-skeleton.v1";
pub const REQUIRED_GENERATED_HEADER_INPUTS: &[&str] = &[
    "pluginterfaces/base/ibstream.h",
    "pluginterfaces/base/fplatform.h",
    "pluginterfaces/base/fstrdefs.h",
    "pluginterfaces/base/funknown.h",
    "pluginterfaces/base/ipluginbase.h",
    "pluginterfaces/gui/iplugview.h",
    "pluginterfaces/vst/ivstaudioprocessor.h",
    "pluginterfaces/vst/ivstcomponent.h",
    "pluginterfaces/vst/ivsteditcontroller.h",
    "pluginterfaces/vst/ivstevents.h",
    "pluginterfaces/vst/ivstmessage.h",
    "pluginterfaces/vst/ivstmidicontrollers.h",
    "pluginterfaces/vst/ivstnoteexpression.h",
    "pluginterfaces/vst/ivstparameterchanges.h",
    "pluginterfaces/vst/ivstprocesscontext.h",
    "pluginterfaces/vst/ivstunits.h",
    "pluginterfaces/vst/vsttypes.h",
];

const SDK_VERSION_HINT_FILES: &[&str] = &["README.md", "CMakeLists.txt", "VERSION"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingSurfaceSymbolSpec {
    name: &'static str,
    kind: &'static str,
    header: &'static str,
    purpose: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsSurfaceSymbolSpec {
    pub name: &'static str,
    pub kind: &'static str,
    pub header: &'static str,
    pub purpose: &'static str,
}

impl BindingSurfaceSymbolSpec {
    const fn interface(name: &'static str, header: &'static str, purpose: &'static str) -> Self {
        Self {
            name,
            kind: "interface",
            header,
            purpose,
        }
    }

    const fn type_(name: &'static str, header: &'static str, purpose: &'static str) -> Self {
        Self {
            name,
            kind: "type",
            header,
            purpose,
        }
    }

    const fn constant(name: &'static str, header: &'static str, purpose: &'static str) -> Self {
        Self {
            name,
            kind: "constant",
            header,
            purpose,
        }
    }

    const fn public(self) -> GeneratedBindingsSurfaceSymbolSpec {
        GeneratedBindingsSurfaceSymbolSpec {
            name: self.name,
            kind: self.kind,
            header: self.header,
            purpose: self.purpose,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingInterfaceMethodSpec {
    interface: &'static str,
    name: &'static str,
    purpose: &'static str,
    realtime: bool,
}

impl BindingInterfaceMethodSpec {
    const fn new(
        interface: &'static str,
        name: &'static str,
        purpose: &'static str,
        realtime: bool,
    ) -> Self {
        Self {
            interface,
            name,
            purpose,
            realtime,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingInterfaceIdSpec {
    interface: &'static str,
    uid_words: [u32; 4],
    source: &'static str,
}

impl BindingInterfaceIdSpec {
    const fn new(interface: &'static str, uid_words: [u32; 4]) -> Self {
        Self {
            interface,
            uid_words,
            source: "upstream-vst3-0.3.0/src/bindings.rs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingComObjectInterfaceSpec {
    object: &'static str,
    interface: &'static str,
    exposure: &'static str,
    source: &'static str,
    required: bool,
}

impl BindingComObjectInterfaceSpec {
    const fn new(object: &'static str, interface: &'static str) -> Self {
        Self {
            object,
            interface,
            exposure: "implemented-by-current-vesty-vst3-adapter",
            source: "crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces",
            required: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingComObjectIdentitySpec {
    object: &'static str,
    root_interface: &'static str,
    funknown_identity: &'static str,
    refcount_policy: &'static str,
    unknown_iid_result: &'static str,
    null_object_pointer_result: &'static str,
    source: &'static str,
}

impl BindingComObjectIdentitySpec {
    const fn new(object: &'static str, root_interface: &'static str) -> Self {
        Self {
            object,
            root_interface,
            funknown_identity: "single-controlling-funknown-per-com-object",
            refcount_policy: "query-interface-success-addref-release-decrements-wrapper",
            unknown_iid_result: "kNoInterface",
            null_object_pointer_result: "kInvalidArgument",
            source: "crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingFactoryExportPlanSpec {
    factory_object: &'static str,
    factory_interface: &'static str,
    class_count: usize,
    count_classes_result: &'static str,
    get_factory_info_source: &'static str,
    source: &'static str,
}

impl BindingFactoryExportPlanSpec {
    const fn new() -> Self {
        Self {
            factory_object: "VestyFactory",
            factory_interface: "IPluginFactory",
            class_count: 2,
            count_classes_result: "2",
            get_factory_info_source: "PluginInfo vendor/url/email + kUnicode",
            source: "crates/vesty-vst3/src/bindings_impl.rs::VestyFactory",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingFactoryClassPlanSpec {
    class_kind: &'static str,
    class_index: usize,
    class_object: &'static str,
    root_interface: &'static str,
    category: &'static str,
    name_source: &'static str,
    cid_source: &'static str,
    cid_policy: &'static str,
    cardinality: &'static str,
    get_class_info_result: &'static str,
    invalid_class_index_result: &'static str,
    create_instance_object: &'static str,
    create_instance_root_interface: &'static str,
    unknown_cid_result: &'static str,
    construction_failure_result: &'static str,
    requested_iid_dispatch: &'static str,
    source: &'static str,
}

impl BindingFactoryClassPlanSpec {
    const fn processor() -> Self {
        Self {
            class_kind: "processor",
            class_index: 0,
            class_object: "VestyProcessor",
            root_interface: "IComponent",
            category: "Audio Module Class",
            name_source: "PluginInfo::name",
            cid_source: "PluginInfo::class_id",
            cid_policy: "processor-cid-is-plugin-class-id",
            cardinality: "kManyInstances",
            get_class_info_result: "kResultOk",
            invalid_class_index_result: "kInvalidArgument",
            create_instance_object: "VestyProcessor",
            create_instance_root_interface: "IComponent",
            unknown_cid_result: "kInvalidArgument",
            construction_failure_result: "kResultFalse",
            requested_iid_dispatch: "delegate-to-created-instance-queryInterface",
            source: "crates/vesty-vst3/src/bindings_impl.rs::VestyFactory",
        }
    }

    const fn controller() -> Self {
        Self {
            class_kind: "controller",
            class_index: 1,
            class_object: "VestyController",
            root_interface: "IEditController",
            category: "Component Controller Class",
            name_source: "PluginInfo::name",
            cid_source: "PluginInfo::class_id[15].wrapping_add(1)",
            cid_policy: "controller-cid-last-byte-wrapping-add-1",
            cardinality: "kManyInstances",
            get_class_info_result: "kResultOk",
            invalid_class_index_result: "kInvalidArgument",
            create_instance_object: "VestyController",
            create_instance_root_interface: "IEditController",
            unknown_cid_result: "kInvalidArgument",
            construction_failure_result: "kResultFalse",
            requested_iid_dispatch: "delegate-to-created-instance-queryInterface",
            source: "crates/vesty-vst3/src/bindings_impl.rs::VestyFactory",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingModuleExportPlanSpec {
    const_name: &'static str,
    symbol: &'static str,
    platforms: &'static str,
    signature: &'static str,
    purpose: &'static str,
    implementation: &'static str,
    return_policy: &'static str,
    generated_callable: bool,
    source: &'static str,
}

impl BindingModuleExportPlanSpec {
    const fn new(
        const_name: &'static str,
        symbol: &'static str,
        platforms: &'static str,
        signature: &'static str,
        purpose: &'static str,
        implementation: &'static str,
        return_policy: &'static str,
    ) -> Self {
        Self {
            const_name,
            symbol,
            platforms,
            signature,
            purpose,
            implementation,
            return_policy,
            generated_callable: false,
            source: "crates/vesty/src/lib.rs::export_vst3!",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BindingBinaryExportSymbolPlanSpec {
    const_name: &'static str,
    platform: &'static str,
    binary_format: &'static str,
    symbol: &'static str,
    tool_symbol: &'static str,
    inspection_tool: &'static str,
    required: bool,
    verified_by_generated_bindings: bool,
    source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryExportSymbolPlan {
    pub platform: &'static str,
    pub binary_format: &'static str,
    pub symbol: &'static str,
    pub tool_symbol: &'static str,
    pub inspection_tool: &'static str,
    pub required: bool,
    pub verified_by_generated_bindings: bool,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryExportInspectionToolPlan {
    pub platform: &'static str,
    pub program: &'static str,
    pub args: &'static [&'static str],
}

impl BinaryExportInspectionToolPlan {
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.to_string()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

impl BindingBinaryExportSymbolPlanSpec {
    const fn new(
        const_name: &'static str,
        platform: &'static str,
        binary_format: &'static str,
        symbol: &'static str,
        tool_symbol: &'static str,
        inspection_tool: &'static str,
    ) -> Self {
        Self {
            const_name,
            platform,
            binary_format,
            symbol,
            tool_symbol,
            inspection_tool,
            required: true,
            verified_by_generated_bindings: false,
            source: "crates/vesty/src/lib.rs::export_vst3!",
        }
    }

    const fn public(self) -> BinaryExportSymbolPlan {
        BinaryExportSymbolPlan {
            platform: self.platform,
            binary_format: self.binary_format,
            symbol: self.symbol,
            tool_symbol: self.tool_symbol,
            inspection_tool: self.inspection_tool,
            required: self.required,
            verified_by_generated_bindings: self.verified_by_generated_bindings,
            source: self.source,
        }
    }
}

const GENERATED_BINDINGS_SURFACE_SYMBOLS: &[BindingSurfaceSymbolSpec] = &[
    BindingSurfaceSymbolSpec::interface(
        "FUnknown",
        "pluginterfaces/base/funknown.h",
        "core COM identity and queryInterface base",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IPluginFactory",
        "pluginterfaces/base/ipluginbase.h",
        "plugin factory export and class discovery",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IPluginBase",
        "pluginterfaces/base/ipluginbase.h",
        "shared initialize/terminate lifecycle",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IConnectionPoint",
        "pluginterfaces/vst/ivstmessage.h",
        "processor/controller telemetry connection",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IBStream",
        "pluginterfaces/base/ibstream.h",
        "component and controller state streams",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IPlugView",
        "pluginterfaces/gui/iplugview.h",
        "system WebView editor attach, detach and resize",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IPlugFrame",
        "pluginterfaces/gui/iplugview.h",
        "host editor frame resize negotiation",
    ),
    BindingSurfaceSymbolSpec::type_(
        "ViewRect",
        "pluginterfaces/gui/iplugview.h",
        "editor bounds and size constraints",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IComponent",
        "pluginterfaces/vst/ivstcomponent.h",
        "VST3 component bus, state and controller class API",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IAudioProcessor",
        "pluginterfaces/vst/ivstaudioprocessor.h",
        "realtime process callback and bus arrangement API",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IProcessContextRequirements",
        "pluginterfaces/vst/ivstaudioprocessor.h",
        "transport/process-context requirement declaration",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IEditController",
        "pluginterfaces/vst/ivsteditcontroller.h",
        "host-visible parameter controller API",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IComponentHandler",
        "pluginterfaces/vst/ivsteditcontroller.h",
        "host parameter gesture and restart callback API",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IMidiMapping",
        "pluginterfaces/vst/ivstmidicontrollers.h",
        "host MIDI CC to parameter assignment API",
    ),
    BindingSurfaceSymbolSpec::interface(
        "INoteExpressionController",
        "pluginterfaces/vst/ivstnoteexpression.h",
        "host Note Expression metadata and value/string conversion API",
    ),
    BindingSurfaceSymbolSpec::interface(
        "INoteExpressionPhysicalUIMapping",
        "pluginterfaces/vst/ivstnoteexpression.h",
        "host Note Expression physical UI mapping API",
    ),
    BindingSurfaceSymbolSpec::type_(
        "NoteExpressionTypeInfo",
        "pluginterfaces/vst/ivstnoteexpression.h",
        "host-visible Note Expression type metadata",
    ),
    BindingSurfaceSymbolSpec::type_(
        "PhysicalUIMap",
        "pluginterfaces/vst/ivstnoteexpression.h",
        "static Note Expression physical UI mapping payload",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IParameterChanges",
        "pluginterfaces/vst/ivstparameterchanges.h",
        "sample-accurate automation input/output list",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IParamValueQueue",
        "pluginterfaces/vst/ivstparameterchanges.h",
        "sample-accurate automation point queue",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IEventList",
        "pluginterfaces/vst/ivstevents.h",
        "VST3 note, MIDI CC and expression event list",
    ),
    BindingSurfaceSymbolSpec::type_(
        "Event",
        "pluginterfaces/vst/ivstevents.h",
        "note on/off, poly pressure and legacy MIDI event payloads",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IMessage",
        "pluginterfaces/vst/ivstmessage.h",
        "controller/processor telemetry bind message",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IAttributeList",
        "pluginterfaces/vst/ivstmessage.h",
        "typed message attributes for telemetry bind",
    ),
    BindingSurfaceSymbolSpec::type_(
        "ProcessData",
        "pluginterfaces/vst/ivstaudioprocessor.h",
        "audio buffers, automation, events and process mode",
    ),
    BindingSurfaceSymbolSpec::type_(
        "ProcessContext",
        "pluginterfaces/vst/ivstprocesscontext.h",
        "tempo, play state and sample position mirror",
    ),
    BindingSurfaceSymbolSpec::type_(
        "BusInfo",
        "pluginterfaces/vst/ivstcomponent.h",
        "audio/event bus negotiation metadata",
    ),
    BindingSurfaceSymbolSpec::type_(
        "RoutingInfo",
        "pluginterfaces/vst/ivstcomponent.h",
        "routing callback ABI placeholder",
    ),
    BindingSurfaceSymbolSpec::type_(
        "ParameterInfo",
        "pluginterfaces/vst/ivsteditcontroller.h",
        "host parameter metadata",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IUnitInfo",
        "pluginterfaces/vst/ivstunits.h",
        "host unit and factory program-list metadata API",
    ),
    BindingSurfaceSymbolSpec::interface(
        "IProgramListData",
        "pluginterfaces/vst/ivstunits.h",
        "controller-side program data save/load API",
    ),
    BindingSurfaceSymbolSpec::type_(
        "UnitInfo",
        "pluginterfaces/vst/ivstunits.h",
        "host-visible unit metadata",
    ),
    BindingSurfaceSymbolSpec::type_(
        "ProgramListInfo",
        "pluginterfaces/vst/ivstunits.h",
        "host-visible factory program-list metadata",
    ),
    BindingSurfaceSymbolSpec::type_(
        "ParamID",
        "pluginterfaces/vst/vsttypes.h",
        "stable host-visible parameter identifier",
    ),
    BindingSurfaceSymbolSpec::type_(
        "ParamValue",
        "pluginterfaces/vst/vsttypes.h",
        "normalized parameter value ABI",
    ),
    BindingSurfaceSymbolSpec::type_(
        "TChar",
        "pluginterfaces/base/fstrdefs.h",
        "UTF-16 parameter and bus display strings",
    ),
    BindingSurfaceSymbolSpec::type_(
        "TUID",
        "pluginterfaces/base/funknown.h",
        "VST3 class and interface identifiers",
    ),
    BindingSurfaceSymbolSpec::constant(
        "kResultOk",
        "pluginterfaces/base/funknown.h",
        "VST3 callback success result",
    ),
    BindingSurfaceSymbolSpec::constant(
        "kInvalidArgument",
        "pluginterfaces/base/funknown.h",
        "VST3 callback invalid argument result",
    ),
    BindingSurfaceSymbolSpec::constant(
        "kNotImplemented",
        "pluginterfaces/base/funknown.h",
        "VST3 optional API not implemented result",
    ),
    BindingSurfaceSymbolSpec::constant(
        "kPlatformTypeHWND",
        "pluginterfaces/gui/iplugview.h",
        "Windows editor parent platform type",
    ),
    BindingSurfaceSymbolSpec::constant(
        "kPlatformTypeNSView",
        "pluginterfaces/gui/iplugview.h",
        "macOS editor parent platform type",
    ),
    BindingSurfaceSymbolSpec::constant(
        "kPlatformTypeX11EmbedWindowID",
        "pluginterfaces/gui/iplugview.h",
        "Linux X11 editor parent platform type",
    ),
];

const GENERATED_BINDINGS_INTERFACE_IDS: &[BindingInterfaceIdSpec] = &[
    BindingInterfaceIdSpec::new("FUnknown", [0x00000000, 0x00000000, 0xC0000000, 0x00000046]),
    BindingInterfaceIdSpec::new(
        "IPluginFactory",
        [0x7A4D811C, 0x52114A1F, 0xAED9D2EE, 0x0B43BF9F],
    ),
    BindingInterfaceIdSpec::new(
        "IPluginBase",
        [0x22888DDB, 0x156E45AE, 0x8358B348, 0x08190625],
    ),
    BindingInterfaceIdSpec::new(
        "IConnectionPoint",
        [0x70A4156F, 0x6E6E4026, 0x989148BF, 0xAA60D8D1],
    ),
    BindingInterfaceIdSpec::new("IBStream", [0xC3BF6EA2, 0x30994752, 0x9B6BF990, 0x1EE33E9B]),
    BindingInterfaceIdSpec::new(
        "IPlugView",
        [0x5BC32507, 0xD06049EA, 0xA6151B52, 0x2B755B29],
    ),
    BindingInterfaceIdSpec::new(
        "IPlugFrame",
        [0x367FAF01, 0xAFA94693, 0x8D4DA2A0, 0xED0882A3],
    ),
    BindingInterfaceIdSpec::new(
        "IComponent",
        [0xE831FF31, 0xF2D54301, 0x928EBBEE, 0x25697802],
    ),
    BindingInterfaceIdSpec::new(
        "IAudioProcessor",
        [0x42043F99, 0xB7DA453C, 0xA569E79D, 0x9AAEC33D],
    ),
    BindingInterfaceIdSpec::new(
        "IProcessContextRequirements",
        [0x2A654303, 0xEF764E3D, 0x95B5FE83, 0x730EF6D0],
    ),
    BindingInterfaceIdSpec::new(
        "IEditController",
        [0xDCD7BBE3, 0x7742448D, 0xA874AACC, 0x979C759E],
    ),
    BindingInterfaceIdSpec::new(
        "IComponentHandler",
        [0x93A0BEA3, 0x0BD045DB, 0x8E890B0C, 0xC1E46AC6],
    ),
    BindingInterfaceIdSpec::new(
        "IMidiMapping",
        [0xDF0FF9F7, 0x49B74669, 0xB63AB732, 0x7ADBF5E5],
    ),
    BindingInterfaceIdSpec::new(
        "INoteExpressionController",
        [0xB7F8F859, 0x41234872, 0x91169581, 0x4F3721A3],
    ),
    BindingInterfaceIdSpec::new(
        "INoteExpressionPhysicalUIMapping",
        [0xB03078FF, 0x94D24AC8, 0x90CCD303, 0xD4133324],
    ),
    BindingInterfaceIdSpec::new(
        "IParameterChanges",
        [0xA4779663, 0x0BB64A56, 0xB44384A8, 0x466FEB9D],
    ),
    BindingInterfaceIdSpec::new(
        "IParamValueQueue",
        [0x01263A18, 0xED074F6F, 0x98C9D356, 0x4686F9BA],
    ),
    BindingInterfaceIdSpec::new(
        "IEventList",
        [0x3A2C4214, 0x346349FE, 0xB2C4F397, 0xB9695A44],
    ),
    BindingInterfaceIdSpec::new("IMessage", [0x936F033B, 0xC6C047DB, 0xBB0882F8, 0x13C1E613]),
    BindingInterfaceIdSpec::new(
        "IAttributeList",
        [0x1E5F0AEB, 0xCC7F4533, 0xA2544011, 0x38AD5EE4],
    ),
    BindingInterfaceIdSpec::new(
        "IUnitInfo",
        [0x3D4BD6B5, 0x913A4FD2, 0xA886E768, 0xA5EB92C1],
    ),
    BindingInterfaceIdSpec::new(
        "IProgramListData",
        [0x8683B01F, 0x7B354F70, 0xA2651DEC, 0x353AF4FF],
    ),
];

const GENERATED_BINDINGS_COM_OBJECT_INTERFACES: &[BindingComObjectInterfaceSpec] = &[
    BindingComObjectInterfaceSpec::new("VestyAttributeList", "IAttributeList"),
    BindingComObjectInterfaceSpec::new("VestyMessage", "IMessage"),
    BindingComObjectInterfaceSpec::new("VestyProcessor", "IComponent"),
    BindingComObjectInterfaceSpec::new("VestyProcessor", "IAudioProcessor"),
    BindingComObjectInterfaceSpec::new("VestyProcessor", "IProcessContextRequirements"),
    BindingComObjectInterfaceSpec::new("VestyProcessor", "IConnectionPoint"),
    BindingComObjectInterfaceSpec::new("VestyPlugView", "IPlugView"),
    BindingComObjectInterfaceSpec::new("VestyController", "IEditController"),
    BindingComObjectInterfaceSpec::new("VestyController", "IConnectionPoint"),
    BindingComObjectInterfaceSpec::new("VestyController", "IMidiMapping"),
    BindingComObjectInterfaceSpec::new("VestyController", "IUnitInfo"),
    BindingComObjectInterfaceSpec::new("VestyController", "IProgramListData"),
    BindingComObjectInterfaceSpec::new("VestyController", "INoteExpressionController"),
    BindingComObjectInterfaceSpec::new("VestyController", "INoteExpressionPhysicalUIMapping"),
    BindingComObjectInterfaceSpec::new("VestyFactory", "IPluginFactory"),
];

const GENERATED_BINDINGS_COM_OBJECTS: &[&str] = &[
    "VestyAttributeList",
    "VestyMessage",
    "VestyProcessor",
    "VestyPlugView",
    "VestyController",
    "VestyFactory",
];

const GENERATED_BINDINGS_COM_OBJECT_IDENTITIES: &[BindingComObjectIdentitySpec] = &[
    BindingComObjectIdentitySpec::new("VestyAttributeList", "IAttributeList"),
    BindingComObjectIdentitySpec::new("VestyMessage", "IMessage"),
    BindingComObjectIdentitySpec::new("VestyProcessor", "IComponent"),
    BindingComObjectIdentitySpec::new("VestyPlugView", "IPlugView"),
    BindingComObjectIdentitySpec::new("VestyController", "IEditController"),
    BindingComObjectIdentitySpec::new("VestyFactory", "IPluginFactory"),
];

const GENERATED_BINDINGS_FACTORY_EXPORT_PLAN: BindingFactoryExportPlanSpec =
    BindingFactoryExportPlanSpec::new();

const GENERATED_BINDINGS_FACTORY_CLASS_PLANS: &[BindingFactoryClassPlanSpec] = &[
    BindingFactoryClassPlanSpec::processor(),
    BindingFactoryClassPlanSpec::controller(),
];

const GENERATED_BINDINGS_MODULE_EXPORT_PLANS: &[BindingModuleExportPlanSpec] = &[
    BindingModuleExportPlanSpec::new(
        "GETPLUGINFACTORY_MODULE_EXPORT_PLAN",
        "GetPluginFactory",
        "windows,macos,linux",
        "extern \"system\" fn() -> *mut IPluginFactory",
        "return VST3 plugin factory pointer",
        "vesty_vst3::create_plugin_factory::<Plugin>()",
        "returns owned COM factory pointer for host discovery",
    ),
    BindingModuleExportPlanSpec::new(
        "WINDOWS_INITDLL_MODULE_EXPORT_PLAN",
        "InitDll",
        "windows",
        "extern \"system\" fn() -> bool",
        "Windows VST3 module initialization entry",
        "return true",
        "host may continue loading the module",
    ),
    BindingModuleExportPlanSpec::new(
        "WINDOWS_EXITDLL_MODULE_EXPORT_PLAN",
        "ExitDll",
        "windows",
        "extern \"system\" fn() -> bool",
        "Windows VST3 module termination entry",
        "return true",
        "host may unload the module",
    ),
    BindingModuleExportPlanSpec::new(
        "MACOS_BUNDLEENTRY_MODULE_EXPORT_PLAN",
        "bundleEntry",
        "macos",
        "extern \"system\" fn(bundle_ref: *mut c_void) -> bool",
        "macOS VST3 bundle initialization entry",
        "vesty_vst3::set_macos_bundle_ref(bundle_ref); return true",
        "bundle resources path is captured when possible",
    ),
    BindingModuleExportPlanSpec::new(
        "MACOS_BUNDLEEXIT_MODULE_EXPORT_PLAN",
        "bundleExit",
        "macos",
        "extern \"system\" fn() -> bool",
        "macOS VST3 bundle termination entry",
        "return true",
        "host may unload the bundle",
    ),
    BindingModuleExportPlanSpec::new(
        "MACOS_BUNDLEENTRY_COMPAT_MODULE_EXPORT_PLAN",
        "BundleEntry",
        "macos",
        "extern \"system\" fn(bundle_ref: *mut c_void) -> bool",
        "macOS compatibility initialization alias",
        "delegate to bundleEntry(bundle_ref)",
        "keeps uppercase host lookup compatibility",
    ),
    BindingModuleExportPlanSpec::new(
        "MACOS_BUNDLEEXIT_COMPAT_MODULE_EXPORT_PLAN",
        "BundleExit",
        "macos",
        "extern \"system\" fn() -> bool",
        "macOS compatibility termination alias",
        "delegate to bundleExit()",
        "keeps uppercase host lookup compatibility",
    ),
    BindingModuleExportPlanSpec::new(
        "LINUX_MODULEENTRY_MODULE_EXPORT_PLAN",
        "ModuleEntry",
        "linux",
        "extern \"system\" fn(library_handle: *mut c_void) -> bool",
        "Linux VST3 module initialization entry",
        "return true",
        "host may continue loading the module",
    ),
    BindingModuleExportPlanSpec::new(
        "LINUX_MODULEEXIT_MODULE_EXPORT_PLAN",
        "ModuleExit",
        "linux",
        "extern \"system\" fn() -> bool",
        "Linux VST3 module termination entry",
        "return true",
        "host may unload the module",
    ),
];

const GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS: &[BindingBinaryExportSymbolPlanSpec] = &[
    BindingBinaryExportSymbolPlanSpec::new(
        "WINDOWS_GETPLUGINFACTORY_BINARY_EXPORT_SYMBOL_PLAN",
        "windows-x64",
        "PE/COFF",
        "GetPluginFactory",
        "GetPluginFactory",
        "dumpbin /exports or llvm-objdump -p",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "WINDOWS_INITDLL_BINARY_EXPORT_SYMBOL_PLAN",
        "windows-x64",
        "PE/COFF",
        "InitDll",
        "InitDll",
        "dumpbin /exports or llvm-objdump -p",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "WINDOWS_EXITDLL_BINARY_EXPORT_SYMBOL_PLAN",
        "windows-x64",
        "PE/COFF",
        "ExitDll",
        "ExitDll",
        "dumpbin /exports or llvm-objdump -p",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "MACOS_GETPLUGINFACTORY_BINARY_EXPORT_SYMBOL_PLAN",
        "macos",
        "Mach-O",
        "GetPluginFactory",
        "_GetPluginFactory",
        "nm -gU or llvm-nm -gU",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "MACOS_BUNDLEENTRY_BINARY_EXPORT_SYMBOL_PLAN",
        "macos",
        "Mach-O",
        "bundleEntry",
        "_bundleEntry",
        "nm -gU or llvm-nm -gU",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "MACOS_BUNDLEEXIT_BINARY_EXPORT_SYMBOL_PLAN",
        "macos",
        "Mach-O",
        "bundleExit",
        "_bundleExit",
        "nm -gU or llvm-nm -gU",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "MACOS_BUNDLEENTRY_COMPAT_BINARY_EXPORT_SYMBOL_PLAN",
        "macos",
        "Mach-O",
        "BundleEntry",
        "_BundleEntry",
        "nm -gU or llvm-nm -gU",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "MACOS_BUNDLEEXIT_COMPAT_BINARY_EXPORT_SYMBOL_PLAN",
        "macos",
        "Mach-O",
        "BundleExit",
        "_BundleExit",
        "nm -gU or llvm-nm -gU",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "LINUX_GETPLUGINFACTORY_BINARY_EXPORT_SYMBOL_PLAN",
        "linux-x64",
        "ELF",
        "GetPluginFactory",
        "GetPluginFactory",
        "nm -D --defined-only or llvm-nm -D --defined-only",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "LINUX_MODULEENTRY_BINARY_EXPORT_SYMBOL_PLAN",
        "linux-x64",
        "ELF",
        "ModuleEntry",
        "ModuleEntry",
        "nm -D --defined-only or llvm-nm -D --defined-only",
    ),
    BindingBinaryExportSymbolPlanSpec::new(
        "LINUX_MODULEEXIT_BINARY_EXPORT_SYMBOL_PLAN",
        "linux-x64",
        "ELF",
        "ModuleExit",
        "ModuleExit",
        "nm -D --defined-only or llvm-nm -D --defined-only",
    ),
];

pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[0].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[1].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[2].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[3].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[4].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[5].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[6].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[7].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[8].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[9].public(),
    GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS[10].public(),
];

const WINDOWS_X64_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS: &[&str] =
    &["GetPluginFactory", "InitDll", "ExitDll"];
const MACOS_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS: &[&str] = &[
    "_GetPluginFactory",
    "_bundleEntry",
    "_bundleExit",
    "_BundleEntry",
    "_BundleExit",
];
const LINUX_X64_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS: &[&str] =
    &["GetPluginFactory", "ModuleEntry", "ModuleExit"];

const MACOS_BINARY_EXPORT_INSPECTION_TOOLS: &[BinaryExportInspectionToolPlan] = &[
    BinaryExportInspectionToolPlan {
        platform: "macos",
        program: "nm",
        args: &["-gU"],
    },
    BinaryExportInspectionToolPlan {
        platform: "macos",
        program: "llvm-nm",
        args: &["-gU"],
    },
];

const WINDOWS_X64_BINARY_EXPORT_INSPECTION_TOOLS: &[BinaryExportInspectionToolPlan] = &[
    BinaryExportInspectionToolPlan {
        platform: "windows-x64",
        program: "llvm-objdump",
        args: &["-p"],
    },
    BinaryExportInspectionToolPlan {
        platform: "windows-x64",
        program: "dumpbin",
        args: &["/exports"],
    },
];

const LINUX_X64_BINARY_EXPORT_INSPECTION_TOOLS: &[BinaryExportInspectionToolPlan] = &[
    BinaryExportInspectionToolPlan {
        platform: "linux-x64",
        program: "nm",
        args: &["-D", "--defined-only"],
    },
    BinaryExportInspectionToolPlan {
        platform: "linux-x64",
        program: "llvm-nm",
        args: &["-D", "--defined-only"],
    },
];

pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[
    MACOS_BINARY_EXPORT_INSPECTION_TOOLS[0],
    MACOS_BINARY_EXPORT_INSPECTION_TOOLS[1],
    WINDOWS_X64_BINARY_EXPORT_INSPECTION_TOOLS[0],
    WINDOWS_X64_BINARY_EXPORT_INSPECTION_TOOLS[1],
    LINUX_X64_BINARY_EXPORT_INSPECTION_TOOLS[0],
    LINUX_X64_BINARY_EXPORT_INSPECTION_TOOLS[1],
];

pub fn binary_export_symbol_plans() -> &'static [BinaryExportSymbolPlan] {
    BINARY_EXPORT_SYMBOL_PLANS
}

pub fn binary_export_symbol_plan(
    platform: &str,
    tool_symbol: &str,
) -> Option<&'static BinaryExportSymbolPlan> {
    BINARY_EXPORT_SYMBOL_PLANS
        .iter()
        .find(|plan| plan.platform == platform && plan.tool_symbol == tool_symbol)
}

pub fn required_binary_export_tool_symbols(platform: &str) -> Option<&'static [&'static str]> {
    match platform {
        "windows-x64" => Some(WINDOWS_X64_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS),
        "macos" => Some(MACOS_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS),
        "linux-x64" => Some(LINUX_X64_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS),
        _ => None,
    }
}

pub fn binary_export_inspection_tool_plans() -> &'static [BinaryExportInspectionToolPlan] {
    BINARY_EXPORT_INSPECTION_TOOL_PLANS
}

pub fn binary_export_inspection_tools(
    platform: &str,
) -> Option<&'static [BinaryExportInspectionToolPlan]> {
    match platform {
        "macos" => Some(MACOS_BINARY_EXPORT_INSPECTION_TOOLS),
        "windows-x64" => Some(WINDOWS_X64_BINARY_EXPORT_INSPECTION_TOOLS),
        "linux-x64" => Some(LINUX_X64_BINARY_EXPORT_INSPECTION_TOOLS),
        _ => None,
    }
}

pub fn required_binary_export_symbol_count(platform: &str) -> usize {
    required_binary_export_tool_symbols(platform)
        .map(<[&str]>::len)
        .unwrap_or(0)
}

pub fn first_missing_binary_export_symbol(
    platform: &str,
    found_symbols: &[&str],
) -> Option<&'static str> {
    required_binary_export_tool_symbols(platform)?
        .iter()
        .copied()
        .find(|required| !found_symbols.iter().any(|found| found == required))
}

pub fn binary_export_required_symbols_present(platform: &str, found_symbols: &[&str]) -> bool {
    required_binary_export_symbol_count(platform) > 0
        && first_missing_binary_export_symbol(platform, found_symbols).is_none()
}

const GENERATED_BINDINGS_INTERFACE_METHODS: &[BindingInterfaceMethodSpec] = &[
    BindingInterfaceMethodSpec::new(
        "FUnknown",
        "queryInterface",
        "query COM interface identity",
        false,
    ),
    BindingInterfaceMethodSpec::new("FUnknown", "addRef", "increment COM reference count", false),
    BindingInterfaceMethodSpec::new(
        "FUnknown",
        "release",
        "decrement COM reference count",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPluginFactory",
        "countClasses",
        "report exported VST3 class count",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPluginFactory",
        "getClassInfo",
        "return VST3 class metadata",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPluginFactory",
        "createInstance",
        "instantiate processor or controller COM object",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPluginBase",
        "initialize",
        "initialize a processor or controller with host context",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPluginBase",
        "terminate",
        "terminate a processor or controller instance",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IConnectionPoint",
        "connect",
        "connect processor/controller telemetry peer",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IConnectionPoint",
        "disconnect",
        "disconnect processor/controller telemetry peer",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IConnectionPoint",
        "notify",
        "deliver processor/controller telemetry message",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IBStream",
        "read",
        "read component/controller state bytes",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IBStream",
        "write",
        "write component/controller state bytes",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IBStream",
        "seek",
        "seek component/controller state stream",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IBStream",
        "tell",
        "query component/controller state stream offset",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "isPlatformTypeSupported",
        "check host editor parent platform type",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "attached",
        "attach system WebView to host editor parent",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "removed",
        "detach system WebView from host editor parent",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "onWheel",
        "optional editor mouse-wheel callback",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "onKeyDown",
        "optional editor key-down callback",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "onKeyUp",
        "optional editor key-up callback",
        false,
    ),
    BindingInterfaceMethodSpec::new("IPlugView", "getSize", "query current editor bounds", false),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "onSize",
        "resize system WebView editor bounds",
        false,
    ),
    BindingInterfaceMethodSpec::new("IPlugView", "onFocus", "editor focus notification", false),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "setFrame",
        "set host editor frame callback",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "canResize",
        "report editor resize support",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugView",
        "checkSizeConstraint",
        "clamp editor size constraints",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IPlugFrame",
        "resizeView",
        "request host editor frame resize",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "getControllerClassId",
        "return paired edit controller class id",
        false,
    ),
    BindingInterfaceMethodSpec::new("IComponent", "setIoMode", "set host IO mode", false),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "getBusCount",
        "report audio/event bus count",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "getBusInfo",
        "return audio/event bus metadata",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "getRoutingInfo",
        "return bus routing metadata",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "activateBus",
        "activate or deactivate audio/event bus",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "setActive",
        "activate or deactivate component",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "setState",
        "restore processor/component state",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponent",
        "getState",
        "save processor/component state",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "setBusArrangements",
        "negotiate speaker arrangements",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "getBusArrangement",
        "query negotiated speaker arrangement",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "canProcessSampleSize",
        "report f32/f64 process support",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "getLatencySamples",
        "report fixed latency",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "setupProcessing",
        "prepare realtime processing resources",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "setProcessing",
        "start or stop realtime processing",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "process",
        "realtime audio/MIDI/process callback",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IAudioProcessor",
        "getTailSamples",
        "report fixed tail length",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IProcessContextRequirements",
        "getProcessContextRequirements",
        "declare required host transport/process context flags",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "setComponentState",
        "restore controller from component state",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "setState",
        "restore controller state",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "getState",
        "save controller state",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "getParameterCount",
        "report host parameter count",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "getParameterInfo",
        "return host parameter metadata",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "getParamStringByValue",
        "format normalized parameter value",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "getParamValueByString",
        "parse parameter display text",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "normalizedParamToPlain",
        "convert normalized parameter value to plain",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "plainParamToNormalized",
        "convert plain parameter value to normalized",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "getParamNormalized",
        "read controller parameter value",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "setParamNormalized",
        "write controller parameter value",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "setComponentHandler",
        "install host component handler",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IEditController",
        "createView",
        "create system WebView editor IPlugView",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponentHandler",
        "beginEdit",
        "notify host parameter gesture begin",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponentHandler",
        "performEdit",
        "notify host parameter gesture value",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponentHandler",
        "endEdit",
        "notify host parameter gesture end",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IComponentHandler",
        "restartComponent",
        "request host component restart",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IMidiMapping",
        "getMidiControllerAssignment",
        "map MIDI controller to host parameter id",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "INoteExpressionController",
        "getNoteExpressionCount",
        "report Note Expression type count",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "INoteExpressionController",
        "getNoteExpressionInfo",
        "return Note Expression type metadata",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "INoteExpressionController",
        "getNoteExpressionStringByValue",
        "format Note Expression value",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "INoteExpressionController",
        "getNoteExpressionValueByString",
        "parse Note Expression value text",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "INoteExpressionPhysicalUIMapping",
        "getPhysicalUIMapping",
        "return physical UI mapping for Note Expression",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IParameterChanges",
        "getParameterCount",
        "report automation queue count",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IParameterChanges",
        "getParameterData",
        "return automation queue by index",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IParamValueQueue",
        "getParameterId",
        "return automation parameter id",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IParamValueQueue",
        "getPointCount",
        "return automation point count",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IParamValueQueue",
        "getPoint",
        "return sample-accurate automation point",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IParamValueQueue",
        "addPoint",
        "append automation output point",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IEventList",
        "getEventCount",
        "report input event count",
        true,
    ),
    BindingInterfaceMethodSpec::new(
        "IEventList",
        "getEvent",
        "return input event by index",
        true,
    ),
    BindingInterfaceMethodSpec::new("IEventList", "addEvent", "append output event", true),
    BindingInterfaceMethodSpec::new(
        "IMessage",
        "getMessageID",
        "read processor/controller message id",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IMessage",
        "setMessageID",
        "set processor/controller message id",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IMessage",
        "getAttributes",
        "read processor/controller message attributes",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "setInt",
        "set integer message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "getInt",
        "get integer message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "setFloat",
        "set float message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "getFloat",
        "get float message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "setString",
        "set string message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "getString",
        "get string message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "setBinary",
        "set binary message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IAttributeList",
        "getBinary",
        "get binary message attribute",
        false,
    ),
    BindingInterfaceMethodSpec::new("IUnitInfo", "getUnitCount", "report host unit count", false),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "getUnitInfo",
        "return host unit metadata",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "getProgramListCount",
        "report program-list count",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "getProgramListInfo",
        "return program-list metadata",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "getProgramName",
        "return program display name",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "getProgramInfo",
        "return program attribute text",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "hasProgramPitchNames",
        "report program pitch-name support",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "getProgramPitchName",
        "return program pitch display name",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "getSelectedUnit",
        "return selected unit id",
        false,
    ),
    BindingInterfaceMethodSpec::new("IUnitInfo", "selectUnit", "select unit id", false),
    BindingInterfaceMethodSpec::new("IUnitInfo", "getUnitByBus", "map bus to unit id", false),
    BindingInterfaceMethodSpec::new(
        "IUnitInfo",
        "setUnitProgramData",
        "restore unit program data",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IProgramListData",
        "programDataSupported",
        "report program-data save/load support",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IProgramListData",
        "getProgramData",
        "save program data to stream",
        false,
    ),
    BindingInterfaceMethodSpec::new(
        "IProgramListData",
        "setProgramData",
        "restore program data from stream",
        false,
    ),
];

pub fn generated_bindings_surface_symbol_names_for_header(header: &str) -> Vec<&'static str> {
    GENERATED_BINDINGS_SURFACE_SYMBOLS
        .iter()
        .filter(|symbol| symbol.header == header)
        .map(|symbol| symbol.name)
        .collect()
}

pub fn generated_bindings_surface_symbol_specs() -> Vec<GeneratedBindingsSurfaceSymbolSpec> {
    GENERATED_BINDINGS_SURFACE_SYMBOLS
        .iter()
        .map(|symbol| symbol.public())
        .collect()
}

pub fn generated_bindings_interface_method_names_for_interface(
    interface: &str,
) -> Vec<&'static str> {
    GENERATED_BINDINGS_INTERFACE_METHODS
        .iter()
        .filter(|method| method.interface == interface)
        .map(|method| method.name)
        .collect()
}

pub fn generated_bindings_interface_method_signatures_for_interface(
    interface: &str,
) -> Vec<String> {
    GENERATED_BINDINGS_INTERFACE_METHODS
        .iter()
        .filter(|method| method.interface == interface)
        .map(interface_method_signature_intent)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingBackend {
    UpstreamVst3Crate,
    GeneratedHeadersReserved,
    MetadataOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingBaseline {
    pub steinberg_sdk: &'static str,
    pub upstream_vst3_crate: &'static str,
    pub backend: BindingBackend,
}

pub const fn active_binding_backend() -> BindingBackend {
    if cfg!(feature = "generated-headers") {
        BindingBackend::GeneratedHeadersReserved
    } else if cfg!(feature = "upstream-vst3") {
        BindingBackend::UpstreamVst3Crate
    } else {
        BindingBackend::MetadataOnly
    }
}

pub const BINDING_BASELINE: BindingBaseline = BindingBaseline {
    steinberg_sdk: STEINBERG_VST3_SDK_BASELINE,
    upstream_vst3_crate: UPSTREAM_VST3_CRATE_BASELINE,
    backend: active_binding_backend(),
};

pub const fn binding_backend_name(backend: BindingBackend) -> &'static str {
    match backend {
        BindingBackend::UpstreamVst3Crate => "upstream-vst3-crate",
        BindingBackend::GeneratedHeadersReserved => "generated-headers-reserved",
        BindingBackend::MetadataOnly => "metadata-only",
    }
}

pub fn generated_header_bindings_reserved() -> bool {
    cfg!(feature = "generated-headers")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SdkHeaderProbe {
    pub root: PathBuf,
    pub baseline: &'static str,
    pub present_headers: Vec<&'static str>,
    pub missing_headers: Vec<&'static str>,
    pub version_hint: Option<String>,
}

impl SdkHeaderProbe {
    pub fn ready_for_generated_headers(&self) -> bool {
        self.missing_headers.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SdkHeaderInputManifest {
    pub version: u32,
    pub generator: String,
    pub steinberg_sdk_baseline: String,
    pub upstream_vst3_crate_baseline: String,
    pub complete: bool,
    pub version_hint: Option<String>,
    pub headers: Vec<SdkHeaderInput>,
    pub missing_headers: Vec<String>,
}

impl SdkHeaderInputManifest {
    pub fn header(&self, path: &str) -> Option<&SdkHeaderInput> {
        self.headers.iter().find(|header| header.path == path)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SdkHeaderInput {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsPlan {
    pub version: u32,
    pub generator: String,
    pub status: String,
    pub bindings_generated: bool,
    pub steinberg_sdk_baseline: String,
    pub upstream_vst3_crate_baseline: String,
    pub active_backend: String,
    pub sdk_dir: String,
    pub bindings_module: String,
    pub header_manifest: SdkHeaderInputManifest,
    pub checks: Vec<GeneratedBindingsPlanCheck>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsPlanCheck {
    pub name: String,
    pub status: String,
    pub value: String,
    pub hint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsSurface {
    pub version: u32,
    pub generator: String,
    pub status: String,
    pub bindings_generated: bool,
    pub steinberg_sdk_baseline: String,
    pub upstream_vst3_crate_baseline: String,
    pub active_backend: String,
    pub sdk_dir: String,
    pub header_manifest: SdkHeaderInputManifest,
    pub required_headers: Vec<String>,
    pub missing_headers: Vec<String>,
    pub missing_symbols: Vec<String>,
    pub symbols: Vec<GeneratedBindingsSurfaceSymbol>,
    pub blockers: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBindingsSurfaceSymbol {
    pub name: String,
    pub kind: String,
    pub header: String,
    pub purpose: String,
    pub header_present: bool,
    pub symbol_present: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsScaffold {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsAbiSeed {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsAbi {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsInterfaceSkeleton {
    pub plan: GeneratedBindingsPlan,
    pub surface: GeneratedBindingsSurface,
    pub module: String,
}

#[derive(Debug)]
pub enum SdkHeaderManifestError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidHeaderInput {
        path: PathBuf,
        reason: String,
    },
    Drift {
        differences: Vec<String>,
    },
}

impl std::fmt::Display for SdkHeaderManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SdkHeaderManifestError::Io { path, source } => {
                write!(formatter, "{}: {source}", path.display())
            }
            SdkHeaderManifestError::InvalidHeaderInput { path, reason } => {
                write!(formatter, "{}: {reason}", path.display())
            }
            SdkHeaderManifestError::Drift { differences } => {
                write!(
                    formatter,
                    "SDK header manifest drift: {}",
                    differences.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for SdkHeaderManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SdkHeaderManifestError::Io { source, .. } => Some(source),
            SdkHeaderManifestError::InvalidHeaderInput { .. }
            | SdkHeaderManifestError::Drift { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum GeneratedBindingsScaffoldError {
    Manifest(SdkHeaderManifestError),
    Blocked { blockers: Vec<String> },
}

impl std::fmt::Display for GeneratedBindingsScaffoldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneratedBindingsScaffoldError::Manifest(error) => write!(formatter, "{error}"),
            GeneratedBindingsScaffoldError::Blocked { blockers } => write!(
                formatter,
                "generated bindings scaffold is blocked: {}",
                blockers.join("; ")
            ),
        }
    }
}

impl std::error::Error for GeneratedBindingsScaffoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GeneratedBindingsScaffoldError::Manifest(error) => Some(error),
            GeneratedBindingsScaffoldError::Blocked { .. } => None,
        }
    }
}

impl From<SdkHeaderManifestError> for GeneratedBindingsScaffoldError {
    fn from(error: SdkHeaderManifestError) -> Self {
        GeneratedBindingsScaffoldError::Manifest(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SdkHeaderProbeError {
    MissingEnv,
}

pub fn probe_sdk_headers_from_env() -> Result<SdkHeaderProbe, SdkHeaderProbeError> {
    let root = std::env::var_os(VST3_SDK_DIR_ENV).ok_or(SdkHeaderProbeError::MissingEnv)?;
    Ok(probe_sdk_headers(root))
}

pub fn probe_sdk_headers(root: impl AsRef<Path>) -> SdkHeaderProbe {
    let root = root.as_ref().to_path_buf();
    let mut present_headers = Vec::new();
    let mut missing_headers = Vec::new();
    for header in REQUIRED_GENERATED_HEADER_INPUTS {
        if path_is_regular_file_no_symlink(&root.join(header)) {
            present_headers.push(*header);
        } else {
            missing_headers.push(*header);
        }
    }

    SdkHeaderProbe {
        version_hint: sdk_version_hint(&root),
        root,
        baseline: STEINBERG_VST3_SDK_BASELINE,
        present_headers,
        missing_headers,
    }
}

pub fn sdk_header_input_manifest(
    root: impl AsRef<Path>,
) -> Result<SdkHeaderInputManifest, SdkHeaderManifestError> {
    let root = root.as_ref();
    let probe = probe_sdk_headers(root);
    let mut headers = Vec::new();
    let mut missing_headers = Vec::new();

    for relative in REQUIRED_GENERATED_HEADER_INPUTS {
        let path = root.join(relative);
        let Some((metadata, bytes)) = sdk_header_bytes_no_symlink(&path)? else {
            missing_headers.push((*relative).to_string());
            continue;
        };
        headers.push(SdkHeaderInput {
            path: (*relative).to_string(),
            size: metadata.len(),
            sha256: sha256_hex(&bytes),
        });
    }

    Ok(SdkHeaderInputManifest {
        version: SDK_HEADER_MANIFEST_VERSION,
        generator: SDK_HEADER_MANIFEST_GENERATOR.to_string(),
        steinberg_sdk_baseline: STEINBERG_VST3_SDK_BASELINE.to_string(),
        upstream_vst3_crate_baseline: UPSTREAM_VST3_CRATE_BASELINE.to_string(),
        complete: missing_headers.is_empty(),
        version_hint: probe.version_hint,
        headers,
        missing_headers,
    })
}

fn sdk_header_bytes_no_symlink(
    path: &Path,
) -> Result<Option<(std::fs::Metadata, Vec<u8>)>, SdkHeaderManifestError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(SdkHeaderManifestError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Err(SdkHeaderManifestError::InvalidHeaderInput {
            path: path.to_path_buf(),
            reason: "header input must be a regular file, not a symlink".to_string(),
        });
    }
    if !file_type.is_file() {
        return Err(SdkHeaderManifestError::InvalidHeaderInput {
            path: path.to_path_buf(),
            reason: "header input must be a regular file".to_string(),
        });
    }
    let bytes = std::fs::read(path).map_err(|source| SdkHeaderManifestError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Some((metadata, bytes)))
}

fn path_is_regular_file_no_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| {
        let file_type = metadata.file_type();
        file_type.is_file() && !file_type.is_symlink()
    })
}

pub fn check_sdk_header_input_manifest(
    root: impl AsRef<Path>,
    expected: &SdkHeaderInputManifest,
) -> Result<SdkHeaderInputManifest, SdkHeaderManifestError> {
    let actual = sdk_header_input_manifest(root)?;
    let differences = sdk_header_manifest_differences(expected, &actual);
    if differences.is_empty() {
        Ok(actual)
    } else {
        Err(SdkHeaderManifestError::Drift { differences })
    }
}

pub fn sdk_header_manifest_differences(
    expected: &SdkHeaderInputManifest,
    actual: &SdkHeaderInputManifest,
) -> Vec<String> {
    let mut differences = Vec::new();
    if expected.version != actual.version {
        differences.push(format!(
            "version expected {} actual {}",
            expected.version, actual.version
        ));
    }
    if expected.generator != actual.generator {
        differences.push(format!(
            "generator expected {} actual {}",
            expected.generator, actual.generator
        ));
    }
    if expected.steinberg_sdk_baseline != actual.steinberg_sdk_baseline {
        differences.push(format!(
            "Steinberg SDK baseline expected {} actual {}",
            expected.steinberg_sdk_baseline, actual.steinberg_sdk_baseline
        ));
    }
    if expected.upstream_vst3_crate_baseline != actual.upstream_vst3_crate_baseline {
        differences.push(format!(
            "upstream vst3 crate baseline expected {} actual {}",
            expected.upstream_vst3_crate_baseline, actual.upstream_vst3_crate_baseline
        ));
    }
    if expected.complete != actual.complete {
        differences.push(format!(
            "complete expected {} actual {}",
            expected.complete, actual.complete
        ));
    }
    if expected.version_hint != actual.version_hint {
        differences.push(format!(
            "version hint expected {:?} actual {:?}",
            expected.version_hint, actual.version_hint
        ));
    }
    if expected.missing_headers != actual.missing_headers {
        differences.push(format!(
            "missing headers expected {:?} actual {:?}",
            expected.missing_headers, actual.missing_headers
        ));
    }

    let expected_headers = expected
        .headers
        .iter()
        .map(|header| (header.path.as_str(), header))
        .collect::<BTreeMap<_, _>>();
    let actual_headers = actual
        .headers
        .iter()
        .map(|header| (header.path.as_str(), header))
        .collect::<BTreeMap<_, _>>();

    for path in expected_headers.keys() {
        if !actual_headers.contains_key(path) {
            differences.push(format!("header missing from actual manifest: {path}"));
        }
    }
    for path in actual_headers.keys() {
        if !expected_headers.contains_key(path) {
            differences.push(format!("unexpected header in actual manifest: {path}"));
        }
    }
    for (path, expected_header) in expected_headers {
        let Some(actual_header) = actual_headers.get(path) else {
            continue;
        };
        if expected_header.size != actual_header.size {
            differences.push(format!(
                "{path} size expected {} actual {}",
                expected_header.size, actual_header.size
            ));
        }
        if expected_header.sha256 != actual_header.sha256 {
            differences.push(format!("{path} sha256 mismatch"));
        }
    }

    differences
}

pub fn generated_bindings_plan(
    root: impl AsRef<Path>,
    bindings_module: impl AsRef<Path>,
) -> Result<GeneratedBindingsPlan, SdkHeaderManifestError> {
    let root = root.as_ref();
    let bindings_module = bindings_module.as_ref();
    let manifest = sdk_header_input_manifest(root)?;
    let mut checks = Vec::new();
    let mut blockers = Vec::new();

    if manifest.complete {
        checks.push(GeneratedBindingsPlanCheck {
            name: "sdk header inputs".to_string(),
            status: "ok".to_string(),
            value: format!("{} required header(s)", manifest.headers.len()),
            hint: None,
        });
    } else {
        let missing = manifest.missing_headers.join(", ");
        blockers.push(format!("missing SDK header inputs: {missing}"));
        checks.push(GeneratedBindingsPlanCheck {
            name: "sdk header inputs".to_string(),
            status: "failed".to_string(),
            value: format!(
                "missing {} required header(s)",
                manifest.missing_headers.len()
            ),
            hint: Some(format!(
                "use an official Steinberg VST3 SDK {STEINBERG_VST3_SDK_BASELINE} checkout"
            )),
        });
    }

    let module_check = generated_bindings_module_path_check(bindings_module);
    if module_check.status == "failed" {
        blockers.push(module_check.value.clone());
    }
    checks.push(module_check);
    checks.push(GeneratedBindingsPlanCheck {
        name: "binding emitter".to_string(),
        status: "reserved".to_string(),
        value: "Vesty has locked SDK inputs; the full SDK 3.8 Rust binding emitter is not enabled yet"
            .to_string(),
        hint: Some(
            "keep using the upstream `vst3` crate backend until generated bindings are implemented and validated"
                .to_string(),
        ),
    });

    let status = if blockers.is_empty() {
        "ready-for-binding-generator"
    } else {
        "blocked"
    };

    Ok(GeneratedBindingsPlan {
        version: GENERATED_BINDINGS_PLAN_VERSION,
        generator: GENERATED_BINDINGS_PLAN_GENERATOR.to_string(),
        status: status.to_string(),
        bindings_generated: false,
        steinberg_sdk_baseline: STEINBERG_VST3_SDK_BASELINE.to_string(),
        upstream_vst3_crate_baseline: UPSTREAM_VST3_CRATE_BASELINE.to_string(),
        active_backend: binding_backend_name(BINDING_BASELINE.backend).to_string(),
        sdk_dir: root.display().to_string(),
        bindings_module: bindings_module.display().to_string(),
        header_manifest: manifest,
        checks,
        blockers,
        next_steps: vec![
            "keep the SDK header manifest under release evidence when auditing generated-header inputs".to_string(),
            "implement the generated binding emitter from the locked header set".to_string(),
            "compare generated bindings against upstream `vst3` crate coverage before switching the adapter backend".to_string(),
            "run Steinberg validator and DAW smoke before claiming generated SDK 3.8 backend support".to_string(),
        ],
    })
}

pub fn generated_bindings_surface(
    root: impl AsRef<Path>,
) -> Result<GeneratedBindingsSurface, SdkHeaderManifestError> {
    let root = root.as_ref();
    let manifest = sdk_header_input_manifest(root)?;
    let present_headers = manifest
        .headers
        .iter()
        .map(|header| header.path.as_str())
        .collect::<BTreeSet<_>>();
    let required_headers = REQUIRED_GENERATED_HEADER_INPUTS
        .iter()
        .map(|header| (*header).to_string())
        .collect::<Vec<_>>();
    let mut header_texts = BTreeMap::new();
    for header in &manifest.headers {
        let path = root.join(&header.path);
        let Some((_, bytes)) = sdk_header_bytes_no_symlink(&path)? else {
            continue;
        };
        header_texts.insert(
            header.path.as_str(),
            String::from_utf8_lossy(&bytes).into_owned(),
        );
    }
    let symbols = GENERATED_BINDINGS_SURFACE_SYMBOLS
        .iter()
        .map(|symbol| {
            let header_present = present_headers.contains(symbol.header);
            let symbol_present = header_present
                && header_texts
                    .get(symbol.header)
                    .is_some_and(|text| contains_identifier_token(text, symbol.name));
            GeneratedBindingsSurfaceSymbol {
                name: symbol.name.to_string(),
                kind: symbol.kind.to_string(),
                header: symbol.header.to_string(),
                purpose: symbol.purpose.to_string(),
                header_present,
                symbol_present,
            }
        })
        .collect::<Vec<_>>();

    let mut blockers = Vec::new();
    if !manifest.complete {
        blockers.push(format!(
            "missing SDK header inputs: {}",
            manifest.missing_headers.join(", ")
        ));
    }

    let unknown_symbol_headers = symbols
        .iter()
        .filter(|symbol| {
            !REQUIRED_GENERATED_HEADER_INPUTS
                .iter()
                .any(|header| *header == symbol.header)
        })
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if !unknown_symbol_headers.is_empty() {
        blockers.push(format!(
            "surface symbols reference headers outside the locked input set: {}",
            unknown_symbol_headers.join(", ")
        ));
    }

    let missing_symbol_headers = symbols
        .iter()
        .filter(|symbol| !symbol.header_present)
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if !missing_symbol_headers.is_empty() {
        blockers.push(format!(
            "surface symbols reference missing headers: {}",
            missing_symbol_headers.join(", ")
        ));
    }

    let missing_symbols = symbols
        .iter()
        .filter(|symbol| symbol.header_present && !symbol.symbol_present)
        .map(|symbol| format!("{} -> {}", symbol.name, symbol.header))
        .collect::<Vec<_>>();
    if !missing_symbols.is_empty() {
        blockers.push(format!(
            "surface symbols are absent from their locked headers: {}",
            missing_symbols.join(", ")
        ));
    }

    let status = if blockers.is_empty() {
        "ready-for-binding-emitter"
    } else {
        "blocked"
    };

    Ok(GeneratedBindingsSurface {
        version: GENERATED_BINDINGS_SURFACE_VERSION,
        generator: GENERATED_BINDINGS_SURFACE_GENERATOR.to_string(),
        status: status.to_string(),
        bindings_generated: false,
        steinberg_sdk_baseline: STEINBERG_VST3_SDK_BASELINE.to_string(),
        upstream_vst3_crate_baseline: UPSTREAM_VST3_CRATE_BASELINE.to_string(),
        active_backend: binding_backend_name(BINDING_BASELINE.backend).to_string(),
        sdk_dir: root.display().to_string(),
        header_manifest: manifest,
        required_headers,
        missing_headers: missing_symbol_headers,
        missing_symbols,
        symbols,
        blockers,
        notes: vec![
            "this report locks the expected VST3 binding symbol surface for the future generated-header backend and verifies identifier tokens are present in the locked headers".to_string(),
            "it does not parse C++ AST, verify ABI layout, or claim generated bindings are complete".to_string(),
            "bindingsGenerated must remain false until a real emitter generates and validates usable Rust COM bindings".to_string(),
        ],
    })
}

pub fn generated_bindings_surface_differences(
    expected: &GeneratedBindingsSurface,
    actual: &GeneratedBindingsSurface,
) -> Vec<String> {
    let mut differences = Vec::new();
    if expected.version != actual.version {
        differences.push(format!(
            "version expected {} actual {}",
            expected.version, actual.version
        ));
    }
    if expected.generator != actual.generator {
        differences.push(format!(
            "generator expected {} actual {}",
            expected.generator, actual.generator
        ));
    }
    if expected.status != actual.status {
        differences.push(format!(
            "status expected {} actual {}",
            expected.status, actual.status
        ));
    }
    if expected.bindings_generated != actual.bindings_generated {
        differences.push(format!(
            "bindings_generated expected {} actual {}",
            expected.bindings_generated, actual.bindings_generated
        ));
    }
    if expected.steinberg_sdk_baseline != actual.steinberg_sdk_baseline {
        differences.push(format!(
            "Steinberg SDK baseline expected {} actual {}",
            expected.steinberg_sdk_baseline, actual.steinberg_sdk_baseline
        ));
    }
    if expected.upstream_vst3_crate_baseline != actual.upstream_vst3_crate_baseline {
        differences.push(format!(
            "upstream vst3 crate baseline expected {} actual {}",
            expected.upstream_vst3_crate_baseline, actual.upstream_vst3_crate_baseline
        ));
    }
    if expected.active_backend != actual.active_backend {
        differences.push(format!(
            "active backend expected {} actual {}",
            expected.active_backend, actual.active_backend
        ));
    }
    if expected.sdk_dir != actual.sdk_dir {
        differences.push(format!(
            "sdk dir expected {} actual {}",
            expected.sdk_dir, actual.sdk_dir
        ));
    }
    differences.extend(sdk_header_manifest_differences(
        &expected.header_manifest,
        &actual.header_manifest,
    ));
    if expected.required_headers != actual.required_headers {
        differences.push("required headers changed".to_string());
    }
    if expected.missing_headers != actual.missing_headers {
        differences.push(format!(
            "missing surface headers expected {:?} actual {:?}",
            expected.missing_headers, actual.missing_headers
        ));
    }
    if expected.missing_symbols != actual.missing_symbols {
        differences.push(format!(
            "missing surface symbols expected {:?} actual {:?}",
            expected.missing_symbols, actual.missing_symbols
        ));
    }
    if expected.symbols != actual.symbols {
        differences.push("surface symbols changed".to_string());
    }
    if expected.blockers != actual.blockers {
        differences.push(format!(
            "blockers expected {:?} actual {:?}",
            expected.blockers, actual.blockers
        ));
    }
    if expected.notes != actual.notes {
        differences.push("notes changed".to_string());
    }
    differences
}

pub fn generated_bindings_scaffold(
    root: impl AsRef<Path>,
    bindings_module: impl AsRef<Path>,
) -> Result<GeneratedBindingsScaffold, GeneratedBindingsScaffoldError> {
    let root = root.as_ref();
    let plan = generated_bindings_plan(root, bindings_module)?;
    let surface = generated_bindings_surface(root)?;
    if !plan.blockers.is_empty() || plan.status != "ready-for-binding-generator" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: plan.blockers.clone(),
        });
    }
    if !surface.blockers.is_empty() || surface.status != "ready-for-binding-emitter" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: surface.blockers.clone(),
        });
    }
    let module = generated_bindings_scaffold_module(&plan, &surface);
    Ok(GeneratedBindingsScaffold {
        plan,
        surface,
        module,
    })
}

pub fn generated_bindings_abi_seed(
    root: impl AsRef<Path>,
    bindings_module: impl AsRef<Path>,
) -> Result<GeneratedBindingsAbiSeed, GeneratedBindingsScaffoldError> {
    let root = root.as_ref();
    let plan = generated_bindings_plan(root, bindings_module)?;
    let surface = generated_bindings_surface(root)?;
    if !plan.blockers.is_empty() || plan.status != "ready-for-binding-generator" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: plan.blockers.clone(),
        });
    }
    if !surface.blockers.is_empty() || surface.status != "ready-for-binding-emitter" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: surface.blockers.clone(),
        });
    }
    let module = generated_bindings_abi_seed_module(&plan, &surface);
    Ok(GeneratedBindingsAbiSeed {
        plan,
        surface,
        module,
    })
}

pub fn generated_bindings_abi(
    root: impl AsRef<Path>,
    bindings_module: impl AsRef<Path>,
) -> Result<GeneratedBindingsAbi, GeneratedBindingsScaffoldError> {
    let root = root.as_ref();
    let plan = generated_bindings_plan(root, bindings_module)?;
    let surface = generated_bindings_surface(root)?;
    if !plan.blockers.is_empty() || plan.status != "ready-for-binding-generator" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: plan.blockers.clone(),
        });
    }
    if !surface.blockers.is_empty() || surface.status != "ready-for-binding-emitter" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: surface.blockers.clone(),
        });
    }
    let module = generated_bindings_abi_module(&plan, &surface);
    Ok(GeneratedBindingsAbi {
        plan,
        surface,
        module,
    })
}

pub fn generated_bindings_interface_skeleton(
    root: impl AsRef<Path>,
    bindings_module: impl AsRef<Path>,
) -> Result<GeneratedBindingsInterfaceSkeleton, GeneratedBindingsScaffoldError> {
    let root = root.as_ref();
    let plan = generated_bindings_plan(root, bindings_module)?;
    let surface = generated_bindings_surface(root)?;
    if !plan.blockers.is_empty() || plan.status != "ready-for-binding-generator" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: plan.blockers.clone(),
        });
    }
    if !surface.blockers.is_empty() || surface.status != "ready-for-binding-emitter" {
        return Err(GeneratedBindingsScaffoldError::Blocked {
            blockers: surface.blockers.clone(),
        });
    }
    let module = generated_bindings_interface_skeleton_module(&plan, &surface);
    Ok(GeneratedBindingsInterfaceSkeleton {
        plan,
        surface,
        module,
    })
}

pub fn generated_bindings_scaffold_module(
    plan: &GeneratedBindingsPlan,
    surface: &GeneratedBindingsSurface,
) -> String {
    let mut module = String::new();
    module.push_str("// @generated by ");
    module.push_str(GENERATED_BINDINGS_SCAFFOLD_GENERATOR);
    module.push('\n');
    module.push_str("// This is a metadata-only scaffold for Vesty's reserved VST3 SDK generated-bindings backend.\n");
    module.push_str("// It intentionally does not contain Steinberg VST3 COM/API bindings yet.\n");
    module.push_str("#![allow(dead_code)]\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct HeaderInput {\n");
    module.push_str("    pub path: &'static str,\n");
    module.push_str("    pub size: u64,\n");
    module.push_str("    pub sha256: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct BindingSymbol {\n");
    module.push_str("    pub name: &'static str,\n");
    module.push_str("    pub kind: &'static str,\n");
    module.push_str("    pub header: &'static str,\n");
    module.push_str("    pub purpose: &'static str,\n");
    module.push_str("    pub symbol_present: bool,\n");
    module.push_str("}\n\n");
    module.push_str("pub const GENERATOR: &str = ");
    module.push_str(&rust_string_literal(GENERATED_BINDINGS_SCAFFOLD_GENERATOR));
    module.push_str(";\n");
    module.push_str("pub const PLAN_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&plan.generator));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&surface.generator));
    module.push_str(";\n");
    module.push_str("pub const STATUS: &str = \"metadata-scaffold\";\n");
    module.push_str("pub const PLAN_STATUS: &str = ");
    module.push_str(&rust_string_literal(&plan.status));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_STATUS: &str = ");
    module.push_str(&rust_string_literal(&surface.status));
    module.push_str(";\n");
    module.push_str("pub const BINDINGS_GENERATED: bool = false;\n");
    module.push_str("pub const STEINBERG_VST3_SDK_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.steinberg_sdk_baseline));
    module.push_str(";\n");
    module.push_str("pub const UPSTREAM_VST3_CRATE_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.upstream_vst3_crate_baseline));
    module.push_str(";\n");
    module.push_str("pub const ACTIVE_BACKEND: &str = ");
    module.push_str(&rust_string_literal(&plan.active_backend));
    module.push_str(";\n");
    module.push_str("pub const REQUIRED_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.headers.len().to_string());
    module.push_str(";\n");
    module.push_str("pub const MISSING_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.missing_headers.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const SURFACE_SYMBOL_COUNT: usize = ");
    module.push_str(&surface.symbols.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const HEADER_INPUTS: &[HeaderInput] = &[\n");
    for header in &plan.header_manifest.headers {
        module.push_str("    HeaderInput { path: ");
        module.push_str(&rust_string_literal(&header.path));
        module.push_str(", size: ");
        module.push_str(&header.size.to_string());
        module.push_str(", sha256: ");
        module.push_str(&rust_string_literal(&header.sha256));
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const MISSING_HEADERS: &[&str] = &[\n");
    for header in &plan.header_manifest.missing_headers {
        module.push_str("    ");
        module.push_str(&rust_string_literal(header));
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol] = &[\n");
    for symbol in &surface.symbols {
        module.push_str("    BindingSymbol { name: ");
        module.push_str(&rust_string_literal(&symbol.name));
        module.push_str(", kind: ");
        module.push_str(&rust_string_literal(&symbol.kind));
        module.push_str(", header: ");
        module.push_str(&rust_string_literal(&symbol.header));
        module.push_str(", purpose: ");
        module.push_str(&rust_string_literal(&symbol.purpose));
        module.push_str(", symbol_present: ");
        module.push_str(if symbol.symbol_present {
            "true"
        } else {
            "false"
        });
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const fn bindings_are_generated() -> bool {\n");
    module.push_str("    BINDINGS_GENERATED\n");
    module.push_str("}\n");
    module
}

pub fn generated_bindings_abi_seed_module(
    plan: &GeneratedBindingsPlan,
    surface: &GeneratedBindingsSurface,
) -> String {
    let mut module = String::new();
    module.push_str("// @generated by ");
    module.push_str(GENERATED_BINDINGS_ABI_SEED_GENERATOR);
    module.push('\n');
    module.push_str("// This is a deterministic ABI seed for Vesty's reserved VST3 SDK generated-bindings backend.\n");
    module.push_str("// It contains basic ABI aliases/constants only; full Steinberg VST3 COM/API bindings are not generated yet.\n");
    module.push_str("#![allow(dead_code, non_camel_case_types, non_upper_case_globals)]\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct HeaderInput {\n");
    module.push_str("    pub path: &'static str,\n");
    module.push_str("    pub size: u64,\n");
    module.push_str("    pub sha256: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct BindingSymbol {\n");
    module.push_str("    pub name: &'static str,\n");
    module.push_str("    pub kind: &'static str,\n");
    module.push_str("    pub header: &'static str,\n");
    module.push_str("    pub purpose: &'static str,\n");
    module.push_str("    pub symbol_present: bool,\n");
    module.push_str("}\n\n");
    module.push_str("pub const GENERATOR: &str = ");
    module.push_str(&rust_string_literal(GENERATED_BINDINGS_ABI_SEED_GENERATOR));
    module.push_str(";\n");
    module.push_str("pub const PLAN_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&plan.generator));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&surface.generator));
    module.push_str(";\n");
    module.push_str("pub const STATUS: &str = \"abi-seed\";\n");
    module.push_str("pub const PLAN_STATUS: &str = ");
    module.push_str(&rust_string_literal(&plan.status));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_STATUS: &str = ");
    module.push_str(&rust_string_literal(&surface.status));
    module.push_str(";\n");
    module.push_str("pub const ABI_SEED_GENERATED: bool = true;\n");
    module.push_str("pub const BINDINGS_GENERATED: bool = false;\n");
    module.push_str("pub const FULL_COM_BINDINGS_GENERATED: bool = false;\n");
    module.push_str("pub const STEINBERG_VST3_SDK_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.steinberg_sdk_baseline));
    module.push_str(";\n");
    module.push_str("pub const UPSTREAM_VST3_CRATE_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.upstream_vst3_crate_baseline));
    module.push_str(";\n");
    module.push_str("pub const ACTIVE_BACKEND: &str = ");
    module.push_str(&rust_string_literal(&plan.active_backend));
    module.push_str(";\n");
    module.push_str("pub const REQUIRED_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.headers.len().to_string());
    module.push_str(";\n");
    module.push_str("pub const MISSING_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.missing_headers.len().to_string());
    module.push_str(";\n");
    module.push_str("pub const SURFACE_SYMBOL_COUNT: usize = ");
    module.push_str(&surface.symbols.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub type TResult = i32;\n");
    module.push_str("pub type ParamID = u32;\n");
    module.push_str("pub type ParamValue = f64;\n");
    module.push_str("pub type TChar = u16;\n");
    module.push_str("pub type TUID = [std::os::raw::c_char; 16];\n");
    module.push_str("pub type PlatformType = &'static str;\n\n");
    module.push_str("pub const kResultOk: TResult = 0;\n");
    module.push_str("pub const kInvalidArgument: TResult = 2;\n");
    module.push_str("pub const kNotImplemented: TResult = 3;\n");
    module.push_str("pub const kPlatformTypeHWND: PlatformType = \"HWND\";\n");
    module.push_str("pub const kPlatformTypeNSView: PlatformType = \"NSView\";\n");
    module.push_str(
        "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";\n\n",
    );
    module.push_str("pub const HEADER_INPUTS: &[HeaderInput] = &[\n");
    for header in &plan.header_manifest.headers {
        module.push_str("    HeaderInput { path: ");
        module.push_str(&rust_string_literal(&header.path));
        module.push_str(", size: ");
        module.push_str(&header.size.to_string());
        module.push_str(", sha256: ");
        module.push_str(&rust_string_literal(&header.sha256));
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const MISSING_HEADERS: &[&str] = &[\n");
    for header in &plan.header_manifest.missing_headers {
        module.push_str("    ");
        module.push_str(&rust_string_literal(header));
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol] = &[\n");
    for symbol in &surface.symbols {
        module.push_str("    BindingSymbol { name: ");
        module.push_str(&rust_string_literal(&symbol.name));
        module.push_str(", kind: ");
        module.push_str(&rust_string_literal(&symbol.kind));
        module.push_str(", header: ");
        module.push_str(&rust_string_literal(&symbol.header));
        module.push_str(", purpose: ");
        module.push_str(&rust_string_literal(&symbol.purpose));
        module.push_str(", symbol_present: ");
        module.push_str(if symbol.symbol_present {
            "true"
        } else {
            "false"
        });
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const fn full_com_bindings_are_generated() -> bool {\n");
    module.push_str("    FULL_COM_BINDINGS_GENERATED\n");
    module.push_str("}\n");
    module
}

pub fn generated_bindings_abi_module(
    plan: &GeneratedBindingsPlan,
    surface: &GeneratedBindingsSurface,
) -> String {
    let mut module = String::new();
    module.push_str("// @generated by ");
    module.push_str(GENERATED_BINDINGS_ABI_GENERATOR);
    module.push('\n');
    module.push_str("// This is a deterministic foundational ABI layout module for Vesty's reserved VST3 SDK generated-bindings backend.\n");
    module.push_str("// It contains repr(C) base layouts and basic aliases/constants only; full Steinberg VST3 COM/API bindings are not generated yet.\n");
    module.push_str(
        "#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals)]\n\n",
    );
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct HeaderInput {\n");
    module.push_str("    pub path: &'static str,\n");
    module.push_str("    pub size: u64,\n");
    module.push_str("    pub sha256: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct BindingSymbol {\n");
    module.push_str("    pub name: &'static str,\n");
    module.push_str("    pub kind: &'static str,\n");
    module.push_str("    pub header: &'static str,\n");
    module.push_str("    pub purpose: &'static str,\n");
    module.push_str("    pub symbol_present: bool,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct AbiLayoutRecord {\n");
    module.push_str("    pub type_name: &'static str,\n");
    module.push_str("    pub size: usize,\n");
    module.push_str("    pub align: usize,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct AbiFieldOffset {\n");
    module.push_str("    pub owner: &'static str,\n");
    module.push_str("    pub field: &'static str,\n");
    module.push_str("    pub offset: usize,\n");
    module.push_str("}\n\n");
    module.push_str("pub const GENERATOR: &str = ");
    module.push_str(&rust_string_literal(GENERATED_BINDINGS_ABI_GENERATOR));
    module.push_str(";\n");
    module.push_str("pub const PLAN_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&plan.generator));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&surface.generator));
    module.push_str(";\n");
    module.push_str("pub const STATUS: &str = \"abi-layout\";\n");
    module.push_str("pub const PLAN_STATUS: &str = ");
    module.push_str(&rust_string_literal(&plan.status));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_STATUS: &str = ");
    module.push_str(&rust_string_literal(&surface.status));
    module.push_str(";\n");
    module.push_str("pub const ABI_LAYOUT_GENERATED: bool = true;\n");
    module.push_str("pub const BINDINGS_GENERATED: bool = false;\n");
    module.push_str("pub const FULL_COM_BINDINGS_GENERATED: bool = false;\n");
    module.push_str("pub const STEINBERG_VST3_SDK_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.steinberg_sdk_baseline));
    module.push_str(";\n");
    module.push_str("pub const UPSTREAM_VST3_CRATE_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.upstream_vst3_crate_baseline));
    module.push_str(";\n");
    module.push_str("pub const ACTIVE_BACKEND: &str = ");
    module.push_str(&rust_string_literal(&plan.active_backend));
    module.push_str(";\n");
    module.push_str("pub const REQUIRED_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.headers.len().to_string());
    module.push_str(";\n");
    module.push_str("pub const MISSING_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.missing_headers.len().to_string());
    module.push_str(";\n");
    module.push_str("pub const SURFACE_SYMBOL_COUNT: usize = ");
    module.push_str(&surface.symbols.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub type int32 = i32;\n");
    module.push_str("pub type uint32 = u32;\n");
    module.push_str("pub type int16 = i16;\n");
    module.push_str("pub type uint16 = u16;\n");
    module.push_str("pub type int64 = i64;\n");
    module.push_str("pub type uint64 = u64;\n");
    module.push_str("pub type TBool = u8;\n");
    module.push_str("pub type TResult = i32;\n");
    module.push_str("pub type FIDString = *const std::os::raw::c_char;\n");
    module.push_str("pub type FUnknownPtr = *mut FUnknown;\n");
    module.push_str("pub type ParamID = u32;\n");
    module.push_str("pub type ParamValue = f64;\n");
    module.push_str("pub type TChar = u16;\n");
    module.push_str("pub type String128 = [TChar; STRING128_CODE_UNITS];\n");
    module.push_str("pub type UnitID = int32;\n");
    module.push_str("pub type ProgramListID = int32;\n");
    module.push_str("pub type NoteExpressionTypeID = uint32;\n");
    module.push_str("pub type NoteExpressionValue = f64;\n");
    module.push_str("pub type PhysicalUITypeID = uint32;\n");
    module.push_str("pub type Sample32 = f32;\n");
    module.push_str("pub type Sample64 = f64;\n");
    module.push_str("pub type SampleRate = f64;\n");
    module.push_str("pub type PlatformType = &'static str;\n\n");
    module.push_str("pub const TUID_BYTE_LEN: usize = 16;\n");
    module.push_str("pub const STRING128_CODE_UNITS: usize = 128;\n");
    module.push_str("pub const FUNKNOWN_VTABLE_ENTRIES: usize = 3;\n");
    module.push_str("pub const VIEW_RECT_FIELD_COUNT: usize = 4;\n");
    module.push_str("pub const PROGRAM_LIST_INFO_FIELD_COUNT: usize = 3;\n");
    module.push_str("pub const UNIT_INFO_FIELD_COUNT: usize = 4;\n");
    module.push_str("pub const NOTE_EXPRESSION_TYPE_INFO_FIELD_COUNT: usize = 8;\n");
    module.push_str("pub const PHYSICAL_UI_MAP_FIELD_COUNT: usize = 2;\n");
    module.push_str("pub const kResultOk: TResult = 0;\n");
    module.push_str("pub const kInvalidArgument: TResult = 2;\n");
    module.push_str("pub const kNotImplemented: TResult = 3;\n");
    module.push_str("pub const kRootUnitId: UnitID = 0;\n");
    module.push_str("pub const kNoParentUnitId: UnitID = -1;\n");
    module.push_str("pub const kNoProgramListId: ProgramListID = -1;\n");
    module.push_str("pub const kPlatformTypeHWND: PlatformType = \"HWND\";\n");
    module.push_str("pub const kPlatformTypeNSView: PlatformType = \"NSView\";\n");
    module.push_str(
        "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";\n\n",
    );
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct TUID {\n");
    module.push_str("    pub data: [std::os::raw::c_char; TUID_BYTE_LEN],\n");
    module.push_str("}\n\n");
    module.push_str("impl TUID {\n");
    module.push_str("    pub const ZERO: Self = Self { data: [0; TUID_BYTE_LEN] };\n");
    module.push_str("}\n\n");
    module.push_str("pub type FUnknownQueryInterface = unsafe extern \"system\" fn(\n");
    module.push_str("    this: *mut FUnknown,\n");
    module.push_str("    iid: *const TUID,\n");
    module.push_str("    obj: *mut *mut std::ffi::c_void,\n");
    module.push_str(") -> TResult;\n");
    module.push_str(
        "pub type FUnknownAddRef = unsafe extern \"system\" fn(this: *mut FUnknown) -> uint32;\n",
    );
    module.push_str("pub type FUnknownRelease = unsafe extern \"system\" fn(this: *mut FUnknown) -> uint32;\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy)]\n");
    module.push_str("pub struct FUnknownVTable {\n");
    module.push_str("    pub queryInterface: FUnknownQueryInterface,\n");
    module.push_str("    pub addRef: FUnknownAddRef,\n");
    module.push_str("    pub release: FUnknownRelease,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy)]\n");
    module.push_str("pub struct FUnknown {\n");
    module.push_str("    pub vtable: *const FUnknownVTable,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\n");
    module.push_str("pub struct ViewRect {\n");
    module.push_str("    pub left: int32,\n");
    module.push_str("    pub top: int32,\n");
    module.push_str("    pub right: int32,\n");
    module.push_str("    pub bottom: int32,\n");
    module.push_str("}\n\n");
    module.push_str("impl ViewRect {\n");
    module.push_str("    pub const fn width(&self) -> int32 {\n");
    module.push_str("        self.right - self.left\n");
    module.push_str("    }\n\n");
    module.push_str("    pub const fn height(&self) -> int32 {\n");
    module.push_str("        self.bottom - self.top\n");
    module.push_str("    }\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct ProgramListInfo {\n");
    module.push_str("    pub id: ProgramListID,\n");
    module.push_str("    pub name: String128,\n");
    module.push_str("    pub programCount: int32,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct UnitInfo {\n");
    module.push_str("    pub id: UnitID,\n");
    module.push_str("    pub parentUnitId: UnitID,\n");
    module.push_str("    pub name: String128,\n");
    module.push_str("    pub programListId: ProgramListID,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq)]\n");
    module.push_str("pub struct NoteExpressionValueDescription {\n");
    module.push_str("    pub defaultValue: NoteExpressionValue,\n");
    module.push_str("    pub minimum: NoteExpressionValue,\n");
    module.push_str("    pub maximum: NoteExpressionValue,\n");
    module.push_str("    pub stepCount: int32,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq)]\n");
    module.push_str("pub struct NoteExpressionTypeInfo {\n");
    module.push_str("    pub typeId: NoteExpressionTypeID,\n");
    module.push_str("    pub title: String128,\n");
    module.push_str("    pub shortTitle: String128,\n");
    module.push_str("    pub units: String128,\n");
    module.push_str("    pub unitId: int32,\n");
    module.push_str("    pub valueDesc: NoteExpressionValueDescription,\n");
    module.push_str("    pub associatedParameterId: ParamID,\n");
    module.push_str("    pub flags: int32,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct PhysicalUIMap {\n");
    module.push_str("    pub physicalUITypeID: PhysicalUITypeID,\n");
    module.push_str("    pub noteExpressionTypeID: NoteExpressionTypeID,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct PhysicalUIMapList {\n");
    module.push_str("    pub count: uint32,\n");
    module.push_str("    pub map: *mut PhysicalUIMap,\n");
    module.push_str("}\n\n");
    module.push_str("pub const ABI_LAYOUT_TYPES: &[&str] = &[\n");
    for ty in [
        "TUID",
        "FUnknownVTable",
        "FUnknown",
        "ViewRect",
        "ProgramListInfo",
        "UnitInfo",
        "NoteExpressionValueDescription",
        "NoteExpressionTypeInfo",
        "PhysicalUIMap",
        "PhysicalUIMapList",
        "TResult",
        "ParamID",
        "ParamValue",
        "TChar",
        "String128",
        "UnitID",
        "ProgramListID",
        "NoteExpressionTypeID",
        "NoteExpressionValue",
        "PhysicalUITypeID",
        "Sample32",
        "Sample64",
    ] {
        module.push_str("    ");
        module.push_str(&rust_string_literal(ty));
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const ABI_LAYOUT_RECORDS: &[AbiLayoutRecord] = &[\n");
    for ty in [
        "TUID",
        "FUnknownVTable",
        "FUnknown",
        "ViewRect",
        "ProgramListInfo",
        "UnitInfo",
        "NoteExpressionValueDescription",
        "NoteExpressionTypeInfo",
        "PhysicalUIMap",
        "PhysicalUIMapList",
        "TResult",
        "ParamID",
        "ParamValue",
        "TChar",
        "String128",
        "UnitID",
        "ProgramListID",
        "NoteExpressionTypeID",
        "NoteExpressionValue",
        "PhysicalUITypeID",
        "Sample32",
        "Sample64",
    ] {
        module.push_str("    AbiLayoutRecord { type_name: ");
        module.push_str(&rust_string_literal(ty));
        module.push_str(", size: std::mem::size_of::<");
        module.push_str(ty);
        module.push_str(">(), align: std::mem::align_of::<");
        module.push_str(ty);
        module.push_str(">() },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const ABI_FIELD_OFFSETS: &[AbiFieldOffset] = &[\n");
    for (owner, field) in [
        ("TUID", "data"),
        ("FUnknownVTable", "queryInterface"),
        ("FUnknownVTable", "addRef"),
        ("FUnknownVTable", "release"),
        ("FUnknown", "vtable"),
        ("ViewRect", "left"),
        ("ViewRect", "top"),
        ("ViewRect", "right"),
        ("ViewRect", "bottom"),
        ("ProgramListInfo", "id"),
        ("ProgramListInfo", "name"),
        ("ProgramListInfo", "programCount"),
        ("UnitInfo", "id"),
        ("UnitInfo", "parentUnitId"),
        ("UnitInfo", "name"),
        ("UnitInfo", "programListId"),
        ("NoteExpressionValueDescription", "defaultValue"),
        ("NoteExpressionValueDescription", "minimum"),
        ("NoteExpressionValueDescription", "maximum"),
        ("NoteExpressionValueDescription", "stepCount"),
        ("NoteExpressionTypeInfo", "typeId"),
        ("NoteExpressionTypeInfo", "title"),
        ("NoteExpressionTypeInfo", "shortTitle"),
        ("NoteExpressionTypeInfo", "units"),
        ("NoteExpressionTypeInfo", "unitId"),
        ("NoteExpressionTypeInfo", "valueDesc"),
        ("NoteExpressionTypeInfo", "associatedParameterId"),
        ("NoteExpressionTypeInfo", "flags"),
        ("PhysicalUIMap", "physicalUITypeID"),
        ("PhysicalUIMap", "noteExpressionTypeID"),
        ("PhysicalUIMapList", "count"),
        ("PhysicalUIMapList", "map"),
    ] {
        module.push_str("    AbiFieldOffset { owner: ");
        module.push_str(&rust_string_literal(owner));
        module.push_str(", field: ");
        module.push_str(&rust_string_literal(field));
        module.push_str(", offset: std::mem::offset_of!(");
        module.push_str(owner);
        module.push_str(", ");
        module.push_str(field);
        module.push_str(") },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const HEADER_INPUTS: &[HeaderInput] = &[\n");
    for header in &plan.header_manifest.headers {
        module.push_str("    HeaderInput { path: ");
        module.push_str(&rust_string_literal(&header.path));
        module.push_str(", size: ");
        module.push_str(&header.size.to_string());
        module.push_str(", sha256: ");
        module.push_str(&rust_string_literal(&header.sha256));
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const MISSING_HEADERS: &[&str] = &[\n");
    for header in &plan.header_manifest.missing_headers {
        module.push_str("    ");
        module.push_str(&rust_string_literal(header));
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol] = &[\n");
    for symbol in &surface.symbols {
        module.push_str("    BindingSymbol { name: ");
        module.push_str(&rust_string_literal(&symbol.name));
        module.push_str(", kind: ");
        module.push_str(&rust_string_literal(&symbol.kind));
        module.push_str(", header: ");
        module.push_str(&rust_string_literal(&symbol.header));
        module.push_str(", purpose: ");
        module.push_str(&rust_string_literal(&symbol.purpose));
        module.push_str(", symbol_present: ");
        module.push_str(if symbol.symbol_present {
            "true"
        } else {
            "false"
        });
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const fn abi_layout_is_generated() -> bool {\n");
    module.push_str("    ABI_LAYOUT_GENERATED\n");
    module.push_str("}\n\n");
    module.push_str("pub const fn full_com_bindings_are_generated() -> bool {\n");
    module.push_str("    FULL_COM_BINDINGS_GENERATED\n");
    module.push_str("}\n");
    module
}

pub fn generated_bindings_interface_skeleton_module(
    plan: &GeneratedBindingsPlan,
    surface: &GeneratedBindingsSurface,
) -> String {
    let mut module = String::new();
    module.push_str("// @generated by ");
    module.push_str(GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR);
    module.push('\n');
    module.push_str("// This is a deterministic COM interface skeleton for Vesty's reserved VST3 SDK generated-bindings backend.\n");
    module.push_str("// It emits repr(C) interface/vtable callback field layout seeds plus method-surface/slot-order/signature-intent/vtable-slot-seed/callback-type-alias-seed/interface-id/query-interface-plan/com-object-interface exposure/object-identity/object-dispatch/factory-class/binary-export-symbol metadata and pure IID/dispatch lookup helpers,\n");
    module.push_str(
        "// but intentionally omits callable implementations and full COM binding glue.\n",
    );
    module.push_str(
        "#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals)]\n\n",
    );
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct HeaderInput {\n");
    module.push_str("    pub path: &'static str,\n");
    module.push_str("    pub size: u64,\n");
    module.push_str("    pub sha256: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct BindingSymbol {\n");
    module.push_str("    pub name: &'static str,\n");
    module.push_str("    pub kind: &'static str,\n");
    module.push_str("    pub header: &'static str,\n");
    module.push_str("    pub purpose: &'static str,\n");
    module.push_str("    pub symbol_present: bool,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct InterfaceMethod {\n");
    module.push_str("    pub slot: usize,\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub name: &'static str,\n");
    module.push_str("    pub purpose: &'static str,\n");
    module.push_str("    pub realtime: bool,\n");
    module.push_str("    pub signature: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct InterfaceCallbackType {\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub method: &'static str,\n");
    module.push_str("    pub callback_type: &'static str,\n");
    module.push_str("    pub signature: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct InterfaceVTableFieldOffset {\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub field: &'static str,\n");
    module.push_str("    pub callback_type: &'static str,\n");
    module.push_str("    pub offset: usize,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct InterfaceVTableSlot {\n");
    module.push_str("    pub local_slot: usize,\n");
    module.push_str("    pub global_slot: usize,\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub method: &'static str,\n");
    module.push_str("    pub field: &'static str,\n");
    module.push_str("    pub callback_type: &'static str,\n");
    module.push_str("    pub signature: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct InterfaceId {\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub iid_const: &'static str,\n");
    module.push_str("    pub uid_words: [u32; 4],\n");
    module.push_str("    pub source: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct QueryInterfaceEntry {\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub iid_const: &'static str,\n");
    module.push_str("    pub inherits_funknown: bool,\n");
    module.push_str("    pub implementation: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct ComObjectInterface {\n");
    module.push_str("    pub object: &'static str,\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub iid_const: &'static str,\n");
    module.push_str("    pub exposure: &'static str,\n");
    module.push_str("    pub source: &'static str,\n");
    module.push_str("    pub required: bool,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct ComObjectIdentityPlan {\n");
    module.push_str("    pub object: &'static str,\n");
    module.push_str("    pub root_interface: &'static str,\n");
    module.push_str("    pub root_iid_const: &'static str,\n");
    module.push_str("    pub funknown_identity: &'static str,\n");
    module.push_str("    pub refcount_policy: &'static str,\n");
    module.push_str("    pub unknown_iid_result: &'static str,\n");
    module.push_str("    pub null_object_pointer_result: &'static str,\n");
    module.push_str("    pub source: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct ComObjectQueryInterfaceDispatchEntry {\n");
    module.push_str("    pub object: &'static str,\n");
    module.push_str("    pub interface: &'static str,\n");
    module.push_str("    pub iid_const: &'static str,\n");
    module.push_str("    pub root_interface: &'static str,\n");
    module.push_str("    pub returns_same_identity: bool,\n");
    module.push_str("    pub success_result: &'static str,\n");
    module.push_str("    pub add_ref_on_success: bool,\n");
    module.push_str("    pub implementation: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct FactoryExportPlan {\n");
    module.push_str("    pub factory_object: &'static str,\n");
    module.push_str("    pub factory_interface: &'static str,\n");
    module.push_str("    pub factory_iid_const: &'static str,\n");
    module.push_str("    pub class_count: usize,\n");
    module.push_str("    pub count_classes_result: &'static str,\n");
    module.push_str("    pub get_factory_info_source: &'static str,\n");
    module.push_str("    pub source: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct FactoryClassPlan {\n");
    module.push_str("    pub class_kind: &'static str,\n");
    module.push_str("    pub class_index: usize,\n");
    module.push_str("    pub class_object: &'static str,\n");
    module.push_str("    pub root_interface: &'static str,\n");
    module.push_str("    pub root_iid_const: &'static str,\n");
    module.push_str("    pub category: &'static str,\n");
    module.push_str("    pub name_source: &'static str,\n");
    module.push_str("    pub cid_source: &'static str,\n");
    module.push_str("    pub cid_policy: &'static str,\n");
    module.push_str("    pub cardinality: &'static str,\n");
    module.push_str("    pub get_class_info_result: &'static str,\n");
    module.push_str("    pub invalid_class_index_result: &'static str,\n");
    module.push_str("    pub create_instance_object: &'static str,\n");
    module.push_str("    pub create_instance_root_interface: &'static str,\n");
    module.push_str("    pub create_instance_root_iid_const: &'static str,\n");
    module.push_str("    pub unknown_cid_result: &'static str,\n");
    module.push_str("    pub construction_failure_result: &'static str,\n");
    module.push_str("    pub requested_iid_dispatch: &'static str,\n");
    module.push_str("    pub source: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct ModuleExportPlan {\n");
    module.push_str("    pub symbol: &'static str,\n");
    module.push_str("    pub platforms: &'static str,\n");
    module.push_str("    pub signature: &'static str,\n");
    module.push_str("    pub purpose: &'static str,\n");
    module.push_str("    pub implementation: &'static str,\n");
    module.push_str("    pub return_policy: &'static str,\n");
    module.push_str("    pub generated_callable: bool,\n");
    module.push_str("    pub source: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct BinaryExportSymbolPlan {\n");
    module.push_str("    pub platform: &'static str,\n");
    module.push_str("    pub binary_format: &'static str,\n");
    module.push_str("    pub symbol: &'static str,\n");
    module.push_str("    pub tool_symbol: &'static str,\n");
    module.push_str("    pub inspection_tool: &'static str,\n");
    module.push_str("    pub required: bool,\n");
    module.push_str("    pub verified_by_generated_bindings: bool,\n");
    module.push_str("    pub source: &'static str,\n");
    module.push_str("}\n\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct BinaryExportInspectionToolPlan {\n");
    module.push_str("    pub platform: &'static str,\n");
    module.push_str("    pub program: &'static str,\n");
    module.push_str("    pub args: &'static [&'static str],\n");
    module.push_str("}\n\n");
    module.push_str("pub const GENERATOR: &str = ");
    module.push_str(&rust_string_literal(
        GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR,
    ));
    module.push_str(";\n");
    module.push_str("pub const PLAN_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&plan.generator));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_GENERATOR: &str = ");
    module.push_str(&rust_string_literal(&surface.generator));
    module.push_str(";\n");
    module.push_str("pub const STATUS: &str = \"interface-skeleton\";\n");
    module.push_str("pub const PLAN_STATUS: &str = ");
    module.push_str(&rust_string_literal(&plan.status));
    module.push_str(";\n");
    module.push_str("pub const SURFACE_STATUS: &str = ");
    module.push_str(&rust_string_literal(&surface.status));
    module.push_str(";\n");
    module.push_str("pub const INTERFACE_SKELETON_GENERATED: bool = true;\n");
    module.push_str("pub const BINDINGS_GENERATED: bool = false;\n");
    module.push_str("pub const FULL_COM_BINDINGS_GENERATED: bool = false;\n");
    module.push_str("pub const STEINBERG_VST3_SDK_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.steinberg_sdk_baseline));
    module.push_str(";\n");
    module.push_str("pub const UPSTREAM_VST3_CRATE_BASELINE: &str = ");
    module.push_str(&rust_string_literal(&plan.upstream_vst3_crate_baseline));
    module.push_str(";\n");
    module.push_str("pub const ACTIVE_BACKEND: &str = ");
    module.push_str(&rust_string_literal(&plan.active_backend));
    module.push_str(";\n");
    module.push_str("pub const REQUIRED_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.headers.len().to_string());
    module.push_str(";\n");
    module.push_str("pub const MISSING_HEADER_COUNT: usize = ");
    module.push_str(&plan.header_manifest.missing_headers.len().to_string());
    module.push_str(";\n");
    module.push_str("pub const SURFACE_SYMBOL_COUNT: usize = ");
    module.push_str(&surface.symbols.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const INTERFACE_ID_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_INTERFACE_IDS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const QUERY_INTERFACE_ENTRY_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_INTERFACE_IDS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const COM_OBJECT_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_COM_OBJECTS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const COM_OBJECT_INTERFACE_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_COM_OBJECT_INTERFACES.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const COM_OBJECT_IDENTITY_PLAN_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_COM_OBJECT_IDENTITIES.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRY_COUNT: usize = ");
    module.push_str(
        &(GENERATED_BINDINGS_COM_OBJECT_IDENTITIES.len()
            + GENERATED_BINDINGS_COM_OBJECT_INTERFACES.len())
        .to_string(),
    );
    module.push_str(";\n\n");
    module.push_str("pub const FACTORY_EXPORT_PLAN_COUNT: usize = 1;\n\n");
    module.push_str("pub const FACTORY_CLASS_PLAN_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_FACTORY_CLASS_PLANS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const MODULE_EXPORT_PLAN_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_MODULE_EXPORT_PLANS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const BINARY_EXPORT_SYMBOL_PLAN_COUNT: usize = ");
    module.push_str(
        &GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS
            .len()
            .to_string(),
    );
    module.push_str(";\n\n");
    module.push_str("pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = ");
    module.push_str(&BINARY_EXPORT_INSPECTION_TOOL_PLANS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const INTERFACE_METHOD_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_INTERFACE_METHODS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const INTERFACE_VTABLE_SLOT_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_INTERFACE_METHODS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const INTERFACE_VTABLE_FIELD_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_INTERFACE_METHODS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const INTERFACE_VTABLE_FIELD_OFFSET_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_INTERFACE_METHODS.len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const INTERFACE_CALLBACK_TYPE_COUNT: usize = ");
    module.push_str(&GENERATED_BINDINGS_INTERFACE_METHODS.len().to_string());
    module.push_str(";\n\n");
    module
        .push_str("pub const INTERFACE_METHOD_SLOT_SCOPE: &str = \"per-interface-order-audit\";\n");
    module.push_str(
        "pub const INTERFACE_METHOD_SIGNATURE_SCOPE: &str = \"signature-intent-audit\";\n\n",
    );
    module.push_str("pub const INTERFACE_VTABLE_SLOT_SCOPE: &str = \"per-interface-local-vtable-seed-audit\";\n\n");
    module.push_str("pub const INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE: &str = \"com-vtable-global-slot-seed-audit\";\n\n");
    module.push_str("pub const INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE: &str = \"pure-vtable-slot-lookup-seed-audit\";\n\n");
    module.push_str(
        "pub const INTERFACE_VTABLE_FIELD_SCOPE: &str = \"repr-c-vtable-callback-field-layout-seed-audit\";\n\n",
    );
    module.push_str(
        "pub const INTERFACE_VTABLE_FIELD_OFFSET_SCOPE: &str = \"repr-c-vtable-callback-field-offset-fingerprint-audit\";\n\n",
    );
    module.push_str("pub const INTERFACE_VTABLE_FIELD_OFFSET_LOOKUP_SCOPE: &str = \"pure-vtable-field-offset-lookup-seed-audit\";\n\n");
    module.push_str(
        "pub const INTERFACE_CALLBACK_TYPE_SCOPE: &str = \"callback-type-alias-seed-audit\";\n\n",
    );
    module.push_str(
        "pub const INTERFACE_ID_SCOPE: &str = \"upstream-vst3-interface-iid-audit\";\n\n",
    );
    module.push_str(
        "pub const QUERY_INTERFACE_ENTRY_SCOPE: &str = \"query-interface-dispatch-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const QUERY_INTERFACE_IID_LOOKUP_SCOPE: &str = \"pure-iid-dispatch-lookup-seed-audit\";\n\n",
    );
    module.push_str(
        "pub const COM_OBJECT_INTERFACE_SCOPE: &str = \"vesty-com-object-interface-exposure-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const COM_OBJECT_IDENTITY_PLAN_SCOPE: &str = \"vesty-com-object-funknown-identity-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_SCOPE: &str = \"vesty-com-object-query-interface-dispatch-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const FACTORY_EXPORT_PLAN_SCOPE: &str = \"vesty-factory-export-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const FACTORY_CLASS_PLAN_SCOPE: &str = \"vesty-factory-class-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const MODULE_EXPORT_PLAN_SCOPE: &str = \"vesty-module-export-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const BINARY_EXPORT_SYMBOL_PLAN_SCOPE: &str = \"vesty-binary-export-symbol-plan-audit\";\n\n",
    );
    module.push_str(
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE: &str = \"vesty-binary-export-inspection-tool-plan-audit\";\n\n",
    );
    module.push_str("pub const BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED: bool = true;\n");
    module.push_str("pub const BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED: bool = false;\n\n");
    module.push_str("pub type int16 = i16;\n");
    module.push_str("pub type int32 = i32;\n");
    module.push_str("pub type int64 = i64;\n");
    module.push_str("pub type uint32 = u32;\n");
    module.push_str("pub type TResult = i32;\n");
    module.push_str("pub type char16 = u16;\n");
    module.push_str("pub type TBool = u8;\n");
    module.push_str("pub type TChar = u16;\n");
    module.push_str("pub type FIDString = *const std::os::raw::c_char;\n");
    module.push_str("pub type CString = *const std::os::raw::c_char;\n");
    module.push_str("pub type TCharConstPtr = *const TChar;\n");
    module.push_str("pub type AttrID = FIDString;\n");
    module.push_str("pub type ParamID = u32;\n");
    module.push_str("pub type ParamValue = f64;\n");
    module.push_str("pub type CtrlNumber = int16;\n");
    module.push_str("pub type IoMode = int32;\n");
    module.push_str("pub type MediaType = int32;\n");
    module.push_str("pub type BusDirection = int32;\n");
    module.push_str("pub type SpeakerArrangement = u64;\n");
    module.push_str("pub type UnitID = int32;\n");
    module.push_str("pub type ProgramListID = int32;\n");
    module.push_str("pub type NoteExpressionTypeID = uint32;\n");
    module.push_str("pub type NoteExpressionValue = f64;\n");
    module.push_str("pub type PClassInfo = std::ffi::c_void;\n");
    module.push_str("pub type ViewRect = std::ffi::c_void;\n");
    module.push_str("pub type BusInfo = std::ffi::c_void;\n");
    module.push_str("pub type RoutingInfo = std::ffi::c_void;\n");
    module.push_str("pub type ProcessSetup = std::ffi::c_void;\n");
    module.push_str("pub type ProcessData = std::ffi::c_void;\n");
    module.push_str("pub type ParameterInfo = std::ffi::c_void;\n");
    module.push_str("pub type Event = std::ffi::c_void;\n");
    module.push_str("pub type UnitInfo = std::ffi::c_void;\n");
    module.push_str("pub type ProgramListInfo = std::ffi::c_void;\n");
    module.push_str("pub type NoteExpressionTypeInfo = std::ffi::c_void;\n");
    module.push_str("pub type PhysicalUIMapList = std::ffi::c_void;\n");
    module.push_str("pub type String128 = *mut TChar;\n\n");
    module.push_str("pub const TUID_BYTE_LEN: usize = 16;\n");
    module.push_str("pub const FUNKNOWN_VTABLE_ENTRIES: usize = 3;\n");
    module.push_str("pub const kResultOk: TResult = 0;\n");
    module.push_str("pub const kInvalidArgument: TResult = 2;\n");
    module.push_str("pub const kNotImplemented: TResult = 3;\n");
    module.push_str("#[cfg(windows)]\n");
    module.push_str("pub const kNoInterface: TResult = 0x80004002u32 as i32;\n");
    module.push_str("#[cfg(not(windows))]\n");
    module.push_str("pub const kNoInterface: TResult = -1;\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n");
    module.push_str("pub struct TUID {\n");
    module.push_str("    pub data: [std::os::raw::c_char; TUID_BYTE_LEN],\n");
    module.push_str("}\n\n");
    module.push_str("impl TUID {\n");
    module.push_str("    pub const ZERO: Self = Self { data: [0; TUID_BYTE_LEN] };\n");
    module.push_str("}\n\n");
    module.push_str("pub const fn iid_from_words(a: u32, b: u32, c: u32, d: u32) -> TUID {\n");
    module.push_str("    #[cfg(target_os = \"windows\")]\n");
    module.push_str("    let data = [\n");
    module.push_str("        ((a & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("        ((a & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((a & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((a & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("    ];\n");
    module.push_str("    #[cfg(not(target_os = \"windows\"))]\n");
    module.push_str("    let data = [\n");
    module.push_str("        ((a & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((a & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((a & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((a & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((b & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((c & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0xFF000000) >> 24) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0x00FF0000) >> 16) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0x0000FF00) >> 8) as std::os::raw::c_char,\n");
    module.push_str("        ((d & 0x000000FF) >> 0) as std::os::raw::c_char,\n");
    module.push_str("    ];\n");
    module.push_str("    TUID { data }\n");
    module.push_str("}\n\n");
    for spec in GENERATED_BINDINGS_INTERFACE_IDS {
        push_interface_iid_const(&mut module, spec);
    }
    module.push('\n');
    module.push_str("pub type FUnknownQueryInterface = unsafe extern \"system\" fn(\n");
    module.push_str("    this: *mut FUnknown,\n");
    module.push_str("    iid: *const TUID,\n");
    module.push_str("    obj: *mut *mut std::ffi::c_void,\n");
    module.push_str(") -> TResult;\n");
    module.push_str(
        "pub type FUnknownAddRef = unsafe extern \"system\" fn(this: *mut FUnknown) -> uint32;\n",
    );
    module.push_str("pub type FUnknownRelease = unsafe extern \"system\" fn(this: *mut FUnknown) -> uint32;\n\n");
    module.push_str(
        "// Callback type aliases are audit seeds for the future generated-headers emitter.\n",
    );
    module.push_str("// VTable structs below intentionally contain callback field layout seeds but no callback implementations.\n");
    for method in GENERATED_BINDINGS_INTERFACE_METHODS
        .iter()
        .filter(|method| method.interface != "FUnknown")
    {
        push_interface_callback_type_alias(&mut module, method);
    }
    module.push('\n');
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy)]\n");
    module.push_str("pub struct FUnknownVTable {\n");
    module.push_str("    pub queryInterface: FUnknownQueryInterface,\n");
    module.push_str("    pub addRef: FUnknownAddRef,\n");
    module.push_str("    pub release: FUnknownRelease,\n");
    module.push_str("}\n\n");
    module.push_str("#[repr(C)]\n");
    module.push_str("#[derive(Clone, Copy)]\n");
    module.push_str("pub struct FUnknown {\n");
    module.push_str("    pub vtable: *const FUnknownVTable,\n");
    module.push_str("}\n\n");

    for symbol in surface
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == "interface" && symbol.name != "FUnknown")
    {
        module.push_str("#[repr(C)]\n");
        module.push_str("#[derive(Clone, Copy)]\n");
        module.push_str("pub struct ");
        module.push_str(&symbol.name);
        module.push_str("VTable {\n");
        module.push_str("    pub unknown: FUnknownVTable,\n");
        let methods = interface_methods(symbol.name.as_str());
        for method in &methods {
            push_interface_vtable_field(&mut module, method);
        }
        module.push_str("}\n\n");
        module.push_str("pub const ");
        module.push_str(&interface_methods_const_name(symbol.name.as_str()));
        module.push_str(": &[InterfaceMethod] = &[\n");
        for (slot, method) in methods.iter().enumerate() {
            push_interface_method_record(&mut module, method, slot);
        }
        module.push_str("];\n\n");
        module.push_str("pub const ");
        module.push_str(&interface_vtable_slots_const_name(symbol.name.as_str()));
        module.push_str(": &[InterfaceVTableSlot] = &[\n");
        for (slot, method) in methods.iter().enumerate() {
            push_interface_vtable_slot_record(&mut module, method, slot);
        }
        module.push_str("];\n\n");
        module.push_str("pub const ");
        module.push_str(&interface_callback_types_const_name(symbol.name.as_str()));
        module.push_str(": &[InterfaceCallbackType] = &[\n");
        for method in &methods {
            push_interface_callback_type_record(&mut module, method);
        }
        module.push_str("];\n\n");
        module.push_str("pub const ");
        module.push_str(&interface_vtable_field_offsets_const_name(
            symbol.name.as_str(),
        ));
        module.push_str(": &[InterfaceVTableFieldOffset] = &[\n");
        for method in &methods {
            push_interface_vtable_field_offset_record(&mut module, method);
        }
        module.push_str("];\n\n");
        module.push_str("pub const ");
        module.push_str(&interface_method_count_const_name(symbol.name.as_str()));
        module.push_str(": usize = ");
        module.push_str(&methods.len().to_string());
        module.push_str(";\n\n");
        module.push_str("#[repr(C)]\n");
        module.push_str("#[derive(Clone, Copy)]\n");
        module.push_str("pub struct ");
        module.push_str(&symbol.name);
        module.push_str(" {\n");
        module.push_str("    pub vtable: *const ");
        module.push_str(&symbol.name);
        module.push_str("VTable,\n");
        module.push_str("}\n\n");
    }

    module.push_str("pub const INTERFACE_SKELETON_TYPES: &[&str] = &[\n");
    module.push_str("    \"FUnknown\",\n");
    for symbol in surface
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == "interface" && symbol.name != "FUnknown")
    {
        module.push_str("    ");
        module.push_str(&rust_string_literal(&symbol.name));
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const FUNKNOWN_METHODS: &[InterfaceMethod] = &[\n");
    for (slot, method) in interface_methods("FUnknown").iter().enumerate() {
        push_interface_method_record(&mut module, method, slot);
    }
    module.push_str("];\n\n");
    module.push_str("pub const FUNKNOWN_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[\n");
    for (slot, method) in interface_methods("FUnknown").iter().enumerate() {
        push_interface_vtable_slot_record(&mut module, method, slot);
    }
    module.push_str("];\n\n");
    module.push_str("pub const FUNKNOWN_CALLBACK_TYPES: &[InterfaceCallbackType] = &[\n");
    for method in interface_methods("FUnknown") {
        push_interface_callback_type_record(&mut module, method);
    }
    module.push_str("];\n\n");
    module
        .push_str("pub const FUNKNOWN_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[\n");
    for method in interface_methods("FUnknown") {
        push_interface_vtable_field_offset_record(&mut module, method);
    }
    module.push_str("];\n\n");
    module.push_str("pub const FUNKNOWN_METHOD_COUNT: usize = ");
    module.push_str(&interface_methods("FUnknown").len().to_string());
    module.push_str(";\n\n");
    module.push_str("pub const INTERFACE_IDS: &[InterfaceId] = &[\n");
    for spec in GENERATED_BINDINGS_INTERFACE_IDS {
        push_interface_id_record(&mut module, spec);
    }
    module.push_str("];\n\n");
    module.push_str("pub const QUERY_INTERFACE_ENTRIES: &[QueryInterfaceEntry] = &[\n");
    for spec in GENERATED_BINDINGS_INTERFACE_IDS {
        push_query_interface_entry_record(&mut module, spec);
    }
    module.push_str("];\n\n");
    module.push_str("pub fn interface_id_for_iid(iid: &TUID) -> Option<&'static InterfaceId> {\n");
    module.push_str("    for entry in INTERFACE_IDS {\n");
    module.push_str("        let candidate = iid_from_words(\n");
    module.push_str("            entry.uid_words[0],\n");
    module.push_str("            entry.uid_words[1],\n");
    module.push_str("            entry.uid_words[2],\n");
    module.push_str("            entry.uid_words[3],\n");
    module.push_str("        );\n");
    module.push_str("        if candidate.data == iid.data {\n");
    module.push_str("            return Some(entry);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str(
        "pub fn query_interface_entry_by_interface(interface: &str) -> Option<&'static QueryInterfaceEntry> {\n",
    );
    module.push_str("    for entry in QUERY_INTERFACE_ENTRIES {\n");
    module.push_str("        if entry.interface == interface {\n");
    module.push_str("            return Some(entry);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str(
        "pub fn query_interface_entry_for_iid(iid: &TUID) -> Option<&'static QueryInterfaceEntry> {\n",
    );
    module.push_str("    let interface_id = interface_id_for_iid(iid)?;\n");
    module.push_str("    query_interface_entry_by_interface(interface_id.interface)\n");
    module.push_str("}\n\n");
    module.push_str("pub const COM_OBJECTS: &[&str] = &[\n");
    for object in GENERATED_BINDINGS_COM_OBJECTS {
        module.push_str("    ");
        module.push_str(&rust_string_literal(object));
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    for object in GENERATED_BINDINGS_COM_OBJECTS {
        module.push_str("pub const ");
        module.push_str(&com_object_interfaces_const_name(object));
        module.push_str(": &[ComObjectInterface] = &[\n");
        for spec in GENERATED_BINDINGS_COM_OBJECT_INTERFACES
            .iter()
            .filter(|spec| spec.object == *object)
        {
            push_com_object_interface_record(&mut module, spec);
        }
        module.push_str("];\n\n");
        if let Some(identity) = com_object_identity_spec(object) {
            module.push_str("pub const ");
            module.push_str(&com_object_identity_plan_const_name(object));
            module.push_str(": ComObjectIdentityPlan = ");
            push_com_object_identity_plan_record(&mut module, identity);
            module.push_str(";\n\n");
            module.push_str("pub const ");
            module.push_str(&com_object_query_interface_dispatch_const_name(object));
            module.push_str(": &[ComObjectQueryInterfaceDispatchEntry] = &[\n");
            push_com_object_query_interface_dispatch_entry_record(
                &mut module,
                object,
                "FUnknown",
                identity.root_interface,
            );
            for spec in GENERATED_BINDINGS_COM_OBJECT_INTERFACES
                .iter()
                .filter(|spec| spec.object == *object)
            {
                push_com_object_query_interface_dispatch_entry_record(
                    &mut module,
                    spec.object,
                    spec.interface,
                    identity.root_interface,
                );
            }
            module.push_str("];\n\n");
        }
    }
    module.push_str("pub const COM_OBJECT_INTERFACES: &[ComObjectInterface] = &[\n");
    for spec in GENERATED_BINDINGS_COM_OBJECT_INTERFACES {
        push_com_object_interface_record(&mut module, spec);
    }
    module.push_str("];\n\n");
    module.push_str("pub const COM_OBJECT_IDENTITY_PLANS: &[ComObjectIdentityPlan] = &[\n");
    for identity in GENERATED_BINDINGS_COM_OBJECT_IDENTITIES {
        module.push_str("    ");
        push_com_object_identity_plan_record(&mut module, identity);
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES: &[ComObjectQueryInterfaceDispatchEntry] = &[\n");
    for identity in GENERATED_BINDINGS_COM_OBJECT_IDENTITIES {
        push_com_object_query_interface_dispatch_entry_record(
            &mut module,
            identity.object,
            "FUnknown",
            identity.root_interface,
        );
    }
    for spec in GENERATED_BINDINGS_COM_OBJECT_INTERFACES {
        if let Some(identity) = com_object_identity_spec(spec.object) {
            push_com_object_query_interface_dispatch_entry_record(
                &mut module,
                spec.object,
                spec.interface,
                identity.root_interface,
            );
        }
    }
    module.push_str("];\n\n");
    module.push_str("pub fn com_object_query_interface_dispatch_by_interface(\n");
    module.push_str("    object: &str,\n");
    module.push_str("    interface: &str,\n");
    module.push_str(") -> Option<&'static ComObjectQueryInterfaceDispatchEntry> {\n");
    module.push_str("    for entry in COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES {\n");
    module.push_str("        if entry.object == object && entry.interface == interface {\n");
    module.push_str("            return Some(entry);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str("pub fn com_object_query_interface_dispatch_for_iid(\n");
    module.push_str("    object: &str,\n");
    module.push_str("    iid: &TUID,\n");
    module.push_str(") -> Option<&'static ComObjectQueryInterfaceDispatchEntry> {\n");
    module.push_str("    let interface_id = interface_id_for_iid(iid)?;\n");
    module.push_str(
        "    com_object_query_interface_dispatch_by_interface(object, interface_id.interface)\n",
    );
    module.push_str("}\n\n");
    module.push_str("pub const FACTORY_EXPORT_PLAN: FactoryExportPlan = ");
    push_factory_export_plan_record(&mut module, &GENERATED_BINDINGS_FACTORY_EXPORT_PLAN);
    module.push_str(";\n\n");
    for spec in GENERATED_BINDINGS_FACTORY_CLASS_PLANS {
        module.push_str("pub const ");
        module.push_str(&factory_class_plan_const_name(spec.class_object));
        module.push_str(": FactoryClassPlan = ");
        push_factory_class_plan_record(&mut module, spec);
        module.push_str(";\n\n");
    }
    module.push_str("pub const FACTORY_CLASS_PLANS: &[FactoryClassPlan] = &[\n");
    for spec in GENERATED_BINDINGS_FACTORY_CLASS_PLANS {
        module.push_str("    ");
        push_factory_class_plan_record(&mut module, spec);
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    for spec in GENERATED_BINDINGS_MODULE_EXPORT_PLANS {
        module.push_str("pub const ");
        module.push_str(spec.const_name);
        module.push_str(": ModuleExportPlan = ");
        push_module_export_plan_record(&mut module, spec);
        module.push_str(";\n\n");
    }
    module.push_str("pub const MODULE_EXPORT_PLANS: &[ModuleExportPlan] = &[\n");
    for spec in GENERATED_BINDINGS_MODULE_EXPORT_PLANS {
        module.push_str("    ");
        push_module_export_plan_record(&mut module, spec);
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    for spec in GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS {
        module.push_str("pub const ");
        module.push_str(spec.const_name);
        module.push_str(": BinaryExportSymbolPlan = ");
        push_binary_export_symbol_plan_record(&mut module, spec);
        module.push_str(";\n\n");
    }
    module.push_str("pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[\n");
    for spec in GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS {
        module.push_str("    ");
        push_binary_export_symbol_plan_record(&mut module, spec);
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str(
        "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &[\n",
    );
    for tool in BINARY_EXPORT_INSPECTION_TOOL_PLANS {
        push_binary_export_inspection_tool_plan_record(&mut module, tool);
    }
    module.push_str("];\n\n");
    module.push_str("pub fn binary_export_inspection_tools(\n");
    module.push_str("    platform: &str,\n");
    module.push_str(") -> &'static [BinaryExportInspectionToolPlan] {\n");
    module.push_str("    let mut start = None;\n");
    module.push_str("    let mut end = 0;\n");
    module.push_str("    let mut index = 0;\n");
    module.push_str("    while index < BINARY_EXPORT_INSPECTION_TOOL_PLANS.len() {\n");
    module.push_str("        let tool = &BINARY_EXPORT_INSPECTION_TOOL_PLANS[index];\n");
    module.push_str("        if tool.platform == platform {\n");
    module.push_str("            if start.is_none() {\n");
    module.push_str("                start = Some(index);\n");
    module.push_str("            }\n");
    module.push_str("            end = index + 1;\n");
    module.push_str("        } else if start.is_some() {\n");
    module.push_str("            break;\n");
    module.push_str("        }\n");
    module.push_str("        index += 1;\n");
    module.push_str("    }\n");
    module.push_str("    match start {\n");
    module.push_str("        Some(start) => &BINARY_EXPORT_INSPECTION_TOOL_PLANS[start..end],\n");
    module.push_str("        None => &[],\n");
    module.push_str("    }\n");
    module.push_str("}\n\n");
    module.push_str("pub fn binary_export_symbol_plan_by_platform_and_symbol(\n");
    module.push_str("    platform: &str,\n");
    module.push_str("    tool_symbol: &str,\n");
    module.push_str(") -> Option<&'static BinaryExportSymbolPlan> {\n");
    module.push_str("    for plan in BINARY_EXPORT_SYMBOL_PLANS {\n");
    module.push_str("        if plan.platform == platform && plan.tool_symbol == tool_symbol {\n");
    module.push_str("            return Some(plan);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str("pub fn required_binary_export_symbol_count(platform: &str) -> usize {\n");
    module.push_str("    BINARY_EXPORT_SYMBOL_PLANS\n");
    module.push_str("        .iter()\n");
    module.push_str("        .filter(|plan| plan.platform == platform && plan.required)\n");
    module.push_str("        .count()\n");
    module.push_str("}\n\n");
    module.push_str("pub fn first_missing_binary_export_symbol(\n");
    module.push_str("    platform: &str,\n");
    module.push_str("    found_symbols: &[&str],\n");
    module.push_str(") -> Option<&'static str> {\n");
    module.push_str("    for plan in BINARY_EXPORT_SYMBOL_PLANS {\n");
    module.push_str("        if plan.platform == platform\n");
    module.push_str("            && plan.required\n");
    module
        .push_str("            && !found_symbols.iter().any(|found| *found == plan.tool_symbol)\n");
    module.push_str("        {\n");
    module.push_str("            return Some(plan.tool_symbol);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str("pub fn binary_export_required_symbols_present(\n");
    module.push_str("    platform: &str,\n");
    module.push_str("    found_symbols: &[&str],\n");
    module.push_str(") -> bool {\n");
    module.push_str("    required_binary_export_symbol_count(platform) > 0\n");
    module.push_str(
        "        && first_missing_binary_export_symbol(platform, found_symbols).is_none()\n",
    );
    module.push_str("}\n\n");
    module.push_str("pub const INTERFACE_METHODS: &[InterfaceMethod] = &[\n");
    for method in GENERATED_BINDINGS_INTERFACE_METHODS {
        push_interface_method_record(&mut module, method, interface_method_slot(method));
    }
    module.push_str("];\n\n");
    module.push_str("pub const INTERFACE_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[\n");
    for method in GENERATED_BINDINGS_INTERFACE_METHODS {
        push_interface_vtable_slot_record(&mut module, method, interface_method_slot(method));
    }
    module.push_str("];\n\n");
    module.push_str("pub fn interface_vtable_slot_by_interface_and_method(\n");
    module.push_str("    interface: &str,\n");
    module.push_str("    method: &str,\n");
    module.push_str(") -> Option<&'static InterfaceVTableSlot> {\n");
    module.push_str("    for slot in INTERFACE_VTABLE_SLOTS {\n");
    module.push_str("        if slot.interface == interface && slot.method == method {\n");
    module.push_str("            return Some(slot);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str("pub fn interface_vtable_slot_by_interface_and_global_slot(\n");
    module.push_str("    interface: &str,\n");
    module.push_str("    global_slot: usize,\n");
    module.push_str(") -> Option<&'static InterfaceVTableSlot> {\n");
    module.push_str("    for slot in INTERFACE_VTABLE_SLOTS {\n");
    module
        .push_str("        if slot.interface == interface && slot.global_slot == global_slot {\n");
    module.push_str("            return Some(slot);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str("pub const INTERFACE_CALLBACK_TYPES: &[InterfaceCallbackType] = &[\n");
    for method in GENERATED_BINDINGS_INTERFACE_METHODS {
        push_interface_callback_type_record(&mut module, method);
    }
    module.push_str("];\n\n");
    module
        .push_str("pub const INTERFACE_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &[\n");
    for method in GENERATED_BINDINGS_INTERFACE_METHODS {
        push_interface_vtable_field_offset_record(&mut module, method);
    }
    module.push_str("];\n\n");
    module.push_str("pub fn interface_vtable_field_offset_by_interface_and_field(\n");
    module.push_str("    interface: &str,\n");
    module.push_str("    field: &str,\n");
    module.push_str(") -> Option<&'static InterfaceVTableFieldOffset> {\n");
    module.push_str("    for offset in INTERFACE_VTABLE_FIELD_OFFSETS {\n");
    module.push_str("        if offset.interface == interface && offset.field == field {\n");
    module.push_str("            return Some(offset);\n");
    module.push_str("        }\n");
    module.push_str("    }\n");
    module.push_str("    None\n");
    module.push_str("}\n\n");
    module.push_str("pub const HEADER_INPUTS: &[HeaderInput] = &[\n");
    for header in &plan.header_manifest.headers {
        module.push_str("    HeaderInput { path: ");
        module.push_str(&rust_string_literal(&header.path));
        module.push_str(", size: ");
        module.push_str(&header.size.to_string());
        module.push_str(", sha256: ");
        module.push_str(&rust_string_literal(&header.sha256));
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const MISSING_HEADERS: &[&str] = &[\n");
    for header in &plan.header_manifest.missing_headers {
        module.push_str("    ");
        module.push_str(&rust_string_literal(header));
        module.push_str(",\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol] = &[\n");
    for symbol in &surface.symbols {
        module.push_str("    BindingSymbol { name: ");
        module.push_str(&rust_string_literal(&symbol.name));
        module.push_str(", kind: ");
        module.push_str(&rust_string_literal(&symbol.kind));
        module.push_str(", header: ");
        module.push_str(&rust_string_literal(&symbol.header));
        module.push_str(", purpose: ");
        module.push_str(&rust_string_literal(&symbol.purpose));
        module.push_str(", symbol_present: ");
        module.push_str(if symbol.symbol_present {
            "true"
        } else {
            "false"
        });
        module.push_str(" },\n");
    }
    module.push_str("];\n\n");
    module.push_str("pub const fn interface_skeleton_is_generated() -> bool {\n");
    module.push_str("    INTERFACE_SKELETON_GENERATED\n");
    module.push_str("}\n\n");
    module.push_str("pub const fn full_com_bindings_are_generated() -> bool {\n");
    module.push_str("    FULL_COM_BINDINGS_GENERATED\n");
    module.push_str("}\n");
    module
}

fn push_interface_method_record(
    module: &mut String,
    method: &BindingInterfaceMethodSpec,
    slot: usize,
) {
    let signature = interface_method_signature_intent(method);
    module.push_str("    InterfaceMethod { slot: ");
    module.push_str(&slot.to_string());
    module.push_str(", interface: ");
    module.push_str(&rust_string_literal(method.interface));
    module.push_str(", name: ");
    module.push_str(&rust_string_literal(method.name));
    module.push_str(", purpose: ");
    module.push_str(&rust_string_literal(method.purpose));
    module.push_str(", realtime: ");
    module.push_str(if method.realtime { "true" } else { "false" });
    module.push_str(", signature: ");
    module.push_str(&rust_string_literal(&signature));
    module.push_str(" },\n");
}

fn push_interface_vtable_slot_record(
    module: &mut String,
    method: &BindingInterfaceMethodSpec,
    slot: usize,
) {
    let signature = interface_method_signature_intent(method);
    module.push_str("    InterfaceVTableSlot { local_slot: ");
    module.push_str(&slot.to_string());
    module.push_str(", global_slot: ");
    module.push_str(&interface_method_global_slot(method, slot).to_string());
    module.push_str(", interface: ");
    module.push_str(&rust_string_literal(method.interface));
    module.push_str(", method: ");
    module.push_str(&rust_string_literal(method.name));
    module.push_str(", field: ");
    module.push_str(&rust_string_literal(method.name));
    module.push_str(", callback_type: ");
    module.push_str(&rust_string_literal(&interface_method_callback_type_name(
        method,
    )));
    module.push_str(", signature: ");
    module.push_str(&rust_string_literal(&signature));
    module.push_str(" },\n");
}

fn push_interface_vtable_field(module: &mut String, method: &BindingInterfaceMethodSpec) {
    module.push_str("    pub ");
    module.push_str(method.name);
    module.push_str(": ");
    module.push_str(&interface_method_callback_type_name(method));
    module.push_str(",\n");
}

fn push_interface_callback_type_alias(module: &mut String, method: &BindingInterfaceMethodSpec) {
    module.push_str("pub type ");
    module.push_str(&interface_method_callback_type_name(method));
    module.push_str(" = ");
    module.push_str(&interface_method_signature_intent(method));
    module.push_str(";\n");
}

fn push_interface_callback_type_record(module: &mut String, method: &BindingInterfaceMethodSpec) {
    let signature = interface_method_signature_intent(method);
    module.push_str("    InterfaceCallbackType { interface: ");
    module.push_str(&rust_string_literal(method.interface));
    module.push_str(", method: ");
    module.push_str(&rust_string_literal(method.name));
    module.push_str(", callback_type: ");
    module.push_str(&rust_string_literal(&interface_method_callback_type_name(
        method,
    )));
    module.push_str(", signature: ");
    module.push_str(&rust_string_literal(&signature));
    module.push_str(" },\n");
}

fn push_interface_vtable_field_offset_record(
    module: &mut String,
    method: &BindingInterfaceMethodSpec,
) {
    let callback_type = interface_method_callback_type_name(method);
    let vtable_type = format!("{}VTable", method.interface);
    module.push_str("    InterfaceVTableFieldOffset { interface: ");
    module.push_str(&rust_string_literal(method.interface));
    module.push_str(", field: ");
    module.push_str(&rust_string_literal(method.name));
    module.push_str(", callback_type: ");
    module.push_str(&rust_string_literal(&callback_type));
    module.push_str(", offset: std::mem::offset_of!(");
    module.push_str(&vtable_type);
    module.push_str(", ");
    module.push_str(method.name);
    module.push_str(") },\n");
}

fn push_interface_iid_const(module: &mut String, spec: &BindingInterfaceIdSpec) {
    module.push_str("pub const ");
    module.push_str(&interface_iid_const_name(spec.interface));
    module.push_str(": TUID = iid_from_words(");
    for (index, word) in spec.uid_words.iter().enumerate() {
        if index > 0 {
            module.push_str(", ");
        }
        module.push_str(&format!("0x{word:08X}"));
    }
    module.push_str(");\n");
}

fn push_interface_id_record(module: &mut String, spec: &BindingInterfaceIdSpec) {
    module.push_str("    InterfaceId { interface: ");
    module.push_str(&rust_string_literal(spec.interface));
    module.push_str(", iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(
        spec.interface,
    )));
    module.push_str(", uid_words: [");
    for (index, word) in spec.uid_words.iter().enumerate() {
        if index > 0 {
            module.push_str(", ");
        }
        module.push_str(&format!("0x{word:08X}"));
    }
    module.push_str("], source: ");
    module.push_str(&rust_string_literal(spec.source));
    module.push_str(" },\n");
}

fn push_query_interface_entry_record(module: &mut String, spec: &BindingInterfaceIdSpec) {
    module.push_str("    QueryInterfaceEntry { interface: ");
    module.push_str(&rust_string_literal(spec.interface));
    module.push_str(", iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(
        spec.interface,
    )));
    module.push_str(", inherits_funknown: ");
    module.push_str(if spec.interface == "FUnknown" {
        "false"
    } else {
        "true"
    });
    module.push_str(", implementation: \"planned-dispatch-entry-no-callable-glue\" },\n");
}

fn push_com_object_interface_record(module: &mut String, spec: &BindingComObjectInterfaceSpec) {
    module.push_str("    ComObjectInterface { object: ");
    module.push_str(&rust_string_literal(spec.object));
    module.push_str(", interface: ");
    module.push_str(&rust_string_literal(spec.interface));
    module.push_str(", iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(
        spec.interface,
    )));
    module.push_str(", exposure: ");
    module.push_str(&rust_string_literal(spec.exposure));
    module.push_str(", source: ");
    module.push_str(&rust_string_literal(spec.source));
    module.push_str(", required: ");
    module.push_str(if spec.required { "true" } else { "false" });
    module.push_str(" },\n");
}

fn push_com_object_identity_plan_record(module: &mut String, spec: &BindingComObjectIdentitySpec) {
    module.push_str("ComObjectIdentityPlan { object: ");
    module.push_str(&rust_string_literal(spec.object));
    module.push_str(", root_interface: ");
    module.push_str(&rust_string_literal(spec.root_interface));
    module.push_str(", root_iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(
        spec.root_interface,
    )));
    module.push_str(", funknown_identity: ");
    module.push_str(&rust_string_literal(spec.funknown_identity));
    module.push_str(", refcount_policy: ");
    module.push_str(&rust_string_literal(spec.refcount_policy));
    module.push_str(", unknown_iid_result: ");
    module.push_str(&rust_string_literal(spec.unknown_iid_result));
    module.push_str(", null_object_pointer_result: ");
    module.push_str(&rust_string_literal(spec.null_object_pointer_result));
    module.push_str(", source: ");
    module.push_str(&rust_string_literal(spec.source));
    module.push_str(" }");
}

fn push_com_object_query_interface_dispatch_entry_record(
    module: &mut String,
    object: &str,
    interface: &str,
    root_interface: &str,
) {
    module.push_str("    ComObjectQueryInterfaceDispatchEntry { object: ");
    module.push_str(&rust_string_literal(object));
    module.push_str(", interface: ");
    module.push_str(&rust_string_literal(interface));
    module.push_str(", iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(interface)));
    module.push_str(", root_interface: ");
    module.push_str(&rust_string_literal(root_interface));
    module.push_str(", returns_same_identity: true");
    module.push_str(", success_result: \"kResultOk\"");
    module.push_str(", add_ref_on_success: true");
    module.push_str(
        ", implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" },\n",
    );
}

fn push_factory_export_plan_record(module: &mut String, spec: &BindingFactoryExportPlanSpec) {
    module.push_str("FactoryExportPlan { factory_object: ");
    module.push_str(&rust_string_literal(spec.factory_object));
    module.push_str(", factory_interface: ");
    module.push_str(&rust_string_literal(spec.factory_interface));
    module.push_str(", factory_iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(
        spec.factory_interface,
    )));
    module.push_str(", class_count: ");
    module.push_str(&spec.class_count.to_string());
    module.push_str(", count_classes_result: ");
    module.push_str(&rust_string_literal(spec.count_classes_result));
    module.push_str(", get_factory_info_source: ");
    module.push_str(&rust_string_literal(spec.get_factory_info_source));
    module.push_str(", source: ");
    module.push_str(&rust_string_literal(spec.source));
    module.push_str(" }");
}

fn push_factory_class_plan_record(module: &mut String, spec: &BindingFactoryClassPlanSpec) {
    module.push_str("FactoryClassPlan { class_kind: ");
    module.push_str(&rust_string_literal(spec.class_kind));
    module.push_str(", class_index: ");
    module.push_str(&spec.class_index.to_string());
    module.push_str(", class_object: ");
    module.push_str(&rust_string_literal(spec.class_object));
    module.push_str(", root_interface: ");
    module.push_str(&rust_string_literal(spec.root_interface));
    module.push_str(", root_iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(
        spec.root_interface,
    )));
    module.push_str(", category: ");
    module.push_str(&rust_string_literal(spec.category));
    module.push_str(", name_source: ");
    module.push_str(&rust_string_literal(spec.name_source));
    module.push_str(", cid_source: ");
    module.push_str(&rust_string_literal(spec.cid_source));
    module.push_str(", cid_policy: ");
    module.push_str(&rust_string_literal(spec.cid_policy));
    module.push_str(", cardinality: ");
    module.push_str(&rust_string_literal(spec.cardinality));
    module.push_str(", get_class_info_result: ");
    module.push_str(&rust_string_literal(spec.get_class_info_result));
    module.push_str(", invalid_class_index_result: ");
    module.push_str(&rust_string_literal(spec.invalid_class_index_result));
    module.push_str(", create_instance_object: ");
    module.push_str(&rust_string_literal(spec.create_instance_object));
    module.push_str(", create_instance_root_interface: ");
    module.push_str(&rust_string_literal(spec.create_instance_root_interface));
    module.push_str(", create_instance_root_iid_const: ");
    module.push_str(&rust_string_literal(&interface_iid_const_name(
        spec.create_instance_root_interface,
    )));
    module.push_str(", unknown_cid_result: ");
    module.push_str(&rust_string_literal(spec.unknown_cid_result));
    module.push_str(", construction_failure_result: ");
    module.push_str(&rust_string_literal(spec.construction_failure_result));
    module.push_str(", requested_iid_dispatch: ");
    module.push_str(&rust_string_literal(spec.requested_iid_dispatch));
    module.push_str(", source: ");
    module.push_str(&rust_string_literal(spec.source));
    module.push_str(" }");
}

fn push_module_export_plan_record(module: &mut String, spec: &BindingModuleExportPlanSpec) {
    module.push_str("ModuleExportPlan { symbol: ");
    module.push_str(&rust_string_literal(spec.symbol));
    module.push_str(", platforms: ");
    module.push_str(&rust_string_literal(spec.platforms));
    module.push_str(", signature: ");
    module.push_str(&rust_string_literal(spec.signature));
    module.push_str(", purpose: ");
    module.push_str(&rust_string_literal(spec.purpose));
    module.push_str(", implementation: ");
    module.push_str(&rust_string_literal(spec.implementation));
    module.push_str(", return_policy: ");
    module.push_str(&rust_string_literal(spec.return_policy));
    module.push_str(", generated_callable: ");
    module.push_str(if spec.generated_callable {
        "true"
    } else {
        "false"
    });
    module.push_str(", source: ");
    module.push_str(&rust_string_literal(spec.source));
    module.push_str(" }");
}

fn push_binary_export_symbol_plan_record(
    module: &mut String,
    spec: &BindingBinaryExportSymbolPlanSpec,
) {
    module.push_str("BinaryExportSymbolPlan { platform: ");
    module.push_str(&rust_string_literal(spec.platform));
    module.push_str(", binary_format: ");
    module.push_str(&rust_string_literal(spec.binary_format));
    module.push_str(", symbol: ");
    module.push_str(&rust_string_literal(spec.symbol));
    module.push_str(", tool_symbol: ");
    module.push_str(&rust_string_literal(spec.tool_symbol));
    module.push_str(", inspection_tool: ");
    module.push_str(&rust_string_literal(spec.inspection_tool));
    module.push_str(", required: ");
    module.push_str(if spec.required { "true" } else { "false" });
    module.push_str(", verified_by_generated_bindings: ");
    module.push_str(if spec.verified_by_generated_bindings {
        "true"
    } else {
        "false"
    });
    module.push_str(", source: ");
    module.push_str(&rust_string_literal(spec.source));
    module.push_str(" }");
}

fn push_binary_export_inspection_tool_plan_record(
    module: &mut String,
    tool: &BinaryExportInspectionToolPlan,
) {
    module.push_str("    BinaryExportInspectionToolPlan { platform: ");
    module.push_str(&rust_string_literal(tool.platform));
    module.push_str(", program: ");
    module.push_str(&rust_string_literal(tool.program));
    module.push_str(", args: &[");
    for (index, arg) in tool.args.iter().enumerate() {
        if index > 0 {
            module.push_str(", ");
        }
        module.push_str(&rust_string_literal(arg));
    }
    module.push_str("] },\n");
}

fn interface_method_slot(method: &BindingInterfaceMethodSpec) -> usize {
    interface_methods(method.interface)
        .iter()
        .position(|candidate| candidate.name == method.name)
        .unwrap_or(0)
}

fn interface_method_global_slot(method: &BindingInterfaceMethodSpec, local_slot: usize) -> usize {
    if method.interface == "FUnknown" {
        local_slot
    } else {
        3 + local_slot
    }
}

fn interface_method_callback_type_name(method: &BindingInterfaceMethodSpec) -> String {
    let mut callback = String::from(method.interface);
    let mut capitalize_next = true;
    for character in method.name.chars() {
        if !character.is_ascii_alphanumeric() {
            capitalize_next = true;
            continue;
        }
        if capitalize_next {
            callback.push(character.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            callback.push(character);
        }
    }
    callback
}

fn interface_method_signature_intent(method: &BindingInterfaceMethodSpec) -> String {
    let signature = match (method.interface, method.name) {
        ("FUnknown", "queryInterface") => {
            "unsafe extern \"system\" fn(this: *mut FUnknown, iid: *const TUID, obj: *mut *mut std::ffi::c_void) -> TResult"
        }
        ("FUnknown", "addRef") => "unsafe extern \"system\" fn(this: *mut FUnknown) -> uint32",
        ("FUnknown", "release") => "unsafe extern \"system\" fn(this: *mut FUnknown) -> uint32",
        ("IPluginFactory", "countClasses") => {
            "unsafe extern \"system\" fn(this: *mut IPluginFactory) -> int32"
        }
        ("IPluginFactory", "getClassInfo") => {
            "unsafe extern \"system\" fn(this: *mut IPluginFactory, index: int32, info: *mut PClassInfo) -> TResult"
        }
        ("IPluginFactory", "createInstance") => {
            "unsafe extern \"system\" fn(this: *mut IPluginFactory, cid: FIDString, iid: FIDString, obj: *mut *mut std::ffi::c_void) -> TResult"
        }
        ("IPluginBase", "initialize") => {
            "unsafe extern \"system\" fn(this: *mut IPluginBase, context: *mut FUnknown) -> TResult"
        }
        ("IPluginBase", "terminate") => {
            "unsafe extern \"system\" fn(this: *mut IPluginBase) -> TResult"
        }
        ("IConnectionPoint", "connect") => {
            "unsafe extern \"system\" fn(this: *mut IConnectionPoint, other: *mut IConnectionPoint) -> TResult"
        }
        ("IConnectionPoint", "disconnect") => {
            "unsafe extern \"system\" fn(this: *mut IConnectionPoint, other: *mut IConnectionPoint) -> TResult"
        }
        ("IConnectionPoint", "notify") => {
            "unsafe extern \"system\" fn(this: *mut IConnectionPoint, message: *mut IMessage) -> TResult"
        }
        ("IBStream", "read") => {
            "unsafe extern \"system\" fn(this: *mut IBStream, buffer: *mut std::ffi::c_void, num_bytes: int32, num_bytes_read: *mut int32) -> TResult"
        }
        ("IBStream", "write") => {
            "unsafe extern \"system\" fn(this: *mut IBStream, buffer: *mut std::ffi::c_void, num_bytes: int32, num_bytes_written: *mut int32) -> TResult"
        }
        ("IBStream", "seek") => {
            "unsafe extern \"system\" fn(this: *mut IBStream, pos: int64, mode: int32, result: *mut int64) -> TResult"
        }
        ("IBStream", "tell") => {
            "unsafe extern \"system\" fn(this: *mut IBStream, pos: *mut int64) -> TResult"
        }
        ("IPlugView", "isPlatformTypeSupported") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, type_: FIDString) -> TResult"
        }
        ("IPlugView", "attached") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, parent: *mut std::ffi::c_void, type_: FIDString) -> TResult"
        }
        ("IPlugView", "removed") => "unsafe extern \"system\" fn(this: *mut IPlugView) -> TResult",
        ("IPlugView", "onWheel") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, distance: f32) -> TResult"
        }
        ("IPlugView", "onKeyDown") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, key: char16, key_code: int16, modifiers: int16) -> TResult"
        }
        ("IPlugView", "onKeyUp") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, key: char16, key_code: int16, modifiers: int16) -> TResult"
        }
        ("IPlugView", "getSize") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, size: *mut ViewRect) -> TResult"
        }
        ("IPlugView", "onSize") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, new_size: *mut ViewRect) -> TResult"
        }
        ("IPlugView", "onFocus") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, state: TBool) -> TResult"
        }
        ("IPlugView", "setFrame") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, frame: *mut IPlugFrame) -> TResult"
        }
        ("IPlugView", "canResize") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView) -> TResult"
        }
        ("IPlugView", "checkSizeConstraint") => {
            "unsafe extern \"system\" fn(this: *mut IPlugView, rect: *mut ViewRect) -> TResult"
        }
        ("IPlugFrame", "resizeView") => {
            "unsafe extern \"system\" fn(this: *mut IPlugFrame, view: *mut IPlugView, new_size: *mut ViewRect) -> TResult"
        }
        ("IComponent", "getControllerClassId") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, class_id: *mut TUID) -> TResult"
        }
        ("IComponent", "setIoMode") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, mode: IoMode) -> TResult"
        }
        ("IComponent", "getBusCount") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, type_: MediaType, dir: BusDirection) -> int32"
        }
        ("IComponent", "getBusInfo") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, type_: MediaType, dir: BusDirection, index: int32, bus: *mut BusInfo) -> TResult"
        }
        ("IComponent", "getRoutingInfo") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, in_info: *mut RoutingInfo, out_info: *mut RoutingInfo) -> TResult"
        }
        ("IComponent", "activateBus") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, type_: MediaType, dir: BusDirection, index: int32, state: TBool) -> TResult"
        }
        ("IComponent", "setActive") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, state: TBool) -> TResult"
        }
        ("IComponent", "setState") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, state: *mut IBStream) -> TResult"
        }
        ("IComponent", "getState") => {
            "unsafe extern \"system\" fn(this: *mut IComponent, state: *mut IBStream) -> TResult"
        }
        ("IAudioProcessor", "setBusArrangements") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor, inputs: *mut SpeakerArrangement, num_ins: int32, outputs: *mut SpeakerArrangement, num_outs: int32) -> TResult"
        }
        ("IAudioProcessor", "getBusArrangement") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor, dir: BusDirection, index: int32, arr: *mut SpeakerArrangement) -> TResult"
        }
        ("IAudioProcessor", "canProcessSampleSize") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor, symbolic_sample_size: int32) -> TResult"
        }
        ("IAudioProcessor", "getLatencySamples") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor) -> uint32"
        }
        ("IAudioProcessor", "setupProcessing") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor, setup: *mut ProcessSetup) -> TResult"
        }
        ("IAudioProcessor", "setProcessing") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor, state: TBool) -> TResult"
        }
        ("IAudioProcessor", "process") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult"
        }
        ("IAudioProcessor", "getTailSamples") => {
            "unsafe extern \"system\" fn(this: *mut IAudioProcessor) -> uint32"
        }
        ("IProcessContextRequirements", "getProcessContextRequirements") => {
            "unsafe extern \"system\" fn(this: *mut IProcessContextRequirements) -> uint32"
        }
        ("IEditController", "setComponentState") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, state: *mut IBStream) -> TResult"
        }
        ("IEditController", "setState") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, state: *mut IBStream) -> TResult"
        }
        ("IEditController", "getState") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, state: *mut IBStream) -> TResult"
        }
        ("IEditController", "getParameterCount") => {
            "unsafe extern \"system\" fn(this: *mut IEditController) -> int32"
        }
        ("IEditController", "getParameterInfo") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, param_index: int32, info: *mut ParameterInfo) -> TResult"
        }
        ("IEditController", "getParamStringByValue") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, id: ParamID, value_normalized: ParamValue, string: String128) -> TResult"
        }
        ("IEditController", "getParamValueByString") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, id: ParamID, string: TCharConstPtr, value_normalized: *mut ParamValue) -> TResult"
        }
        ("IEditController", "normalizedParamToPlain") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, id: ParamID, value_normalized: ParamValue) -> ParamValue"
        }
        ("IEditController", "plainParamToNormalized") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, id: ParamID, plain_value: ParamValue) -> ParamValue"
        }
        ("IEditController", "getParamNormalized") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, id: ParamID) -> ParamValue"
        }
        ("IEditController", "setParamNormalized") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, id: ParamID, value: ParamValue) -> TResult"
        }
        ("IEditController", "setComponentHandler") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, handler: *mut IComponentHandler) -> TResult"
        }
        ("IEditController", "createView") => {
            "unsafe extern \"system\" fn(this: *mut IEditController, name: FIDString) -> *mut IPlugView"
        }
        ("IComponentHandler", "beginEdit") => {
            "unsafe extern \"system\" fn(this: *mut IComponentHandler, id: ParamID) -> TResult"
        }
        ("IComponentHandler", "performEdit") => {
            "unsafe extern \"system\" fn(this: *mut IComponentHandler, id: ParamID, value_normalized: ParamValue) -> TResult"
        }
        ("IComponentHandler", "endEdit") => {
            "unsafe extern \"system\" fn(this: *mut IComponentHandler, id: ParamID) -> TResult"
        }
        ("IComponentHandler", "restartComponent") => {
            "unsafe extern \"system\" fn(this: *mut IComponentHandler, flags: int32) -> TResult"
        }
        ("IMidiMapping", "getMidiControllerAssignment") => {
            "unsafe extern \"system\" fn(this: *mut IMidiMapping, bus_index: int32, channel: int16, midi_controller_number: CtrlNumber, id: *mut ParamID) -> TResult"
        }
        ("INoteExpressionController", "getNoteExpressionCount") => {
            "unsafe extern \"system\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16) -> int32"
        }
        ("INoteExpressionController", "getNoteExpressionInfo") => {
            "unsafe extern \"system\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult"
        }
        ("INoteExpressionController", "getNoteExpressionStringByValue") => {
            "unsafe extern \"system\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, id: NoteExpressionTypeID, value: NoteExpressionValue, string: String128) -> TResult"
        }
        ("INoteExpressionController", "getNoteExpressionValueByString") => {
            "unsafe extern \"system\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, id: NoteExpressionTypeID, string: TCharConstPtr, value: *mut NoteExpressionValue) -> TResult"
        }
        ("INoteExpressionPhysicalUIMapping", "getPhysicalUIMapping") => {
            "unsafe extern \"system\" fn(this: *mut INoteExpressionPhysicalUIMapping, bus_index: int32, channel: int16, list: *mut PhysicalUIMapList) -> TResult"
        }
        ("IParameterChanges", "getParameterCount") => {
            "unsafe extern \"system\" fn(this: *mut IParameterChanges) -> int32"
        }
        ("IParameterChanges", "getParameterData") => {
            "unsafe extern \"system\" fn(this: *mut IParameterChanges, index: int32) -> *mut IParamValueQueue"
        }
        ("IParamValueQueue", "getParameterId") => {
            "unsafe extern \"system\" fn(this: *mut IParamValueQueue) -> ParamID"
        }
        ("IParamValueQueue", "getPointCount") => {
            "unsafe extern \"system\" fn(this: *mut IParamValueQueue) -> int32"
        }
        ("IParamValueQueue", "getPoint") => {
            "unsafe extern \"system\" fn(this: *mut IParamValueQueue, index: int32, sample_offset: *mut int32, value: *mut ParamValue) -> TResult"
        }
        ("IParamValueQueue", "addPoint") => {
            "unsafe extern \"system\" fn(this: *mut IParamValueQueue, sample_offset: int32, value: ParamValue, index: *mut int32) -> TResult"
        }
        ("IEventList", "getEventCount") => {
            "unsafe extern \"system\" fn(this: *mut IEventList) -> int32"
        }
        ("IEventList", "getEvent") => {
            "unsafe extern \"system\" fn(this: *mut IEventList, index: int32, event: *mut Event) -> TResult"
        }
        ("IEventList", "addEvent") => {
            "unsafe extern \"system\" fn(this: *mut IEventList, event: *mut Event) -> TResult"
        }
        ("IMessage", "getMessageID") => {
            "unsafe extern \"system\" fn(this: *mut IMessage) -> FIDString"
        }
        ("IMessage", "setMessageID") => {
            "unsafe extern \"system\" fn(this: *mut IMessage, id: FIDString) -> ()"
        }
        ("IMessage", "getAttributes") => {
            "unsafe extern \"system\" fn(this: *mut IMessage) -> *mut IAttributeList"
        }
        ("IAttributeList", "setInt") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, value: int64) -> TResult"
        }
        ("IAttributeList", "getInt") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, value: *mut int64) -> TResult"
        }
        ("IAttributeList", "setFloat") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, value: f64) -> TResult"
        }
        ("IAttributeList", "getFloat") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, value: *mut f64) -> TResult"
        }
        ("IAttributeList", "setString") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, string: *const TChar) -> TResult"
        }
        ("IAttributeList", "getString") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, string: *mut TChar, size_in_bytes: uint32) -> TResult"
        }
        ("IAttributeList", "setBinary") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, data: *const std::ffi::c_void, size_in_bytes: uint32) -> TResult"
        }
        ("IAttributeList", "getBinary") => {
            "unsafe extern \"system\" fn(this: *mut IAttributeList, id: AttrID, data: *mut *const std::ffi::c_void, size_in_bytes: *mut uint32) -> TResult"
        }
        ("IUnitInfo", "getUnitCount") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo) -> int32"
        }
        ("IUnitInfo", "getUnitInfo") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, unit_index: int32, info: *mut UnitInfo) -> TResult"
        }
        ("IUnitInfo", "getProgramListCount") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo) -> int32"
        }
        ("IUnitInfo", "getProgramListInfo") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult"
        }
        ("IUnitInfo", "getProgramName") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, list_id: ProgramListID, program_index: int32, name: String128) -> TResult"
        }
        ("IUnitInfo", "getProgramInfo") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, list_id: ProgramListID, program_index: int32, attribute_id: CString, attribute_value: String128) -> TResult"
        }
        ("IUnitInfo", "hasProgramPitchNames") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, list_id: ProgramListID, program_index: int32) -> TResult"
        }
        ("IUnitInfo", "getProgramPitchName") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, list_id: ProgramListID, program_index: int32, midi_pitch: int16, name: String128) -> TResult"
        }
        ("IUnitInfo", "getSelectedUnit") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo) -> UnitID"
        }
        ("IUnitInfo", "selectUnit") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, unit_id: UnitID) -> TResult"
        }
        ("IUnitInfo", "getUnitByBus") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, type_: MediaType, dir: BusDirection, bus_index: int32, channel: int32, unit_id: *mut UnitID) -> TResult"
        }
        ("IUnitInfo", "setUnitProgramData") => {
            "unsafe extern \"system\" fn(this: *mut IUnitInfo, list_or_unit_id: int32, program_index: int32, data: *mut IBStream) -> TResult"
        }
        ("IProgramListData", "programDataSupported") => {
            "unsafe extern \"system\" fn(this: *mut IProgramListData, list_id: ProgramListID) -> TResult"
        }
        ("IProgramListData", "getProgramData") => {
            "unsafe extern \"system\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult"
        }
        ("IProgramListData", "setProgramData") => {
            "unsafe extern \"system\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult"
        }
        _ => {
            return format!(
                "unsafe extern \"system\" fn(this: *mut {}, /* {} parameters: signature-intent pending */) -> TResult",
                method.interface, method.name
            );
        }
    };
    signature.to_string()
}

fn generated_bindings_module_path_check(path: &Path) -> GeneratedBindingsPlanCheck {
    let extension_ok = path.extension().and_then(|extension| extension.to_str()) == Some("rs");
    let file_name_ok = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some();
    if extension_ok && file_name_ok {
        GeneratedBindingsPlanCheck {
            name: "bindings module path".to_string(),
            status: "ok".to_string(),
            value: path.display().to_string(),
            hint: None,
        }
    } else {
        GeneratedBindingsPlanCheck {
            name: "bindings module path".to_string(),
            status: "failed".to_string(),
            value: format!(
                "bindings module path must name a Rust `.rs` file: {}",
                path.display()
            ),
            hint: Some("use a path such as target/vst3-sdk/generated.rs".to_string()),
        }
    }
}

fn interface_methods(interface: &str) -> Vec<&'static BindingInterfaceMethodSpec> {
    GENERATED_BINDINGS_INTERFACE_METHODS
        .iter()
        .filter(|method| method.interface == interface)
        .collect()
}

fn interface_methods_const_name(interface: &str) -> String {
    format!("{}_METHODS", rust_identifier_constant_fragment(interface))
}

fn interface_vtable_slots_const_name(interface: &str) -> String {
    format!(
        "{}_VTABLE_SLOTS",
        rust_identifier_constant_fragment(interface)
    )
}

fn interface_callback_types_const_name(interface: &str) -> String {
    format!(
        "{}_CALLBACK_TYPES",
        rust_identifier_constant_fragment(interface)
    )
}

fn interface_vtable_field_offsets_const_name(interface: &str) -> String {
    format!(
        "{}_VTABLE_FIELD_OFFSETS",
        rust_identifier_constant_fragment(interface)
    )
}

fn interface_iid_const_name(interface: &str) -> String {
    format!("{}_IID", rust_identifier_constant_fragment(interface))
}

fn com_object_interfaces_const_name(object: &str) -> String {
    format!("{}_INTERFACES", rust_identifier_constant_fragment(object))
}

fn com_object_identity_spec(object: &str) -> Option<&'static BindingComObjectIdentitySpec> {
    GENERATED_BINDINGS_COM_OBJECT_IDENTITIES
        .iter()
        .find(|spec| spec.object == object)
}

fn com_object_identity_plan_const_name(object: &str) -> String {
    format!(
        "{}_IDENTITY_PLAN",
        rust_identifier_constant_fragment(object)
    )
}

fn com_object_query_interface_dispatch_const_name(object: &str) -> String {
    format!(
        "{}_QUERY_INTERFACE_DISPATCH",
        rust_identifier_constant_fragment(object)
    )
}

fn factory_class_plan_const_name(class_object: &str) -> String {
    format!(
        "{}_FACTORY_CLASS_PLAN",
        rust_identifier_constant_fragment(class_object)
    )
}

fn interface_method_count_const_name(interface: &str) -> String {
    format!(
        "{}_METHOD_COUNT",
        rust_identifier_constant_fragment(interface)
    )
}

fn rust_identifier_constant_fragment(value: &str) -> String {
    let mut fragment = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            fragment.push(character.to_ascii_uppercase());
        } else {
            fragment.push('_');
        }
    }
    fragment
}

fn rust_string_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for character in value.chars() {
        literal.extend(character.escape_default());
    }
    literal.push('"');
    literal
}

fn contains_identifier_token(text: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    text.match_indices(token).any(|(start, _)| {
        let before = start
            .checked_sub(1)
            .and_then(|index| text.as_bytes().get(index))
            .copied();
        let after = text.as_bytes().get(start + token.len()).copied();
        !before.is_some_and(is_identifier_byte) && !after.is_some_and(is_identifier_byte)
    })
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sdk_version_hint(root: &Path) -> Option<String> {
    for relative in SDK_VERSION_HINT_FILES {
        let path = root.join(relative);
        if !path_is_regular_file_no_symlink(&path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if text.contains(STEINBERG_VST3_SDK_BASELINE) {
            return Some(format!("{relative} mentions {STEINBERG_VST3_SDK_BASELINE}"));
        }
        if text.contains("3.8.0") || text.contains("3.8") {
            return Some(format!("{relative} mentions VST3 SDK 3.8"));
        }
    }
    None
}

#[cfg(feature = "upstream-vst3")]
pub mod upstream {
    pub use vst3::*;
}

#[cfg(feature = "upstream-vst3")]
pub fn upstream_vst3_available() -> bool {
    let _ = std::any::TypeId::of::<vst3::Steinberg::tresult>();
    true
}

#[cfg(not(feature = "upstream-vst3"))]
pub fn upstream_vst3_available() -> bool {
    false
}

#[cfg(feature = "generated-headers")]
pub mod generated_headers {
    pub const STATUS: &str = "reserved";
    pub const SDK_DIR_ENV: &str = super::VST3_SDK_DIR_ENV;
    pub const SDK_BASELINE: &str = super::STEINBERG_VST3_SDK_BASELINE;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sdk_root(missing: &[&str]) -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("pluginterfaces/base")).unwrap();
        std::fs::create_dir_all(root.join("pluginterfaces/vst")).unwrap();
        std::fs::write(root.join("README.md"), "VST3 SDK v3.8.0_build_66").unwrap();
        for header in REQUIRED_GENERATED_HEADER_INPUTS {
            if missing.contains(header) {
                continue;
            }
            std::fs::create_dir_all(root.join(header).parent().unwrap()).unwrap();
            std::fs::write(root.join(header), test_header_text(header)).unwrap();
        }
        temp
    }

    fn test_header_text(header: &str) -> String {
        let mut text = format!("/* {header} */\n");
        for symbol in generated_bindings_surface_symbol_names_for_header(header) {
            text.push_str(symbol);
            text.push('\n');
        }
        text
    }

    #[test]
    fn binding_baseline_matches_framework_plan() {
        assert_eq!(BINDING_BASELINE.steinberg_sdk, "v3.8.0_build_66");
        assert_eq!(BINDING_BASELINE.upstream_vst3_crate, "0.3.0");
        let expected_backend = if cfg!(feature = "generated-headers") {
            BindingBackend::GeneratedHeadersReserved
        } else if cfg!(feature = "upstream-vst3") {
            BindingBackend::UpstreamVst3Crate
        } else {
            BindingBackend::MetadataOnly
        };
        assert_eq!(BINDING_BASELINE.backend, expected_backend);
    }

    #[test]
    fn upstream_probe_follows_feature_flag() {
        assert_eq!(upstream_vst3_available(), cfg!(feature = "upstream-vst3"));
    }

    #[test]
    fn sdk_header_probe_reports_missing_and_present_headers() {
        let temp = create_sdk_root(&["pluginterfaces/vst/ivstmessage.h"]);
        let root = temp.path();

        let probe = probe_sdk_headers(root);

        assert!(!probe.ready_for_generated_headers());
        assert_eq!(
            probe.missing_headers,
            vec!["pluginterfaces/vst/ivstmessage.h"]
        );
        assert!(probe.version_hint.as_deref().unwrap().contains("3.8"));

        std::fs::write(
            root.join("pluginterfaces/vst/ivstmessage.h"),
            "/* header */",
        )
        .unwrap();
        let probe = probe_sdk_headers(root);
        assert!(probe.ready_for_generated_headers());
        assert!(probe.missing_headers.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn sdk_header_probe_treats_symlink_inputs_as_missing() {
        use std::os::unix::fs::symlink;

        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);
        let target = temp.path().join("README.md");
        symlink(target, temp.path().join("pluginterfaces/base/funknown.h")).unwrap();

        let probe = probe_sdk_headers(temp.path());

        assert!(!probe.ready_for_generated_headers());
        assert!(
            probe
                .missing_headers
                .contains(&"pluginterfaces/base/funknown.h")
        );
        assert!(
            !probe
                .present_headers
                .contains(&"pluginterfaces/base/funknown.h")
        );
    }

    #[test]
    fn sdk_header_manifest_records_required_inputs_and_detects_drift() {
        let temp = create_sdk_root(&[]);
        let root = temp.path();

        let manifest = sdk_header_input_manifest(root).unwrap();

        assert_eq!(manifest.version, SDK_HEADER_MANIFEST_VERSION);
        assert_eq!(manifest.generator, SDK_HEADER_MANIFEST_GENERATOR);
        assert_eq!(manifest.steinberg_sdk_baseline, "v3.8.0_build_66");
        assert_eq!(manifest.upstream_vst3_crate_baseline, "0.3.0");
        assert!(manifest.complete);
        assert!(manifest.missing_headers.is_empty());
        assert_eq!(
            manifest.headers.len(),
            REQUIRED_GENERATED_HEADER_INPUTS.len()
        );
        let header = manifest
            .header("pluginterfaces/vst/vsttypes.h")
            .expect("vsttypes header manifest entry");
        assert_eq!(
            header.size,
            test_header_text("pluginterfaces/vst/vsttypes.h").len() as u64
        );
        assert_eq!(header.sha256.len(), 64);
        assert!(header.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));

        check_sdk_header_input_manifest(root, &manifest).unwrap();

        std::fs::write(
            root.join("pluginterfaces/vst/vsttypes.h"),
            "/* changed header */\n",
        )
        .unwrap();
        let error = check_sdk_header_input_manifest(root, &manifest).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("SDK header manifest drift"));
        assert!(text.contains("pluginterfaces/vst/vsttypes.h"));
    }

    #[test]
    fn sdk_header_manifest_reports_missing_inputs() {
        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

        let manifest = sdk_header_input_manifest(temp.path()).unwrap();

        assert!(!manifest.complete);
        assert_eq!(
            manifest.missing_headers,
            vec!["pluginterfaces/base/funknown.h"]
        );
        assert!(manifest.header("pluginterfaces/base/funknown.h").is_none());
    }

    #[test]
    fn binary_export_symbol_plan_public_api_matches_required_symbol_helpers() {
        assert_eq!(binary_export_symbol_plans().len(), 11);
        assert_eq!(required_binary_export_symbol_count("macos"), 5);
        assert_eq!(required_binary_export_symbol_count("windows-x64"), 3);
        assert_eq!(required_binary_export_symbol_count("linux-x64"), 3);
        assert_eq!(required_binary_export_symbol_count("wayland"), 0);

        let macos = required_binary_export_tool_symbols("macos").unwrap();
        assert_eq!(
            macos,
            [
                "_GetPluginFactory",
                "_bundleEntry",
                "_bundleExit",
                "_BundleEntry",
                "_BundleExit"
            ]
        );
        assert_eq!(
            required_binary_export_tool_symbols("windows-x64").unwrap(),
            ["GetPluginFactory", "InitDll", "ExitDll"]
        );
        assert_eq!(
            required_binary_export_tool_symbols("linux-x64").unwrap(),
            ["GetPluginFactory", "ModuleEntry", "ModuleExit"]
        );
        assert!(required_binary_export_tool_symbols("linux-x11").is_none());
        assert_eq!(binary_export_inspection_tool_plans().len(), 6);
        assert!(binary_export_inspection_tools("linux-x11").is_none());
        assert_eq!(
            binary_export_inspection_tools("macos")
                .unwrap()
                .iter()
                .map(BinaryExportInspectionToolPlan::display)
                .collect::<Vec<_>>(),
            vec!["nm -gU".to_string(), "llvm-nm -gU".to_string()]
        );
        assert_eq!(
            binary_export_inspection_tools("windows-x64")
                .unwrap()
                .iter()
                .map(BinaryExportInspectionToolPlan::display)
                .collect::<Vec<_>>(),
            vec![
                "llvm-objdump -p".to_string(),
                "dumpbin /exports".to_string()
            ]
        );
        assert_eq!(
            binary_export_inspection_tools("linux-x64")
                .unwrap()
                .iter()
                .map(BinaryExportInspectionToolPlan::display)
                .collect::<Vec<_>>(),
            vec![
                "nm -D --defined-only".to_string(),
                "llvm-nm -D --defined-only".to_string()
            ]
        );

        for platform in ["macos", "windows-x64", "linux-x64"] {
            let required = required_binary_export_tool_symbols(platform).unwrap();
            let tools = binary_export_inspection_tools(platform).unwrap();
            assert_eq!(
                tools,
                binary_export_inspection_tool_plans()
                    .iter()
                    .filter(|tool| tool.platform == platform)
                    .copied()
                    .collect::<Vec<_>>()
            );
            let planned = binary_export_symbol_plans()
                .iter()
                .filter(|plan| plan.platform == platform && plan.required)
                .map(|plan| plan.tool_symbol)
                .collect::<Vec<_>>();
            assert_eq!(planned, required);
            for symbol in required {
                let plan = binary_export_symbol_plan(platform, symbol)
                    .expect("required symbol should have a public plan");
                assert_eq!(plan.platform, platform);
                assert_eq!(plan.tool_symbol, *symbol);
                assert!(plan.required);
                assert!(!plan.verified_by_generated_bindings);
            }
        }

        assert!(binary_export_required_symbols_present(
            "linux-x64",
            &["GetPluginFactory", "ModuleEntry", "ModuleExit"]
        ));
        assert_eq!(
            first_missing_binary_export_symbol("windows-x64", &["GetPluginFactory", "InitDll"]),
            Some("ExitDll")
        );
        assert!(!binary_export_required_symbols_present(
            "windows-x64",
            &["GetPluginFactory", "InitDll"]
        ));
        assert!(binary_export_symbol_plan("macos", "bundleEntry").is_none());
    }

    #[test]
    fn generated_bindings_plan_reports_ready_inputs_without_claiming_generated_bindings() {
        let temp = create_sdk_root(&[]);

        let plan = generated_bindings_plan(temp.path(), "target/vst3-sdk/generated.rs").unwrap();

        assert_eq!(plan.version, GENERATED_BINDINGS_PLAN_VERSION);
        assert_eq!(plan.generator, GENERATED_BINDINGS_PLAN_GENERATOR);
        assert_eq!(plan.status, "ready-for-binding-generator");
        assert!(!plan.bindings_generated);
        assert!(plan.header_manifest.complete);
        assert!(plan.blockers.is_empty());
        assert!(plan.checks.iter().any(|check| {
            check.name == "binding emitter"
                && check.status == "reserved"
                && check.value.contains("not enabled yet")
        }));
    }

    #[test]
    fn generated_bindings_plan_reports_missing_inputs_and_bad_output_path() {
        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

        let plan = generated_bindings_plan(temp.path(), "target/vst3-sdk/generated.txt").unwrap();

        assert_eq!(plan.status, "blocked");
        assert!(!plan.bindings_generated);
        assert_eq!(plan.blockers.len(), 2);
        assert!(plan.blockers[0].contains("pluginterfaces/base/funknown.h"));
        assert!(plan.blockers[1].contains(".rs"));
        assert!(
            plan.checks
                .iter()
                .any(|check| { check.name == "bindings module path" && check.status == "failed" })
        );
    }

    #[test]
    fn generated_bindings_surface_locks_expected_symbols_without_claiming_bindings() {
        let temp = create_sdk_root(&[]);

        let surface = generated_bindings_surface(temp.path()).unwrap();
        let repeated = generated_bindings_surface(temp.path()).unwrap();

        assert_eq!(surface, repeated);
        assert_eq!(surface.version, GENERATED_BINDINGS_SURFACE_VERSION);
        assert_eq!(surface.generator, GENERATED_BINDINGS_SURFACE_GENERATOR);
        assert_eq!(surface.status, "ready-for-binding-emitter");
        assert!(!surface.bindings_generated);
        assert!(surface.header_manifest.complete);
        assert!(surface.blockers.is_empty());
        assert_eq!(
            surface.required_headers.len(),
            REQUIRED_GENERATED_HEADER_INPUTS.len()
        );
        assert!(surface.symbols.iter().all(|symbol| symbol.header_present));
        assert!(surface.symbols.iter().all(|symbol| symbol.symbol_present));
        assert!(surface.missing_symbols.is_empty());
        assert!(surface.symbols.iter().any(|symbol| {
            symbol.name == "IPlugView" && symbol.header == "pluginterfaces/gui/iplugview.h"
        }));
        assert!(surface.symbols.iter().any(|symbol| {
            symbol.name == "IMidiMapping"
                && symbol.header == "pluginterfaces/vst/ivstmidicontrollers.h"
        }));
        assert!(surface.symbols.iter().any(|symbol| {
            symbol.name == "INoteExpressionController"
                && symbol.header == "pluginterfaces/vst/ivstnoteexpression.h"
        }));
        assert!(surface.symbols.iter().any(|symbol| {
            symbol.name == "IProgramListData" && symbol.header == "pluginterfaces/vst/ivstunits.h"
        }));
        assert!(
            surface
                .notes
                .iter()
                .any(|note| note.contains("does not parse C++ AST"))
        );
    }

    #[test]
    fn generated_bindings_surface_reports_missing_symbol_tokens() {
        let temp = create_sdk_root(&[]);
        let root = temp.path();
        std::fs::write(
            root.join("pluginterfaces/vst/ivstmidicontrollers.h"),
            "/* header exists but target token is absent */\nIMidiMappingExtra\n",
        )
        .unwrap();

        let surface = generated_bindings_surface(root).unwrap();

        assert_eq!(surface.status, "blocked");
        assert!(!surface.bindings_generated);
        assert!(
            surface
                .missing_symbols
                .iter()
                .any(|entry| entry.contains("IMidiMapping"))
        );
        assert!(
            surface
                .blockers
                .iter()
                .any(|blocker| blocker.contains("absent from their locked headers"))
        );
        assert!(surface.symbols.iter().any(|symbol| {
            symbol.name == "IMidiMapping" && symbol.header_present && !symbol.symbol_present
        }));
    }

    #[test]
    fn generated_bindings_surface_reports_missing_symbol_headers() {
        let temp = create_sdk_root(&["pluginterfaces/gui/iplugview.h"]);

        let surface = generated_bindings_surface(temp.path()).unwrap();

        assert_eq!(surface.status, "blocked");
        assert!(!surface.bindings_generated);
        assert!(
            surface
                .blockers
                .iter()
                .any(|blocker| blocker.contains("pluginterfaces/gui/iplugview.h"))
        );
        assert!(
            surface
                .missing_headers
                .iter()
                .any(|entry| entry.contains("IPlugView"))
        );
    }

    #[test]
    fn generated_bindings_scaffold_emits_deterministic_metadata_without_claiming_bindings() {
        let temp = create_sdk_root(&[]);

        let scaffold =
            generated_bindings_scaffold(temp.path(), "target/vst3-sdk/generated.rs").unwrap();
        let repeated =
            generated_bindings_scaffold(temp.path(), "target/vst3-sdk/generated.rs").unwrap();

        assert_eq!(scaffold.module, repeated.module);
        assert_eq!(scaffold.plan.status, "ready-for-binding-generator");
        assert_eq!(scaffold.surface.status, "ready-for-binding-emitter");
        assert!(!scaffold.plan.bindings_generated);
        assert!(!scaffold.surface.bindings_generated);
        assert!(
            scaffold
                .module
                .contains(GENERATED_BINDINGS_SCAFFOLD_GENERATOR)
        );
        assert!(
            scaffold
                .module
                .contains("pub const BINDINGS_GENERATED: bool = false;")
        );
        assert!(
            scaffold
                .module
                .contains("pub const STATUS: &str = \"metadata-scaffold\";")
        );
        assert!(
            scaffold
                .module
                .contains("pub const SURFACE_STATUS: &str = \"ready-for-binding-emitter\";")
        );
        assert!(
            scaffold
                .module
                .contains("pub const HEADER_INPUTS: &[HeaderInput] = &[")
        );
        assert!(
            scaffold
                .module
                .contains("pub const BINDING_SURFACE_SYMBOLS: &[BindingSymbol] = &[")
        );
        assert!(scaffold.module.contains("IPlugView"));
        assert!(scaffold.module.contains("IMidiMapping"));
        assert!(scaffold.module.contains("pluginterfaces/base/funknown.h"));
        assert!(
            scaffold
                .module
                .contains("pub const MISSING_HEADERS: &[&str] = &[")
        );
        assert!(!scaffold.module.contains("BINDINGS_GENERATED: bool = true"));
    }

    #[test]
    fn generated_bindings_scaffold_is_blocked_until_inputs_and_module_path_are_ready() {
        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

        let error =
            generated_bindings_scaffold(temp.path(), "target/vst3-sdk/generated.txt").unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[test]
    fn generated_bindings_abi_seed_emits_deterministic_seed_without_claiming_full_bindings() {
        let temp = create_sdk_root(&[]);

        let seed =
            generated_bindings_abi_seed(temp.path(), "target/vst3-sdk/generated-abi-seed.rs")
                .unwrap();
        let repeated =
            generated_bindings_abi_seed(temp.path(), "target/vst3-sdk/generated-abi-seed.rs")
                .unwrap();

        assert_eq!(seed.module, repeated.module);
        assert_eq!(seed.plan.status, "ready-for-binding-generator");
        assert_eq!(seed.surface.status, "ready-for-binding-emitter");
        assert!(!seed.plan.bindings_generated);
        assert!(!seed.surface.bindings_generated);
        assert!(seed.module.contains(GENERATED_BINDINGS_ABI_SEED_GENERATOR));
        assert!(
            seed.module
                .contains("pub const STATUS: &str = \"abi-seed\";")
        );
        assert!(
            seed.module
                .contains("pub const ABI_SEED_GENERATED: bool = true;")
        );
        assert!(
            seed.module
                .contains("pub const BINDINGS_GENERATED: bool = false;")
        );
        assert!(
            seed.module
                .contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;")
        );
        assert!(seed.module.contains("pub type TResult = i32;"));
        assert!(seed.module.contains("pub type ParamID = u32;"));
        assert!(seed.module.contains("pub type ParamValue = f64;"));
        assert!(seed.module.contains("pub type TChar = u16;"));
        assert!(
            seed.module
                .contains("pub type TUID = [std::os::raw::c_char; 16];")
        );
        assert!(seed.module.contains("pub const kResultOk: TResult = 0;"));
        assert!(
            seed.module
                .contains("pub const kInvalidArgument: TResult = 2;")
        );
        assert!(
            seed.module
                .contains("pub const kNotImplemented: TResult = 3;")
        );
        assert!(
            seed.module
                .contains("pub const kPlatformTypeHWND: PlatformType = \"HWND\";")
        );
        assert!(
            seed.module
                .contains("pub const kPlatformTypeNSView: PlatformType = \"NSView\";")
        );
        assert!(seed.module.contains(
            "pub const kPlatformTypeX11EmbedWindowID: PlatformType = \"X11EmbedWindowID\";"
        ));
        assert!(seed.module.contains("IPlugView"));
        assert!(seed.module.contains("IMidiMapping"));
        assert!(seed.module.contains("pluginterfaces/base/funknown.h"));
        assert!(
            !seed
                .module
                .contains("FULL_COM_BINDINGS_GENERATED: bool = true")
        );
    }

    #[test]
    fn generated_bindings_abi_seed_is_blocked_until_inputs_and_module_path_are_ready() {
        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

        let error =
            generated_bindings_abi_seed(temp.path(), "target/vst3-sdk/generated-abi-seed.txt")
                .unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[test]
    fn generated_bindings_abi_emits_deterministic_compileable_layout_without_claiming_full_bindings()
     {
        let temp = create_sdk_root(&[]);

        let abi = generated_bindings_abi(temp.path(), "target/vst3-sdk/generated-abi.rs").unwrap();
        let repeated =
            generated_bindings_abi(temp.path(), "target/vst3-sdk/generated-abi.rs").unwrap();

        assert_eq!(abi.module, repeated.module);
        assert_eq!(abi.plan.status, "ready-for-binding-generator");
        assert_eq!(abi.surface.status, "ready-for-binding-emitter");
        assert!(!abi.plan.bindings_generated);
        assert!(!abi.surface.bindings_generated);
        assert!(abi.module.contains(GENERATED_BINDINGS_ABI_GENERATOR));
        assert!(
            abi.module
                .contains("pub const STATUS: &str = \"abi-layout\";")
        );
        assert!(
            abi.module
                .contains("pub const ABI_LAYOUT_GENERATED: bool = true;")
        );
        assert!(
            abi.module
                .contains("pub const BINDINGS_GENERATED: bool = false;")
        );
        assert!(
            abi.module
                .contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;")
        );
        assert!(abi.module.contains("#[repr(C)]"));
        assert!(abi.module.contains("pub struct TUID"));
        assert!(abi.module.contains("pub struct FUnknownVTable"));
        assert!(abi.module.contains("pub struct FUnknown"));
        assert!(abi.module.contains("pub struct ViewRect"));
        assert!(
            abi.module
                .contains("pub type String128 = [TChar; STRING128_CODE_UNITS];")
        );
        assert!(abi.module.contains("pub type UnitID = int32;"));
        assert!(abi.module.contains("pub type ProgramListID = int32;"));
        assert!(
            abi.module
                .contains("pub type NoteExpressionTypeID = uint32;")
        );
        assert!(abi.module.contains("pub type NoteExpressionValue = f64;"));
        assert!(abi.module.contains("pub type PhysicalUITypeID = uint32;"));
        assert!(abi.module.contains("pub struct ProgramListInfo"));
        assert!(abi.module.contains("pub struct UnitInfo"));
        assert!(
            abi.module
                .contains("pub struct NoteExpressionValueDescription")
        );
        assert!(abi.module.contains("pub struct NoteExpressionTypeInfo"));
        assert!(abi.module.contains("pub struct PhysicalUIMap"));
        assert!(abi.module.contains("pub struct PhysicalUIMapList"));
        assert!(abi.module.contains("pub const kRootUnitId: UnitID = 0;"));
        assert!(
            abi.module
                .contains("pub const kNoProgramListId: ProgramListID = -1;")
        );
        assert!(
            abi.module
                .contains("pub type FUnknownQueryInterface = unsafe extern \"system\" fn(")
        );
        assert!(abi.module.contains("pub type ParamID = u32;"));
        assert!(abi.module.contains("pub type ParamValue = f64;"));
        assert!(abi.module.contains("pub type Sample32 = f32;"));
        assert!(abi.module.contains("pub type Sample64 = f64;"));
        assert!(
            abi.module
                .contains("pub const ABI_LAYOUT_TYPES: &[&str] = &[")
        );
        assert!(
            abi.module
                .contains("pub const ABI_LAYOUT_RECORDS: &[AbiLayoutRecord] = &[")
        );
        assert!(abi.module.contains(
            "type_name: \"ProgramListInfo\", size: std::mem::size_of::<ProgramListInfo>()"
        ));
        assert!(
            abi.module
                .contains("type_name: \"NoteExpressionTypeInfo\", size: std::mem::size_of::<NoteExpressionTypeInfo>()")
        );
        assert!(
            abi.module
                .contains("pub const ABI_FIELD_OFFSETS: &[AbiFieldOffset] = &[")
        );
        assert!(
            abi.module
                .contains("owner: \"ProgramListInfo\", field: \"programCount\", offset: std::mem::offset_of!(ProgramListInfo, programCount)")
        );
        assert!(
            abi.module
                .contains("owner: \"NoteExpressionTypeInfo\", field: \"valueDesc\", offset: std::mem::offset_of!(NoteExpressionTypeInfo, valueDesc)")
        );
        assert!(
            abi.module
                .contains("owner: \"PhysicalUIMapList\", field: \"map\", offset: std::mem::offset_of!(PhysicalUIMapList, map)")
        );
        assert!(abi.module.contains("IPlugView"));
        assert!(abi.module.contains("IMidiMapping"));
        assert!(abi.module.contains("pluginterfaces/base/funknown.h"));
        assert!(
            !abi.module
                .contains("FULL_COM_BINDINGS_GENERATED: bool = true")
        );

        let module_path = temp.path().join("generated_abi.rs");
        std::fs::write(&module_path, &abi.module).unwrap();
        let output_path = temp.path().join("libgenerated_abi.rlib");
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let output = std::process::Command::new(&rustc)
            .arg("--edition=2021")
            .arg("--crate-type")
            .arg("lib")
            .arg(&module_path)
            .arg("-o")
            .arg(&output_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "generated ABI layout module should compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn generated_bindings_abi_is_blocked_until_inputs_and_module_path_are_ready() {
        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

        let error =
            generated_bindings_abi(temp.path(), "target/vst3-sdk/generated-abi.txt").unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[test]
    fn generated_bindings_interface_skeleton_emits_deterministic_compileable_skeleton_without_claiming_full_bindings()
     {
        let temp = create_sdk_root(&[]);

        let skeleton = generated_bindings_interface_skeleton(
            temp.path(),
            "target/vst3-sdk/generated-interface-skeleton.rs",
        )
        .unwrap();
        let repeated = generated_bindings_interface_skeleton(
            temp.path(),
            "target/vst3-sdk/generated-interface-skeleton.rs",
        )
        .unwrap();

        assert_eq!(skeleton.module, repeated.module);
        assert_eq!(skeleton.plan.status, "ready-for-binding-generator");
        assert_eq!(skeleton.surface.status, "ready-for-binding-emitter");
        assert!(!skeleton.plan.bindings_generated);
        assert!(!skeleton.surface.bindings_generated);
        assert!(
            skeleton
                .module
                .contains(GENERATED_BINDINGS_INTERFACE_SKELETON_GENERATOR)
        );
        assert!(
            skeleton
                .module
                .contains("pub const STATUS: &str = \"interface-skeleton\";")
        );
        assert!(
            skeleton
                .module
                .contains("pub const INTERFACE_SKELETON_GENERATED: bool = true;")
        );
        assert!(
            skeleton
                .module
                .contains("pub const BINDINGS_GENERATED: bool = false;")
        );
        assert!(
            skeleton
                .module
                .contains("pub const FULL_COM_BINDINGS_GENERATED: bool = false;")
        );
        assert!(skeleton.module.contains("pub struct FUnknownVTable"));
        assert!(skeleton.module.contains("pub struct FUnknown"));
        assert!(skeleton.module.contains("pub struct IPlugViewVTable"));
        assert!(skeleton.module.contains("pub struct IPlugView"));
        assert!(skeleton.module.contains("pub struct IEditControllerVTable"));
        assert!(skeleton.module.contains("pub struct IMidiMappingVTable"));
        assert!(skeleton.module.contains("pub struct IUnitInfoVTable"));
        assert!(
            skeleton
                .module
                .contains("pub struct IProgramListDataVTable")
        );
        assert!(
            skeleton
                .module
                .contains("pub struct INoteExpressionControllerVTable")
        );
        assert!(skeleton.module.contains("pub struct InterfaceMethod"));
        assert!(skeleton.module.contains("pub struct InterfaceVTableSlot"));
        assert!(skeleton.module.contains("pub struct InterfaceCallbackType"));
        assert!(
            skeleton
                .module
                .contains("pub struct InterfaceVTableFieldOffset")
        );
        assert!(skeleton.module.contains("pub struct ComObjectInterface"));
        assert!(skeleton.module.contains("pub struct ComObjectIdentityPlan"));
        assert!(
            skeleton
                .module
                .contains("pub struct ComObjectQueryInterfaceDispatchEntry")
        );
        assert!(skeleton.module.contains("pub struct FactoryExportPlan"));
        assert!(skeleton.module.contains("pub struct FactoryClassPlan"));
        assert!(skeleton.module.contains("pub struct ModuleExportPlan"));
        assert!(
            skeleton
                .module
                .contains("pub struct BinaryExportSymbolPlan")
        );
        assert!(
            skeleton
                .module
                .contains("pub struct BinaryExportInspectionToolPlan")
        );
        assert!(skeleton.module.contains("pub slot: usize,"));
        assert!(skeleton.module.contains("pub signature: &'static str,"));
        assert!(skeleton.module.contains("pub local_slot: usize,"));
        assert!(skeleton.module.contains("pub global_slot: usize,"));
        assert!(skeleton.module.contains("pub callback_type: &'static str,"));
        assert!(skeleton.module.contains("pub object: &'static str,"));
        assert!(skeleton.module.contains("pub required: bool,"));
        assert!(
            skeleton
                .module
                .contains("pub root_interface: &'static str,")
        );
        assert!(
            skeleton
                .module
                .contains("pub root_iid_const: &'static str,")
        );
        assert!(
            skeleton
                .module
                .contains("pub funknown_identity: &'static str,")
        );
        assert!(
            skeleton
                .module
                .contains("pub unknown_iid_result: &'static str,")
        );
        assert!(skeleton.module.contains("pub returns_same_identity: bool,"));
        assert!(skeleton.module.contains("pub add_ref_on_success: bool,"));
        assert!(
            skeleton
                .module
                .contains("pub factory_object: &'static str,")
        );
        assert!(
            skeleton
                .module
                .contains("pub factory_iid_const: &'static str,")
        );
        assert!(skeleton.module.contains("pub class_count: usize,"));
        assert!(skeleton.module.contains("pub class_kind: &'static str,"));
        assert!(skeleton.module.contains("pub class_index: usize,"));
        assert!(skeleton.module.contains("pub cid_policy: &'static str,"));
        assert!(
            skeleton
                .module
                .contains("pub create_instance_root_iid_const: &'static str,")
        );
        assert!(
            skeleton
                .module
                .contains("pub requested_iid_dispatch: &'static str,")
        );
        assert!(skeleton.module.contains("pub symbol: &'static str,"));
        assert!(skeleton.module.contains("pub platforms: &'static str,"));
        assert!(skeleton.module.contains("pub generated_callable: bool,"));
        assert!(skeleton.module.contains("pub binary_format: &'static str,"));
        assert!(skeleton.module.contains("pub tool_symbol: &'static str,"));
        assert!(
            skeleton
                .module
                .contains("pub inspection_tool: &'static str,")
        );
        assert!(
            skeleton
                .module
                .contains("pub verified_by_generated_bindings: bool,")
        );
        assert!(skeleton.module.contains("pub program: &'static str,"));
        assert!(
            skeleton
                .module
                .contains("pub args: &'static [&'static str],")
        );
        assert!(skeleton.module.contains(
            "pub const INTERFACE_METHOD_SLOT_SCOPE: &str = \"per-interface-order-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_METHOD_SIGNATURE_SCOPE: &str = \"signature-intent-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_VTABLE_SLOT_SCOPE: &str = \"per-interface-local-vtable-seed-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_VTABLE_GLOBAL_SLOT_SCOPE: &str = \"com-vtable-global-slot-seed-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_VTABLE_SLOT_LOOKUP_SCOPE: &str = \"pure-vtable-slot-lookup-seed-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_VTABLE_FIELD_SCOPE: &str = \"repr-c-vtable-callback-field-layout-seed-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_VTABLE_FIELD_OFFSET_SCOPE: &str = \"repr-c-vtable-callback-field-offset-fingerprint-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_VTABLE_FIELD_OFFSET_LOOKUP_SCOPE: &str = \"pure-vtable-field-offset-lookup-seed-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const INTERFACE_CALLBACK_TYPE_SCOPE: &str = \"callback-type-alias-seed-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const COM_OBJECT_INTERFACE_SCOPE: &str = \"vesty-com-object-interface-exposure-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const COM_OBJECT_IDENTITY_PLAN_SCOPE: &str = \"vesty-com-object-funknown-identity-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_SCOPE: &str = \"vesty-com-object-query-interface-dispatch-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const QUERY_INTERFACE_IID_LOOKUP_SCOPE: &str = \"pure-iid-dispatch-lookup-seed-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const FACTORY_EXPORT_PLAN_SCOPE: &str = \"vesty-factory-export-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const FACTORY_CLASS_PLAN_SCOPE: &str = \"vesty-factory-class-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const MODULE_EXPORT_PLAN_SCOPE: &str = \"vesty-module-export-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const BINARY_EXPORT_SYMBOL_PLAN_SCOPE: &str = \"vesty-binary-export-symbol-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_SCOPE: &str = \"vesty-binary-export-inspection-tool-plan-audit\";"
        ));
        assert!(skeleton.module.contains(
            "pub const BINARY_EXPORT_SYMBOL_REQUIREMENT_HELPERS_GENERATED: bool = true;"
        ));
        assert!(
            skeleton
                .module
                .contains("pub const BINARY_EXPORT_SYMBOL_INSPECTION_GENERATED: bool = false;")
        );
        assert!(
            skeleton
                .module
                .contains("pub const FACTORY_EXPORT_PLAN_COUNT: usize = 1;")
        );
        assert!(
            skeleton
                .module
                .contains("pub const FACTORY_CLASS_PLAN_COUNT: usize = 2;")
        );
        assert!(
            skeleton
                .module
                .contains("pub const MODULE_EXPORT_PLAN_COUNT: usize = 9;")
        );
        assert!(
            skeleton
                .module
                .contains("pub const BINARY_EXPORT_SYMBOL_PLAN_COUNT: usize = 11;")
        );
        assert!(
            skeleton
                .module
                .contains("pub const BINARY_EXPORT_INSPECTION_TOOL_PLAN_COUNT: usize = 6;")
        );
        assert!(skeleton.module.contains("pub const kNoInterface: TResult"));
        assert!(
            skeleton
                .module
                .contains("pub const INTERFACE_CALLBACK_TYPES: &[InterfaceCallbackType] = &[")
        );
        assert!(skeleton.module.contains(
            "pub const INTERFACE_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        ));
        assert!(
            skeleton
                .module
                .contains("pub fn interface_vtable_field_offset_by_interface_and_field(")
        );
        assert!(
            skeleton
                .module
                .contains("pub const INTERFACE_METHODS: &[InterfaceMethod] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const INTERFACE_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub fn interface_vtable_slot_by_interface_and_method(")
        );
        assert!(
            skeleton
                .module
                .contains("pub fn interface_vtable_slot_by_interface_and_global_slot(")
        );
        assert!(
            skeleton.module.contains(
                "pub fn interface_id_for_iid(iid: &TUID) -> Option<&'static InterfaceId>"
            )
        );
        assert!(skeleton.module.contains(
            "pub fn query_interface_entry_by_interface(interface: &str) -> Option<&'static QueryInterfaceEntry>"
        ));
        assert!(skeleton.module.contains(
            "pub fn query_interface_entry_for_iid(iid: &TUID) -> Option<&'static QueryInterfaceEntry>"
        ));
        assert!(
            skeleton
                .module
                .contains("pub const COM_OBJECTS: &[&str] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const COM_OBJECT_INTERFACES: &[ComObjectInterface] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const COM_OBJECT_IDENTITY_PLANS: &[ComObjectIdentityPlan] = &[")
        );
        assert!(skeleton.module.contains(
            "pub const COM_OBJECT_QUERY_INTERFACE_DISPATCH_ENTRIES: &[ComObjectQueryInterfaceDispatchEntry] = &["
        ));
        assert!(
            skeleton
                .module
                .contains("pub fn com_object_query_interface_dispatch_by_interface(")
        );
        assert!(
            skeleton
                .module
                .contains("pub fn com_object_query_interface_dispatch_for_iid(")
        );
        assert!(
            skeleton
                .module
                .contains("pub const FACTORY_EXPORT_PLAN: FactoryExportPlan = ")
        );
        assert!(
            skeleton
                .module
                .contains("pub const FACTORY_CLASS_PLANS: &[FactoryClassPlan] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const VESTYPROCESSOR_FACTORY_CLASS_PLAN: FactoryClassPlan = ")
        );
        assert!(
            skeleton
                .module
                .contains("pub const VESTYCONTROLLER_FACTORY_CLASS_PLAN: FactoryClassPlan = ")
        );
        assert!(
            skeleton
                .module
                .contains("pub const GETPLUGINFACTORY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(
            skeleton
                .module
                .contains("pub const WINDOWS_INITDLL_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(
            skeleton
                .module
                .contains("pub const MACOS_BUNDLEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(
            skeleton
                .module
                .contains("pub const LINUX_MODULEENTRY_MODULE_EXPORT_PLAN: ModuleExportPlan = ")
        );
        assert!(
            skeleton
                .module
                .contains("pub const MODULE_EXPORT_PLANS: &[ModuleExportPlan] = &[")
        );
        assert!(skeleton.module.contains(
            "pub const WINDOWS_GETPLUGINFACTORY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = "
        ));
        assert!(skeleton.module.contains(
            "pub const MACOS_BUNDLEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = "
        ));
        assert!(skeleton.module.contains(
            "pub const LINUX_MODULEENTRY_BINARY_EXPORT_SYMBOL_PLAN: BinaryExportSymbolPlan = "
        ));
        assert!(
            skeleton
                .module
                .contains("pub const BINARY_EXPORT_SYMBOL_PLANS: &[BinaryExportSymbolPlan] = &[")
        );
        assert!(skeleton.module.contains(
            "pub const BINARY_EXPORT_INSPECTION_TOOL_PLANS: &[BinaryExportInspectionToolPlan] = &["
        ));
        assert!(
            skeleton
                .module
                .contains("pub fn binary_export_inspection_tools(")
        );
        assert!(
            skeleton
                .module
                .contains("pub fn binary_export_symbol_plan_by_platform_and_symbol(")
        );
        assert!(
            skeleton
                .module
                .contains("pub fn required_binary_export_symbol_count(platform: &str) -> usize")
        );
        assert!(
            skeleton
                .module
                .contains("pub fn first_missing_binary_export_symbol(")
        );
        assert!(
            skeleton
                .module
                .contains("pub fn binary_export_required_symbols_present(")
        );
        assert!(
            skeleton
                .module
                .contains("pub const VESTYPROCESSOR_INTERFACES: &[ComObjectInterface] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const VESTYCONTROLLER_INTERFACES: &[ComObjectInterface] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const VESTYPROCESSOR_IDENTITY_PLAN: ComObjectIdentityPlan = ")
        );
        assert!(skeleton.module.contains(
            "pub const VESTYPROCESSOR_QUERY_INTERFACE_DISPATCH: &[ComObjectQueryInterfaceDispatchEntry] = &["
        ));
        assert!(
            skeleton
                .module
                .contains("pub const IAUDIOPROCESSOR_METHODS: &[InterfaceMethod] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const IAUDIOPROCESSOR_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[")
        );
        assert!(
            skeleton.module.contains(
                "pub const IAUDIOPROCESSOR_CALLBACK_TYPES: &[InterfaceCallbackType] = &["
            )
        );
        assert!(skeleton.module.contains(
            "pub const IAUDIOPROCESSOR_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        ));
        assert!(
            skeleton
                .module
                .contains("pub type IAudioProcessorProcess = unsafe extern \"system\" fn(")
        );
        assert!(
            skeleton
                .module
                .contains("pub process: IAudioProcessorProcess,")
        );
        assert!(
            skeleton
                .module
                .contains("offset: std::mem::offset_of!(IAudioProcessorVTable, process)")
        );
        assert!(skeleton.module.contains("slot: 6, interface: \"IAudioProcessor\", name: \"process\", purpose: \"realtime audio/MIDI/process callback\", realtime: true, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
        assert!(skeleton.module.contains("local_slot: 6, global_slot: 9, interface: \"IAudioProcessor\", method: \"process\", field: \"process\", callback_type: \"IAudioProcessorProcess\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
        assert!(skeleton.module.contains("InterfaceCallbackType { interface: \"IAudioProcessor\", method: \"process\", callback_type: \"IAudioProcessorProcess\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IAudioProcessor, data: *mut ProcessData) -> TResult\""));
        assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyProcessor\", interface: \"IProcessContextRequirements\", iid_const: \"IPROCESSCONTEXTREQUIREMENTS_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyController\", interface: \"IEditController\", iid_const: \"IEDITCONTROLLER_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyPlugView\", interface: \"IPlugView\", iid_const: \"IPLUGVIEW_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(skeleton.module.contains("ComObjectInterface { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", exposure: \"implemented-by-current-vesty-vst3-adapter\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\", required: true }"));
        assert!(skeleton.module.contains("ComObjectIdentityPlan { object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }"));
        assert!(skeleton.module.contains("ComObjectIdentityPlan { object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", funknown_identity: \"single-controlling-funknown-per-com-object\", refcount_policy: \"query-interface-success-addref-release-decrements-wrapper\", unknown_iid_result: \"kNoInterface\", null_object_pointer_result: \"kInvalidArgument\", source: \"crates/vesty-vst3/src/bindings_impl.rs::Class::Interfaces\" }"));
        assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"FUnknown\", iid_const: \"FUNKNOWN_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
        assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyProcessor\", interface: \"IAudioProcessor\", iid_const: \"IAUDIOPROCESSOR_IID\", root_interface: \"IComponent\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
        assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyController\", interface: \"IProgramListData\", iid_const: \"IPROGRAMLISTDATA_IID\", root_interface: \"IEditController\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
        assert!(skeleton.module.contains("ComObjectQueryInterfaceDispatchEntry { object: \"VestyFactory\", interface: \"IPluginFactory\", iid_const: \"IPLUGINFACTORY_IID\", root_interface: \"IPluginFactory\", returns_same_identity: true, success_result: \"kResultOk\", add_ref_on_success: true, implementation: \"planned-object-query-interface-dispatch-no-callable-glue\" }"));
        assert!(skeleton.module.contains("FactoryExportPlan { factory_object: \"VestyFactory\", factory_interface: \"IPluginFactory\", factory_iid_const: \"IPLUGINFACTORY_IID\", class_count: 2, count_classes_result: \"2\", get_factory_info_source: \"PluginInfo vendor/url/email + kUnicode\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
        assert!(skeleton.module.contains("FactoryClassPlan { class_kind: \"processor\", class_index: 0, class_object: \"VestyProcessor\", root_interface: \"IComponent\", root_iid_const: \"ICOMPONENT_IID\", category: \"Audio Module Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id\", cid_policy: \"processor-cid-is-plugin-class-id\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyProcessor\", create_instance_root_interface: \"IComponent\", create_instance_root_iid_const: \"ICOMPONENT_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
        assert!(skeleton.module.contains("FactoryClassPlan { class_kind: \"controller\", class_index: 1, class_object: \"VestyController\", root_interface: \"IEditController\", root_iid_const: \"IEDITCONTROLLER_IID\", category: \"Component Controller Class\", name_source: \"PluginInfo::name\", cid_source: \"PluginInfo::class_id[15].wrapping_add(1)\", cid_policy: \"controller-cid-last-byte-wrapping-add-1\", cardinality: \"kManyInstances\", get_class_info_result: \"kResultOk\", invalid_class_index_result: \"kInvalidArgument\", create_instance_object: \"VestyController\", create_instance_root_interface: \"IEditController\", create_instance_root_iid_const: \"IEDITCONTROLLER_IID\", unknown_cid_result: \"kInvalidArgument\", construction_failure_result: \"kResultFalse\", requested_iid_dispatch: \"delegate-to-created-instance-queryInterface\", source: \"crates/vesty-vst3/src/bindings_impl.rs::VestyFactory\" }"));
        assert!(skeleton.module.contains("ModuleExportPlan { symbol: \"GetPluginFactory\", platforms: \"windows,macos,linux\", signature: \"extern \\\"system\\\" fn() -> *mut IPluginFactory\", purpose: \"return VST3 plugin factory pointer\", implementation: \"vesty_vst3::create_plugin_factory::<Plugin>()\", return_policy: \"returns owned COM factory pointer for host discovery\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(skeleton.module.contains("ModuleExportPlan { symbol: \"bundleEntry\", platforms: \"macos\", signature: \"extern \\\"system\\\" fn(bundle_ref: *mut c_void) -> bool\", purpose: \"macOS VST3 bundle initialization entry\", implementation: \"vesty_vst3::set_macos_bundle_ref(bundle_ref); return true\", return_policy: \"bundle resources path is captured when possible\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(skeleton.module.contains("ModuleExportPlan { symbol: \"ModuleEntry\", platforms: \"linux\", signature: \"extern \\\"system\\\" fn(library_handle: *mut c_void) -> bool\", purpose: \"Linux VST3 module initialization entry\", implementation: \"return true\", return_policy: \"host may continue loading the module\", generated_callable: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(skeleton.module.contains("BinaryExportSymbolPlan { platform: \"windows-x64\", binary_format: \"PE/COFF\", symbol: \"GetPluginFactory\", tool_symbol: \"GetPluginFactory\", inspection_tool: \"dumpbin /exports or llvm-objdump -p\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(skeleton.module.contains("BinaryExportSymbolPlan { platform: \"macos\", binary_format: \"Mach-O\", symbol: \"bundleEntry\", tool_symbol: \"_bundleEntry\", inspection_tool: \"nm -gU or llvm-nm -gU\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(skeleton.module.contains("BinaryExportSymbolPlan { platform: \"linux-x64\", binary_format: \"ELF\", symbol: \"ModuleEntry\", tool_symbol: \"ModuleEntry\", inspection_tool: \"nm -D --defined-only or llvm-nm -D --defined-only\", required: true, verified_by_generated_bindings: false, source: \"crates/vesty/src/lib.rs::export_vst3!\" }"));
        assert!(skeleton.module.contains("BinaryExportInspectionToolPlan { platform: \"macos\", program: \"nm\", args: &[\"-gU\"] }"));
        assert!(skeleton.module.contains("BinaryExportInspectionToolPlan { platform: \"windows-x64\", program: \"llvm-objdump\", args: &[\"-p\"] }"));
        assert!(skeleton.module.contains("BinaryExportInspectionToolPlan { platform: \"linux-x64\", program: \"llvm-nm\", args: &[\"-D\", \"--defined-only\"] }"));
        assert!(
            skeleton
                .module
                .contains("pub const IUNITINFO_METHODS: &[InterfaceMethod] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const IUNITINFO_VTABLE_SLOTS: &[InterfaceVTableSlot] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub const IUNITINFO_CALLBACK_TYPES: &[InterfaceCallbackType] = &[")
        );
        assert!(skeleton.module.contains(
            "pub const IUNITINFO_VTABLE_FIELD_OFFSETS: &[InterfaceVTableFieldOffset] = &["
        ));
        assert!(
            skeleton
                .module
                .contains("pub type IUnitInfoGetProgramListInfo = unsafe extern \"system\" fn(")
        );
        assert!(
            skeleton
                .module
                .contains("pub getProgramListInfo: IUnitInfoGetProgramListInfo,")
        );
        assert!(
            skeleton
                .module
                .contains("offset: std::mem::offset_of!(IUnitInfoVTable, getProgramListInfo)")
        );
        assert!(skeleton.module.contains("name: \"getProgramListInfo\", purpose: \"return program-list metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\""));
        assert!(skeleton.module.contains("local_slot: 3, global_slot: 6, interface: \"IUnitInfo\", method: \"getProgramListInfo\", field: \"getProgramListInfo\", callback_type: \"IUnitInfoGetProgramListInfo\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IUnitInfo, list_index: int32, info: *mut ProgramListInfo) -> TResult\""));
        assert!(
            skeleton
                .module
                .contains("pub const IPROGRAMLISTDATA_METHODS: &[InterfaceMethod] = &[")
        );
        assert!(
            skeleton
                .module
                .contains("pub type IProgramListDataGetProgramData = unsafe extern \"system\" fn(")
        );
        assert!(
            skeleton
                .module
                .contains("pub getProgramData: IProgramListDataGetProgramData,")
        );
        assert!(
            skeleton
                .module
                .contains("offset: std::mem::offset_of!(IProgramListDataVTable, getProgramData)")
        );
        assert!(skeleton.module.contains("name: \"getProgramData\", purpose: \"save program data to stream\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\""));
        assert!(skeleton.module.contains("local_slot: 1, global_slot: 4, interface: \"IProgramListData\", method: \"getProgramData\", field: \"getProgramData\", callback_type: \"IProgramListDataGetProgramData\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut IProgramListData, list_id: ProgramListID, program_index: int32, data: *mut IBStream) -> TResult\""));
        assert!(
            skeleton
                .module
                .contains("pub const INOTEEXPRESSIONCONTROLLER_METHODS: &[InterfaceMethod] = &[")
        );
        assert!(skeleton.module.contains(
            "pub type INoteExpressionControllerGetNoteExpressionInfo = unsafe extern \"system\" fn("
        ));
        assert!(skeleton.module.contains(
            "pub getNoteExpressionInfo: INoteExpressionControllerGetNoteExpressionInfo,"
        ));
        assert!(skeleton.module.contains(
            "offset: std::mem::offset_of!(INoteExpressionControllerVTable, getNoteExpressionInfo)"
        ));
        assert!(skeleton.module.contains("name: \"getNoteExpressionInfo\", purpose: \"return Note Expression type metadata\", realtime: false, signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\""));
        assert!(skeleton.module.contains("local_slot: 1, global_slot: 4, interface: \"INoteExpressionController\", method: \"getNoteExpressionInfo\", field: \"getNoteExpressionInfo\", callback_type: \"INoteExpressionControllerGetNoteExpressionInfo\", signature: \"unsafe extern \\\"system\\\" fn(this: *mut INoteExpressionController, bus_index: int32, channel: int16, note_expression_index: int32, info: *mut NoteExpressionTypeInfo) -> TResult\""));
        assert!(
            skeleton
                .module
                .contains("pub const INTERFACE_SKELETON_TYPES: &[&str] = &[")
        );
        assert!(skeleton.module.contains("pluginterfaces/base/funknown.h"));
        assert!(
            skeleton
                .module
                .contains("pluginterfaces/vst/ivstmidicontrollers.h")
        );
        assert!(
            skeleton
                .module
                .contains("pluginterfaces/vst/ivstnoteexpression.h")
        );
        assert!(skeleton.module.contains("pluginterfaces/vst/ivstunits.h"));
        assert!(
            !skeleton
                .module
                .contains("FULL_COM_BINDINGS_GENERATED: bool = true")
        );

        let module_path = temp.path().join("generated_interface_skeleton.rs");
        std::fs::write(&module_path, &skeleton.module).unwrap();
        let output_path = temp.path().join("libgenerated_interface_skeleton.rlib");
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let output = std::process::Command::new(&rustc)
            .arg("--edition=2021")
            .arg("--crate-type")
            .arg("lib")
            .arg(&module_path)
            .arg("-o")
            .arg(&output_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "generated interface skeleton module should compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let helper_check_path = temp.path().join("generated_interface_skeleton_check.rs");
        std::fs::write(
            &helper_check_path,
            r#"
#[path = "generated_interface_skeleton.rs"]
mod generated_interface_skeleton;

use generated_interface_skeleton::*;

fn main() {
    let interface = interface_id_for_iid(&IAUDIOPROCESSOR_IID)
        .expect("IAudioProcessor IID should resolve");
    assert_eq!(interface.interface, "IAudioProcessor");

    let query = query_interface_entry_for_iid(&IAUDIOPROCESSOR_IID)
        .expect("IAudioProcessor queryInterface entry should resolve");
    assert_eq!(query.interface, "IAudioProcessor");
    assert!(query.inherits_funknown);

    let dispatch =
        com_object_query_interface_dispatch_for_iid("VestyProcessor", &IAUDIOPROCESSOR_IID)
            .expect("VestyProcessor should expose IAudioProcessor");
    assert_eq!(dispatch.object, "VestyProcessor");
    assert_eq!(dispatch.interface, "IAudioProcessor");
    assert!(dispatch.returns_same_identity);
    assert_eq!(dispatch.success_result, "kResultOk");

    assert!(
        com_object_query_interface_dispatch_for_iid("VestyPlugView", &IAUDIOPROCESSOR_IID)
            .is_none()
    );
    assert!(query_interface_entry_for_iid(&TUID::ZERO).is_none());

    let process_slot = INTERFACE_VTABLE_SLOTS
        .iter()
        .find(|slot| slot.interface == "IAudioProcessor" && slot.method == "process")
        .expect("IAudioProcessor process vtable slot should be emitted");
    assert_eq!(process_slot.local_slot, 6);
    assert_eq!(process_slot.global_slot, 9);
    let process_slot_by_method =
        interface_vtable_slot_by_interface_and_method("IAudioProcessor", "process")
            .expect("IAudioProcessor process slot lookup should resolve");
    assert_eq!(process_slot_by_method.global_slot, 9);
    let process_slot_by_global =
        interface_vtable_slot_by_interface_and_global_slot("IAudioProcessor", 9)
            .expect("IAudioProcessor global slot lookup should resolve");
    assert_eq!(process_slot_by_global.method, "process");
    assert!(
        interface_vtable_slot_by_interface_and_global_slot("IAudioProcessor", 99).is_none()
    );
    let process_offset =
        interface_vtable_field_offset_by_interface_and_field("IAudioProcessor", "process")
            .expect("IAudioProcessor process field offset lookup should resolve");
    assert_eq!(process_offset.callback_type, "IAudioProcessorProcess");
    assert_eq!(
        process_offset.offset,
        std::mem::offset_of!(IAudioProcessorVTable, process)
    );
    assert!(
        interface_vtable_field_offset_by_interface_and_field("IAudioProcessor", "missing")
            .is_none()
    );

    let release_slot = FUNKNOWN_VTABLE_SLOTS
        .iter()
        .find(|slot| slot.method == "release")
        .expect("FUnknown release slot should be emitted");
    assert_eq!(release_slot.local_slot, 2);
    assert_eq!(release_slot.global_slot, 2);

    let macos_bundle_entry =
        binary_export_symbol_plan_by_platform_and_symbol("macos", "_bundleEntry")
            .expect("macOS bundleEntry binary export plan should resolve");
    assert_eq!(macos_bundle_entry.symbol, "bundleEntry");
    assert_eq!(macos_bundle_entry.binary_format, "Mach-O");
    assert_eq!(required_binary_export_symbol_count("macos"), 5);
    assert_eq!(required_binary_export_symbol_count("windows-x64"), 3);
    assert_eq!(required_binary_export_symbol_count("linux-x64"), 3);
    assert_eq!(required_binary_export_symbol_count("wayland"), 0);
    assert!(binary_export_required_symbols_present(
        "linux-x64",
        &["GetPluginFactory", "ModuleEntry", "ModuleExit"]
    ));
    assert!(binary_export_required_symbols_present(
        "macos",
        &[
            "_GetPluginFactory",
            "_bundleEntry",
            "_bundleExit",
            "_BundleEntry",
            "_BundleExit"
        ]
    ));
    assert_eq!(
        first_missing_binary_export_symbol("windows-x64", &["GetPluginFactory", "InitDll"]),
        Some("ExitDll")
    );
    assert!(
        !binary_export_required_symbols_present("windows-x64", &["GetPluginFactory", "InitDll"])
    );
    assert!(
        binary_export_symbol_plan_by_platform_and_symbol("macos", "bundleEntry").is_none()
    );
}
"#,
        )
        .unwrap();
        let helper_binary = if cfg!(windows) {
            "generated_interface_skeleton_check.exe"
        } else {
            "generated_interface_skeleton_check"
        };
        let helper_output_path = temp.path().join(helper_binary);
        let output = std::process::Command::new(&rustc)
            .arg("--edition=2021")
            .arg(&helper_check_path)
            .arg("-o")
            .arg(&helper_output_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "generated interface skeleton helper check should compile\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let output = std::process::Command::new(&helper_output_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "generated interface skeleton helper check should run\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn generated_bindings_interface_skeleton_is_blocked_until_inputs_and_module_path_are_ready() {
        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);

        let error = generated_bindings_interface_skeleton(
            temp.path(),
            "target/vst3-sdk/generated-interface-skeleton.txt",
        )
        .unwrap_err();

        let text = error.to_string();
        assert!(text.contains("generated bindings scaffold is blocked"));
        assert!(text.contains("pluginterfaces/base/funknown.h"));
        assert!(text.contains(".rs"));
    }

    #[cfg(unix)]
    #[test]
    fn sdk_header_manifest_rejects_symlink_inputs() {
        use std::os::unix::fs::symlink;

        let temp = create_sdk_root(&["pluginterfaces/base/funknown.h"]);
        let target = temp.path().join("README.md");
        symlink(target, temp.path().join("pluginterfaces/base/funknown.h")).unwrap();

        let error = sdk_header_input_manifest(temp.path()).unwrap_err();

        assert!(error.to_string().contains("not a symlink"));
    }
}
