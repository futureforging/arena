//! HTTP server entrypoint for the knock-knock arena stub.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use arena_stub::{process_audience_turn, ARENA_STUB_LISTEN_PORT};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

/// Display name for the arena-stub process (mirrors `aria-core` `Agent::print`: `"{name} -> {reply}"`).
const STUB_AGENT_NAME: &str = "agent";

/// Label for incoming teller lines (mirrors `aria-core` `Agent::receive_message` print: `peer <- {message}`).
const PEER_LABEL: &str = "peer";

#[derive(Clone)]
struct AppState {
    step: Arc<Mutex<u8>>,
}

#[derive(Deserialize)]
struct MessageRequest {
    message: String,
}

#[derive(Serialize)]
struct MessageReply {
    reply: String,
}

async fn post_message(
    State(state): State<AppState>,
    Json(body): Json<MessageRequest>,
) -> Json<MessageReply> {
    let mut guard = state
        .step
        .lock()
        .expect("audience state mutex poisoned");
    println!("{} <- {}", PEER_LABEL, body.message);
    let reply = process_audience_turn(&mut guard, &body.message);
    println!("{} -> {}", STUB_AGENT_NAME, reply);
    Json(MessageReply {
        reply,
    })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], ARENA_STUB_LISTEN_PORT));
    let state = AppState {
        step: Arc::new(Mutex::new(0)),
    };
    let app = Router::new()
        .route("/message", post(post_message))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("arena-stub listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
