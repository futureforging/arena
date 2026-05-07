//! Transport adapters: protocol-shaped ports for outbound communication used by other adapters.
//!
//! `verity-core` is intentionally transport-agnostic. The `Arena` and `Llm` ports describe
//! *what* an agent does, not *how* the bytes get there. Concrete adapters (e.g. [`DemoArena`])
//! reach the network through one of the protocol-shaped traits in this module — today
//! [`http::HttpTransport`]; future protocols (SMTP, WebSocket, gRPC, etc.) live alongside it.
//!
//! Each transport trait is a *generic port* with no built-in implementation. Concrete impls
//! live wherever the host capabilities are: WASI-backed HTTP, for instance, lives in the
//! WASI guest crate (`secure-agent`) where `omnia_wasi_http` is available.
//!
//! [`DemoArena`]: crate::arena::DemoArena

pub mod http;

pub use http::{HttpError, HttpTransport};
