mod application;
mod core;
mod infrastructure;

pub use core::{
    agent::Agent,
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
};

pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::environment::ShellEnvironment;

fn main() {
    let agent = create_agent(
        "Aria",
        "Hello, world!",
        ShellEnvironment {
            logging_level: LoggingLevel::Standard,
        },
    );
    agent.log("This is a standard log message", LogMessageLevel::Standard);
    agent.log("This is a verbose log message", LogMessageLevel::Verbose);
    agent.print();
}
