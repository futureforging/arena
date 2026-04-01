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
    llm::{ClaudeLlm, DummyLlm},
};

/// Base system instructions merged with the per-session prompt on every completion (model-/adapter-level).
const BASE_SYSTEM_PROMPT: &str = "You are a concise, helpful assistant.";

/// User line that starts the scripted knock-knock exchange.
const KNOCK_KNOCK_OPENER: &str = "Tell me a knock knock joke.";

/// Standard knock-knock reply after "Knock knock."
const KNOCK_KNOCK_WHOS_THERE: &str = "Who's there?";

/// Laughter line before the assistant’s parting pleasantry.
const KNOCK_KNOCK_HAHA: &str = "haha";

/// Session-scoped instructions for the knock-knock game (merged with [`BASE_SYSTEM_PROMPT`] for each API call).
const KNOCK_KNOCK_SESSION_PROMPT: &str = r#"You are running a fixed knock-knock joke exchange.

Turn order:
1) The user opens by asking for a joke. Reply with only this on the first line: Knock knock.
2) The user says "Who's there?". Put the setup name as exactly one word on the first line of your message (no leading label or punctuation before that word). The user will repeat it in "{word} who?".
3) The user says "{word} who?" using that setup word. Give a short punchline.
4) The user says "haha". Reply with one brief parting pleasantry only. The scripted exchange ends there.

Be concise at every step."#;

/// Returns the setup word from the assistant’s message after "Who's there?" (first non-empty line, first word, without trailing ASCII punctuation).
fn parse_setup_from_assistant_reply(text: &str) -> Option<String> {
    let first_line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let first_word = first_line
        .split_whitespace()
        .next()?;
    let word = first_word.trim_end_matches(|c: char| c.is_ascii_punctuation());
    if word.is_empty() {
        None
    } else {
        Some(word.to_string())
    }
}

fn play_knock_knock(agent: &mut Agent<ShellEnvironment, ClaudeLlm>) {
    if let Err(e) =
        agent.start_session(Session::new(KNOCK_KNOCK_SESSION_PROMPT), ASSISTANT_ROLE, USER_ROLE)
    {
        eprintln!("Failed to start session: {e:?}");
        std::process::exit(1);
    }

    let mut send = |text: &str| -> String {
        agent
            .receive_message(text)
            .unwrap_or_else(|e| {
                eprintln!("receive_message failed: {e:?}");
                std::process::exit(1);
            })
    };

    let _after_opener = send(KNOCK_KNOCK_OPENER);
    let after_whos_there = send(KNOCK_KNOCK_WHOS_THERE);
    let setup = parse_setup_from_assistant_reply(&after_whos_there).unwrap_or_else(|| {
        eprintln!("Could not parse setup name from assistant reply: {}", after_whos_there);
        std::process::exit(1);
    });
    let setup_who = format!("{setup} who?");
    let _after_setup_who = send(&setup_who);
    let _parting = send(KNOCK_KNOCK_HAHA);

    let _ended = agent.stop_session();
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

    let mut agent = create_agent(
        "Aria",
        ShellEnvironment {
            logging_level: LoggingLevel::Standard,
        },
        llm,
    );

    play_knock_knock(&mut agent);
}

#[cfg(test)]
mod knock_knock_tests {
    use super::parse_setup_from_assistant_reply;

    #[test]
    fn parse_setup_single_word_no_punctuation() {
        assert_eq!(parse_setup_from_assistant_reply("Boo").as_deref(), Some("Boo"));
    }

    #[test]
    fn parse_setup_strips_trailing_period() {
        assert_eq!(parse_setup_from_assistant_reply("Boo.").as_deref(), Some("Boo"));
    }

    #[test]
    fn parse_setup_first_line_first_word() {
        assert_eq!(parse_setup_from_assistant_reply("Lettuce\n").as_deref(), Some("Lettuce"));
    }

    #[test]
    fn parse_setup_skips_leading_blank_lines() {
        assert_eq!(parse_setup_from_assistant_reply("\n\nOrange.").as_deref(), Some("Orange"));
    }

    #[test]
    fn parse_setup_empty_is_none() {
        assert_eq!(parse_setup_from_assistant_reply(""), None);
        assert_eq!(parse_setup_from_assistant_reply("   \n  "), None);
    }
}
