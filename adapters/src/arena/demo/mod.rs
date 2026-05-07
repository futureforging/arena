//! [`DemoArena`]: adapter against the Scaling Trust Arena's HTTP API. Composed over an
//! [`HttpTransport`], so the protocol logic (signed-join, session lifecycle, chat polling,
//! operator sync) is decoupled from how bytes reach the network.
//!
//! Submodules group the protocol by concern:
//! - [`session`] — session state and lifecycle (`ensure_psi_session`, `SessionSnapshot`).
//! - [`signer`] — Ed25519 signing via the host signer service.
//! - [`join`] — `/arena/join` and best-effort username registration.
//! - [`chat`] — peer chat send/receive with the polling loop.
//! - [`operator`] — operator-channel sync and structured submissions.
//! - [`url`] — base-URL normalization helpers.

mod chat;
mod join;
mod operator;
mod session;
mod signer;
mod url;

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use verity_core::arena::{Arena, ArenaError};

use self::session::SessionState;
use crate::transport::HttpTransport;

/// Maximum number of `chat/sync` polls per turn before giving up. Each poll is a
/// network round-trip to the Arena, which provides natural backpressure; with
/// typical RTTs of 100–500ms this yields a per-turn deadline of roughly 1–5 minutes.
const MAX_CHAT_SYNC_POLLS: u32 = 600;

/// Per-instance configuration for [`DemoArena::new`]. Grouped into a struct so the constructor
/// stays inside the workspace `too-many-arguments` clippy threshold and gains named-field clarity.
#[derive(Clone, Debug)]
pub struct DemoArenaConfig {
    /// Arena base URL (e.g. `https://arena-engine.nicolaos.org`). Normalized on construction.
    pub arena_url: String,
    /// Invite code for the challenge to join.
    pub invite: String,
    /// Signer service base URL (e.g. `http://127.0.0.1:8090`). Normalized on construction.
    pub signer_url: String,
    /// Display name to register on the arena.
    pub username: String,
    /// Model identifier advertised to the arena (matches outbound LLM payload).
    pub model: String,
}

/// Talks to the Scaling Trust Arena via HTTP using an injected [`HttpTransport`].
#[derive(Clone)]
pub struct DemoArena<H: HttpTransport> {
    http: H,
    base_url: String,
    signer_url: String,
    invite: String,
    username: String,
    model: String,
    session: Arc<Mutex<SessionState>>,
}

impl<H: HttpTransport> DemoArena<H> {
    pub fn new(http: H, config: DemoArenaConfig) -> Self {
        let DemoArenaConfig {
            arena_url,
            invite,
            signer_url,
            username,
            model,
        } = config;
        Self {
            http,
            base_url: url::normalize_arena_base_url(&arena_url),
            signer_url: url::normalize_arena_base_url(&signer_url),
            invite,
            username,
            model,
            session: Arc::new(Mutex::new(SessionState::default())),
        }
    }

    /// Builds an HTTP request with `content-type: application/json` and an optional
    /// `Authorization: Bearer <session_key>` header. When `require_session` is true and no
    /// `session_key` is provided, fails with [`ArenaError`] rather than silently sending unauthed.
    fn build_authed_request(
        session_key: Option<&str>,
        method: http::Method,
        url: &str,
        body: Bytes,
        require_session: bool,
    ) -> Result<http::Request<Bytes>, ArenaError> {
        if require_session && session_key.is_none() {
            return Err(ArenaError::Other(
                "arena transport: missing session key for authenticated request".to_string(),
            ));
        }

        let mut req = http::Request::builder()
            .method(method)
            .uri(url)
            .header("content-type", "application/json");

        if let Some(sk) = session_key {
            req = req.header("Authorization", format!("Bearer {}", sk.trim()));
        }

        req.body(body)
            .map_err(|e| ArenaError::Other(e.to_string()))
    }
}

impl<H: HttpTransport> Arena for DemoArena<H> {
    fn reset_async(&self) -> impl std::future::Future<Output = Result<(), ArenaError>> + Send {
        std::future::ready(Ok(()))
    }

    fn send_async(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<String, ArenaError>> + Send {
        let s = self.clone();
        let msg = message.to_string();
        async move {
            s.send_inner(&msg)
                .await
        }
    }

    fn send_only_async(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<(), ArenaError>> + Send {
        let s = self.clone();
        let m = message.to_string();
        async move {
            s.send_only_inner(&m)
                .await
        }
    }

    fn receive_async(
        &self,
    ) -> impl std::future::Future<Output = Result<String, ArenaError>> + Send {
        let s = self.clone();
        async move {
            s.receive_inner()
                .await
        }
    }

    fn operator_sync_async(
        &self,
        start_index: usize,
    ) -> impl std::future::Future<Output = Result<Vec<String>, ArenaError>> + Send {
        let s = self.clone();
        async move {
            s.operator_sync_inner(start_index)
                .await
        }
    }

    fn submit_message_async(
        &self,
        message_type: &str,
        content: &str,
    ) -> impl std::future::Future<Output = Result<(), ArenaError>> + Send {
        let s = self.clone();
        let mt = message_type.to_string();
        let c = content.to_string();
        async move {
            s.submit_message_inner(&mt, &c)
                .await
        }
    }
}
