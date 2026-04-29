//! Shared [`ArenaTransport`] seam and error type for production Arena adapters.

use std::future::Future;

/// Failure from outbound arena HTTP or response parsing in the WASI guest.
#[derive(Clone, Debug)]
pub enum WasiArenaError {
    /// Adapter-specific failure message.
    Other(String),
}

impl std::fmt::Display for WasiArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasiArenaError::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for WasiArenaError {}

/// Outbound Arena transport (production signed join).
pub trait ArenaTransport: Clone + Send + Sync + 'static {
    fn reset_async(&self) -> impl Future<Output = Result<(), WasiArenaError>> + Send;

    fn send_async(
        &self,
        message: &str,
    ) -> impl Future<Output = Result<String, WasiArenaError>> + Send;

    /// Post a chat message and return immediately, without polling for a peer reply.
    /// Use only for terminal messages (e.g. a closing "Goodbye.") where there is no
    /// expectation of a follow-up from the peer. Calling this in mid-protocol would
    /// strand the agent's transcript out of sync with the chat channel.
    fn send_only_async(
        &self,
        message: &str,
    ) -> impl Future<Output = Result<(), WasiArenaError>> + Send;

    fn receive_async(&self) -> impl Future<Output = Result<String, WasiArenaError>> + Send;

    /// Fetch operator messages addressed to this agent on the arena channel.
    /// Returns the raw `content` string of each operator message in order, starting
    /// at `start_index` (per-recipient, like `chat/sync`'s server-side index).
    fn operator_sync_async(
        &self,
        start_index: usize,
    ) -> impl Future<Output = Result<Vec<String>, WasiArenaError>> + Send;

    /// Submit a structured answer to the operator channel.
    /// `message_type` is the challenge's method name (e.g. `"guess"` for PSI).
    /// `content` is whatever string the challenge expects (for PSI, a JSON array of
    /// numbers as a string).
    fn submit_message_async(
        &self,
        message_type: &str,
        content: &str,
    ) -> impl Future<Output = Result<(), WasiArenaError>> + Send;

    /// Synchronous wrapper for tool callbacks (uses `block_on` for async WASI I/O).
    fn send_sync(&self, message: &str) -> Result<String, WasiArenaError> {
        wit_bindgen::block_on(ArenaTransport::send_async(self, message))
    }
}
