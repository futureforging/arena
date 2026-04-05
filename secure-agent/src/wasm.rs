mod play_wasi;
mod wasi_arena;
mod wasi_environment;
mod wasi_llm;

use anyhow::Context;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use http::StatusCode;
use serde_json::Value;
use tracing::Level;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response as WasiResponse};

use play_wasi::{play_knock_knock_wasi, play_psi_wasi};
use wasi_arena::WasiArena;
use wasi_environment::WasiEnvironment;
use wasi_llm::WasiLlm;

struct Http;
wasip3::http::service::export!(Http);

impl Guest for Http {
    #[omnia_wasi_otel::instrument(name = "secure_agent_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<WasiResponse, ErrorCode> {
        let router = Router::new().route("/play", post(play_handler));
        omnia_wasi_http::serve(router, request).await
    }
}

/// Handles POST /play — plays a game to completion.
///
/// Request body: `{"arena_url": "http://127.0.0.1:3000", "game": "knock-knock"}` or `"game": "psi"` (both fields required).
/// Response body: `{"turns": 5, "status": "complete", "game": "knock-knock"}` (fields may vary by game).
///
/// Note: no `#[omnia_wasi_otel::instrument]` on this handler — that wrapper breaks Axum’s `Handler`
/// impl for wasm.
async fn play_handler(body: Bytes) -> impl IntoResponse {
    match play_handler_inner(body).await {
        Ok(json) => json.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")).into_response(),
    }
}

async fn play_handler_inner(body: Bytes) -> anyhow::Result<Json<Value>> {
    let input: Value = serde_json::from_slice(&body).context("parsing request body")?;
    let arena_url = input["arena_url"]
        .as_str()
        .context("missing 'arena_url' field")?;
    let game_name = input["game"]
        .as_str()
        .context("missing 'game' field (use \"knock-knock\" or \"psi\")")?;

    let llm = WasiLlm::new().await.context("initializing LLM")?;
    let environment = WasiEnvironment;
    let agent = secure_core::agent::Agent {
        name: String::from("SecureAgent"),
        environment,
        llm,
        active_session: None,
    };

    let arena = WasiArena::new(arena_url);

    let turns = match game_name {
        "knock-knock" => play_knock_knock_wasi(agent, arena).await,
        "psi" => play_psi_wasi(agent, arena).await,
        other => anyhow::bail!("unknown game: {other}"),
    }
    .map_err(|e| anyhow::anyhow!("game failed: {e:?}"))?;

    Ok(Json(serde_json::json!({
        "turns": turns,
        "status": "complete",
        "game": game_name,
    })))
}
