//! [`WasiHttpAdapter`]: implements [`HttpTransport`] against `omnia_wasi_http`.
//!
//! Sole place in the workspace that touches the WASI HTTP outbound surface; consumers
//! (`DemoArena`, `WasiLlm` once migrated) reach the network through the [`HttpTransport`]
//! port instead.

use std::future::Future;

use bytes::Bytes;
use http_body_util::Full;

use verity_adapters::transport::{HttpError, HttpTransport};

/// HTTP transport backed by `omnia_wasi_http`. Lives in `secure-agent` (not `verity-adapters`)
/// because it depends on WASI guest crates.
#[derive(Clone)]
pub struct WasiHttpAdapter;

impl WasiHttpAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl HttpTransport for WasiHttpAdapter {
    fn exchange(
        &self,
        request: http::Request<Bytes>,
    ) -> impl Future<Output = Result<http::Response<Bytes>, HttpError>> + Send {
        async move {
            let (parts, body) = request.into_parts();
            let req = http::Request::from_parts(parts, Full::new(body));
            let resp = omnia_wasi_http::handle(req)
                .await
                .map_err(|e| HttpError(format!("{e}")))?;
            let (parts, body) = resp.into_parts();
            Ok(http::Response::from_parts(parts, Bytes::from(body)))
        }
    }
}
