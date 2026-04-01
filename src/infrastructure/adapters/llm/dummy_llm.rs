use crate::core::llm::{ChatMessage, Llm, LlmCompletion};

const DUMMY_LLM_RESPONSE: &str = "Hello.";

/// Stub LLM adapter that always replies with [`DUMMY_LLM_RESPONSE`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DummyLlm;

impl DummyLlm {
    /// Returns a stub LLM that always replies with [`DUMMY_LLM_RESPONSE`].
    pub const fn new() -> Self {
        Self
    }
}

impl Default for DummyLlm {
    fn default() -> Self {
        Self::new()
    }
}

impl Llm for DummyLlm {
    fn complete(&self, _system: Option<&str>, _messages: &[ChatMessage]) -> LlmCompletion {
        LlmCompletion {
            reply: DUMMY_LLM_RESPONSE.to_string(),
            request_body_json: None,
        }
    }
}
