use crate::core::{agent::Agent, environment::Environment, llm::Llm};

/// Creates an [`Agent`] with the given `name`, `environment`, and `llm`.
pub fn create_agent<E: Environment, L: Llm>(
    name: impl Into<String>,
    environment: E,
    llm: L,
) -> Agent<E, L> {
    Agent {
        name: name.into(),
        environment,
        llm,
    }
}

#[cfg(test)]
mod tests {
    use super::create_agent;
    use crate::core::{
        environment::{Environment, LoggingLevel},
        llm::Llm,
    };

    struct StubLlm;

    impl Llm for StubLlm {
        fn receive_message(&self, _message: &str) -> String {
            String::new()
        }
    }

    struct NoopEnvironment;

    impl Environment for NoopEnvironment {
        fn print(&self, _s: &str) {}

        fn logging_level(&self) -> LoggingLevel {
            LoggingLevel::None
        }

        fn emit_log(&self, _message: &str) {}
    }

    #[test]
    fn create_agent_sets_name_and_environment() {
        let agent = create_agent("test", NoopEnvironment, StubLlm);
        assert_eq!(agent.name, "test");
    }
}
