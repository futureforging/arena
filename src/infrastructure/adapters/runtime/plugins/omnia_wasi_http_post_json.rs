//! [`PostJsonTransport`](aria_core::transport::PostJsonTransport) using Omnia’s [`wasi:http`](https://github.com/WebAssembly/wasi-http)
//! host implementation ([`HttpDefault`](omnia_wasi_http::HttpDefault)), which performs outbound HTTP the same way as the Omnia runtime’s HTTP service.

use std::{convert::Infallible, future::poll_fn, pin::Pin, sync::Mutex};

use aria_core::transport::{PostJsonTransport, TransportError};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, Method, Request, Uri};
use http_body_util::{BodyExt, Full};
use omnia_wasi_http::HttpDefault;
use serde_json::Value;
use wasmtime_wasi_http::p3::{bindings::http::types::ErrorCode, WasiHttpCtx};

/// Outbound JSON POST via Omnia [`HttpDefault`](omnia_wasi_http::HttpDefault) (`wasi:http` host path).
pub struct OmniaWasiHttpPostJson {
    inner: Mutex<HttpDefault>,
    rt: tokio::runtime::Runtime,
}

impl OmniaWasiHttpPostJson {
    /// Builds a transport with a current-thread async runtime (same pattern as [`crate::infrastructure::adapters::runtime::OmniaRuntime`]).
    ///
    /// Returns [`std::io::Error`] if the Tokio runtime cannot be created.
    pub fn new() -> Result<Self, std::io::Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            inner: Mutex::new(HttpDefault),
            rt,
        })
    }
}

impl PostJsonTransport for OmniaWasiHttpPostJson {
    fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Vec<u8>, TransportError> {
        let uri: Uri = url
            .parse()
            .map_err(|e| TransportError::Other(format!("invalid URL: {e}")))?;
        let body_bytes =
            serde_json::to_vec(body).map_err(|e| TransportError::Other(e.to_string()))?;

        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(CONTENT_TYPE, "application/json");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }

        let body_stream = Full::new(Bytes::from(body_bytes))
            .map_err(|e: Infallible| match e {})
            .boxed_unsync();

        let request = builder
            .body(body_stream)
            .map_err(|e| TransportError::Other(e.to_string()))?;

        let response_fut = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| {
                    TransportError::Other("WASI HTTP transport lock poisoned".to_string())
                })?;
            WasiHttpCtx::send_request(
                &mut *guard,
                request,
                None,
                Box::new(async move { Ok::<(), ErrorCode>(()) }),
            )
        };

        let mut response_fut = response_fut;
        let response_result = self
            .rt
            .block_on(poll_fn(move |cx| {
                // SAFETY: `response_fut` is a local `Box<dyn Future + Send>` that is not moved
                // until after `block_on` returns; we only pin it for polling (`dyn Future` is `!Unpin`).
                unsafe { Pin::new_unchecked(response_fut.as_mut()).poll(cx) }
            }))
            .map_err(|e| TransportError::Other(e.to_string()))?;

        let (response, _trailers) = response_result;
        let (parts, resp_body) = response.into_parts();
        let status = parts.status;

        let collected = self
            .rt
            .block_on(async {
                resp_body
                    .collect()
                    .await
            })
            .map_err(|e| TransportError::Other(format!("{e:?}")))?;

        let bytes = collected.to_bytes();

        if !status.is_success() {
            let text = String::from_utf8_lossy(&bytes);
            return Err(TransportError::Other(format!("HTTP {status}: {text}")));
        }

        Ok(bytes.to_vec())
    }
}
