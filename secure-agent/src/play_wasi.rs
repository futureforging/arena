//! Game loop for the WASI guest without `wit_bindgen::block_on` inside Omnia’s async HTTP handler.
//!
//! [`verity_core::game_loop::play_game`] is synchronous and calls [`verity_core::llm::Llm::complete`] /
//! the arena adapter's `send_sync` (which uses `block_on` for async WASI I/O). Invoking that from
//! `async fn play_handler` deadlocks: the handler never completes the nested outbound HTTP work.
//!
//! Axum’s `Handler` trait requires a `Send` future. An `async fn` that took `game: &dyn Game` produced a
//! non-`Send` future on wasm32; this module uses the concrete [`PsiGame`] only.

use verity_core::agent::Agent;
use verity_core::game::Game;
use verity_core::llm::{ChatMessage, Llm};
use verity_core::session::{
    merge_system_prompts, ReceiveMessageError, Session, StartSessionError, ASSISTANT_ROLE, USER_ROLE,
};
use verity_core::games::{PsiGame, Role};

use crate::arena_transport::{ArenaTransport, WasiArenaError};
use crate::wasi_environment::WasiEnvironment;
use crate::wasi_llm::WasiLlm;

const PEER_INCOMING_PRINT_LABEL: &str = "peer";

/// Must match `PSI_PEER_AGREED_MESSAGE` in `arena-stub` (`psi_peer.rs`).
const PSI_PEER_AGREED_MESSAGE: &str = "Agreed. Send your hashes.";

/// One peer line: append transcript, print, LLM async completion, print agent line. Inlined so the
/// outer `play_psi_wasi` future stays `Send` for Axum (no nested `async fn` with `&mut Agent`).
macro_rules! receive_one_turn {
    ($agent:ident, $message:expr) => {{
        let mut active = $agent
            .active_session
            .take()
            .ok_or(PlayGameWasiError::AgentReceive(ReceiveMessageError::NoActiveSession))?;
        active.session.transcript.push(ChatMessage {
            role: active.peer_role.clone(),
            content: $message.to_string(),
        });
        $agent.print(&format!("{PEER_INCOMING_PRINT_LABEL} <- {}", $message));
        let system = merge_system_prompts(
            $agent.llm.base_system_prompt(),
            &active.session.system_prompt,
        );
        let reply = match $agent
            .llm
            .complete_async(system.as_deref(), &active.session.transcript)
            .await
        {
            Ok(s) => s,
            Err(e) => format!("(anthropic error) {e}"),
        };
        active.session.transcript.push(ChatMessage {
            role: active.agent_role.clone(),
            content: reply.clone(),
        });
        $agent.active_session = Some(active);
        $agent.print(&format!("{} -> {}", $agent.name, reply));
        reply
    }};
}

/// PSI game — uses a concrete [`PsiGame`] so the future is [`Send`] for Axum.
pub async fn play_psi_wasi<A: ArenaTransport>(
    mut agent: Agent<WasiEnvironment, WasiLlm>,
    arena: A,
    role: Role,
) -> Result<usize, PlayGameWasiError> {
    arena
        .reset_async()
        .await
        .map_err(PlayGameWasiError::Arena)?;

    let private_set = generate_random_set();
    let game = PsiGame::new(private_set, role);
    let challenge = game.challenge();

    let system_prompt = match challenge.private_context {
        Some(ref ctx) => format!("{}\n\n{ctx}", challenge.system_prompt),
        None => challenge.system_prompt,
    };

    agent
        .start_session(Session::new(system_prompt), ASSISTANT_ROLE, USER_ROLE)
        .map_err(PlayGameWasiError::SessionStart)?;

    // Role-specific first turn:
    //   First mover  — send a literal "Hello." to the arena (no LLM call), then
    //                  receive the peer's response and run it through the agent.
    //                  This counts as one completed peer exchange (turn = 1).
    //   Second mover — receive the peer's opening line from the arena (production),
    //                  or on StubArena fall back to the canned opening_message (stub
    //                  does not support receive-only sync). Then run through the agent.
    //                  No send round trip yet (turn = 0).
    let (mut agent_reply, mut turn) = match role {
        Role::First => {
            agent.print(&format!("{} -> Hello.", agent.name));
            let peer_reply = arena
                .send_async("Hello.")
                .await
                .map_err(PlayGameWasiError::Arena)?;
            let reply = receive_one_turn!(agent, &peer_reply);
            (reply, 1usize)
        }
        Role::Second => {
            let reply = match arena.receive_async().await {
                Ok(peer_line) => receive_one_turn!(agent, &peer_line),
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("StubArena") {
                        receive_one_turn!(agent, &challenge.opening_message)
                    } else {
                        return Err(PlayGameWasiError::Arena(e));
                    }
                }
            };
            (reply, 0usize)
        }
    };

    let mut printed_private_set_after_agreement = false;
    loop {
        let peer_reply = arena
            .send_async(&agent_reply)
            .await
            .map_err(PlayGameWasiError::Arena)?;

        turn += 1;

        if game.is_complete(turn, &peer_reply) {
            break;
        }

        if !printed_private_set_after_agreement && peer_reply.trim() == PSI_PEER_AGREED_MESSAGE {
            printed_private_set_after_agreement = true;
            agent.print(&format!(
                "[PRIVATE — local only, not sent to peer] {} private letter set: {:?}",
                agent.name,
                game.private_set()
            ));
        }

        agent_reply = receive_one_turn!(agent, &peer_reply);
    }

    let _ = agent.stop_session();
    Ok(turn)
}

/// Generate 10 unique random lowercase letters.
fn generate_random_set() -> Vec<char> {
    let mut letters: Vec<char> = ('a'..='z').collect();
    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy).expect("getrandom failed");
    for (k, i) in (1..letters.len()).rev().enumerate() {
        let j = (entropy[k] as usize) % (i + 1);
        letters.swap(i, j);
    }
    letters.truncate(10);
    letters.sort();
    letters
}

#[derive(Debug)]
pub enum PlayGameWasiError {
    SessionStart(StartSessionError),
    AgentReceive(ReceiveMessageError),
    Arena(WasiArenaError),
}

impl std::fmt::Display for PlayGameWasiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionStart(e) => write!(f, "session start failed: {e:?}"),
            Self::AgentReceive(e) => write!(f, "agent receive failed: {e:?}"),
            Self::Arena(e) => write!(f, "arena: {e:?}"),
        }
    }
}

impl std::error::Error for PlayGameWasiError {}
