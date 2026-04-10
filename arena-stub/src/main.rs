//! HTTP server entrypoint for the arena stub (Scaling Trust Arena API shapes, PSI only).

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use arena_stub::{
    ArenaState, StubError, ARENA_STUB_LISTEN_PORT, STUB_CHALLENGE_ID, STUB_INVITE_AGENT,
    STUB_INVITE_PEER,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

/// Display name for the arena-stub process (mirrors `verity-core` `Agent::print`: `"{name} -> {reply}"`).
const STUB_AGENT_NAME: &str = "agent";

/// Label for incoming chat lines (mirrors `verity-core` `Agent::receive_message` print: `peer <- {message}`).
const PEER_LABEL: &str = "peer";

#[derive(Clone)]
struct AppState {
    arena: Arc<Mutex<ArenaState>>,
}

#[derive(Serialize)]
struct CreateChallengeResponseJson {
    id: String,
    invites: Vec<String>,
}

#[derive(Deserialize)]
struct JoinBody {
    invite: String,
}

#[derive(Serialize)]
struct JoinResponseJson {
    #[serde(rename = "ChallengeID")]
    challenge_id: String,
}

#[derive(Deserialize)]
struct ArenaSyncQuery {
    channel: String,
    #[serde(rename = "from")]
    recipient: String,
    index: usize,
}

#[derive(Serialize)]
struct OperatorMessageJson {
    from: String,
    index: usize,
    content: String,
}

#[derive(Serialize)]
struct OperatorSyncResponse {
    messages: Vec<OperatorMessageJson>,
}

#[derive(Deserialize)]
struct ChatSendBody {
    channel: String,
    #[serde(rename = "from")]
    sender: String,
    content: String,
}

#[derive(Serialize)]
struct ChatMessageJson {
    from: String,
    index: usize,
    content: String,
}

#[derive(Serialize)]
struct ChatSyncResponse {
    messages: Vec<ChatMessageJson>,
}

#[derive(Deserialize)]
struct ArenaMessageBody {
    #[serde(rename = "challengeId")]
    challenge_id: String,
    #[serde(rename = "from")]
    sender: String,
    #[serde(rename = "messageType")]
    message_type: String,
    content: String,
}

fn map_stub_error(e: StubError) -> (StatusCode, String) {
    let status = match e {
        StubError::NoActiveChallenge | StubError::UnknownChannel => StatusCode::BAD_REQUEST,
        StubError::UnknownInvite => StatusCode::NOT_FOUND,
    };
    (status, e.to_string())
}

async fn post_reset(State(state): State<AppState>) -> StatusCode {
    let mut guard = state
        .arena
        .lock()
        .expect("arena state mutex poisoned");
    guard.reset();
    println!("arena-stub: state reset");
    StatusCode::NO_CONTENT
}

async fn post_challenges_psi(State(state): State<AppState>) -> impl IntoResponse {
    let mut guard = state
        .arena
        .lock()
        .expect("arena state mutex poisoned");
    let r = guard.create_or_replace_psi_challenge();
    println!("arena-stub: created PSI challenge {}", r.id);
    Json(CreateChallengeResponseJson {
        id: r.id,
        invites: r.invites,
    })
}

async fn post_arena_join(
    State(state): State<AppState>,
    Json(body): Json<JoinBody>,
) -> impl IntoResponse {
    let mut guard = state
        .arena
        .lock()
        .expect("arena state mutex poisoned");
    match guard.join_with_invite(&body.invite) {
        Ok(j) => Json(JoinResponseJson {
            challenge_id: j.challenge_id,
        })
        .into_response(),
        Err(e) => {
            let (code, msg) = map_stub_error(e);
            (code, msg).into_response()
        },
    }
}

async fn get_arena_sync(
    State(state): State<AppState>,
    Query(q): Query<ArenaSyncQuery>,
) -> impl IntoResponse {
    let guard = state
        .arena
        .lock()
        .expect("arena state mutex poisoned");
    match guard.operator_sync(&q.channel, &q.recipient, q.index) {
        Ok(rows) => {
            let messages: Vec<OperatorMessageJson> = rows
                .into_iter()
                .map(|(index, from, content)| OperatorMessageJson {
                    from,
                    index,
                    content,
                })
                .collect();
            Json(OperatorSyncResponse {
                messages,
            })
            .into_response()
        },
        Err(e) => {
            let (code, msg) = map_stub_error(e);
            (code, msg).into_response()
        },
    }
}

async fn post_chat_send(
    State(state): State<AppState>,
    Json(body): Json<ChatSendBody>,
) -> impl IntoResponse {
    println!("{} <- {} (channel {})", PEER_LABEL, body.content, body.channel);
    let mut guard = state
        .arena
        .lock()
        .expect("arena state mutex poisoned");
    match guard.chat_send(&body.channel, &body.sender, &body.content) {
        Ok(()) => {
            let peer_line = guard
                .challenge
                .as_ref()
                .and_then(|c| {
                    c.chat_messages
                        .last()
                })
                .filter(|m| m.from == STUB_INVITE_PEER);
            if let Some(last) = peer_line {
                println!("{} -> {}", STUB_AGENT_NAME, last.content);
            }
            StatusCode::OK.into_response()
        },
        Err(e) => {
            let (code, msg) = map_stub_error(e);
            (code, msg).into_response()
        },
    }
}

async fn get_chat_sync(
    State(state): State<AppState>,
    Query(q): Query<ArenaSyncQuery>,
) -> impl IntoResponse {
    let guard = state
        .arena
        .lock()
        .expect("arena state mutex poisoned");
    match guard.chat_sync(&q.channel, q.index) {
        Ok(rows) => {
            let messages: Vec<ChatMessageJson> = rows
                .into_iter()
                .map(|(index, from, content)| ChatMessageJson {
                    from,
                    index,
                    content,
                })
                .collect();
            Json(ChatSyncResponse {
                messages,
            })
            .into_response()
        },
        Err(e) => {
            let (code, msg) = map_stub_error(e);
            (code, msg).into_response()
        },
    }
}

async fn post_arena_message(
    State(state): State<AppState>,
    Json(body): Json<ArenaMessageBody>,
) -> impl IntoResponse {
    let mut guard = state
        .arena
        .lock()
        .expect("arena state mutex poisoned");
    match guard.arena_message_submit(
        &body.challenge_id,
        &body.sender,
        &body.message_type,
        &body.content,
    ) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            let (code, msg) = map_stub_error(e);
            (code, msg).into_response()
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], ARENA_STUB_LISTEN_PORT));
    let state = AppState {
        arena: Arc::new(Mutex::new(ArenaState::new())),
    };
    let app = Router::new()
        .route("/reset", post(post_reset))
        .route("/api/v1/challenges/psi", post(post_challenges_psi))
        .route("/api/v1/arena/join", post(post_arena_join))
        .route("/api/v1/arena/sync", axum::routing::get(get_arena_sync))
        .route("/api/v1/chat/send", post(post_chat_send))
        .route("/api/v1/chat/sync", axum::routing::get(get_chat_sync))
        .route("/api/v1/arena/message", post(post_arena_message))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("arena-stub listening on {addr}");
    println!(
        "stub challenge id: {STUB_CHALLENGE_ID}, invites: {STUB_INVITE_AGENT}, {STUB_INVITE_PEER}"
    );
    axum::serve(listener, app).await?;
    Ok(())
}
