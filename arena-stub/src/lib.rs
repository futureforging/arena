//! Local HTTP arena stub: canned PSI challenge lifecycle matching the real Arena API shapes.

pub mod psi_peer;

/// TCP port the arena-stub HTTP server binds to on **`127.0.0.1`**.
pub const ARENA_STUB_LISTEN_PORT: u16 = 3000;

/// Deterministic challenge id for the stub (single active challenge).
pub const STUB_CHALLENGE_ID: &str = "challenge_stub_001";

/// Invite code for the agent player in standalone stub mode.
pub const STUB_INVITE_AGENT: &str = "inv_stub_agent";

/// Invite code for the scripted peer in standalone stub mode.
pub const STUB_INVITE_PEER: &str = "inv_stub_peer";

// --- Operator seed text (mirrors `verity_core::games::psi` prompts; stub is source for operator sync) ---

const STUB_AGENT_OPERATOR_BODY: &str = r#"You are playing a Private Set Intersection (PSI) game. You have a private set of letters. Your goal is to find which letters you and your peer have in common WITHOUT revealing letters that are not in the intersection.

Turn order:
1) The peer opens with a brief greeting (e.g. "Hello."). Ask if they would like to play Private Set Intersection (one short sentence ending with a question mark).
2) When the peer says "yes", propose the strategy: "I propose we use SHA-256 Hash Intersection. I will send you SHA-256 hashes of each item in my set. You hash your own items and compare. Then send me back the plaintext intersection." Wait for acceptance.
3) When the peer accepts (says "agreed" or similar), send your hashed items as a JSON array on a single line, like: ["aef3...", "b2c4...", ...]. Use the exact hashes from your private data below. Do not add any other text on that line.
4) The peer will respond with the plaintext intersection as a JSON array, like: ["a", "c", "f"]. Verify every item in their claimed intersection appears in YOUR private set. If all items are in your set, respond with "Result: correct". If any item is NOT in your set, respond with "Result: incorrect".
5) After reporting the result, say "Goodbye." on its own line.

IMPORTANT: When sending hashes in step 3, send ONLY the JSON array on a single line. No preamble, no explanation. Use the exact hex hashes provided in your private data.

Your private letter set and hashes are assigned by your local runtime (not repeated here)."#;

const STUB_PEER_OPERATOR_BODY: &str = r#"You are the scripted arena peer for Private Set Intersection (PSI). Follow the hash-intersection protocol. Your private letter set (local to the stub) is fixed for this challenge."#;

/// Script step for the scripted PSI peer (`psi_peer_reply` uses this counter).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PeerState {
    pub step: u8,
}

impl PeerState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// In-memory state for the arena stub.
#[derive(Clone, Debug, Default)]
pub struct ArenaState {
    /// The single active challenge (if any).
    pub challenge: Option<StubChallenge>,
}

/// One active PSI challenge in the stub.
#[derive(Clone, Debug)]
pub struct StubChallenge {
    pub id: String,
    pub invites: [String; 2],
    pub joined: [bool; 2],
    pub operator_messages: Vec<OperatorMessage>,
    pub chat_messages: Vec<ChatMessage>,
    pub psi_state: PeerState,
    pub submissions: Vec<Submission>,
}

#[derive(Clone, Debug)]
pub struct OperatorMessage {
    pub from: String,
    pub to: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub from: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct Submission {
    pub from: String,
    pub message_type: String,
    pub content: String,
}

/// `(index, from, content)` for operator and chat sync payloads.
pub type IndexedMessageRow = (usize, String, String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatePsiChallengeResponse {
    pub id: String,
    pub invites: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinResponse {
    pub challenge_id: String,
}

/// Client error for stub HTTP handlers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StubError {
    NoActiveChallenge,
    UnknownInvite,
    UnknownChannel,
}

impl std::fmt::Display for StubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StubError::NoActiveChallenge => write!(f, "no active challenge"),
            StubError::UnknownInvite => write!(f, "unknown invite"),
            StubError::UnknownChannel => write!(f, "unknown channel"),
        }
    }
}

impl std::error::Error for StubError {}

fn invite_index(invites: &[String; 2], invite: &str) -> Option<usize> {
    invites
        .iter()
        .position(|s| s == invite)
}

fn seed_operator_messages() -> Vec<OperatorMessage> {
    let agent_json = format!(
        "{{\"role\":\"operator\",\"challenge\":\"psi\",\"instructions\":{}}}",
        serde_json::to_string(STUB_AGENT_OPERATOR_BODY).unwrap_or_else(|_| "\"\"".to_string())
    );
    let peer_set: String = psi_peer::PSI_PEER_SET
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let peer_json = format!(
        "{{\"role\":\"operator\",\"challenge\":\"psi\",\"instructions\":{},\"private_letter_set\":\"[{peer_set}]\"}}",
        serde_json::to_string(STUB_PEER_OPERATOR_BODY).unwrap_or_else(|_| "\"\"".to_string())
    );

    vec![
        OperatorMessage {
            from: "operator".to_string(),
            to: STUB_INVITE_AGENT.to_string(),
            content: agent_json,
        },
        OperatorMessage {
            from: "operator".to_string(),
            to: STUB_INVITE_PEER.to_string(),
            content: peer_json,
        },
    ]
}

impl ArenaState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.challenge = None;
    }

    /// Creates or replaces the single PSI challenge and seeds operator messages.
    pub fn create_or_replace_psi_challenge(&mut self) -> CreatePsiChallengeResponse {
        self.challenge = Some(StubChallenge {
            id: STUB_CHALLENGE_ID.to_string(),
            invites: [STUB_INVITE_AGENT.to_string(), STUB_INVITE_PEER.to_string()],
            joined: [false, false],
            operator_messages: seed_operator_messages(),
            chat_messages: Vec::new(),
            psi_state: PeerState::new(),
            submissions: Vec::new(),
        });
        CreatePsiChallengeResponse {
            id: STUB_CHALLENGE_ID.to_string(),
            invites: vec![STUB_INVITE_AGENT.to_string(), STUB_INVITE_PEER.to_string()],
        }
    }

    /// Marks the invite as joined (idempotent).
    pub fn join_with_invite(&mut self, invite: &str) -> Result<JoinResponse, StubError> {
        let c = self
            .challenge
            .as_mut()
            .ok_or(StubError::NoActiveChallenge)?;
        let idx = invite_index(&c.invites, invite).ok_or(StubError::UnknownInvite)?;
        c.joined[idx] = true;
        Ok(JoinResponse {
            challenge_id: c
                .id
                .clone(),
        })
    }

    fn challenge_ref(&self, channel: &str) -> Result<&StubChallenge, StubError> {
        let c = self
            .challenge
            .as_ref()
            .ok_or(StubError::NoActiveChallenge)?;
        if c.id != channel {
            return Err(StubError::UnknownChannel);
        }
        Ok(c)
    }

    fn challenge_mut(&mut self, channel: &str) -> Result<&mut StubChallenge, StubError> {
        let c = self
            .challenge
            .as_mut()
            .ok_or(StubError::NoActiveChallenge)?;
        if c.id != channel {
            return Err(StubError::UnknownChannel);
        }
        Ok(c)
    }

    /// Operator messages for `to == recipient`, starting at `start_index` in that recipient's stream.
    pub fn operator_sync(
        &self,
        channel: &str,
        recipient: &str,
        start_index: usize,
    ) -> Result<Vec<IndexedMessageRow>, StubError> {
        let c = self.challenge_ref(channel)?;
        let mut per_recipient_index: usize = 0;
        let mut out = Vec::new();
        for m in &c.operator_messages {
            if m.to != recipient {
                continue;
            }
            if per_recipient_index >= start_index {
                out.push((
                    per_recipient_index,
                    m.from
                        .clone(),
                    m.content
                        .clone(),
                ));
            }
            per_recipient_index = per_recipient_index.saturating_add(1);
        }
        Ok(out)
    }

    /// Appends a chat line. When `from` is the agent invite, appends the scripted peer reply.
    pub fn chat_send(&mut self, channel: &str, from: &str, content: &str) -> Result<(), StubError> {
        let c = self.challenge_mut(channel)?;
        c.chat_messages
            .push(ChatMessage {
                from: from.to_string(),
                content: content.to_string(),
            });

        if from == STUB_INVITE_AGENT {
            let step = c
                .psi_state
                .step;
            c.psi_state
                .step = step.saturating_add(1);
            let peer_line = psi_peer::psi_peer_reply(step, content);
            c.chat_messages
                .push(ChatMessage {
                    from: STUB_INVITE_PEER.to_string(),
                    content: peer_line,
                });
        }

        Ok(())
    }

    /// All chat messages starting at global `start_index`.
    pub fn chat_sync(
        &self,
        channel: &str,
        start_index: usize,
    ) -> Result<Vec<IndexedMessageRow>, StubError> {
        let c = self.challenge_ref(channel)?;
        Ok(c.chat_messages
            .iter()
            .enumerate()
            .skip(start_index)
            .map(|(i, m)| {
                (
                    i,
                    m.from
                        .clone(),
                    m.content
                        .clone(),
                )
            })
            .collect())
    }

    /// Records a submission and appends a short operator message to the submitter's operator stream.
    pub fn arena_message_submit(
        &mut self,
        challenge_id: &str,
        from: &str,
        message_type: &str,
        content: &str,
    ) -> Result<(), StubError> {
        let c = self.challenge_mut(challenge_id)?;
        c.submissions
            .push(Submission {
                from: from.to_string(),
                message_type: message_type.to_string(),
                content: content.to_string(),
            });
        let ack = format!(
            "{{\"kind\":\"operator_ack\",\"messageType\":{},\"note\":\"stub recorded submission\"}}",
            serde_json::to_string(message_type).unwrap_or_else(|_| "\"\"".to_string())
        );
        c.operator_messages
            .push(OperatorMessage {
                from: "operator".to_string(),
                to: from.to_string(),
                content: ack,
            });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use verity_core::games::sha256_hex;

    use super::*;

    #[test]
    fn reset_clears_challenge() {
        let mut s = ArenaState::new();
        s.create_or_replace_psi_challenge();
        s.reset();
        assert!(s
            .challenge
            .is_none());
    }

    #[test]
    fn create_psi_challenge_is_deterministic() {
        let mut s = ArenaState::new();
        let r = s.create_or_replace_psi_challenge();
        assert_eq!(r.id, STUB_CHALLENGE_ID);
        assert_eq!(r.invites, vec![STUB_INVITE_AGENT.to_string(), STUB_INVITE_PEER.to_string()]);
    }

    #[test]
    fn join_marks_invite() {
        let mut s = ArenaState::new();
        s.create_or_replace_psi_challenge();
        let j = s
            .join_with_invite(STUB_INVITE_AGENT)
            .unwrap();
        assert_eq!(j.challenge_id, STUB_CHALLENGE_ID);
        assert!(
            s.challenge
                .as_ref()
                .unwrap()
                .joined[0]
        );
    }

    #[test]
    fn operator_sync_filters_recipient_and_index() {
        let mut s = ArenaState::new();
        s.create_or_replace_psi_challenge();
        let rows = s
            .operator_sync(STUB_CHALLENGE_ID, STUB_INVITE_AGENT, 0)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, 0);
        assert_eq!(rows[0].1, "operator");
        assert!(rows[0]
            .2
            .contains("Private Set Intersection"));
    }

    #[test]
    fn chat_send_from_agent_triggers_scripted_peer() {
        let mut s = ArenaState::new();
        s.create_or_replace_psi_challenge();
        s.chat_send(STUB_CHALLENGE_ID, STUB_INVITE_AGENT, "hello")
            .unwrap();
        let rows = s
            .chat_sync(STUB_CHALLENGE_ID, 0)
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, STUB_INVITE_AGENT);
        assert_eq!(rows[1].1, STUB_INVITE_PEER);
        assert_eq!(rows[1].2, "yes");
    }

    #[test]
    fn chat_sync_skips_prefix_indices() {
        let mut s = ArenaState::new();
        s.create_or_replace_psi_challenge();
        s.chat_send(STUB_CHALLENGE_ID, STUB_INVITE_AGENT, "a")
            .unwrap();
        let rows = s
            .chat_sync(STUB_CHALLENGE_ID, 1)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, STUB_INVITE_PEER);
        assert_eq!(rows[0].2, "yes");
    }

    #[test]
    fn full_psi_scripted_exchange_via_chat_send() {
        let mut s = ArenaState::new();
        s.create_or_replace_psi_challenge();
        s.chat_send(
            STUB_CHALLENGE_ID,
            STUB_INVITE_AGENT,
            "Would you like to play Private Set Intersection?",
        )
        .unwrap();
        s.chat_send(
            STUB_CHALLENGE_ID,
            STUB_INVITE_AGENT,
            "I propose we use SHA-256 Hash Intersection.",
        )
        .unwrap();
        let h_a = sha256_hex("a");
        let h_m = sha256_hex("m");
        let hashes = format!(r#"["{h_a}","{h_m}"]"#);
        s.chat_send(STUB_CHALLENGE_ID, STUB_INVITE_AGENT, &hashes)
            .unwrap();
        let rows = s
            .chat_sync(STUB_CHALLENGE_ID, 0)
            .unwrap();
        let last_peer = rows
            .iter()
            .rev()
            .find(|r| r.1 == STUB_INVITE_PEER)
            .map(|r| {
                r.2.as_str()
            })
            .unwrap();
        let parsed: Vec<String> = serde_json::from_str(last_peer).expect("JSON");
        assert!(parsed.contains(&"a".to_string()));
        assert!(parsed.contains(&"m".to_string()));
    }

    #[test]
    fn submission_appends_operator_message() {
        let mut s = ArenaState::new();
        s.create_or_replace_psi_challenge();
        s.arena_message_submit(STUB_CHALLENGE_ID, STUB_INVITE_AGENT, "guess", "x")
            .unwrap();
        let rows = s
            .operator_sync(STUB_CHALLENGE_ID, STUB_INVITE_AGENT, 1)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0]
            .2
            .contains("stub recorded"));
    }
}
