/// Language-model port: turns an incoming message into a response string.
pub trait Llm {
    /// Returns the model’s reply for `message`.
    fn receive_message(&self, message: &str) -> String;
}
