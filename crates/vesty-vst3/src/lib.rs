#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use vesty_core::{Plugin, PluginInfo};
pub use vesty_vst3_sys as sys;

#[cfg(feature = "vst3-bindings")]
mod bindings_impl;

#[cfg(feature = "vst3-bindings")]
pub use bindings_impl::{create_plugin_factory, raw};

#[derive(Debug, Error)]
pub enum Vst3AdapterError {
    #[error("plugin instance is faulted")]
    Faulted,
    #[error("panic crossed VST3 callback boundary")]
    Panic,
}

#[derive(Debug, Default)]
pub struct FaultState {
    faulted: AtomicBool,
    fault_count: AtomicU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaultReport {
    pub faulted: bool,
    pub fault_count: u64,
}

impl FaultState {
    pub fn is_faulted(&self) -> bool {
        self.faulted.load(Ordering::Acquire)
    }

    pub fn fault_count(&self) -> u64 {
        self.fault_count.load(Ordering::Acquire)
    }

    pub fn report(&self) -> FaultReport {
        FaultReport {
            faulted: self.is_faulted(),
            fault_count: self.fault_count(),
        }
    }

    pub fn mark_faulted(&self) {
        if self
            .faulted
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.fault_count.fetch_add(1, Ordering::AcqRel);
        }
    }
}

pub fn panic_guard<T>(fault: &FaultState, fallback: T, f: impl FnOnce() -> T) -> T {
    if fault.is_faulted() {
        return fallback;
    }

    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            fault.mark_faulted();
            fallback
        }
    }
}

pub fn abi_guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(fallback)
}

#[derive(Clone, Copy, Debug)]
pub struct Vst3BundleMetadata {
    pub info: PluginInfo,
    pub processor_class_id: [u8; 16],
    pub controller_class_id: [u8; 16],
}

impl Vst3BundleMetadata {
    pub fn for_plugin<P: Plugin>() -> Self {
        let info = P::INFO;
        let mut controller_class_id = info.class_id;
        controller_class_id[15] = controller_class_id[15].wrapping_add(1);
        Self {
            info,
            processor_class_id: info.class_id,
            controller_class_id,
        }
    }
}

#[cfg(feature = "vst3-bindings")]
pub fn bindings_enabled() -> bool {
    let _ = std::any::TypeId::of::<vst3::Steinberg::tresult>();
    true
}

#[cfg(not(feature = "vst3-bindings"))]
pub fn bindings_enabled() -> bool {
    false
}

pub fn binding_baseline() -> sys::BindingBaseline {
    sys::BINDING_BASELINE
}

#[cfg(target_os = "macos")]
static MACOS_BUNDLE_RESOURCES: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFBundleCopyResourcesDirectoryURL(
        bundle: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;
    fn CFURLGetFileSystemRepresentation(
        url: *const std::ffi::c_void,
        resolve_against_base: u8,
        buffer: *mut u8,
        max_buf_len: isize,
    ) -> u8;
    fn CFRelease(value: *const std::ffi::c_void);
}

#[cfg(target_os = "macos")]
pub fn set_macos_bundle_ref(bundle_ref: *mut std::ffi::c_void) {
    if bundle_ref.is_null() || MACOS_BUNDLE_RESOURCES.get().is_some() {
        return;
    }

    // SAFETY: Steinberg calls BundleEntry with a valid CFBundleRef; null/CF returns are checked and the copied CFURL is released.
    unsafe {
        let url = CFBundleCopyResourcesDirectoryURL(bundle_ref.cast_const());
        if url.is_null() {
            return;
        }

        let mut buffer = [0_u8; 4096];
        let ok =
            CFURLGetFileSystemRepresentation(url, 1, buffer.as_mut_ptr(), buffer.len() as isize);
        CFRelease(url);
        if ok == 0 {
            return;
        }

        let len = buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(buffer.len());
        if len == 0 {
            return;
        }

        let path = PathBuf::from(String::from_utf8_lossy(&buffer[..len]).into_owned());
        let _ = MACOS_BUNDLE_RESOURCES.set(path);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_macos_bundle_ref(_bundle_ref: *mut std::ffi::c_void) {}

pub fn bundle_resources_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        MACOS_BUNDLE_RESOURCES.get().cloned()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(test)]
mod tests;
