//! Peer chat send/receive against `/api/v1/chat/{send,sync}`. The polling loop and the
//! per-attempt parsing are extracted into helpers so the public methods stay flat.

use bytes::Bytes;
use serde_json::{json, Value};
use verity_core::arena::ArenaError;

use super::{session::SessionSnapshot, MAX_CHAT_SYNC_POLLS};
use crate::transport::HttpTransport;

/// Outcome of one or more `chat/sync` polls: the highest index seen, plus the peer message body
/// when one was found.
struct ChatSyncResult {
    last_index: Option<usize>,
    peer_line: Option<String>,
}

impl<H: HttpTransport> super::DemoArena<H> {
    pub(super) async fn send_inner(&self, message: &str) -> Result<String, ArenaError> {
        self.ensure_psi_session()
            .await?;
        let snap = self.session_snapshot()?;
        let start_index = self.next_chat_index_owned()?;

        self.post_chat_message(&snap, message)
            .await?;

        let sync_url = chat_sync_url(&self.base_url, &snap.challenge_id, start_index);
        let result = self
            .poll_for_peer_message(&sync_url, &snap.session_key, Some(message), "chat sync")
            .await?;
        let reply = result
            .peer_line
            .ok_or_else(|| {
                ArenaError::Other(format!(
                    "no peer message after {MAX_CHAT_SYNC_POLLS} chat/sync polls"
                ))
            })?;
        if let Some(last) = result.last_index {
            self.advance_next_chat_index_to(last.saturating_add(1))?;
        }
        Ok(reply)
    }

    /// Post one chat line with bearer auth. Does NOT poll `chat/sync` afterward.
    /// `next_chat_index` is intentionally not advanced here — the message we just sent
    /// will appear on a subsequent `send_inner` or `receive_inner`'s sync if needed.
    pub(super) async fn send_only_inner(&self, message: &str) -> Result<(), ArenaError> {
        self.ensure_psi_session()
            .await?;
        let snap = self.session_snapshot()?;
        self.post_chat_message(&snap, message)
            .await
    }

    pub(super) async fn receive_inner(&self) -> Result<String, ArenaError> {
        self.ensure_psi_session()
            .await?;
        let snap = self.session_snapshot()?;
        let start_index = self.next_chat_index_owned()?;

        let sync_url = chat_sync_url(&self.base_url, &snap.challenge_id, start_index);
        let result = self
            .poll_for_peer_message(&sync_url, &snap.session_key, None, "chat receive")
            .await?;
        let reply = result
            .peer_line
            .ok_or_else(|| {
                ArenaError::Other(format!(
                    "no peer message after {MAX_CHAT_SYNC_POLLS} chat/sync polls (receive)"
                ))
            })?;
        if let Some(last) = result.last_index {
            self.advance_next_chat_index_to(last.saturating_add(1))?;
        }
        Ok(reply)
    }

    /// `POST /api/v1/chat/send` with bearer auth.
    async fn post_chat_message(
        &self,
        snap: &SessionSnapshot,
        message: &str,
    ) -> Result<(), ArenaError> {
        let send_url = format!("{}/api/v1/chat/send", self.base_url);
        let send_body = serde_json::to_vec(&json!({
            "channel": snap.challenge_id,
            "content": message,
        }))
        .map_err(|e| ArenaError::Other(e.to_string()))?;

        let send_req = Self::build_authed_request(
            Some(
                snap.session_key
                    .as_str(),
            ),
            http::Method::POST,
            &send_url,
            Bytes::from(send_body),
            true,
        )?;

        let send_resp = self
            .http
            .exchange(send_req)
            .await
            .map_err(|e| ArenaError::Other(format!("arena chat send failed: {e}")))?;
        if send_resp
            .status()
            .is_success()
        {
            return Ok(());
        }
        let status = send_resp.status();
        let body_text = std::string::String::from_utf8_lossy(&send_resp.into_body()).into_owned();
        Err(ArenaError::Other(format!("arena chat send status {status}: {body_text}")))
    }

    /// Repeatedly polls `chat/sync` until a peer message appears or [`MAX_CHAT_SYNC_POLLS`] is
    /// exhausted. `bootstrap_via_sent`: when `Some`, the loop attempts to identify our own
    /// `from` id from a server-echoed copy of the just-sent message (send context); when `None`,
    /// any incoming message is treated as the peer's (receive context).
    async fn poll_for_peer_message(
        &self,
        sync_url: &str,
        session_key: &str,
        bootstrap_via_sent: Option<&str>,
        log_prefix: &str,
    ) -> Result<ChatSyncResult, ArenaError> {
        let mut last_index: Option<usize> = None;
        for attempt in 1..=MAX_CHAT_SYNC_POLLS {
            let result = self
                .one_chat_sync_attempt(sync_url, session_key, bootstrap_via_sent)
                .await?;
            last_index = result.last_index;
            if result
                .peer_line
                .is_some()
            {
                return Ok(result);
            }
            let self_id_known = self
                .self_user_id_owned()?
                .is_some();
            tracing::debug!(
                "{log_prefix} attempt {attempt}/{MAX_CHAT_SYNC_POLLS}: no peer message yet (self_id_known={self_id_known})"
            );
        }
        Ok(ChatSyncResult {
            last_index,
            peer_line: None,
        })
    }

    /// Single `GET /api/v1/chat/sync` round-trip: fetches messages and returns the peer line if
    /// any (per `bootstrap_via_sent` policy) plus the highest message index seen.
    async fn one_chat_sync_attempt(
        &self,
        sync_url: &str,
        session_key: &str,
        bootstrap_via_sent: Option<&str>,
    ) -> Result<ChatSyncResult, ArenaError> {
        let req = Self::build_authed_request(
            Some(session_key),
            http::Method::GET,
            sync_url,
            Bytes::new(),
            true,
        )?;
        let resp = self
            .http
            .exchange(req)
            .await
            .map_err(|e| ArenaError::Other(format!("arena chat sync failed: {e}")))?;
        let status = resp.status();
        let body = resp.into_body();
        if !status.is_success() {
            return Err(ArenaError::Other(format!("arena chat sync status: {status}")));
        }
        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| ArenaError::Other(format!("invalid JSON from arena chat sync: {e}")))?;
        let messages = v["messages"]
            .as_array()
            .ok_or_else(|| {
                ArenaError::Other("arena chat sync missing 'messages' array".to_string())
            })?;

        if let Some(sent) = bootstrap_via_sent {
            self.maybe_set_self_user_id_from_content(messages, sent)?;
        }
        let self_id_opt = self.self_user_id_owned()?;

        let mut last_index: Option<usize> = None;
        let mut peer_line: Option<String> = None;
        for m in messages {
            let idx = parse_message_index(m)?;
            last_index = Some(idx);
            if is_peer_message(m, self_id_opt.as_deref(), bootstrap_via_sent.is_some()) {
                peer_line = m["content"]
                    .as_str()
                    .map(std::string::ToString::to_string);
            }
        }
        Ok(ChatSyncResult {
            last_index,
            peer_line,
        })
    }
}

fn chat_sync_url(base_url: &str, challenge_id: &str, start_index: usize) -> String {
    format!(
        "{}/api/v1/chat/sync?channel={challenge_id}&index={start_index}",
        base_url.trim_end_matches('/')
    )
}

fn parse_message_index(m: &Value) -> Result<usize, ArenaError> {
    m["index"]
        .as_u64()
        .map(|u| u as usize)
        .or_else(|| {
            m["index"]
                .as_i64()
                .map(|i| i as usize)
        })
        .ok_or_else(|| ArenaError::Other("arena chat message missing numeric index".to_string()))
}

/// Whether the given message is from the peer (not us).
///
/// - When our own id is known, peer = `from != self_id`.
/// - When our id is unknown:
///   - In *send* context (`in_send_context = true`), we have not yet bootstrapped self-id from a
///     server-echoed copy of our message, so we can't safely classify anything as the peer yet.
///   - In *receive* context (`in_send_context = false`), we treat any incoming message as the
///     peer's — there's no prior send to bootstrap from.
fn is_peer_message(m: &Value, self_id: Option<&str>, in_send_context: bool) -> bool {
    match self_id {
        Some(sid) => m["from"].as_str() != Some(sid),
        None => !in_send_context,
    }
}
