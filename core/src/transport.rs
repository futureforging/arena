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
/// Infrastructure adapters (for example Omnia WASI HTTP) implement this port;
/// Claude LLM adapters use it for the Anthropic Messages API.
pub trait PostJsonTransport {
    /// POSTs JSON to `url` with the given headers and returns the response body on HTTP success.
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Vec<u8>, TransportError>;
}

/// Owned [`PostJsonTransport`] for injection into adapters that store HTTP behind a trait object.
pub type BoxedPostJsonTransport = Box<dyn PostJsonTransport + Send + Sync>;

/// Converts a concrete [`PostJsonTransport`] or an existing boxed transport into a single
/// [`BoxedPostJsonTransport`] without double-boxing.
pub trait IntoBoxedPostJsonTransport {
    /// Consumes `self` and returns one owned trait object.
    fn into_boxed_post_json_transport(self) -> BoxedPostJsonTransport;
}

impl<T: PostJsonTransport + Send + Sync + 'static> IntoBoxedPostJsonTransport for T {
    fn into_boxed_post_json_transport(self) -> BoxedPostJsonTransport {
        Box::new(self)
    }
}

impl IntoBoxedPostJsonTransport for BoxedPostJsonTransport {
    fn into_boxed_post_json_transport(self) -> BoxedPostJsonTransport {
        self
    }
}
