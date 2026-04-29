//! Stub Arena transport: create challenge + self-join (`arena-stub` / local dev).

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::{json, Value};

use crate::arena_transport::{ArenaTransport, WasiArenaError};
use crate::arena_url::normalize_arena_base_url;

#[derive(Debug, Default)]
struct SessionState {
    initialized: bool,
    challenge_id: Option<String>,
    agent_invite: Option<String>,
    next_chat_index: usize,
}

/// Outbound Arena HTTP client for the local stub (create + self-join path).
#[derive(Clone)]
pub struct StubArena {
    base_url: String,
    session: Arc<Mutex<SessionState>>,
}

impl StubArena {
    /// Create + join mode (local stub / dev). Calls `POST /api/v1/challenges/psi`
    /// then joins with the first returned invite.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: normalize_arena_base_url(base_url),
            session: Arc::new(Mutex::new(SessionState::default())),
        }
    }

    /// `POST /reset` — clears stub state before a new game (idempotent).
    pub async fn reset_async(&self) -> Result<(), WasiArenaError> {
        {
            let mut g = self
                .session
                .lock()
                .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
            g.initialized = false;
            g.challenge_id = None;
            g.agent_invite = None;
            g.next_chat_index = 0;
        }

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
        let _body = response.into_body();

        if status == StatusCode::NO_CONTENT || status == StatusCode::OK {
            Ok(())
        } else {
            Err(WasiArenaError::Other(format!(
                "arena reset unexpected status: {status}"
            )))
        }
    }

    async fn ensure_psi_session(&self) -> Result<(), WasiArenaError> {
        let need_init = {
            let g = self
                .session
                .lock()
                .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
            !g.initialized
        };

        if !need_init {
            return Ok(());
        }

        let (challenge_id, agent_invite) = self.create_and_join().await?;

        let mut g = self
            .session
            .lock()
            .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
        g.initialized = true;
        g.challenge_id = Some(challenge_id);
        g.agent_invite = Some(agent_invite);
        g.next_chat_index = 0;
        Ok(())
    }

    /// Creates a challenge via `POST /api/v1/challenges/psi`, parses the response
    /// for `id` and `invites`, then joins with `invites[0]`. Returns `(challenge_id, agent_invite)`.
    async fn create_and_join(&self) -> Result<(String, String), WasiArenaError> {
        let create_url = format!("{}/api/v1/challenges/psi", self.base_url);
        let create_req = http::Request::builder()
            .method(http::Method::POST)
            .uri(&create_url)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::new()))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;
        let create_resp = omnia_wasi_http::handle(create_req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena create challenge failed: {e}")))?;
        if !create_resp.status().is_success() {
            return Err(WasiArenaError::Other(format!(
                "arena create challenge status: {}",
                create_resp.status()
            )));
        }
        let create_body = create_resp.into_body();
        let create_json: Value = serde_json::from_slice(&create_body).map_err(|e| {
            WasiArenaError::Other(format!("invalid JSON from create challenge: {e}"))
        })?;

        let challenge_id = create_json["id"]
            .as_str()
            .ok_or_else(|| WasiArenaError::Other("create challenge response missing 'id'".to_string()))?
            .to_string();

        let agent_invite = create_json["invites"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .ok_or_else(|| {
                WasiArenaError::Other("create challenge response missing 'invites[0]'".to_string())
            })?
            .to_string();

        self.do_join_stub(&agent_invite).await?;

        Ok((challenge_id, agent_invite))
    }

    /// `POST /api/v1/arena/join` with the stub body (invite-only). Response is not required for state.
    async fn do_join_stub(&self, invite: &str) -> Result<(), WasiArenaError> {
        let join_url = format!("{}/api/v1/arena/join", self.base_url);
        let body = json!({ "invite": invite });
        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| WasiArenaError::Other(e.to_string()))?;
        let join_req = http::Request::builder()
            .method(http::Method::POST)
            .uri(&join_url)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(body_bytes)))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;
        let join_resp = omnia_wasi_http::handle(join_req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena join failed: {e}")))?;
        if !join_resp.status().is_success() {
            return Err(WasiArenaError::Other(format!(
                "arena join status: {}",
                join_resp.status()
            )));
        }
        Ok(())
    }

    /// Returns `(challenge_id, agent_invite)` from the initialized session, or errors.
    fn session_ids(&self) -> Result<(String, String), WasiArenaError> {
        let g = self
            .session
            .lock()
            .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
        let cid = g.challenge_id.clone().ok_or_else(|| {
            WasiArenaError::Other("arena session not initialized (no challenge_id)".to_string())
        })?;
        let inv = g.agent_invite.clone().ok_or_else(|| {
            WasiArenaError::Other("arena session not initialized (no agent_invite)".to_string())
        })?;
        Ok((cid, inv))
    }

    /// Sends a chat line as the agent invite and returns the peer line from `GET /api/v1/chat/sync`.
    pub async fn send_async(&self, message: &str) -> Result<String, WasiArenaError> {
        self.ensure_psi_session().await?;

        let (challenge_id, agent_invite) = self.session_ids()?;

        let start_index = {
            let g = self
                .session
                .lock()
                .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
            g.next_chat_index
        };

        let send_url = format!("{}/api/v1/chat/send", self.base_url);
        let send_body = json!({
            "channel": challenge_id,
            "from": agent_invite,
            "content": message,
        });
        let send_bytes =
            serde_json::to_vec(&send_body).map_err(|e| WasiArenaError::Other(e.to_string()))?;
        let send_req = http::Request::builder()
            .method(http::Method::POST)
            .uri(&send_url)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(send_bytes)))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let send_resp = omnia_wasi_http::handle(send_req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena chat send failed: {e}")))?;
        let send_status = send_resp.status();
        let _send_body = send_resp.into_body();
        if !send_status.is_success() {
            return Err(WasiArenaError::Other(format!(
                "arena chat send status: {send_status}"
            )));
        }

        let sync_url = format!(
            "{}/api/v1/chat/sync?channel={}&from={}&index={}",
            self.base_url, challenge_id, agent_invite, start_index
        );
        let sync_req = http::Request::builder()
            .method(http::Method::GET)
            .uri(&sync_url)
            .body(Full::new(Bytes::new()))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let sync_resp = omnia_wasi_http::handle(sync_req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena chat sync failed: {e}")))?;
        let sync_status = sync_resp.status();
        let sync_body = sync_resp.into_body();
        if !sync_status.is_success() {
            return Err(WasiArenaError::Other(format!(
                "arena chat sync status: {sync_status}"
            )));
        }
        let v: Value = serde_json::from_slice(&sync_body)
            .map_err(|e| WasiArenaError::Other(format!("invalid JSON from arena chat sync: {e}")))?;

        let messages = v["messages"].as_array().ok_or_else(|| {
            WasiArenaError::Other("arena chat sync missing 'messages' array".to_string())
        })?;

        let mut last_index: Option<usize> = None;
        let mut peer_line: Option<String> = None;

        for m in messages {
            let idx = m["index"]
                .as_u64()
                .map(|u| u as usize)
                .or_else(|| m["index"].as_i64().map(|i| i as usize))
                .ok_or_else(|| {
                    WasiArenaError::Other("arena chat message missing numeric index".to_string())
                })?;
            last_index = Some(idx);
            if m["from"].as_str() != Some(agent_invite.as_str()) {
                peer_line = m["content"]
                    .as_str()
                    .map(std::string::ToString::to_string);
            }
        }

        let reply = peer_line.ok_or_else(|| {
            WasiArenaError::Other("arena chat sync had no peer message".to_string())
        })?;

        if let Some(last) = last_index {
            let mut g = self
                .session
                .lock()
                .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
            g.next_chat_index = last.saturating_add(1);
        }

        Ok(reply)
    }
}

impl ArenaTransport for StubArena {
    fn reset_async(&self) -> impl std::future::Future<Output = Result<(), WasiArenaError>> + Send {
        let s = self.clone();
        async move { s.reset_async().await }
    }

    fn send_async(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<String, WasiArenaError>> + Send {
        let s = self.clone();
        let msg = message.to_string();
        async move { s.send_async(&msg).await }
    }
}
