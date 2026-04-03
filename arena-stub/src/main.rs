//! HTTP server entrypoint for the knock-knock arena stub.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use arena_stub::process_audience_turn;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

/// Default TCP port when `ARENA_STUB_PORT` is unset.
const DEFAULT_LISTEN_PORT: u16 = 3000;

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
    let reply = process_audience_turn(&mut guard, &body.message);
    Json(MessageReply {
        reply,
    })
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let port = std::env::var("ARENA_STUB_PORT")
        .ok()
        .and_then(|s| {
            s.parse::<u16>()
                .ok()
        })
        .unwrap_or(DEFAULT_LISTEN_PORT);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
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
