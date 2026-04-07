use crate::{
    environment::{Environment, LogMessageLevel},
    llm::{ChatMessage, Llm},
    session::{
        merge_system_prompts, ActiveSession, ReceiveMessageError, Session, StartSessionError,
    },
    tool::ToolRegistry,
};

/// Label for [`Agent::receive_message`] printed incoming lines (`"{label} <- {message}"`).
const PEER_INCOMING_PRINT_LABEL: &str = "peer";

/// An autonomous agent identified by name.
pub struct Agent<E: Environment, L: Llm> {
    /// Display name for this agent.
    pub name: String,
    /// Bridges output to the host environment.
    pub environment: E,
    /// Language model used for message exchange.
    pub llm: L,
    /// Tools this agent is permitted to use (frozen after construction).
    pub tools: ToolRegistry,
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

    /// Records a **peer** message, prints the incoming line as **`peer <- {message}`** (always, independent of logging level), completes with [`Llm::complete`](Llm::complete), logs [`LlmCompletion`](crate::llm::LlmCompletion) request JSON at [`LogMessageLevel::Verbose`] when the adapter supplies it and the environment allows verbose logs, then appends this **Agent**’s reply (under [`ActiveSession::agent_role`](crate::session::ActiveSession)) and prints it as **`{name} -> {reply}`**.
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

        self.print(&format!("{PEER_INCOMING_PRINT_LABEL} <- {message}"));

        let system = merge_system_prompts(
            self.llm
                .base_system_prompt(),
            &active
                .session
                .system_prompt,
        );
        let completion = self
            .llm
            .complete(
                system.as_deref(),
                &active
                    .session
                    .transcript,
            );
        if let Some(ref json) = completion.request_body_json {
            self.log(json, LogMessageLevel::Verbose);
        }
        let reply = completion.reply;

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
mod tests {
    use super::Agent;
    use crate::{
        environment::LoggingLevel,
        session::{
            merge_system_prompts, ReceiveMessageError, Session, StartSessionError, ASSISTANT_ROLE,
            USER_ROLE,
        },
        test_support::{
            agent_with_stub, InMemoryEnvironment, StubLlm, STUB_LLM_REPLY, STUB_REQUEST_JSON,
        },
        tool::ToolRegistry,
    };

    type ExpectedLoggedLines = &'static [&'static str];
    type VerboseLoggingCase = (LoggingLevel, ExpectedLoggedLines);

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
            vec![String::from("peer <- ping"), format!("a -> {}", STUB_LLM_REPLY),]
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

    #[test]
    fn stop_session_returns_none_when_no_active_session() {
        let mut agent = agent_with_stub();
        assert_eq!(agent.stop_session(), None);
    }

    #[test]
    fn stop_session_returns_none_second_time_after_session_taken() {
        let mut agent = agent_with_stub();
        agent
            .start_session(Session::new("s"), ASSISTANT_ROLE, USER_ROLE)
            .unwrap();
        assert!(agent
            .stop_session()
            .is_some());
        assert_eq!(agent.stop_session(), None);
    }

    #[test]
    fn receive_message_prints_incoming_peer_line_not_only_via_log_filter() {
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
            vec![String::from("peer <- ping"), format!("a -> {}", STUB_LLM_REPLY),]
        );
        assert_eq!(
            agent
                .environment
                .logged_lines(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn receive_message_passes_merged_system_and_transcript_to_llm() {
        let mut agent = Agent {
            name: String::from("a"),
            environment: InMemoryEnvironment::new(LoggingLevel::Standard),
            llm: StubLlm::default(),
            tools: ToolRegistry::new(vec![]),
            active_session: None,
        };
        agent
            .start_session(Session::new("session-sys"), ASSISTANT_ROLE, USER_ROLE)
            .unwrap();
        agent
            .receive_message("ping")
            .unwrap();
        let (system, messages) = agent
            .llm
            .last_complete();
        assert_eq!(system, merge_system_prompts(None, "session-sys"));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, USER_ROLE);
        assert_eq!(messages[0].content, "ping");
    }

    #[test]
    fn receive_message_passes_merged_base_and_session_system_to_llm() {
        let mut agent = Agent {
            name: String::from("a"),
            environment: InMemoryEnvironment::new(LoggingLevel::Standard),
            llm: StubLlm::with_base_prompt("adapter-base"),
            tools: ToolRegistry::new(vec![]),
            active_session: None,
        };
        agent
            .start_session(Session::new("session"), ASSISTANT_ROLE, USER_ROLE)
            .unwrap();
        agent
            .receive_message("x")
            .unwrap();
        let (system, _) = agent
            .llm
            .last_complete();
        assert_eq!(system, merge_system_prompts(Some("adapter-base"), "session"));
    }

    #[test]
    fn receive_message_logs_request_body_json_at_verbose_only() {
        let cases: &[VerboseLoggingCase] =
            &[(LoggingLevel::Verbose, &[STUB_REQUEST_JSON]), (LoggingLevel::Standard, &[])];
        for &(level, expected_logged) in cases {
            let mut agent = Agent {
                name: String::from("a"),
                environment: InMemoryEnvironment::new(level),
                llm: StubLlm::with_request_json(STUB_REQUEST_JSON),
                tools: ToolRegistry::new(vec![]),
                active_session: None,
            };
            agent
                .start_session(Session::new("t"), ASSISTANT_ROLE, USER_ROLE)
                .unwrap();
            agent
                .receive_message("hi")
                .unwrap();
            let expected_logged: Vec<String> = expected_logged
                .iter()
                .copied()
                .map(String::from)
                .collect();
            assert_eq!(
                agent
                    .environment
                    .logged_lines(),
                expected_logged
            );
            assert_eq!(
                agent
                    .environment
                    .lines(),
                vec![String::from("peer <- hi"), format!("a -> {}", STUB_LLM_REPLY),]
            );
        }
    }
}
