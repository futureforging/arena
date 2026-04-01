use super::{
    environment::{Environment, LogMessageLevel},
    llm::{ChatMessage, Llm},
    session::{
        merge_system_prompts, ActiveSession, ReceiveMessageError, Session, StartSessionError,
    },
};

/// An autonomous agent identified by name.
pub struct Agent<E: Environment, L: Llm> {
    /// Display name for this agent.
    pub name: String,
    /// Bridges output to the host environment.
    pub environment: E,
    /// Language model used for message exchange.
    pub llm: L,
    /// Conversation in progress, if any.
    pub active_session: Option<ActiveSession>,
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

    /// Begins a session with the given `session` and role labels for transcript entries.
    ///
    /// Returns [`StartSessionError::AlreadyActive`] if a session is already in progress.
    pub fn start_session(
        &mut self,
        session: Session,
        agent_role: impl Into<String>,
        peer_role: impl Into<String>,
    ) -> Result<(), StartSessionError> {
        if self
            .active_session
            .is_some()
        {
            return Err(StartSessionError::AlreadyActive);
        }
        self.active_session = Some(ActiveSession {
            session,
            agent_role: agent_role.into(),
            peer_role: peer_role.into(),
        });
        Ok(())
    }

    /// Ends the active session and returns its [`Session`] for persistence or inspection.
    ///
    /// Returns [`None`] if there was no active session.
    pub fn stop_session(&mut self) -> Option<Session> {
        self.active_session
            .take()
            .map(|active| active.session)
    }

    /// Records a peer message, completes with [`Llm::complete`](Llm::complete), appends the assistant reply, and prints it.
    ///
    /// Returns [`ReceiveMessageError::NoActiveSession`] if [`start_session`](Self::start_session) was not called or after [`stop_session`](Self::stop_session).
    pub fn receive_message(&mut self, message: &str) -> Result<String, ReceiveMessageError> {
        let mut active = self
            .active_session
            .take()
            .ok_or(ReceiveMessageError::NoActiveSession)?;

        active
            .session
            .transcript
            .push(ChatMessage {
                role: active
                    .peer_role
                    .clone(),
                content: message.to_string(),
            });

        self.log(&format!("{} <- {}", self.name, message), LogMessageLevel::Standard);

        let system = merge_system_prompts(
            self.llm
                .base_system_prompt(),
            &active
                .session
                .system_prompt,
        );
        let reply = self
            .llm
            .complete(
                system.as_deref(),
                &active
                    .session
                    .transcript,
            );

        active
            .session
            .transcript
            .push(ChatMessage {
                role: active
                    .agent_role
                    .clone(),
                content: reply.clone(),
            });

        self.active_session = Some(active);

        self.print(&format!("{} -> {}", self.name, reply));

        Ok(reply)
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
    use crate::core::{
        environment::LoggingLevel,
        llm::ChatMessage,
        session::{ReceiveMessageError, Session, StartSessionError, ASSISTANT_ROLE, USER_ROLE},
    };

    /// Reply returned by [`StubLlm`]; only used to assert the agent prints whatever the port returns.
    const STUB_LLM_REPLY: &str = "stub-llm-reply";

    struct StubLlm;

    impl crate::core::llm::Llm for StubLlm {
        fn complete(&self, _system: Option<&str>, _messages: &[ChatMessage]) -> String {
            STUB_LLM_REPLY.to_string()
        }
    }

    fn agent_with_stub() -> Agent<InMemoryEnvironment, StubLlm> {
        Agent {
            name: String::from("a"),
            environment: InMemoryEnvironment::new(LoggingLevel::Standard),
            llm: StubLlm,
            active_session: None,
        }
    }

    #[test]
    fn agent_print_delegates_text_to_environment() {
        let agent = agent_with_stub();
        agent.print("hello");
        assert_eq!(
            agent
                .environment
                .lines(),
            vec![String::from("hello")]
        );
    }

    #[test]
    fn agent_receive_message_prints_llm_reply_via_environment_after_start_session() {
        let mut agent = agent_with_stub();
        agent
            .start_session(Session::new("task"), ASSISTANT_ROLE, USER_ROLE)
            .unwrap();
        agent
            .receive_message("ping")
            .unwrap();
        assert_eq!(
            agent
                .environment
                .lines(),
            vec![format!("a -> {}", STUB_LLM_REPLY)]
        );
    }

    #[test]
    fn receive_message_without_active_session_returns_no_active_session() {
        let mut agent = agent_with_stub();
        assert_eq!(
            agent
                .receive_message("x")
                .unwrap_err(),
            ReceiveMessageError::NoActiveSession
        );
    }

    #[test]
    fn start_session_twice_returns_already_active() {
        let mut agent = agent_with_stub();
        agent
            .start_session(Session::new("one"), ASSISTANT_ROLE, USER_ROLE)
            .unwrap();
        assert_eq!(
            agent
                .start_session(Session::new("two"), ASSISTANT_ROLE, USER_ROLE)
                .unwrap_err(),
            StartSessionError::AlreadyActive
        );
    }

    #[test]
    fn two_receive_messages_extend_transcript() {
        let mut agent = agent_with_stub();
        agent
            .start_session(Session::new("sys"), ASSISTANT_ROLE, USER_ROLE)
            .unwrap();
        agent
            .receive_message("hi")
            .unwrap();
        agent
            .receive_message("bye")
            .unwrap();
        let session = agent
            .stop_session()
            .expect("session");
        assert_eq!(
            session
                .transcript
                .len(),
            4
        );
        assert_eq!(session.transcript[0].role, USER_ROLE);
        assert_eq!(session.transcript[0].content, "hi");
        assert_eq!(session.transcript[1].role, ASSISTANT_ROLE);
        assert_eq!(session.transcript[1].content, STUB_LLM_REPLY);
        assert_eq!(session.transcript[2].role, USER_ROLE);
        assert_eq!(session.transcript[2].content, "bye");
        assert_eq!(session.transcript[3].role, ASSISTANT_ROLE);
    }
}
