/// One turn in a chat transcript (role name + text), used by [`Llm::complete`](Llm::complete).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessage {
    /// Transcript role for this line (peer vs. this agent; often [`USER_ROLE`] or [`ASSISTANT_ROLE`] when talking to provider APIs).
    pub role: String,
    pub content: String,
}

/// Result of [`Llm::complete`](Llm::complete): text for the hosting agent’s next turn, plus an optional request body for logging.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlmCompletion {
    /// Text to record under [`ActiveSession::agent_role`](crate::core::session::ActiveSession) and show via the environment.
    pub reply: String,
    /// Pretty-printed JSON request body when the adapter exposes it (e.g. Anthropic Messages); omit secrets such as API keys (they live in headers).
    pub request_body_json: Option<String>,
}

/// Language-model port: stateless completion given merged system text and message history.
pub trait Llm {
    /// Optional model- or adapter-level system instructions (merged per request with session system text).
    fn base_system_prompt(&self) -> Option<&str> {
        None
    }

    /// Returns the hosting agent’s reply and optional request snapshot for the given `messages` using optional merged `system`.
    fn complete(&self, system: Option<&str>, messages: &[ChatMessage]) -> LlmCompletion;
}
