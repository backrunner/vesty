use serde::{Deserialize, Serialize};
use thiserror::Error;
pub use vesty_core::UiDescriptor;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorConstraints {
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub resizable: bool,
}

#[derive(Debug, Error)]
pub enum UiError {
    #[error("webview runtime is unavailable: {0}")]
    RuntimeUnavailable(String),
    #[error("editor parent handle is unsupported")]
    UnsupportedParent,
    #[error("bridge error: {0}")]
    Bridge(String),
}

pub trait EditorRuntime {
    type Parent;

    fn attach(&mut self, parent: Self::Parent, descriptor: &UiDescriptor) -> Result<(), UiError>;
    fn resize(&mut self, size: EditorSize) -> Result<(), UiError>;
    fn detach(&mut self);
}
