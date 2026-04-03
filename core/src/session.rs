use crate::llm::ChatMessage;

/// Transcript role string used for the **peer**’s lines in many provider APIs (e.g. Anthropic Messages `user`).
pub const USER_ROLE: &str = "user";
/// Transcript role string used for this **Agent**’s lines in many provider APIs (e.g. Anthropic Messages `assistant`).
pub const ASSISTANT_ROLE: &str = "assistant";

/// Session-level instructions and conversation transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    /// Instructions for this conversation (merged with [`Llm::base_system_prompt`](crate::llm::Llm::base_system_prompt) on each turn).
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

/// Active conversation state held by [`Agent`](crate::agent::Agent) between [`start_session`](crate::agent::Agent::start_session) and [`stop_session`](crate::agent::Agent::stop_session).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSession {
    pub session: Session,
    pub agent_role: String,
    pub peer_role: String,
}

/// Returned when [`Agent::start_session`](crate::agent::Agent::start_session) is called while a session is already active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartSessionError {
    AlreadyActive,
}

/// Returned when [`Agent::receive_message`](crate::agent::Agent::receive_message) is called with no active session.
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

#[cfg(test)]
mod tests {
    use super::merge_system_prompts;

    type MergeSystemPromptCase<'a> = (Option<&'a str>, &'a str, Option<&'a str>);

    #[test]
    fn merge_system_prompts_covers_base_and_session_combinations() {
        let cases: &[MergeSystemPromptCase<'_>] = &[
            (None, "", None),
            (None, "session-only", Some("session-only")),
            (Some(""), "", None),
            (Some(""), "session-only", Some("session-only")),
            (Some("base"), "", Some("base")),
            (Some("base"), "session", Some("base\n\nsession")),
        ];
        for &(base, session_system, want) in cases {
            assert_eq!(
                merge_system_prompts(base, session_system),
                want.map(String::from),
                "merge_system_prompts({base:?}, {session_system:?})",
            );
        }
    }
}
