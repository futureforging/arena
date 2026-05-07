//! Operator-channel sync (`GET /api/v1/arena/sync`) and structured submissions
//! (`POST /api/v1/arena/message`).

use bytes::Bytes;
use serde_json::{json, Value};
use verity_core::arena::ArenaError;

use crate::transport::HttpTransport;

impl<H: HttpTransport> super::DemoArena<H> {
    /// `GET /api/v1/arena/sync?channel=<id>&index=<n>` with bearer auth.
    /// Returns the content strings of all messages whose `from` is `"operator"`.
    pub(super) async fn operator_sync_inner(
        &self,
        start_index: usize,
    ) -> Result<Vec<String>, ArenaError> {
        self.ensure_psi_session()
            .await?;
        let snap = self.session_snapshot()?;

        let url = format!(
            "{}/api/v1/arena/sync?channel={}&index={start_index}",
            self.base_url
                .trim_end_matches('/'),
            snap.challenge_id,
        );
        let req = Self::build_authed_request(
            Some(
                snap.session_key
                    .as_str(),
            ),
            http::Method::GET,
            &url,
            Bytes::new(),
            true,
        )?;
        let resp = self
            .http
            .exchange(req)
            .await
            .map_err(|e| ArenaError::Other(format!("arena operator sync failed: {e}")))?;
        let status = resp.status();
        let body = resp.into_body();
        if !status.is_success() {
            let text = std::string::String::from_utf8_lossy(&body).into_owned();
            return Err(ArenaError::Other(format!("arena operator sync status {status}: {text}")));
        }
        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| ArenaError::Other(format!("invalid JSON from operator sync: {e}")))?;

        let messages = v["messages"]
            .as_array()
            .ok_or_else(|| {
                ArenaError::Other("operator sync missing 'messages' array".to_string())
            })?;
        let out: Vec<String> = messages
            .iter()
            .filter(|m| m["from"].as_str() == Some("operator"))
            .filter_map(|m| {
                m["content"]
                    .as_str()
                    .map(std::string::ToString::to_string)
            })
            .collect();
        tracing::debug!("operator_sync returned {} operator messages", out.len());
        Ok(out)
    }

    /// `POST /api/v1/arena/message` with bearer auth (no `from` field).
    pub(super) async fn submit_message_inner(
        &self,
        message_type: &str,
        content: &str,
    ) -> Result<(), ArenaError> {
        self.ensure_psi_session()
            .await?;
        let snap = self.session_snapshot()?;

        let url = format!("{}/api/v1/arena/message", self.base_url);
        let body = serde_json::to_vec(&json!({
            "challengeId": snap.challenge_id,
            "messageType": message_type,
            "content": content,
        }))
        .map_err(|e| ArenaError::Other(e.to_string()))?;
        let req = Self::build_authed_request(
            Some(
                snap.session_key
                    .as_str(),
            ),
            http::Method::POST,
            &url,
            Bytes::from(body),
            true,
        )?;
        let resp = self
            .http
            .exchange(req)
            .await
            .map_err(|e| ArenaError::Other(format!("arena message submit failed: {e}")))?;
        let status = resp.status();
        let resp_body = resp.into_body();
        if !status.is_success() {
            let text = std::string::String::from_utf8_lossy(&resp_body).into_owned();
            return Err(ArenaError::Other(format!("arena message submit status {status}: {text}")));
        }
        Ok(())
    }
}
