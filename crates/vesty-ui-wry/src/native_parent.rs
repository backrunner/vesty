#[cfg(target_os = "macos")]
use raw_window_handle::AppKitWindowHandle;
#[cfg(target_os = "windows")]
use raw_window_handle::Win32WindowHandle;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use raw_window_handle::XlibWindowHandle;
use raw_window_handle::{HasWindowHandle, RawWindowHandle, WindowHandle};
use vesty_ui::UiError;
#[cfg(target_os = "windows")]
use std::num::NonZeroIsize;
#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

#[cfg(feature = "wry-backend")]
#[derive(Clone, Copy, Debug)]
pub enum NativeParent {
    #[cfg(target_os = "macos")]
    MacOsNsView(NonNull<c_void>),
    #[cfg(target_os = "windows")]
    WindowsHwnd(NonZeroIsize),
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    XlibWindow(std::ffi::c_ulong),
}

#[cfg(feature = "wry-backend")]
impl NativeParent {
    #[cfg(target_os = "macos")]
    /// # Safety
    ///
    /// `ns_view` must be a valid `NSView` owned by the host editor on the UI thread.
    pub unsafe fn macos_ns_view(ns_view: *mut c_void) -> Result<Self, UiError> {
        NonNull::new(ns_view)
            .map(Self::MacOsNsView)
            .ok_or(UiError::UnsupportedParent)
    }

    #[cfg(target_os = "windows")]
    /// # Safety
    ///
    /// `hwnd` must be a valid HWND owned by the current UI thread.
    pub unsafe fn windows_hwnd(hwnd: isize) -> Result<Self, UiError> {
        NonZeroIsize::new(hwnd)
            .map(Self::WindowsHwnd)
            .ok_or(UiError::UnsupportedParent)
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    /// # Safety
    ///
    /// `window` must be a valid Xlib `Window`; Wayland is intentionally not represented here.
    pub unsafe fn xlib_window(window: std::ffi::c_ulong) -> Result<Self, UiError> {
        if window == 0 {
            Err(UiError::UnsupportedParent)
        } else {
            Ok(Self::XlibWindow(window))
        }
    }
}

#[cfg(feature = "wry-backend")]
impl HasWindowHandle for NativeParent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
        let handle = match *self {
            #[cfg(target_os = "macos")]
            Self::MacOsNsView(ns_view) => RawWindowHandle::AppKit(AppKitWindowHandle::new(ns_view)),
            #[cfg(target_os = "windows")]
            Self::WindowsHwnd(hwnd) => RawWindowHandle::Win32(Win32WindowHandle::new(hwnd)),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            Self::XlibWindow(window) => RawWindowHandle::Xlib(XlibWindowHandle::new(window)),
        };
        // SAFETY: NativeParent constructors validate non-null/non-zero handles and the caller
        // guarantees the host-owned parent remains valid for the WebView attach lifetime.
        Ok(unsafe { WindowHandle::borrow_raw(handle) })
    }
}
