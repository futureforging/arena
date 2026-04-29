//! Production Arena transport: signed join + bearer session; no private key in the guest.

use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use serde_json::{json, Value};

use crate::{
    arena_transport::{ArenaTransport, WasiArenaError},
    arena_url::normalize_arena_base_url,
};

/// Maximum number of `chat/sync` polls per turn before giving up. Each poll is a
/// network round-trip to the Arena, which provides natural backpressure; with
/// typical RTTs of 100–500ms this yields a per-turn deadline of roughly 1–5 minutes.
const MAX_CHAT_SYNC_POLLS: u32 = 600;

#[derive(Debug, Default)]
struct SessionState {
    initialized: bool,
    challenge_id: Option<String>,
    session_key: Option<String>,
    self_user_id: Option<String>,
    next_chat_index: usize,
}

#[derive(Clone)]
pub struct ProductionArena {
    base_url: String,
    signer_url: String,
    invite: String,
    username: String,
    session: Arc<Mutex<SessionState>>,
}

impl ProductionArena {
    pub fn new(arena_url: &str, invite: &str, signer_url: &str, username: &str) -> Self {
        Self {
            base_url: normalize_arena_base_url(arena_url),
            signer_url: normalize_arena_base_url(signer_url),
            invite: invite.to_string(),
            username: username.to_string(),
            session: Arc::new(Mutex::new(SessionState::default())),
        }
    }

    /// No-op — production Arena has no guest-visible `/reset`.
    pub async fn reset_async(&self) -> Result<(), WasiArenaError> {
        Ok(())
    }

    /// `GET /api/v1/arena/sync?channel=<id>&index=<n>` with bearer auth.
    /// Returns the content strings of all messages whose `from` is `"operator"`.
    pub async fn operator_sync_async(
        &self,
        start_index: usize,
    ) -> Result<Vec<String>, WasiArenaError> {
        self.ensure_psi_session().await?;
        let (challenge_id, session_key) = self.session_snapshot()?;

        let url = format!(
            "{}/api/v1/arena/sync?channel={}&index={}",
            self.base_url.trim_end_matches('/'),
            challenge_id,
            start_index,
        );
        let req = ProductionArena::build_authed_request(
            Some(session_key.as_str()),
            http::Method::GET,
            &url,
            Bytes::new(),
            true,
        )?;
        let resp = omnia_wasi_http::handle(req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena operator sync failed: {e}")))?;
        let status = resp.status();
        let body = resp.into_body();
        if !status.is_success() {
            let text = std::string::String::from_utf8_lossy(&body).into_owned();
            return Err(WasiArenaError::Other(format!(
                "arena operator sync status {status}: {text}"
            )));
        }
        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| WasiArenaError::Other(format!("invalid JSON from operator sync: {e}")))?;

        let messages = v["messages"].as_array().ok_or_else(|| {
            WasiArenaError::Other("operator sync missing 'messages' array".to_string())
        })?;
        let mut out = Vec::new();
        for m in messages {
            if m["from"].as_str() == Some("operator") {
                if let Some(c) = m["content"].as_str() {
                    out.push(c.to_string());
                }
            }
        }
        tracing::debug!("operator_sync returned {} operator messages", out.len());
        Ok(out)
    }

    /// `POST /api/v1/arena/message` with bearer auth (no `from` field).
    pub async fn submit_message_async(
        &self,
        message_type: &str,
        content: &str,
    ) -> Result<(), WasiArenaError> {
        self.ensure_psi_session().await?;
        let (challenge_id, session_key) = self.session_snapshot()?;

        let url = format!("{}/api/v1/arena/message", self.base_url);
        let body = serde_json::to_vec(&json!({
            "challengeId": challenge_id,
            "messageType": message_type,
            "content": content,
        }))
        .map_err(|e| WasiArenaError::Other(e.to_string()))?;
        let req = ProductionArena::build_authed_request(
            Some(session_key.as_str()),
            http::Method::POST,
            &url,
            Bytes::from(body),
            true,
        )?;
        let resp = omnia_wasi_http::handle(req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena message submit failed: {e}")))?;
        let status = resp.status();
        let resp_body = resp.into_body();
        if !status.is_success() {
            let text = std::string::String::from_utf8_lossy(&resp_body).into_owned();
            return Err(WasiArenaError::Other(format!(
                "arena message submit status {status}: {text}"
            )));
        }
        Ok(())
    }

    fn build_authed_request(
        session_key: Option<&str>,
        method: http::Method,
        url: &str,
        body: Bytes,
        require_session: bool,
    ) -> Result<http::Request<Full<Bytes>>, WasiArenaError> {
        if require_session && session_key.is_none() {
            return Err(WasiArenaError::Other(
                "arena transport: missing session key for authenticated request".to_string(),
            ));
        }

        let mut req = http::Request::builder()
            .method(method)
            .uri(url)
            .header("content-type", "application/json");

        if let Some(sk) = session_key {
            req = req.header("Authorization", format!("Bearer {}", sk.trim()));
        }

        req.body(Full::new(body))
            .map_err(|e| WasiArenaError::Other(e.to_string()))
    }

    fn timestamp_ms_now() -> Result<u64, WasiArenaError> {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| WasiArenaError::Other(format!("arena wall clock unavailable: {e}")))?;
        Ok(ms.as_millis() as u64)
    }

    async fn signer_get_pubkey_hex(&self) -> Result<String, WasiArenaError> {
        let url = format!(
            "{}/pubkey",
            self.signer_url
                .trim_end_matches('/')
        );
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri(&url)
            .body(Full::new(Bytes::new()))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;
        let response = omnia_wasi_http::handle(request)
            .await
            .map_err(|e| WasiArenaError::Other(format!("signer pubkey request failed: {e}")))?;
        if response.status() != StatusCode::OK {
            return Err(WasiArenaError::Other(format!(
                "signer pubkey status: {}",
                response.status()
            )));
        }
        let body = response.into_body();
        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| WasiArenaError::Other(format!("invalid JSON from signer /pubkey: {e}")))?;
        v["publicKey"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| {
                WasiArenaError::Other("signer pubkey response missing publicKey".to_string())
            })
    }

    async fn signer_sign_message(&self, message: &str) -> Result<String, WasiArenaError> {
        let url = format!(
            "{}/sign",
            self.signer_url
                .trim_end_matches('/')
        );
        let body = serde_json::to_vec(&json!({ "message": message }))
            .map_err(|e| WasiArenaError::Other(e.to_string()))?;
        let request = ProductionArena::build_authed_request(
            None,
            http::Method::POST,
            &url,
            Bytes::from(body),
            false,
        )?;
        let response = omnia_wasi_http::handle(request)
            .await
            .map_err(|e| WasiArenaError::Other(format!("signer /sign failed: {e}")))?;
        if response.status() != StatusCode::OK {
            return Err(WasiArenaError::Other(format!(
                "signer /sign status: {}",
                response.status()
            )));
        }
        let resp_body = response.into_body();
        let v: Value = serde_json::from_slice(&resp_body)
            .map_err(|e| WasiArenaError::Other(format!("invalid JSON from signer /sign: {e}")))?;
        v["signature"]
            .as_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| WasiArenaError::Other("signer /sign missing signature".to_string()))
    }

    async fn arena_join_signed(
        &self,
        public_key_hex: &str,
        signature_hex: &str,
        timestamp_ms: u64,
    ) -> Result<Value, WasiArenaError> {
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
        .map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let request = ProductionArena::build_authed_request(
            None,
            http::Method::POST,
            &url,
            Bytes::from(body),
            false,
        )?;
        let response = omnia_wasi_http::handle(request)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena join failed: {e}")))?;
        let status = response.status();
        let resp_body = response.into_body();

        if !status.is_success() {
            let text = std::string::String::from_utf8_lossy(&resp_body).into_owned();
            return Err(WasiArenaError::Other(format!("arena join status {status}: {text}")));
        }

        serde_json::from_slice(&resp_body)
            .map_err(|e| WasiArenaError::Other(format!("invalid JSON from join: {e}")))
    }

    async fn set_username_best_effort(&self, session_key: &str) {
        let url = format!("{}/api/v1/users", self.base_url);
        let body = match serde_json::to_vec(&json!({
            "username": self.username,
            "model": crate::wasi_llm::DEFAULT_MODEL,
        })) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("set username: serialize body failed: {e}");
                return;
            }
        };

        let request = match ProductionArena::build_authed_request(
            Some(session_key),
            http::Method::POST,
            &url,
            Bytes::from(body),
            true,
        ) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("set username: build request failed: {e}");
                return;
            }
        };

        match omnia_wasi_http::handle(request).await {
            Ok(response) => {
                let status = response.status();
                let body_vec = response.into_body();
                if status.is_success() {
                    tracing::info!("set arena username to '{}'", self.username);
                } else {
                    let text = std::string::String::from_utf8_lossy(&body_vec).into_owned();
                    tracing::warn!("set username failed: status={status} body={text}");
                }
            }
            Err(e) => tracing::warn!("set username HTTP error: {e}"),
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

        let public_hex = self
            .signer_get_pubkey_hex()
            .await?;
        let timestamp_ms = Self::timestamp_ms_now()?;
        let message = format!("arena:v1:join:{}:{}", self.invite, timestamp_ms);
        let signature_hex = self
            .signer_sign_message(&message)
            .await?;
        let join_body = self
            .arena_join_signed(&public_hex, &signature_hex, timestamp_ms)
            .await?;

        let challenge_id = join_body["ChallengeID"]
            .as_str()
            .ok_or_else(|| WasiArenaError::Other("join response missing ChallengeID".to_string()))?
            .to_string();
        let session_key = join_body["sessionKey"]
            .as_str()
            .ok_or_else(|| WasiArenaError::Other("join response missing sessionKey".to_string()))?
            .to_string();

        self.set_username_best_effort(&session_key).await;

        let mut g = self
            .session
            .lock()
            .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
        g.initialized = true;
        g.challenge_id = Some(challenge_id);
        g.session_key = Some(session_key);
        g.self_user_id = None;
        g.next_chat_index = 0;
        Ok(())
    }

    fn session_snapshot(&self) -> Result<(String, String), WasiArenaError> {
        let g = self
            .session
            .lock()
            .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
        let cid = g
            .challenge_id
            .clone()
            .ok_or_else(|| {
                WasiArenaError::Other("arena session not initialized (no challenge_id)".to_string())
            })?;
        let sk = g
            .session_key
            .clone()
            .ok_or_else(|| {
                WasiArenaError::Other("arena session not initialized (no session_key)".to_string())
            })?;
        Ok((cid, sk))
    }

    /// Identify our own user id by finding the message whose `content` matches what we just sent.
    /// Iterates in reverse so the most recent matching message wins (in case a prior turn
    /// happened to send identical content). No-op once `self_user_id` is set.
    fn maybe_set_self_user_id_from_content(
        &self,
        messages: &[Value],
        sent_content: &str,
    ) -> Result<(), WasiArenaError> {
        let mut g = self
            .session
            .lock()
            .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
        if g.self_user_id.is_some() {
            return Ok(());
        }
        for m in messages.iter().rev() {
            if m["content"].as_str() == Some(sent_content) {
                if let Some(fid) = m["from"].as_str() {
                    g.self_user_id = Some(fid.to_string());
                    return Ok(());
                }
            }
        }
        // Our message hasn't appeared yet (server hasn't echoed it back). Leave
        // self_user_id unset; the next poll will try again.
        Ok(())
    }

    fn self_user_id_owned(&self) -> Result<Option<String>, WasiArenaError> {
        let g = self
            .session
            .lock()
            .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
        Ok(g.self_user_id
            .clone())
    }

    pub async fn send_async(&self, message: &str) -> Result<String, WasiArenaError> {
        self.ensure_psi_session()
            .await?;

        let (challenge_id, session_key) = self.session_snapshot()?;

        let start_index = {
            let g = self
                .session
                .lock()
                .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
            g.next_chat_index
        };

        let send_url = format!("{}/api/v1/chat/send", self.base_url);
        let send_body = serde_json::to_vec(&json!({
            "channel": challenge_id,
            "content": message,
        }))
        .map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let send_req = ProductionArena::build_authed_request(
            Some(session_key.as_str()),
            http::Method::POST,
            &send_url,
            Bytes::from(send_body),
            true,
        )?;

        let send_resp = omnia_wasi_http::handle(send_req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena chat send failed: {e}")))?;
        let send_status = send_resp.status();
        let _send_body = send_resp.into_body();
        if !send_status.is_success() {
            return Err(WasiArenaError::Other(format!("arena chat send status: {send_status}")));
        }

        let sync_url = format!(
            "{}/api/v1/chat/sync?channel={}&index={}",
            self.base_url
                .trim_end_matches('/'),
            challenge_id,
            start_index
        );

        let mut last_index: Option<usize> = None;
        let mut peer_line: Option<String> = None;

        for attempt in 1..=MAX_CHAT_SYNC_POLLS {
            let sync_req = ProductionArena::build_authed_request(
                Some(session_key.as_str()),
                http::Method::GET,
                &sync_url,
                Bytes::new(),
                true,
            )?;

            let sync_resp = omnia_wasi_http::handle(sync_req)
                .await
                .map_err(|e| WasiArenaError::Other(format!("arena chat sync failed: {e}")))?;
            let sync_status = sync_resp.status();
            let sync_body = sync_resp.into_body();
            if !sync_status.is_success() {
                return Err(WasiArenaError::Other(format!(
                    "arena chat sync status: {sync_status} (attempt {attempt})"
                )));
            }

            let v: Value = serde_json::from_slice(&sync_body).map_err(|e| {
                WasiArenaError::Other(format!("invalid JSON from arena chat sync: {e}"))
            })?;

            let messages = v["messages"].as_array().ok_or_else(|| {
                WasiArenaError::Other("arena chat sync missing 'messages' array".to_string())
            })?;

            // Bootstrap self_user_id by matching our just-sent content (no-op after first success).
            self.maybe_set_self_user_id_from_content(messages, message)?;

            let self_id_opt = self.self_user_id_owned()?;

            last_index = None;
            peer_line = None;
            for m in messages {
                let idx = m["index"]
                    .as_u64()
                    .map(|u| u as usize)
                    .or_else(|| {
                        m["index"]
                            .as_i64()
                            .map(|i| i as usize)
                    })
                    .ok_or_else(|| {
                        WasiArenaError::Other("arena chat message missing numeric index".to_string())
                    })?;
                last_index = Some(idx);
                if let Some(ref sid) = self_id_opt {
                    if m["from"].as_str() != Some(sid.as_str()) {
                        peer_line = m["content"].as_str().map(std::string::ToString::to_string);
                    }
                }
            }

            if peer_line.is_some() {
                break;
            }

            tracing::debug!(
                "chat sync attempt {attempt}/{polls}: no peer message yet (self_id_known={sid})",
                polls = MAX_CHAT_SYNC_POLLS,
                sid = self_id_opt.is_some(),
            );
        }

        let reply = peer_line.ok_or_else(|| {
            WasiArenaError::Other(format!(
                "no peer message after {MAX_CHAT_SYNC_POLLS} chat/sync polls"
            ))
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

    /// Post one chat line with bearer auth. Does NOT poll `chat/sync` afterward.
    /// `next_chat_index` is intentionally not advanced here — the message we just sent
    /// will appear on a subsequent `send_async` or `receive_async`'s sync if needed.
    pub async fn send_only_async(&self, message: &str) -> Result<(), WasiArenaError> {
        self.ensure_psi_session().await?;
        let (challenge_id, session_key) = self.session_snapshot()?;

        let send_url = format!("{}/api/v1/chat/send", self.base_url);
        let send_body = serde_json::to_vec(&json!({
            "channel": challenge_id,
            "content": message,
        }))
        .map_err(|e| WasiArenaError::Other(e.to_string()))?;

        let send_req = ProductionArena::build_authed_request(
            Some(session_key.as_str()),
            http::Method::POST,
            &send_url,
            Bytes::from(send_body),
            true,
        )?;

        let send_resp = omnia_wasi_http::handle(send_req)
            .await
            .map_err(|e| WasiArenaError::Other(format!("arena chat send failed: {e}")))?;
        if !send_resp.status().is_success() {
            let status = send_resp.status();
            let body_text = std::string::String::from_utf8_lossy(&send_resp.into_body()).into_owned();
            return Err(WasiArenaError::Other(format!(
                "arena chat send status {status}: {body_text}"
            )));
        }
        Ok(())
    }

    pub async fn receive_async(&self) -> Result<String, WasiArenaError> {
        self.ensure_psi_session().await?;

        let (challenge_id, session_key) = self.session_snapshot()?;

        let start_index = {
            let g = self
                .session
                .lock()
                .map_err(|_| WasiArenaError::Other("arena session mutex poisoned".to_string()))?;
            g.next_chat_index
        };

        let sync_url = format!(
            "{}/api/v1/chat/sync?channel={}&index={}",
            self.base_url.trim_end_matches('/'),
            challenge_id,
            start_index
        );

        let mut last_index: Option<usize> = None;
        let mut peer_line: Option<String> = None;

        for attempt in 1..=MAX_CHAT_SYNC_POLLS {
            let sync_req = ProductionArena::build_authed_request(
                Some(session_key.as_str()),
                http::Method::GET,
                &sync_url,
                Bytes::new(),
                true,
            )?;

            let sync_resp = omnia_wasi_http::handle(sync_req)
                .await
                .map_err(|e| WasiArenaError::Other(format!("arena chat sync failed: {e}")))?;
            let sync_status = sync_resp.status();
            let sync_body = sync_resp.into_body();
            if !sync_status.is_success() {
                return Err(WasiArenaError::Other(format!(
                    "arena chat sync status: {sync_status} (receive attempt {attempt})"
                )));
            }

            let v: Value = serde_json::from_slice(&sync_body).map_err(|e| {
                WasiArenaError::Other(format!("invalid JSON from arena chat sync: {e}"))
            })?;

            let messages = v["messages"].as_array().ok_or_else(|| {
                WasiArenaError::Other("arena chat sync missing 'messages' array".to_string())
            })?;

            let self_id_opt = self.self_user_id_owned()?;

            last_index = None;
            peer_line = None;
            for m in messages {
                let idx = m["index"]
                    .as_u64()
                    .map(|u| u as usize)
                    .or_else(|| m["index"].as_i64().map(|i| i as usize))
                    .ok_or_else(|| {
                        WasiArenaError::Other("arena chat message missing numeric index".to_string())
                    })?;
                last_index = Some(idx);
                let from = m["from"].as_str();
                let is_peer = match self_id_opt {
                    Some(ref sid) => from != Some(sid.as_str()),
                    None => true,
                };
                if is_peer {
                    peer_line = m["content"].as_str().map(std::string::ToString::to_string);
                }
            }

            if peer_line.is_some() {
                break;
            }

            tracing::debug!(
                "chat receive attempt {attempt}/{MAX_CHAT_SYNC_POLLS}: no peer message yet (self_id_known={})",
                self_id_opt.is_some()
            );
        }

        let reply = peer_line.ok_or_else(|| {
            WasiArenaError::Other(format!(
                "no peer message after {MAX_CHAT_SYNC_POLLS} chat/sync polls (receive)"
            ))
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

impl ArenaTransport for ProductionArena {
    fn reset_async(&self) -> impl std::future::Future<Output = Result<(), WasiArenaError>> + Send {
        let s = self.clone();
        async move {
            s.reset_async()
                .await
        }
    }

    fn send_async(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<String, WasiArenaError>> + Send {
        let s = self.clone();
        let msg = message.to_string();
        async move {
            s.send_async(&msg)
                .await
        }
    }

    fn send_only_async(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<(), WasiArenaError>> + Send {
        let s = self.clone();
        let m = message.to_string();
        async move { s.send_only_async(&m).await }
    }

    fn receive_async(&self) -> impl std::future::Future<Output = Result<String, WasiArenaError>> + Send {
        let s = self.clone();
        async move {
            s.receive_async().await
        }
    }

    fn operator_sync_async(
        &self,
        start_index: usize,
    ) -> impl std::future::Future<Output = Result<Vec<String>, WasiArenaError>> + Send {
        let s = self.clone();
        async move {
            s.operator_sync_async(start_index).await
        }
    }

    fn submit_message_async(
        &self,
        message_type: &str,
        content: &str,
    ) -> impl std::future::Future<Output = Result<(), WasiArenaError>> + Send {
        let s = self.clone();
        let mt = message_type.to_string();
        let c = content.to_string();
        async move {
            s.submit_message_async(&mt, &c).await
        }
    }
}
