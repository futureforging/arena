mod application;
mod core;
mod infrastructure;

pub use core::{agent::Agent, environment::Environment};

pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::environment::ShellEnvironment;

fn main() {
    let agent = create_agent("Aria", "Hello, world!", ShellEnvironment);
    agent.print();
}
