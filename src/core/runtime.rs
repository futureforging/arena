use serde_json::Value;

use super::transport::{BoxedPostJsonTransport, TransportError};

/// Secret name used with [`Runtime::get_secret`] for the Anthropic API key (must match the vault secret id in infrastructure).
pub const ANTHROPIC_API_KEY_SECRET: &str = "anthropic_api_key";

/// Error returned by [`Runtime`] operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    /// The requested item was not found.
    NotFound(String),
    /// An implementation-specific failure.
    Other(String),
}

/// Secure runtime port: mediates all privileged operations on behalf of the agent.
///
/// The runtime is the enforcement boundary — the agent can only access
/// capabilities the runtime explicitly exposes (secrets, outbound HTTP, and
/// factories for components that take owned transports).
pub trait Runtime {
    /// Retrieves a secret value by `name` from the runtime's secrets capability.
    fn get_secret(&self, name: &str) -> Result<String, RuntimeError>;

    /// POSTs JSON to `url` with the given headers and returns the response body on HTTP success.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Vec<u8>, TransportError>;

    /// Creates an owned [`PostJsonTransport`] backed by this runtime's HTTP capability.
    ///
    /// Used by factories like [`crate::infrastructure::adapters::agent::SecureAgent::new`]
    /// to provide HTTP to components that take ownership of a transport.
    fn create_transport(&self) -> Result<BoxedPostJsonTransport, RuntimeError>;
}
