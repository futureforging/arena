//! Shared test doubles and fixtures for `verity-core` and downstream crates.
//!
//! Agent / [`Environment`] / [`Llm`] fakes, plus temp-file helpers for runtime adapter tests.

use std::{cell::RefCell, io::Write, sync::Mutex};

use tempfile::NamedTempFile;

use crate::{
    agent::Agent,
    environment::{Environment, LoggingLevel},
    game::{Challenge, Game},
    llm::{ChatMessage, Llm, LlmCompletion},
    tool::{Tool, ToolDescriptor, ToolError, ToolRegistry},
};

// ---------------------------------------------------------------------------
// Temp files (runtime / vault tests)
// ---------------------------------------------------------------------------

/// Creates a [`NamedTempFile`] and writes `line` with [`writeln!`], matching hand-rolled test setup.
pub fn named_temp_file_with_writeln(line: &str) -> std::io::Result<NamedTempFile> {
    let mut tmp = NamedTempFile::new()?;
    writeln!(tmp, "{line}")?;
    Ok(tmp)
}

// ---------------------------------------------------------------------------
// Agent / environment / LLM doubles
// ---------------------------------------------------------------------------

/// Reply returned by [`StubLlm`]; only used to assert the agent prints whatever the port returns.
pub const STUB_LLM_REPLY: &str = "stub-llm-reply";

pub const STUB_REQUEST_JSON: &str = r#"{"logged":"request"}"#;

/// Arguments captured by [`StubLlm`] on the last [`Llm::complete`] call.
type StubCompleteInputs = (Option<String>, Vec<ChatMessage>);

/// Records [`Environment::print`] and, after [`Environment::log`] filtering, every string passed to [`Environment::emit_log`].
pub struct InMemoryEnvironment {
    lines: RefCell<Vec<String>>,
    logged: RefCell<Vec<String>>,
    pub logging_level: LoggingLevel,
}

impl InMemoryEnvironment {
    pub fn new(logging_level: LoggingLevel) -> Self {
        Self {
            lines: RefCell::new(Vec::new()),
            logged: RefCell::new(Vec::new()),
            logging_level,
        }
    }

    /// Returns a copy of every string passed to [`Environment::print`].
    pub fn lines(&self) -> Vec<String> {
        self.lines
            .borrow()
            .clone()
    }

    /// Returns a copy of every string passed to [`Environment::emit_log`] via [`Environment::log`].
    pub fn logged_lines(&self) -> Vec<String> {
        self.logged
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
        self.logged
            .borrow_mut()
            .push(message.to_string());
    }
}

pub struct StubLlm {
    reply: String,
    request_body_json: Option<String>,
    base_prompt: Option<String>,
    last_complete: RefCell<Option<StubCompleteInputs>>,
}

impl Default for StubLlm {
    fn default() -> Self {
        Self {
            reply: STUB_LLM_REPLY.to_string(),
            request_body_json: None,
            base_prompt: None,
            last_complete: RefCell::new(None),
        }
    }
}

impl StubLlm {
    pub fn with_request_json(json: impl Into<String>) -> Self {
        Self {
            request_body_json: Some(json.into()),
            ..Default::default()
        }
    }

    pub fn with_base_prompt(s: impl Into<String>) -> Self {
        Self {
            base_prompt: Some(s.into()),
            ..Default::default()
        }
    }

    pub fn last_complete(&self) -> StubCompleteInputs {
        self.last_complete
            .borrow()
            .clone()
            .expect("complete was called")
    }
}

impl Llm for StubLlm {
    fn base_system_prompt(&self) -> Option<&str> {
        self.base_prompt
            .as_deref()
    }

    fn complete(&self, system: Option<&str>, messages: &[ChatMessage]) -> LlmCompletion {
        *self
            .last_complete
            .borrow_mut() = Some((system.map(str::to_string), messages.to_vec()));
        LlmCompletion {
            reply: self
                .reply
                .clone(),
            request_body_json: self
                .request_body_json
                .clone(),
        }
    }
}

pub fn agent_with_stub() -> Agent<InMemoryEnvironment, StubLlm> {
    Agent {
        name: String::from("a"),
        environment: InMemoryEnvironment::new(LoggingLevel::Standard),
        llm: StubLlm::default(),
        tools: ToolRegistry::new(vec![]),
        active_session: None,
    }
}

/// Minimal [`Environment`] for tests that only need trait satisfaction (e.g. wiring tests).
pub struct NoopEnvironment;

impl Environment for NoopEnvironment {
    fn print(&self, _s: &str) {}

    fn logging_level(&self) -> LoggingLevel {
        LoggingLevel::None
    }

    fn emit_log(&self, _message: &str) {}
}

/// Minimal [`Llm`] with an empty reply (for factory / wiring tests).
pub struct EmptyReplyLlm;

impl Llm for EmptyReplyLlm {
    fn complete(&self, _system: Option<&str>, _messages: &[ChatMessage]) -> LlmCompletion {
        LlmCompletion {
            reply: String::new(),
            request_body_json: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Arena / Game doubles (game loop tests)
// ---------------------------------------------------------------------------

/// Stub arena tool for game loop tests. Returns replies from a list
/// in order, then returns empty strings.
pub struct StubArenaTool {
    replies: Mutex<Vec<String>>,
}

impl StubArenaTool {
    pub fn new(replies: Vec<String>) -> Self {
        Self {
            replies: Mutex::new(replies),
        }
    }
}

impl Tool for StubArenaTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "arena",
            description: "stub arena tool for tests",
            input_schema: serde_json::json!({}),
        }
    }

    fn execute(&self, _input: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let mut replies = self
            .replies
            .lock()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let reply = if replies.is_empty() {
            String::new()
        } else {
            replies.remove(0)
        };
        Ok(serde_json::json!({ "reply": reply }))
    }
}

/// Stub arena tool that always fails.
pub struct FailingArenaTool;

impl Tool for FailingArenaTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "arena",
            description: "failing arena tool for tests",
            input_schema: serde_json::json!({}),
        }
    }

    fn execute(&self, _input: &serde_json::Value) -> Result<serde_json::Value, ToolError> {
        Err(ToolError::ExecutionFailed(String::from("boom")))
    }
}

/// Minimal [`Game`] for [`crate::game_loop::play_game`] tests.
pub struct StubGame {
    pub max_turns: usize,
}

impl Game for StubGame {
    fn challenge(&self) -> Challenge {
        Challenge {
            system_prompt: String::from("test system prompt"),
            private_context: None,
            opening_message: String::from("hello"),
        }
    }

    fn is_complete(&self, turn: usize, last_peer_reply: &str) -> bool {
        turn >= self.max_turns || last_peer_reply.is_empty()
    }
}
