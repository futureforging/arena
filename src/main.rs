mod application;
mod core;
mod infrastructure;

pub use core::{
    agent::Agent,
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
    llm::Llm,
};

use anthropic_api_key_from_local_file::anthropic_api_key_from_local_file;
pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::{
    environment::ShellEnvironment,
    llm::{ClaudeLlm, DummyLlm},
};

/// System prompt sent with every Anthropic Messages request for this binary.
const SYSTEM_PROMPT: &str = "You are a concise, helpful assistant.";

fn main() {
    let api_key = match anthropic_api_key_from_local_file(None) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Failed to load Anthropic API key: {e}");
            std::process::exit(1);
        },
    };
    let llm = ClaudeLlm::new(api_key, Some(SYSTEM_PROMPT.to_string()));
    let agent = create_agent(
        "Aria",
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
        llm,
    );
    agent.receive_message("What is the capital of France?");
}
