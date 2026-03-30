use crate::core::{agent::Agent, environment::Environment};

/// Creates an [`Agent`] with the given `name`, `message`, and `environment`.
pub fn create_agent<E: Environment>(
    name: impl Into<String>,
    message: impl Into<String>,
    environment: E,
) -> Agent<E> {
    Agent {
        name: name.into(),
        message: message.into(),
        environment,
    }
}

#[cfg(test)]
mod tests {
    use super::create_agent;
    use crate::core::environment::Environment;

    struct NoopEnvironment;

    impl Environment for NoopEnvironment {
        fn print(&self, _s: &str) {}
    }

    #[test]
    fn create_agent_sets_name_message_and_environment() {
        let agent = create_agent("test", "ping", NoopEnvironment);
        assert_eq!(agent.name, "test");
        assert_eq!(agent.message, "ping");
    }
}
