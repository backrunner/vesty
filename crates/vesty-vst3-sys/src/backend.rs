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
