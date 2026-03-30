/// Routes agent output to the host environment (e.g. shell).
pub trait EnvironmentAdapter {
    /// Prints `s` to the environment in the appropriate way.
    fn print(&self, s: &str);
}

/// Sends output to the process shell via `println!`.
#[derive(Clone, Copy, Debug)]
pub struct ShellAdapter;

impl EnvironmentAdapter for ShellAdapter {
    fn print(&self, s: &str) {
        println!("{s}");
    }
}

/// An autonomous agent identified by name and carrying a message.
pub struct Agent<E: EnvironmentAdapter> {
    /// Display name for this agent.
    pub name: String,
    /// Message associated with this agent.
    pub message: String,
    /// Bridges output to the host environment (e.g. [`ShellAdapter`]).
    pub adapter: E,
}

impl<E: EnvironmentAdapter> Agent<E> {
    /// Prints this agent’s [`message`](Agent::message) through its [`adapter`](Agent::adapter).
    pub fn print(&self) {
        self.adapter
            .print(&self.message);
    }
}

/// Creates an [`Agent`] with the given `name`, `message`, and environment `adapter`.
pub fn create_agent<E: EnvironmentAdapter>(
    name: impl Into<String>,
    message: impl Into<String>,
    adapter: E,
) -> Agent<E> {
    Agent {
        name: name.into(),
        message: message.into(),
        adapter,
    }
}

fn main() {
    let agent = create_agent("Aria", "Hello, world!", ShellAdapter);
    agent.print();
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{create_agent, EnvironmentAdapter, ShellAdapter};

    struct InMemoryAdapter {
        lines: RefCell<Vec<String>>,
    }

    impl InMemoryAdapter {
        fn new() -> Self {
            Self {
                lines: RefCell::new(Vec::new()),
            }
        }

        fn lines(&self) -> Vec<String> {
            self.lines
                .borrow()
                .clone()
        }
    }

    impl EnvironmentAdapter for InMemoryAdapter {
        fn print(&self, s: &str) {
            self.lines
                .borrow_mut()
                .push(s.to_string());
        }
    }

    #[test]
    fn create_agent_sets_name_message_and_adapter() {
        let agent = create_agent("test", "ping", ShellAdapter);
        assert_eq!(agent.name, "test");
        assert_eq!(agent.message, "ping");
    }

    #[test]
    fn agent_print_delegates_message_to_adapter() {
        let cap = InMemoryAdapter::new();
        let agent = create_agent("a", "hello", cap);
        agent.print();
        assert_eq!(
            agent
                .adapter
                .lines(),
            vec![String::from("hello")]
        );
    }
}
