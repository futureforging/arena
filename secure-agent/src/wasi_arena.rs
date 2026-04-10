use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::{json, Value};

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

#[derive(Debug, Default)]
struct SessionState {
    initialized: bool,
    challenge_id: Option<String>,
    agent_invite: Option<String>,
    next_chat_index: usize,
}

/// Outbound Arena HTTP client (Scaling Trust API shapes).
///
/// Two modes:
/// - **Create + join** (`new`): creates a challenge, takes the first invite, joins. For local stub / dev.
/// - **Join only** (`with_invite`): joins with a pre-assigned invite code. For the production Arena.
#[derive(Clone)]
pub struct WasiArena {
    base_url: String,
    /// Pre-assigned invite code (join-only mode). `None` means create + self-join.
    invite: Option<String>,
    session: Arc<Mutex<SessionState>>,
}

impl WasiArena {
    /// Create + join mode (local stub / dev). Calls `POST /api/v1/challenges/psi`
    /// then joins with the first returned invite.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: normalize_arena_base_url(base_url),
            invite: None,
            session: Arc::new(Mutex::new(SessionState::default())),
        }
    }

    /// Join-only mode (production Arena). Joins with the provided invite code,
    /// skipping challenge creation.
    pub fn with_invite(base_url: &str, invite: &str) -> Self {
        Self {
            base_url: normalize_arena_base_url(base_url),
            invite: Some(invite.to_string()),
            session: Arc::new(Mutex::new(SessionState::default())),
        }
    }

    /// Synchronous send (wraps async via block_on). Use from sync
    /// closures like the ArenaClientTool backing function.
    pub fn send_sync(&self, message: &str) -> Result<String, WasiArenaError> {
        wit_bindgen::block_on(self.send_async(message))
    }

    /// `POST /reset` — clears stub state before a new game (idempotent).
    pub async fn reset_async(&self) -> Result<(), WasiArenaError> {
        // /reset is only available on the local stub; skip for production Arena.
        if self.invite.is_some() {
            return Ok(());
        }

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

        let (challenge_id, agent_invite) = match &self.invite {
            None => self.create_and_join().await?,
            Some(inv) => self.join_only(inv).await?,
        };

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

        self.do_join(&agent_invite).await?;

        Ok((challenge_id, agent_invite))
    }

    /// Joins with the provided invite via `POST /api/v1/arena/join`, parses
    /// `ChallengeID` from the response. Returns `(challenge_id, agent_invite)`.
    async fn join_only(&self, invite: &str) -> Result<(String, String), WasiArenaError> {
        let join_body = self.do_join(invite).await?;

        let challenge_id = join_body["ChallengeID"]
            .as_str()
            .ok_or_else(|| WasiArenaError::Other("join response missing 'ChallengeID'".to_string()))?
            .to_string();

        Ok((challenge_id, invite.to_string()))
    }

    /// `POST /api/v1/arena/join` with the given invite. Returns the parsed response JSON.
    async fn do_join(&self, invite: &str) -> Result<Value, WasiArenaError> {
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
        let resp_body = join_resp.into_body();
        serde_json::from_slice(&resp_body)
            .map_err(|e| WasiArenaError::Other(format!("invalid JSON from join: {e}")))
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
