/// One turn in a chat transcript (role name + text), used by [`Llm::complete`](Llm::complete).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessage {
    /// Role label (e.g. [`USER_ROLE`](crate::core::session::USER_ROLE)).
    pub role: String,
    pub content: String,
}

/// Language-model port: stateless completion given merged system text and message history.
pub trait Llm {
    /// Optional model- or adapter-level system instructions (merged per request with session system text).
    fn base_system_prompt(&self) -> Option<&str> {
        None
    }

    /// Returns the model’s reply for the given `messages` transcript using optional merged `system`.
    fn complete(&self, system: Option<&str>, messages: &[ChatMessage]) -> String;
}
