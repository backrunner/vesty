use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StateError {
    #[error("state serialization failed: {0}")]
    Serialize(String),
    #[error("state deserialization failed: {0}")]
    Deserialize(String),
    #[error("custom state error: {0}")]
    Custom(String),
}

impl StateError {
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }
}

pub trait PluginState {
    type State: Serialize + DeserializeOwned + 'static;

    fn save_state(&self) -> Self::State;
    fn load_state(&self, state: Self::State) -> Result<(), StateError>;
}

pub fn save_plugin_state<P: PluginState>(plugin: &P) -> Result<serde_json::Value, StateError> {
    serde_json::to_value(plugin.save_state())
        .map_err(|error| StateError::Serialize(error.to_string()))
}

pub fn load_plugin_state<P: PluginState>(
    plugin: &P,
    value: serde_json::Value,
) -> Result<(), StateError> {
    let state = serde_json::from_value(value)
        .map_err(|error| StateError::Deserialize(error.to_string()))?;
    plugin.load_state(state)
}
