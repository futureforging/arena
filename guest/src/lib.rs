//! # Arena Echo Guest
//!
//! WASI HTTP guest that echoes back whatever request it receives:
//! method, path, headers, and body.

#![cfg(target_arch = "wasm32")]

use axum::extract::Request;
use axum::routing::any;
use axum::{Json, Router};
use http_body_util::BodyExt;
use omnia_sdk::HttpResult;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::Level;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request as WasiRequest, Response};

struct EchoGuest;
wasip3::http::service::export!(EchoGuest);

impl Guest for EchoGuest {
    /// Routes incoming HTTP requests to the echo handler.
    #[omnia_wasi_otel::instrument(name = "echo_guest_handle", level = Level::DEBUG)]
    async fn handle(request: WasiRequest) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/{*path}", any(echo));
        omnia_wasi_http::serve(router, request).await
    }
}

/// Echo handler: returns method, path, headers, and body as JSON.
#[omnia_wasi_otel::instrument]
async fn echo(request: Request) -> HttpResult<Json<Value>> {
    let (parts, body) = request.into_parts();

    let method = parts.method.to_string();
    let path = parts.uri.path().to_string();
    let query = parts
        .uri
        .query()
        .map(|s| s.to_string())
        .unwrap_or_default();

    let headers: HashMap<String, String> = parts
        .headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();

    let body_bytes = body
        .collect()
        .await
        .map_err(|_| omnia_sdk::server_error!("Failed to read request body"))?
        .to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    Ok(Json(json!({
        "method": method,
        "path": path,
        "query": if query.is_empty() { Value::Null } else { Value::String(query) },
        "headers": headers,
        "body": body_str
    })))
}
