//! Shared test doubles and fixtures (`#[cfg(test)]` only).
//!
//! Agent / [`Environment`] / [`Llm`] fakes for core tests, plus temp-file helpers for runtime adapter tests.

use std::{cell::RefCell, io::Write};

use tempfile::NamedTempFile;

use crate::core::{
    agent::Agent,
    environment::{Environment, LoggingLevel},
    llm::{ChatMessage, Llm, LlmCompletion},
    transport::{PostJsonTransport, TransportError},
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
        active_session: None,
    }
}

/// Minimal [`Environment`] for tests that only need trait satisfaction (e.g. [`crate::application::factories::create_agent::create_agent`] wiring).
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
// Transport doubles
// ---------------------------------------------------------------------------

/// Configurable [`PostJsonTransport`] for tests. Returns `response_bytes` on every call,
/// or `Err(TransportError::Other(...))` if `error` is set.
pub struct StubPostJsonTransport {
    response_bytes: Vec<u8>,
    error: Option<String>,
}

impl StubPostJsonTransport {
    /// Creates a transport that always returns the given bytes.
    pub fn with_response(response_bytes: Vec<u8>) -> Self {
        Self {
            response_bytes,
            error: None,
        }
    }

    /// Creates a transport that always returns the given error.
    pub fn with_error(msg: impl Into<String>) -> Self {
        Self {
            response_bytes: Vec::new(),
            error: Some(msg.into()),
        }
    }
}

impl PostJsonTransport for StubPostJsonTransport {
    fn post_json(
        &self,
        _url: &str,
        _headers: &[(&str, &str)],
        _body: &serde_json::Value,
    ) -> Result<Vec<u8>, TransportError> {
        if let Some(ref err) = self.error {
            return Err(TransportError::Other(err.clone()));
        }
        Ok(self
            .response_bytes
            .clone())
    }
}
