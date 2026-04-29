mod arena_transport;
mod play_wasi;
mod production_arena;
mod stub_arena;
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

use arena_transport::ArenaTransport;
use play_wasi::play_psi_wasi;
use production_arena::ProductionArena;
use stub_arena::StubArena;
use verity_core::agent::Agent;
use verity_core::tool::ToolRegistry;
use verity_tools::arena_client::ArenaClientTool;
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
/// Request body: `{"arena_url": "...", "game": "psi"}` (both required).
/// Optional: `"invite": "inv_..."` — when present, uses signed production join + bearer session;
/// when absent, creates a challenge and self-joins against `arena_url` (local stub / dev).
/// Optional: `"signer_url"` — signer base URL (default `http://127.0.0.1:8090`; used when `invite` is present).
/// Response body: `{"turns": 5, "status": "complete", "game": "psi"}` (fields may vary by game).
///
/// Note: no `#[omnia_wasi_otel::instrument]` on this handler — that wrapper breaks Axum’s `Handler`
/// impl for wasm.
async fn play_handler(body: Bytes) -> impl IntoResponse {
    match play_handler_inner(body).await {
        Ok(json) => json.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")).into_response(),
    }
}

async fn build_psi_guest_agent<A: ArenaTransport>(arena: A) -> anyhow::Result<Agent<WasiEnvironment, WasiLlm>> {
    let llm = WasiLlm::new().await.context("initializing LLM")?;
    let environment = WasiEnvironment;

    let arena_for_tool = arena.clone();
    let arena_tool = ArenaClientTool::new(move |msg| {
        arena_for_tool.send_sync(msg).map_err(|e| e.to_string())
    });

    let registry = ToolRegistry::new(vec![
        Box::new(arena_tool),
        // SecretsTool and HttpClientTool can be added here when
        // the game loop needs them; for now arena is sufficient
    ]);

    Ok(Agent {
        name: String::from("SecureAgent"),
        environment,
        llm,
        tools: registry,
        active_session: None,
    })
}

async fn play_handler_inner(body: Bytes) -> anyhow::Result<Json<Value>> {
    let input: Value = serde_json::from_slice(&body).context("parsing request body")?;

    let game_name = input["game"].as_str().context("missing 'game' field")?;
    if game_name != "psi" {
        anyhow::bail!("only 'psi' is supported in this build (got {game_name:?})");
    }

    let arena_url = input["arena_url"]
        .as_str()
        .context("missing 'arena_url' field")?;
    let invite = input["invite"].as_str();
    let signer_url = input["signer_url"]
        .as_str()
        .unwrap_or("http://127.0.0.1:8090");

    let turns = match invite {
        None => {
            let arena = StubArena::new(arena_url);
            let agent = build_psi_guest_agent(arena.clone()).await?;
            play_psi_wasi(agent, arena).await
        }
        Some(inv) => {
            let arena = ProductionArena::new(arena_url, inv, signer_url);
            let agent = build_psi_guest_agent(arena.clone()).await?;
            play_psi_wasi(agent, arena).await
        }
    }
    .map_err(|e| anyhow::anyhow!("game failed: {e:?}"))?;

    Ok(Json(serde_json::json!({
        "turns": turns,
        "status": "complete",
        "game": game_name,
    })))
}
