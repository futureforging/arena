use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Blocking HTTP client for JSON request bodies; returns raw response bytes on success.
///
/// On success, [`JsonHttp::post_json`](JsonHttp::post_json) returns the response body **without**
/// interpreting JSON—callers parse for their API. On failure, errors are [`String`] messages:
/// transport errors use [`std::error::Error::to_string`], and non-success HTTP statuses use
/// `HTTP {status}: {body}` with the response body as lossy UTF-8.
pub struct JsonHttp {
    client: Client,
}

impl JsonHttp {
    /// Builds a client with the default request timeout (120 seconds).
    pub fn new() -> Self {
        Self::with_timeout(DEFAULT_TIMEOUT)
    }

    /// Builds a client with the given request timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
        }
    }

    /// POSTs JSON to `url` with the given headers and returns the response body on HTTP success.
    pub fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Vec<u8>, String> {
        let mut req = self
            .client
            .post(url)
            .json(body);
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        let response = req
            .send()
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let bytes = response
            .bytes()
            .map_err(|e| e.to_string())?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&bytes);
            return Err(format!("HTTP {status}: {text}"));
        }
        Ok(bytes.to_vec())
    }
}
