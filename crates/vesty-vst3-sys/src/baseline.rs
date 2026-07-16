use crate::*;

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
