//! Sync wrapper around [`verity_core::arena::Arena::send_async`] for tool callbacks.
//!
//! [`verity_core::tool::Tool::execute`] is synchronous, but the WASI guest's HTTP path is async.
//! This is the only place in the workspace that calls [`wit_bindgen::block_on`] for arena I/O —
//! the [`Arena`] trait itself stays async-only.

use verity_core::arena::{Arena, ArenaError};

/// Sends `message` to the peer and blocks until the peer's reply arrives.
pub fn send_sync<A: Arena>(arena: &A, message: &str) -> Result<String, ArenaError> {
    wit_bindgen::block_on(arena.send_async(message))
}
