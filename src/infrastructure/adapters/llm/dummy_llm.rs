use crate::core::llm::Llm;

const DUMMY_LLM_RESPONSE: &str = "Message received.";

/// Stub LLM adapter that always replies with [`DUMMY_LLM_RESPONSE`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DummyLlm;

impl Llm for DummyLlm {
    fn receive_message(&self, _message: &str) -> String {
        DUMMY_LLM_RESPONSE.to_string()
    }
}
