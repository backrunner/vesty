use thiserror::Error;
use vesty_ipc::BridgePacket;
use vesty_params::ParamSpecError;

pub const MAX_SUBSCRIPTIONS: usize = 256;
pub const MAX_SUBSCRIPTION_TOPIC_BYTES: usize = 128;
pub const MAX_PENDING_PARAM_GESTURES: usize = 1024;
pub const MAX_PARAM_GESTURE_ID_BYTES: usize = 128;
pub const MAX_CONFIG_KEY_BYTES: usize = 128;
pub const MAX_CONFIG_ENTRIES: usize = 256;

pub trait BridgeTransport {
    fn send(&mut self, packet: &BridgePacket) -> Result<(), BridgeTransportError>;

    fn send_batch(&mut self, packets: &[BridgePacket]) -> Result<(), BridgeTransportError> {
        for packet in packets {
            self.send(packet)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
#[error("bridge transport error: {message}")]
pub struct BridgeTransportError {
    pub message: String,
}

impl BridgeTransportError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum BridgeRuntimeError {
    #[error("ipc parse error: {0}")]
    Ipc(#[from] vesty_ipc::IpcError),
    #[error("transport error: {0}")]
    Transport(#[from] BridgeTransportError),
}

#[derive(Debug, Error)]
pub enum BridgeRuntimeCreateError {
    #[error("invalid parameter schema: {0}")]
    ParamSpec(#[from] ParamSpecError),
    #[error("{0}")]
    Session(&'static str),
}

#[derive(Default)]
pub struct MemoryTransport {
    pub sent: Vec<BridgePacket>,
}

impl BridgeTransport for MemoryTransport {
    fn send(&mut self, packet: &BridgePacket) -> Result<(), BridgeTransportError> {
        self.sent.push(packet.clone());
        Ok(())
    }
}
