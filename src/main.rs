mod application;
mod core;
mod infrastructure;

#[cfg(test)]
mod test_support;

pub use core::{
    agent::Agent,
    environment::{log_message_is_allowed, Environment, LogMessageLevel, LoggingLevel},
    llm::{ChatMessage, Llm, LlmCompletion},
    runtime::{Runtime, RuntimeError, ANTHROPIC_API_KEY_SECRET},
    session::{
        merge_system_prompts, ActiveSession, ReceiveMessageError, Session, StartSessionError,
        ASSISTANT_ROLE, USER_ROLE,
    },
    transport::{PostJsonTransport, TransportError},
};

pub use application::factories::create_agent::create_agent;
pub use infrastructure::adapters::{
    environment::ShellEnvironment,
    llm::{ClaudeLlm, DummyLlm, KnockKnockAudienceLlm},
    transport::JsonHttp,
    OmniaRuntime, SecureAgent, VaultAnthropicLocalFile, ANTHROPIC_VAULT_LOCKER_ID,
    ANTHROPIC_VAULT_SECRET_ID,
};

/// Base system instructions merged with the per-session prompt on every completion (model-/adapter-level).
const BASE_SYSTEM_PROMPT: &str =
    "You are telling a knock-knock joke to your peer. Be concise at every step.";

/// Synthetic peer line so the joke-telling participant (display name **SecureAgent**) can open the dialogue (see [`KNOCK_KNOCK_TELLER_SESSION_PROMPT`] step 1).
const SYNTHETIC_PEER_GREETING: &str = "Hello.";

/// Session-scoped instructions for the joke teller (merged with [`BASE_SYSTEM_PROMPT`] for each API call).
const KNOCK_KNOCK_TELLER_SESSION_PROMPT: &str = r#"You are running a fixed knock-knock joke exchange. You are the joke teller; your peer is the audience.

Turn order:
1) The peer opens with a brief greeting (e.g. "Hello."). Reply only by asking if they would like to hear a knock knock joke (one short sentence ending with a question mark).
2) When the peer says "yes", reply with only this on the first line: Knock knock.
3) When the peer says "Who's there?", put the setup name as exactly one word on the first line of your message (no leading label or punctuation before that word). The peer will say "{word} who?".
4) When the peer says "{word} who?" using that setup word, give a short punchline.
5) When the peer says "haha", reply with one brief parting pleasantry only. The scripted exchange ends there.

Be concise at every step."#;

fn play_knock_knock(
    peer: &mut Agent<ShellEnvironment, KnockKnockAudienceLlm>,
    agent: &mut SecureAgent,
) {
    // Transcript roles must match Anthropic Messages API: Claude outputs `assistant`, canned lines are `user`,
    // so each turn ends with `user` before the next completion (see `Agent::receive_message`).
    if let Err(e) = peer.start_session(Session::new(""), USER_ROLE, ASSISTANT_ROLE) {
        eprintln!("Failed to start peer session: {e:?}");
        std::process::exit(1);
    }
    if let Err(e) = agent.start_session(
        Session::new(KNOCK_KNOCK_TELLER_SESSION_PROMPT),
        ASSISTANT_ROLE,
        USER_ROLE,
    ) {
        eprintln!("Failed to start agent session: {e:?}");
        std::process::exit(1);
    }

    let mut peer_recv = |text: &str| -> String {
        peer.receive_message(text)
            .unwrap_or_else(|e| {
                eprintln!("peer receive_message failed: {e:?}");
                std::process::exit(1);
            })
    };
    let mut agent_recv = |text: &str| -> String {
        agent
            .receive_message(text)
            .unwrap_or_else(|e| {
                eprintln!("agent receive_message failed: {e:?}");
                std::process::exit(1);
            })
    };

    let invitation = agent_recv(SYNTHETIC_PEER_GREETING);
    let after_yes = peer_recv(&invitation);
    let after_whos_there = peer_recv(&agent_recv(&after_yes));
    let after_setup_who = peer_recv(&agent_recv(&after_whos_there));
    let _haha = peer_recv(&agent_recv(&after_setup_who));
    let _parting = agent_recv(&_haha);

    let _ = peer.stop_session();
    let _ = agent.stop_session();
}

fn main() {
    let mut peer = create_agent(
        "Peer",
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
        KnockKnockAudienceLlm::new(),
    );

    let vault = Box::new(VaultAnthropicLocalFile::new(None));
    let runtime = match OmniaRuntime::new(vault, ANTHROPIC_VAULT_LOCKER_ID) {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create OmniaRuntime: {e:?}");
            std::process::exit(1);
        },
    };

    let mut agent = match SecureAgent::new(
        runtime,
        JsonHttp::new(),
        Some(BASE_SYSTEM_PROMPT.to_string()),
        ShellEnvironment {
            logging_level: LoggingLevel::None,
        },
    ) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to initialize SecureAgent: {e:?}");
            std::process::exit(1);
        },
    };

    play_knock_knock(&mut peer, &mut agent);
}
