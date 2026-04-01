mod application;
mod core;
mod infrastructure;

pub use core::{
    agent::Agent,
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
    llm::{ChatMessage, Llm},
    session::{
        merge_system_prompts, ActiveSession, ReceiveMessageError, Session, StartSessionError,
        ASSISTANT_ROLE, USER_ROLE,
    },
};
use std::io::{self, BufRead};

use anthropic_api_key_from_local_file::anthropic_api_key_from_local_file;
pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::{
    environment::ShellEnvironment,
    llm::{ClaudeLlm, DummyLlm},
};

/// Base system instructions merged with the per-session prompt on every completion (model-/adapter-level).
const BASE_SYSTEM_PROMPT: &str = "You are a concise, helpful assistant.";

/// Session-scoped task instructions (merged with [`BASE_SYSTEM_PROMPT`] for each API call).
const SESSION_SYSTEM_PROMPT: &str = "You are in an interactive CLI chat. Answer the user’s questions. The session ends when they send an empty line.";

fn main() {
    let api_key = match anthropic_api_key_from_local_file(None) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Failed to load Anthropic API key: {e}");
            std::process::exit(1);
        },
    };
    let llm = ClaudeLlm::new(api_key, Some(BASE_SYSTEM_PROMPT.to_string()));
    let static_claude_config = llm
        .static_config_json()
        .to_owned();
    let mut agent = create_agent(
        "Aria",
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
        llm,
    );
    agent.log(&static_claude_config, LogMessageLevel::Verbose);

    if let Err(e) =
        agent.start_session(Session::new(SESSION_SYSTEM_PROMPT), ASSISTANT_ROLE, USER_ROLE)
    {
        eprintln!("Failed to start session: {e:?}");
        std::process::exit(1);
    }

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader
            .read_line(&mut line)
            .unwrap_or_else(|e| {
                eprintln!("Failed to read stdin: {e}");
                std::process::exit(1);
            });
        if bytes_read == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Err(e) = agent.receive_message(trimmed) {
            eprintln!("receive_message failed: {e:?}");
            std::process::exit(1);
        }
    }

    let _ended = agent.stop_session();
}
