use super::{
    environment::{Environment, LogMessageLevel},
    llm::Llm,
};

/// An autonomous agent identified by name.
pub struct Agent<E: Environment, L: Llm> {
    /// Display name for this agent.
    pub name: String,
    /// Bridges output to the host environment.
    pub environment: E,
    /// Language model used for message exchange.
    pub llm: L,
}

impl<E: Environment, L: Llm> Agent<E, L> {
    /// Expresses `text` through [`environment`](Agent::environment) (e.g. stdout in the shell adapter).
    pub fn print(&self, text: &str) {
        self.environment
            .print(text);
    }

    /// Logs `message` at `level` through its [`environment`](Agent::environment).
    pub fn log(&self, message: &str, level: LogMessageLevel) {
        self.environment
            .log(message, level);
    }

    /// Sends `message` to [`llm`](Agent::llm) and prints the reply via [`print`](Agent::print).
    pub fn receive_message(&self, message: &str) {
        self.log(&format!("{} <- {}", self.name, message), LogMessageLevel::Standard);
        let reply = self
            .llm
            .receive_message(message);
        self.print(&format!("{} -> {}", self.name, reply));
    }
}

#[cfg(test)]
mod in_memory_environment {
    use std::cell::RefCell;

    use crate::core::environment::{Environment, LoggingLevel};

    /// Records [`Environment::print`](Environment::print) in memory (e.g. for tests). [`Environment::log`](Environment::log) is accepted but not stored.
    pub struct InMemoryEnvironment {
        lines: RefCell<Vec<String>>,
        pub logging_level: LoggingLevel,
    }

    impl InMemoryEnvironment {
        pub fn new(logging_level: LoggingLevel) -> Self {
            Self {
                lines: RefCell::new(Vec::new()),
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

        fn emit_log(&self, _message: &str) {}
    }
}

#[cfg(test)]
mod tests {
    use super::{in_memory_environment::InMemoryEnvironment, Agent};
    use crate::core::environment::LoggingLevel;

    /// Reply returned by [`StubLlm`]; only used to assert the agent prints whatever the port returns.
    const STUB_LLM_REPLY: &str = "stub-llm-reply";

    struct StubLlm;

    impl crate::core::llm::Llm for StubLlm {
        fn receive_message(&self, _message: &str) -> String {
            STUB_LLM_REPLY.to_string()
        }
    }

    #[test]
    fn agent_print_delegates_text_to_environment() {
        let mem = InMemoryEnvironment::new(LoggingLevel::Standard);
        let agent = Agent {
            name: String::from("a"),
            environment: mem,
            llm: StubLlm,
        };
        agent.print("hello");
        assert_eq!(
            agent
                .environment
                .lines(),
            vec![String::from("hello")]
        );
    }

    #[test]
    fn agent_receive_message_prints_llm_reply_via_environment() {
        let mem = InMemoryEnvironment::new(LoggingLevel::Standard);
        let agent = Agent {
            name: String::from("a"),
            environment: mem,
            llm: StubLlm,
        };
        agent.receive_message("ping");
        assert_eq!(
            agent
                .environment
                .lines(),
            vec![format!("a -> {}", STUB_LLM_REPLY)]
        );
    }
}
