//! Game loop for the WASI guest without `wit_bindgen::block_on` inside Omnia’s async HTTP handler.
//!
//! [`aria_core::game_loop::play_game`] is synchronous and calls [`aria_core::llm::Llm::complete`] /
//! [`crate::wasi_arena::WasiArena::send`], which use `block_on` for async WASI I/O. Invoking that from
//! `async fn play_handler` deadlocks: the handler never completes the nested outbound HTTP work.
//!
//! Axum’s `Handler` trait requires a `Send` future. An `async fn` that took `game: &dyn Game` produced a
//! non-`Send` future on wasm32; this module uses the concrete [`KnockKnockGame`] only.

use aria_core::agent::Agent;
use aria_core::arena::ArenaError;
use aria_core::game::Game;
use aria_core::llm::{ChatMessage, Llm};
use aria_core::session::{
    merge_system_prompts, ReceiveMessageError, Session, StartSessionError, ASSISTANT_ROLE, USER_ROLE,
};
use aria_core::games::KnockKnockGame;

use crate::wasi_arena::WasiArena;
use crate::wasi_environment::WasiEnvironment;
use crate::wasi_llm::WasiLlm;

const PEER_INCOMING_PRINT_LABEL: &str = "peer";

/// One peer line: append transcript, print, LLM async completion, print agent line. Inlined so the
/// outer `play_knock_knock_wasi` future stays `Send` for Axum (no nested `async fn` with `&mut Agent`).
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

/// Knock-knock game only — uses a concrete [`KnockKnockGame`] (no `&dyn Game`) so the future is [`Send`].
pub async fn play_knock_knock_wasi(
    mut agent: Agent<WasiEnvironment, WasiLlm>,
    arena: WasiArena,
) -> Result<usize, PlayGameWasiError> {
    let game = KnockKnockGame;
    let challenge = game.challenge();

    let system_prompt = match challenge.private_context {
        Some(ref ctx) => format!("{}\n\n{ctx}", challenge.system_prompt),
        None => challenge.system_prompt,
    };

    agent
        .start_session(Session::new(system_prompt), ASSISTANT_ROLE, USER_ROLE)
        .map_err(PlayGameWasiError::SessionStart)?;

    let mut agent_reply = receive_one_turn!(agent, &challenge.opening_message);

    let mut turn = 0;
    loop {
        let peer_reply = arena
            .send_async(&agent_reply)
            .await
            .map_err(PlayGameWasiError::Arena)?;

        turn += 1;

        if game.is_complete(turn, &peer_reply) {
            break;
        }

        agent_reply = receive_one_turn!(agent, &peer_reply);
    }

    let _ = agent.stop_session();
    Ok(turn)
}

#[derive(Debug)]
pub enum PlayGameWasiError {
    SessionStart(StartSessionError),
    AgentReceive(ReceiveMessageError),
    Arena(ArenaError),
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
