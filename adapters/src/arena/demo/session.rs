//! Session lifecycle for [`super::DemoArena`]: signed-join bootstrap, snapshot of in-flight
//! identifiers, and per-channel chat-index bookkeeping.

use serde_json::Value;
use verity_core::arena::ArenaError;

use crate::transport::HttpTransport;

/// Mutable per-instance state captured during signed join. Held behind a mutex on `DemoArena`.
#[derive(Debug, Default)]
pub(super) struct SessionState {
    pub(super) initialized: bool,
    pub(super) challenge_id: Option<String>,
    pub(super) session_key: Option<String>,
    pub(super) self_user_id: Option<String>,
    pub(super) next_chat_index: usize,
}

/// Immutable snapshot of joined-session identifiers used by per-call HTTP request building.
pub(super) struct SessionSnapshot {
    pub(super) challenge_id: String,
    pub(super) session_key: String,
}

impl<H: HttpTransport> super::DemoArena<H> {
    pub(super) async fn ensure_psi_session(&self) -> Result<(), ArenaError> {
        let need_init = {
            let g = self
                .session
                .lock()
                .map_err(|_| ArenaError::Other("arena session mutex poisoned".to_string()))?;
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
            .ok_or_else(|| ArenaError::Other("join response missing ChallengeID".to_string()))?
            .to_string();
        let session_key = join_body["sessionKey"]
            .as_str()
            .ok_or_else(|| ArenaError::Other("join response missing sessionKey".to_string()))?
            .to_string();

        self.set_username_best_effort(&session_key)
            .await;

        let mut g = self
            .session
            .lock()
            .map_err(|_| ArenaError::Other("arena session mutex poisoned".to_string()))?;
        g.initialized = true;
        g.challenge_id = Some(challenge_id);
        g.session_key = Some(session_key);
        g.self_user_id = None;
        g.next_chat_index = 0;
        Ok(())
    }

    pub(super) fn session_snapshot(&self) -> Result<SessionSnapshot, ArenaError> {
        let g = self
            .session
            .lock()
            .map_err(|_| ArenaError::Other("arena session mutex poisoned".to_string()))?;
        let challenge_id = g
            .challenge_id
            .clone()
            .ok_or_else(|| {
                ArenaError::Other("arena session not initialized (no challenge_id)".to_string())
            })?;
        let session_key = g
            .session_key
            .clone()
            .ok_or_else(|| {
                ArenaError::Other("arena session not initialized (no session_key)".to_string())
            })?;
        Ok(SessionSnapshot {
            challenge_id,
            session_key,
        })
    }

    /// Identify our own user id by finding the message whose `content` matches what we just sent.
    /// Iterates in reverse so the most recent matching message wins (in case a prior turn
    /// happened to send identical content). No-op once `self_user_id` is set.
    pub(super) fn maybe_set_self_user_id_from_content(
        &self,
        messages: &[Value],
        sent_content: &str,
    ) -> Result<(), ArenaError> {
        let mut g = self
            .session
            .lock()
            .map_err(|_| ArenaError::Other("arena session mutex poisoned".to_string()))?;
        if g.self_user_id
            .is_some()
        {
            return Ok(());
        }
        for m in messages
            .iter()
            .rev()
        {
            if m["content"].as_str() != Some(sent_content) {
                continue;
            }
            let Some(fid) = m["from"].as_str() else {
                continue;
            };
            g.self_user_id = Some(fid.to_string());
            return Ok(());
        }
        // Our message hasn't appeared yet (server hasn't echoed it back). Leave
        // self_user_id unset; the next poll will try again.
        Ok(())
    }

    pub(super) fn self_user_id_owned(&self) -> Result<Option<String>, ArenaError> {
        let g = self
            .session
            .lock()
            .map_err(|_| ArenaError::Other("arena session mutex poisoned".to_string()))?;
        Ok(g.self_user_id
            .clone())
    }

    pub(super) fn next_chat_index_owned(&self) -> Result<usize, ArenaError> {
        let g = self
            .session
            .lock()
            .map_err(|_| ArenaError::Other("arena session mutex poisoned".to_string()))?;
        Ok(g.next_chat_index)
    }

    pub(super) fn advance_next_chat_index_to(&self, idx: usize) -> Result<(), ArenaError> {
        let mut g = self
            .session
            .lock()
            .map_err(|_| ArenaError::Other("arena session mutex poisoned".to_string()))?;
        g.next_chat_index = idx;
        Ok(())
    }
}
