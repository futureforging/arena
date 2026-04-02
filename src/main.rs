mod application;
mod core;
mod infrastructure;

pub use core::{
    agent::Agent,
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
    llm::{ChatMessage, Llm, LlmCompletion},
    session::{
        merge_system_prompts, ActiveSession, ReceiveMessageError, Session, StartSessionError,
        ASSISTANT_ROLE, USER_ROLE,
    },
};

use anthropic_api_key_from_local_file::anthropic_api_key_from_local_file;
pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::{
    environment::ShellEnvironment,
    llm::{ClaudeLlm, DummyLlm, KnockKnockUserLlm},
};

/// Base system instructions merged with the per-session prompt on every completion (model-/adapter-level).
const BASE_SYSTEM_PROMPT: &str = "You are a concise, helpful assistant.";

/// User line that starts the scripted knock-knock exchange.
const KNOCK_KNOCK_OPENER: &str = "Tell me a knock knock joke.";

/// Session-scoped instructions for the knock-knock game (merged with [`BASE_SYSTEM_PROMPT`] for each API call).
const KNOCK_KNOCK_SESSION_PROMPT: &str = r#"You are running a fixed knock-knock joke exchange.

Turn order:
1) The user opens by asking for a joke. Reply with only this on the first line: Knock knock.
2) The user says "Who's there?". Put the setup name as exactly one word on the first line of your message (no leading label or punctuation before that word). The user will repeat it in "{word} who?".
3) The user says "{word} who?" using that setup word. Give a short punchline.
4) The user says "haha". Reply with one brief parting pleasantry only. The scripted exchange ends there.

Be concise at every step."#;

fn play_knock_knock(
    assistant: &mut Agent<ShellEnvironment, ClaudeLlm>,
    user: &mut Agent<ShellEnvironment, KnockKnockUserLlm>,
) {
    if let Err(e) =
        assistant.start_session(Session::new(KNOCK_KNOCK_SESSION_PROMPT), ASSISTANT_ROLE, USER_ROLE)
    {
        eprintln!("Failed to start assistant session: {e:?}");
        std::process::exit(1);
    }
    if let Err(e) = user.start_session(Session::new(""), USER_ROLE, ASSISTANT_ROLE) {
        eprintln!("Failed to start user session: {e:?}");
        std::process::exit(1);
    }

    let mut assistant_recv = |text: &str| -> String {
        assistant
            .receive_message(text)
            .unwrap_or_else(|e| {
                eprintln!("assistant receive_message failed: {e:?}");
                std::process::exit(1);
            })
    };
    let mut user_recv = |text: &str| -> String {
        user.receive_message(text)
            .unwrap_or_else(|e| {
                eprintln!("user receive_message failed: {e:?}");
                std::process::exit(1);
            })
    };

    let after_opener = assistant_recv(KNOCK_KNOCK_OPENER);
    let after_whos_there = assistant_recv(&user_recv(&after_opener));
    let after_setup_who = assistant_recv(&user_recv(&after_whos_there));
    let _parting = assistant_recv(&user_recv(&after_setup_who));

    let _ = assistant.stop_session();
    let _ = user.stop_session();
}

fn main() {
    let api_key = match anthropic_api_key_from_local_file(None) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Failed to load Anthropic API key: {e}");
            std::process::exit(1);
        },
    };
    let llm = ClaudeLlm::new(api_key, Some(BASE_SYSTEM_PROMPT.to_string()));

    let mut assistant = create_agent(
        "Assistant",
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
        llm,
    );

    let mut user = create_agent(
        "User",
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
        KnockKnockUserLlm::new(),
    );

    play_knock_knock(&mut assistant, &mut user);
}
