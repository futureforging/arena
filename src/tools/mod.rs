mod crypto;
mod messaging;
mod negotiation_protocol_picker;
mod strategy_picker;

pub use crypto::CryptoTool;
pub use messaging::{PeerConnection, ReceiveFromPeerTool, SendToPeerTool};
pub use negotiation_protocol_picker::NegotiationProtocolPickerTool;
pub use strategy_picker::StrategyPickerTool;

/// Logs messages in clean output mode. (direction, message) where direction is "sent" or "received".
pub type MessageLogFn = std::sync::Arc<dyn Fn(&str, &str) + Send + Sync>;
