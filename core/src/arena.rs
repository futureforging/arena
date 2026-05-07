//! Arena port: how an agent interacts with an arena server (peer-to-peer chat plus
//! operator channel for structured challenge messages).

use std::future::Future;

/// Failure from arena interactions (transport, protocol, or response parsing).
#[derive(Clone, Debug)]
pub enum ArenaError {
    /// Adapter-specific failure message.
    Other(String),
}

impl std::fmt::Display for ArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArenaError::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for ArenaError {}

/// How an agent reaches an arena. Adapters decide the wire protocol; the trait is the domain port.
pub trait Arena: Clone + Send + Sync + 'static {
    /// Resets per-game adapter state when supported (e.g. local stubs); arenas without a
    /// guest-visible reset return `Ok(())`.
    fn reset_async(&self) -> impl Future<Output = Result<(), ArenaError>> + Send;

    /// Posts a chat message to the peer and waits for the peer's reply.
    fn send_async(&self, message: &str) -> impl Future<Output = Result<String, ArenaError>> + Send;

    /// Posts a chat message and returns immediately, without waiting for a peer reply.
    /// Use only for terminal messages (e.g. a closing "Goodbye.") where there is no
    /// expectation of a follow-up. Calling this in mid-protocol would strand the agent's
    /// transcript out of sync with the chat channel.
    fn send_only_async(&self, message: &str)
        -> impl Future<Output = Result<(), ArenaError>> + Send;

    /// Waits for and returns the next chat message from the peer.
    fn receive_async(&self) -> impl Future<Output = Result<String, ArenaError>> + Send;

    /// Fetches operator messages addressed to this agent on the arena channel.
    /// Returns the raw `content` string of each operator message in order, starting
    /// at `start_index` (per-recipient, like a chat sync index).
    fn operator_sync_async(
        &self,
        start_index: usize,
    ) -> impl Future<Output = Result<Vec<String>, ArenaError>> + Send;

    /// Submits a structured answer to the operator channel.
    /// `message_type` is the challenge's method name (e.g. `"guess"` for PSI).
    /// `content` is whatever string the challenge expects (for PSI, a JSON array of
    /// numbers as a string).
    fn submit_message_async(
        &self,
        message_type: &str,
        content: &str,
    ) -> impl Future<Output = Result<(), ArenaError>> + Send;
}
