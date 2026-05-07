use std::future::Future;

use bytes::Bytes;

/// Failure from an [`HttpTransport`] call (connect, send, response read, body decode).
#[derive(Clone, Debug)]
pub struct HttpError(pub String);

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HttpError {}

/// HTTP transport port. Adapters implement this to perform a single request/response exchange.
///
/// The shape mirrors the [`http`] crate's typed request/response with a [`Bytes`] body, which is
/// the thinnest contract that still lets consumers build/decode HTTP semantically (method, URL,
/// headers, status). The body is fully buffered in both directions; streaming bodies are out of
/// scope for now.
pub trait HttpTransport: Clone + Send + Sync + 'static {
    /// Sends a request and returns the response body collected into [`Bytes`].
    fn exchange(
        &self,
        request: http::Request<Bytes>,
    ) -> impl Future<Output = Result<http::Response<Bytes>, HttpError>> + Send;
}
