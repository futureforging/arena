use aria_poc_2::{
    Arena, ArenaHttpClient, LoggingLevel, OmniaRuntime, OmniaWasiHttpPostJson,
    OmniaWasiVaultAnthropicLocal, SecureAgent, Session, ShellEnvironment,
    ANTHROPIC_VAULT_LOCKER_ID, ASSISTANT_ROLE, USER_ROLE,
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

fn play_knock_knock_via_arena(agent: &mut SecureAgent, arena: &dyn Arena) {
    if let Err(e) = agent.start_session(
        Session::new(KNOCK_KNOCK_TELLER_SESSION_PROMPT),
        ASSISTANT_ROLE,
        USER_ROLE,
    ) {
        eprintln!("Failed to start agent session: {e:?}");
        std::process::exit(1);
    }

    // The agent needs a first message to respond to.
    // Send the synthetic greeting as if it came from the peer.
    let mut agent_reply = agent
        .receive_message(SYNTHETIC_PEER_GREETING)
        .unwrap_or_else(|e| {
            eprintln!("agent receive_message failed: {e:?}");
            std::process::exit(1);
        });

    // Exchange messages via the arena until the peer sends an empty reply
    // or we hit the turn limit (knock-knock is 5 exchanges).
    let max_turns = 10;
    for _ in 0..max_turns {
        let peer_reply = match arena.send(&agent_reply) {
            Ok(reply) if reply.is_empty() => break,
            Ok(reply) => reply,
            Err(e) => {
                eprintln!("arena send failed: {e}");
                std::process::exit(1);
            },
        };

        agent_reply = agent
            .receive_message(&peer_reply)
            .unwrap_or_else(|e| {
                eprintln!("agent receive_message failed: {e:?}");
                std::process::exit(1);
            });
    }

    let _ = agent.stop_session();
}

fn main() {
    // Runtime (vault for API key)
    let vault = Box::new(OmniaWasiVaultAnthropicLocal::new(None));
    let runtime = match OmniaRuntime::new(vault, ANTHROPIC_VAULT_LOCKER_ID) {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create OmniaRuntime: {e:?}");
            std::process::exit(1);
        },
    };

    // Transport (outbound HTTP for Claude API)
    let transport = match OmniaWasiHttpPostJson::new() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create OmniaWasiHttpPostJson: {e:?}");
            std::process::exit(1);
        },
    };

    // Arena client (talks to arena-stub on localhost:3000)
    let arena_transport = match OmniaWasiHttpPostJson::new() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create arena transport: {e:?}");
            std::process::exit(1);
        },
    };
    let arena = ArenaHttpClient::new("http://127.0.0.1:3000", arena_transport);

    // SecureAgent (Claude-backed joke teller)
    let mut agent = match SecureAgent::new(
        runtime,
        transport,
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

    play_knock_knock_via_arena(&mut agent, &arena);
}
