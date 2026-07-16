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

pub(crate) const SDK_VERSION_HINT_FILES: &[&str] = &["README.md", "CMakeLists.txt", "VERSION"];
