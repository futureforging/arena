//! Game loop for the WASI guest without `wit_bindgen::block_on` inside Omnia’s async HTTP handler.
//!
//! [`verity_core::game_loop::play_game`] is synchronous and calls [`verity_core::llm::Llm::complete`] /
//! the arena adapter's `send_sync` (which uses `block_on` for async WASI I/O). Invoking that from
//! `async fn play_handler` deadlocks: the handler never completes the nested outbound HTTP work.
//!
//! Axum’s `Handler` trait requires a `Send` future. An `async fn` that took `game: &dyn Game` produced a
//! non-`Send` future on wasm32; this module uses the concrete [`PsiGame`] only.

use verity_core::{
    agent::Agent,
    game::Game,
    games::{PsiGame, Role},
    llm::{ChatMessage, Llm},
    session::{
        merge_system_prompts, ReceiveMessageError, Session, StartSessionError, ASSISTANT_ROLE,
        USER_ROLE,
    },
};

use crate::{
    arena_transport::{ArenaTransport, WasiArenaError},
    extract_peer_arrays::{extract_peer_hash_array, extract_peer_number_array},
    operator_parse::parse_private_set,
    wasi_environment::WasiEnvironment,
    wasi_llm::WasiLlm,
};

const PEER_INCOMING_PRINT_LABEL: &str = "peer";

fn collect_transcript_for_extraction(agent: &Agent<WasiEnvironment, WasiLlm>) -> Vec<ChatMessage> {
    agent
        .active_session
        .as_ref()
        .map(|s| {
            s.session
                .transcript
                .clone()
        })
        .unwrap_or_default()
}

/// One peer line: append transcript, print, LLM async completion, print agent line. Inlined so the
/// outer `play_psi_wasi` future stays `Send` for Axum (no nested `async fn` with `&mut Agent`).
macro_rules! receive_one_turn {
    ($agent:ident, $message:expr) => {{
        let mut active = $agent
            .active_session
            .take()
            .ok_or(PlayGameWasiError::AgentReceive(ReceiveMessageError::NoActiveSession))?;
        active
            .session
            .transcript
            .push(ChatMessage {
                role: active
                    .peer_role
                    .clone(),
                content: $message.to_string(),
            });
        $agent.print(&format!("{PEER_INCOMING_PRINT_LABEL} <- {}", $message));
        let system = merge_system_prompts(
            $agent
                .llm
                .base_system_prompt(),
            &active
                .session
                .system_prompt,
        );
        let reply = match $agent
            .llm
            .complete_async(
                system.as_deref(),
                &active
                    .session
                    .transcript,
            )
            .await
        {
            Ok(s) => s,
            Err(e) => format!("(anthropic error) {e}"),
        };
        active
            .session
            .transcript
            .push(ChatMessage {
                role: active
                    .agent_role
                    .clone(),
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

    // 1) Fetch private set from operator.
    let operator_messages = arena
        .operator_sync_async(0)
        .await
        .map_err(PlayGameWasiError::Arena)?;

    // Always print operator messages so we can see the format the server actually uses.
    for (i, m) in operator_messages
        .iter()
        .enumerate()
    {
        agent.print(&format!("[operator msg {i}/{}] {m}", operator_messages.len()));
    }

    let private_set = operator_messages
        .iter()
        .find_map(|m| parse_private_set(m))
        .ok_or_else(|| {
            let preview = operator_messages
                .iter()
                .enumerate()
                .map(|(i, m)| format!("  msg {i}: {m}"))
                .collect::<Vec<_>>()
                .join("\n");
            PlayGameWasiError::Arena(WasiArenaError::Other(format!(
                "no parseable private_set in {} operator message(s):\n{preview}",
                operator_messages.len()
            )))
        })?;

    agent.print(&format!(
        "[PRIVATE — local only, not sent to peer] {} private number set: {:?}",
        agent.name, private_set
    ));

    let game = PsiGame::new(private_set, role);
    let challenge = game.challenge();

    let system_prompt = match challenge.private_context {
        Some(ref ctx) => format!("{}\n\n{ctx}", challenge.system_prompt),
        None => challenge.system_prompt,
    };

    agent
        .start_session(Session::new(system_prompt), ASSISTANT_ROLE, USER_ROLE)
        .map_err(PlayGameWasiError::SessionStart)?;

    // 2) Role-conditional first turn.
    let (mut agent_reply, mut turn) = match role {
        Role::First => {
            agent.print(&format!("{} -> Hello.", agent.name));
            let peer_reply = arena
                .send_async("Hello.")
                .await
                .map_err(PlayGameWasiError::Arena)?;
            let reply = receive_one_turn!(agent, &peer_reply);
            (reply, 1usize)
        },
        Role::Second => {
            let peer_line = arena
                .receive_async()
                .await
                .map_err(PlayGameWasiError::Arena)?;
            let reply = receive_one_turn!(agent, &peer_line);
            (reply, 0usize)
        },
    };

    // 3) Chat exchange.
    loop {
        let peer_reply = arena
            .send_async(&agent_reply)
            .await
            .map_err(PlayGameWasiError::Arena)?;

        turn += 1;

        if game.is_complete(turn, &peer_reply) {
            // Peer's last message terminated the protocol. Generate our farewell
            // (per first-mover prompt step 4 / second-mover prompt step 5) and
            // send it without polling — the peer is already on its way out.
            let farewell = receive_one_turn!(agent, &peer_reply);
            arena
                .send_only_async(&farewell)
                .await
                .map_err(PlayGameWasiError::Arena)?;
            break;
        }

        agent_reply = receive_one_turn!(agent, &peer_reply);

        // If our generated reply is itself terminal (case-insensitive contains
        // "goodbye"), send it once and break. Polling for a peer reply here would
        // hang because the peer's loop will also be exiting on this same message.
        if agent_reply
            .to_lowercase()
            .contains("goodbye")
        {
            arena
                .send_only_async(&agent_reply)
                .await
                .map_err(PlayGameWasiError::Arena)?;
            break;
        }
    }

    // 4) Extract guess from transcript BEFORE stop_session (which clears active state).
    let transcript = collect_transcript_for_extraction(&agent);
    let _ = agent.stop_session();

    let guess: Vec<u32> = match role {
        Role::First => {
            let peer_hashes = extract_peer_hash_array(&transcript, USER_ROLE).ok_or_else(|| {
                PlayGameWasiError::Arena(WasiArenaError::Other(
                    "could not extract peer hash array from transcript".to_string(),
                ))
            })?;
            game.intersection_against_peer_hashes(&peer_hashes)
        },
        Role::Second => {
            let peer_intersection =
                extract_peer_number_array(&transcript, USER_ROLE).ok_or_else(|| {
                    PlayGameWasiError::Arena(WasiArenaError::Other(
                        "could not extract peer plaintext intersection from transcript".to_string(),
                    ))
                })?;
            peer_intersection
                .into_iter()
                .filter(|n| {
                    game.private_set()
                        .contains(&n)
                })
                .collect()
        },
    };

    agent.print(&format!("{} submitting guess: {:?}", agent.name, guess));
    let guess_json = serde_json::to_string(&guess)
        .map_err(|e| PlayGameWasiError::Arena(WasiArenaError::Other(e.to_string())))?;
    arena
        .submit_message_async("guess", &guess_json)
        .await
        .map_err(PlayGameWasiError::Arena)?;

    // 5) Best-effort: log final operator messages (scores).
    match arena
        .operator_sync_async(0)
        .await
    {
        Ok(msgs) => {
            for m in msgs {
                agent.print(&format!("operator -> {}", m));
            }
        },
        Err(e) => tracing::warn!("post-submit operator sync failed (non-fatal): {e}"),
    }

    Ok(turn)
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
