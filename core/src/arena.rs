use std::fmt;

/// Error returned by [`Arena`] operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArenaError {
    /// The peer's response could not be obtained.
    PeerUnavailable(String),
    /// An implementation-specific failure.
    Other(String),
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArenaError::PeerUnavailable(msg) => write!(f, "peer unavailable: {msg}"),
            ArenaError::Other(msg) => f.write_str(msg),
        }
    }
}

/// Port for exchanging messages with a peer in an arena challenge.
///
/// Infrastructure adapters (for example HTTP `ArenaHttpClient` in the agent crate) implement this port;
/// the agent loop in `main` uses it to interact with a counterparty
/// without knowing whether the peer is local, a stub server, or the real
/// [Scaling Trust Arena](https://arena.nicolaos.org).
pub trait Arena {
    /// Sends a message to the peer and returns their reply.
    fn send(&self, message: &str) -> Result<String, ArenaError>;
}
