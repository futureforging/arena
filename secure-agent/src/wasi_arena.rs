use aria_core::arena::{Arena, ArenaError};
use bytes::Bytes;
use http_body_util::Full;
use serde_json::json;

use crate::arena_url::normalize_arena_base_url;

pub struct WasiArena {
    base_url: String,
}

impl WasiArena {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: normalize_arena_base_url(base_url),
        }
    }
}

impl Arena for WasiArena {
    fn send(&self, message: &str) -> Result<String, ArenaError> {
        wit_bindgen::block_on(self.send_async(message))
    }
}

impl WasiArena {
    /// Outbound arena `POST /message` (use from async guest code; avoid [`Arena::send`]'s `block_on` there).
    pub async fn send_async(&self, message: &str) -> Result<String, ArenaError> {
        let url = format!("{}/message", self.base_url);
        let body = json!({"message": message});
        let body_bytes = serde_json::to_vec(&body).map_err(|e| ArenaError::Other(e.to_string()))?;

        let request = http::Request::builder()
            .method(http::Method::POST)
            .uri(&url)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(body_bytes)))
            .map_err(|e| ArenaError::Other(e.to_string()))?;

        let response = omnia_wasi_http::handle(request)
            .await
            .map_err(|e| ArenaError::Other(format!("arena HTTP failed: {e}")))?;
        let response_body = response.into_body();

        let v: serde_json::Value = serde_json::from_slice(&response_body)
            .map_err(|e| ArenaError::Other(format!("invalid JSON from arena: {e}")))?;

        v["reply"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ArenaError::Other("arena response missing 'reply' field".to_string()))
    }
}
