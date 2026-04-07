//! HTTP server entrypoint for the arena stub (knock-knock and PSI).

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use arena_stub::{process_turn, PeerState, ARENA_STUB_LISTEN_PORT};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

/// Display name for the arena-stub process (mirrors `verity-core` `Agent::print`: `"{name} -> {reply}"`).
const STUB_AGENT_NAME: &str = "agent";

/// Label for incoming teller lines (mirrors `verity-core` `Agent::receive_message` print: `peer <- {message}`).
const PEER_LABEL: &str = "peer";

#[derive(Clone)]
struct AppState {
    state: Arc<Mutex<PeerState>>,
}

#[derive(Deserialize)]
struct MessageRequest {
    message: String,
}

#[derive(Serialize)]
struct MessageReply {
    reply: String,
}

/// Clears scripted peer state so the next `POST /message` starts a new game (game kind detected from the first line).
async fn post_reset(State(state): State<AppState>) -> StatusCode {
    let mut guard = state
        .state
        .lock()
        .expect("peer state mutex poisoned");
    *guard = PeerState::new();
    println!("arena-stub: peer state reset for next game");
    StatusCode::NO_CONTENT
}

async fn post_message(
    State(state): State<AppState>,
    Json(body): Json<MessageRequest>,
) -> Json<MessageReply> {
    let mut guard = state
        .state
        .lock()
        .expect("peer state mutex poisoned");
    println!("{} <- {}", PEER_LABEL, body.message);
    let reply = process_turn(&mut guard, &body.message);
    println!("{} -> {}", STUB_AGENT_NAME, reply);
    Json(MessageReply {
        reply,
    })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], ARENA_STUB_LISTEN_PORT));
    let state = AppState {
        state: Arc::new(Mutex::new(PeerState::new())),
    };
    let app = Router::new()
        .route("/message", post(post_message))
        .route("/reset", post(post_reset))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("arena-stub listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
