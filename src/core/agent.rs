use super::environment::Environment;

/// An autonomous agent identified by name and carrying a message.
pub struct Agent<E: Environment> {
    /// Display name for this agent.
    pub name: String,
    /// Message associated with this agent.
    pub message: String,
    /// Bridges output to the host environment.
    pub environment: E,
}

impl<E: Environment> Agent<E> {
    /// Prints this agent’s [`message`](Agent::message) through its [`environment`](Agent::environment).
    pub fn print(&self) {
        self.environment
            .print(&self.message);
    }
}

#[cfg(test)]
mod in_memory_environment {
    use std::cell::RefCell;

    use crate::core::environment::Environment;

    /// Records [`Environment::print`](Environment::print) calls in memory (e.g. for tests).
    pub struct InMemoryEnvironment {
        lines: RefCell<Vec<String>>,
    }

    impl Default for InMemoryEnvironment {
        fn default() -> Self {
            Self {
                lines: RefCell::new(Vec::new()),
            }
        }
    }

    impl InMemoryEnvironment {
        pub fn new() -> Self {
            Self::default()
        }

        /// Returns a copy of every string passed to [`Environment::print`](Environment::print).
        pub fn lines(&self) -> Vec<String> {
            self.lines
                .borrow()
                .clone()
        }
    }

    impl Environment for InMemoryEnvironment {
        fn print(&self, s: &str) {
            self.lines
                .borrow_mut()
                .push(s.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{in_memory_environment::InMemoryEnvironment, Agent};

    #[test]
    fn agent_print_delegates_message_to_environment() {
        let mem = InMemoryEnvironment::new();
        let agent = Agent {
            name: String::from("a"),
            message: String::from("hello"),
            environment: mem,
        };
        agent.print();
        assert_eq!(
            agent
                .environment
                .lines(),
            vec![String::from("hello")]
        );
    }
}
