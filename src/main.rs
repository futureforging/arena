mod application;
mod core;
mod infrastructure;

pub use core::{
    agent::Agent,
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
    llm::Llm,
};

pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::{
    environment::ShellEnvironment,
    llm::{ClaudeLlm, DummyLlm},
};

fn main() {
    let llm = match ClaudeLlm::load_from_default_key_file() {
        Ok(llm) => llm,
        Err(e) => {
            eprintln!("Failed to load Anthropic API key: {e}");
            std::process::exit(1);
        },
    };
    let agent = create_agent(
        "Aria",
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
        llm,
    );
    agent.receive_message("What is the capital of France?");
}
