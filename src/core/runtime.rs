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
/// capabilities the runtime explicitly exposes. For now the only capability
/// is secret retrieval; tool discovery and execution will be added later.
pub trait Runtime {
    /// Retrieves a secret value by `name` from the runtime's secrets capability.
    fn get_secret(&self, name: &str) -> Result<String, RuntimeError>;
}
