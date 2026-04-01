use super::llm::ChatMessage;

/// Canonical transcript role for the human peer.
pub const USER_ROLE: &str = "user";
/// Canonical transcript role for the model side of the conversation.
pub const ASSISTANT_ROLE: &str = "assistant";

/// Session-level instructions and conversation transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    /// Instructions for this conversation (merged with [`Llm::base_system_prompt`](super::llm::Llm::base_system_prompt) on each turn).
    pub system_prompt: String,
    pub transcript: Vec<ChatMessage>,
}

impl Session {
    /// Creates a session with `system_prompt` and an empty transcript.
    pub fn new(system_prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            transcript: Vec::new(),
        }
    }
}

/// Active conversation state held by [`Agent`](crate::core::agent::Agent) between [`start_session`](crate::core::agent::Agent::start_session) and [`stop_session`](crate::core::agent::Agent::stop_session).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSession {
    pub session: Session,
    pub agent_role: String,
    pub peer_role: String,
}

/// Returned when [`Agent::start_session`](crate::core::agent::Agent::start_session) is called while a session is already active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartSessionError {
    AlreadyActive,
}

/// Returned when [`Agent::receive_message`](crate::core::agent::Agent::receive_message) is called with no active session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiveMessageError {
    NoActiveSession,
}

/// Combines optional base system text from the LLM adapter with the session’s system prompt for the provider `system` field.
pub fn merge_system_prompts(base: Option<&str>, session_system: &str) -> Option<String> {
    match base {
        None => {
            if session_system.is_empty() {
                None
            } else {
                Some(session_system.to_string())
            }
        },
        Some("") => {
            if session_system.is_empty() {
                None
            } else {
                Some(session_system.to_string())
            }
        },
        Some(b) => {
            if session_system.is_empty() {
                Some(b.to_string())
            } else {
                Some(format!("{b}\n\n{session_system}"))
            }
        },
    }
}
