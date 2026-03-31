mod application;
mod core;
mod infrastructure;

pub use core::{
    agent::Agent,
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
    llm::Llm,
};

pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::{environment::ShellEnvironment, llm::DummyLlm};

fn main() {
    let agent = create_agent(
        "Aria",
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
        DummyLlm,
    );
    agent.receive_message("Example user message");
}
