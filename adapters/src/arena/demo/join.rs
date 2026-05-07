//! Signed `/api/v1/arena/join` and best-effort username registration.

use bytes::Bytes;
use serde_json::{json, Value};
use verity_core::arena::ArenaError;

use crate::transport::HttpTransport;

impl<H: HttpTransport> super::DemoArena<H> {
    pub(super) async fn arena_join_signed(
        &self,
        public_key_hex: &str,
        signature_hex: &str,
        timestamp_ms: u64,
    ) -> Result<Value, ArenaError> {
        let url = format!("{}/api/v1/arena/join", self.base_url);
        let invite = self
            .invite
            .clone();
        let body = serde_json::to_vec(&json!({
            "invite": invite,
            "publicKey": public_key_hex,
            "signature": signature_hex,
            "timestamp": timestamp_ms,
        }))
        .map_err(|e| ArenaError::Other(e.to_string()))?;

        let request =
            Self::build_authed_request(None, http::Method::POST, &url, Bytes::from(body), false)?;
        let response = self
            .http
            .exchange(request)
            .await
            .map_err(|e| ArenaError::Other(format!("arena join failed: {e}")))?;
        let status = response.status();
        let resp_body = response.into_body();

        if !status.is_success() {
            let text = std::string::String::from_utf8_lossy(&resp_body).into_owned();
            return Err(ArenaError::Other(format!("arena join status {status}: {text}")));
        }

        serde_json::from_slice(&resp_body)
            .map_err(|e| ArenaError::Other(format!("invalid JSON from join: {e}")))
    }

    pub(super) async fn set_username_best_effort(&self, session_key: &str) {
        let public_key_hex = match self
            .signer_get_pubkey_hex()
            .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("set username: signer pubkey failed: {e}");
                return;
            },
        };
        let timestamp_ms = match Self::timestamp_ms_now() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("set username: timestamp failed: {e}");
                return;
            },
        };

        // TODO: canonical message format for POST /api/v1/users is undocumented and we
        // have not been able to derive it. All of the following return 401 Invalid
        // signature (with bearer auth + signed envelope, body containing publicKey,
        // signature, timestamp, username, model):
        //   arena:v1:users:update:{username}:{model}:{timestamp}
        //   arena:v1:users:update:{username}:{timestamp}
        //   arena:v1:users:{username}:{timestamp}
        //   arena:v1:users:update:{timestamp}
        //   arena:v1:user:update:{username}:{timestamp}
        //   arena:v1:profile:update:{username}:{timestamp}
        // userId (per the GET /api/v1/users listing) is a 64-hex-char SHA-256-shape
        // value, almost certainly SHA-256 of the SPKI DER pubkey bytes — a userId-based
        // canonical message is the next thing to try if/when we get the spec.
        // Awaiting confirmation from arena-engine maintainer.
        let message = format!("arena:v1:users:update:{}:{}", self.username, timestamp_ms);

        let signature_hex = match self
            .signer_sign_message(&message)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("set username: signer sign failed: {e}");
                return;
            },
        };

        let url = format!("{}/api/v1/users", self.base_url);
        let body = match serde_json::to_vec(&json!({
            "username": self.username,
            "model": self.model,
            "publicKey": public_key_hex,
            "signature": signature_hex,
            "timestamp": timestamp_ms,
        })) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("set username: serialize body failed: {e}");
                return;
            },
        };

        let request = match Self::build_authed_request(
            Some(session_key),
            http::Method::POST,
            &url,
            Bytes::from(body),
            true,
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("set username: build request failed: {e}");
                return;
            },
        };

        match self
            .http
            .exchange(request)
            .await
        {
            Ok(response) => {
                let status = response.status();
                let body_vec = response.into_body();
                if status.is_success() {
                    eprintln!("set arena username to '{}'", self.username);
                } else {
                    let text = std::string::String::from_utf8_lossy(&body_vec).into_owned();
                    eprintln!("set username failed: status={status} body={text}");
                }
            },
            Err(e) => eprintln!("set username HTTP error: {e}"),
        }
    }
}
