use crate::core::llm::Llm;

const DUMMY_LLM_RESPONSE: &str = "Paris is the capital of France.";

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
    fn receive_message(&self, _message: &str) -> String {
        DUMMY_LLM_RESPONSE.to_string()
    }
}
