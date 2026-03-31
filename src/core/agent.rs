use super::environment::{Environment, LogMessageLevel};

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

    /// Logs `message` at `level` through its [`environment`](Agent::environment).
    pub fn log(&self, message: &str, level: LogMessageLevel) {
        self.environment
            .log(message, level);
    }
}

#[cfg(test)]
mod in_memory_environment {
    use std::cell::RefCell;

    use crate::core::environment::{Environment, LoggingLevel};

    /// Records [`Environment::print`](Environment::print) and [`Environment::log`](Environment::log) in memory (e.g. for tests).
    pub struct InMemoryEnvironment {
        lines: RefCell<Vec<String>>,
        log_lines: RefCell<Vec<String>>,
        pub logging_level: LoggingLevel,
    }

    impl InMemoryEnvironment {
        pub fn new(logging_level: LoggingLevel) -> Self {
            Self {
                lines: RefCell::new(Vec::new()),
                log_lines: RefCell::new(Vec::new()),
                logging_level,
            }
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

        fn logging_level(&self) -> LoggingLevel {
            self.logging_level
        }

        fn emit_log(&self, message: &str) {
            self.log_lines
                .borrow_mut()
                .push(message.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{in_memory_environment::InMemoryEnvironment, Agent};
    use crate::core::environment::LoggingLevel;

    #[test]
    fn agent_print_delegates_message_to_environment() {
        let mem = InMemoryEnvironment::new(LoggingLevel::Standard);
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
