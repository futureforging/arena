//! Ed25519 signing via the host signer service. The guest never holds a private key; instead it
//! calls a localhost HTTP service (`/pubkey`, `/sign`) that performs signing on its behalf.

use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use http::StatusCode;
use serde_json::{json, Value};
use verity_core::arena::ArenaError;

use crate::transport::HttpTransport;

impl<H: HttpTransport> super::DemoArena<H> {
    pub(super) async fn signer_get_pubkey_hex(&self) -> Result<String, ArenaError> {
        let url = format!(
            "{}/pubkey",
            self.signer_url
                .trim_end_matches('/')
        );
        let request =
            Self::build_authed_request(None, http::Method::GET, &url, Bytes::new(), false)?;
        let response = self
            .http
            .exchange(request)
            .await
            .map_err(|e| ArenaError::Other(format!("signer pubkey request failed: {e}")))?;
        if response.status() != StatusCode::OK {
            return Err(ArenaError::Other(format!("signer pubkey status: {}", response.status())));
        }
        let body = response.into_body();
        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| ArenaError::Other(format!("invalid JSON from signer /pubkey: {e}")))?;
        v["publicKey"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| {
                ArenaError::Other("signer pubkey response missing publicKey".to_string())
            })
    }

    pub(super) async fn signer_sign_message(&self, message: &str) -> Result<String, ArenaError> {
        let url = format!(
            "{}/sign",
            self.signer_url
                .trim_end_matches('/')
        );
        let body = serde_json::to_vec(&json!({ "message": message }))
            .map_err(|e| ArenaError::Other(e.to_string()))?;
        let request =
            Self::build_authed_request(None, http::Method::POST, &url, Bytes::from(body), false)?;
        let response = self
            .http
            .exchange(request)
            .await
            .map_err(|e| ArenaError::Other(format!("signer /sign failed: {e}")))?;
        if response.status() != StatusCode::OK {
            return Err(ArenaError::Other(format!("signer /sign status: {}", response.status())));
        }
        let resp_body = response.into_body();
        let v: Value = serde_json::from_slice(&resp_body)
            .map_err(|e| ArenaError::Other(format!("invalid JSON from signer /sign: {e}")))?;
        v["signature"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| ArenaError::Other("signer /sign missing signature".to_string()))
    }

    pub(super) fn timestamp_ms_now() -> Result<u64, ArenaError> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ArenaError::Other(format!("arena wall clock unavailable: {e}")))?;
        Ok(ms.as_millis() as u64)
    }
}
