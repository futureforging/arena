//! Shared [`ArenaTransport`] seam and error type for stub vs production Arena adapters.

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

/// Outbound Arena transport (local stub or production signed join).
pub trait ArenaTransport: Clone + Send + Sync + 'static {
    fn reset_async(&self) -> impl Future<Output = Result<(), WasiArenaError>> + Send;
    fn send_async(
        &self,
        message: &str,
    ) -> impl Future<Output = Result<String, WasiArenaError>> + Send;

    /// Receive a single peer message without sending. Used by second-mover self-play.
    /// Implementations must poll until a peer message arrives or fail with a clear
    /// timeout error. Implementations that don't support self-play (e.g. the local
    /// stub) return an error.
    fn receive_async(&self) -> impl Future<Output = Result<String, WasiArenaError>> + Send;

    /// Synchronous wrapper for tool callbacks (uses `block_on` for async WASI I/O).
    fn send_sync(&self, message: &str) -> Result<String, WasiArenaError> {
        wit_bindgen::block_on(ArenaTransport::send_async(self, message))
    }
}
