use serde::{Deserialize, Serialize};

use crate::contains_control_chars;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginKind {
    AudioEffect,
    Instrument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: &'static str,
    pub vendor: &'static str,
    pub url: &'static str,
    pub email: &'static str,
    pub version: &'static str,
    pub class_id: [u8; 16],
    pub kind: PluginKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AudioOutputBus {
    pub name: &'static str,
    pub channels: u16,
}

impl AudioOutputBus {
    pub const fn new(name: &'static str, channels: u16) -> Self {
        Self { name, channels }
    }

    pub const fn mono(name: &'static str) -> Self {
        Self::new(name, 1)
    }

    pub const fn stereo(name: &'static str) -> Self {
        Self::new(name, 2)
    }

    pub fn is_valid(&self) -> bool {
        !self.name.is_empty()
            && !contains_control_chars(self.name)
            && matches!(self.channels, 1 | 2)
    }
}

pub static DEFAULT_AUDIO_OUTPUT_BUSES: [AudioOutputBus; 1] = [AudioOutputBus::stereo("Output")];
