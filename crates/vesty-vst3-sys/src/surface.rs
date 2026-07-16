use crate::interface_method_signature_intent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingSurfaceSymbolSpec {
    pub(crate) name: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) header: &'static str,
    pub(crate) purpose: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedBindingsSurfaceSymbolSpec {
    pub name: &'static str,
    pub kind: &'static str,
    pub header: &'static str,
    pub purpose: &'static str,
}

impl BindingSurfaceSymbolSpec {
    pub(crate) const fn interface(
        name: &'static str,
        header: &'static str,
        purpose: &'static str,
    ) -> Self {
        Self {
            name,
            kind: "interface",
            header,
            purpose,
        }
    }

    pub(crate) const fn type_(
        name: &'static str,
        header: &'static str,
        purpose: &'static str,
    ) -> Self {
        Self {
            name,
            kind: "type",
            header,
            purpose,
        }
    }

    pub(crate) const fn constant(
        name: &'static str,
        header: &'static str,
        purpose: &'static str,
    ) -> Self {
        Self {
            name,
            kind: "constant",
            header,
            purpose,
        }
    }

    pub(crate) const fn public(self) -> GeneratedBindingsSurfaceSymbolSpec {
        GeneratedBindingsSurfaceSymbolSpec {
            name: self.name,
            kind: self.kind,
            header: self.header,
            purpose: self.purpose,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingInterfaceMethodSpec {
    pub(crate) interface: &'static str,
    pub(crate) name: &'static str,
    pub(crate) purpose: &'static str,
    pub(crate) realtime: bool,
}

impl BindingInterfaceMethodSpec {
    pub(crate) const fn new(
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
pub(crate) struct BindingInterfaceIdSpec {
    pub(crate) interface: &'static str,
    pub(crate) uid_words: [u32; 4],
    pub(crate) source: &'static str,
}

impl BindingInterfaceIdSpec {
    pub(crate) const fn new(interface: &'static str, uid_words: [u32; 4]) -> Self {
        Self {
            interface,
            uid_words,
            source: "upstream-vst3-0.3.0/src/bindings.rs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BindingComObjectInterfaceSpec {
    pub(crate) object: &'static str,
    pub(crate) interface: &'static str,
    pub(crate) exposure: &'static str,
    pub(crate) source: &'static str,
    pub(crate) required: bool,
}

impl BindingComObjectInterfaceSpec {
    pub(crate) const fn new(object: &'static str, interface: &'static str) -> Self {
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
pub(crate) struct BindingComObjectIdentitySpec {
    pub(crate) object: &'static str,
    pub(crate) root_interface: &'static str,
    pub(crate) funknown_identity: &'static str,
    pub(crate) refcount_policy: &'static str,
    pub(crate) unknown_iid_result: &'static str,
    pub(crate) null_object_pointer_result: &'static str,
    pub(crate) source: &'static str,
}

impl BindingComObjectIdentitySpec {
    pub(crate) const fn new(object: &'static str, root_interface: &'static str) -> Self {
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
pub(crate) struct BindingFactoryExportPlanSpec {
    pub(crate) factory_object: &'static str,
    pub(crate) factory_interface: &'static str,
    pub(crate) class_count: usize,
    pub(crate) count_classes_result: &'static str,
    pub(crate) get_factory_info_source: &'static str,
    pub(crate) source: &'static str,
}

impl BindingFactoryExportPlanSpec {
    pub(crate) const fn new() -> Self {
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
pub(crate) struct BindingFactoryClassPlanSpec {
    pub(crate) class_kind: &'static str,
    pub(crate) class_index: usize,
    pub(crate) class_object: &'static str,
    pub(crate) root_interface: &'static str,
    pub(crate) category: &'static str,
    pub(crate) name_source: &'static str,
    pub(crate) cid_source: &'static str,
    pub(crate) cid_policy: &'static str,
    pub(crate) cardinality: &'static str,
    pub(crate) get_class_info_result: &'static str,
    pub(crate) invalid_class_index_result: &'static str,
    pub(crate) create_instance_object: &'static str,
    pub(crate) create_instance_root_interface: &'static str,
    pub(crate) unknown_cid_result: &'static str,
    pub(crate) construction_failure_result: &'static str,
    pub(crate) requested_iid_dispatch: &'static str,
    pub(crate) source: &'static str,
}

impl BindingFactoryClassPlanSpec {
    pub(crate) const fn processor() -> Self {
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

    pub(crate) const fn controller() -> Self {
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
pub(crate) struct BindingModuleExportPlanSpec {
    pub(crate) const_name: &'static str,
    pub(crate) symbol: &'static str,
    pub(crate) platforms: &'static str,
    pub(crate) signature: &'static str,
    pub(crate) purpose: &'static str,
    pub(crate) implementation: &'static str,
    pub(crate) return_policy: &'static str,
    pub(crate) generated_callable: bool,
    pub(crate) source: &'static str,
}

impl BindingModuleExportPlanSpec {
    pub(crate) const fn new(
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
pub(crate) struct BindingBinaryExportSymbolPlanSpec {
    pub(crate) const_name: &'static str,
    pub(crate) platform: &'static str,
    pub(crate) binary_format: &'static str,
    pub(crate) symbol: &'static str,
    pub(crate) tool_symbol: &'static str,
    pub(crate) inspection_tool: &'static str,
    pub(crate) required: bool,
    pub(crate) verified_by_generated_bindings: bool,
    pub(crate) source: &'static str,
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
    pub(crate) const fn new(
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

    pub(crate) const fn public(self) -> BinaryExportSymbolPlan {
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

pub(crate) const GENERATED_BINDINGS_SURFACE_SYMBOLS: &[BindingSurfaceSymbolSpec] = &[
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

pub(crate) const GENERATED_BINDINGS_INTERFACE_IDS: &[BindingInterfaceIdSpec] = &[
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

pub(crate) const GENERATED_BINDINGS_COM_OBJECT_INTERFACES: &[BindingComObjectInterfaceSpec] = &[
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

pub(crate) const GENERATED_BINDINGS_COM_OBJECTS: &[&str] = &[
    "VestyAttributeList",
    "VestyMessage",
    "VestyProcessor",
    "VestyPlugView",
    "VestyController",
    "VestyFactory",
];

pub(crate) const GENERATED_BINDINGS_COM_OBJECT_IDENTITIES: &[BindingComObjectIdentitySpec] = &[
    BindingComObjectIdentitySpec::new("VestyAttributeList", "IAttributeList"),
    BindingComObjectIdentitySpec::new("VestyMessage", "IMessage"),
    BindingComObjectIdentitySpec::new("VestyProcessor", "IComponent"),
    BindingComObjectIdentitySpec::new("VestyPlugView", "IPlugView"),
    BindingComObjectIdentitySpec::new("VestyController", "IEditController"),
    BindingComObjectIdentitySpec::new("VestyFactory", "IPluginFactory"),
];

pub(crate) const GENERATED_BINDINGS_FACTORY_EXPORT_PLAN: BindingFactoryExportPlanSpec =
    BindingFactoryExportPlanSpec::new();

pub(crate) const GENERATED_BINDINGS_FACTORY_CLASS_PLANS: &[BindingFactoryClassPlanSpec] = &[
    BindingFactoryClassPlanSpec::processor(),
    BindingFactoryClassPlanSpec::controller(),
];

pub(crate) const GENERATED_BINDINGS_MODULE_EXPORT_PLANS: &[BindingModuleExportPlanSpec] = &[
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

pub(crate) const GENERATED_BINDINGS_BINARY_EXPORT_SYMBOL_PLANS:
    &[BindingBinaryExportSymbolPlanSpec] = &[
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

pub(crate) const WINDOWS_X64_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS: &[&str] =
    &["GetPluginFactory", "InitDll", "ExitDll"];
pub(crate) const MACOS_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS: &[&str] = &[
    "_GetPluginFactory",
    "_bundleEntry",
    "_bundleExit",
    "_BundleEntry",
    "_BundleExit",
];
pub(crate) const LINUX_X64_REQUIRED_BINARY_EXPORT_TOOL_SYMBOLS: &[&str] =
    &["GetPluginFactory", "ModuleEntry", "ModuleExit"];

pub(crate) const MACOS_BINARY_EXPORT_INSPECTION_TOOLS: &[BinaryExportInspectionToolPlan] = &[
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

pub(crate) const WINDOWS_X64_BINARY_EXPORT_INSPECTION_TOOLS: &[BinaryExportInspectionToolPlan] = &[
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

pub(crate) const LINUX_X64_BINARY_EXPORT_INSPECTION_TOOLS: &[BinaryExportInspectionToolPlan] = &[
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

pub(crate) const GENERATED_BINDINGS_INTERFACE_METHODS: &[BindingInterfaceMethodSpec] = &[
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
