use std::{cell::Cell, path::PathBuf, rc::Rc};
use vesty_ipc::{
    BridgeErrorCode, BridgeErrorPayload, BridgeKind, BridgePacket, advance_bridge_packet_seq,
    validate_bridge_packet_flags, validate_bridge_packet_id, validate_bridge_packet_seq,
    validate_bridge_session, validate_packet_type,
};
use vesty_ui::{EditorRuntime, EditorSize, UiDescriptor, UiError};

use crate::{
    NativeParent, WryBridgeError, batch_scripts, bootstrap_script, packet_script,
};
use crate::assets::{
    asset_response, load_asset_manifest, release_ipc_allowed, release_navigation_allowed,
};

#[cfg(feature = "wry-backend")]
pub(crate) type IpcHandler = Rc<dyn Fn(String) -> Vec<BridgePacket>>;

#[cfg(feature = "wry-backend")]
#[derive(Default)]
pub struct WryEditorRuntime {
    webview: Option<Box<wry::WebView>>,
    ipc_handler: Option<IpcHandler>,
}

#[cfg(feature = "wry-backend")]
impl WryEditorRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ipc_handler(handler: impl Fn(String) + 'static) -> Self {
        Self {
            webview: None,
            ipc_handler: Some(Rc::new(move |text| {
                handler(text);
                Vec::new()
            })),
        }
    }

    pub fn with_bridge_handler(handler: impl Fn(String) -> Vec<BridgePacket> + 'static) -> Self {
        Self {
            webview: None,
            ipc_handler: Some(Rc::new(handler)),
        }
    }

    pub fn set_ipc_handler(&mut self, handler: impl Fn(String) + 'static) {
        self.ipc_handler = Some(Rc::new(move |text| {
            handler(text);
            Vec::new()
        }));
    }

    pub fn set_bridge_handler(&mut self, handler: impl Fn(String) -> Vec<BridgePacket> + 'static) {
        self.ipc_handler = Some(Rc::new(handler));
    }

    pub fn evaluate_packet(&self, packet: &BridgePacket) -> Result<(), WryBridgeError> {
        if let Some(webview) = &self.webview {
            evaluate_packet(webview, packet)?;
        }
        Ok(())
    }
}

#[cfg(feature = "wry-backend")]
impl EditorRuntime for WryEditorRuntime {
    type Parent = NativeParent;

    fn attach(&mut self, parent: Self::Parent, descriptor: &UiDescriptor) -> Result<(), UiError> {
        let bounds = bounds_for_size(descriptor.width, descriptor.height);
        let mut builder = wry::WebViewBuilder::new()
            .with_initialization_script(bootstrap_script())
            .with_bounds(bounds)
            .with_devtools(use_devtools())
            .with_general_autofill_enabled(false);
        let use_dev_url = use_dev_url() && descriptor.dev_url.is_some();

        if !use_dev_url {
            builder = builder
                .with_navigation_handler(release_navigation_allowed)
                .with_download_started_handler(|_, _| false)
                .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny);
        }

        let webview_ptr = Rc::new(Cell::new(std::ptr::null::<wry::WebView>()));
        if let Some(handler) = self.ipc_handler.clone() {
            let webview_ptr = webview_ptr.clone();
            let release_ipc_only = !use_dev_url;
            builder = builder.with_ipc_handler(move |request| {
                if release_ipc_only {
                    let uri = request.uri().to_string();
                    if !release_ipc_allowed(&uri) {
                        return;
                    }
                }
                let packets = call_ipc_handler_guarded(&handler, request.body().clone());
                if packets.is_empty() {
                    return;
                }
                let Ok(scripts) = batch_scripts(&packets) else {
                    return;
                };
                let ptr = webview_ptr.get();
                if !ptr.is_null() {
                    // SAFETY: `ptr` is set immediately after WebView construction and cleared by
                    // dropping the runtime on the same UI thread. The IPC closure only uses it
                    // synchronously to evaluate a response batch while the WebView is alive.
                    unsafe {
                        for script in scripts {
                            let _ = (&*ptr).evaluate_script(&script);
                        }
                    }
                }
            });
        }

        if use_dev_url {
            let Some(dev_url) = descriptor.dev_url.as_ref() else {
                return Err(UiError::RuntimeUnavailable(
                    "VESTY_UI_DEV requested a dev URL, but UiDescriptor.dev_url is empty"
                        .to_string(),
                ));
            };
            builder = builder.with_url(dev_url);
        } else {
            let assets_dir = PathBuf::from(&descriptor.assets_dir);
            let asset_manifest = load_asset_manifest(&assets_dir).map_err(|error| {
                UiError::RuntimeUnavailable(format!(
                    "failed to load UI asset manifest for {}: {error}",
                    assets_dir.display()
                ))
            })?;
            let protocol_root = assets_dir.clone();
            builder = builder
                .with_custom_protocol("vesty".to_string(), move |_id, request| {
                    asset_response(&protocol_root, &asset_manifest, request.uri().path())
                })
                .with_url("vesty://assets/index.html");
        }

        let webview = Box::new(
            builder
                .build_as_child(&parent)
                .map_err(|error| UiError::RuntimeUnavailable(error.to_string()))?,
        );
        webview_ptr.set(webview.as_ref() as *const wry::WebView);
        self.webview = Some(webview);
        Ok(())
    }

    fn resize(&mut self, size: EditorSize) -> Result<(), UiError> {
        if let Some(webview) = &self.webview {
            webview
                .set_bounds(bounds_for_size(size.width, size.height))
                .map_err(|error| UiError::RuntimeUnavailable(error.to_string()))?;
        }
        Ok(())
    }

    fn detach(&mut self) {
        self.webview = None;
    }
}

#[cfg(feature = "wry-backend")]
pub(crate) fn call_ipc_handler_guarded(handler: &IpcHandler, body: String) -> Vec<BridgePacket> {
    let panic_response = ipc_handler_panic_response(&body);
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(body))) {
        Ok(packets) => packets,
        Err(_) => panic_response,
    }
}

#[cfg(feature = "wry-backend")]
pub(crate) fn ipc_handler_panic_response(body: &str) -> Vec<BridgePacket> {
    let Ok(packet) = vesty_ipc::parse_packet(body) else {
        return Vec::new();
    };
    if packet.kind != BridgeKind::Request
        || validate_bridge_session(&packet.session).is_err()
        || validate_packet_type(&packet.packet_type).is_err()
        || validate_bridge_packet_seq(packet.seq).is_err()
        || validate_bridge_packet_flags(&packet.flags).is_err()
        || packet
            .id
            .as_deref()
            .is_none_or(|id| validate_bridge_packet_id(id).is_err())
        || packet.reply_to.is_some()
        || packet.error.is_some()
    {
        return Vec::new();
    }

    vec![packet.error_to(
        advance_bridge_packet_seq(packet.seq),
        BridgeErrorPayload::new(
            BridgeErrorCode::InternalError,
            "native IPC handler panicked",
            true,
        ),
    )]
}

#[cfg(feature = "wry-backend")]
pub(crate) fn use_dev_url() -> bool {
    ui_env_flag_or_default("VESTY_UI_DEV", cfg!(debug_assertions))
}

#[cfg(feature = "wry-backend")]
pub(crate) fn use_devtools() -> bool {
    ui_env_flag_or_default("VESTY_UI_DEVTOOLS", cfg!(debug_assertions))
}

#[cfg(feature = "wry-backend")]
pub(crate) fn ui_env_flag_or_default(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| ui_env_flag_truthy(&value))
        .unwrap_or(default)
}

#[cfg(feature = "wry-backend")]
pub(crate) fn ui_env_flag_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(feature = "wry-backend")]
pub(crate) fn bounds_for_size(width: u32, height: u32) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(0, 0).into(),
        size: wry::dpi::LogicalSize::new(width, height).into(),
    }
}

#[cfg(feature = "wry-backend")]
pub fn evaluate_packet(
    webview: &wry::WebView,
    packet: &BridgePacket,
) -> Result<(), WryBridgeError> {
    let script = packet_script(packet)?;
    webview.evaluate_script(&script)?;
    Ok(())
}
