use std::fmt;

use serde_json::Value;

/// Failure from an outbound JSON POST (transport, HTTP, or adapter-specific text).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportError {
    /// Adapter-specific failure message (includes non-success HTTP status bodies when applicable).
    Other(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Other(msg) => f.write_str(msg),
        }
    }
}

/// Outbound HTTP POST with a JSON body; returns raw response bytes on HTTP success.
///
/// Infrastructure adapters (e.g. [`crate::infrastructure::adapters::transport::JsonHttp`]) implement this port;
/// [`crate::infrastructure::adapters::llm::ClaudeLlm`] uses it for the Anthropic Messages API.
pub trait PostJsonTransport {
    /// POSTs JSON to `url` with the given headers and returns the response body on HTTP success.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Vec<u8>, TransportError>;
}
