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
        active_session: None,
    }
}

#[cfg(test)]
mod tests {
    use super::create_agent;
    use crate::test_support::{EmptyReplyLlm, NoopEnvironment};

    #[test]
    fn create_agent_sets_name_and_environment() {
        let agent = create_agent("test", NoopEnvironment, EmptyReplyLlm);
        assert_eq!(agent.name, "test");
    }
}
