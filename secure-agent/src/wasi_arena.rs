use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::json;

use crate::arena_url::normalize_arena_base_url;

/// Failure from outbound arena HTTP or response parsing in the WASI guest.
#[derive(Clone, Debug)]
pub enum WasiArenaError {
    /// Adapter-specific failure message.
    Other(String),
}

impl std::fmt::Display for WasiArenaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasiArenaError::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for WasiArenaError {}

pub struct WasiArena {
    base_url: String,
}

impl WasiArena {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: normalize_arena_base_url(base_url),
        }
    }

    /// Synchronous send (wraps async via block_on). Use from sync
    /// closures like the ArenaClientTool backing function.
    pub fn send_sync(&self, message: &str) -> Result<String, WasiArenaError> {
        wit_bindgen::block_on(self.send_async(message))
    }

    /// `POST /reset` — clears stub peer state before a new game (idempotent).
    pub async fn reset_async(&self) -> Result<(), WasiArenaError> {
        let url = format!("{}/reset", self.base_url);
        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri(&url)
            .body(Full::new(Bytes::new()))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let response = omnia_wasi_http::handle(request)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena HTTP reset failed: {e}")))?;
        let status = response.status();
        let _body = response
            .into_body();

        if status == StatusCode::NO_CONTENT || status == StatusCode::OK {
            Ok(())
        } else {
            Err(WasiArenaError::Other(format!(
                "arena reset unexpected status: {status}"
            )))
        }
    }

    /// Outbound arena `POST /message` (use from async game loops to avoid deadlocks).
    pub async fn send_async(&self, message: &str) -> Result<String, WasiArenaError> {
        let url = format!("{}/message", self.base_url);
        let body = json!({"message": message});
        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri(&url)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(body_bytes)))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let response = omnia_wasi_http::handle(request)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena HTTP failed: {e}")))?;
        let response_body = response.into_body();

        let v: serde_json::Value = serde_json::from_slice(&response_body)
            .map_err(|e| WasiArenaError::Other(format!("invalid JSON from arena: {e}")))?;

        v["reply"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                WasiArenaError::Other("arena response missing 'reply' field".to_string())
            })
    }
}
